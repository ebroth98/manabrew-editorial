//! Extracts a normalized [`StateSnapshot`] from a [`GameState`].
//!
//! The snapshot uses only card names (not engine-internal IDs) and sorts all
//! lists alphabetically so that two independently-running engines can be
//! compared field-by-field.

use std::collections::BTreeMap;
use std::time::Instant;

use forge_engine_core::card::CounterType;
use forge_engine_core::game::GameState;
use forge_engine_core::ids::PlayerId;
use forge_foundation::ZoneType;

use crate::perf;
use crate::protocol::{CardSnapshot, PlayerSnapshot, StateSnapshot};

/// Extract a normalized snapshot from the current game state.
pub fn snapshot_game(game: &GameState) -> StateSnapshot {
    let t_total = Instant::now();
    let mut players = Vec::new();
    let t_players = Instant::now();
    for player in &game.players {
        players.push(snapshot_player(game, player.id));
    }
    perf::record("snapshot_game.players", t_players.elapsed());

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
    perf::record("snapshot_game.total", t_total.elapsed());

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
    let bf_cards = game.cards_in_zone(ZoneType::Battlefield, pid);
    let mut battlefield: Vec<CardSnapshot> = bf_cards
        .iter()
        .map(|&cid| {
            let card = game.card(cid);
            let counters = snapshot_counters(card);
            CardSnapshot {
                name: if card.face_down {
                    String::new()
                } else {
                    card.card_name.clone()
                },
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
                summoning_sick: card.summoning_sick && !card.has_haste(),
                counters,
                controller: card.controller.0,
            }
        })
        .collect();
    battlefield.sort_by(|a, b| {
        a.name
            .cmp(&b.name)
            .then_with(|| a.power.cmp(&b.power))
            .then_with(|| a.toughness.cmp(&b.toughness))
            .then_with(|| {
                // Compare counters using Java TreeMap.toString() format so sort
                // order matches Java's `Comparator.comparing(... counters.toString())`.
                // Java format: "{key1=val1, key2=val2}" (BTreeMap order, comma-space separated).
                let a_str = counters_to_java_string(&a.counters);
                let b_str = counters_to_java_string(&b.counters);
                a_str.cmp(&b_str)
            })
            .then_with(|| a.tapped.cmp(&b.tapped))
            .then_with(|| a.damage.cmp(&b.damage))
            .then_with(|| a.summoning_sick.cmp(&b.summoning_sick))
            .then_with(|| a.controller.cmp(&b.controller))
    });

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

/// Format a counter map as Java's `TreeMap.toString()`, e.g. `"{-1/-1=1, +1/+1=2}"` or `"{}"`.
/// BTreeMap iteration is already sorted by key, matching Java's TreeMap.
fn counters_to_java_string(counters: &BTreeMap<String, i32>) -> String {
    if counters.is_empty() {
        return "{}".to_string();
    }
    let entries: Vec<String> = counters
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect();
    format!("{{{}}}", entries.join(", "))
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
