use super::{parse_param, resolve_numeric_svar, EffectContext};
use crate::spellability::SpellAbility;

/// `SP$ ChooseNumber` — the activating player chooses a number.
/// Stores the result in `source.chosen_number` for subsequent effects.
///
/// Mirrors Java's `ChooseNumberEffect.java`.
/// - `Min$` — minimum value (default 0).
/// - `Max$` — maximum value (default 10).
/// - `Random` — if true, choose randomly instead of asking.
///
/// # Card script examples
/// ```text
/// A:SP$ ChooseNumber | Min$ 0 | Max$ 5
/// A:SP$ ChooseNumber | Min$ 1 | Max$ X
/// ```
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let controller = sa.activating_player;
    let min = parse_param(&sa.ability_text, "Min$ ")
        .unwrap_or_else(|| resolve_numeric_svar(ctx.game, sa, "Min", 0));
    let max = parse_param(&sa.ability_text, "Max$ ")
        .unwrap_or_else(|| resolve_numeric_svar(ctx.game, sa, "Max", 10));

    let is_random = sa.params.get("Random")
        .map(|s| s.eq_ignore_ascii_case("True"))
        .unwrap_or(false);

    let chosen = if is_random {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        Some(rng.gen_range(min..=max))
    } else {
        ctx.agents[controller.index()].choose_number(controller, min, max)
    };

    if let Some(num) = chosen {
        if let Some(source_id) = sa.source {
            ctx.game.card_mut(source_id).chosen_number = Some(num);
        }
    }
}
