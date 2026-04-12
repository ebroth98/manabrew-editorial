use forge_foundation::ZoneType;

use super::EffectContext;
use crate::event::{RunParams, TriggerType};
use crate::spellability::SpellAbility;

/// Resolve `SP$ Phases` — phase permanents in or out.
///
/// Mirrors Java `PhasesEffect.java`.
/// Toggles or sets `card.phased_out` on target cards. Phased-out permanents
/// are treated as not on the battlefield for game purposes.
///
/// # Card script examples
/// ```text
/// A:SP$ Phases | ValidTgts$ Creature | TgtPrompt$ Select target creature
/// A:SP$ Phases | Defined$ Self | PhaseInOrOut$ Out
/// ```
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let phase_mode = sa.params.get("PhaseInOrOut").unwrap_or("Out");

    // Targeted: use the chosen target card.
    if let Some(target_card) = sa.target_chosen.target_card {
        if ctx.game.card(target_card).zone == ZoneType::Battlefield {
            apply_phase(ctx, target_card, phase_mode);
        }
        return;
    }

    // Defined$ Self — phase the source card.
    if let Some(source) = sa.source {
        if ctx.game.card(source).zone == ZoneType::Battlefield {
            apply_phase(ctx, source, phase_mode);
        }
    }
}

fn apply_phase(ctx: &mut EffectContext, card_id: crate::ids::CardId, mode: &str) {
    match mode {
        "In" => {
            if ctx.game.card(card_id).phased_out {
                ctx.game.card_mut(card_id).set_phased_out(false);
                ctx.trigger_handler.run_trigger(
                    TriggerType::PhasedIn,
                    RunParams {
                        card: Some(card_id),
                        ..Default::default()
                    },
                    false,
                );
            }
        }
        "Out" | _ => {
            if !ctx.game.card(card_id).phased_out {
                ctx.game.card_mut(card_id).set_phased_out(true);
                ctx.trigger_handler.run_trigger(
                    TriggerType::PhasedOut,
                    RunParams {
                        card: Some(card_id),
                        ..Default::default()
                    },
                    false,
                );
            }
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
    fn phase_out_target() {
        let mut game = GameState::new(&["Alice", "Bob"], 20);
        let p0 = PlayerId(0);
        let c1 = make_creature(&mut game, p0);
        game.move_card(c1, ZoneType::Battlefield, p0);
        assert!(!game.card(c1).phased_out);

        let mut sa = SpellAbility::new_simple(None, p0, "SP$ Phases | PhaseInOrOut$ Out");
        sa.target_chosen.target_card = Some(c1);

        let mut th = TriggerHandler::new();
        let mut agents: Vec<Box<dyn crate::agent::PlayerAgent>> =
            vec![Box::new(PassAgent), Box::new(PassAgent)];
        let mut mp = vec![ManaPool::default(), ManaPool::default()];
        let templates = HashMap::new();
        let templates_variants = HashMap::new();
        let token_fallback = HashMap::new();
        let mut rng_adapter = crate::game_rng::ThreadRngAdapter;
        let mut ctx = EffectContext {
            game: &mut game,
            combat: None,
            agents: &mut agents,
            trigger_handler: &mut th,
            token_templates: &templates,
            token_art_variants: &templates_variants,
            token_fallback: &token_fallback,
            mana_pools: &mut mp,
            parent_target_card: None,
            rng: &mut rng_adapter,
        };
        super::resolve(&mut ctx, &sa);

        assert!(ctx.game.card(c1).phased_out);
    }

    #[test]
    fn phase_in_target() {
        let mut game = GameState::new(&["Alice", "Bob"], 20);
        let p0 = PlayerId(0);
        let c1 = make_creature(&mut game, p0);
        game.move_card(c1, ZoneType::Battlefield, p0);
        game.card_mut(c1).set_phased_out(true);

        let mut sa = SpellAbility::new_simple(None, p0, "SP$ Phases | PhaseInOrOut$ In");
        sa.target_chosen.target_card = Some(c1);

        let mut th = TriggerHandler::new();
        let mut agents: Vec<Box<dyn crate::agent::PlayerAgent>> =
            vec![Box::new(PassAgent), Box::new(PassAgent)];
        let mut mp = vec![ManaPool::default(), ManaPool::default()];
        let templates = HashMap::new();
        let templates_variants = HashMap::new();
        let token_fallback = HashMap::new();
        let mut rng_adapter = crate::game_rng::ThreadRngAdapter;
        let mut ctx = EffectContext {
            game: &mut game,
            combat: None,
            agents: &mut agents,
            trigger_handler: &mut th,
            token_templates: &templates,
            token_art_variants: &templates_variants,
            token_fallback: &token_fallback,
            mana_pools: &mut mp,
            parent_target_card: None,
            rng: &mut rng_adapter,
        };
        super::resolve(&mut ctx, &sa);

        assert!(!ctx.game.card(c1).phased_out);
    }
}
