use super::EffectContext;
use crate::spellability::{build_spell_ability, SpellAbility};

/// `SP$ FlipACoin` — flip a coin, resolve different abilities for win/lose.
///
/// Mirrors Java's `FlipCoinEffect.java`.
/// - `NoCall` — if set, don't ask the player to call; just resolve HeadsSub$/TailsSub$.
/// - `WinSubAbility$` / `LoseSubAbility$` — SVars for the sub-abilities to resolve.
/// - `HeadsSubAbility$` / `TailsSubAbility$` — used with NoCall.
///
/// # Card script examples
/// ```text
/// A:SP$ FlipACoin | WinSubAbility$ Win | LoseSubAbility$ Lose
/// A:SP$ FlipACoin | NoCall$ True | HeadsSubAbility$ Heads | TailsSubAbility$ Tails
/// ```
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let controller = sa.activating_player;
    let no_call = sa
        .params
        .get("NoCall")
        .map(|s| s.eq_ignore_ascii_case("True"))
        .unwrap_or(false);

    // Flip the coin (random)
    let is_heads = ctx.rng.next_int(2) == 0;

    let source_id = match sa.source {
        Some(id) => id,
        None => return,
    };

    if no_call {
        // No-call mode: resolve HeadsSub$ or TailsSub$ directly
        let sub_key = if is_heads {
            "HeadsSubAbility"
        } else {
            "TailsSubAbility"
        };
        if let Some(sub_svar) = sa.params.get(sub_key) {
            if let Some(sub_text) = ctx.game.card(source_id).svars.get(sub_svar).cloned() {
                let sub_sa = build_spell_ability(ctx.game, source_id, &sub_text, controller);
                resolve_sub_chain(ctx, sub_sa);
            }
        }
    } else {
        // Call mode: player calls heads/tails
        let called_heads = ctx.agents[controller.index()].flip_coin_call(controller);
        let won = called_heads == is_heads;

        let sub_key = if won {
            "WinSubAbility"
        } else {
            "LoseSubAbility"
        };
        if let Some(sub_svar) = sa.params.get(sub_key) {
            if let Some(sub_text) = ctx.game.card(source_id).svars.get(sub_svar).cloned() {
                let sub_sa = build_spell_ability(ctx.game, source_id, &sub_text, controller);
                resolve_sub_chain(ctx, sub_sa);
            }
        }
    }
}

fn resolve_sub_chain(ctx: &mut EffectContext, initial: SpellAbility) {
    let mut cur_opt: Option<SpellAbility> = Some(initial);
    while let Some(cur_sa) = cur_opt {
        super::resolve_effect(ctx, &cur_sa);
        cur_opt = cur_sa.sub_ability.map(|b| *b);
        if ctx.game.game_over {
            break;
        }
    }
}
