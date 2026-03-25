use super::trigger::{check_card_filter, matches_valid_sa, TriggerMode};
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
    let TriggerMode::AbilityResolves {
        valid_spell_ability,
        valid_source,
    } = mode
    else {
        panic!("Expected AbilityResolves mode");
    };
    let Some(sa) = params.spell_ability.as_ref() else {
        return false;
    };
    if let Some(filter) = valid_spell_ability {
        if !matches_valid_sa(filter, sa) {
            return false;
        }
    }
    check_card_filter(valid_source, params.card, host_card, host_controller, game)
}
