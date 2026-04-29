use std::collections::BTreeMap;

use super::cost_payment::CostPaymentContext;
use super::*;

/// Sacrifice a batch of cards. Mirrors Java's `GameAction.sacrifice(Iterable<Card>, ...)`
/// at `forge/forge-game/src/main/java/forge/game/GameAction.java:2104`.
///
/// Single chokepoint for the per-card sequence:
///   1. capture LKI (counter map, power, toughness) before any zone change
///   2. write `lki_counters` and `set_lki_power_toughness` on the card so death
///      triggers (Modular, Servant of the Scale, etc.) see pre-move state
///   3. record the card on `game.last_sacrificed_card` for `Sacrificed$CardPower`
///      SVar lookups (Rite of Consumption, Altar's Reap)
///   4. fire `TriggerType::Sacrificed` with the controller as `player`
///   5. emit `ChangesZone(Battlefield → Graveyard)` carrying LKI counters
///   6. flush waiting triggers — matches against pre-move state, **with pump
///      triggers still on the card**, so Animate-granted triggers
///      (Supernatural Stamina's death-return) fire correctly. `clear_pump_triggers`
///      cleanup runs inside `move_card` after the match has already happened.
///   7. move the card to its owner's graveyard with agent notifications
///
/// After all cards: fire `TriggerType::SacrificedOnce` once per distinct
/// controller, with the batch payload in `RunParams.cards`. Java fires this
/// from the same chokepoint at `GameAction.java:2133-2138`. Cards that key off
/// it: Tasteful Offering, Camellia, the Seedmiser.
///
/// Returns the cards that were actually sacrificed (skipping any that had
/// been removed from the battlefield by an interim trigger).
///
/// **Ordering rationale (flush before move):** matches the existing pattern at
/// `cost_payment.rs:606`, `mana_payment.rs:349`, `cost_adjustment.rs:1219`
/// and the rest of the majority. Required so that a Sacrificed trigger on
/// the dying card itself (`ValidCard$ Card.Self` with `TriggerZones$
/// Battlefield`) finds its host still on the battlefield at match time.
///
/// **Why not `clear_pump_triggers` before emit:** see the note in the original
/// `pay_sacrifice_cost_internal` — Animate-granted triggers (Supernatural
/// Stamina) must remain active so they can be matched and queued by
/// `flush_waiting_triggers`. The `move_card` call below cleans them up after
/// the match is done.
pub(crate) fn perform_sacrifice(
    game: &mut GameState,
    trigger_handler: &mut TriggerHandler,
    agents: &mut [Box<dyn PlayerAgent>],
    cards: &[CardId],
) -> Vec<CardId> {
    let mut sacrificed: Vec<CardId> = Vec::with_capacity(cards.len());
    let mut by_controller: BTreeMap<PlayerId, Vec<CardId>> = BTreeMap::new();

    for &card_id in cards {
        if game.card(card_id).zone != ZoneType::Battlefield {
            continue;
        }
        let owner = game.card(card_id).owner;
        let controller = game.card(card_id).controller;

        let lki_counters = game.card(card_id).counters.clone();
        let lki_power = game.card(card_id).power();
        let lki_toughness = game.card(card_id).toughness();
        let lki_p1p1 = *lki_counters
            .get(&crate::card::CounterType::P1P1)
            .unwrap_or(&0);

        {
            let card = game.card_mut(card_id);
            card.lki_counters = Some(lki_counters);
            card.set_lki_power_toughness(Some(lki_power), Some(lki_toughness));
        }
        game.last_sacrificed_card = Some(card_id);

        trigger_handler.run_trigger(
            TriggerType::Sacrificed,
            RunParams {
                card: Some(card_id),
                player: Some(controller),
                ..Default::default()
            },
            false,
        );
        crate::ability::effects::emit_zone_trigger_with_lki_counters(
            trigger_handler,
            card_id,
            ZoneType::Battlefield,
            ZoneType::Graveyard,
            lki_p1p1,
            lki_power,
            lki_toughness,
        );
        trigger_handler.flush_waiting_triggers(game);
        game.move_card_with_agents(card_id, ZoneType::Graveyard, owner, agents);

        sacrificed.push(card_id);
        by_controller.entry(controller).or_default().push(card_id);
    }

    fire_sacrificed_once_for_batch(game, trigger_handler, &by_controller);

    sacrificed
}

/// Fire `TriggerType::SacrificedOnce` once per distinct controller in the batch.
///
/// Mirrors Java's batch tail at `GameAction.java:2133-2138`. Exposed separately
/// so call sites that retain their own per-card emission loop (because they
/// have site-specific LKI handling) can still get the batch trigger. Pass the
/// `(controller → cards)` map you accumulated as you sacrificed.
pub(crate) fn fire_sacrificed_once_for_batch(
    game: &mut GameState,
    trigger_handler: &mut TriggerHandler,
    by_controller: &BTreeMap<PlayerId, Vec<CardId>>,
) {
    if by_controller.is_empty() {
        return;
    }
    for (controller, batch_cards) in by_controller {
        trigger_handler.run_trigger(
            TriggerType::SacrificedOnce,
            RunParams {
                player: Some(*controller),
                cards: Some(batch_cards.clone()),
                ..Default::default()
            },
            false,
        );
    }
    trigger_handler.flush_waiting_triggers(game);
}

impl GameLoop {
    fn fixed_reserved_sacrifices_for_action(sa: &SpellAbility, source: CardId) -> Vec<CardId> {
        let mut reserved = Vec::new();
        let Some(pay_costs) = sa.pay_costs.as_ref() else {
            return reserved;
        };
        for part in &pay_costs.parts {
            if let CostPart::Sacrifice { type_filter, .. } = part {
                let reserved_card = match type_filter.as_str() {
                    "CARDNAME" | "NICKNAME" => Some(source),
                    "OriginalHost" => sa.original_host,
                    _ => None,
                };
                if let Some(card_id) = reserved_card {
                    if !reserved.contains(&card_id) {
                        reserved.push(card_id);
                    }
                }
            }
        }
        reserved
    }

    pub(crate) fn emit_tap_for_mana_triggers(&mut self, player: PlayerId, tapped_lands: &[CardId]) {
        for &land_id in tapped_lands {
            self.trigger_handler.run_trigger(
                TriggerType::Taps,
                RunParams {
                    card: Some(land_id),
                    player: Some(player),
                    ..Default::default()
                },
                false,
            );
            self.trigger_handler.run_trigger(
                TriggerType::TapsForMana,
                RunParams {
                    card: Some(land_id),
                    player: Some(player),
                    ..Default::default()
                },
                false,
            );
        }
    }

    pub(crate) fn get_activatable_abilities(
        &self,
        game: &GameState,
        player: PlayerId,
        can_play_sorcery: bool,
    ) -> Vec<(CardId, usize)> {
        let mut result = Vec::new();
        let available_mana = mana::calculate_available_mana(self.pool(player), game, player);
        let mut battlefield = game.cards_in_zone(ZoneType::Battlefield, player).to_vec();
        for &other_player in &game.player_order {
            if other_player == player {
                continue;
            }
            battlefield.extend(game.cards_in_zone(ZoneType::Battlefield, other_player));
        }
        let can_activate = |card_id: CardId, ab: &crate::ability::ActivatedAbility| {
            // Per-game activation cap (e.g. "GameActivationLimit$ 1").
            if let Some(limit) = ab.game_activation_limit {
                let used = game
                    .card(card_id)
                    .activations_this_game
                    .get(&ab.ability_index)
                    .copied()
                    .unwrap_or(0);
                if used >= limit {
                    return false;
                }
            }
            // PowerUp: once-per-game restriction
            if ab.power_up {
                let card = game.card(card_id);
                if card
                    .activations_this_game
                    .get(&ab.ability_index)
                    .copied()
                    .unwrap_or(0)
                    > 0
                {
                    return false;
                }
            }
            if ab.sorcery_speed && !can_play_sorcery {
                return false;
            }
            // Activated abilities that require targets should only be offered
            // when at least one legal target candidate exists.
            let sa_for_target_check =
                crate::spellability::build_spell_ability(game, card_id, &ab.ability_text, player);
            if crate::staticability::static_ability_cant_be_cast::cant_be_activated_ability(
                game,
                &game.cards,
                &sa_for_target_check,
                game.card(card_id),
                player,
            ) {
                return false;
            }
            // Activated-ability legality checks (split second, suppression, detention, etc.).
            if !crate::spellability::ability_activated::can_play(&sa_for_target_check, game) {
                return false;
            }
            if let Some(tr) = sa_for_target_check.target_restrictions.as_ref() {
                let min_targets = tr.get_min_targets(game, &sa_for_target_check);
                if min_targets > 0
                    && !crate::spellability::target_restrictions::has_candidates_in_spell_ability_chain(
                        game,
                        player,
                        &sa_for_target_check,
                    )
                {
                    return false;
                }
            }
            // Java parity: Equip/Attach abilities are offered if ValidTgts
            // candidates exist, without checking CantAttach statics.  CantAttach
            // is enforced at resolution time, not during action-space generation.
            // This matches Java's ActionSpace.hasValidTargets() behavior.
            let needs_mana = ab
                .cost
                .parts
                .iter()
                .any(|p| matches!(p, crate::cost::CostPart::Mana { .. }));
            let reserved_sacrifices =
                Self::fixed_reserved_sacrifices_for_action(&sa_for_target_check, card_id);
            let mana_for_check = if needs_mana {
                // Java parity: ComputerUtilMana.canPayManaCost(...) excludes mana
                // abilities on the same host card as the spell/ability being paid for.
                // Without that, cards like Gilded Goose incorrectly appear able to pay
                // for their own non-mana activated abilities.
                mana::calculate_available_mana_excluding(
                    self.pool(player),
                    game,
                    player,
                    Some(card_id),
                )
            } else {
                available_mana.clone()
            };
            let can_pay_cost = if reserved_sacrifices.is_empty() {
                crate::cost::can_pay_with_ability(
                    &ab.cost,
                    game,
                    &mana_for_check,
                    card_id,
                    player,
                    Some(&sa_for_target_check),
                )
            } else {
                crate::cost::can_pay_ignoring_mana_with_ability(
                    &ab.cost,
                    game,
                    card_id,
                    player,
                    &sa_for_target_check,
                ) && mana::can_pay_mana_cost_with_reserved_sacrifices(
                    game,
                    self.pool(player),
                    player,
                    card_id,
                    &ab.cost,
                    &reserved_sacrifices,
                    Some(&crate::mana::payment_context_for_sa(
                        game,
                        &sa_for_target_check,
                    )),
                )
            };
            if !can_pay_cost {
                return false;
            }
            true
        };

        for card_id in battlefield {
            let card = game.card(card_id);
            // Face-down creatures only expose morph turn-face-up ability (game rule).
            // All other abilities are hidden while face-down.
            if card.face_down {
                for ab in &card.activated_abilities {
                    if ab.ability_text.contains("Mode$ TurnFaceUp") && can_activate(card_id, ab) {
                        result.push((card_id, ab.ability_index));
                    }
                }
                continue;
            }
            for ab in &card.activated_abilities {
                if ab.is_mana_ability || ab.is_unlock_door {
                    continue;
                }
                // Skip abilities with ActivationZone$ Hand — they're for hand, not battlefield
                if ab.activation_zone == Some(ZoneType::Hand) {
                    continue;
                }
                if can_activate(card_id, ab) {
                    result.push((card_id, ab.ability_index));
                }
            }
        }

        // Check hand for abilities with ActivationZone$ Hand (e.g. Cycling)
        let hand = game.cards_in_zone(ZoneType::Hand, player).to_vec();
        for card_id in hand {
            let card = game.card(card_id);
            for ab in &card.activated_abilities {
                if ab.is_mana_ability || ab.is_unlock_door {
                    continue;
                }
                if ab.activation_zone == Some(ZoneType::Hand) && can_activate(card_id, ab) {
                    result.push((card_id, ab.ability_index));
                }
            }
        }

        // Check graveyard for abilities with ActivationZone$ Graveyard (e.g. Scavenge, Unearth)
        let graveyard = game.cards_in_zone(ZoneType::Graveyard, player).to_vec();
        for card_id in graveyard {
            let card = game.card(card_id);
            for ab in &card.activated_abilities {
                if ab.is_mana_ability || ab.is_unlock_door {
                    continue;
                }
                if ab.activation_zone == Some(ZoneType::Graveyard) && can_activate(card_id, ab) {
                    result.push((card_id, ab.ability_index));
                }
            }
        }

        // Check exile for abilities with ActivationZone$ Exile
        let exile = game.cards_in_zone(ZoneType::Exile, player).to_vec();
        for card_id in exile {
            let card = game.card(card_id);
            for ab in &card.activated_abilities {
                if ab.is_mana_ability || ab.is_unlock_door {
                    continue;
                }
                if ab.activation_zone == Some(ZoneType::Exile) && can_activate(card_id, ab) {
                    result.push((card_id, ab.ability_index));
                }
            }
        }

        result
    }

    /// Resolve an ability immediately without using the stack.
    ///
    /// Used for abilities that Java models as `AbilityStatic` (e.g. Plot) and
    /// for special actions like turning a Morph face up.
    pub(crate) fn resolve_immediate_ability(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        card_id: CardId,
        ab: &crate::ability::activated::ActivatedAbility,
    ) -> bool {
        // Pay costs
        if !self.pay_ability_cost(
            game,
            agents,
            player,
            card_id,
            &ab.cost,
            ab.ability_api,
            ab.cost.mandatory,
            CostPaymentContext::ActivatedAbility,
            None,
        ) {
            return false;
        }

        let card_name = game.card(card_id).card_name.clone();
        let ability_kind = if ab.ability_kind.is_empty() {
            "Unknown"
        } else {
            ab.ability_kind.as_str()
        };
        crate::agent::notify_all_agents(
            agents,
            crate::agent::GameLogEvent::action(format!(
                "Activated ability: {} | source={}",
                ability_kind, card_name
            ))
            .with_player(player)
            .with_source_card(card_id),
        );

        // Build the spell ability and resolve effect immediately (no stack).
        let sa = crate::spellability::build_spell_ability(game, card_id, &ab.ability_text, player);
        let entry = StackEntry {
            id: 0,
            spell_ability: sa,
            is_pending_cast: false,
            is_creature_spell: false,
            is_permanent_spell: false,
            cast_from_zone: None,
            optional_trigger_decider: None,
            optional_trigger_description: None,
            optional_trigger_source_name: None,
        };
        self.resolve_spell_effect(game, agents, &entry);

        // Apply continuous effects and SBA after the immediate resolution.
        crate::staticability::layer::apply_continuous_effects(game);
        super::check_sba(game, &mut self.trigger_handler, agents);
        self.process_triggers(game, agents);
        true
    }

    /// Resolve a mana ability immediately (no stack).
    pub(crate) fn resolve_mana_ability(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        card_id: CardId,
        ab: &crate::ability::activated::ActivatedAbility,
        express_choice: Option<u16>,
    ) {
        let api = crate::ability::api_type::ApiType::smart_value_of(ab.ability_kind.as_str());
        if !self.pay_ability_cost(
            game,
            agents,
            player,
            card_id,
            &ab.cost,
            api,
            ab.cost.mandatory,
            CostPaymentContext::ManaAbility,
            None,
        ) {
            return;
        }

        // If this is a ManaReflected ability, delegate to the effect resolver
        if ab.is_mana_reflected {
            let mut sa =
                crate::spellability::build_spell_ability(game, card_id, &ab.ability_text, player);
            sa.express_mana_choice = express_choice;
            self.resolve_single_effect(game, agents, &sa, None);
            self.mark_mana_undo_disqualified();
            // Fire triggers
            self.trigger_handler.run_trigger(
                TriggerType::TapsForMana,
                RunParams {
                    card: Some(card_id),
                    player: Some(player),
                    activator: Some(player),
                    ..Default::default()
                },
                false,
            );
            self.trigger_handler.run_trigger(
                TriggerType::ManaAdded,
                RunParams {
                    card: Some(card_id),
                    player: Some(player),
                    activator: Some(player),
                    ..Default::default()
                },
                false,
            );
            return;
        }

        // Build metadata params for produced mana
        let source_is_snow = game.card(card_id).type_line.is_snow();
        let mana_params = crate::mana::ManaProductionParams {
            source_card: card_id,
            is_snow: source_is_snow,
            restriction: ab.restrict_valid.as_deref().map(str::to_string),
            adds_no_counter: ab.adds_no_counter,
            adds_keywords: ab.adds_keywords.clone(),
            adds_keywords_valid: ab.adds_keywords_valid.clone(),
            adds_counters: ab.adds_counters.clone(),
            adds_counters_valid: ab.adds_counters_valid.clone(),
            triggers_when_spent: ab.triggers_when_spent.clone(),
        };

        if let Some(produced) = ab.produced.as_deref() {
            if produced.starts_with("Special") {
                // Delegate to the special mana handler in mana_effect
                let special = produced.strip_prefix("Special ").unwrap_or("");
                let sa = crate::spellability::build_spell_ability(
                    game,
                    card_id,
                    &ab.ability_text,
                    player,
                );
                let mut effect_ctx = crate::ability::effects::EffectContext {
                    game,
                    combat: Some(&mut self.combat),
                    agents,
                    trigger_handler: &mut self.trigger_handler,
                    token_templates: &self.token_templates,
                    token_art_variants: &self.token_art_variants,
                    token_fallback: &self.token_fallback,
                    edition_dates: &self.edition_dates,
                    mana_pools: &mut self.mana_pools,
                    parent_target_card: None,
                    rng: &mut *self.game_rng,
                };
                let tokens = crate::ability::effects::mana_effect::resolve_special_mana(
                    &mut effect_ctx,
                    &sa,
                    card_id,
                    player,
                    special,
                );
                // Add produced mana with metadata
                let mana_str = tokens.join(" ");
                crate::mana::add_produced_mana_to_pool(
                    &mut effect_ctx.mana_pools[player.index()],
                    &mana_str,
                    &mana_params,
                );
                // Fire triggers and return
                self.trigger_handler.run_trigger(
                    TriggerType::TapsForMana,
                    RunParams {
                        card: Some(card_id),
                        player: Some(player),
                        activator: Some(player),
                        ..Default::default()
                    },
                    false,
                );
                self.trigger_handler.run_trigger(
                    TriggerType::ManaAdded,
                    RunParams {
                        card: Some(card_id),
                        player: Some(player),
                        activator: Some(player),
                        ..Default::default()
                    },
                    false,
                );
                self.mark_mana_undo_disqualified();
                return;
            }

            // Determine mana production (color choice, Amount$, replacement effects)
            let amount_param = ab.amount.as_deref();
            let mana_string = crate::mana::determine_mana_production(
                game,
                agents,
                player,
                card_id,
                produced,
                amount_param,
                express_choice,
            );

            // Add the produced mana to the pool
            if let Some(ref ms) = mana_string {
                crate::mana::add_produced_mana_to_pool(self.pool_mut(player), ms, &mana_params);
            }
        }

        // Resolve SubAbility chain (e.g. DealDamage on pain lands)
        if let Some(sub_svar_name) = ab.sub_ability.as_deref() {
            if let Some(sub_text) = game.card(card_id).svars.get(sub_svar_name).cloned() {
                let sub_sa =
                    crate::spellability::build_spell_ability(game, card_id, &sub_text, player);
                self.resolve_single_effect(game, agents, &sub_sa, None);
                self.mark_mana_undo_disqualified();
            }
        }

        // Fire TapsForMana trigger (mana abilities produce mana)
        self.trigger_handler.run_trigger(
            TriggerType::TapsForMana,
            RunParams {
                card: Some(card_id),
                player: Some(player),
                activator: Some(player),
                ..Default::default()
            },
            false,
        );

        // Fire ManaAdded trigger (mirrors Java AbilityManaPart.produceMana)
        self.trigger_handler.run_trigger(
            TriggerType::ManaAdded,
            RunParams {
                card: Some(card_id),
                player: Some(player),
                activator: Some(player),
                ..Default::default()
            },
            false,
        );

        // Resolve mana-producing triggers inline (Static$ True triggers like Utopia Sprawl).
        // These fire from TapsForMana and produce extra mana without using the stack.
        // Mirrors Java's AbilityStatic resolution path for mana triggers.
        let pending = self.trigger_handler.run_waiting_triggers(game);
        if !pending.is_empty() {
            self.mark_mana_undo_disqualified();
        }
        for pt in pending {
            self.resolve_single_effect(game, agents, &pt.entry.spell_ability, None);
        }
    }

    /// Activate a non-mana ability: choose targets, pay costs, put on stack.
    pub(crate) fn play_activated_ability_on_stack(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        card_id: CardId,
        ab: &crate::ability::activated::ActivatedAbility,
    ) -> bool {
        let ability_text = ab.ability_text.clone();

        // Build full SpellAbility chain (including SubAbility$ links) and choose targets.
        // This mirrors Java activated ability resolution for abilities like Walking Bulwark,
        // where the root AB$ Pump resolves a DB$ Effect sub-ability.
        let mut sa = crate::spellability::build_spell_ability(game, card_id, &ability_text, player);
        sa.is_activated = true;
        if sa.api == Some(crate::ability::api_type::ApiType::Charm)
            && !crate::ability::effects::charm_effect::make_choices_precast(game, agents, &mut sa)
        {
            return false;
        }
        if !sa.setup_targets(game, agents, &self.mana_pools) {
            return false;
        }
        crate::ability::effects::emit_targeting_triggers_for_sa(
            &mut self.trigger_handler,
            game,
            card_id,
            &sa,
        );

        // PowerUp: reduce cost by card's mana cost if it entered the battlefield this turn
        let adjusted_cost = if ab.power_up && game.card(card_id).entered_battlefield_this_turn {
            let mut cost = ab.cost.clone();
            // Subtract the card's mana cost from the ability's mana cost
            let card_mc = game.card(card_id).mana_cost.clone();
            for part in &mut cost.parts {
                if let crate::cost::CostPart::Mana {
                    cost: ref mut mc, ..
                } = part
                {
                    *mc = mc.reduce_generic(card_mc.cmc());
                    break;
                }
            }
            cost
        } else {
            ab.cost.clone()
        };
        let host_before_payment = game.card(card_id).clone();
        let spell_desc = ab.spell_description_lower.as_str();
        let is_vehicle_crew = host_before_payment.type_line.has_subtype("Vehicle")
            && (host_before_payment
                .has_keyword_enum(crate::keyword::keyword_instance::Keyword::Crew)
                || spell_desc.starts_with("crew"));
        let is_mount_saddle = host_before_payment.type_line.has_subtype("Mount")
            && (host_before_payment
                .has_keyword_enum(crate::keyword::keyword_instance::Keyword::Saddle)
                || spell_desc.starts_with("saddle"));
        let is_station = (host_before_payment.type_line.has_subtype("Spacecraft")
            || host_before_payment.type_line.has_subtype("Planet"))
            && (host_before_payment
                .has_keyword_enum(crate::keyword::keyword_instance::Keyword::Station)
                || spell_desc.starts_with("station"));
        let uses_waterbend = adjusted_cost
            .parts
            .iter()
            .any(|p| matches!(p, crate::cost::CostPart::Waterbend { .. }));
        let untapped_before_payment: Vec<CardId> = game
            .cards_in_zone(ZoneType::Battlefield, player)
            .iter()
            .copied()
            .filter(|&cid| !game.card(cid).tapped)
            .collect();

        // Pay costs
        if !self.pay_ability_cost(
            game,
            agents,
            player,
            card_id,
            &adjusted_cost,
            ab.ability_api,
            adjusted_cost.mandatory,
            CostPaymentContext::ActivatedAbility,
            Some(&mut sa),
        ) {
            return false;
        }
        let tapped_by_activation: Vec<CardId> = untapped_before_payment
            .into_iter()
            .filter(|&cid| game.card(cid).tapped)
            .collect();
        let tapped_crews: Vec<CardId> = tapped_by_activation
            .iter()
            .copied()
            .filter(|&cid| cid != card_id && game.card(cid).is_creature())
            .collect();

        // Track activation count (for PowerUp once-per-game)
        game.card_mut(card_id)
            .activations_this_game
            .entry(ab.ability_index)
            .and_modify(|c| *c += 1)
            .or_insert(1);

        // Push to stack
        let card_name = game.card(card_id).card_name.clone();
        let target_card = sa.target_chosen.target_card;
        let entry = StackEntry {
            id: 0,
            spell_ability: sa,
            is_pending_cast: false,
            is_creature_spell: false,
            is_permanent_spell: false,
            cast_from_zone: None,
            optional_trigger_decider: None,
            optional_trigger_description: None,
            optional_trigger_source_name: None,
        };
        let ability_kind = if ab.ability_kind.is_empty() {
            "Unknown"
        } else {
            ab.ability_kind.as_str()
        };
        let stack_message = format!("Activated ability: {} | source={}", ability_kind, card_name);
        let sa_for_trigger = self.push_spell_ability_to_stack(
            game,
            agents,
            player,
            StackPushContext {
                source_card: card_id,
                entry,
                pending_stack_id: None,
                stack_log_name: format!("{} ability", card_name),
                stack_message,
                target_card,
                event_kind: SpellAbilityLogEventKind::Action,
                move_source_to_stack: false,
                register_source_trigger: false,
            },
        );
        self.emit_post_stack_spell_ability_triggers(
            game,
            player,
            &sa_for_trigger,
            PostStackTriggerContext {
                source_card: card_id,
                cast_trigger: TriggerType::AbilityCast,
                emit_ability_activated: true,
                emit_waterbend: uses_waterbend,
                waterbend_cards: Vec::new(),
            },
        );
        if is_vehicle_crew || is_mount_saddle || is_station {
            for crew_card in &tapped_crews {
                let run_params = RunParams {
                    card: Some(card_id),
                    crew_cards: Some(vec![*crew_card]),
                    source_sa: Some(sa_for_trigger.clone()),
                    spell_ability: Some(sa_for_trigger.clone()),
                    cause: Some(sa_for_trigger.clone()),
                    cause_card: Some(card_id),
                    player: Some(player),
                    ..Default::default()
                };
                if is_vehicle_crew {
                    self.trigger_handler.run_trigger(
                        TriggerType::Crewed,
                        run_params.clone(),
                        false,
                    );
                }
                if is_mount_saddle {
                    self.trigger_handler.run_trigger(
                        TriggerType::Saddled,
                        run_params.clone(),
                        false,
                    );
                }
                if is_station {
                    self.trigger_handler.run_trigger(
                        TriggerType::Stationed,
                        run_params.clone(),
                        false,
                    );
                }
            }
        }
        true
    }
}
