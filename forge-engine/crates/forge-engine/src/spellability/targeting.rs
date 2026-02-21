//! Targeting system for spells and abilities.
//!
//! Handles parsing ValidTgts$ from ability text, checking creature filters,
//! and prompting agents for target selection.

use forge_foundation::{ColorSet, ZoneType};

use crate::agent::{PlayerAgent, TargetChoice};
use crate::card::CardInstance;
use crate::game::GameState;
use crate::ids::{CardId, PlayerId};
use crate::mana::ManaPool;

/// What kinds of targets a spell can select.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetKind {
    /// Player only (e.g. "ValidTgts$ Player")
    Player,
    /// Any player or creature (e.g. "ValidTgts$ Any")
    Any,
    /// Creature with optional filter (e.g. "ValidTgts$ Creature.nonBlack")
    Creature(Option<String>),
    /// Card in a specific zone with optional filter
    CardInZone {
        zone: ZoneType,
        filter: Option<String>,
    },
    /// No targets
    None,
}

/// Parse ValidTgts$ and Origin$ from an ability string.
pub fn parse_valid_targets(ability: &str) -> TargetKind {
    let mut origin_zone: Option<ZoneType> = None;
    let mut creature_filter: Option<String> = None;
    
    for part in ability.split('|') {
        let part = part.trim();
        if let Some(val) = part.strip_prefix("ValidTgts$") {
            let val = val.trim();
            if val.eq_ignore_ascii_case("Any") {
                return TargetKind::Any;
            } else if val.eq_ignore_ascii_case("Player") {
                return TargetKind::Player;
            } else if val.starts_with("Creature") {
                // e.g. "Creature.nonBlack" or just "Creature"
                let filter = val.strip_prefix("Creature").unwrap();
                if filter.is_empty() {
                    creature_filter = None;
                } else {
                    // Strip leading dot if present
                    let filter = filter.strip_prefix('.').unwrap_or(filter);
                    creature_filter = Some(filter.to_string());
                }
            }
            // Check for CardInZone patterns like "Creature.YouCtrl"
            else if val.contains(".") && !val.starts_with("Creature.") {
                // This might be a zone-based target like "Creature.YouCtrl" from graveyard
                // We'll handle this in combination with Origin
                creature_filter = Some(val.to_string());
            }
        } else if let Some(val) = part.strip_prefix("Origin$") {
            let val = val.trim();
            origin_zone = parse_zone_type(val);
        }
    }
    
    // If we have an origin zone specified that's NOT battlefield, convert to CardInZone
    if let Some(zone) = origin_zone {
        if zone != ZoneType::Battlefield {
            return TargetKind::CardInZone { 
                zone, 
                filter: creature_filter 
            };
        }
    }
    
    // Otherwise return Creature targeting (or None if no target)
    match creature_filter {
        Some(filter) => TargetKind::Creature(Some(filter)),
        None => {
            // Check if we ever found a Creature target without a specific filter
            if ability.contains("ValidTgts$") && ability.contains("Creature") {
                TargetKind::Creature(None)
            } else {
                TargetKind::None
            }
        }
    }
}

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

/// Check if a creature matches a filter string like "nonBlack", "nonWhite", etc.
pub fn matches_creature_filter(card: &CardInstance, filter: &str) -> bool {
    let lower = filter.to_ascii_lowercase();
    if let Some(color_name) = lower.strip_prefix("non") {
        let excluded = ColorSet::from_names(color_name);
        !card.color.shares_color_with(excluded)
    } else {
        // No recognized filter — match everything
        true
    }
}

/// Check if a card matches a target filter (e.g. "YouCtrl", "OpponentCtrl")
pub fn matches_card_filter(card: &CardInstance, filter: &str, controller: PlayerId, owner: PlayerId) -> bool {
    match filter {
        "YouCtrl" => card.controller == controller,
        "OpponentCtrl" => card.controller != controller,
        _ => matches_creature_filter(card, filter),
    }
}

/// Get all cards in a zone matching the filter
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
            .filter(|&cid| {
                let card = game.card(cid);
                matches_card_filter(card, f, player, player)
            })
            .collect(),
    }
}

/// Get all creatures on the battlefield (any player).
pub fn get_all_battlefield_creatures(game: &GameState) -> Vec<CardId> {
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

/// Get creatures matching an optional filter (e.g. "nonBlack").
pub fn get_valid_creature_targets(game: &GameState, filter: Option<&str>) -> Vec<CardId> {
    let all = get_all_battlefield_creatures(game);
    match filter {
        None => all,
        Some(f) => all
            .into_iter()
            .filter(|&cid| {
                matches_creature_filter(game.card(cid), f)
            })
            .collect(),
    }
}

/// Check if there is at least one valid target for the given ability.
/// Used to determine if a spell is playable.
pub fn has_valid_target(game: &GameState, player: PlayerId, ability: &str) -> bool {
    let target_kind = parse_valid_targets(ability);
    match target_kind {
        TargetKind::None => true,
        TargetKind::Player => {
            game.alive_players().into_iter().any(|p| p != player)
        }
        TargetKind::Any => {
            let has_opponent = game.alive_players().into_iter().any(|p| p != player);
            if has_opponent {
                true
            } else {
                !get_all_battlefield_creatures(game).is_empty()
            }
        }
        TargetKind::Creature(ref filter) => {
            !get_valid_creature_targets(game, filter.as_deref()).is_empty()
        }
        TargetKind::CardInZone { zone, filter } => {
            !get_valid_cards_in_zone(game, zone, player, filter.as_deref()).is_empty()
        }
    }
}

/// Prompt the agent to choose targets for an ability.
/// Returns (target_player, target_card).
pub fn choose_targets(
    game: &GameState,
    agents: &mut [Box<dyn PlayerAgent>],
    mana_pools: &[ManaPool],
    player: PlayerId,
    ability: &str,
) -> (Option<PlayerId>, Option<CardId>) {
    let mut target_player = None;
    let mut target_card: Option<CardId> = None;

    let target_kind = parse_valid_targets(ability);
    match target_kind {
        TargetKind::None => {}
        TargetKind::Player => {
            agents[player.index()].snapshot_state(game, mana_pools);
            let agent = &mut agents[player.index()];
            let opponents: Vec<PlayerId> = game
                .alive_players()
                .into_iter()
                .filter(|&p| p != player)
                .collect();
            target_player = agent.choose_target_player(player, &opponents);
        }
        TargetKind::Any => {
            let opponents: Vec<PlayerId> = game
                .alive_players()
                .into_iter()
                .filter(|&p| p != player)
                .collect();
            let valid_creatures = get_all_battlefield_creatures(game);
            agents[player.index()].snapshot_state(game, mana_pools);
            let agent = &mut agents[player.index()];
            match agent.choose_target_any(player, &opponents, &valid_creatures) {
                TargetChoice::Player(pid) => target_player = Some(pid),
                TargetChoice::Card(cid) => target_card = Some(cid),
                TargetChoice::None => {}
            }
        }
        TargetKind::Creature(ref filter) => {
            let valid = get_valid_creature_targets(game, filter.as_deref());
            agents[player.index()].snapshot_state(game, mana_pools);
            let agent = &mut agents[player.index()];
            target_card = agent.choose_target_card(player, &valid);
        }
        TargetKind::CardInZone { zone, filter } => {
            let valid = get_valid_cards_in_zone(game, zone, player, filter.as_deref());
            agents[player.index()].snapshot_state(game, mana_pools);
            let agent = &mut agents[player.index()];
            target_card = agent.choose_target_card_from_zone(player, zone, &valid);
        }
    }

    (target_player, target_card)
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
        // Test parsing for Raise Dead style: ValidTgts$ Creature.YouCtrl with Origin$ Graveyard
        let ability = "SP$ ChangeZone | Origin$ Graveyard | Destination$ Hand | ValidTgts$ Creature.YouCtrl";
        let target_kind = parse_valid_targets(ability);
        assert!(matches!(target_kind, TargetKind::CardInZone { zone: ZoneType::Graveyard, .. }));
    }

    #[test]
    fn creature_filter_non_black() {
        use forge_foundation::ManaCost;

        let black_creature = CardInstance::new(
            CardId(0),
            "Doom".to_string(),
            PlayerId(0),
            forge_foundation::CardTypeLine::parse("Creature - Zombie"),
            ManaCost::parse("1 B"),
            ColorSet::BLACK,
            Some(2),
            Some(2),
            vec![],
            vec![],
        );
        let green_creature = CardInstance::new(
            CardId(1),
            "Bear".to_string(),
            PlayerId(0),
            forge_foundation::CardTypeLine::parse("Creature - Bear"),
            ManaCost::parse("1 G"),
            ColorSet::GREEN,
            Some(2),
            Some(2),
            vec![],
            vec![],
        );
        assert!(!matches_creature_filter(&black_creature, "nonBlack"));
        assert!(matches_creature_filter(&green_creature, "nonBlack"));
    }

    #[test]
    fn matches_card_filter_you_ctrl() {
        use forge_foundation::ManaCost;

        let mut card = CardInstance::new(
            CardId(0),
            "Bear".to_string(),
            PlayerId(0),
            forge_foundation::CardTypeLine::parse("Creature - Bear"),
            ManaCost::parse("1 G"),
            ColorSet::GREEN,
            Some(2),
            Some(2),
            vec![],
            vec![],
        );
        card.controller = PlayerId(0);
        
        assert!(matches_card_filter(&card, "YouCtrl", PlayerId(0), PlayerId(0)));
        assert!(!matches_card_filter(&card, "YouCtrl", PlayerId(1), PlayerId(0)));
    }
}
