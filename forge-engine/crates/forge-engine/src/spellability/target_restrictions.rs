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
    /// Card in a specific zone with optional filter (e.g. Raise Dead from graveyard)
    CardInZone {
        zone: ZoneType,
        filter: Option<String>,
    },
    /// Spell on the stack (for Counter effects, e.g. "ValidTgts$ Spell")
    Spell,
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
    /// Additional target type filter (e.g. "Spell" from TargetType$ parameter)
    pub target_type_filter: Option<String>,
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
        let valid_tgts: Vec<String> = valid_tgts_str
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();
        let mut target_kind = parse_target_kind(&valid_tgts[0]);
        let min_targets = params
            .get("TargetMin")
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);
        let max_targets = params
            .get("TargetMax")
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);

        // Parse TargetType$ parameter if present (used by counterspells)
        let target_type_filter = params.get("TargetType").cloned();

        // If TargetType$ Spell is specified, override to Spell targeting
        // This handles cases like Counterspell: "ValidTgts$ Card | TargetType$ Spell"
        if let Some(ref target_type) = target_type_filter {
            if target_type.eq_ignore_ascii_case("Spell") {
                target_kind = TargetKind::Spell;
            }
        }

        Some(TargetRestrictions {
            valid_tgts,
            target_kind,
            target_type_filter,
            min_targets,
            max_targets,
            tgt_zone: vec![ZoneType::Battlefield],
        })
    }

    /// Check if there is at least one valid target candidate.
    /// Accounts for Hexproof, Shroud, and Protection when `source_card` is provided.
    /// Mirrors Java's `TargetRestrictions.hasCandidates()`.
    pub fn has_candidates(
        &self,
        game: &GameState,
        player: PlayerId,
        source_card: Option<CardId>,
    ) -> bool {
        match &self.target_kind {
            TargetKind::None => true,
            // "target player" = any alive player (including the caster themselves).
            TargetKind::Player => !game.alive_players().is_empty(),
            // "any target" = any alive player or any creature on the battlefield.
            TargetKind::Any => {
                if !game.alive_players().is_empty() {
                    return true;
                }
                get_all_candidates_creatures(game)
                    .into_iter()
                    .any(|cid| can_be_targeted_by(game, cid, player, source_card))
            }
            TargetKind::Creature(ref filter) => {
                get_all_candidates_creature_filtered(game, filter.as_deref(), player)
                    .into_iter()
                    .any(|cid| can_be_targeted_by(game, cid, player, source_card))
            }
            TargetKind::CardInZone { zone, filter } => {
                has_valid_target_in_zone(game, player, *zone, filter.as_deref())
            }
            TargetKind::Spell => {
                // If we have a TargetType$ filter, apply it
                if let Some(ref filter) = self.target_type_filter {
                    has_valid_spell_with_filter(game, filter)
                } else {
                    !game.stack.is_empty()
                }
            }
        }
    }
}

/// Check if there are valid spells on the stack matching the TargetType$ filter.
pub fn has_valid_spell_with_filter(game: &GameState, filter: &str) -> bool {
    // For now, we only support "Spell" filter which matches all spells
    // In the future, we could filter by spell type (e.g., "Creature", "Instant", "Sorcery")
    if filter.eq_ignore_ascii_case("Spell") {
        // Look for any spell on the stack (abilities are not spells)
        game.stack.iter().any(|entry| entry.spell_ability.is_spell)
    } else {
        // Unknown filter, fall back to checking if stack is not empty
        !game.stack.is_empty()
    }
}

/// Filter stack entries to only include spells matching the TargetType$ filter.
pub fn filter_spells_by_type(game: &GameState, candidates: &[u32], filter: &str) -> Vec<u32> {
    if filter.eq_ignore_ascii_case("Spell") {
        // Only include entries that are actual spells (not abilities)
        candidates
            .iter()
            .filter(|&&id| {
                game.stack
                    .iter()
                    .any(|entry| entry.id == id && entry.spell_ability.is_spell)
            })
            .cloned()
            .collect()
    } else {
        // Unknown filter, return all candidates
        candidates.to_vec()
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
    if val.eq_ignore_ascii_case("Spell") {
        return TargetKind::Spell;
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
/// Enhanced version that also considers `Origin$` for zone targeting.
/// Convenience wrapper for code that doesn't have parsed params yet.
pub fn parse_valid_targets(ability: &str) -> TargetKind {
    let params = crate::trigger::parse_pipe_params(ability);
    let origin_zone = params.get("Origin").and_then(|v| parse_zone_type(v));
    match params.get("ValidTgts") {
        Some(val) => parse_target_kind_enhanced(val, origin_zone),
        None => TargetKind::None,
    }
}

/// Check if there is at least one valid target for the given ability string.
/// Convenience wrapper that creates a temporary TargetRestrictions.
pub fn has_candidates(
    game: &GameState,
    player: PlayerId,
    ability: &str,
    source: Option<CardId>,
) -> bool {
    let params = crate::trigger::parse_pipe_params(ability);
    match TargetRestrictions::new(&params) {
        Some(tr) => tr.has_candidates(game, player, source),
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
    if !has_candidates(game, player, ability, source) {
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

/// Check if a card can be targeted by a spell/ability controlled by `source_controller`.
/// Mirrors Java's `Card.canBeTargetedBy(SpellAbility)` which delegates to
/// `StaticAbilityCantTarget` for Hexproof, Shroud, and Protection checks.
pub fn can_be_targeted_by(
    game: &GameState,
    target_id: CardId,
    source_controller: PlayerId,
    source_card: Option<CardId>,
) -> bool {
    let target = game.card(target_id);
    // Shroud: can't be targeted by anyone
    if target.has_shroud() {
        return false;
    }
    // Hexproof: can't be targeted by opponents
    if target.has_hexproof() && target.controller != source_controller {
        return false;
    }
    if let Some(src_id) = source_card {
        let src = game.card(src_id);
        // Check "Hexproof from <color>"
        if target.controller != source_controller {
            for color in &["white", "blue", "black", "red", "green"] {
                if target.has_hexproof_from(color) {
                    let has_color = match *color {
                        "white" => src.color.has_white(),
                        "blue" => src.color.has_blue(),
                        "black" => src.color.has_black(),
                        "red" => src.color.has_red(),
                        "green" => src.color.has_green(),
                        _ => false,
                    };
                    if has_color {
                        return false;
                    }
                }
            }
        }
        // Protection: can't be targeted by matching sources
        if target.is_protected_from(src) {
            return false;
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

// ── Zone-aware targeting for cards like Raise Dead ───────────────────

/// Parse a zone type string ("Graveyard", "Hand", "Battlefield", etc.)
fn parse_zone_type(s: &str) -> Option<ZoneType> {
    match s.to_lowercase().as_str() {
        "graveyard" => Some(ZoneType::Graveyard),
        "hand" => Some(ZoneType::Hand),
        "battlefield" => Some(ZoneType::Battlefield),
        "library" => Some(ZoneType::Library),
        "exile" => Some(ZoneType::Exile),
        "command" => Some(ZoneType::Command),
        _ => None,
    }
}

/// Enhanced parser that considers Origin$ parameter for zone targeting.
/// This parser handles both legacy battlefield targeting and zone-aware targeting
/// (e.g., Raise Dead with Origin$ Graveyard).
fn parse_target_kind_enhanced(val: &str, origin_zone: Option<ZoneType>) -> TargetKind {
    let val = val.trim();

    // Handle the special case of CardInZone targeting first
    if let Some(zone) = origin_zone {
        if zone != ZoneType::Battlefield {
            // If we have a non-battlefield origin, this is zone targeting
            // The filter might be in the ValidTgts$ (e.g., "Creature.YouCtrl")
            let filter = if val.starts_with("Creature") {
                let filter_part = val.strip_prefix("Creature").unwrap();
                let filter_part = filter_part.strip_prefix('.').unwrap_or(filter_part);
                if filter_part.is_empty() {
                    None
                } else {
                    Some(filter_part.to_string())
                }
            } else if val.contains('.') {
                // Handle formats like "Creature.YouCtrl" directly
                Some(val.to_string())
            } else {
                None
            };
            return TargetKind::CardInZone { zone, filter };
        }
    }

    // For battlefield targeting (or no origin specified), use traditional parsing
    parse_target_kind_legacy(val)
}

/// Legacy parser for battlefield-targeting spells (Unsummon, Doom Blade, etc.)
fn parse_target_kind_legacy(val: &str) -> TargetKind {
    let val = val.trim();
    if val.eq_ignore_ascii_case("Any") {
        return TargetKind::Any;
    }
    if val.eq_ignore_ascii_case("Player") {
        return TargetKind::Player;
    }
    if val.eq_ignore_ascii_case("Spell") {
        return TargetKind::Spell;
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

/// Get all cards in a zone matching the filter (for Raise Dead style targeting)
pub fn get_valid_cards_in_zone(
    game: &GameState,
    zone: ZoneType,
    player: PlayerId,
    filter: Option<&str>,
) -> Vec<CardId> {
    let zone_cards = game.cards_in_zone(zone, player).to_vec();

    match filter {
        None => zone_cards,
        Some(f) => zone_cards
            .into_iter()
            .filter(|&cid| card_property::card_has_property(game.card(cid), f, player))
            .collect(),
    }
}

/// Get all stack entry IDs for spells that can be countered.
/// Mirrors Java's `TargetRestrictions.getAllCandidates()` for Spell targets.
pub fn get_all_candidates_spells(game: &GameState) -> Vec<u32> {
    game.stack.iter().map(|e| e.id).collect()
}

/// Check if there are valid targets in a specific zone.
pub fn has_valid_target_in_zone(
    game: &GameState,
    player: PlayerId,
    zone: ZoneType,
    filter: Option<&str>,
) -> bool {
    !get_valid_cards_in_zone(game, zone, player, filter).is_empty()
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
    fn parse_valid_targets_graveyard_creature() {
        // Test parsing for Raise Dead style: ValidTgts$ Creature with Origin$ Graveyard
        let ability =
            "SP$ ChangeZone | Origin$ Graveyard | Destination$ Hand | ValidTgts$ Creature.YouCtrl";
        let target_kind = parse_valid_targets(ability);
        assert!(matches!(
            target_kind,
            TargetKind::CardInZone {
                zone: ZoneType::Graveyard,
                ..
            }
        ));
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
