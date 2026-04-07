use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
    parsing::Params,
    spellability::SpellAbility,
};

use super::trigger::TriggerMode;

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    game: &GameState,
    host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    let _ = (params, game, host_card, host_controller);
    if let TriggerMode::BlockersDeclared = mode {
        return true;
    }
    panic!("Expected BlockersDeclared mode");
}

pub fn parse_mode(_params: &Params) -> TriggerMode {
    TriggerMode::BlockersDeclared
}

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    if let Some(blockers) = params.blocker_ids.as_ref() {
        let csv = blockers
            .iter()
            .map(|c| c.0.to_string())
            .collect::<Vec<_>>()
            .join(",");
        sa.add_triggering_object("Blockers", &csv);
    }
    if let Some(attackers) = params.attacker_ids.as_ref() {
        let csv = attackers
            .iter()
            .map(|c| c.0.to_string())
            .collect::<Vec<_>>()
            .join(",");
        sa.add_triggering_object("Attackers", &csv);
    }
}

pub fn get_important_stack_objects(sa: &SpellAbility) -> String {
    format!(
        "Blockers: {}",
        sa.get_triggering_object("Blockers").unwrap_or("")
    )
}
