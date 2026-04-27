use forge_foundation::ManaCost;

use crate::game::GameState;
use crate::ids::{CardId, PlayerId};

use super::{
    auto_tap_lands_trace, auto_tap_lands_with_chooser, AutoTapChoice, ManaPayCallbackFn,
    ManaPaymentContext, ManaPool, SacrificeChooser,
};

pub struct AutoPayResult {
    pub tapped: Vec<CardId>,
    pub choices: Vec<AutoTapChoice>,
    pub life_paid: i32,
    pub colors_spent: u16,
    pub paying_mana: Vec<u16>,
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
    let tapped = match sacrifice_chooser {
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
            needs_express_choice: false,
        })
        .collect();

    let payment = pool.try_pay_for_spell_converted_with_phyrexian_life_result(
        mana_cost,
        payment_ctx,
        any_color_conversion,
        game.player(player).life,
    )?;
    if commander_tax > 0 && !pool.try_pay_extra_generic(commander_tax) {
        return None;
    }
    Some(AutoPayResult {
        tapped,
        choices,
        life_paid: payment.life_paid,
        colors_spent: payment.colors_spent,
        paying_mana: payment.paying_mana,
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
    pay_mana_cost_auto_with_callback_and_reserved_sacrifices(
        game,
        pool,
        player,
        mana_cost,
        current_spell,
        commander_tax,
        payment_ctx,
        any_color_conversion,
        &[],
        callback,
    )
}

/// Same as [`pay_mana_cost_auto_with_callback`] but takes a list of permanents
/// already reserved by an additional-cost sacrifice on the current spell. The
/// auto-payer skips those permanents when picking mana abilities, so the spell's
/// `Sac<1/X>` cost and its mana payment can never double-book the same card.
///
/// Without this, the spell auto-payer (`cast_spell.rs:pay_mana_cost_session`)
/// would freely pick the very permanent the player already chose to sacrifice
/// for the additional cost, then silently drop the additional sacrifice at
/// pay-time. That's the seed-62 Eviscerator's Insight bug.
#[allow(clippy::too_many_arguments)]
pub fn pay_mana_cost_auto_with_callback_and_reserved_sacrifices(
    game: &mut GameState,
    pool: &mut ManaPool,
    player: PlayerId,
    mana_cost: &ManaCost,
    current_spell: Option<CardId>,
    commander_tax: i32,
    payment_ctx: &ManaPaymentContext,
    any_color_conversion: bool,
    reserved_sacrifices: &[CardId],
    callback: ManaPayCallbackFn<'_>,
) -> Option<AutoPayResult> {
    let mut choices =
        super::computer_util_mana::auto_tap_lands_trace_with_callbacks_and_reserved_sacrifices(
            game,
            pool,
            player,
            mana_cost,
            current_spell,
            reserved_sacrifices,
            callback,
        );
    if commander_tax > 0 {
        let tapped_tax =
            super::computer_util_mana::auto_tap_lands_trace_with_callbacks_and_reserved_sacrifices(
                game,
                pool,
                player,
                &ManaCost::generic(commander_tax),
                current_spell,
                reserved_sacrifices,
                callback,
            );
        choices.extend(tapped_tax);
    }
    let tapped = choices.iter().map(|choice| choice.card_id).collect();

    let payment = pool.try_pay_for_spell_converted_with_phyrexian_life_result(
        mana_cost,
        payment_ctx,
        any_color_conversion,
        game.player(player).life,
    )?;
    if commander_tax > 0 && !pool.try_pay_extra_generic(commander_tax) {
        return None;
    }
    Some(AutoPayResult {
        tapped,
        choices,
        life_paid: payment.life_paid,
        colors_spent: payment.colors_spent,
        paying_mana: payment.paying_mana,
    })
}
