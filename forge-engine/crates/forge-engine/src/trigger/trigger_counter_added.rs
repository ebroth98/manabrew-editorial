use crate::parsing::{keys, Params};
use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
    spellability::SpellAbility,
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

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    if let Some(card) = params.card {
        sa.add_triggering_object("Card", &card.0.to_string());
    }
    if let Some(p) = params.player {
        sa.add_triggering_object("Player", &p.0.to_string());
    }
}

pub fn get_important_stack_objects(sa: &SpellAbility) -> String {
    let card = sa.trigger_objects.get("Card");
    let player = sa.trigger_objects.get("Player");
    if let Some(c) = card {
        format!("AddedOnce: {}", c)
    } else if let Some(p) = player {
        format!("AddedOnce: {}", p)
    } else {
        String::new()
    }
}
