//! Implements `DB$ Play` — cast a card (often without paying its mana cost).
//!
//! Used by Rebound's delayed trigger to cast the exiled spell during the
//! controller's next upkeep.
//!
//! Mirrors Java's `forge/game/ability/effects/PlayEffect.java` (simplified).

use forge_foundation::ZoneType;

use super::EffectContext;
use crate::agent::GameLogEvent;
use crate::event::{RunParams, TriggerType};
use crate::ids::CardId;
use crate::spellability::{build_spell_ability, SpellAbility, StackEntry};

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let defined = sa.params.get("Defined").cloned().unwrap_or_default();
    let controller = sa.activating_player;

    // Resolve the target card from Defined$ parameter
    let card_id = if let Some(uid_str) = defined.strip_prefix("CardUID_") {
        uid_str.parse::<u32>().ok().map(CardId)
    } else if defined.eq_ignore_ascii_case("Self") {
        sa.source
    } else {
        sa.source
    };

    let card_id = match card_id {
        Some(cid) => cid,
        None => return,
    };

    // Card must exist and not already be on the stack
    if ctx.game.card(card_id).zone == ZoneType::Stack {
        return;
    }

    // Build a spell ability from the card's abilities
    let abilities = ctx.game.card(card_id).abilities.clone();
    let ability_text = abilities.first().cloned().unwrap_or_default();
    let mut spell_sa = build_spell_ability(ctx.game, card_id, &ability_text, controller);
    spell_sa.is_spell = true;

    // Choose targets if the spell needs them
    spell_sa.setup_targets(ctx.game, ctx.agents, ctx.mana_pools);
    let chosen_target = spell_sa.target_chosen.target_card;

    let is_creature = ctx.game.card(card_id).is_creature();
    let is_permanent = ctx.game.card(card_id).is_permanent();
    let card_name = ctx.game.card(card_id).card_name.clone();
    let cast_zone = Some(ctx.game.card(card_id).zone);

    let entry = StackEntry {
        id: 0,
        spell_ability: spell_sa,
        is_creature_spell: is_creature,
        is_permanent_spell: is_permanent,
        cast_from_zone: cast_zone,
    };

    // Push onto stack
    ctx.game.stack.push(entry);

    // Move card from its current zone (exile) to the stack
    ctx.game.move_card(card_id, ZoneType::Stack, controller);

    // Count as a spell cast
    ctx.game.player_mut(controller).spells_cast_this_turn += 1;

    // Emit SpellCast trigger
    ctx.trigger_handler.run_trigger(
        TriggerType::SpellCast,
        RunParams {
            spell_card: Some(card_id),
            spell_controller: Some(controller),
            ..Default::default()
        },
        false,
    );

    let mut event = GameLogEvent::stack(format!("Rebound: cast {}", card_name))
        .with_player(controller)
        .with_source_card(card_id);
    if let Some(target_id) = chosen_target {
        event = event.with_target_card(target_id);
    }
    crate::agent::notify_all_agents(ctx.agents, event);
}
