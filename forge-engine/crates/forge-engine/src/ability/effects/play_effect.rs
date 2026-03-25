//! Implements `DB$ Play` — cast a card (often without paying its mana cost).
//!
//! Two distinct use-cases share this handler:
//!
//! * **Rebound** — exiles a spell on resolution, then casts it again on the
//!   controller's next upkeep via a delayed trigger (no cost, no optional).
//! * **Madness** — optional casting at madness cost after a discard exile.
//!   If the player declines, the card moves from exile to graveyard.
//!
//! Mirrors Java's `forge/game/ability/effects/PlayEffect.java` (simplified).

use forge_foundation::ZoneType;

use super::EffectContext;
use crate::agent::GameLogEvent;
use crate::card::PARAM_MADNESS_PLAY;
use crate::event::{RunParams, TriggerType};
use crate::ids::CardId;
use crate::parsing::keys;
use crate::spellability::{SpellAbility, StackEntry};

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let card_id = match resolve_target_card(sa) {
        Some(cid) => cid,
        None => return,
    };

    // Card must exist and not already be on the stack
    if ctx.game.card(card_id).zone == ZoneType::Stack {
        return;
    }

    let is_madness = sa.param_is_true(PARAM_MADNESS_PLAY);

    if is_madness {
        resolve_madness_play(ctx, sa, card_id);
    } else {
        resolve_rebound_play(ctx, sa, card_id);
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

/// Optional play prompt. Returns `true` if the player accepts (or if not optional).
fn prompt_optional_play(
    ctx: &mut EffectContext,
    sa: &SpellAbility,
    card_id: CardId,
    controller: crate::ids::PlayerId,
) -> bool {
    if !sa.param_is_true(keys::OPTIONAL) {
        return true;
    }
    let card_name = ctx.game.card(card_id).card_name.clone();
    ctx.agents[controller.index()].confirm_action(
        controller,
        None,
        &format!("Do you want to play {}?", card_name),
        &[],
        Some(&card_name),
        Some(crate::ability::api_type::ApiType::Play),
    )
}

/// Move a madness-exiled card to graveyard (cleanup when play is declined).
fn madness_exile_to_graveyard(ctx: &mut EffectContext, card_id: CardId) {
    if ctx.game.card(card_id).zone == ZoneType::Exile {
        let owner = ctx.game.card(card_id).owner;
        ctx.game.move_card(card_id, ZoneType::Graveyard, owner);
        super::helpers::remove_madness_exiled_marker(ctx.game.card_mut(card_id));
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
    ctx.game.move_card(card_id, ZoneType::Stack, controller);
    {
        let p = ctx.game.player_mut(controller);
        p.spells_cast_this_turn += 1;
        p.cards_cast_this_turn.push(card_id);
    }

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

// ── Madness path ──────────────────────────────────────────────────────

fn resolve_madness_play(ctx: &mut EffectContext, sa: &SpellAbility, card_id: CardId) {
    let controller = sa.activating_player;

    if !prompt_optional_play(ctx, sa, card_id, controller) {
        madness_exile_to_graveyard(ctx, card_id);
        return;
    }

    let play_cost = sa.params.get(keys::PLAY_COST).map(|s| s.to_string());

    // Pay madness mana cost BEFORE target setup (matches Java's cast flow order).
    if let Some(ref cost_str) = play_cost {
        let madness_mc = forge_foundation::ManaCost::parse(cost_str);
        let available = crate::mana::calculate_available_mana(
            &ctx.mana_pools[controller.index()],
            ctx.game,
            controller,
        );
        if !available.can_pay(&madness_mc) {
            madness_exile_to_graveyard(ctx, card_id);
            return;
        }
        let tapped = crate::mana::auto_tap_lands(
            ctx.game,
            &mut ctx.mana_pools[controller.index()],
            controller,
            &madness_mc,
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
        ctx.mana_pools[controller.index()].try_pay(&madness_mc);
    }

    // Build SA, set up targets, push to stack
    let mut spell_sa =
        crate::spellability::build_spell_ability_for_card_cast(ctx.game, card_id, controller);
    spell_sa.alt_cost = Some(crate::spellability::AlternativeCost::Madness);
    if let Some(cost) = spell_sa.pay_costs.as_mut() {
        cost.mandatory = true;
    }

    spell_sa.setup_targets(ctx.game, ctx.agents, ctx.mana_pools);

    super::helpers::remove_madness_exiled_marker(ctx.game.card_mut(card_id));
    push_spell_to_stack(ctx, card_id, spell_sa, "Madness");
}

// ── Rebound path ──────────────────────────────────────────────────────

fn resolve_rebound_play(ctx: &mut EffectContext, sa: &SpellAbility, card_id: CardId) {
    let controller = sa.activating_player;

    let mut spell_sa =
        crate::spellability::build_spell_ability_for_card_cast(ctx.game, card_id, controller);
    if let Some(cost) = spell_sa.pay_costs.as_mut() {
        cost.mandatory = true;
    }

    spell_sa.setup_targets(ctx.game, ctx.agents, ctx.mana_pools);
    push_spell_to_stack(ctx, card_id, spell_sa, "Rebound");
}
