use forge_foundation::ZoneType;

use super::{matches_valid_cards, EffectContext};
use crate::ids::CardId;
use crate::spellability::SpellAbility;

/// `SP$ TapAll` — tap all matching permanents.
///
/// Mirrors Java's `TapAllEffect.java`.
/// Uses the standard two-pass collect → act pattern to avoid borrow issues.
///
/// # Card script examples
/// ```text
/// A:SP$ TapAll | ValidCards$ Creature.Blue
/// A:SP$ TapAll | ValidCards$ Creature.OppCtrl
/// ```
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let valid_cards_filter = sa
        .params
        .get("ValidCards")
        .map(|s| s.to_string())
        .unwrap_or_else(|| "Creature".to_string());
    let activating_player = sa.activating_player;

    let player_ids = ctx.game.player_order.clone();
    let mut to_tap: Vec<CardId> = Vec::new();
    for &pid in &player_ids {
        let zone_cards = ctx.game.cards_in_zone(ZoneType::Battlefield, pid).to_vec();
        for cid in zone_cards {
            if matches_valid_cards(ctx.game.card(cid), &valid_cards_filter, activating_player) {
                to_tap.push(cid);
            }
        }
    }

    for card_id in to_tap {
        if ctx.game.card(card_id).zone == ZoneType::Battlefield {
            ctx.game.tap(card_id);
            // Fire Taps trigger per card
            ctx.trigger_handler.run_trigger(
                crate::event::TriggerType::Taps,
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

    fn make_creature(game: &mut GameState, owner: PlayerId) -> CardId {
        let c = Card::new(
            CardId(0),
            "Bear".into(),
            owner,
            CardTypeLine::parse("Creature - Bear"),
            ManaCost::parse("1 G"),
            ColorSet::GREEN,
            Some(2),
            Some(2),
            vec![],
            vec![],
        );
        game.create_card(c)
    }

    #[test]
    fn tap_all_taps_matching_creatures() {
        let mut game = GameState::new(&["Alice", "Bob"], 20);
        let p0 = PlayerId(0);
        let p1 = PlayerId(1);
        let c1 = make_creature(&mut game, p0);
        let c2 = make_creature(&mut game, p1);
        game.move_card(c1, ZoneType::Battlefield, p0);
        game.move_card(c2, ZoneType::Battlefield, p1);
        assert!(!game.card(c1).tapped);
        assert!(!game.card(c2).tapped);

        let sa = SpellAbility::new_simple(None, p0, "A:SP$ TapAll | ValidCards$ Creature");
        let mut th = TriggerHandler::new();
        let mut agents: Vec<Box<dyn crate::agent::PlayerAgent>> =
            vec![Box::new(PassAgent), Box::new(PassAgent)];
        let mut mp = vec![ManaPool::default(), ManaPool::default()];
        let templates = HashMap::new();
        let templates_variants = HashMap::new();
        let token_fallback = HashMap::new();
        let edition_dates: HashMap<String, String> = HashMap::new();
        let mut rng_adapter = crate::game_rng::ThreadRngAdapter;
        let mut ctx = EffectContext {
            game: &mut game,
            combat: None,
            agents: &mut agents,
            trigger_handler: &mut th,
            token_templates: &templates,
            token_art_variants: &templates_variants,
            token_fallback: &token_fallback,
            edition_dates: &edition_dates,
            mana_pools: &mut mp,
            parent_target_card: None,
            rng: &mut rng_adapter,
        };
        super::resolve(&mut ctx, &sa);

        assert!(ctx.game.card(c1).tapped);
        assert!(ctx.game.card(c2).tapped);
    }
}
