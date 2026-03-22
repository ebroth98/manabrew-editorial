pub mod params;
pub mod target_choices;
pub mod target_restrictions;

use serde::{Deserialize, Serialize};

use crate::ability::api_type::ApiType;
use crate::ability::effects::resolve_defined_players;
use crate::agent::{PlayerAgent, TargetChoice};
use crate::card::card_damage_map::CardDamageMap;
use crate::card::card_zone_table::CardZoneTable;
use crate::cost::{parse_cost, Cost};
use crate::game::GameState;
use crate::ids::{CardId, PlayerId};
use crate::mana::ManaPool;
use crate::parsing::{keys, Params};
use forge_foundation::ZoneType;

pub use target_choices::TargetChoices;
pub use target_restrictions::{TargetKind, TargetRestrictions};

/// Alternative casting costs — mirrors Java's `OptionalCost` / `AlternativeCost`.
/// Tracks how a spell was cast so resolution can apply the correct behaviour
/// (e.g. Evoke → sacrifice on ETB, Dash → haste + bounce, Flashback → exile on resolve).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlternativeCost {
    Flashback,
    Spectacle,
    Evoke,
    Dash,
    Blitz,
    Escape,
    Overload,
    Madness,
    Foretell,
    Emerge,
    Suspend,
    /// Cast face-down as a 2/2 creature for {3} (Morph).
    Morph,
    /// Cast face-down as a 2/2 creature for {3}, +1/+1 counter on turn face-up (Megamorph).
    Megamorph,
    /// Cast as an Aura with enchant creature for the bestow cost.
    Bestow,
    /// Cast for warp cost; exile at beginning of next end step.
    Warp,
    /// Sacrifice-based alternative cost (e.g. Fireblast: sacrifice two Mountains).
    SacrificeAlt,
    /// Cast a plotted card from exile for free.
    Plot,
}

impl AlternativeCost {
    /// True if this is a morph-style face-down cast (Morph or Megamorph).
    pub fn is_morph(self) -> bool {
        matches!(self, AlternativeCost::Morph | AlternativeCost::Megamorph)
    }
}

/// Generic mana cost for casting a card face-down via Morph/Megamorph ({3}).
pub const MORPH_GENERIC_COST: i32 = 3;

/// Power and toughness of a face-down morph creature.
pub const MORPH_PT: i32 = 2;

// ── SpellAbility (mirrors Java's SpellAbility.java) ──────────────────

/// A spell or ability with its own targeting, costs, and sub-ability chain.
/// Mirrors Java's `SpellAbility` class — each node in the chain has its own
/// `target_restrictions`, `target_chosen`, `sub_ability`, `api`, etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpellAbility {
    /// Effect API type (e.g. DealDamage, Destroy, Draw).
    /// Mirrors Java's `ApiType api` field.
    pub api: Option<ApiType>,
    /// The card that hosts this ability. Mirrors Java's `hostCard`.
    pub source: Option<CardId>,
    /// The player who activated/cast this. Mirrors Java's `activatingPlayer`.
    pub activating_player: PlayerId,
    /// The player who chooses this ability's targets. Mirrors Java's
    /// `targetingPlayer` field.
    pub targeting_player: Option<PlayerId>,
    /// The raw ability text (pipe-delimited params).
    pub ability_text: String,
    /// Parsed pipe-delimited parameters.
    pub params: Params,
    /// Targeting restrictions parsed from `ValidTgts$`.
    /// `None` means this ability doesn't use targeting.
    /// Mirrors Java's `targetRestrictions` field.
    pub target_restrictions: Option<TargetRestrictions>,
    /// The chosen targets for this ability.
    /// Mirrors Java's `targetChosen` field.
    pub target_chosen: TargetChoices,
    /// Parsed costs from `Cost$` parameter.
    /// Mirrors Java's `payCosts` field.
    pub pay_costs: Option<Cost>,
    /// Linked sub-ability chain. Mirrors Java's `subAbility` field
    /// (AbilitySub extends SpellAbility).
    pub sub_ability: Option<Box<SpellAbility>>,
    /// Whether this is a spell (not an ability).
    pub is_spell: bool,
    /// Whether this is a triggered ability.
    pub is_trigger: bool,
    /// Whether this is an activated ability.
    pub is_activated: bool,
    /// Card that owns the trigger (for intervening-if recheck).
    pub trigger_source: Option<CardId>,
    /// Index into card.triggers for intervening-if recheck.
    pub trigger_index: Option<usize>,
    /// Alternative cost used to cast this spell (Flashback, Spectacle, Evoke, Dash, etc.).
    pub alt_cost: Option<AlternativeCost>,
    /// Whether the kicker cost was paid.
    pub kicked: bool,
    /// Whether buyback was paid (spell returns to hand on resolve).
    pub buyback_paid: bool,
    /// Whether this spell is overloaded (targets all valid instead of one).
    pub overloaded: bool,
    /// Whether this spell is a copy (created by Storm, Replicate, etc.).
    pub is_copy: bool,
    /// Number of times the kicker/multikicker cost was paid.
    pub kick_count: u32,
    /// Number of times the replicate cost was paid.
    pub replicate_count: u32,
    /// Whether a generic optional additional cost was paid.
    pub optional_generic_cost_paid: bool,
    /// Sum of integer values remembered on the trigger that spawned this
    /// ability (Java: TriggerRememberAmount / sa.getTriggerRemembered()).
    pub trigger_remembered_amount: i32,
    /// The value chosen for X in the mana cost (e.g. Fireball X=5 means 5 damage).
    /// Mirrors Java's `SpellAbility.getXManaCostPaid()`.
    pub x_mana_cost_paid: u32,
    /// Cards discarded as part of the cost payment.
    /// Mirrors Java's `CostPayment.getPaidList("Discarded")`.
    pub discarded_cost_cards: Vec<crate::ids::CardId>,
    /// Java parity: batched zone-change table accumulated for `ChangeZoneResolve`.
    #[serde(skip)]
    pub change_zone_table: Option<CardZoneTable>,
    /// Java parity: accumulated damage map for `DamageResolve`.
    #[serde(skip)]
    pub damage_map: Option<CardDamageMap>,
    /// Java parity: accumulated prevented-damage map for `DamageResolve`.
    #[serde(skip)]
    pub prevent_map: Option<CardDamageMap>,
}

impl SpellAbility {
    /// Whether this ability uses targeting.
    /// Mirrors Java's `usesTargeting()`: `return targetRestrictions != null`.
    pub fn uses_targeting(&self) -> bool {
        self.target_restrictions.is_some()
    }

    /// Check if a parameter is set to "True" (case-insensitive).
    /// Common pattern for boolean params like `Ninjutsu$ True`, `Mega$ True`, etc.
    pub fn param_is_true(&self, key: &str) -> bool {
        self.params.is_true(key)
    }

    /// Get the chosen targets. Mirrors Java's `getTargets()`.
    pub fn get_targets(&self) -> &TargetChoices {
        &self.target_chosen
    }

    /// Get the chosen targets mutably. Mirrors Java's `getTargets()` for mutation.
    pub fn get_targets_mut(&mut self) -> &mut TargetChoices {
        &mut self.target_chosen
    }

    /// Get the sub-ability. Mirrors Java's `getSubAbility()`.
    pub fn get_sub_ability(&self) -> Option<&SpellAbility> {
        self.sub_ability.as_deref()
    }

    /// Get the sub-ability mutably.
    pub fn get_sub_ability_mut(&mut self) -> Option<&mut SpellAbility> {
        self.sub_ability.as_deref_mut()
    }

    /// Clear the chosen targets. Mirrors Java's `clearTargets()`.
    pub fn clear_targets(&mut self) {
        self.target_chosen = TargetChoices::default();
    }

    /// Walk the entire ability chain and choose targets for each node that
    /// uses targeting. Mirrors Java's `SpellAbility.setupTargets()` do/while loop.
    ///
    /// Returns `true` if all targeting succeeded, `false` if any node couldn't
    /// find valid targets.
    pub fn setup_targets(
        &mut self,
        game: &GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        mana_pools: &[ManaPool],
    ) -> bool {
        // Walk self, then sub_ability chain — mirrors Java's do/while
        if self.uses_targeting() {
            self.clear_targets();
            self.targeting_player = choose_targeting_player(self, game, agents);
            if !choose_targets_for(self, game, agents, mana_pools) {
                return false;
            }
        }

        // Walk sub-ability chain
        let mut current = self.sub_ability.as_deref_mut();
        while let Some(sa) = current {
            if sa.uses_targeting() {
                sa.clear_targets();
                sa.targeting_player = choose_targeting_player(sa, game, agents);
                if !choose_targets_for(sa, game, agents, mana_pools) {
                    return false;
                }
            }
            current = sa.sub_ability.as_deref_mut();
        }

        if !crate::staticability::static_ability_must_target::meets_must_target_restriction(
            game, self,
        ) {
            return false;
        }

        true
    }

    /// Create a simple SpellAbility for tests and triggers.
    pub fn new_simple(source: Option<CardId>, player: PlayerId, ability_text: &str) -> Self {
        let params = Params::from_raw(ability_text);
        let api = params
            .get(keys::SP)
            .or_else(|| params.get(keys::DB))
            .or_else(|| params.get(keys::AB))
            .and_then(|s| ApiType::smart_value_of(s));
        let target_restrictions = TargetRestrictions::new(&params);
        let cost = params.get(keys::COST).map(parse_cost);

        SpellAbility {
            api,
            source,
            activating_player: player,
            targeting_player: None,
            ability_text: ability_text.to_string(),
            params,
            target_restrictions,
            target_chosen: TargetChoices::default(),
            pay_costs: cost,
            sub_ability: None,
            is_spell: false,
            is_trigger: false,
            is_activated: false,
            trigger_source: None,
            trigger_index: None,
            alt_cost: None,
            kicked: false,
            buyback_paid: false,
            overloaded: false,
            is_copy: false,
            kick_count: 0,
            replicate_count: 0,
            optional_generic_cost_paid: false,
            trigger_remembered_amount: 0,
            x_mana_cost_paid: 0,
            discarded_cost_cards: Vec::new(),
            change_zone_table: None,
            damage_map: None,
            prevent_map: None,
        }
    }
}

// build_spell_ability now lives in ability::ability_factory.
// Re-export here for backward compatibility.
pub use crate::ability::ability_factory::build_spell_ability;

/// Choose targets for a single SpellAbility node, populating its `target_chosen`.
/// Mirrors Java's `PlayerController.chooseTargetsFor(currentAbility)`.
/// Returns `true` if targeting succeeded.
fn choose_targets_for(
    sa: &mut SpellAbility,
    game: &GameState,
    agents: &mut [Box<dyn PlayerAgent>],
    mana_pools: &[ManaPool],
) -> bool {
    let tr = match &sa.target_restrictions {
        Some(tr) => tr,
        None => return true,
    };

    let player = sa.targeting_player.unwrap_or(sa.activating_player);

    // Spells with TargetMin$ 0 (e.g. Fireball) can be cast with zero targets.
    // Java's DeterministicController skips setupDeterministicTargets when
    // isTargetNumberValid() is already true (min=0, 0 targets), consuming no RNG.
    // We must match by returning early without calling any agent choose method.
    let min_targets = tr.get_min_targets(game, sa);
    if min_targets <= 0 {
        return true;
    }

    if !tr.has_candidates(game, player, sa.source) {
        return false;
    }

    match &tr.target_kind {
        TargetKind::None => {}
        TargetKind::Player => {
            agents[player.index()].snapshot_state(game, mana_pools);
            let agent = &mut agents[player.index()];
            // Filter valid players by ValidTgts: "Opponent" restricts to opponents only,
            // "Player" means any player including the caster.
            let is_opponent_only = tr
                .valid_tgts
                .iter()
                .any(|v| v.eq_ignore_ascii_case("Opponent"));
            let valid_players: Vec<PlayerId> = game
                .alive_players()
                .into_iter()
                .filter(|&pid| !is_opponent_only || pid != player)
                .collect();
            sa.target_chosen.target_player = agent.choose_target_player(player, &valid_players);
        }
        TargetKind::Any => {
            let valid_players: Vec<PlayerId> =
                if target_restrictions::any_target_allows_players(&tr.valid_tgts) {
                    game.alive_players().into_iter().collect()
                } else {
                    Vec::new()
                };
            let valid_cards: Vec<CardId> =
                target_restrictions::get_all_candidates_any_filtered(game, &tr.valid_tgts, player)
                    .into_iter()
                    .filter(|&cid| {
                        target_restrictions::can_be_targeted_by_sa(game, cid, player, sa)
                    })
                    .collect();
            agents[player.index()].snapshot_state(game, mana_pools);
            let agent = &mut agents[player.index()];
            match agent.choose_target_any(player, &valid_players, &valid_cards) {
                TargetChoice::Player(pid) => sa.target_chosen.target_player = Some(pid),
                TargetChoice::Card(cid) => sa.target_chosen.target_card = Some(cid),
                TargetChoice::None => {}
            }
        }
        TargetKind::Creature(ref filter) => {
            let base = target_restrictions::get_all_candidates_creature_filtered(
                game,
                filter.as_deref(),
                player,
            );
            let valid: Vec<CardId> =
                target_restrictions::apply_other_source_filter(base, filter.as_deref(), sa.source)
                    .into_iter()
                    .filter(|&cid| {
                        target_restrictions::can_be_targeted_by_sa(game, cid, player, sa)
                    })
                    .collect();
            agents[player.index()].snapshot_state(game, mana_pools);
            let agent = &mut agents[player.index()];
            sa.target_chosen.target_card = agent.choose_target_card(player, &valid);
        }
        TargetKind::Permanent(ref filter) => {
            let base = target_restrictions::get_all_battlefield_permanents_filtered(
                game,
                filter.as_deref(),
                player,
            );
            let valid: Vec<CardId> =
                target_restrictions::apply_other_source_filter(base, filter.as_deref(), sa.source)
                    .into_iter()
                    .filter(|&cid| {
                        target_restrictions::can_be_targeted_by_sa(game, cid, player, sa)
                    })
                    .collect();
            agents[player.index()].snapshot_state(game, mana_pools);
            let agent = &mut agents[player.index()];
            sa.target_chosen.target_card = agent.choose_target_card(player, &valid);
        }
        TargetKind::CardInZone { zone, filter } => {
            let valid = target_restrictions::get_valid_cards_in_zone(
                game,
                *zone,
                player,
                filter.as_deref(),
                sa.source,
            );
            agents[player.index()].snapshot_state(game, mana_pools);
            let agent = &mut agents[player.index()];
            sa.target_chosen.target_card =
                agent.choose_target_card_from_zone(player, *zone, &valid);
        }
        TargetKind::Spell => {
            let valid = target_restrictions::get_all_candidates_spells(game);
            // Apply TargetType$ filter if present
            let valid = if let Some(ref filter) = sa
                .target_restrictions
                .as_ref()
                .and_then(|tr| tr.target_type_filter.as_ref())
            {
                target_restrictions::filter_spells_by_type(game, &valid, filter)
            } else {
                valid
            };
            agents[player.index()].snapshot_state(game, mana_pools);
            let agent = &mut agents[player.index()];
            sa.target_chosen.target_stack_entry = agent.choose_target_spell(player, &valid);
        }
    }

    true
}

fn choose_targeting_player(
    sa: &SpellAbility,
    game: &GameState,
    agents: &mut [Box<dyn PlayerAgent>],
) -> Option<PlayerId> {
    if let Some(defined) = sa.params.get(keys::TARGETING_PLAYER) {
        let candidates = resolve_defined_players(defined, sa.activating_player, game);
        if candidates.is_empty() {
            return None;
        }
        return agents[sa.activating_player.index()]
            .choose_target_player(sa.activating_player, &candidates);
    }
    Some(sa.activating_player)
}

// Re-export MagicStack and StackEntry from zone module (their canonical home,
// matching Java's `forge.game.zone.MagicStack`).
pub use crate::zone::magic_stack::{MagicStack, StackEntry};
