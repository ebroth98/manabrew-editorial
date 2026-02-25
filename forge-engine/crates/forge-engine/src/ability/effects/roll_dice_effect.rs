use super::EffectContext;
use crate::spellability::{build_spell_ability, SpellAbility};

/// `SP$ RollDice` — roll a die and resolve a sub-ability based on the result.
///
/// Mirrors Java's `RollDiceEffect.java`.
/// - `Sides$` — number of sides on the die (default 20 for d20).
/// - `ResultSubAbilities$` — comma-separated list of "threshold:SVar" pairs.
///   e.g. "1:Low,10:Mid,20:High" means 1-9→Low, 10-19→Mid, 20→High.
///
/// # Card script examples
/// ```text
/// A:SP$ RollDice | Sides$ 20 | ResultSubAbilities$ 1:Low,10:Mid,20:High
/// A:SP$ RollDice | Sides$ 6 | ResultSubAbilities$ 1:Fail,4:Success
/// ```
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let controller = sa.activating_player;
    let sides = sa.params.get("Sides")
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(20);

    // Roll the die
    let result = {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        rng.gen_range(1..=sides)
    };

    let source_id = match sa.source {
        Some(id) => id,
        None => return,
    };

    // Notify agents of the roll result
    ctx.agents[controller.index()].notify(&format!("Rolled a {} (d{})", result, sides));

    // Parse ResultSubAbilities$ and find the matching threshold
    if let Some(result_str) = sa.params.get("ResultSubAbilities") {
        let mut thresholds: Vec<(i32, String)> = Vec::new();
        for entry in result_str.split(',') {
            let parts: Vec<&str> = entry.splitn(2, ':').collect();
            if parts.len() == 2 {
                if let Ok(threshold) = parts[0].trim().parse::<i32>() {
                    thresholds.push((threshold, parts[1].trim().to_string()));
                }
            }
        }
        // Sort by threshold descending to find the highest matching
        thresholds.sort_by(|a, b| b.0.cmp(&a.0));

        for (threshold, svar_name) in &thresholds {
            if result >= *threshold {
                if let Some(sub_text) = ctx.game.card(source_id).svars.get(svar_name).cloned() {
                    let sub_sa = build_spell_ability(ctx.game, source_id, &sub_text, controller);
                    resolve_sub_chain(ctx, sub_sa);
                }
                break;
            }
        }
    }
}

fn resolve_sub_chain(ctx: &mut EffectContext, initial: SpellAbility) {
    let mut cur_opt: Option<SpellAbility> = Some(initial);
    while let Some(cur_sa) = cur_opt {
        super::resolve_effect(ctx, &cur_sa);
        cur_opt = cur_sa.sub_ability.map(|b| *b);
        if ctx.game.game_over { break; }
    }
}
