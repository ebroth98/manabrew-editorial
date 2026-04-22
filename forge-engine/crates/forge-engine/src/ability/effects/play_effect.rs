use forge_foundation::ZoneType;

use super::EffectContext;
use crate::agent::{GameEntity, GameLogEvent};
use crate::event::RunParams;
use crate::ids::CardId;
use crate::parsing::keys;
use crate::spellability::{SpellAbility, StackEntry};
use crate::trigger::TriggerType;

/// Struct form of this effect so it can participate in the
/// `SpellAbilityEffect` trait hierarchy — mirrors Java's
/// `PlayEffect` class extending `SpellAbilityEffect`.
#[forge_engine_macros::spell_effect(PlayEffect)]
fn resolve(ctx: &mut EffectContext, sa: &crate::spellability::SpellAbility) {
    let card_id = match resolve_target_card(sa) {
        Some(cid) => cid,
        None => return,
    };

    if ctx.game.card(card_id).zone == ZoneType::Stack {
        return;
    }

    let controller = sa.activating_player;
    let without_mana_cost = sa.params.has("WithoutManaCost");
    let play_cost = sa.params.get(keys::PLAY_COST).map(|s| s.to_string());
    let remember = sa.param_is_true("RememberPlayed");
    let optional = sa.param_is_true(keys::OPTIONAL);
    let is_madness = sa.params.get(keys::PLAY_COST).is_some()
        && sa
            .source
            .map_or(false, |src| ctx.game.card(src).has_keyword("Madness"));

    // ── Step 1: Choose card — mirrors Java PlayEffect.java line 234 ──
    // chooseSingleEntityForEffect(tgtCards, sa, ..., !singleOption && optional, ...)
    // For single-card + optional: isOptional=false (auto-pick), then confirmAction below.
    let tgt_cards = vec![GameEntity::Card(card_id)];
    let chosen = ctx.agents[controller.index()].choose_single_entity_for_effect(
        controller, &tgt_cards, false, // !singleOption && optional = false for single card
    );
    let card_id = match chosen {
        Some(GameEntity::Card(cid)) => cid,
        None => return,
        Some(GameEntity::Player(_)) => return,
    };

    // ── Step 2: Optional confirm — mirrors Java PlayEffect.java line 250 ──
    // if (singleOption && !confirmAction(...)) break;
    if optional {
        let card_name = ctx.game.card(card_id).card_name.clone();
        let accepted = ctx.agents[controller.index()].confirm_action(
            controller,
            None,
            &format!("Do you want to play {}?", card_name),
            &[],
            Some(&card_name),
            Some(crate::ability::api_type::ApiType::Play),
        );
        if !accepted {
            return;
        }
    }

    // ── Step 3: Get ability to play — mirrors Java PlayEffect.java line 318 ──
    // tgtSA = controller.getController().getAbilityToPlay(tgtCard, sas);
    let spell_sa_base =
        crate::spellability::build_spell_ability_for_card_cast(ctx.game, card_id, controller);
    let abilities = vec![spell_sa_base];
    let sa_idx = ctx.agents[controller.index()].get_ability_to_play(controller, &abilities);
    let mut spell_sa = match sa_idx {
        Some(idx) => abilities.into_iter().nth(idx).unwrap(),
        None => return,
    };

    // Mirror Java's DeterministicPlayPlumbing.playSaFromPlayEffect():
    // optional play-effect spells consume one more boolean after ability
    // selection and before the spell is actually played.
    let play_effect_optional = spell_sa
        .pay_costs
        .as_ref()
        .map(|cost| !cost.mandatory)
        .unwrap_or(false);
    if play_effect_optional {
        let card_name = ctx.game.card(card_id).card_name.clone();
        let accepted = ctx.agents[controller.index()].confirm_action(
            controller,
            Some("PlayEffectOptional"),
            "play_effect_optional",
            &[],
            Some(&card_name),
            Some(crate::ability::api_type::ApiType::Play),
        );
        if !accepted {
            return;
        }
    }

    // ── Step 3: Cost replacement ────────────────────────────────────
    if without_mana_cost {
        if let Some(ref mut cost) = spell_sa.pay_costs {
            cost.parts
                .retain(|part| !matches!(part, crate::cost::CostPart::Mana { .. }));
        }
    } else if let Some(ref cost_str) = play_cost {
        let alt_mc = forge_foundation::ManaCost::parse(cost_str);
        if let Some(ref existing) = spell_sa.pay_costs {
            spell_sa.pay_costs = Some(existing.copy_with_defined_mana(alt_mc));
        }
    }

    // ── Step 4: Alt-cost flags ──────────────────────────────────────
    if is_madness {
        spell_sa.alt_cost = Some(crate::spellability::AlternativeCost::Madness);
    }

    // Java PlayEffect.java line 398: setMandatory(true) for 118.8c
    if let Some(ref mut cost) = spell_sa.pay_costs {
        cost.mandatory = true;
    }

    // Remove zone restriction — allow casting from exile/library/etc.
    spell_sa
        .params
        .put("CastFromPlayEffect".to_string(), "True".to_string());

    // ── Step 5: Pay mana ────────────────────────────────────────────
    if !without_mana_cost {
        let mc = if let Some(ref cost_str) = play_cost {
            forge_foundation::ManaCost::parse(cost_str)
        } else {
            ctx.game.card(card_id).mana_cost.clone()
        };

        let available = crate::mana::calculate_available_mana(
            &ctx.mana_pools[controller.index()],
            ctx.game,
            controller,
        );
        if !available.can_pay(&mc) {
            return;
        }
        let tapped = crate::mana::auto_tap_lands(
            ctx.game,
            &mut ctx.mana_pools[controller.index()],
            controller,
            &mc,
            Some(card_id),
        );
        for &land_id in &tapped {
            ctx.trigger_handler.run_trigger(
                TriggerType::TapsForMana,
                RunParams {
                    card: Some(land_id),
                    player: Some(controller),
                    ..Default::default()
                },
                false,
            );
        }
        ctx.mana_pools[controller.index()].try_pay(&mc);
    }

    // ── Step 6: Set up targets ──────────────────────────────────────
    spell_sa.setup_targets(ctx.game, ctx.agents, ctx.mana_pools);

    // ── Step 7: Push to stack ───────────────────────────────────────
    let label = if is_madness {
        "Madness"
    } else if without_mana_cost {
        "Rebound"
    } else {
        "Play"
    };
    push_spell_to_stack(ctx, card_id, spell_sa, label);

    // ── Step 8: RememberPlayed ──────────────────────────────────────
    // Mirrors Java PlayEffect.java line 457:
    //   if (remember) source.addRemembered(played);
    // The sub-ability chain (ChangeZone + Cleanup) uses ConditionCompare$ EQ0
    // on Remembered to decide whether to move the card to graveyard.
    if remember {
        if let Some(source_id) = sa.source {
            ctx.game.card_mut(source_id).remembered_cards.push(card_id);
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────

/// Resolve the target card from `Defined$` parameter.
fn resolve_target_card(sa: &SpellAbility) -> Option<CardId> {
    let defined = sa
        .params
        .get(keys::DEFINED)
        .map(|s| s.to_string())
        .unwrap_or_default();
    if let Some(uid_str) = defined.strip_prefix("CardUID_") {
        uid_str.parse::<u32>().ok().map(CardId)
    } else {
        // "Self" or fallback — use the source card
        sa.source
    }
}

/// Push the spell onto the stack and fire SpellCast trigger.
fn push_spell_to_stack(
    ctx: &mut EffectContext,
    card_id: CardId,
    spell_sa: SpellAbility,
    label: &str,
) {
    let controller = spell_sa.activating_player;
    let is_creature = ctx.game.card(card_id).is_creature();
    let is_permanent = ctx.game.card(card_id).is_permanent();
    let cast_zone = Some(ctx.game.card(card_id).zone);
    let card_name = ctx.game.card(card_id).card_name.clone();
    let chosen_target = spell_sa.target_chosen.target_card;

    let entry = StackEntry {
        id: 0,
        spell_ability: spell_sa,
        is_creature_spell: is_creature,
        is_permanent_spell: is_permanent,
        cast_from_zone: cast_zone,
        optional_trigger_decider: None,
        optional_trigger_description: None,
        optional_trigger_source_name: None,
    };
    let trigger_sa = entry.spell_ability.clone();

    ctx.game.stack.push(entry);
    ctx.move_card(card_id, ZoneType::Stack, controller);
    ctx.game.player_record_spell_cast(controller, card_id);

    ctx.trigger_handler.run_trigger(
        TriggerType::SpellCast,
        RunParams {
            spell_card: Some(card_id),
            spell_controller: Some(controller),
            source_sa: Some(trigger_sa.clone()),
            ..Default::default()
        },
        false,
    );
    super::emit_targeting_triggers(ctx, card_id, &trigger_sa);

    let mut event = GameLogEvent::stack(format!("{}: cast {}", label, card_name))
        .with_player(controller)
        .with_source_card(card_id);
    if let Some(target_id) = chosen_target {
        event = event.with_target_card(target_id);
    }
    crate::agent::notify_all_agents(ctx.agents, event);
}

/// Create a replacement effect that exiles a card instead of putting it into
/// the graveyard from the stack. Mirrors Java `PlayEffect.addReplaceGraveyardEffect`.
pub fn add_replace_graveyard_effect(
    ctx: &mut EffectContext,
    card_id: CardId,
    _host_card: CardId,
    sa: &SpellAbility,
    zone: &str,
) {
    let controller = sa.activating_player;

    let effect_card = crate::card::Card::new(
        CardId(0),
        "ReplaceGraveyard Effect".to_string(),
        controller,
        forge_foundation::CardTypeLine::default(),
        forge_foundation::ManaCost::no_cost(),
        forge_foundation::ColorSet::COLORLESS,
        None,
        None,
        vec![],
        vec![],
    );
    let effect_id = ctx.game.create_card(effect_card);

    ctx.game.card_mut(effect_id).remembered_cards.push(card_id);

    let dest_zone = if zone.is_empty() { "Exile" } else { zone };
    ctx.game.card_mut(effect_id).set_s_var(
        "ReplacementEffect",
        "Event$ Moved | ValidCard$ Card.IsRemembered | Origin$ Stack | Destination$ Graveyard | Description$ If that card would be put into your graveyard this turn, exile it instead.".to_string(),
    );
    ctx.game
        .card_mut(effect_id)
        .set_s_var("ReplacementDestination", dest_zone.to_string());

    ctx.game.move_card(effect_id, ZoneType::Command, controller);

    ctx.game
        .card_mut(effect_id)
        .set_s_var("ExileAtEndOfTurn", "True".to_string());
}
