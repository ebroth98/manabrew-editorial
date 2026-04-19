use forge_foundation::ZoneType;

use crate::card::{valid_filter, Card};
use crate::event::{RunParams, TriggerType};
use crate::game::GameState;
use crate::ids::CardId;
use crate::parsing::keys;
use crate::trigger::Trigger;

pub fn is_disabled(
    game: &GameState,
    trigger_host: CardId,
    regtrig: &Trigger,
    _run_params: &RunParams,
) -> bool {
    let host = game.card(trigger_host);
    for source in game
        .cards
        .iter()
        .filter(|c| c.zone == ZoneType::Battlefield)
    {
        for st_ab in source
            .static_abilities
            .iter()
            .filter(|sa| sa.mode == crate::staticability::StaticMode::DisableTriggers)
        {
            if let Some(valid_mode) = st_ab.params.get(keys::VALID_MODE) {
                let modes = valid_mode.split(',').map(|s| s.trim());
                let trig_mode = regtrig.kind.name();
                if !modes.clone().any(|m| m.eq_ignore_ascii_case(trig_mode)) {
                    continue;
                }
            }
            if let Some(valid_card) = st_ab.params.get(keys::VALID_CARD) {
                if !matches_valid_card(valid_card, host, source) {
                    continue;
                }
            }
            if let Some(valid_trigger) = st_ab.params.get(keys::VALID_TRIGGER) {
                if !trigger_matches(valid_trigger, game, trigger_host, regtrig) {
                    continue;
                }
            }
            if !mode_specific_matches(st_ab, game, regtrig, _run_params, source.controller) {
                continue;
            }
            return true;
        }
    }
    false
}

pub fn disabled(
    game: &GameState,
    trigger_host: CardId,
    regtrig: &Trigger,
    run_params: &RunParams,
) -> bool {
    is_disabled(game, trigger_host, regtrig, run_params)
}

fn mode_specific_matches(
    st_ab: &crate::staticability::StaticAbility,
    game: &GameState,
    regtrig: &Trigger,
    run_params: &RunParams,
    source_controller: crate::ids::PlayerId,
) -> bool {
    match regtrig.kind {
        TriggerType::ChangesZone => {
            let origin = regtrig.origin_zone();
            let moved = if origin == Some(ZoneType::Battlefield) {
                run_params.card_lki
            } else {
                run_params.card
            };
            if let Some(valid_cause) = st_ab.params.get(keys::VALID_CAUSE) {
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
            if let Some(valid_cause) = st_ab.params.get(keys::VALID_CAUSE) {
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
            let origin = regtrig.origin_zone();
            let destination = regtrig.destination_zone();
            zone_changes.iter().any(|zc| {
                origin.is_none_or(|expected| zc.origin == expected)
                    && destination.is_none_or(|expected| zc.destination == expected)
            })
        }
        TriggerType::SpellCast
        | TriggerType::AbilityCast
        | TriggerType::SpellAbilityCast
        | TriggerType::SpellCastOrCopy
        | TriggerType::SpellCopied
        | TriggerType::SpellCopy
        | TriggerType::SpellAbilityCopy => {
            if let Some(valid_cause) = st_ab.params.get(keys::VALID_CAUSE) {
                let Some(cid) = run_params.spell_card else {
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
            if let Some(valid_activator) = st_ab.params.get(keys::VALID_ACTIVATOR) {
                let Some(pid) = run_params.spell_controller else {
                    return false;
                };
                if !matches_valid_player(valid_activator, pid, source_controller) {
                    return false;
                }
            }
            true
        }
        TriggerType::Attacks => {
            if let Some(valid_cause) = st_ab.params.get(keys::VALID_CAUSE) {
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
        TriggerType::DamageDone | TriggerType::DamageDealtOnce => {
            if let Some(combat_damage) = st_ab.params.get(keys::COMBAT_DAMAGE) {
                let wanted = combat_damage.eq_ignore_ascii_case("True");
                if run_params.is_combat_damage != Some(wanted) {
                    return false;
                }
            }
            if let Some(valid_source) = st_ab.params.get(keys::VALID_SOURCE) {
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
            if let Some(valid_target) = st_ab.params.get(keys::VALID_TARGET) {
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

fn matches_valid_card(valid: &str, card: &Card, source: &Card) -> bool {
    valid_filter::matches_valid_card(valid, card, source)
}

fn matches_valid_card_for_controller(
    valid: &str,
    card: &Card,
    source_controller: crate::ids::PlayerId,
) -> bool {
    // Create a temporary Card with the controller for matching
    // For disable triggers, we just need the controller, so we use a dummy source
    let mut dummy_source = card.clone();
    dummy_source.controller = source_controller;
    valid_filter::matches_valid_card(valid, card, &dummy_source)
}

fn matches_valid_player(
    valid: &str,
    player: crate::ids::PlayerId,
    source_controller: crate::ids::PlayerId,
) -> bool {
    valid_filter::matches_valid_player(valid, player, source_controller)
}

fn trigger_matches(
    valid_trigger: &str,
    game: &GameState,
    trigger_host: CardId,
    regtrig: &Trigger,
) -> bool {
    let host = game.card(trigger_host);
    let Some(exec_text) = host.svars.get(&regtrig.execute) else {
        return false;
    };
    let params = crate::parsing::Params::from_raw(exec_text);
    valid_trigger
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .all(|tok| match tok.to_ascii_lowercase().as_str() {
            "spell" => params.has(keys::SP),
            "ability" => params.has(keys::AB) || params.has(keys::DB),
            "trigger" => true,
            _ => true,
        })
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
