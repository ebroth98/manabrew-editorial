//! Replacement logic for `Event$ BeginPhase`.
//!
//! Mirrors Java `ReplaceBeginPhase.java` in `forge/game/replacement/`.

use crate::card::Card;
use crate::game::GameState;
use crate::ids::CardId;
use crate::parsing::keys;
use forge_foundation::PhaseType;

use super::replacement_effect::ReplacementEffect;
use crate::card_trait_base::CardTrait;
use super::replacement_handler::ReplacementEvent;
use super::replacement_result::ReplacementResult;
use super::replacement_type::ReplacementType;

/// Mirrors Java `ReplaceBeginPhase.canReplace()`.
pub fn can_replace(
    effect: &ReplacementEffect,
    event: &ReplacementEvent,
    _game: &GameState,
    source_card: &Card,
) -> bool {
    if effect.event != ReplacementType::BeginPhase {
        return false;
    }
    let (player, phase) = match event {
        ReplacementEvent::BeginPhase { player, phase } => (*player, *phase),
        _ => return false,
    };
    if let Some(valid) = effect.params.get(keys::VALID_PLAYER) {
        if !effect.matches_valid_player(valid, player, source_card) {
            return false;
        }
    }
    if let Some(phases) = effect
        .params
        .get(keys::PHASE)
        .or_else(|| effect.params.get(keys::ACTIVE_PHASES))
    {
        let matches_phase = phases
            .split(',')
            .filter_map(|name| PhaseType::from_script_name(name.trim()))
            .any(|candidate| candidate == phase);
        if !matches_phase {
            return false;
        }
    }
    true
}

/// Mirrors Java `ReplacementHandler.executeReplacement()` for BeginPhase.
pub fn execute(
    effect: &ReplacementEffect,
    _event: &mut ReplacementEvent,
    _game: &GameState,
    _source_card_id: CardId,
) -> ReplacementResult {
    if effect
        .params
        .get(keys::PREVENT)
        .map(|s| s == "True")
        .unwrap_or(false)
        || effect.params.has(keys::SKIP)
    {
        return ReplacementResult::Skipped;
    }
    ReplacementResult::Replaced
}
