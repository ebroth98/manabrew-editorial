//! Last-Known Information (LKI) system.
//!
//! Mirrors Java's `Game.copyLastState()` + `Game.lastStateBattlefield`.
//! Stores lightweight snapshots of battlefield cards at key checkpoints
//! so trigger SVars (e.g. `TriggeredCard$CardPower`) resolve using the
//! card's state at the time it was last on the battlefield.

use crate::card::{CardInstance, CounterType};
use crate::ids::{CardId, PlayerId};
use forge_foundation::ZoneType;
use std::collections::BTreeMap;

/// Lightweight snapshot of a card's state on the battlefield.
/// Captured by `GameState::copy_last_state()` at key checkpoints.
#[derive(Debug, Clone)]
pub struct CardSnapshot {
    pub id: CardId,
    pub controller: PlayerId,
    pub owner: PlayerId,
    pub power: i32,
    pub toughness: i32,
    pub counters: BTreeMap<CounterType, i32>,
    pub tapped: bool,
    pub zone: ZoneType,
    pub card_name: String,
}

impl CardSnapshot {
    /// Create a snapshot from a live card.
    pub fn from_card(card: &CardInstance) -> Self {
        Self {
            id: card.id,
            controller: card.controller,
            owner: card.owner,
            power: card.power(),
            toughness: card.toughness(),
            counters: card.counters.clone(),
            tapped: card.tapped,
            zone: card.zone,
            card_name: card.card_name.clone(),
        }
    }
}
