use crate::parsing::{keys, Params};
use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
    spellability::SpellAbility,
};

use super::trigger::{check_card_filter, TriggerMode};

pub fn parse_mode(params: &Params) -> TriggerMode {
    let valid_card = params.get_cloned(keys::VALID_CARD);
    let valid_explored = params.get_cloned("ValidExplored");
    TriggerMode::Explored {
        valid_card,
        valid_explored,
    }
}

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    game: &GameState,
    host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    if let TriggerMode::Explored {
        valid_card,
        valid_explored,
    } = mode
    {
        return check_card_filter(valid_card, params.card, host_card, host_controller, game)
            && check_card_filter(
                valid_explored,
                params.explored,
                host_card,
                host_controller,
                game,
            );
    }
    panic!("Expected Explored mode");
}

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    // Java: sa.setTriggeringObject(AbilityKey.Explorer, runParams.get(AbilityKey.Card));
    //       if (runParams.containsKey(AbilityKey.Explored)) sa.setTriggeringObjectsFrom(runParams, AbilityKey.Explored);
    if let Some(card_id) = params.card {
        sa.add_triggering_object("Explorer", &card_id.0.to_string());
    }
    if let Some(explored) = params.explored {
        sa.add_triggering_object("Explored", &explored.0.to_string());
    }
}

pub fn get_important_stack_objects(sa: &SpellAbility) -> String {
    // Java: "Explorer: " + Explorer + optional ", Explored: " + Explored
    let mut sb = format!(
        "Explorer: {}",
        sa.get_triggering_object("Explorer").unwrap_or_default()
    );
    if let Some(explored) = sa.get_triggering_object("Explored") {
        sb.push_str(&format!(", Explored: {}", explored));
    }
    sb
}
