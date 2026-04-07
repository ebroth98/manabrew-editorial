use super::trigger::{check_card_filter, matches_valid_sa, TriggerMode};
use crate::ability::AbilityKey;
use crate::{
    event::{AbilityValue, RunParams},
    game::GameState,
    ids::{CardId, PlayerId},
    spellability::SpellAbility,
};

fn split_csv(s: &str) -> impl Iterator<Item = &str> {
    s.split(',').map(str::trim).filter(|p| !p.is_empty())
}

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    game: &GameState,
    host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    let TriggerMode::AbilityTriggered {
        valid_mode,
        valid_destination,
        valid_spell_ability,
        valid_source,
        valid_cause,
        triggered_own_ability,
    } = mode
    else {
        panic!("Expected AbilityTriggered mode");
    };

    let Some(AbilityValue::SpellAbility(sa)) = params.get_value(AbilityKey::SpellAbility) else {
        return false;
    };
    let Some(source) = sa.source else {
        return false;
    };

    if let Some(valid_modes) = valid_mode {
        let Some(AbilityValue::String(mode_name)) = params.get_value(AbilityKey::Mode) else {
            return false;
        };
        if !split_csv(valid_modes).any(|m| m.eq_ignore_ascii_case(&mode_name)) {
            return false;
        }
    }

    if let Some(valid_destinations) = valid_destination {
        let destinations = if let Some(AbilityValue::String(destinations)) =
            params.get_value(AbilityKey::Destination)
        {
            destinations
        } else if let Some(AbilityValue::Zone(dest)) = params.get_value(AbilityKey::Destination) {
            format!("{dest:?}")
        } else {
            return false;
        };
        if split_csv(&destinations).all(|destination_name| {
            !split_csv(valid_destinations).any(|d| d.eq_ignore_ascii_case(destination_name))
        }) {
            return false;
        }
    }

    if let Some(filter) = valid_spell_ability {
        if !matches_valid_sa(filter, &sa) {
            return false;
        }
    }

    if !check_card_filter(valid_source, Some(source), host_card, host_controller, game) {
        return false;
    }

    let mut causes = match params.get_value(AbilityKey::Cause) {
        Some(AbilityValue::Cards(cards)) => cards,
        Some(AbilityValue::Card(card)) => vec![card],
        _ => params.cards.clone().unwrap_or_default(),
    };

    if let Some(filter) = valid_cause {
        if causes.is_empty() {
            return false;
        }
        if !causes.iter().any(|&c| {
            check_card_filter(
                &Some(filter.clone()),
                Some(c),
                host_card,
                host_controller,
                game,
            )
        }) {
            return false;
        }
    }

    if *triggered_own_ability && !causes.contains(&source) {
        return false;
    }

    true
}

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    // Java: sa.setTriggeringObject(AbilityKey.Source, triggeredSA.getHostCard());
    // The source is the host card of the triggered SpellAbility
    if let Some(ref triggered_sa) = params.spell_ability {
        if let Some(source) = triggered_sa.source {
            sa.add_triggering_object("Source", &source.0.to_string());
        }
    } else if let Some(card) = params.source_card {
        sa.add_triggering_object("Source", &card.0.to_string());
    }
    // Java: sa.setTriggeringObjectsFrom(runParams, AbilityKey.SpellAbility, AbilityKey.Cause);
    // SpellAbility and Cause are complex objects; store what we can
    if let Some(ref cause_cards) = params.cards {
        let csv = cause_cards
            .iter()
            .map(|c| c.0.to_string())
            .collect::<Vec<_>>()
            .join(",");
        sa.add_triggering_object("Cause", &csv);
    } else if let Some(cause_card) = params.cause_card {
        sa.add_triggering_object("Cause", &cause_card.0.to_string());
    }
}

pub fn get_important_stack_objects(sa: &SpellAbility) -> String {
    format!(
        "SpellAbility: {}",
        sa.get_triggering_object("SpellAbility").unwrap_or("")
    )
}
