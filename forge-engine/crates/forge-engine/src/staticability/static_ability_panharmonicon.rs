use forge_foundation::ZoneType;

use crate::card::CardInstance;
use crate::event::RunParams;
use crate::game::GameState;
use crate::parsing::keys;
use crate::ids::CardId;
use crate::trigger::Trigger;
use crate::trigger::TriggerMode;

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
                let trig_mode = trigger_mode_name(&trigger.mode);
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

fn trigger_mode_name(mode: &crate::trigger::TriggerMode) -> &'static str {
    match mode {
        crate::trigger::TriggerMode::ChangesZone { .. } => "ChangesZone",
        crate::trigger::TriggerMode::Phase { .. } => "Phase",
        crate::trigger::TriggerMode::SpellCast { .. } => "SpellCast",
        crate::trigger::TriggerMode::Attacks { .. } => "Attacks",
        crate::trigger::TriggerMode::DamageDone { .. } => "DamageDone",
        crate::trigger::TriggerMode::Countered { .. } => "Countered",
        crate::trigger::TriggerMode::Blocks { .. } => "Blocks",
        crate::trigger::TriggerMode::AttackerBlocked { .. } => "AttackerBlocked",
        crate::trigger::TriggerMode::AttackerUnblocked { .. } => "AttackerUnblocked",
        crate::trigger::TriggerMode::LifeGained { .. } => "LifeGained",
        crate::trigger::TriggerMode::LifeLost { .. } => "LifeLost",
        crate::trigger::TriggerMode::CounterAdded { .. } => "CounterAdded",
        crate::trigger::TriggerMode::CounterRemoved { .. } => "CounterRemoved",
        crate::trigger::TriggerMode::Sacrificed { .. } => "Sacrificed",
        crate::trigger::TriggerMode::Drawn { .. } => "Drawn",
        crate::trigger::TriggerMode::Milled { .. } => "Milled",
        crate::trigger::TriggerMode::Taps { .. } => "Taps",
        crate::trigger::TriggerMode::Untaps { .. } => "Untaps",
        crate::trigger::TriggerMode::Transformed { .. } => "Transformed",
        crate::trigger::TriggerMode::TurnFaceUp { .. } => "TurnFaceUp",
        crate::trigger::TriggerMode::Attached { .. } => "Attached",
        crate::trigger::TriggerMode::Unattached { .. } => "Unattached",
        crate::trigger::TriggerMode::LandPlayed { .. } => "LandPlayed",
        crate::trigger::TriggerMode::BecomesTarget { .. } => "BecomesTarget",
        crate::trigger::TriggerMode::TapsForMana { .. } => "TapsForMana",
        crate::trigger::TriggerMode::AbilityActivated { .. } => "AbilityActivated",
        crate::trigger::TriggerMode::Explored { .. } => "Explored",
        crate::trigger::TriggerMode::Exploited { .. } => "Exploited",
        crate::trigger::TriggerMode::BecomeMonstrous { .. } => "BecomeMonstrous",
        crate::trigger::TriggerMode::BecomeMonarch { .. } => "BecomeMonarch",
        crate::trigger::TriggerMode::DamageDealtOnce { .. } => "DamageDealtOnce",
        crate::trigger::TriggerMode::Destroyed { .. } => "Destroyed",
        crate::trigger::TriggerMode::Exiled { .. } => "Exiled",
        crate::trigger::TriggerMode::TokenCreated { .. } => "TokenCreated",
        crate::trigger::TriggerMode::SpellCopied { .. } => "SpellCopied",
        crate::trigger::TriggerMode::AttackersDeclared { .. } => "AttackersDeclared",
        crate::trigger::TriggerMode::BlockersDeclared => "BlockersDeclared",
        crate::trigger::TriggerMode::ChangesZoneAll { .. } => "ChangesZoneAll",
        crate::trigger::TriggerMode::ChangesController { .. } => "ChangesController",
        crate::trigger::TriggerMode::TurnBegin { .. } => "TurnBegin",
        crate::trigger::TriggerMode::DamageDoneOnce { .. } => "DamageDoneOnce",
        crate::trigger::TriggerMode::SpellCastAll { .. } => "SpellCastAll",
        crate::trigger::TriggerMode::LifeLostAll { .. } => "LifeLostAll",
        crate::trigger::TriggerMode::CounterAddedOnce { .. } => "CounterAddedOnce",
        crate::trigger::TriggerMode::DiscardedAll { .. } => "DiscardedAll",
        crate::trigger::TriggerMode::SacrificedOnce { .. } => "SacrificedOnce",
        crate::trigger::TriggerMode::Cycled { .. } => "Cycled",
        crate::trigger::TriggerMode::PhasedIn { .. } => "PhasedIn",
        crate::trigger::TriggerMode::PhasedOut { .. } => "PhasedOut",
        crate::trigger::TriggerMode::Always => "Always",
        crate::trigger::TriggerMode::Immediate => "Immediate",
        crate::trigger::TriggerMode::Surveil { .. } => "Surveil",
        crate::trigger::TriggerMode::Scry { .. } => "Scry",
        crate::trigger::TriggerMode::Foretell { .. } => "Foretell",
        crate::trigger::TriggerMode::SearchedLibrary { .. } => "SearchedLibrary",
        crate::trigger::TriggerMode::Shuffled { .. } => "Shuffled",
        crate::trigger::TriggerMode::ManaAdded { .. } => "ManaAdded",
        crate::trigger::TriggerMode::TokenCreatedOnce { .. } => "TokenCreatedOnce",
        crate::trigger::TriggerMode::TapAll { .. } => "TapAll",
        crate::trigger::TriggerMode::UntapAll { .. } => "UntapAll",
        crate::trigger::TriggerMode::BecomesTargetOnce { .. } => "BecomesTargetOnce",
        crate::trigger::TriggerMode::AttackerBlockedByCreature { .. } => {
            "AttackerBlockedByCreature"
        }
        crate::trigger::TriggerMode::AttackerBlockedOnce { .. } => "AttackerBlockedOnce",
        crate::trigger::TriggerMode::AttackerUnblockedOnce { .. } => "AttackerUnblockedOnce",
        crate::trigger::TriggerMode::SpellCastOnce { .. } => "SpellCastOnce",
        crate::trigger::TriggerMode::SpellCastOfType { .. } => "SpellCastOfType",
        crate::trigger::TriggerMode::DamageAll { .. } => "DamageAll",
        crate::trigger::TriggerMode::DamagePreventedOnce { .. } => "DamagePreventedOnce",
        crate::trigger::TriggerMode::ExcessDamage { .. } => "ExcessDamage",
        crate::trigger::TriggerMode::LifeGainedAll { .. } => "LifeGainedAll",
        crate::trigger::TriggerMode::CounterRemovedOnce { .. } => "CounterRemovedOnce",
        crate::trigger::TriggerMode::Exerted { .. } => "Exerted",
        crate::trigger::TriggerMode::CollectEvidence { .. } => "CollectEvidence",
        crate::trigger::TriggerMode::Forage { .. } => "Forage",
        crate::trigger::TriggerMode::Enlisted { .. } => "Enlisted",
        crate::trigger::TriggerMode::FlippedCoin { .. } => "FlippedCoin",
        crate::trigger::TriggerMode::RolledDie { .. } => "RolledDie",
        crate::trigger::TriggerMode::RolledDieOnce { .. } => "RolledDieOnce",
        crate::trigger::TriggerMode::ManaExpend { .. } => "ManaExpend",
    }
}

fn mode_specific_matches(
    st_ab: &crate::staticability::StaticAbility,
    trigger: &Trigger,
    run_params: &RunParams,
    game: &GameState,
    source_controller: crate::ids::PlayerId,
) -> bool {
    match trigger.mode {
        TriggerMode::ChangesZone { origin, .. } => {
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
        TriggerMode::Attacks { .. } => {
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
        TriggerMode::SpellCast { .. } => {
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
        TriggerMode::DamageDone { .. } => {
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
        TriggerMode::DamageDealtOnce { .. } => {
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

fn matches_valid_card(valid: &str, card: &CardInstance, source: &CardInstance) -> bool {
    matches_any_valid_card_token(valid, card, source.controller, Some(source.id))
}

fn matches_valid_card_for_controller(
    valid: &str,
    card: &CardInstance,
    source_controller: crate::ids::PlayerId,
) -> bool {
    matches_any_valid_card_token(valid, card, source_controller, None)
}

fn matches_any_valid_card_token(
    valid: &str,
    card: &CardInstance,
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
    card: &CardInstance,
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
