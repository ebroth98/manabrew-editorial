use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
    spellability::SpellAbility,
};

use super::trigger::{check_player_filter, TriggerMode};

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    _game: &GameState,
    _host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    if let TriggerMode::ManaExpend {
        valid_player,
        amount,
    } = mode
    {
        return check_player_filter(valid_player, params.player, host_controller)
            && params.mana_expend_amount == Some(*amount);
    }
    panic!("Expected ManaExpend mode");
}

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    if let Some(amount) = params.mana_expend_amount {
        sa.add_triggering_object("Amount", &amount.to_string());
    }
    if let Some(p) = params.player {
        sa.add_triggering_object("Player", &p.0.to_string());
    }
}

pub fn get_important_stack_objects(sa: &SpellAbility) -> String {
    format!(
        "{} expended {} mana",
        sa.trigger_objects
            .get("Player")
            .map(|s| s.as_str())
            .unwrap_or(""),
        sa.trigger_objects
            .get("Amount")
            .map(|s| s.as_str())
            .unwrap_or("")
    )
}
