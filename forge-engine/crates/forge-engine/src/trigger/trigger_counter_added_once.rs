use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
    spellability::SpellAbility,
};

use super::trigger::{check_card_filter, check_counter_type_filter, TriggerMode};

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    game: &GameState,
    host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    if let TriggerMode::CounterAddedOnce {
        valid_card,
        counter_type,
        valid_source,
    } = mode
    {
        if !check_card_filter(valid_card, params.card, host_card, host_controller, game) {
            return false;
        }
        if !check_counter_type_filter(counter_type, &params.counter_type) {
            return false;
        }
        if let Some(filter) = valid_source {
            if filter.eq_ignore_ascii_case("You") {
                return params.cause_player == Some(host_controller);
            }
        }
        return true;
    }
    panic!("Expected CounterAddedOnce mode");
}

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    if let Some(card) = params.card {
        sa.add_triggering_object("Card", &card.0.to_string());
    }
    if let Some(p) = params.player {
        sa.add_triggering_object("Player", &p.0.to_string());
    }
    if let Some(amount) = params.counter_amount {
        sa.add_triggering_object("Amount", &amount.to_string());
    }
}

pub fn get_important_stack_objects(sa: &SpellAbility) -> String {
    let target = sa
        .trigger_objects
        .get("Card")
        .or(sa.trigger_objects.get("Player"));
    format!(
        "AddedOnce: {}, Amount: {}",
        target.cloned().unwrap_or_default(),
        sa.trigger_objects
            .get("Amount")
            .cloned()
            .unwrap_or_default()
    )
}
