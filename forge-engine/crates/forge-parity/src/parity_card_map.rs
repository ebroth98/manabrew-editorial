use std::collections::HashMap;

use forge_engine_core::game::GameState;
use forge_engine_core::ids::{CardId, PlayerId};
use forge_foundation::ZoneType;

#[derive(Clone, Default)]
pub struct ParityCardMap {
    by_card: HashMap<CardId, u32>,
}

impl ParityCardMap {
    pub fn from_opening_state(game: &GameState) -> Self {
        let mut by_card: HashMap<CardId, u32> = HashMap::new();
        let mut next: u32 = 1;

        let mut players: Vec<PlayerId> = game.player_order.clone();
        players.sort_by_key(|p| p.0);

        for pid in players {
            for &cid in game.cards_in_zone(ZoneType::Hand, pid) {
                if by_card.insert(cid, next).is_none() {
                    next += 1;
                }
            }
            // Rust library top is the end of the vector; Java top is iterated first.
            // Assign parity ids in draw order (top -> bottom) to match Java.
            for &cid in game.cards_in_zone(ZoneType::Library, pid).iter().rev() {
                if by_card.insert(cid, next).is_none() {
                    next += 1;
                }
            }
        }

        Self { by_card }
    }

    pub fn id(&self, cid: CardId) -> u32 {
        self.by_card
            .get(&cid)
            .copied()
            // Use i32::MAX to match Java's Integer.MAX_VALUE fallback for tokens.
            .unwrap_or((i32::MAX as u32).saturating_sub(cid.0))
    }
}
