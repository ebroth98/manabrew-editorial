use super::EffectContext;
use crate::agent::GameLogEvent;
use crate::replacement::replacement_handler::{apply_replacements, ReplacementEvent};
use crate::replacement::ReplacementResult;
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

    // Run RollDice replacement effects before rolling.
    let mut event = ReplacementEvent::RollDice {
        player: controller,
    };
    let result = apply_replacements(ctx.game, &mut event);
    if result == ReplacementResult::Skipped || result == ReplacementResult::Replaced {
        return;
    }

    let sides = sa
        .params
        .as_i32(crate::parsing::keys::SIDES)
        .unwrap_or(20);

    // Roll the die
    let result = ctx.rng.next_int(sides) + 1;

    let source_id = match sa.source {
        Some(id) => id,
        None => return,
    };

    // Notify agents of the roll result
    crate::agent::notify_all_agents(
        ctx.agents,
        GameLogEvent::rule(format!("Rolled a {} (d{})", result, sides)).with_player(controller),
    );

    // Parse ResultSubAbilities$ and find the matching threshold
    if let Some(result_str) = sa.params.get(crate::parsing::keys::RESULT_SUB_ABILITIES) {
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
        if ctx.game.game_over {
            break;
        }
    }
}
