use forge_foundation::ZoneType;

use crate::card::Card;
use crate::event::{RunParams};
use crate::trigger::TriggerType;
use crate::game::GameState;
use crate::ids::CardId;
use crate::parsing::keys;
use crate::trigger::Trigger;

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
            if let Some(valid_card) = st_ab.params.get(keys::VALID_CARD) {
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
            if let Some(valid_zone) = st_ab.params.get(keys::VALID_ZONE) {
                if !zone_list_matches(valid_zone, trig_host.zone) {
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
    if let Some(valid_card) = st_ab.params.get(keys::VALID_CARD) {
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
            let origin = trigger.origin_zone();
            let destination = trigger.destination_zone();
            zone_changes.iter().any(|zc| {
                origin.is_none_or(|expected| zc.origin == expected)
                    && destination.is_none_or(|expected| zc.destination == expected)
            })
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
        TriggerType::SpellCast
        | TriggerType::AbilityCast
        | TriggerType::SpellAbilityCast
        | TriggerType::SpellCastOrCopy
        | TriggerType::SpellCopied
        | TriggerType::SpellCopy
        | TriggerType::SpellAbilityCopy => {
            if let Some(valid_cause) = st_ab.params.get(keys::VALID_CAUSE) {
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
            if let Some(valid_activator) = st_ab.params.get(keys::VALID_ACTIVATOR) {
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

fn zone_list_matches(zones: &str, zone: ZoneType) -> bool {
    zones
        .split(',')
        .map(|s| s.trim())
        .any(|z| match z.to_ascii_lowercase().as_str() {
            "battlefield" => zone == ZoneType::Battlefield,
            "hand" => zone == ZoneType::Hand,
            "graveyard" => zone == ZoneType::Graveyard,
            "library" => zone == ZoneType::Library,
            "exile" => zone == ZoneType::Exile,
            "stack" => zone == ZoneType::Stack,
            "command" => zone == ZoneType::Command,
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

fn matches_valid_player(
    valid: &str,
    player: crate::ids::PlayerId,
    source_controller: crate::ids::PlayerId,
) -> bool {
    if valid.eq_ignore_ascii_case("Player") {
        return true;
    }
    if valid.eq_ignore_ascii_case("You") || valid.eq_ignore_ascii_case("YouCtrl") {
        return player == source_controller;
    }
    if valid.eq_ignore_ascii_case("Opponent") || valid.eq_ignore_ascii_case("OppCtrl") {
        return player != source_controller;
    }
    true
}

fn matches_valid_card(valid: &str, card: &Card, source: &Card) -> bool {
    matches_any_valid_card_token(valid, card, source.controller, Some(source.id))
}

fn matches_valid_card_for_controller(
    valid: &str,
    card: &Card,
    source_controller: crate::ids::PlayerId,
) -> bool {
    matches_any_valid_card_token(valid, card, source_controller, None)
}

fn matches_any_valid_card_token(
    valid: &str,
    card: &Card,
    source_controller: crate::ids::PlayerId,
    source_id: Option<CardId>,
) -> bool {
    valid
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .any(|token| matches_valid_card_token(token, card, source_controller, source_id))
}

fn matches_valid_card_token(
    token: &str,
    card: &Card,
    source_controller: crate::ids::PlayerId,
    source_id: Option<CardId>,
) -> bool {
    if token.eq_ignore_ascii_case("Any") {
        return true;
    }
    if token.eq_ignore_ascii_case("Card.Self") {
        return source_id == Some(card.id);
    }

    let mut parts = token.split('.');
    let kind = parts.next().unwrap_or_default();
    let kind_ok = match kind.to_ascii_lowercase().as_str() {
        "card" | "permanent" => true,
        "creature" => card.is_creature(),
        "artifact" => card.type_line.is_artifact(),
        "enchantment" => card.type_line.is_enchantment(),
        "land" => card.is_land(),
        "planeswalker" => card.type_line.is_planeswalker(),
        "nonland" => !card.is_land(),
        "noncreature" => !card.is_creature(),
        _ => false,
    };
    if !kind_ok {
        return false;
    }

    for qualifier in parts {
        match qualifier.to_ascii_lowercase().as_str() {
            "you" | "youctrl" | "youcontrol" => {
                if card.controller != source_controller {
                    return false;
                }
            }
            "opponent" | "oppctrl" | "opponentctrl" => {
                if card.controller == source_controller {
                    return false;
                }
            }
            "self" => {
                if source_id != Some(card.id) {
                    return false;
                }
            }
            _ => return false,
        }
    }
    true
}
