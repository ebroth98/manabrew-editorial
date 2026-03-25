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
    let valid_causer = params.get_cloned("ValidCauser");
    TriggerMode::Destroyed {
        valid_card,
        valid_causer,
    }
}

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    game: &GameState,
    host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    if let TriggerMode::Destroyed {
        valid_card,
        valid_causer,
    } = mode
    {
        return check_card_filter(valid_card, params.card, host_card, host_controller, game)
            && check_card_filter(
                valid_causer,
                params
                    .causer
                    .or(params.cause_card)
                    .or_else(|| params.cause.as_ref().and_then(|sa| sa.source)),
                host_card,
                host_controller,
                game,
            );
    }
    panic!("Expected Destroyed mode");
}

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    if let Some(card_id) = params.card {
        sa.add_triggering_object("Destroyed", &card_id.0.to_string());
    }
}
