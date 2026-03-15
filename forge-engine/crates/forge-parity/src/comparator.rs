//! Snapshot diff engine: compares two [`StateSnapshot`]s and produces a list
//! of [`Divergence`]s describing every field that differs.

use crate::protocol::{CardSnapshot, Divergence, PlayerSnapshot, StateSnapshot};

/// Compare two snapshots and return all divergences.
pub fn compare(index: usize, rust: &StateSnapshot, java: &StateSnapshot) -> Vec<Divergence> {
    let mut divs = Vec::new();
    let turn = rust.turn;
    let phase = rust.phase.clone();

    // Top-level fields
    if rust.turn != java.turn {
        divs.push(divergence(
            index, turn, &phase, "turn", &rust.turn, &java.turn,
        ));
    }
    if rust.phase != java.phase {
        divs.push(divergence(
            index,
            turn,
            &phase,
            "phase",
            &rust.phase,
            &java.phase,
        ));
    }
    if rust.active_player != java.active_player {
        divs.push(divergence(
            index,
            turn,
            &phase,
            "active_player",
            &rust.active_player,
            &java.active_player,
        ));
    }
    if rust.game_over != java.game_over {
        divs.push(divergence(
            index,
            turn,
            &phase,
            "game_over",
            &rust.game_over,
            &java.game_over,
        ));
    }
    if rust.winner != java.winner {
        divs.push(divergence(
            index,
            turn,
            &phase,
            "winner",
            &format!("{:?}", rust.winner),
            &format!("{:?}", java.winner),
        ));
    }
    if rust.stack != java.stack {
        divs.push(divergence(
            index,
            turn,
            &phase,
            "stack",
            &format!("{:?}", rust.stack),
            &format!("{:?}", java.stack),
        ));
    }

    // Per-player comparison
    let max_players = rust.players.len().max(java.players.len());
    for i in 0..max_players {
        let prefix = format!("players[{}]", i);
        match (rust.players.get(i), java.players.get(i)) {
            (Some(rp), Some(jp)) => {
                compare_players(&mut divs, index, turn, &phase, &prefix, rp, jp);
            }
            (Some(_), None) => {
                divs.push(divergence(
                    index,
                    turn,
                    &phase,
                    &format!("{}.exists", prefix),
                    &"present",
                    &"missing",
                ));
            }
            (None, Some(_)) => {
                divs.push(divergence(
                    index,
                    turn,
                    &phase,
                    &format!("{}.exists", prefix),
                    &"missing",
                    &"present",
                ));
            }
            (None, None) => {}
        }
    }

    divs
}

fn compare_players(
    divs: &mut Vec<Divergence>,
    index: usize,
    turn: u32,
    phase: &str,
    prefix: &str,
    rust: &PlayerSnapshot,
    java: &PlayerSnapshot,
) {
    macro_rules! cmp_field {
        ($field:ident) => {
            if rust.$field != java.$field {
                divs.push(divergence(
                    index,
                    turn,
                    phase,
                    &format!("{}.{}", prefix, stringify!($field)),
                    &rust.$field,
                    &java.$field,
                ));
            }
        };
    }

    cmp_field!(name);
    cmp_field!(life);
    cmp_field!(poison);
    cmp_field!(lands_played);
    cmp_field!(has_lost);
    cmp_field!(has_won);
    cmp_field!(library_size);

    // Zone comparisons (sorted card name lists)
    compare_name_list(
        divs,
        index,
        turn,
        phase,
        &format!("{}.graveyard", prefix),
        &rust.graveyard,
        &java.graveyard,
    );
    compare_name_list(
        divs,
        index,
        turn,
        phase,
        &format!("{}.hand", prefix),
        &rust.hand,
        &java.hand,
    );
    compare_name_list(
        divs,
        index,
        turn,
        phase,
        &format!("{}.exile", prefix),
        &rust.exile,
        &java.exile,
    );

    // Battlefield comparison
    compare_battlefield(
        divs,
        index,
        turn,
        phase,
        &format!("{}.battlefield", prefix),
        &rust.battlefield,
        &java.battlefield,
    );
}

fn compare_name_list(
    divs: &mut Vec<Divergence>,
    index: usize,
    turn: u32,
    phase: &str,
    field: &str,
    rust: &[String],
    java: &[String],
) {
    if rust != java {
        divs.push(divergence(
            index,
            turn,
            phase,
            field,
            &format!("{:?}", rust),
            &format!("{:?}", java),
        ));
    }
}

fn compare_battlefield(
    divs: &mut Vec<Divergence>,
    index: usize,
    turn: u32,
    phase: &str,
    prefix: &str,
    rust: &[CardSnapshot],
    java: &[CardSnapshot],
) {
    // Both are sorted by name. Walk in lockstep.
    let max = rust.len().max(java.len());
    if rust.len() != java.len() {
        divs.push(divergence(
            index,
            turn,
            phase,
            &format!("{}.count", prefix),
            &rust.len(),
            &java.len(),
        ));
    }

    for i in 0..max {
        let card_prefix = format!("{}[{}]", prefix, i);
        match (rust.get(i), java.get(i)) {
            (Some(rc), Some(jc)) => {
                if rc.name != jc.name {
                    divs.push(divergence(
                        index,
                        turn,
                        phase,
                        &format!("{}.name", card_prefix),
                        &rc.name,
                        &jc.name,
                    ));
                }
                if rc.tapped != jc.tapped {
                    divs.push(divergence(
                        index,
                        turn,
                        phase,
                        &format!("{}.tapped", card_prefix),
                        &rc.tapped,
                        &jc.tapped,
                    ));
                }
                if rc.power != jc.power {
                    divs.push(divergence(
                        index,
                        turn,
                        phase,
                        &format!("{}.power", card_prefix),
                        &format!("{:?}", rc.power),
                        &format!("{:?}", jc.power),
                    ));
                }
                if rc.toughness != jc.toughness {
                    divs.push(divergence(
                        index,
                        turn,
                        phase,
                        &format!("{}.toughness", card_prefix),
                        &format!("{:?}", rc.toughness),
                        &format!("{:?}", jc.toughness),
                    ));
                }
                if rc.damage != jc.damage {
                    divs.push(divergence(
                        index,
                        turn,
                        phase,
                        &format!("{}.damage", card_prefix),
                        &rc.damage,
                        &jc.damage,
                    ));
                }
                if rc.summoning_sick != jc.summoning_sick {
                    divs.push(divergence(
                        index,
                        turn,
                        phase,
                        &format!("{}.summoning_sick", card_prefix),
                        &rc.summoning_sick,
                        &jc.summoning_sick,
                    ));
                }
                if rc.counters != jc.counters {
                    divs.push(divergence(
                        index,
                        turn,
                        phase,
                        &format!("{}.counters", card_prefix),
                        &format!("{:?}", rc.counters),
                        &format!("{:?}", jc.counters),
                    ));
                }
                if rc.controller != jc.controller {
                    divs.push(divergence(
                        index,
                        turn,
                        phase,
                        &format!("{}.controller", card_prefix),
                        &rc.controller,
                        &jc.controller,
                    ));
                }
            }
            (Some(rc), None) => {
                divs.push(divergence(
                    index,
                    turn,
                    phase,
                    &format!("{}.exists", card_prefix),
                    &rc.name,
                    &"<missing>",
                ));
            }
            (None, Some(jc)) => {
                divs.push(divergence(
                    index,
                    turn,
                    phase,
                    &format!("{}.exists", card_prefix),
                    &"<missing>",
                    &jc.name,
                ));
            }
            (None, None) => {}
        }
    }
}

fn divergence<R: std::fmt::Display, J: std::fmt::Display>(
    snapshot_index: usize,
    turn: u32,
    phase: &str,
    field: &str,
    rust_value: &R,
    java_value: &J,
) -> Divergence {
    Divergence {
        snapshot_index,
        turn,
        phase: phase.to_string(),
        field: field.to_string(),
        rust_value: rust_value.to_string(),
        java_value: java_value.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{PlayerSnapshot, StateSnapshot};
    use std::collections::BTreeMap;

    fn empty_player(name: &str, idx: u32) -> PlayerSnapshot {
        PlayerSnapshot {
            name: name.into(),
            index: idx,
            life: 20,
            poison: 0,
            lands_played: 0,
            has_lost: false,
            has_won: false,
            battlefield: vec![],
            graveyard: vec![],
            hand: vec![],
            exile: vec![],
            library_size: 30,
        }
    }

    fn base_snapshot() -> StateSnapshot {
        StateSnapshot {
            turn: 1,
            phase: "Main1".into(),
            active_player: 0,
            game_over: false,
            winner: None,
            players: vec![empty_player("Alice", 0), empty_player("Bob", 1)],
            stack: vec![],
        }
    }

    #[test]
    fn identical_snapshots_no_divergences() {
        let a = base_snapshot();
        let b = base_snapshot();
        let divs = compare(0, &a, &b);
        assert!(divs.is_empty());
    }

    #[test]
    fn life_difference_detected() {
        let a = base_snapshot();
        let mut b = base_snapshot();
        b.players[0].life = 18;

        let divs = compare(0, &a, &b);
        assert_eq!(divs.len(), 1);
        assert_eq!(divs[0].field, "players[0].life");
        assert_eq!(divs[0].rust_value, "20");
        assert_eq!(divs[0].java_value, "18");
    }

    #[test]
    fn battlefield_card_difference() {
        let mut a = base_snapshot();
        let mut b = base_snapshot();

        a.players[0].battlefield.push(CardSnapshot {
            name: "Mountain".into(),
            tapped: false,
            power: None,
            toughness: None,
            damage: 0,
            summoning_sick: false,
            counters: BTreeMap::new(),
            controller: 0,
        });

        b.players[0].battlefield.push(CardSnapshot {
            name: "Mountain".into(),
            tapped: true, // Different!
            power: None,
            toughness: None,
            damage: 0,
            summoning_sick: false,
            counters: BTreeMap::new(),
            controller: 0,
        });

        let divs = compare(0, &a, &b);
        assert!(!divs.is_empty());
        assert!(divs.iter().any(|d| d.field.contains("tapped")));
    }
}
