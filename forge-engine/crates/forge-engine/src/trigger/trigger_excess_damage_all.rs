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
    let TriggerMode::ExcessDamageAll {
        valid_target,
        combat_damage_only,
    } = mode
    else {
        panic!("Expected ExcessDamageAll mode");
    };

    if *combat_damage_only && params.is_combat_damage != Some(true) {
        return false;
    }

    let targets = params.cards.as_deref().unwrap_or(&[]);
    if targets.is_empty() {
        return false;
    }
    if let Some(filter) = valid_target {
        return targets.iter().any(|&cid| {
            check_card_filter(
                &Some(filter.clone()),
                Some(cid),
                host_card,
                host_controller,
                game,
            )
        });
    }
    true
}
