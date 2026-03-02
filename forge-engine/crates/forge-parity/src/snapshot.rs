//! Extracts a normalized [`StateSnapshot`] from a [`GameState`].
//!
//! The snapshot uses only card names (not engine-internal IDs) and sorts all
//! lists alphabetically so that two independently-running engines can be
//! compared field-by-field.

use std::collections::BTreeMap;

use forge_engine_core::card::CounterType;
use forge_engine_core::game::GameState;
use forge_engine_core::ids::PlayerId;
use forge_foundation::ZoneType;

use crate::protocol::{CardSnapshot, PlayerSnapshot, StateSnapshot};

/// Extract a normalized snapshot from the current game state.
pub fn snapshot_game(game: &GameState) -> StateSnapshot {
    let mut players = Vec::new();
    for player in &game.players {
        players.push(snapshot_player(game, player.id));
    }

    // Stack: collect card/ability names
    let mut stack: Vec<String> = game
        .stack
        .iter()
        .map(|entry| {
            if let Some(source) = entry.spell_ability.source {
                game.card(source).card_name.clone()
            } else {
                entry.spell_ability.ability_text.clone()
            }
        })
        .collect();
    stack.sort();

    StateSnapshot {
        turn: game.turn.turn_number,
        phase: format!("{:?}", game.turn.phase),
        active_player: game.turn.active_player.0,
        game_over: game.game_over,
        winner: game.winner.map(|p| p.0),
        players,
        stack,
    }
}

fn snapshot_player(game: &GameState, pid: PlayerId) -> PlayerSnapshot {
    let player = game.player(pid);

    // Battlefield cards with full details
    let mut battlefield: Vec<CardSnapshot> = game
        .cards_in_zone(ZoneType::Battlefield, pid)
        .iter()
        .map(|&cid| {
            let card = game.card(cid);
            let counters = snapshot_counters(card);
            CardSnapshot {
                name: card.card_name.clone(),
                tapped: card.tapped,
                power: if card.is_creature() {
                    Some(card.power())
                } else {
                    None
                },
                toughness: if card.is_creature() {
                    Some(card.toughness())
                } else {
                    None
                },
                damage: card.damage,
                summoning_sick: card.summoning_sick,
                counters,
                controller: card.controller.0,
            }
        })
        .collect();
    battlefield.sort_by(|a, b| a.name.cmp(&b.name));

    // Other zones: sorted card names
    let mut graveyard: Vec<String> = game
        .cards_in_zone(ZoneType::Graveyard, pid)
        .iter()
        .map(|&cid| game.card(cid).card_name.clone())
        .collect();
    graveyard.sort();

    let mut hand: Vec<String> = game
        .cards_in_zone(ZoneType::Hand, pid)
        .iter()
        .map(|&cid| game.card(cid).card_name.clone())
        .collect();
    hand.sort();

    let mut exile: Vec<String> = game
        .cards_in_zone(ZoneType::Exile, pid)
        .iter()
        .map(|&cid| game.card(cid).card_name.clone())
        .collect();
    exile.sort();

    let library_size = game.zone(ZoneType::Library, pid).len();

    PlayerSnapshot {
        name: player.name.clone(),
        index: pid.0,
        life: player.life,
        poison: player.poison_counters,
        lands_played: player.lands_played_this_turn,
        has_lost: player.has_lost,
        has_won: player.has_won,
        battlefield,
        graveyard,
        hand,
        exile,
        library_size,
    }
}

/// Convert counter map to sorted string-keyed map for deterministic comparison.
fn snapshot_counters(card: &forge_engine_core::card::CardInstance) -> BTreeMap<String, i32> {
    let mut map = BTreeMap::new();
    for (ct, &count) in &card.counters {
        if count > 0 {
            let name = counter_type_name(ct);
            map.insert(name, count);
        }
    }
    map
}

fn counter_type_name(ct: &CounterType) -> String {
    match ct {
        CounterType::P1P1 => "+1/+1".into(),
        CounterType::M1M1 => "-1/-1".into(),
        CounterType::Loyalty => "loyalty".into(),
        CounterType::Charge => "charge".into(),
        CounterType::Quest => "quest".into(),
        CounterType::Study => "study".into(),
        CounterType::Age => "age".into(),
        CounterType::Fade => "fade".into(),
        CounterType::Time => "time".into(),
        CounterType::Depletion => "depletion".into(),
        CounterType::Storage => "storage".into(),
        CounterType::Mining => "mining".into(),
        CounterType::Brick => "brick".into(),
        CounterType::Level => "level".into(),
        CounterType::Lore => "lore".into(),
        CounterType::Page => "page".into(),
        CounterType::Dream => "dream".into(),
        CounterType::Poison => "poison".into(),
        CounterType::Named(name) => name.to_lowercase(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_engine_core::card::CardInstance;
    use forge_engine_core::ids::CardId;
    use forge_foundation::{CardTypeLine, ColorSet, ManaCost};

    #[test]
    fn snapshot_empty_game() {
        let game = GameState::new(&["Alice", "Bob"], 20);
        let snap = snapshot_game(&game);
        assert_eq!(snap.turn, 1);
        assert_eq!(snap.players.len(), 2);
        assert_eq!(snap.players[0].name, "Alice");
        assert_eq!(snap.players[0].life, 20);
        assert_eq!(snap.players[1].name, "Bob");
        assert!(!snap.game_over);
    }

    #[test]
    fn snapshot_sorts_battlefield() {
        let mut game = GameState::new(&["Alice", "Bob"], 20);
        let p0 = PlayerId(0);

        // Create cards in reverse alphabetical order
        let zephyr = CardInstance::new(
            CardId(0),
            "Zephyr Spirit".into(),
            p0,
            CardTypeLine::parse("Creature Spirit"),
            ManaCost::parse("U"),
            ColorSet::COLORLESS,
            Some(0),
            Some(1),
            vec![],
            vec![],
        );
        let alpha = CardInstance::new(
            CardId(0),
            "Alpha Myr".into(),
            p0,
            CardTypeLine::parse("Creature Myr"),
            ManaCost::parse("2"),
            ColorSet::COLORLESS,
            Some(2),
            Some(1),
            vec![],
            vec![],
        );

        let z_id = game.create_card(zephyr);
        let a_id = game.create_card(alpha);
        game.zone_mut(ZoneType::Battlefield, p0).add(z_id);
        game.zone_mut(ZoneType::Battlefield, p0).add(a_id);

        let snap = snapshot_game(&game);
        assert_eq!(snap.players[0].battlefield[0].name, "Alpha Myr");
        assert_eq!(snap.players[0].battlefield[1].name, "Zephyr Spirit");
    }
}
