use super::trigger::{check_player_filter, TriggerMode};
use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
    spellability::SpellAbility,
};

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    _game: &GameState,
    _host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    let TriggerMode::Discover { valid_player } = mode else {
        panic!("Expected Discover mode");
    };
    check_player_filter(valid_player, params.player, host_controller)
}

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    if let Some(p) = params.player {
        sa.add_triggering_object("Player", &p.0.to_string());
    }
    if let Some(n) = params.num {
        sa.add_triggering_object("Amount", &n.to_string());
    }
}

pub fn get_important_stack_objects(sa: &SpellAbility) -> String {
    format!(
        "Player: {}, Amount: {}",
        sa.get_triggering_object("Player").unwrap_or_default(),
        sa.get_triggering_object("Amount").unwrap_or_default()
    )
}
