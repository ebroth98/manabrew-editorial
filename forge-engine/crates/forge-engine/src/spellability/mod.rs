pub mod target_choices;
pub mod target_restrictions;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::agent::{PlayerAgent, TargetChoice};
use crate::cost::{parse_cost, Cost};
use crate::game::GameState;
use crate::ids::{CardId, PlayerId};
use crate::mana::ManaPool;
use crate::trigger::parse_pipe_params;
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
}

// ── SpellAbility (mirrors Java's SpellAbility.java) ──────────────────

/// A spell or ability with its own targeting, costs, and sub-ability chain.
/// Mirrors Java's `SpellAbility` class — each node in the chain has its own
/// `target_restrictions`, `target_chosen`, `sub_ability`, `api`, etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpellAbility {
    /// Effect API type (e.g. "DealDamage", "Destroy", "Draw").
    /// Mirrors Java's `ApiType api` field.
    pub api: Option<String>,
    /// The card that hosts this ability. Mirrors Java's `hostCard`.
    pub source: Option<CardId>,
    /// The player who activated/cast this. Mirrors Java's `activatingPlayer`.
    pub activating_player: PlayerId,
    /// The raw ability text (pipe-delimited params).
    pub ability_text: String,
    /// Parsed pipe-delimited parameters.
    pub params: BTreeMap<String, String>,
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
}

impl SpellAbility {
    /// Whether this ability uses targeting.
    /// Mirrors Java's `usesTargeting()`: `return targetRestrictions != null`.
    pub fn uses_targeting(&self) -> bool {
        self.target_restrictions.is_some()
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
            if !choose_targets_for(self, game, agents, mana_pools) {
                return false;
            }
        }

        // Walk sub-ability chain
        let mut current = self.sub_ability.as_deref_mut();
        while let Some(sa) = current {
            if sa.uses_targeting() {
                sa.clear_targets();
                if !choose_targets_for(sa, game, agents, mana_pools) {
                    return false;
                }
            }
            current = sa.sub_ability.as_deref_mut();
        }

        true
    }

    /// Create a simple SpellAbility for tests and triggers.
    pub fn new_simple(source: Option<CardId>, player: PlayerId, ability_text: &str) -> Self {
        let params = parse_pipe_params(ability_text);
        let api = params
            .get("SP")
            .or_else(|| params.get("DB"))
            .or_else(|| params.get("AB"))
            .cloned();
        let target_restrictions = TargetRestrictions::new(&params);
        let cost = params.get("Cost").map(|s| parse_cost(s));

        SpellAbility {
            api,
            source,
            activating_player: player,
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
        }
    }
}

/// Build a SpellAbility chain from a card's ability text, walking SubAbility$
/// SVars to construct the linked list.
/// Mirrors Java's `AbilityFactory.getAbility()` + sub-ability chain construction.
pub fn build_spell_ability(
    game: &GameState,
    card_id: CardId,
    ability_text: &str,
    player: PlayerId,
) -> SpellAbility {
    let params = parse_pipe_params(ability_text);
    let api = params
        .get("SP")
        .or_else(|| params.get("DB"))
        .or_else(|| params.get("AB"))
        .cloned();
    let target_restrictions = TargetRestrictions::new(&params);
    let cost = params.get("Cost").map(|s| parse_cost(s));

    // Recursively build sub-ability chain from SVars
    let sub_ability = if let Some(sub_svar_name) = params.get("SubAbility") {
        if let Some(sub_text) = game.card(card_id).svars.get(sub_svar_name).cloned() {
            Some(Box::new(build_spell_ability(
                game, card_id, &sub_text, player,
            )))
        } else {
            None
        }
    } else {
        None
    };

    SpellAbility {
        api,
        source: Some(card_id),
        activating_player: player,
        ability_text: ability_text.to_string(),
        params,
        target_restrictions,
        target_chosen: TargetChoices::default(),
        pay_costs: cost,
        sub_ability,
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
    }
}

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

    let player = sa.activating_player;

    if !tr.has_candidates(game, player, sa.source) {
        return false;
    }

    match &tr.target_kind {
        TargetKind::None => {}
        TargetKind::Player => {
            agents[player.index()].snapshot_state(game, mana_pools);
            let agent = &mut agents[player.index()];
            // "target player" means any player — including the caster themselves.
            let valid_players: Vec<PlayerId> = game.alive_players().into_iter().collect();
            sa.target_chosen.target_player = agent.choose_target_player(player, &valid_players);
        }
        TargetKind::Any => {
            // "any target" includes all alive players (the caster too) and all creatures.
            let valid_players: Vec<PlayerId> = game.alive_players().into_iter().collect();
            let valid_creatures: Vec<CardId> =
                target_restrictions::get_all_candidates_creatures(game)
                    .into_iter()
                    .filter(|&cid| {
                        target_restrictions::can_be_targeted_by(game, cid, player, sa.source)
                    })
                    .collect();
            agents[player.index()].snapshot_state(game, mana_pools);
            let agent = &mut agents[player.index()];
            match agent.choose_target_any(player, &valid_players, &valid_creatures) {
                TargetChoice::Player(pid) => sa.target_chosen.target_player = Some(pid),
                TargetChoice::Card(cid) => sa.target_chosen.target_card = Some(cid),
                TargetChoice::None => {}
            }
        }
        TargetKind::Creature(ref filter) => {
            let valid: Vec<CardId> = target_restrictions::get_all_candidates_creature_filtered(
                game,
                filter.as_deref(),
                player,
            )
            .into_iter()
            .filter(|&cid| target_restrictions::can_be_targeted_by(game, cid, player, sa.source))
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

// ── StackEntry (mirrors Java's SpellAbilityStackInstance) ────────────

/// An entry on the game stack (spell or ability waiting to resolve).
/// Mirrors Java's `SpellAbilityStackInstance` which wraps a `SpellAbility`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackEntry {
    pub id: u32,
    /// The spell ability with its full sub-ability chain and targets.
    pub spell_ability: SpellAbility,
    /// Whether this is a creature spell (goes to battlefield on resolve).
    pub is_creature_spell: bool,
    /// Whether this is a non-creature permanent spell.
    pub is_permanent_spell: bool,
    /// The zone the spell was cast from (for Flashback exile-on-resolve).
    pub cast_from_zone: Option<ZoneType>,
}

/// The game stack. Spells and abilities are added to the top and resolve LIFO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MagicStack {
    entries: Vec<StackEntry>,
    next_id: u32,
}

impl MagicStack {
    pub fn new() -> Self {
        MagicStack {
            entries: Vec::new(),
            next_id: 0,
        }
    }

    pub fn push(&mut self, mut entry: StackEntry) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        entry.id = id;
        self.entries.push(entry);
        id
    }

    pub fn pop(&mut self) -> Option<StackEntry> {
        self.entries.pop()
    }

    pub fn peek(&self) -> Option<&StackEntry> {
        self.entries.last()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &StackEntry> {
        self.entries.iter()
    }

    /// Remove and return the stack entry with the given ID (for Counter effects).
    /// Returns `None` if no entry with that ID exists.
    pub fn remove_by_id(&mut self, id: u32) -> Option<StackEntry> {
        if let Some(pos) = self.entries.iter().position(|e| e.id == id) {
            Some(self.entries.remove(pos))
        } else {
            None
        }
    }
}

impl Default for MagicStack {
    fn default() -> Self {
        Self::new()
    }
}
