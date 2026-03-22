//! Zone-change aggregation table (Java parity: `CardZoneTable`).

use std::collections::HashMap;

use forge_foundation::ZoneType;

use crate::ability::ability_utils;
use crate::event::{RunParams, TriggerType};
use crate::game::GameState;
use crate::ids::{CardId, PlayerId};
use crate::spellability::SpellAbility;
use crate::trigger::TriggerHandler;

#[derive(Debug, Clone, Default)]
pub struct CardZoneTable {
    data: HashMap<(ZoneType, ZoneType), Vec<CardId>>,
    created_tokens: Vec<CardId>,
    first_time_token_creators: Vec<PlayerId>,
}

impl CardZoneTable {
    /// Java parity: special put that appends cards to an origin->destination bucket.
    pub fn put(&mut self, origin: Option<ZoneType>, destination: Option<ZoneType>, card: CardId) {
        let from = origin.unwrap_or(ZoneType::None);
        let to = destination.unwrap_or(ZoneType::None);
        self.data.entry((from, to)).or_default().push(card);
    }

    pub fn trigger_changes_zone_all(
        &self,
        trigger_handler: &mut TriggerHandler,
        _game: &GameState,
        _cause: Option<&SpellAbility>,
    ) {
        if !self.created_tokens.is_empty() {
            trigger_handler.run_trigger(
                TriggerType::TokenCreatedOnce,
                RunParams::default(),
                false,
            );
        }
        if !self.data.is_empty() {
            trigger_handler.run_trigger(
                TriggerType::ChangesZoneAll,
                RunParams::default(),
                false,
            );
        }
    }

    pub fn filter_cards(
        &self,
        game: &GameState,
        origin: Option<&[ZoneType]>,
        destination: Option<&[ZoneType]>,
        valid: Option<&str>,
        source_controller: PlayerId,
    ) -> Vec<CardId> {
        let mut out = Vec::new();
        for (&(from, to), cards) in &self.data {
            if let Some(origins) = origin {
                if !origins.contains(&from) {
                    continue;
                }
            }
            if let Some(destinations) = destination {
                if !destinations.contains(&to) {
                    continue;
                }
            }
            out.extend(cards.iter().copied());
        }
        if let Some(filter) = valid {
            out.retain(|&cid| ability_utils::matches_valid_cards(game.card(cid), filter, source_controller));
        }
        out
    }

    pub fn all_cards(&self) -> Vec<CardId> {
        self.data.values().flat_map(|v| v.iter().copied()).collect()
    }

    pub fn add_token(&mut self, card: CardId, owner: PlayerId, first_time: bool) {
        self.created_tokens.push(card);
        if first_time && !self.first_time_token_creators.contains(&owner) {
            self.first_time_token_creators.push(owner);
        }
    }
}
