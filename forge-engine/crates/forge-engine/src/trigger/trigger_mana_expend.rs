use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
};

use super::trigger::{check_player_filter, TriggerMode};

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    _game: &GameState,
    _host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    if let TriggerMode::ManaExpend {
        valid_player,
        amount,
    } = mode
    {
        return check_player_filter(valid_player, params.player, host_controller)
            && params.mana_expend_amount == Some(*amount);
    }
    panic!("Expected ManaExpend mode");
}
