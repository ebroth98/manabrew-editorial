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
    if let TriggerMode::RolledDieOnce {
        valid_player,
        valid_result,
        valid_sides,
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
            let Some(result) = params.die_result else {
                return false;
            };
            if !matches_amount(filter, result as usize) {
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
        return true;
    }
    panic!("Expected RolledDieOnce mode");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::RunParams;
    use crate::trigger::TriggerMode;

    #[test]
    fn rolled_die_once_respects_attraction_flag() {
        let mode = TriggerMode::RolledDieOnce {
            valid_player: None,
            valid_result: None,
            valid_sides: None,
            rolled_to_visit_attractions: true,
        };
        let params = RunParams {
            player: Some(PlayerId(0)),
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
