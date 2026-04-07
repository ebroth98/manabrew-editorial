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
    let valid_blocked = params.get_cloned(keys::VALID_BLOCKED);
    TriggerMode::Blocks {
        valid_card,
        valid_blocked,
    }
}

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    game: &GameState,
    host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    if let TriggerMode::Blocks {
        valid_card,
        valid_blocked,
    } = mode
    {
        return check_card_filter(valid_card, params.blocker, host_card, host_controller, game)
            && check_card_filter(
                valid_blocked,
                params.blocked_attacker,
                host_card,
                host_controller,
                game,
            );
    }
    panic!("Expected Blocks mode");
}

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    if let Some(blocker) = params.blocker {
        sa.add_triggering_object("Blocker", &blocker.0.to_string());
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
        "Blocker: {}",
        sa.get_triggering_object("Blocker").unwrap_or("")
    )
}
