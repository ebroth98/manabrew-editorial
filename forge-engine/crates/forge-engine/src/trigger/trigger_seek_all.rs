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
    let TriggerMode::SeekAll { valid_player } = mode else {
        panic!("Expected SeekAll mode");
    };
    check_player_filter(valid_player, params.player, host_controller)
}

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    if let Some(p) = params.player {
        sa.add_triggering_object("Player", &p.0.to_string());
    }
    if let Some(cards) = params.cards.as_ref() {
        let csv = cards
            .iter()
            .map(|c| c.0.to_string())
            .collect::<Vec<_>>()
            .join(",");
        sa.add_triggering_object("Cards", &csv);
    }
}

pub fn get_important_stack_objects(sa: &SpellAbility) -> String {
    format!(
        "Player: {}",
        sa.trigger_objects
            .get("Player")
            .map(|s| s.as_str())
            .unwrap_or("")
    )
}
