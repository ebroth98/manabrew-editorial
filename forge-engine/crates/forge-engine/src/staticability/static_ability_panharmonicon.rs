use forge_foundation::ZoneType;

use crate::card::{valid_filter, Card};
use crate::event::RunParams;
use crate::game::GameState;
use crate::ids::CardId;
use crate::parsing::{keys, CompiledSelector};
use crate::trigger::Trigger;
use crate::trigger::TriggerType;

pub fn extra_triggers(
    game: &GameState,
    trigger_host: CardId,
    trigger: &Trigger,
    _run_params: &RunParams,
) -> i32 {
    let mut n = 0;
    let trig_host = game.card(trigger_host);
    for source in game
        .cards
        .iter()
        .filter(|c| c.zone == ZoneType::Battlefield)
    {
        for st_ab in source
            .static_abilities
            .iter()
            .filter(|sa| sa.mode == crate::staticability::StaticMode::Panharmonicon)
        {
            if let Some(valid_card) = st_ab.params.selector(keys::VALID_CARD) {
                if !matches_valid_card(valid_card, trig_host, source) {
                    continue;
                }
            }
            if let Some(valid_mode) = st_ab.params.get(keys::VALID_MODE) {
                let trig_mode = trigger.kind.name();
                if !valid_mode
                    .split(',')
                    .map(|s| s.trim())
                    .any(|m| m.eq_ignore_ascii_case(trig_mode))
                {
                    continue;
                }
            }
            let valid_zones = st_ab.params.zone_types(keys::VALID_ZONE);
            if !valid_zones.is_empty() {
                if !valid_zones.contains(&trig_host.zone) {
                    continue;
                }
            }
            if !mode_specific_matches(st_ab, trigger, _run_params, game, source.controller) {
                continue;
            }
            n += 1;
        }
    }
    n
}

pub fn handle_panharmonicon(
    game: &GameState,
    trigger_host: CardId,
    trigger: &Trigger,
    run_params: &RunParams,
) -> i32 {
    extra_triggers(game, trigger_host, trigger, run_params)
}

pub fn apply_panharmonicon_ability(
    st_ab: &crate::staticability::StaticAbility,
    source: &Card,
    trigger_host: &Card,
) -> bool {
    if let Some(valid_card) = st_ab.params.selector(keys::VALID_CARD) {
        if !matches_valid_card(valid_card, trigger_host, source) {
            return false;
        }
    }
    true
}

fn mode_specific_matches(
    st_ab: &crate::staticability::StaticAbility,
    trigger: &Trigger,
    run_params: &RunParams,
    game: &GameState,
    source_controller: crate::ids::PlayerId,
) -> bool {
    match trigger.kind {
        TriggerType::ChangesZone => {
            let origin = trigger.origin_zone();
            let moved = if origin == Some(ZoneType::Battlefield) {
                run_params.card_lki
            } else {
                run_params.card
            };
            if let Some(valid_cause) = st_ab.params.selector(keys::VALID_CAUSE) {
                let Some(cid) = moved else {
                    return false;
                };
                if !matches_valid_card_for_controller(
                    valid_cause,
                    game.card(cid),
                    source_controller,
                ) {
                    return false;
                }
            }
            if let Some(origin_filter) = st_ab.params.get(keys::ORIGIN) {
                if !matches_zone(origin_filter, run_params.origin) {
                    return false;
                }
            }
            if let Some(dest_filter) = st_ab.params.get(keys::DESTINATION) {
                if !matches_zone(dest_filter, run_params.destination) {
                    return false;
                }
            }
            true
        }
        TriggerType::ChangesZoneAll => {
            if let Some(valid_cause) = st_ab.params.selector(keys::VALID_CAUSE) {
                let Some(cause_sa) = run_params.cause.as_ref() else {
                    return false;
                };
                let Some(cause_card) = cause_sa.source else {
                    return false;
                };
                if !matches_valid_card_for_controller(
                    valid_cause,
                    game.card(cause_card),
                    source_controller,
                ) {
                    return false;
                }
            }
            let Some(zone_changes) = run_params.zone_changes.as_ref() else {
                return false;
            };
            let origin = trigger.origin_zone();
            let destination = trigger.destination_zone();
            zone_changes.iter().any(|zc| {
                origin.is_none_or(|expected| zc.origin == expected)
                    && destination.is_none_or(|expected| zc.destination == expected)
            })
        }
        TriggerType::Attacks => {
            if let Some(valid_cause) = st_ab.params.selector(keys::VALID_CAUSE) {
                let Some(attacker) = run_params.attacker else {
                    return false;
                };
                if !matches_valid_card_for_controller(
                    valid_cause,
                    game.card(attacker),
                    source_controller,
                ) {
                    return false;
                }
            }
            true
        }
        TriggerType::SpellCast
        | TriggerType::AbilityCast
        | TriggerType::SpellAbilityCast
        | TriggerType::SpellCastOrCopy
        | TriggerType::SpellCopied
        | TriggerType::SpellCopy
        | TriggerType::SpellAbilityCopy => {
            if let Some(valid_cause) = st_ab.params.selector(keys::VALID_CAUSE) {
                let Some(spell_card) = run_params.spell_card else {
                    return false;
                };
                if !matches_valid_card_for_controller(
                    valid_cause,
                    game.card(spell_card),
                    source_controller,
                ) {
                    return false;
                }
            }
            if let Some(valid_activator) = st_ab.params.selector(keys::VALID_ACTIVATOR) {
                let Some(spell_controller) = run_params.spell_controller else {
                    return false;
                };
                if !matches_valid_player(valid_activator, spell_controller, source_controller) {
                    return false;
                }
            }
            true
        }
        TriggerType::DamageDone | TriggerType::DamageDealtOnce => {
            if let Some(combat_damage) = st_ab.params.get(keys::COMBAT_DAMAGE) {
                let wanted = combat_damage.eq_ignore_ascii_case("True");
                if run_params.is_combat_damage != Some(wanted) {
                    return false;
                }
            }
            if let Some(valid_source) = st_ab.params.selector(keys::VALID_SOURCE) {
                let Some(source_id) = run_params.damage_source else {
                    return false;
                };
                if !matches_valid_card_for_controller(
                    valid_source,
                    game.card(source_id),
                    source_controller,
                ) {
                    return false;
                }
            }
            if let Some(valid_target) = st_ab.params.selector(keys::VALID_TARGET) {
                if let Some(target_card) = run_params.damage_target_card {
                    if !matches_valid_card_for_controller(
                        valid_target,
                        game.card(target_card),
                        source_controller,
                    ) {
                        return false;
                    }
                } else if let Some(target_player) = run_params.damage_target_player {
                    if !matches_valid_player(valid_target, target_player, source_controller) {
                        return false;
                    }
                }
            }
            true
        }
        _ => true,
    }
}

fn matches_zone(filter: &str, zone: Option<ZoneType>) -> bool {
    let Some(zone) = zone else {
        return false;
    };
    match filter.to_ascii_lowercase().as_str() {
        "battlefield" => zone == ZoneType::Battlefield,
        "hand" => zone == ZoneType::Hand,
        "graveyard" => zone == ZoneType::Graveyard,
        "library" => zone == ZoneType::Library,
        "exile" => zone == ZoneType::Exile,
        "stack" => zone == ZoneType::Stack,
        "command" => zone == ZoneType::Command,
        "any" => true,
        _ => true,
    }
}

fn matches_valid_player(
    valid: &CompiledSelector,
    player: crate::ids::PlayerId,
    source_controller: crate::ids::PlayerId,
) -> bool {
    valid_filter::matches_valid_player_selector(valid, player, source_controller)
}

fn matches_valid_card(valid: &CompiledSelector, card: &Card, source: &Card) -> bool {
    valid_filter::matches_valid_card_selector(valid, card, source)
}

fn matches_valid_card_for_controller(
    valid: &CompiledSelector,
    card: &Card,
    source_controller: crate::ids::PlayerId,
) -> bool {
    let mut dummy_source = card.clone();
    dummy_source.controller = source_controller;
    valid_filter::matches_valid_card_selector(valid, card, &dummy_source)
}
