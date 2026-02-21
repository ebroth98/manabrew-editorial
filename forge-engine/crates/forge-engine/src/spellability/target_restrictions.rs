//! Target restrictions for spell abilities.
//!
//! Mirrors Java's `spellability/TargetRestrictions.java` — defines what kinds
//! of targets a spell can select, checks for valid candidates, and retrieves
//! all valid target candidates.

use std::collections::BTreeMap;

use forge_foundation::ZoneType;
use serde::{Deserialize, Serialize};

use crate::card::card_property;
use crate::game::GameState;
use crate::ids::{CardId, PlayerId};

/// What kinds of targets a spell can select.
/// Mirrors Java's `TargetRestrictions.getValidTgts()` parsed target types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetKind {
    /// Player only (e.g. "ValidTgts$ Player")
    Player,
    /// Any player or creature (e.g. "ValidTgts$ Any")
    Any,
    /// Creature with optional filter (e.g. "ValidTgts$ Creature.nonBlack")
    Creature(Option<String>),
    /// No targets
    None,
}

/// Targeting restrictions for a spell ability.
/// Mirrors Java's `TargetRestrictions` — defines valid targets, min/max counts,
/// and which zones to search for targets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetRestrictions {
    /// Raw valid target strings (e.g. ["Creature.OppCtrl"])
    pub valid_tgts: Vec<String>,
    /// Parsed target kind
    pub target_kind: TargetKind,
    /// Minimum number of targets (default 1)
    pub min_targets: i32,
    /// Maximum number of targets (default 1)
    pub max_targets: i32,
    /// Zones to search for targets (default [Battlefield])
    pub tgt_zone: Vec<ZoneType>,
}

impl TargetRestrictions {
    /// Construct from parsed pipe params. Returns `None` if no `ValidTgts$`
    /// parameter exists (mirrors Java: null targetRestrictions means no targeting).
    pub fn new(params: &BTreeMap<String, String>) -> Option<Self> {
        let valid_tgts_str = params.get("ValidTgts")?;
        let valid_tgts: Vec<String> = valid_tgts_str.split(',').map(|s| s.trim().to_string()).collect();
        let target_kind = parse_target_kind(&valid_tgts[0]);
        let min_targets = params.get("TargetMin").and_then(|s| s.parse().ok()).unwrap_or(1);
        let max_targets = params.get("TargetMax").and_then(|s| s.parse().ok()).unwrap_or(1);

        Some(TargetRestrictions {
            valid_tgts,
            target_kind,
            min_targets,
            max_targets,
            tgt_zone: vec![ZoneType::Battlefield],
        })
    }

    /// Check if there is at least one valid target candidate.
    /// Mirrors Java's `TargetRestrictions.hasCandidates()`.
    pub fn has_candidates(&self, game: &GameState, player: PlayerId) -> bool {
        match &self.target_kind {
            TargetKind::None => true,
            TargetKind::Player => {
                game.alive_players().into_iter().any(|p| p != player)
            }
            TargetKind::Any => {
                let has_opponent = game.alive_players().into_iter().any(|p| p != player);
                if has_opponent {
                    true
                } else {
                    !get_all_candidates_creatures(game).is_empty()
                }
            }
            TargetKind::Creature(ref filter) => {
                !get_all_candidates_creature_filtered(game, filter.as_deref(), player).is_empty()
            }
        }
    }
}

/// Parse a single ValidTgts value into a TargetKind.
fn parse_target_kind(val: &str) -> TargetKind {
    let val = val.trim();
    if val.eq_ignore_ascii_case("Any") {
        return TargetKind::Any;
    }
    if val.eq_ignore_ascii_case("Player") {
        return TargetKind::Player;
    }
    if val.starts_with("Creature") {
        let filter = val.strip_prefix("Creature").unwrap();
        if filter.is_empty() {
            return TargetKind::Creature(None);
        }
        let filter = filter.strip_prefix('.').unwrap_or(filter);
        return TargetKind::Creature(Some(filter.to_string()));
    }
    // Fallback: treat as "Any" if unrecognized
    TargetKind::Any
}

/// Parse `ValidTgts$` from a raw ability string.
/// Convenience wrapper for code that doesn't have parsed params yet.
pub fn parse_valid_targets(ability: &str) -> TargetKind {
    let params = crate::trigger::parse_pipe_params(ability);
    match params.get("ValidTgts") {
        Some(val) => parse_target_kind(val),
        None => TargetKind::None,
    }
}

/// Check if there is at least one valid target for the given ability string.
/// Convenience wrapper that creates a temporary TargetRestrictions.
pub fn has_candidates(game: &GameState, player: PlayerId, ability: &str) -> bool {
    let params = crate::trigger::parse_pipe_params(ability);
    match TargetRestrictions::new(&params) {
        Some(tr) => tr.has_candidates(game, player),
        None => true, // No targeting = always valid
    }
}

/// Check if there is at least one valid target for every ability in the
/// SubAbility$ chain. Mirrors Java's target validation in `setupTargets()`
/// which checks each ability in the chain has at least one legal target.
pub fn has_candidates_in_chain(
    game: &GameState,
    player: PlayerId,
    ability: &str,
    source: Option<CardId>,
) -> bool {
    if !has_candidates(game, player, ability) {
        return false;
    }

    let params = crate::trigger::parse_pipe_params(ability);
    if let Some(sub_svar_name) = params.get("SubAbility") {
        if let Some(card_id) = source {
            if let Some(sub_text) = game.card(card_id).svars.get(sub_svar_name) {
                let sub_text = sub_text.clone();
                return has_candidates_in_chain(game, player, &sub_text, source);
            }
        }
    }

    true
}

/// Get all creatures on the battlefield (any player).
/// Part of `TargetRestrictions.getAllCandidates()` for creature targets.
pub fn get_all_candidates_creatures(game: &GameState) -> Vec<CardId> {
    let mut creatures = Vec::new();
    for &pid in &game.player_order {
        for &cid in game.cards_in_zone(ZoneType::Battlefield, pid) {
            if game.card(cid).is_creature() {
                creatures.push(cid);
            }
        }
    }
    creatures
}

/// Get creatures matching an optional filter (e.g. "nonBlack", "OppCtrl").
/// Mirrors Java's `TargetRestrictions.getAllCandidates()` with card property filtering.
pub fn get_all_candidates_creature_filtered(
    game: &GameState,
    filter: Option<&str>,
    source_controller: PlayerId,
) -> Vec<CardId> {
    let all = get_all_candidates_creatures(game);
    match filter {
        None => all,
        Some(f) => all
            .into_iter()
            .filter(|&cid| card_property::card_has_property(game.card(cid), f, source_controller))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_targets_any() {
        assert_eq!(
            parse_valid_targets("SP$ DealDamage | ValidTgts$ Any | NumDmg$ 3"),
            TargetKind::Any
        );
    }

    #[test]
    fn parse_valid_targets_creature_filter() {
        assert_eq!(
            parse_valid_targets("SP$ Destroy | ValidTgts$ Creature.nonBlack"),
            TargetKind::Creature(Some("nonBlack".to_string()))
        );
    }

    #[test]
    fn parse_valid_targets_creature_no_filter() {
        assert_eq!(
            parse_valid_targets("SP$ Destroy | ValidTgts$ Creature"),
            TargetKind::Creature(None)
        );
    }

    #[test]
    fn parse_valid_targets_player() {
        assert_eq!(
            parse_valid_targets("SP$ Draw | ValidTgts$ Player"),
            TargetKind::Player
        );
    }

    #[test]
    fn target_restrictions_from_params() {
        let mut params = BTreeMap::new();
        params.insert("ValidTgts".into(), "Creature.OppCtrl".into());
        let tr = TargetRestrictions::new(&params).unwrap();
        assert_eq!(tr.target_kind, TargetKind::Creature(Some("OppCtrl".into())));
        assert_eq!(tr.min_targets, 1);
        assert_eq!(tr.max_targets, 1);
    }

    #[test]
    fn no_valid_tgts_returns_none() {
        let params = BTreeMap::new();
        assert!(TargetRestrictions::new(&params).is_none());
    }
}
