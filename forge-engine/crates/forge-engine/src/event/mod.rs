use forge_foundation::{PhaseType, ZoneType};
use serde::{Deserialize, Serialize};

use crate::ids::{CardId, PlayerId};

/// Event types — mirrors Java TriggerType enum (subset).
/// Start with 5 most common types, expand to full 160+ over time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TriggerType {
    ChangesZone,
    Phase,
    SpellCast,
    Attacks,
    DamageDone,
    /// Two creatures fought each other (SP$ Fight).
    Fight,
    /// A card was discarded (SP$ Discard).
    Discarded,
    /// A spell was countered (SP$ Counter).
    Countered,
}

/// Typed event parameter keys — mirrors Java AbilityKey enum.
/// In Java this is Map<AbilityKey, Object>. In Rust we use a struct
/// because Rust has no Object type (justified deviation).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunParams {
    pub card: Option<CardId>,
    pub card_lki: Option<CardId>,
    pub origin: Option<ZoneType>,
    pub destination: Option<ZoneType>,
    pub cause_player: Option<PlayerId>,
    pub player: Option<PlayerId>,
    pub phase: Option<PhaseType>,
    pub damage_source: Option<CardId>,
    pub damage_target_player: Option<PlayerId>,
    pub damage_target_card: Option<CardId>,
    pub damage_amount: Option<i32>,
    pub is_combat_damage: Option<bool>,
    pub attacker: Option<CardId>,
    pub defending_player: Option<PlayerId>,
    pub spell_card: Option<CardId>,
    pub spell_controller: Option<PlayerId>,
    /// Second card involved (e.g. second creature in a Fight trigger).
    pub card2: Option<CardId>,
    /// SpellAbility that was countered
    pub spell_ability: Option<crate::spellability::SpellAbility>,
    /// Cause of the event (e.g. counterspell)
    pub cause: Option<crate::spellability::SpellAbility>,
}
