use super::{parse_param, resolve_defined_players, EffectContext};
use crate::event::{RunParams, TriggerType};
use crate::spellability::SpellAbility;

/// Resolve `SP$ LifeSet` — set a player's life total to a specific value.
///
/// Mirrors Java `LifeSetEffect.java`.
/// Supports multi-player targeting via `Defined$ Each`.
///
/// # Card script examples
/// ```text
/// A:SP$ LifeSet | Defined$ You | LifeAmount$ 10
/// A:SP$ LifeSet | Defined$ Opponent | LifeAmount$ 1
/// A:SP$ LifeSet | Defined$ Each | LifeAmount$ 20
/// ```
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let controller = sa.activating_player;

    let amount = parse_param(&sa.ability_text, "LifeAmount$ ").unwrap_or(0);

    let defined = sa
        .params
        .get("Defined")
        .map(|s| s.as_str())
        .unwrap_or("You");

    let targets = resolve_defined_players(defined, controller, ctx.game);

    for target in targets {
        if !ctx.game.player(target).is_alive() {
            continue;
        }

        let diff = ctx.game.player_mut(target).set_life(amount);

        // Fire the appropriate life trigger based on the difference
        if diff > 0 {
            ctx.trigger_handler.run_trigger(
                TriggerType::LifeGained,
                RunParams {
                    player: Some(target),
                    life_amount: Some(diff),
                    ..Default::default()
                },
                false,
            );
        } else if diff < 0 {
            ctx.trigger_handler.run_trigger(
                TriggerType::LifeLost,
                RunParams {
                    player: Some(target),
                    life_amount: Some(diff.abs()),
                    ..Default::default()
                },
                false,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::ability::effects::EffectContext;
    use crate::agent::PassAgent;
    use crate::game::GameState;
    use crate::ids::PlayerId;
    use crate::mana::ManaPool;
    use crate::spellability::SpellAbility;
    use crate::trigger::handler::TriggerHandler;

    #[test]
    fn life_set_reduces_life() {
        let mut game = GameState::new(&["Alice", "Bob"], 20);
        let p0 = PlayerId(0);

        let sa = SpellAbility::new_simple(None, p0, "SP$ LifeSet | Defined$ You | LifeAmount$ 10");

        let mut th = TriggerHandler::new();
        let mut agents: Vec<Box<dyn crate::agent::PlayerAgent>> =
            vec![Box::new(PassAgent), Box::new(PassAgent)];
        let mut mp = vec![ManaPool::default(), ManaPool::default()];
        let templates = HashMap::new();
        let mut ctx = EffectContext {
            game: &mut game,
            agents: &mut agents,
            trigger_handler: &mut th,
            token_templates: &templates,
            mana_pools: &mut mp,
            parent_target_card: None,
        };
        super::resolve(&mut ctx, &sa);

        assert_eq!(ctx.game.player(p0).life, 10);
        assert_eq!(ctx.game.player(p0).life_lost_this_turn, 10);
    }

    #[test]
    fn life_set_increases_life() {
        let mut game = GameState::new(&["Alice", "Bob"], 5);
        let p0 = PlayerId(0);

        let sa = SpellAbility::new_simple(None, p0, "SP$ LifeSet | Defined$ You | LifeAmount$ 20");

        let mut th = TriggerHandler::new();
        let mut agents: Vec<Box<dyn crate::agent::PlayerAgent>> =
            vec![Box::new(PassAgent), Box::new(PassAgent)];
        let mut mp = vec![ManaPool::default(), ManaPool::default()];
        let templates = HashMap::new();
        let mut ctx = EffectContext {
            game: &mut game,
            agents: &mut agents,
            trigger_handler: &mut th,
            token_templates: &templates,
            mana_pools: &mut mp,
            parent_target_card: None,
        };
        super::resolve(&mut ctx, &sa);

        assert_eq!(ctx.game.player(p0).life, 20);
        assert_eq!(ctx.game.player(p0).life_gained_this_turn, 15);
    }
}
