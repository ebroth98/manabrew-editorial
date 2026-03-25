use crate::parsing::{keys, Params};
use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
};

use super::trigger::{check_card_filter, check_counter_type_filter, TriggerMode};

pub fn parse_mode(params: &Params) -> TriggerMode {
    let valid_card = params.get_cloned(keys::VALID_CARD);
    let counter_type = params.get_cloned(keys::COUNTER_TYPE);
    TriggerMode::CounterAdded {
        valid_card,
        counter_type,
    }
}

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    game: &GameState,
    host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    if let TriggerMode::CounterAdded {
        valid_card,
        counter_type,
    } = mode
    {
        return check_card_filter(valid_card, params.card, host_card, host_controller, game)
            && check_counter_type_filter(counter_type, &params.counter_type);
    }
    panic!("Expected CounterAdded mode");
}
