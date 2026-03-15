//! Choose a creature type as a cost. Mirrors Java's `CostChooseCreatureType`.
//!
//! NOTE: Payability check is in `can_pay_inner()` in `mod.rs` (the central dispatcher).

use crate::game::GameState;
use crate::ids::{CardId, PlayerId};

/// Pay by setting chosen type on the source card.
/// Mirrors Java's `CostChooseCreatureType.payAsDecided()` →
/// `sa.getHostCard().setChosenType(pd.type)`.
pub fn pay_as_decided(
    game: &mut GameState,
    source: CardId,
    player: PlayerId,
    chosen_type: &str,
) -> bool {
    let card = game.card_mut(source);
    card.chosen_type = Some(chosen_type.to_string());
    card.chosen_type_controller = Some(player);
    card.chosen_type_revealed = false;
    true
}
