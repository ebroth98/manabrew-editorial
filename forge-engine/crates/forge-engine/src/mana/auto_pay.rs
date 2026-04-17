use forge_foundation::ManaCost;

use crate::game::GameState;
use crate::ids::{CardId, PlayerId};

use super::{
    auto_tap_lands_trace, auto_tap_lands_trace_with_callbacks, auto_tap_lands_with_chooser,
    AutoTapChoice, ManaPayCallbackFn, ManaPaymentContext, ManaPool, SacrificeChooser,
};

pub struct AutoPayResult {
    pub tapped: Vec<CardId>,
    pub choices: Vec<AutoTapChoice>,
    pub life_paid: i32,
    pub colors_spent: u16,
}

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
) -> Option<AutoPayResult> {
    pay_mana_cost_auto_with_chooser(
        game,
        pool,
        player,
        mana_cost,
        current_spell,
        commander_tax,
        payment_ctx,
        any_color_conversion,
        None,
    )
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
) -> Option<AutoPayResult> {
    let mut tapped = match sacrifice_chooser {
        Some(chooser) => {
            let mut tapped =
                auto_tap_lands_with_chooser(game, pool, player, mana_cost, current_spell, chooser);
            if commander_tax > 0 {
                tapped.extend(auto_tap_lands_with_chooser(
                    game,
                    pool,
                    player,
                    &ManaCost::generic(commander_tax),
                    current_spell,
                    chooser,
                ));
            }
            tapped
        }
        None => {
            let mut choices = auto_tap_lands_trace(game, pool, player, mana_cost, current_spell);
            if commander_tax > 0 {
                choices.extend(auto_tap_lands_trace(
                    game,
                    pool,
                    player,
                    &ManaCost::generic(commander_tax),
                    current_spell,
                ));
            }
            choices.into_iter().map(|choice| choice.card_id).collect()
        }
    };
    let choices = tapped
        .iter()
        .map(|&card_id| AutoTapChoice {
            card_id,
            mana_ability_index: None,
            chosen_atom: 0,
        })
        .collect();

    let Some(payment) = pool.try_pay_for_spell_converted_with_phyrexian_life_result(
        mana_cost,
        payment_ctx,
        any_color_conversion,
        game.player(player).life,
    ) else {
        return None;
    };
    if commander_tax > 0 && !pool.try_pay_extra_generic(commander_tax) {
        return None;
    }
    Some(AutoPayResult {
        tapped,
        choices,
        life_paid: payment.life_paid,
        colors_spent: payment.colors_spent,
    })
}

/// Same as [`pay_mana_cost_auto`] but accepts the unified callback for both
/// sacrifice chooser and confirm payment (Treasure Token self-sacrifice).
pub fn pay_mana_cost_auto_with_callback(
    game: &mut GameState,
    pool: &mut ManaPool,
    player: PlayerId,
    mana_cost: &ManaCost,
    current_spell: Option<CardId>,
    commander_tax: i32,
    payment_ctx: &ManaPaymentContext,
    any_color_conversion: bool,
    callback: ManaPayCallbackFn<'_>,
) -> Option<AutoPayResult> {
    let mut choices =
        auto_tap_lands_trace_with_callbacks(game, pool, player, mana_cost, current_spell, callback);
    if commander_tax > 0 {
        let tapped_tax = auto_tap_lands_trace_with_callbacks(
            game,
            pool,
            player,
            &ManaCost::generic(commander_tax),
            current_spell,
            callback,
        );
        choices.extend(tapped_tax);
    }
    let tapped = choices.iter().map(|choice| choice.card_id).collect();

    let Some(payment) = pool.try_pay_for_spell_converted_with_phyrexian_life_result(
        mana_cost,
        payment_ctx,
        any_color_conversion,
        game.player(player).life,
    ) else {
        return None;
    };
    if commander_tax > 0 && !pool.try_pay_extra_generic(commander_tax) {
        return None;
    }
    Some(AutoPayResult {
        tapped,
        choices,
        life_paid: payment.life_paid,
        colors_spent: payment.colors_spent,
    })
}
