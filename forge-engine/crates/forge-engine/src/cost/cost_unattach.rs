//! Unattach equipment as a cost. Mirrors Java's `CostUnattach`.
//!
//! NOTE: Payability check is in `can_pay_inner()` in `mod.rs` (the central dispatcher).

use crate::game::GameState;
use crate::ids::CardId;

/// Pay by detaching the source from whatever it's attached to.
/// Mirrors Java's `CostUnattach.doPayment()` → `card.unattachFromEntity()`.
pub fn pay_as_decided(game: &mut GameState, source: CardId) -> bool {
    game.detach(source);
    true
}

pub const HASH_LKI: &str = "Unattached";
pub const HASH_CARDS: &str = "UnattachedCards";
