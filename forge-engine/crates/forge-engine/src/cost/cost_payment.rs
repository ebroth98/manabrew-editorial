//! Cost payment orchestrator — mirrors Java's `forge.game.cost.CostPayment`.
//!
//! `CostPayment` collects payment decisions from the agent (via `decide_cost_part`)
//! and executes them (via `pay_as_decided`). It supports two flows:
//!
//! - `pay_cost()` — sequential decide-then-pay per part (human flow, Java's `payCost()`)
//! - `pay_computer_costs()` — batch all decisions, then batch pay (AI flow, Java's `payComputerCosts()`)
//!
//! The agent's `pays_right_after_decision()` determines which flow is used,
//! mirroring Java's `CostDecisionMakerBase.paysRightAfterDecision()`.
//!
//! ## Architecture Note
//!
//! In Java, `CostPart.payAsDecided()` is a method on each cost part that takes
//! `(Player, PaymentDecision, SpellAbility, boolean)`. The cost part has access
//! to the game via `card.getGame()`.
//!
//! In Rust, many cost payments need access to:
//! - `TriggerHandler` (for firing Sacrificed, Discarded, Taps, etc.)
//! - `ManaPool` (for mana payment, waterbend, add mana)
//! - `PlayerAgent` (for choosing targets: typed sacrifice, typed discard, etc.)
//! - `GameRng` (for flip coin, roll dice)
//!
//! These live on `GameLoop`, not `GameState`. Therefore:
//! - `pay_as_decided()` handles costs that only need `GameState` + decision data
//! - Complex costs requiring GameLoop context (triggers, agents, RNG, mana pools)
//!   are handled by `GameLoop::pay_ability_cost()` in `game_action.rs` which has
//!   full access to all subsystems.

use crate::agent::PlayerAgent;
use crate::cost::payment_decision::PaymentDecision;
use crate::cost::{Cost, CostPart};
use crate::game::GameState;
use crate::ids::{CardId, PlayerId};

/// Orchestrates payment of a `Cost` by collecting decisions from an agent
/// and executing them. Mirrors Java's `CostPayment` class.
///
/// In Java, `CostPayment extends ManaConversionMatrix` — the mana conversion
/// matrix is transferred to `PaymentDecision.matrix` before `payAsDecided`.
pub struct CostPayment {
    /// The original, unadjusted cost.
    pub cost: Cost,
    /// The cost after adjustment (reductions, increases, etc.).
    /// Mirrors Java's `adjustedCost` field.
    pub adjusted_cost: Cost,
    /// The source card being paid for.
    pub source: CardId,
    /// The player paying the cost.
    pub player: PlayerId,
    /// Cost parts that have been successfully paid (for refund on cancel).
    pub paid_cost_parts: Vec<CostPart>,
    /// Whether we are paying as an effect (not a spell/ability activation).
    pub is_effect: bool,
}

impl CostPayment {
    /// Create a new `CostPayment` for the given cost and source.
    /// Mirrors Java's `CostPayment(Cost, SpellAbility)` constructor.
    pub fn new(cost: Cost, source: CardId, player: PlayerId, is_effect: bool) -> Self {
        let adjusted_cost = cost.clone();
        CostPayment {
            cost,
            adjusted_cost,
            source,
            player,
            paid_cost_parts: Vec::new(),
            is_effect,
        }
    }

    /// Check if all cost parts have been paid.
    /// Mirrors Java's `isFullyPaid()`.
    pub fn is_fully_paid(&self) -> bool {
        self.paid_cost_parts.len() == self.adjusted_cost.parts.len()
    }

    /// Refund all paid cost parts (on cancel/failure).
    /// Mirrors Java's `refundPayment()`.
    ///
    /// Java iterates `paidCostParts`, calls `part.refund(sourceCard)`,
    /// resets CostPartWithList tracking lists, then calls
    /// `new ManaRefundService(ability).refundManaPaid()`.
    pub fn refund_payment(&mut self, game: &mut GameState) {
        for part in &self.paid_cost_parts {
            refund_cost_part(game, self.source, self.player, part);
        }
        // TODO: Mana refund — Java calls `new ManaRefundService(ability).refundManaPaid()`
        // which restores mana to the pool. This needs ManaPool access from GameLoop.
        self.paid_cost_parts.clear();
    }

    /// Sequential decide-then-pay flow (human players).
    /// Mirrors Java's `CostPayment.payCost(CostDecisionMakerBase)`.
    ///
    /// For each cost part:
    ///   1. Call `agent.decide_cost_part()` (Java's `part.accept(decisionMaker)`)
    ///   2. If decision is Some, immediately execute `pay_as_decided()`
    ///   3. If decision is None or payment fails, return false
    ///
    /// Java also allows cost reordering via `player.getController().orderCosts()`.
    pub fn pay_cost(
        &mut self,
        game: &mut GameState,
        agent: &mut dyn PlayerAgent,
    ) -> bool {
        // TODO: self.adjusted_cost = CostAdjustment::adjust(&self.cost, ...);
        let mut parts = self.adjusted_cost.parts.clone();

        // Allow agent to reorder costs (human UI picks order).
        // Mirrors Java: if (adjustedCost.getCostParts().size() > 1) { parts = controller.orderCosts(parts); }
        if parts.len() > 1 {
            parts = agent.order_cost_parts(parts);
        }

        for part in &parts {
            let decision = agent.decide_cost_part(
                self.player,
                self.source,
                part,
                game,
            );

            match decision {
                Some(pd) => {
                    if !pay_as_decided(game, self.player, self.source, part, &pd, self.is_effect) {
                        return false;
                    }
                    self.paid_cost_parts.push(part.clone());
                }
                None => {
                    return false;
                }
            }
        }

        true
    }

    /// Batch decide-then-pay flow (AI agents).
    /// Mirrors Java's `CostPayment.payComputerCosts(CostDecisionMakerBase)`.
    ///
    /// Phase 1: Collect all decisions (cancel if any is None).
    /// Phase 2: Execute all decisions in order.
    pub fn pay_computer_costs(
        &mut self,
        game: &mut GameState,
        agent: &mut dyn PlayerAgent,
    ) -> bool {
        // TODO: adjust cost via CostAdjustment::adjust()
        let parts = self.adjusted_cost.parts.clone();

        // Phase 1: Collect all decisions
        let mut decisions: Vec<(CostPart, PaymentDecision)> = Vec::new();
        for part in &parts {
            let decision = agent.decide_cost_part(
                self.player,
                self.source,
                part,
                game,
            );

            match decision {
                Some(pd) => {
                    if agent.pays_right_after_decision() {
                        if !pay_as_decided(game, self.player, self.source, part, &pd, self.is_effect) {
                            return false;
                        }
                    }
                    decisions.push((part.clone(), pd));
                }
                None => return false,
            }
        }

        // Phase 2: Execute all decisions
        for (part, pd) in &decisions {
            if !pay_as_decided(game, self.player, self.source, part, pd, self.is_effect) {
                return false;
            }
        }

        true
    }

    /// Check if a cost can be paid as additional costs.
    /// Mirrors Java's `CostPayment.canPayAdditionalCosts(Cost, SpellAbility, boolean)`.
    pub fn can_pay_additional_costs(
        cost: &Cost,
        game: &GameState,
        source: CardId,
        player: PlayerId,
        _is_effect: bool,
    ) -> bool {
        // TODO: cost = CostAdjustment::adjust(cost, ability, effect);
        crate::cost::can_pay_ignoring_mana(cost, game, source, player)
    }

    /// Handle offering/emerge sacrifice after cost payment.
    /// Mirrors Java's `CostPayment.handleOfferings(SpellAbility, boolean, boolean)`.
    pub fn handle_offerings(
        _game: &mut GameState,
        _source: CardId,
        _test: bool,
        _cost_is_paid: bool,
    ) -> bool {
        // TODO: Port Java's handleOfferings():
        // - If sa.isOffering(): sacrifice the offering card, fire zone triggers
        // - If sa.isEmerge(): sacrifice the emerge card, update LKI, fire zone triggers
        true
    }
}

/// Execute a single cost part payment based on the decision.
/// Mirrors Java's `CostPart.payAsDecided(Player, PaymentDecision, SpellAbility, boolean)`.
///
/// Handles all cost types. For costs requiring only `GameState`, delegates to the
/// individual cost module functions. For costs requiring GameLoop context (triggers,
/// agents, mana pools, RNG), performs the state mutation here where possible and
/// leaves trigger firing to the `GameLoop::pay_ability_cost()` caller.
pub fn pay_as_decided(
    game: &mut GameState,
    player: PlayerId,
    source: CardId,
    cost_part: &CostPart,
    decision: &PaymentDecision,
    _is_effect: bool,
) -> bool {
    match cost_part {
        // ── Tap/Untap ───────────────────────────────────────────────────
        CostPart::Tap => {
            crate::cost::cost_tap::pay_as_decided(game, source);
            true
        }
        CostPart::Untap => {
            crate::cost::cost_untap::pay_as_decided(game, source);
            true
        }

        // ── Mana ────────────────────────────────────────────────────────
        CostPart::Mana(_) => {
            // Mana payment is special — Java calls player.getController().payManaCost()
            // which dispatches to InputPayMana (human) or ComputerUtilMana (AI).
            // In Rust, handled by GameLoop via auto_tap_lands + mana_pool.try_pay().
            // This is a no-op here; GameLoop handles it before calling pay_as_decided.
            true
        }

        // ── Life ────────────────────────────────────────────────────────
        CostPart::PayLife(amount) => {
            let resolved = crate::cost::resolve_dynamic_amount(game, source, player, *amount);
            crate::cost::cost_pay_life::pay_as_decided(game, player, resolved);
            true
        }

        // ── Sacrifice ───────────────────────────────────────────────────
        CostPart::Sacrifice { type_filter, .. } => {
            if type_filter == "CARDNAME" || type_filter == "NICKNAME" {
                crate::cost::cost_sacrifice::pay_as_decided_self(game, source, player);
            } else if let PaymentDecision::Cards(cards) = decision {
                crate::cost::cost_sacrifice::pay_as_decided_cards(game, cards, player);
            }
            // Trigger firing (Sacrificed) handled by GameLoop caller
            true
        }

        // ── Discard ─────────────────────────────────────────────────────
        CostPart::Discard { type_filter, .. } => {
            if type_filter == "CARDNAME" || type_filter == "NICKNAME" {
                crate::cost::cost_discard::pay_as_decided_self(game, source, player);
            } else if let PaymentDecision::Cards(cards) = decision {
                crate::cost::cost_discard::pay_as_decided_cards(game, cards, player);
            }
            // Trigger firing (Discarded, DiscardedAll) handled by GameLoop caller
            true
        }

        // ── Counters ────────────────────────────────────────────────────
        CostPart::SubCounter { amount, counter_type } => {
            let resolved = crate::cost::resolve_dynamic_amount(game, source, player, *amount);
            crate::cost::cost_sub_counter::pay_as_decided(game, source, resolved, counter_type);
            true
        }
        CostPart::AddCounter { amount, counter_type } => {
            let resolved = crate::cost::resolve_dynamic_amount(game, source, player, *amount);
            crate::cost::cost_put_counter::pay_as_decided(game, source, resolved, counter_type);
            true
        }

        // ── Exile ───────────────────────────────────────────────────────
        CostPart::Exile { type_filter, .. } => {
            if type_filter == "CARDNAME" || type_filter == "OriginalHost" {
                crate::cost::cost_exile::pay_as_decided_self(game, source);
            } else if let PaymentDecision::Cards(cards) = decision {
                crate::cost::cost_exile::pay_as_decided_cards(game, cards);
            }
            true
        }
        CostPart::ExileFromAnyGrave { .. } | CostPart::ExileFromSameGrave { .. } => {
            if let PaymentDecision::Cards(cards) = decision {
                crate::cost::cost_exile::pay_as_decided_cards(game, cards);
            }
            true
        }
        CostPart::ExileCtrlOrGrave { .. } => {
            if let PaymentDecision::Cards(cards) = decision {
                crate::cost::cost_exile_ctrl_or_grave::pay_as_decided_cards(game, cards);
            }
            true
        }

        // ── Return ──────────────────────────────────────────────────────
        CostPart::Return { type_filter, .. } => {
            if type_filter == "CARDNAME" || type_filter == "NICKNAME" {
                crate::cost::cost_return::pay_as_decided_self(game, source);
            } else if let PaymentDecision::Cards(cards) = decision {
                crate::cost::cost_return::pay_as_decided_cards(game, cards);
            }
            true
        }

        // ── Tap/Untap Type ──────────────────────────────────────────────
        CostPart::TapType { .. } => {
            if let PaymentDecision::Cards(cards) = decision {
                crate::cost::cost_tap_type::pay_as_decided_cards(game, cards);
            }
            // Taps trigger per card handled by GameLoop caller
            true
        }
        CostPart::UntapType { .. } => {
            if let PaymentDecision::Cards(cards) = decision {
                crate::cost::cost_untap_type::pay_as_decided_cards(game, cards);
            }
            true
        }

        // ── Energy/Shards ───────────────────────────────────────────────
        CostPart::PayEnergy(amount) => {
            let resolved = crate::cost::resolve_dynamic_amount(game, source, player, *amount);
            crate::cost::cost_pay_energy::pay_as_decided(game, player, resolved);
            true
        }
        CostPart::PayShards(amount) => {
            let resolved = crate::cost::resolve_dynamic_amount(game, source, player, *amount);
            crate::cost::cost_pay_shards::pay_as_decided(game, player, resolved);
            true
        }

        // ── Damage ──────────────────────────────────────────────────────
        CostPart::DamageYou(amount) => {
            crate::cost::cost_damage::pay_as_decided(game, player, *amount);
            // DamageDone trigger handled by GameLoop caller
            true
        }

        // ── Draw ────────────────────────────────────────────────────────
        CostPart::Draw(amount) => {
            let resolved = crate::cost::resolve_dynamic_amount(game, source, player, *amount);
            crate::cost::cost_draw::pay_as_decided(game, player, resolved);
            true
        }

        // ── Mill ────────────────────────────────────────────────────────
        CostPart::Mill(amount) => {
            let resolved = crate::cost::resolve_dynamic_amount(game, source, player, *amount);
            crate::cost::cost_mill::pay_as_decided(game, player, resolved);
            // Milled trigger per card + zone change triggers handled by GameLoop caller
            true
        }

        // ── Reveal ──────────────────────────────────────────────────────
        CostPart::Reveal { .. } => {
            // Card selection done by agent in GameLoop; reveal is display-only.
            // Java's CostReveal.doPayment() calls game.getAction().reveal().
            true
        }

        // ── Exert ───────────────────────────────────────────────────────
        CostPart::Exert { .. } => {
            // Exert sets card.exerted = true; needs agent for target selection.
            // Handled by GameLoop::pay_exert_cost().
            true
        }

        // ── Enlist ──────────────────────────────────────────────────────
        CostPart::Enlist { .. } => {
            // Needs agent for creature selection + power transfer + Enlisted trigger.
            // Handled by GameLoop::pay_enlist_cost().
            true
        }

        // ── Gain Life (opponent) ────────────────────────────────────────
        CostPart::GainLife(amount) => {
            crate::cost::cost_gain_life::pay_as_decided(game, player, *amount);
            true
        }

        // ── Gain Control ────────────────────────────────────────────────
        CostPart::GainControl { .. } => {
            if let PaymentDecision::Cards(cards) = decision {
                let opponent = game.opponent_of(player);
                crate::cost::cost_gain_control::pay_as_decided_cards(game, cards, opponent);
            }
            true
        }

        // ── Remove Any Counter ──────────────────────────────────────────
        CostPart::RemoveAnyCounter { .. } => {
            // Needs agent to choose which permanent and which counter type.
            // Handled by GameLoop::pay_remove_any_counter_cost().
            true
        }

        // ── Unattach ────────────────────────────────────────────────────
        CostPart::Unattach => {
            crate::cost::cost_unattach::pay_as_decided(game, source);
            // Unattached trigger handled by GameLoop caller
            true
        }

        // ── Exiled Move To Grave ────────────────────────────────────────
        CostPart::ExiledMoveToGrave { .. } => {
            if let PaymentDecision::Cards(cards) = decision {
                crate::cost::cost_exiled_move_to_grave::pay_as_decided_cards(game, cards);
            }
            true
        }

        // ── Add Mana ────────────────────────────────────────────────────
        CostPart::AddMana { .. } => {
            // Needs ManaPool access from GameLoop
            // Handled by GameLoop::pay_ability_cost().
            true
        }

        // ── Waterbend ───────────────────────────────────────────────────
        CostPart::Waterbend { .. } => {
            // Needs agent (choose_convoke) + mana pool access.
            // Handled by GameLoop::pay_waterbend_cost().
            true
        }

        // ── Choose Color ────────────────────────────────────────────────
        CostPart::ChooseColor(_) => {
            if let PaymentDecision::Colors(colors) = decision {
                game.card_mut(source).chosen_colors = colors.iter().map(|c| c.long_name().to_string()).collect();
            }
            true
        }

        // ── Choose Creature Type ────────────────────────────────────────
        CostPart::ChooseCreatureType(_) => {
            if let PaymentDecision::Type(t) = decision {
                let card = game.card_mut(source);
                card.chosen_type = Some(t.clone());
                card.chosen_type_controller = Some(player);
                card.chosen_type_revealed = false;
            }
            true
        }

        // ── Flip Coin ───────────────────────────────────────────────────
        CostPart::FlipCoin(_) => {
            // Needs GameRng + FlippedCoin trigger.
            // Handled by GameLoop::pay_ability_cost().
            true
        }

        // ── Roll Dice ───────────────────────────────────────────────────
        CostPart::RollDice { .. } => {
            // Needs GameRng + RolledDie/RolledDieOnce triggers.
            // Handled by GameLoop::pay_ability_cost().
            true
        }

        // ── Exile From Stack ────────────────────────────────────────────
        CostPart::ExileFromStack { .. } => {
            // Needs agent for stack target selection.
            // Handled by GameLoop::pay_exile_from_stack_cost().
            true
        }

        // ── Collect Evidence ────────────────────────────────────────────
        CostPart::CollectEvidence(_) => {
            // Needs agent for graveyard card selection + CollectEvidence trigger.
            // Handled by GameLoop::pay_collect_evidence_cost().
            true
        }

        // ── Forage ──────────────────────────────────────────────────────
        CostPart::Forage => {
            // Needs agent for exile-3 vs sac-food choice + Forage trigger.
            // Handled by GameLoop::pay_forage_cost().
            true
        }

        // ── Put Card To Library ─────────────────────────────────────────
        CostPart::PutCardToLib { lib_pos, type_filter, .. } => {
            if type_filter == "CARDNAME" || type_filter == "NICKNAME" {
                crate::cost::cost_put_card_to_lib::pay_as_decided_self(game, source, *lib_pos);
            } else if let PaymentDecision::Cards(cards) = decision {
                crate::cost::cost_put_card_to_lib::pay_as_decided_cards(game, cards, *lib_pos);
            }
            true
        }

        // ── Promise Gift ────────────────────────────────────────────────
        CostPart::PromiseGift => {
            // Needs agent for opponent selection.
            // GameLoop sets game.card_mut(source).promised_gift = chosen.
            true
        }

        // ── Reveal Chosen ───────────────────────────────────────────────
        CostPart::RevealChosen { reveal_type } => {
            crate::cost::cost_reveal_chosen::pay_as_decided(game, source, reveal_type);
            true
        }

        // ── Behold ──────────────────────────────────────────────────────
        CostPart::Behold { exile, .. } => {
            if let PaymentDecision::Cards(cards) = decision {
                crate::cost::cost_behold::pay_as_decided_cards(game, cards, *exile);
            }
            true
        }

        // ── Blight ──────────────────────────────────────────────────────
        CostPart::Blight(_) => {
            if let PaymentDecision::Cards(cards) = decision {
                crate::cost::cost_blight::pay_as_decided_cards(game, cards);
            }
            true
        }
    }
}

/// Refund a single cost part.
/// Mirrors Java's `CostPart.refund(Card sourceCard)`.
///
/// Only some cost types support refund — zone-change costs (sacrifice, exile, etc.)
/// are rolled back via `GameSnapshot` restore instead.
fn refund_cost_part(game: &mut GameState, source: CardId, player: PlayerId, part: &CostPart) {
    match part {
        CostPart::Tap => {
            crate::cost::cost_tap::refund(game, source);
        }
        CostPart::Untap => {
            crate::cost::cost_untap::refund(game, source);
        }
        CostPart::SubCounter { amount, counter_type } => {
            crate::cost::cost_sub_counter::refund(game, source, *amount, counter_type);
        }
        CostPart::AddCounter { amount, counter_type } => {
            crate::cost::cost_put_counter::refund(game, source, *amount, counter_type);
        }
        CostPart::PayEnergy(amount) => {
            crate::cost::cost_pay_energy::refund(game, player, *amount);
        }
        CostPart::PayShards(amount) => {
            crate::cost::cost_pay_shards::refund(game, player, *amount);
        }
        CostPart::PayLife(amount) => {
            // Restore life — mirrors Java's CostPayLife not having explicit refund,
            // but the snapshot rollback handles this. Included for completeness.
            game.player_mut(player).life += *amount;
        }
        CostPart::ChooseColor(_) => {
            crate::cost::cost_choose_color::refund(game, source);
        }
        CostPart::Blight(_) => {
            // Could refund by removing -1/-1 counters, but snapshot rollback handles it.
        }
        // Most cost parts (sacrifice, discard, exile, return, etc.) are not
        // individually refundable — zone changes are rolled back by GameSnapshot.
        // Java's CostPartWithList.refund() is a no-op for most types.
        _ => {}
    }
}
