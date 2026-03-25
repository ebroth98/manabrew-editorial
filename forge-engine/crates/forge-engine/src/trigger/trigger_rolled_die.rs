use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
};

use super::trigger::{check_player_filter, matches_amount, TriggerMode};

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    _game: &GameState,
    _host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    if let TriggerMode::RolledDie {
        valid_player,
        valid_result,
        valid_sides,
        number,
        natural,
        rolled_to_visit_attractions,
    } = mode
    {
        if !check_player_filter(valid_player, params.player, host_controller) {
            return false;
        }
        if *rolled_to_visit_attractions && params.rolled_to_visit_attractions != Some(true) {
            return false;
        }
        if let Some(filter) = valid_result {
            let result = if *natural {
                params.natural_result
            } else {
                params.die_result
            };
            let Some(result) = result else {
                return false;
            };
            if !matches_die_filter(filter, result, params.die_sides) {
                return false;
            }
        }
        if let Some(filter) = valid_sides {
            let Some(sides) = params.die_sides else {
                return false;
            };
            if !matches_amount(filter, sides as usize) {
                return false;
            }
        }
        if let Some(expected_number) = number {
            if params.number != Some(*expected_number) {
                return false;
            }
        }
        return true;
    }
    panic!("Expected RolledDie mode");
}

fn matches_die_filter(filter: &str, result: i32, sides: Option<i32>) -> bool {
    for entry in filter
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
    {
        if entry.eq_ignore_ascii_case("Highest") {
            if sides == Some(result) {
                return true;
            }
            continue;
        }
        if let Ok(value) = entry.parse::<i32>() {
            if value == result {
                return true;
            }
            continue;
        }
        if entry.len() >= 3 && matches_amount(entry, result.max(0) as usize) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::RunParams;
    use crate::trigger::TriggerMode;

    #[test]
    fn rolled_die_respects_natural_and_number() {
        let mode = TriggerMode::RolledDie {
            valid_player: None,
            valid_result: Some("EQ6".to_string()),
            valid_sides: Some("EQ20".to_string()),
            number: Some(2),
            natural: true,
            rolled_to_visit_attractions: false,
        };
        let params = RunParams {
            player: Some(PlayerId(0)),
            die_result: Some(8),
            natural_result: Some(6),
            die_sides: Some(20),
            number: Some(2),
            ..Default::default()
        };
        assert!(perform_test(
            &mode,
            &params,
            &GameState::new(&["A"], 20),
            CardId(0),
            PlayerId(0)
        ));
    }

    #[test]
    fn rolled_die_respects_attraction_flag() {
        let mode = TriggerMode::RolledDie {
            valid_player: None,
            valid_result: Some("Highest".to_string()),
            valid_sides: Some("EQ6".to_string()),
            number: None,
            natural: false,
            rolled_to_visit_attractions: true,
        };
        let params = RunParams {
            player: Some(PlayerId(0)),
            die_result: Some(6),
            die_sides: Some(6),
            rolled_to_visit_attractions: Some(true),
            ..Default::default()
        };
        assert!(perform_test(
            &mode,
            &params,
            &GameState::new(&["A"], 20),
            CardId(0),
            PlayerId(0)
        ));
    }
}
