use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
};

use super::trigger::{check_card_filter, TriggerMode};

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    game: &GameState,
    host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    if let TriggerMode::AttackerBlockedByCreature {
        valid_card,
        valid_blocked,
    } = mode
    {
        return check_card_filter(valid_card, params.blocker, host_card, host_controller, game)
            && check_card_filter(
                valid_blocked,
                params.blocked_attacker,
                host_card,
                host_controller,
                game,
            );
    }
    panic!("Expected AttackerBlockedByCreature mode");
}
