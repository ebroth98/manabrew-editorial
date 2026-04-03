use crate::card::card_damage_map::DamageTarget;
use crate::parsing::{keys, Params};
use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
    spellability::SpellAbility,
};

use super::trigger::{check_card_filter, check_damage_target, TriggerMode};

pub fn parse_mode(params: &Params) -> TriggerMode {
    let valid_source = params.get_cloned(keys::VALID_SOURCE);
    let valid_target = params.get_cloned(keys::VALID_TARGET);
    let combat_damage_only = params.is_true(keys::COMBAT_DAMAGE);
    TriggerMode::DamageDealtOnce {
        valid_source,
        valid_target,
        combat_damage_only,
    }
}

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    game: &GameState,
    host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    if let TriggerMode::DamageDealtOnce {
        valid_source,
        valid_target,
        combat_damage_only,
    } = mode
    {
        if *combat_damage_only && params.is_combat_damage != Some(true) {
            return false;
        }
        return check_card_filter(
            valid_source,
            params.damage_source,
            host_card,
            host_controller,
            game,
        ) && check_damage_target(
            valid_target,
            params,
            host_card,
            host_controller,
            game,
            true,
        );
    }
    panic!("Expected DamageDealtOnce mode");
}

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    if let Some(src) = params.damage_source {
        sa.add_triggering_object("Source", &src.0.to_string());
    }
    if let Some(amount) = params.damage_amount {
        sa.add_triggering_object("DamageAmount", &amount.to_string());
    }
    if let Some(card) = params.damage_target_card {
        sa.add_triggering_object("Targets", &card.0.to_string());
    } else if let Some(player) = params.damage_target_player {
        sa.add_triggering_object("Targets", &player.0.to_string());
    }
}

pub fn get_important_stack_objects(sa: &SpellAbility) -> String {
    // Java: "Damage Source: " + Source + ", Damaged: " + Targets + ", Amount: " + DamageAmount
    format!(
        "Damage Source: {}, Damaged: {}, Amount: {}",
        sa.get_triggering_object("Source").unwrap_or(""),
        sa.get_triggering_object("Targets").unwrap_or(""),
        sa.get_triggering_object("DamageAmount").unwrap_or("")
    )
}

/// Returns the total damage amount from the damage map.
/// Java: TriggerDamageDealtOnce.getDamageAmount
///
/// Note: The Java version filters entries by ValidTarget param; this standalone
/// function passes all entries through. Filtering will be added when trigger
/// param context is available.
pub fn get_damage_amount(params: &RunParams) -> i32 {
    match params.damage_map.as_ref() {
        Some(map) => map.total_amount(),
        None => 0,
    }
}

/// Returns the damage target card IDs from the damage map.
/// Java: TriggerDamageDealtOnce.getDamageTargets
///
/// Note: The Java version filters entries by ValidTarget param; this standalone
/// function returns all target card IDs. Filtering will be added when trigger
/// param context is available.
pub fn get_damage_targets(params: &RunParams) -> Vec<CardId> {
    match params.damage_map.as_ref() {
        Some(map) => {
            let mut targets = Vec::new();
            for (_, target, _) in map.entries() {
                if let DamageTarget::Card(cid) = target {
                    if !targets.contains(&cid) {
                        targets.push(cid);
                    }
                }
            }
            targets
        }
        None => Vec::new(),
    }
}
