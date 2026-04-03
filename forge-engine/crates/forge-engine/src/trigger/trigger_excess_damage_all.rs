use super::trigger::{check_card_filter, TriggerMode};
use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
    spellability::SpellAbility,
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

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    // Java: sa.setTriggeringObject(AbilityKey.Targets, getDamageTargets(DamageTargets))
    // TODO: getDamageTargets filters with ValidTarget — free function has no access to trigger params
    if let Some(cards) = params.cards.as_ref() {
        let csv = cards
            .iter()
            .map(|c| c.0.to_string())
            .collect::<Vec<_>>()
            .join(",");
        sa.add_triggering_object("Targets", &csv);
    }
}

pub fn get_important_stack_objects(sa: &SpellAbility) -> String {
    // Java: "Damaged: " + Targets
    format!(
        "Damaged: {}",
        sa.get_triggering_object("Targets").unwrap_or_default()
    )
}

/// Returns the damage target card IDs from the cards list in RunParams.
/// Java: TriggerExcessDamageAll.getDamageTargets
///
/// Note: The Java version filters entries by ValidTarget param; this standalone
/// function returns all target card IDs. Filtering will be added when trigger
/// param context is available.
pub fn get_damage_targets(params: &RunParams) -> Vec<CardId> {
    params.cards.clone().unwrap_or_default()
}
