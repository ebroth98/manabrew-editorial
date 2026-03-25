use super::trigger::{check_card_filter, TriggerMode};
use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
};

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    game: &GameState,
    host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    let TriggerMode::PayCumulativeUpkeep { valid_card, paid } = mode else {
        panic!("Expected PayCumulativeUpkeep mode");
    };

    if let Some(expected_paid) = paid {
        if params.cumulative_upkeep_paid != Some(*expected_paid) {
            return false;
        }
    }

    check_card_filter(valid_card, params.card, host_card, host_controller, game)
}
