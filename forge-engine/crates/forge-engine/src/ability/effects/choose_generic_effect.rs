//! ChooseGeneric effect — generic modal choice.

use super::EffectContext;
use crate::parsing::keys;
use crate::spellability::{build_spell_ability, SpellAbility};

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let source_id = match sa.source {
        Some(id) => id,
        None => return,
    };

    let player = sa.activating_player;

    // Collect SVar names for each choice — mirrors Java's
    // sa.getAdditionalAbilityList("Choices")
    let choices_str = sa.params.get_cloned(keys::CHOICES).unwrap_or_default();
    if choices_str.is_empty() {
        return;
    }
    let choice_svars: Vec<&str> = choices_str.split(',').map(|s| s.trim()).collect();

    // Resolve SVar names to ability text from the source card
    let svars = ctx.game.card(source_id).svars.clone();
    let choice_texts: Vec<String> = choice_svars
        .iter()
        .filter_map(|svar| svars.get(*svar).cloned())
        .collect();

    if choice_texts.is_empty() {
        return;
    }

    // Build SpellAbility for each choice
    let mut abilities: Vec<SpellAbility> = choice_texts
        .iter()
        .map(|text| {
            let mut choice_sa = build_spell_ability(ctx.game, source_id, text, player);
            choice_sa.source = Some(source_id);
            choice_sa.trigger_remembered_amount = sa.trigger_remembered_amount;
            choice_sa
        })
        .collect();

    // NumRandomChoices — trim list randomly
    if let Some(n_str) = sa.params.get("NumRandomChoices") {
        let n = crate::ability::ability_utils::calculate_amount(n_str) as usize;
        while abilities.len() > n {
            let idx = ctx.rng.next_int(abilities.len() as i32) as usize;
            abilities.remove(idx);
        }
    }

    // Filter out choices whose restrictions fail or whose UnlessCost can't be paid.
    // Mirrors Java ChooseGenericEffect.java lines 69-80.
    abilities.retain(|choice_sa| {
        if let Some(unless_cost_str) = choice_sa.params.get(keys::UNLESS_COST) {
            let cost = crate::cost::parse_cost(unless_cost_str);
            crate::cost::can_pay_with_ability(
                &cost,
                ctx.game,
                &ctx.mana_pools[player.index()],
                source_id,
                player,
                Some(choice_sa),
            )
        } else {
            true
        }
    });

    if abilities.is_empty() {
        // No valid choices — resolve fallback if present
        if let Some(fallback_name) = sa.params.get("FallbackAbility") {
            let svars = ctx.game.card(source_id).svars.clone();
            if let Some(fallback_text) = svars.get(fallback_name) {
                let mut fallback_sa =
                    build_spell_ability(ctx.game, source_id, fallback_text, player);
                fallback_sa.source = Some(source_id);
                super::resolve_effect(ctx, &fallback_sa);
            }
        }
        return;
    }

    let amount = sa
        .params
        .get("ChoiceAmount")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1);

    let mut chosen_sas: Vec<SpellAbility>;

    if sa.params.has(keys::AT_RANDOM) {
        // AtRandom — pick randomly
        chosen_sas = Vec::new();
        for _ in 0..amount.min(abilities.len()) {
            if abilities.is_empty() {
                break;
            }
            let idx = ctx.rng.next_int(abilities.len() as i32) as usize;
            chosen_sas.push(abilities.remove(idx));
        }

        // Urza variant — chosen abilities that use targeting need targets selected.
        // If no valid candidates, re-pick a different random ability.
        // Mirrors Java's AtRandom="Urza" loop.
        if sa.params.get(keys::AT_RANDOM) == Some("Urza") {
            let mut i = 0;
            while i < chosen_sas.len() {
                if !chosen_sas[i].uses_targeting() {
                    i += 1;
                } else if chosen_sas[i]
                    .target_restrictions
                    .as_ref()
                    .map(|tr| tr.has_candidates(ctx.game, player, Some(source_id)))
                    .unwrap_or(false)
                {
                    // Mirrors Java: p.getController().chooseTargetsFor(chosenSAs.get(i))
                    ctx.agents[player.index()].choose_targets_for(
                        &mut chosen_sas[i],
                        ctx.game,
                        ctx.mana_pools,
                    );
                    i += 1;
                } else {
                    // No valid candidates — replace with a different random ability
                    if !abilities.is_empty() {
                        let idx = ctx.rng.next_int(abilities.len() as i32) as usize;
                        chosen_sas[i] = abilities.remove(idx);
                    } else {
                        // No alternatives left, skip
                        i += 1;
                    }
                }
            }
        }
    } else {
        // Player chooses — mirrors Java's chooseSpellAbilitiesForEffect
        ctx.agents[player.index()].snapshot_state(ctx.game, ctx.mana_pools);
        let chosen_indices = ctx.agents[player.index()].choose_spell_abilities_for_effect(
            player,
            &abilities,
            amount,
        );

        chosen_sas = chosen_indices
            .into_iter()
            .filter_map(|i| abilities.get(i).cloned())
            .collect();
    }

    // Resolve each chosen sub-ability
    if !chosen_sas.is_empty() {
        for chosen_sa in chosen_sas {
            // Walk the sub-ability chain: resolve each node
            let mut cur_opt: Option<SpellAbility> = Some(chosen_sa);
            while let Some(cur_sa) = cur_opt {
                super::resolve_effect(ctx, &cur_sa);
                cur_opt = cur_sa.sub_ability.map(|b| *b);
                if ctx.game.game_over {
                    break;
                }
            }
            if ctx.game.game_over {
                break;
            }
        }
    } else {
        // No valid choices — resolve fallback ability if present
        // Mirrors Java's FallbackAbility handling
        if let Some(fallback_name) = sa.params.get("FallbackAbility") {
            if let Some(fallback_text) = svars.get(fallback_name) {
                let mut fallback_sa =
                    build_spell_ability(ctx.game, source_id, fallback_text, player);
                fallback_sa.source = Some(source_id);
                super::resolve_effect(ctx, &fallback_sa);
            }
        }
    }
}
