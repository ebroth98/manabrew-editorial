use forge_foundation::ZoneType;

use super::{emit_zone_trigger, resolve_defined_player, resolve_numeric_svar, EffectContext};
use crate::event::{RunParams};
use crate::trigger::TriggerType;
use crate::replacement::replacement_handler::{apply_replacements, ReplacementEvent};
use crate::replacement::ReplacementResult;
use crate::spellability::SpellAbility;

/// Mirrors Java's `MillEffect.java`.
///
/// `SP$ Mill | NumCards$ N | Defined$ You`
/// Moves the top N cards of the target player's library to their graveyard.
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let num = resolve_numeric_svar(ctx.game, sa, "NumCards", 1).max(0) as usize;

    // Determine target player: targeted (ValidTgts$) takes priority, then Defined$.
    let target = sa
        .target_chosen
        .target_player
        .or_else(|| {
            sa.params
                .get("Defined")
                .and_then(|d| resolve_defined_player(d, sa.activating_player, ctx.game))
        })
        .unwrap_or(sa.activating_player);

    // Run Mill replacement effects before milling.
    let mut event = ReplacementEvent::Mill {
        player: target,
        count: num as i32,
    };
    let result = apply_replacements(ctx.game, &mut event);
    if result == ReplacementResult::Skipped || result == ReplacementResult::Replaced {
        return;
    }
    let num = if let ReplacementEvent::Mill { count, .. } = event {
        count.max(0) as usize
    } else {
        num
    };

    if num == 0 {
        return;
    }

    let lib = ctx.game.cards_in_zone(ZoneType::Library, target);
    let mut milled_cards: Vec<crate::ids::CardId> = lib.iter().rev().take(num).copied().collect();
    if milled_cards.len() > 1 {
        ctx.agents[target.index()].snapshot_state(ctx.game, ctx.mana_pools);
        ctx.agents[target.index()].on_library_peek(ctx.game, &milled_cards);
        let reordered = ctx.agents[target.index()].choose_reorder_library(target, &milled_cards);
        if reordered.len() == milled_cards.len()
            && milled_cards.iter().all(|id| reordered.contains(id))
        {
            milled_cards = reordered;
        }
    }

    for &card_id in &milled_cards {
        ctx.move_card(card_id, ZoneType::Graveyard, target);
        emit_zone_trigger(
            ctx.trigger_handler,
            card_id,
            ZoneType::Library,
            ZoneType::Graveyard,
        );
        // Fire Milled trigger per card
        ctx.trigger_handler.run_trigger(
            TriggerType::Milled,
            RunParams {
                card: Some(card_id),
                player: Some(target),
                ..Default::default()
            },
            false,
        );
    }

    if sa.params.has("RememberMilled") {
        if let Some(source_id) = sa.source {
            ctx.game
                .card_mut(source_id)
                .add_remembered_cards(milled_cards.iter().copied());
        }
    }
    if sa.params.has("Imprint") {
        if let Some(source_id) = sa.source {
            ctx.game
                .card_mut(source_id)
                .add_imprinted_cards(milled_cards.iter().copied());
        }
    }

    if !milled_cards.is_empty() {
        ctx.trigger_handler.run_trigger(
            TriggerType::MilledOnce,
            RunParams {
                player: Some(target),
                cards: Some(milled_cards.clone()),
                ..Default::default()
            },
            false,
        );
        ctx.trigger_handler.run_trigger(
            TriggerType::MilledAll,
            RunParams {
                cards: Some(milled_cards),
                ..Default::default()
            },
            false,
        );
    }
}

#[cfg(test)]
mod tests {
    use forge_foundation::{CardTypeLine, ColorSet, ManaCost, ZoneType};

    use crate::ability::effects::EffectContext;
    use crate::agent::PassAgent;
    use crate::card::Card;
    use crate::game::GameState;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::ManaPool;
    use crate::spellability::SpellAbility;
    use crate::trigger::handler::TriggerHandler;
    use std::collections::HashMap;

    fn make_land(game: &mut GameState, owner: PlayerId) -> CardId {
        let c = Card::new(
            CardId(0),
            "Island".into(),
            owner,
            CardTypeLine::parse("Basic Land Island"),
            ManaCost::parse(""),
            ColorSet::COLORLESS,
            None,
            None,
            vec![],
            vec![],
        );
        game.create_card(c)
    }

    #[test]
    fn mill_moves_cards_to_graveyard() {
        let mut game = GameState::new(&["Alice", "Bob"], 20);
        let p0 = PlayerId(0);
        for _ in 0..3 {
            let id = make_land(&mut game, p0);
            game.move_card(id, ZoneType::Library, p0);
        }
        assert_eq!(game.cards_in_zone(ZoneType::Library, p0).len(), 3);

        let sa = SpellAbility::new_simple(None, p0, "SP$ Mill | NumCards$ 2 | Defined$ You");
        let mut trigger_handler = TriggerHandler::new();
        let mut agents: Vec<Box<dyn crate::agent::PlayerAgent>> =
            vec![Box::new(PassAgent), Box::new(PassAgent)];
        let mut mana_pools = vec![ManaPool::default(), ManaPool::default()];
        let token_templates = HashMap::new();
        let templates_variants: HashMap<(String, String), usize> = HashMap::new();
        let token_fallback: HashMap<String, String> = HashMap::new();
        let edition_dates: HashMap<String, String> = HashMap::new();
        let mut rng_adapter = crate::game_rng::ThreadRngAdapter;
        let mut ctx = EffectContext {
            game: &mut game,
            combat: None,
            agents: &mut agents,
            trigger_handler: &mut trigger_handler,
            token_templates: &token_templates,
            token_art_variants: &templates_variants,
            token_fallback: &token_fallback,
            edition_dates: &edition_dates,
            mana_pools: &mut mana_pools,
            parent_target_card: None,
            rng: &mut rng_adapter,
        };

        super::resolve(&mut ctx, &sa);

        assert_eq!(ctx.game.cards_in_zone(ZoneType::Library, p0).len(), 1);
        assert_eq!(ctx.game.cards_in_zone(ZoneType::Graveyard, p0).len(), 2);
    }

    #[test]
    fn mill_does_not_exceed_library_size() {
        let mut game = GameState::new(&["Alice", "Bob"], 20);
        let p0 = PlayerId(0);
        let id = make_land(&mut game, p0);
        game.move_card(id, ZoneType::Library, p0);

        let sa = SpellAbility::new_simple(None, p0, "SP$ Mill | NumCards$ 5 | Defined$ You");
        let mut trigger_handler = TriggerHandler::new();
        let mut agents: Vec<Box<dyn crate::agent::PlayerAgent>> =
            vec![Box::new(PassAgent), Box::new(PassAgent)];
        let mut mana_pools = vec![ManaPool::default(), ManaPool::default()];
        let token_templates = HashMap::new();
        let templates_variants: HashMap<(String, String), usize> = HashMap::new();
        let token_fallback: HashMap<String, String> = HashMap::new();
        let edition_dates: HashMap<String, String> = HashMap::new();
        let mut rng_adapter = crate::game_rng::ThreadRngAdapter;
        let mut ctx = EffectContext {
            game: &mut game,
            combat: None,
            agents: &mut agents,
            trigger_handler: &mut trigger_handler,
            token_templates: &token_templates,
            token_art_variants: &templates_variants,
            token_fallback: &token_fallback,
            edition_dates: &edition_dates,
            mana_pools: &mut mana_pools,
            parent_target_card: None,
            rng: &mut rng_adapter,
        };

        super::resolve(&mut ctx, &sa);

        assert_eq!(ctx.game.cards_in_zone(ZoneType::Library, p0).len(), 0);
        assert_eq!(ctx.game.cards_in_zone(ZoneType::Graveyard, p0).len(), 1);
    }
}
