use super::trigger::{check_card_filter, TriggerMode};
use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
    spellability::SpellAbility,
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

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    if let Some(card) = params.card {
        sa.add_triggering_object("Card", &card.0.to_string());
    }
    if let Some(mana) = params.produced.as_ref() {
        sa.add_triggering_object("PayingMana", mana);
    }
}

pub fn get_important_stack_objects(sa: &SpellAbility) -> String {
    format!(
        "Mana: {}",
        sa.trigger_objects
            .get("PayingMana")
            .map(|s| s.as_str())
            .unwrap_or("")
    )
}
