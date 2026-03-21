//! Manifest effect — put cards onto the battlefield face-down as 2/2 creatures.
//!
//! Ported from Java's `ManifestEffect.java` + `ManifestBaseEffect.java`.
//!
//! Manifest: Take the top card of a player's library (or chosen cards),
//! turn it face-down, and put it onto the battlefield as a 2/2 creature.
//! The card can be turned face-up by paying its mana cost if it's a creature.

use forge_foundation::ZoneType;
use crate::parsing::keys;

use super::{emit_zone_trigger, EffectContext};
use crate::event::{RunParams, TriggerType};
use crate::ids::{CardId, PlayerId};
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let amount = super::resolve_numeric_svar(ctx.game, sa, "Amount", 1).max(0) as usize;
    let controller = sa.activating_player;

    // Determine which player manifests (DefinedPlayer$ or controller)
    let players = if let Some(def) = sa.defined_player() {
        super::resolve_defined_players(def, controller, ctx.game)
    } else {
        vec![controller]
    };

    for pid in players {
        manifest_for_player(ctx, sa, pid, amount);
    }
}

/// Manifest N cards for a given player.
fn manifest_for_player(
    ctx: &mut EffectContext,
    sa: &SpellAbility,
    player: PlayerId,
    amount: usize,
) {
    let defined = sa.defined().unwrap_or("TopOfLibrary");

    // Determine source cards
    let cards_to_manifest: Vec<CardId> = if defined == "TopOfLibrary" || defined.is_empty() {
        // Default: top N cards of library
        let lib = ctx.game.cards_in_zone(ZoneType::Library, player).to_vec();
        lib.into_iter().rev().take(amount).collect()
    } else if let Some(choice_zone_str) = sa.params.get(crate::parsing::keys::CHOICE_ZONE) {
        // Player chooses from a specific zone
        let zone = super::parse_zone_type(choice_zone_str).unwrap_or(ZoneType::Hand);
        let zone_cards = ctx.game.cards_in_zone(zone, player).to_vec();
        if zone_cards.is_empty() {
            return;
        }
        // Let player choose
        ctx.agents[player.index()].snapshot_state(ctx.game, ctx.mana_pools);
        ctx.agents[player.index()].choose_cards_for_zone_change(
            player,
            &zone_cards,
            amount.min(zone_cards.len()),
            amount.min(zone_cards.len()),
            "Choose cards to manifest",
        )
    } else {
        // Targeted or self
        sa.target_chosen
            .target_card
            .into_iter()
            .collect()
    };

    // Manifest each card one at a time (CR 701.34d)
    for card_id in cards_to_manifest {
        manifest_single_card(ctx, sa, card_id, player);
    }
}

/// Manifest a single card: turn face-down, put on battlefield as 2/2.
fn manifest_single_card(
    ctx: &mut EffectContext,
    sa: &SpellAbility,
    card_id: CardId,
    player: PlayerId,
) {
    let old_zone = ctx.game.card(card_id).zone;

    // Turn face down
    ctx.game.card_mut(card_id).face_down = true;
    ctx.game.card_mut(card_id).manifested = true;

    // Set as 2/2 creature while face-down
    ctx.game.card_mut(card_id).base_power = Some(2);
    ctx.game.card_mut(card_id).base_toughness = Some(2);

    // Move to battlefield under the player's control
    ctx.game.card_mut(card_id).controller = player;
    ctx.game.move_card(card_id, ZoneType::Battlefield, player);

    ctx.trigger_handler
        .register_active_trigger(ctx.game, card_id);

    // RememberManifested$
    if sa.param_is_true(keys::REMEMBER_MANIFESTED) {
        if let Some(source_id) = sa.source {
            ctx.game.card_mut(source_id).add_remembered_card(card_id);
        }
    }

    emit_zone_trigger(ctx.trigger_handler, card_id, old_zone, ZoneType::Battlefield);
}
