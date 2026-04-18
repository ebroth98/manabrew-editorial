use serde::{Deserialize, Serialize};

use crate::card::card_damage_map::DamageTarget;
use crate::event::{RunParams, TriggerType};
use crate::game::GameState;
use crate::ids::CardId;
use crate::parsing::{keys, Params};
use crate::spellability::SpellAbility;

use super::trigger::{check_card_filter, check_damage_target, TriggerBehavior};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerDamageDealtOnce {
    pub valid_source: Option<String>,
    pub valid_target: Option<String>,
    pub combat_damage_only: bool,
}

impl TriggerDamageDealtOnce {
    pub fn parse(params: &Params) -> Box<dyn TriggerBehavior> {
        Box::new(Self {
            valid_source: params.get_cloned(keys::VALID_SOURCE),
            valid_target: params.get_cloned(keys::VALID_TARGET),
            combat_damage_only: params.is_true(keys::COMBAT_DAMAGE),
        })
    }
}

#[typetag::serde]
impl TriggerBehavior for TriggerDamageDealtOnce {
    fn trigger_type(&self) -> TriggerType {
        TriggerType::DamageDealtOnce
    }

    fn perform_test(
        &self,
        trigger: &super::trigger::Trigger,
        params: &RunParams,
        game: &GameState,
    ) -> bool {
        let host_card = trigger.base.card_trait_base.get_host_card().id;
        let host_controller = trigger.base.card_trait_base.get_host_card().controller;
        if self.combat_damage_only && params.is_combat_damage != Some(true) {
            return false;
        }
        check_card_filter(
            &self.valid_source,
            params.damage_source,
            host_card,
            host_controller,
            game,
        ) && check_damage_target(
            &self.valid_target,
            params,
            host_card,
            host_controller,
            game,
            true,
        )
    }

    fn set_triggering_objects(
        &self,
        _trigger: &super::trigger::Trigger,
        sa: &mut SpellAbility,
        params: &RunParams,
        _game: &GameState,
    ) {
        if let Some(src) = params.damage_source {
            sa.set_triggering_object("Source", &src.0.to_string());
        }
        if let Some(amount) = params.damage_amount {
            sa.set_triggering_object("DamageAmount", &amount.to_string());
        }
        if let Some(card) = params.damage_target_card {
            sa.set_triggering_object("Targets", &card.0.to_string());
        } else if let Some(player) = params.damage_target_player {
            sa.set_triggering_object("Targets", &player.0.to_string());
        }
    }

    fn get_important_stack_objects(&self, _trigger: &super::trigger::Trigger, sa: &SpellAbility) -> String {
        // Java: "Damage Source: " + Source + ", Damaged: " + Targets + ", Amount: " + DamageAmount
        format!(
            "Damage Source: {}, Damaged: {}, Amount: {}",
            sa.get_triggering_object("Source").unwrap_or(""),
            sa.get_triggering_object("Targets").unwrap_or(""),
            sa.get_triggering_object("DamageAmount").unwrap_or("")
        )
    }
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
