use crate::ability::AbilityKey;
use crate::parsing::{keys, Params};
use crate::{
    event::{AbilityValue, RunParams},
    game::GameState,
    ids::{CardId, PlayerId},
};

use super::trigger::{check_card_filter, matches_amount, matches_valid_card, TriggerMode};

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    game: &GameState,
    host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    if let TriggerMode::ChangesZoneAll {
        origin,
        destination,
        valid_card,
        valid_cause,
        first_time_only,
        valid_amount,
    } = mode
    {
        let table = match params.get_value(AbilityKey::Cards) {
            Some(AbilityValue::CardZoneTable(table)) => Some(table),
            _ => None,
        };

        if let Some(filter) = valid_cause {
            let Some(cause_card) = (match params.get_value(AbilityKey::Cause) {
                Some(AbilityValue::SpellAbility(sa)) => sa.source,
                Some(AbilityValue::Card(card)) => Some(card),
                _ => None,
            }) else {
                return false;
            };
            if !matches_valid_card(filter, cause_card, host_card, host_controller, game) {
                return false;
            }
        }

        let matching: Vec<CardId> = if let Some(table) = table.as_ref() {
            let origins = origin.map(|zone| vec![zone]);
            let destinations = destination.map(|zone| vec![zone]);
            table.filter_cards(
                game,
                origins.as_deref(),
                destinations.as_deref(),
                valid_card.as_deref(),
                host_controller,
            )
        } else {
            let Some(zone_changes) = params.zone_changes.as_ref() else {
                return false;
            };
            zone_changes
                .iter()
                .filter(|zc| origin.is_none_or(|expected| zc.origin == expected))
                .filter(|zc| destination.is_none_or(|expected| zc.destination == expected))
                .filter_map(|zc| {
                    if check_card_filter(
                        valid_card,
                        Some(zc.card),
                        host_card,
                        host_controller,
                        game,
                    ) {
                        Some(zc.card)
                    } else {
                        None
                    }
                })
                .collect()
        };

        if matching.is_empty() {
            return false;
        }

        if *first_time_only {
            if let Some(table) = table.as_ref() {
                let seen_before = table
                    .filter_cards(
                        game,
                        origin.map(|zone| vec![zone]).as_deref(),
                        destination.map(|zone| vec![zone]).as_deref(),
                        valid_card.as_deref(),
                        host_controller,
                    )
                    .into_iter()
                    .filter(|card| !matching.contains(card))
                    .count();
                if seen_before > 0 {
                    return false;
                }
            } else {
                for &card_id in &matching {
                    let card = game.card(card_id);
                    let zone_owner = if card.zone == forge_foundation::ZoneType::Battlefield {
                        card.controller
                    } else {
                        card.owner
                    };
                    let zone = game.zone(card.zone, zone_owner);
                    let seen_before = zone
                        .cards_added_this_turn
                        .iter()
                        .filter(|(_, seen_card)| !matching.contains(seen_card))
                        .filter(|(seen_origin, seen_card)| {
                            origin.is_none_or(|expected| *seen_origin == expected)
                                && check_card_filter(
                                    valid_card,
                                    Some(*seen_card),
                                    host_card,
                                    host_controller,
                                    game,
                                )
                        })
                        .count();
                    if seen_before > 0 {
                        return false;
                    }
                }
            }
        }

        if let Some(amount_filter) = valid_amount {
            return matches_amount(amount_filter, matching.len());
        }

        return true;
    }
    panic!("Expected ChangesZoneAll mode");
}

pub fn parse_mode(params: &Params) -> TriggerMode {
    let origin = params.get(keys::ORIGIN).and_then(|s| {
        if s == "Any" {
            None
        } else {
            super::trigger::parse_zone(s)
        }
    });
    let destination = params.get(keys::DESTINATION).and_then(|s| {
        if s == "Any" {
            None
        } else {
            super::trigger::parse_zone(s)
        }
    });
    let valid_card = params
        .get(keys::VALID_CARDS)
        .or_else(|| params.get(keys::VALID_CARD))
        .map(|s| s.to_string());
    let valid_cause = params.get_cloned(keys::VALID_CAUSE);
    let first_time_only = params.has("FirstTime");
    let valid_amount = params.get_cloned("ValidAmount");
    TriggerMode::ChangesZoneAll {
        origin,
        destination,
        valid_card,
        valid_cause,
        first_time_only,
        valid_amount,
    }
}
