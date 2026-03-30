use crate::ability::ability_utils::parse_counter_type;
use crate::parsing::compare::compare_expr;
use crate::parsing::{keys, Params};
use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
    spellability::SpellAbility,
};

use super::trigger::{check_card_filter, check_zone_filter, TriggerMode};

fn zone_in_filter(
    actual: Option<forge_foundation::ZoneType>,
    excluded: &Option<Vec<forge_foundation::ZoneType>>,
) -> bool {
    excluded
        .as_ref()
        .is_some_and(|zones| actual.is_some_and(|zone| zones.contains(&zone)))
}

fn evaluate_triggered_card_expr(expr: &str, moved: &crate::card::Card) -> Option<i32> {
    match expr {
        "Count$CardPower" => Some(moved.power()),
        "Count$CardToughness" => Some(moved.toughness()),
        _ => expr
            .strip_prefix("Count$CardCounters.")
            .map(|counter_name| moved.counter_count(&parse_counter_type(counter_name))),
    }
}

fn selected_moved_card(
    params: &RunParams,
    destination: Option<forge_foundation::ZoneType>,
    game: &GameState,
) -> Option<CardId> {
    if params.origin == Some(forge_foundation::ZoneType::Battlefield)
        || (params.origin == Some(forge_foundation::ZoneType::Graveyard)
            && destination != Some(forge_foundation::ZoneType::Battlefield))
    {
        return params.card_lki.or(params.card);
    }
    if destination == Some(forge_foundation::ZoneType::Battlefield) {
        if let Some(card) = params.card {
            let controller = game.card(card).controller;
            let zone = game.zone(forge_foundation::ZoneType::Battlefield, controller);
            if let Some((_, latest)) = zone
                .cards_added_this_turn
                .iter()
                .filter(|(_, added)| *added == card)
                .max_by(|(_, a), (_, b)| {
                    crate::card::card_predicates::compare_by_game_timestamp(game, *a, *b)
                })
            {
                return Some(*latest);
            }
        }
    }
    params.card
}

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    game: &GameState,
    host_card: CardId,
    host_controller: PlayerId,
    current_trigger_id: Option<u32>,
) -> bool {
    if let TriggerMode::ChangesZone {
        origin,
        destination,
        valid_card,
        excluded_origins,
        excluded_destinations,
        valid_cause,
        check_on_triggered_card,
        fizzle,
        not_this_ability,
        condition_you_cast_this_turn,
    } = mode
    {
        if !check_zone_filter(origin, params.origin)
            || !check_zone_filter(destination, params.destination)
        {
            return false;
        }
        if zone_in_filter(params.origin, excluded_origins)
            || zone_in_filter(params.destination, excluded_destinations)
        {
            return false;
        }
        let moved_card = selected_moved_card(params, *destination, game);
        if params.origin == Some(forge_foundation::ZoneType::Battlefield)
            && game.card(host_card).zone == forge_foundation::ZoneType::Graveyard
            && params
                .change_zone_table
                .as_ref()
                .is_some_and(|table| !table.last_state_graveyard().contains(&host_card))
        {
            return false;
        }
        if !check_card_filter(valid_card, moved_card, host_card, host_controller, game) {
            return false;
        }
        if let Some(filter) = valid_cause {
            let cause_matches = params
                .cause
                .as_ref()
                .and_then(|sa| sa.source)
                .or(params.cause_card)
                .or(params.causer)
                .is_some_and(|cause_card| {
                    super::trigger::matches_valid_card(
                        filter,
                        cause_card,
                        host_card,
                        host_controller,
                        game,
                    )
                });
            if !cause_matches {
                return false;
            }
        }
        if let Some(expected_fizzle) = fizzle {
            if params.fizzle != Some(*expected_fizzle) {
                return false;
            }
        }
        if *not_this_ability
            && params
                .cause
                .as_ref()
                .and_then(|cause| cause.source_trigger_id)
                .zip(current_trigger_id)
                .is_some_and(|(source_trigger_id, trigger_id)| source_trigger_id == trigger_id)
        {
            return false;
        }
        if let Some(cast_expr) = condition_you_cast_this_turn {
            let casting_player = params
                .spell_controller
                .or(params.player)
                .unwrap_or(host_controller);
            if !compare_expr(game.player(casting_player).spells_cast_this_turn, cast_expr) {
                return false;
            }
        }
        if let Some(check_expr) = check_on_triggered_card {
            let moved = game.card(moved_card.unwrap_or(host_card));
            let mut parts = check_expr.split_whitespace();
            let lhs = parts.next().unwrap_or("");
            let rhs = parts.next().unwrap_or("GE1");
            let Some(actual_value) = evaluate_triggered_card_expr(lhs, moved) else {
                return false;
            };
            if !compare_expr(actual_value, rhs) {
                return false;
            }
        }
        return true;
    }
    panic!("Expected ChangesZone mode");
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
    let valid_card = params.get_cloned(keys::VALID_CARD);
    let excluded_origins = params.get("ExcludedOrigins").map(|raw| {
        raw.split(',')
            .filter_map(|zone| super::trigger::parse_zone(zone.trim()))
            .collect::<Vec<_>>()
    });
    let excluded_destinations = params.get("ExcludedDestinations").map(|raw| {
        raw.split(',')
            .filter_map(|zone| super::trigger::parse_zone(zone.trim()))
            .collect::<Vec<_>>()
    });
    let valid_cause = params.get_cloned(keys::VALID_CAUSE);
    let check_on_triggered_card = params.get_cloned("CheckOnTriggeredCard");
    let fizzle = params
        .get("Fizzle")
        .map(|value| value.eq_ignore_ascii_case("true"));
    let not_this_ability = params.has("NotThisAbility");
    let condition_you_cast_this_turn = params.get_cloned("ConditionYouCastThisTurn");
    TriggerMode::ChangesZone {
        origin,
        destination,
        valid_card,
        excluded_origins,
        excluded_destinations,
        valid_cause,
        check_on_triggered_card,
        fizzle,
        not_this_ability,
        condition_you_cast_this_turn,
    }
}

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    if params.origin == Some(forge_foundation::ZoneType::Battlefield) {
        if let Some(card_id) = params.card_lki.or(params.card) {
            sa.add_triggering_object("Card", &card_id.0.to_string());
            if let Some(power) = params.lki_power {
                sa.add_triggering_object("TriggeredCardPower", &power.to_string());
            }
            if let Some(toughness) = params.lki_toughness {
                sa.add_triggering_object("TriggeredCardToughness", &toughness.to_string());
            }
        }
        if let Some(card_id) = params.card {
            sa.add_triggering_object("NewCard", &card_id.0.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use forge_foundation::{CardTypeLine, ColorSet, ManaCost, ZoneType};

    use super::perform_test;
    use crate::card::Card;
    use crate::event::RunParams;
    use crate::game::GameState;
    use crate::ids::{CardId, PlayerId};
    use crate::trigger::trigger::TriggerMode;

    #[test]
    fn changes_zone_uses_lki_card_for_leaves_battlefield_valid_card() {
        let mut game = GameState::new(&["A", "B"], 20);
        let p0 = PlayerId(0);

        let mut card = Card::new(
            CardId(0),
            "Test".to_string(),
            p0,
            CardTypeLine::parse("Creature"),
            ManaCost::parse(""),
            ColorSet::COLORLESS,
            Some(2),
            Some(2),
            vec![],
            vec![],
        );
        card.set_zone(ZoneType::Graveyard);
        let cid = game.create_card(card);

        let mode = TriggerMode::ChangesZone {
            origin: Some(ZoneType::Battlefield),
            destination: Some(ZoneType::Graveyard),
            valid_card: Some("Creature".to_string()),
            excluded_origins: None,
            excluded_destinations: None,
            valid_cause: None,
            check_on_triggered_card: None,
            fizzle: None,
            not_this_ability: false,
            condition_you_cast_this_turn: None,
        };

        let params = RunParams {
            card: None,
            card_lki: Some(cid),
            origin: Some(ZoneType::Battlefield),
            destination: Some(ZoneType::Graveyard),
            ..Default::default()
        };

        assert!(perform_test(&mode, &params, &game, cid, p0, None));
    }
}
