use crate::parsing::{keys, Params};
use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
    spellability::SpellAbility,
};

use super::trigger::{check_player_filter, TriggerMode};

pub fn parse_mode(params: &Params) -> TriggerMode {
    let valid_player = params.get_cloned(keys::VALID_PLAYER);
    let valid_source = params.get_cloned(keys::VALID_SOURCE);
    let first_time_only = params.has("FirstTime");
    let spell_only = params.has("Spell");
    TriggerMode::LifeGained {
        valid_player,
        valid_source,
        first_time_only,
        spell_only,
    }
}

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    game: &GameState,
    host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    if let TriggerMode::LifeGained {
        valid_player,
        valid_source,
        first_time_only,
        spell_only,
    } = mode
    {
        if !check_player_filter(valid_player, params.player, host_controller) {
            return false;
        }
        if let Some(filter) = valid_source {
            let source_matches = params
                .source_card
                .or(params.spell_card)
                .is_some_and(|source| {
                    super::trigger::matches_valid_card(
                        filter,
                        source,
                        host_card,
                        host_controller,
                        game,
                    )
                });
            if !source_matches {
                return false;
            }
        }
        if *first_time_only && params.first_time != Some(true) {
            return false;
        }
        if *spell_only
            && !params
                .source_sa
                .as_ref()
                .or(params.spell_ability.as_ref())
                .is_some_and(|sa| sa.is_spell)
        {
            return false;
        }
        return true;
    }
    panic!("Expected LifeGained mode");
}

pub fn set_triggering_objects(sa: &mut SpellAbility, params: &RunParams) {
    if let Some(amount) = params.life_amount {
        sa.add_triggering_object("LifeAmount", &amount.to_string());
    }
    if let Some(p) = params.player {
        sa.add_triggering_object("Player", &p.0.to_string());
    }
}

pub fn get_important_stack_objects(sa: &SpellAbility) -> String {
    // Java: "Player: " + Player + ", GainedAmount: " + LifeAmount
    format!(
        "Player: {}, GainedAmount: {}",
        sa.get_triggering_object("Player").unwrap_or_default(),
        sa.get_triggering_object("LifeAmount").unwrap_or_default()
    )
}
