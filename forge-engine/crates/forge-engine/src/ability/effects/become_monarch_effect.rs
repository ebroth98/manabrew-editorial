use super::{resolve_defined_player, EffectContext};
use crate::event::{RunParams, TriggerType};
use crate::spellability::SpellAbility;

/// Resolve `SP$ BecomeMonarch` — make a player the monarch.
///
/// Mirrors Java `BecomeMonarchEffect.java`.
/// Sets `game.monarch` to the target player and fires a BecomeMonarch trigger.
///
/// # Card script examples
/// ```text
/// A:SP$ BecomeMonarch | Defined$ You
/// A:SP$ BecomeMonarch | Defined$ Opponent
/// ```
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let controller = sa.activating_player;

    let defined = sa
        .params
        .get("Defined")
        .map(|s| s.as_str())
        .unwrap_or("You");

    let target = resolve_defined_player(defined, controller, ctx.game).unwrap_or(controller);

    if !ctx.game.player(target).is_alive() {
        return;
    }

    ctx.game.monarch = Some(target);

    ctx.trigger_handler.run_trigger(
        TriggerType::BecomeMonarch,
        RunParams {
            player: Some(target),
            ..Default::default()
        },
        false,
    );
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
    fn become_monarch_sets_state() {
        let mut game = GameState::new(&["Alice", "Bob"], 20);
        let p0 = PlayerId(0);
        assert!(game.monarch.is_none());

        let sa = SpellAbility::new_simple(None, p0, "SP$ BecomeMonarch | Defined$ You");

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

        assert_eq!(ctx.game.monarch, Some(p0));
    }

    #[test]
    fn become_monarch_transfers() {
        let mut game = GameState::new(&["Alice", "Bob"], 20);
        let p0 = PlayerId(0);
        let p1 = PlayerId(1);
        game.monarch = Some(p0);

        let sa = SpellAbility::new_simple(None, p1, "SP$ BecomeMonarch | Defined$ You");

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

        assert_eq!(ctx.game.monarch, Some(p1));
    }
}
