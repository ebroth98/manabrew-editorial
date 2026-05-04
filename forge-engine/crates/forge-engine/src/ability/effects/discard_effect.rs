use forge_foundation::ZoneType;

use super::EffectContext;
use crate::ability::spell_ability_effect::get_target_players;

/// SP$ Discard — target/defined players discard N cards.
///
/// Mirrors Java's `DiscardEffect.resolve()`.
/// Struct form of this effect so it can participate in the
/// `SpellAbilityEffect` trait hierarchy — mirrors Java's
/// `DiscardEffect` class extending `SpellAbilityEffect`.
#[forge_engine_macros::spell_effect(DiscardEffect)]
fn resolve(ctx: &mut EffectContext, sa: &crate::spellability::SpellAbility) {
    let num = super::resolve_numeric_svar(ctx.game, sa, crate::parsing::keys::NUM_CARDS, 1).max(0)
        as usize;
    let mode = sa.ir.mode_text.as_deref().unwrap_or("TgtChoose");

    // AnyNumber$ True — the discarder may pick 0..=hand.len cards (e.g.
    // Cavalier of Flame, Careful Study). Routed through a distinct agent
    // method so deterministic agents can sample a count + selection with the
    // same RNG trajectory Java uses via its AnyNumber chooser.
    let any_number = sa.ir.any_number;

    // Mode$ Random — discard at random (e.g. Hypnotic Specter).
    // Mirrors Java's DiscardEffect which calls Aggregates.random() bypassing the controller.
    // We route through the agent's choose_random_discard so deterministic agents can
    // use their seeded RNG for parity testing.
    let is_random = mode.eq_ignore_ascii_case("Random");

    let target_players = get_target_players(ctx.game, sa);
    let first_target = target_players.first().copied();

    for target_player in target_players.iter().copied() {
        let mut hand: Vec<_> = ctx
            .game
            .cards_in_zone(ZoneType::Hand, target_player)
            .to_vec();
        if let Some(valid_filter) = sa.ir.discard_valid_text.as_deref() {
            let valid_selector = sa.ir.discard_valid_selector.as_ref();
            hand.retain(|&card_id| {
                super::matches_valid_cards_for_sa(
                    ctx.game,
                    sa,
                    ctx.game.card(card_id),
                    valid_selector,
                    valid_filter,
                )
            });
        }

        // AnyNumber$ True implicitly subsumes Optional — picking 0 cards is
        // the "decline" choice. Java's DiscardEffect doesn't fire a separate
        // confirm_action in that case; mirror that to keep callback counts
        // in parity.
        let chooser_style_optional = matches!(
            mode,
            "TgtChoose" | "YouChoose" | "RevealYouChoose" | "RevealTgtChoose"
        );

        // Java DiscardEffect.resolve(): chooser defaults to the discarder, but
        // *YouChoose modes route the pick to the activating player and
        // RevealTgtChoose pins it to the first target. See DiscardEffect.java
        // lines 236-241.
        let chooser = if mode.ends_with("YouChoose") {
            sa.activating_player
        } else if mode.eq_ignore_ascii_case("RevealTgtChoose") {
            first_target.unwrap_or(target_player)
        } else {
            target_player
        };

        // Mode$ Reveal* — broadcast the discarder's hand to every player
        // before the chooser picks, mirroring `game.getAction().reveal(...)`
        // in Java (DiscardEffect.java:244).
        if mode.starts_with("Reveal") && !hand.is_empty() {
            let source_name = sa.source.map(|cid| ctx.game.card(cid).card_name.clone());
            for agent in ctx.agents.iter_mut() {
                agent.reveal_cards(
                    ctx.game,
                    target_player,
                    &hand,
                    ZoneType::Hand,
                    target_player,
                    source_name.as_deref(),
                );
            }
        }

        if sa.ir.optional && !any_number && !chooser_style_optional {
            let source_name = sa.source.map(|cid| ctx.game.card(cid).card_name.as_str());
            let accepted = ctx.agents[target_player.index()].confirm_action(
                target_player,
                None,
                "Do you want to discard?",
                &[],
                source_name,
                Some(crate::ability::api_type::ApiType::Discard),
            );
            if !accepted {
                continue;
            }
        }

        let to_discard = if is_random {
            ctx.agents[target_player.index()].choose_random_discard(target_player, &hand, num)
        } else if any_number {
            ctx.agents[chooser.index()].choose_discard_any_number(
                target_player,
                &hand,
                0,
                hand.len(),
            )
        } else if sa.ir.optional && chooser_style_optional {
            ctx.agents[chooser.index()].choose_discard_any_number(
                target_player,
                &hand,
                0,
                num.min(hand.len()),
            )
        } else if chooser_style_optional {
            ctx.agents[chooser.index()].choose_discard(target_player, &hand, num)
        } else {
            ctx.agents[target_player.index()].choose_discard(target_player, &hand, num)
        };

        // RememberDiscarded$ — source remembers each card actually discarded
        // so downstream SubAbility effects (e.g. Cavalier of Flame's
        // `NumCards$ Y` where Y = Remembered count) can compute amounts.
        let remember_discarded = sa.ir.remember_discarded;

        let to_discard = if to_discard.len() > 1 {
            let reordered = ctx.agents[target_player.index()]
                .choose_reorder_library(target_player, &to_discard);
            if reordered.len() == to_discard.len() {
                reordered
            } else {
                to_discard
            }
        } else {
            to_discard
        };

        for card_id in to_discard {
            if ctx.game.card(card_id).zone == ZoneType::Hand {
                if remember_discarded {
                    if let Some(sid) = sa.source {
                        ctx.game.card_mut(sid).add_remembered_card(card_id);
                    }
                }
                ctx.game.discard_card(
                    card_id,
                    target_player,
                    Some(sa),
                    Some(ctx.agents),
                    ctx.trigger_handler,
                );
            }
        }
    }
}
