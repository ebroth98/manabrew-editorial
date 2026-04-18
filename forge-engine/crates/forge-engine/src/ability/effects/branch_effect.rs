use super::EffectContext;
use crate::parsing::keys;
use crate::spellability::SpellAbility;

/// `DB$ Branch` — resolve one of two sub-abilities based on a condition SVar.
/// Mirrors Java `BranchEffect`.
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let take_true_branch = evaluate_branch_condition(ctx, sa);
    let key = if take_true_branch {
        keys::TRUE_SUB_ABILITY
    } else {
        keys::FALSE_SUB_ABILITY
    };

    let Some(sub_svar_name) = sa.params.get(key) else {
        return;
    };
    let Some(source_id) = sa.source else {
        return;
    };
    let Some(sub_text) = ctx
        .game
        .card(source_id)
        .get_s_var(sub_svar_name)
        .map(str::to_string)
    else {
        return;
    };

    let mut sub_sa = crate::spellability::build_spell_ability(
        ctx.game,
        source_id,
        &sub_text,
        sa.activating_player,
    );
    sub_sa.target_chosen = sa.target_chosen.clone();
    sub_sa.trigger_source = sa.trigger_source;
    sub_sa.trigger_index = sa.trigger_index;
    sub_sa.trigger_remembered_amount = sa.trigger_remembered_amount;
    sub_sa.x_mana_cost_paid = sa.x_mana_cost_paid;
    sub_sa.kicked = sa.kicked;
    sub_sa.kick_count = sa.kick_count;
    sub_sa.buyback_paid = sa.buyback_paid;
    sub_sa.overloaded = sa.overloaded;
    sub_sa.replicate_count = sa.replicate_count;
    sub_sa.is_copy = sa.is_copy;

    // Walk the full sub-ability chain, just like Java's AbilityUtils.resolve()
    // which follows getSubAbility() after each node. Without this, linked effects
    // (e.g. DBShuffle after DBChangeZoneAll2 in Celestial Reunion) would be skipped.
    let mut cur_opt: Option<SpellAbility> = Some(sub_sa);
    while let Some(cur_sa) = cur_opt {
        super::resolve_effect(ctx, &cur_sa);
        cur_opt = cur_sa.sub_ability.map(|b| *b);
        if ctx.game.game_over {
            break;
        }
    }
}

fn evaluate_branch_condition(ctx: &EffectContext, sa: &SpellAbility) -> bool {
    let Some(condition_svar) = sa.params.get(keys::BRANCH_CONDITION_SVAR) else {
        return true;
    };
    let Some(source_id) = sa.source else {
        return false;
    };
    let Some(expr) = ctx.game.card(source_id).get_s_var(condition_svar) else {
        return false;
    };

    if let Some(valid_filter) = expr.strip_prefix("Remembered$Valid ") {
        let remembered = ctx.game.card(source_id).remembered_cards.clone();
        if remembered.is_empty() {
            return false;
        }
        if valid_filter.eq_ignore_ascii_case("Card.ChosenType") {
            let Some(chosen_type) = ctx.game.card(source_id).chosen_type.clone() else {
                return false;
            };
            return remembered
                .iter()
                .copied()
                .any(|cid| ctx.game.card(cid).type_line.has_subtype(&chosen_type));
        }
        return remembered.iter().copied().any(|cid| {
            super::matches_valid_cards(ctx.game.card(cid), valid_filter, sa.activating_player)
        });
    }

    super::evaluate_svar(expr, sa) > 0
}
