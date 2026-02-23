use forge_foundation::ZoneType;

use super::{emit_zone_trigger, parse_param, resolve_defined_player, EffectContext};
use crate::event::{RunParams, TriggerType};
use crate::spellability::SpellAbility;

/// Mirrors Java's `MillEffect.java`.
///
/// `SP$ Mill | NumCards$ N | Defined$ You`
/// Moves the top N cards of the target player's library to their graveyard.
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let num = parse_param(&sa.ability_text, "NumCards$ ").unwrap_or(1) as usize;

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

    let lib_len = ctx.game.cards_in_zone(ZoneType::Library, target).len();
    let count = num.min(lib_len);

    for _ in 0..count {
        let top = ctx.game.zone_mut(ZoneType::Library, target).take_top();
        if let Some(card_id) = top {
            ctx.game.move_card(card_id, ZoneType::Graveyard, target);
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
    }
}

#[cfg(test)]
mod tests {
    use forge_foundation::{CardTypeLine, ColorSet, ManaCost, ZoneType};

    use crate::ability::effects::EffectContext;
    use crate::agent::PassAgent;
    use crate::card::CardInstance;
    use crate::game::GameState;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::ManaPool;
    use crate::spellability::SpellAbility;
    use crate::trigger::handler::TriggerHandler;
    use std::collections::HashMap;

    fn make_land(game: &mut GameState, owner: PlayerId) -> CardId {
        let c = CardInstance::new(
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
        let mut ctx = EffectContext {
            game: &mut game,
            agents: &mut agents,
            trigger_handler: &mut trigger_handler,
            token_templates: &token_templates,
            mana_pools: &mut mana_pools,
            parent_target_card: None,
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
        let mut ctx = EffectContext {
            game: &mut game,
            agents: &mut agents,
            trigger_handler: &mut trigger_handler,
            token_templates: &token_templates,
            mana_pools: &mut mana_pools,
            parent_target_card: None,
        };

        super::resolve(&mut ctx, &sa);

        assert_eq!(ctx.game.cards_in_zone(ZoneType::Library, p0).len(), 0);
        assert_eq!(ctx.game.cards_in_zone(ZoneType::Graveyard, p0).len(), 1);
    }
}
