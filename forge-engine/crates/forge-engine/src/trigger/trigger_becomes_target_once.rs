use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
    spellability::SpellAbility,
};

use super::trigger::{check_card_filter, TriggerMode};

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    game: &GameState,
    host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    if let TriggerMode::BecomesTargetOnce { valid_card } = mode {
        return check_card_filter(valid_card, params.card, host_card, host_controller, game);
    }
    panic!("Expected BecomesTargetOnce mode");
}

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    // Java: sa.setTriggeringObjectsFrom(runParams, AbilityKey.SourceSA, AbilityKey.Targets);
    // SourceSA is a complex object; we store what we can
    // Java: sa.setTriggeringObject(AbilityKey.Source, ((SpellAbility) runParams.get(AbilityKey.SourceSA)).getHostCard());
    if let Some(ref source_sa) = params.source_sa {
        if let Some(source_card) = source_sa.source {
            sa.add_triggering_object("Source", &source_card.0.to_string());
        }
    } else if let Some(source) = params.source_card {
        sa.add_triggering_object("Source", &source.0.to_string());
    }
    // Targets from the batch targeting event
    if let Some(card) = params.target_card.or(params.card) {
        sa.add_triggering_object("Targets", &card.0.to_string());
    } else if let Some(p) = params.target_player {
        sa.add_triggering_object("Targets", &p.0.to_string());
    }
}

pub fn get_important_stack_objects(sa: &SpellAbility) -> String {
    format!(
        "Source: {}, Targets: {}",
        sa.get_triggering_object("Source").unwrap_or(""),
        sa.get_triggering_object("Targets").unwrap_or("")
    )
}
