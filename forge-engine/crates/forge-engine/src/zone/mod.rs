use forge_foundation::ZoneType;
use serde::{Deserialize, Serialize};

use crate::ids::{CardId, PlayerId};

/// A game zone owned by a specific player.
/// Each player has their own Hand, Library, Graveyard, etc.
/// Battlefield and Stack are shared but cards still track their controller.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Zone {
    pub zone_type: ZoneType,
    pub owner: PlayerId,
    pub cards: Vec<CardId>,
}

impl Zone {
    pub fn new(zone_type: ZoneType, owner: PlayerId) -> Self {
        Zone {
            zone_type,
            owner,
            cards: Vec::new(),
        }
    }

    pub fn add(&mut self, card: CardId) {
        self.cards.push(card);
    }

    pub fn add_to_top(&mut self, card: CardId) {
        self.cards.push(card);
    }

    pub fn add_to_bottom(&mut self, card: CardId) {
        self.cards.insert(0, card);
    }

    pub fn remove(&mut self, card: CardId) -> bool {
        if let Some(pos) = self.cards.iter().position(|&c| c == card) {
            self.cards.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn contains(&self, card: CardId) -> bool {
        self.cards.contains(&card)
    }

    pub fn len(&self) -> usize {
        self.cards.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    /// Take the top card (last element = top of library).
    pub fn take_top(&mut self) -> Option<CardId> {
        self.cards.pop()
    }

    /// Peek at the top card without removing it.
    pub fn peek_top(&self) -> Option<CardId> {
        self.cards.last().copied()
    }
}

/// Key for looking up a zone: (zone_type, owner).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ZoneKey {
    pub zone_type: ZoneType,
    pub owner: PlayerId,
}

impl ZoneKey {
    pub fn new(zone_type: ZoneType, owner: PlayerId) -> Self {
        ZoneKey { zone_type, owner }
    }
}
