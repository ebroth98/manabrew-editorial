//! Mill cards as a cost. Mirrors Java's `CostMill`.
//!
//! NOTE: Payability check is in `can_pay_inner()` in `mod.rs` (the central dispatcher).

use forge_foundation::ZoneType;

use crate::game::GameState;
use crate::ids::PlayerId;

/// Pay by milling cards (library -> graveyard).
/// Mirrors Java's `CostMill.payAsDecided()`.
/// NOTE: Trigger firing (Milled, zone change) must be handled by the caller.
pub fn pay_as_decided(
    game: &mut GameState,
    player: PlayerId,
    amount: i32,
) -> Vec<crate::ids::CardId> {
    let mut milled = Vec::new();
    for _ in 0..amount {
        if let Some(top) = game.zone_mut(ZoneType::Library, player).take_top() {
            game.move_card(top, ZoneType::Graveyard, player);
            milled.push(top);
        }
    }
    milled
}
