use forge_foundation::ZoneType;

use super::{matches_valid_cards, EffectContext};
use crate::ids::CardId;
use crate::spellability::SpellAbility;

/// `SP$ UntapAll` — untap all matching permanents.
///
/// Mirrors Java's `UntapAllEffect.java`.
///
/// # Card script examples
/// ```text
/// A:SP$ UntapAll | ValidCards$ Land.YouCtrl
/// A:SP$ UntapAll | ValidCards$ Creature.YouCtrl
/// ```
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let valid_cards_filter = sa
        .params
        .get("ValidCards")
        .map(|s| s.to_string())
        .unwrap_or_else(|| "Permanent".to_string());
    let activating_player = sa.activating_player;

    let player_ids = ctx.game.player_order.clone();
    let mut to_untap: Vec<CardId> = Vec::new();
    for &pid in &player_ids {
        let zone_cards = ctx.game.cards_in_zone(ZoneType::Battlefield, pid).to_vec();
        for cid in zone_cards {
            if matches_valid_cards(ctx.game.card(cid), &valid_cards_filter, activating_player) {
                to_untap.push(cid);
            }
        }
    }

    for card_id in to_untap {
        if ctx.game.card(card_id).zone == ZoneType::Battlefield {
            ctx.game.untap(card_id);
            // Fire Untaps trigger per card
            ctx.trigger_handler.run_trigger(
                crate::event::TriggerType::Untaps,
                crate::event::RunParams {
                    card: Some(card_id),
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
    use std::collections::HashMap;

    use crate::ability::effects::EffectContext;
    use crate::agent::PassAgent;
    use crate::card::Card;
    use crate::game::GameState;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::ManaPool;
    use crate::spellability::SpellAbility;
    use crate::trigger::handler::TriggerHandler;

    fn make_land(game: &mut GameState, owner: PlayerId) -> CardId {
        let c = Card::new(
            CardId(0),
            "Forest".into(),
            owner,
            CardTypeLine::parse("Basic Land Forest"),
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
    fn untap_all_untaps_matching_permanents() {
        let mut game = GameState::new(&["Alice", "Bob"], 20);
        let p0 = PlayerId(0);
        let l1 = make_land(&mut game, p0);
        let l2 = make_land(&mut game, p0);
        game.move_card(l1, ZoneType::Battlefield, p0);
        game.move_card(l2, ZoneType::Battlefield, p0);
        game.tap(l1);
        game.tap(l2);
        assert!(game.card(l1).tapped);
        assert!(game.card(l2).tapped);

        let sa = SpellAbility::new_simple(None, p0, "A:SP$ UntapAll | ValidCards$ Land.YouCtrl");
        let mut th = TriggerHandler::new();
        let mut agents: Vec<Box<dyn crate::agent::PlayerAgent>> =
            vec![Box::new(PassAgent), Box::new(PassAgent)];
        let mut mp = vec![ManaPool::default(), ManaPool::default()];
        let templates = HashMap::new();
        let mut rng_adapter = crate::game_rng::ThreadRngAdapter;
        let mut ctx = EffectContext {
            game: &mut game,
            agents: &mut agents,
            trigger_handler: &mut th,
            token_templates: &templates,
            mana_pools: &mut mp,
            parent_target_card: None,
            rng: &mut rng_adapter,
        };
        super::resolve(&mut ctx, &sa);

        assert!(!ctx.game.card(l1).tapped);
        assert!(!ctx.game.card(l2).tapped);
    }
}
