use forge_foundation::ZoneType;

use crate::card::{valid_filter, Card};
use crate::event::RunParams;
use crate::game::GameState;
use crate::ids::CardId;
use crate::parsing::keys;
use crate::trigger::Trigger;
use crate::trigger::TriggerMode;

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
                let trig_mode = trigger_type_name(&regtrig.mode);
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
    match regtrig.mode {
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
        TriggerMode::ChangesZoneAll {
            origin,
            destination,
            ..
        } => {
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
            zone_changes.iter().any(|zc| {
                origin.is_none_or(|expected| zc.origin == expected)
                    && destination.is_none_or(|expected| zc.destination == expected)
            })
        }
        TriggerMode::SpellCast { .. }
        | TriggerMode::AbilityCast { .. }
        | TriggerMode::SpellAbilityCast { .. }
        | TriggerMode::SpellCastOrCopy { .. }
        | TriggerMode::SpellCopied { .. }
        | TriggerMode::SpellCopy { .. }
        | TriggerMode::SpellAbilityCopy { .. } => {
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
        TriggerMode::DamageDone { .. } | TriggerMode::DamageDealtOnce { .. } => {
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

fn trigger_type_name(mode: &crate::trigger::TriggerMode) -> &'static str {
    match mode {
        crate::trigger::TriggerMode::ChangesZone { .. } => "ChangesZone",
        crate::trigger::TriggerMode::Phase { .. } => "Phase",
        crate::trigger::TriggerMode::SpellCast { .. } => "SpellCast",
        crate::trigger::TriggerMode::AbilityCast { .. } => "AbilityCast",
        crate::trigger::TriggerMode::SpellAbilityCast { .. } => "SpellAbilityCast",
        crate::trigger::TriggerMode::Attacks { .. } => "Attacks",
        crate::trigger::TriggerMode::Fight { .. } => "Fight",
        crate::trigger::TriggerMode::FightOnce { .. } => "FightOnce",
        crate::trigger::TriggerMode::DamageDone { .. } => "DamageDone",
        crate::trigger::TriggerMode::Countered { .. } => "Countered",
        crate::trigger::TriggerMode::Blocks { .. } => "Blocks",
        crate::trigger::TriggerMode::AttackerBlocked { .. } => "AttackerBlocked",
        crate::trigger::TriggerMode::AttackerUnblocked { .. } => "AttackerUnblocked",
        crate::trigger::TriggerMode::LifeGained { .. } => "LifeGained",
        crate::trigger::TriggerMode::LifeLost { .. } => "LifeLost",
        crate::trigger::TriggerMode::PayLife { .. } => "PayLife",
        crate::trigger::TriggerMode::LosesGame { .. } => "LosesGame",
        crate::trigger::TriggerMode::Discover { .. } => "Discover",
        crate::trigger::TriggerMode::Elementalbend { .. } => "Elementalbend",
        crate::trigger::TriggerMode::CounterAdded { .. } => "CounterAdded",
        crate::trigger::TriggerMode::CounterRemoved { .. } => "CounterRemoved",
        crate::trigger::TriggerMode::Sacrificed { .. } => "Sacrificed",
        crate::trigger::TriggerMode::Drawn { .. } => "Drawn",
        crate::trigger::TriggerMode::Milled { .. } => "Milled",
        crate::trigger::TriggerMode::MilledAll { .. } => "MilledAll",
        crate::trigger::TriggerMode::MilledOnce { .. } => "MilledOnce",
        crate::trigger::TriggerMode::PayEcho { .. } => "PayEcho",
        crate::trigger::TriggerMode::ClassLevelGained { .. } => "ClassLevelGained",
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
        crate::trigger::TriggerMode::SpellCopy { .. } => "SpellCopy",
        crate::trigger::TriggerMode::SpellAbilityCopy { .. } => "SpellAbilityCopy",
        crate::trigger::TriggerMode::SpellCastOrCopy { .. } => "SpellCastOrCopy",
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
        crate::trigger::TriggerMode::CounterRemovedOnce { .. } => "CounterRemovedOnce",
        crate::trigger::TriggerMode::Exerted { .. } => "Exerted",
        crate::trigger::TriggerMode::CollectEvidence { .. } => "CollectEvidence",
        crate::trigger::TriggerMode::Forage { .. } => "Forage",
        crate::trigger::TriggerMode::Enlisted { .. } => "Enlisted",
        crate::trigger::TriggerMode::FlippedCoin { .. } => "FlippedCoin",
        crate::trigger::TriggerMode::RolledDie { .. } => "RolledDie",
        crate::trigger::TriggerMode::RolledDieOnce { .. } => "RolledDieOnce",
        crate::trigger::TriggerMode::ManaExpend { .. } => "ManaExpend",
        crate::trigger::TriggerMode::Mutates { .. } => "Mutates",
        crate::trigger::TriggerMode::SetInMotion { .. } => "SetInMotion",
        crate::trigger::TriggerMode::CaseSolved { .. } => "CaseSolved",
        crate::trigger::TriggerMode::ClaimPrize { .. } => "ClaimPrize",
        crate::trigger::TriggerMode::TakesInitiative { .. } => "TakesInitiative",
        crate::trigger::TriggerMode::Discarded { .. } => "Discarded",
        crate::trigger::TriggerMode::Abandoned { .. } => "Abandoned",
        crate::trigger::TriggerMode::Adapt { .. } => "Adapt",
        crate::trigger::TriggerMode::BecomeRenowned { .. } => "BecomeRenowned",
        crate::trigger::TriggerMode::Evolved { .. } => "Evolved",
        crate::trigger::TriggerMode::PayCumulativeUpkeep { .. } => "PayCumulativeUpkeep",
        crate::trigger::TriggerMode::Investigated { .. } => "Investigated",
        crate::trigger::TriggerMode::Proliferate { .. } => "Proliferate",
        crate::trigger::TriggerMode::CompletedDungeon { .. } => "CompletedDungeon",
        crate::trigger::TriggerMode::CommitCrime { .. } => "CommitCrime",
        crate::trigger::TriggerMode::RingTemptsYou { .. } => "RingTemptsYou",
        crate::trigger::TriggerMode::ManifestDread { .. } => "ManifestDread",
        crate::trigger::TriggerMode::ConjureAll { .. } => "ConjureAll",
        crate::trigger::TriggerMode::SeekAll { .. } => "SeekAll",
        crate::trigger::TriggerMode::PlanarDice { .. } => "PlanarDice",
        crate::trigger::TriggerMode::NewGame => "NewGame",
        crate::trigger::TriggerMode::DayTimeChanges => "DayTimeChanges",
        crate::trigger::TriggerMode::BecomesPlotted { .. } => "BecomesPlotted",
        crate::trigger::TriggerMode::Specializes { .. } => "Specializes",
        crate::trigger::TriggerMode::Trains { .. } => "Trains",
        crate::trigger::TriggerMode::Devoured { .. } => "Devoured",
        crate::trigger::TriggerMode::BecomesCrewed { .. } => "BecomesCrewed",
        crate::trigger::TriggerMode::Championed { .. } => "Championed",
        crate::trigger::TriggerMode::Clashed { .. } => "Clashed",
        crate::trigger::TriggerMode::Mentored { .. } => "Mentored",
        crate::trigger::TriggerMode::FullyUnlock { .. } => "FullyUnlock",
        crate::trigger::TriggerMode::AbilityResolves { .. } => "AbilityResolves",
        crate::trigger::TriggerMode::AbilityTriggered { .. } => "AbilityTriggered",
        crate::trigger::TriggerMode::UnlockDoor { .. } => "UnlockDoor",
        crate::trigger::TriggerMode::CounterAddedAll { .. } => "CounterAddedAll",
        crate::trigger::TriggerMode::CounterPlayerAddedAll { .. } => "CounterPlayerAddedAll",
        crate::trigger::TriggerMode::CounterTypeAddedAll { .. } => "CounterTypeAddedAll",
        crate::trigger::TriggerMode::CrewedSaddled { .. } => "Crewed",
        crate::trigger::TriggerMode::DamageDoneOnceByController { .. } => {
            "DamageDoneOnceByController"
        }
        crate::trigger::TriggerMode::ExcessDamageAll { .. } => "ExcessDamageAll",
        crate::trigger::TriggerMode::PhaseOutAll { .. } => "PhaseOutAll",
        crate::trigger::TriggerMode::Vote => "Vote",
        crate::trigger::TriggerMode::GiveGift { .. } => "GiveGift",
        crate::trigger::TriggerMode::VisitAttraction { .. } => "VisitAttraction",
        crate::trigger::TriggerMode::EnteredRoom { .. } => "EnteredRoom",
        crate::trigger::TriggerMode::ChaosEnsues { .. } => "ChaosEnsues",
        crate::trigger::TriggerMode::BecomesSaddled { .. } => "BecomesSaddled",
        crate::trigger::TriggerMode::PlaneswalkedFrom { .. } => "PlaneswalkedFrom",
        crate::trigger::TriggerMode::PlaneswalkedTo { .. } => "PlaneswalkedTo",
        crate::trigger::TriggerMode::CrankContraption { .. } => "CrankContraption",
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
