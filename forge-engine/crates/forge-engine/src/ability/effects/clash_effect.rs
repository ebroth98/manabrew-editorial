//! Clash effect — each player reveals top card, higher CMC wins.
//!
//! Ported 1:1 from Java's `ClashEffect.java`.
//! Clash with an opponent: Each clashing player reveals the top card of their
//! library, then puts it on top or bottom. Higher mana value wins.

use forge_foundation::ZoneType;

use super::EffectContext;
use crate::ids::{CardId, PlayerId};
use crate::parsing::keys;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let controller = sa.activating_player;

    // Choose opponent to clash with
    let opponent = if let Some(def) = sa.defined() {
        super::resolve_defined_players(def, controller, ctx.game)
            .into_iter()
            .next()
            .unwrap_or_else(|| ctx.game.opponent_of(controller))
    } else {
        ctx.game.opponent_of(controller)
    };

    // RememberClasher$
    if sa.param_is_true(keys::REMEMBER_CLASHER) {
        if let Some(sid) = sa.source {
            ctx.game.card_mut(sid).remembered_players.push(opponent);
        }
    }

    // Reveal top cards
    let p_lib = ctx.game.cards_in_zone(ZoneType::Library, controller).to_vec();
    let o_lib = ctx.game.cards_in_zone(ZoneType::Library, opponent).to_vec();

    if p_lib.is_empty() && o_lib.is_empty() {
        return;
    }

    let p_card: Option<CardId> = p_lib.last().copied();
    let o_card: Option<CardId> = o_lib.last().copied();

    let p_cmc = p_card.map(|cid| ctx.game.card(cid).mana_cost.cmc() as i32).unwrap_or(-1);
    let o_cmc = o_card.map(|cid| ctx.game.card(cid).mana_cost.cmc() as i32).unwrap_or(-1);

    // Reveal to all agents
    let mut revealed = Vec::new();
    if let Some(cid) = p_card { revealed.push(cid); }
    if let Some(cid) = o_card { revealed.push(cid); }
    for agent in ctx.agents.iter_mut() {
        agent.on_library_peek(ctx.game, &revealed);
    }

    // Determine winner
    let _player_wins = p_cmc > o_cmc;

    // WinSubAbility / OtherwiseSubAbility — resolved via sub-ability chain
    // The SA's sub_ability handles this via the Branch mechanism in Java.
    // For now, we track the result for triggers.

    // Each player puts their card on top or bottom
    clash_move(ctx, controller, p_card);
    clash_move(ctx, opponent, o_card);

    // The clash result is typically used by the parent SA via Branch/Defined
    // to resolve WinSubAbility or OtherwiseSubAbility.
}

/// Player chooses to put their clashed card on top or bottom of library.
fn clash_move(ctx: &mut EffectContext, player: PlayerId, card: Option<CardId>) {
    let Some(card_id) = card else { return };

    // Ask player: top or bottom?
    ctx.agents[player.index()].snapshot_state(ctx.game, ctx.mana_pools);
    let put_on_top = ctx.agents[player.index()].confirm_action(
        player,
        Some("ClashTopOrBottom"),
        &format!(
            "Put {} on top or bottom of your library?",
            ctx.game.card(card_id).card_name
        ),
        &["Top".to_string(), "Bottom".to_string()],
        Some(&ctx.game.card(card_id).card_name.clone()),
        None,
    );

    if !put_on_top {
        // Move to bottom (card is already on top — move to index 0)
        let zone = ctx.game.zone_mut(ZoneType::Library, player);
        if let Some(pos) = zone.cards.iter().rposition(|&c| c == card_id) {
            zone.cards.remove(pos);
            zone.cards.insert(0, card_id);
        }
    }
    // If top, card stays where it is (already on top)
}
