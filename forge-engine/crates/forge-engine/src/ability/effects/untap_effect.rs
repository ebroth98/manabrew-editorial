use forge_foundation::ZoneType;

use super::EffectContext;
use crate::event::{RunParams, TriggerType};
use crate::ids::CardId;
use crate::spellability::SpellAbility;

/// Resolve `SP$ Untap` — untap target permanent(s).
///
/// Mirrors Java `UntapEffect.java`.
///
/// # Card script examples
/// ```text
/// A:SP$ Untap | ValidTgts$ Creature | TgtPrompt$ Select target creature to untap
/// A:SP$ Untap | Defined$ Self
/// A:SP$ Untap | Defined$ Self | ETB$ True
/// DB$ Untap | Defined$ ParentTarget
/// ```
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let controller = sa.activating_player;
    let etb = sa.params.has(crate::parsing::keys::ETB);

    let target_card = resolve_untap_target(ctx, sa);

    if let Some(card_id) = target_card {
        if ctx.game.card(card_id).zone == ZoneType::Battlefield {
            untap_card(ctx, card_id, controller, etb);
        }
    }
}

/// Resolve the target card for untap: explicit target, Defined$ Self, or Defined$ ParentTarget.
fn resolve_untap_target(ctx: &EffectContext, sa: &SpellAbility) -> Option<CardId> {
    sa.target_chosen
        .target_card
        .or_else(|| match sa.params.get(crate::parsing::keys::DEFINED) {
            Some("Self") => sa.source,
            Some("ParentTarget") => ctx.parent_target_card,
            _ => None,
        })
}

fn untap_card(
    ctx: &mut EffectContext,
    card_id: CardId,
    controller: crate::ids::PlayerId,
    etb: bool,
) {
    if etb {
        // ETB: directly set untapped without firing trigger
        ctx.game.card_mut(card_id).set_tapped(false);
    } else {
        let untapped = ctx.game.untap(card_id);
        if untapped {
            ctx.trigger_handler.run_trigger(
                TriggerType::Untaps,
                RunParams {
                    card: Some(card_id),
                    player: Some(controller),
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
    fn untap_effect_untaps_target() {
        let mut game = GameState::new(&["Alice", "Bob"], 20);
        let p0 = PlayerId(0);
        let c1 = make_creature(&mut game, p0);
        game.move_card(c1, ZoneType::Battlefield, p0);
        game.tap(c1);
        assert!(game.card(c1).tapped);

        let mut sa = SpellAbility::new_simple(None, p0, "SP$ Untap | ValidTgts$ Creature");
        sa.target_chosen.target_card = Some(c1);

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

        assert!(!ctx.game.card(c1).tapped);
    }
}
