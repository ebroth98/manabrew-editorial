use forge_foundation::ManaCost;

use crate::game::GameState;
use crate::ids::{CardId, PlayerId};

use super::{auto_tap_lands, auto_tap_lands_generic, auto_tap_lands_with_chooser, ManaPaymentContext, ManaPool, SacrificeChooser};

/// Deterministic auto-pay entrypoint used by parity AI paths.
///
/// This mirrors the harness `AutoPay.java` flow at a high level:
/// 1) auto-activate legal mana sources for the required spell cost
/// 2) auto-activate additional sources for commander tax
/// 3) pay exactly the required mana from pool with engine restriction checks
pub fn pay_mana_cost_auto(
    game: &mut GameState,
    pool: &mut ManaPool,
    player: PlayerId,
    mana_cost: &ManaCost,
    current_spell: Option<CardId>,
    commander_tax: i32,
    payment_ctx: &ManaPaymentContext,
    any_color_conversion: bool,
) -> Option<Vec<CardId>> {
    pay_mana_cost_auto_with_chooser(game, pool, player, mana_cost, current_spell, commander_tax, payment_ctx, any_color_conversion, None)
}

/// Same as [`pay_mana_cost_auto`] but accepts an optional sacrifice chooser
/// callback for parity with Java's `choosePermanentsToSacrifice` RNG path.
pub fn pay_mana_cost_auto_with_chooser(
    game: &mut GameState,
    pool: &mut ManaPool,
    player: PlayerId,
    mana_cost: &ManaCost,
    current_spell: Option<CardId>,
    commander_tax: i32,
    payment_ctx: &ManaPaymentContext,
    any_color_conversion: bool,
    sacrifice_chooser: Option<SacrificeChooser<'_>>,
) -> Option<Vec<CardId>> {
    let mut tapped = if let Some(chooser) = sacrifice_chooser {
        auto_tap_lands_with_chooser(game, pool, player, mana_cost, current_spell, chooser)
    } else {
        auto_tap_lands(game, pool, player, mana_cost, current_spell)
    };
    if commander_tax > 0 {
        let tapped_tax = auto_tap_lands_generic(game, pool, player, commander_tax);
        tapped.extend(tapped_tax);
    }

    if !pool.try_pay_for_spell_converted(mana_cost, payment_ctx, any_color_conversion) {
        return None;
    }
    if commander_tax > 0 && !pool.try_pay_extra_generic(commander_tax) {
        return None;
    }
    Some(tapped)
}
