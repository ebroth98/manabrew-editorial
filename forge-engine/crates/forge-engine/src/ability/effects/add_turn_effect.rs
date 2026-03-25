use super::{parse_param, resolve_defined_player, EffectContext};
use crate::parsing::keys;
use crate::phase::ExtraTurn;
use crate::spellability::SpellAbility;

/// Resolve `SP$ AddTurn` — give a player extra turns.
///
/// Mirrors Java `AddTurnEffect.java`.
/// Pushes the player onto the `extra_turns` queue in `GameState`.
/// The game loop's `AdvanceTurn` pops from this queue instead of
/// advancing to the next player in turn order.
///
/// # Card script examples
/// ```text
/// A:SP$ AddTurn | Defined$ You | NumTurns$ 1
/// A:SP$ AddTurn | Defined$ You | NumTurns$ 2
/// A:SP$ AddTurn | Defined$ You | NumTurns$ 1 | SkipUntap$ True
/// ```
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let controller = sa.activating_player;

    let num_turns = parse_param(&sa.ability_text, "NumTurns$ ").unwrap_or(1);
    let skip_untap = sa.params.has(keys::SKIP_UNTAP);

    let defined = sa.params.get(keys::DEFINED).unwrap_or("You");

    let target = resolve_defined_player(defined, controller, ctx.game).unwrap_or(controller);

    if !ctx.game.player(target).is_alive() {
        return;
    }

    for _ in 0..num_turns {
        let mut et = ExtraTurn::new(target);
        et.set_skip_untap(skip_untap);
        ctx.game.extra_turns.push_back(et);
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
    fn add_turn_queues_extra_turns() {
        let mut game = GameState::new(&["Alice", "Bob"], 20);
        let p0 = PlayerId(0);

        let sa = SpellAbility::new_simple(None, p0, "SP$ AddTurn | Defined$ You | NumTurns$ 2");

        let mut th = TriggerHandler::new();
        let mut agents: Vec<Box<dyn crate::agent::PlayerAgent>> =
            vec![Box::new(PassAgent), Box::new(PassAgent)];
        let mut mp = vec![ManaPool::default(), ManaPool::default()];
        let templates = HashMap::new();
        let mut rng_adapter = crate::game_rng::ThreadRngAdapter;
        let mut ctx = EffectContext {
            game: &mut game,
            combat: None,
            agents: &mut agents,
            trigger_handler: &mut th,
            token_templates: &templates,
            mana_pools: &mut mp,
            parent_target_card: None,
            rng: &mut rng_adapter,
        };
        super::resolve(&mut ctx, &sa);

        assert_eq!(ctx.game.extra_turns.len(), 2);
        assert_eq!(ctx.game.extra_turns[0].player, p0);
        assert_eq!(ctx.game.extra_turns[1].player, p0);
    }

    #[test]
    fn add_turn_default_one() {
        let mut game = GameState::new(&["Alice", "Bob"], 20);
        let p0 = PlayerId(0);

        let sa = SpellAbility::new_simple(None, p0, "SP$ AddTurn | Defined$ You");

        let mut th = TriggerHandler::new();
        let mut agents: Vec<Box<dyn crate::agent::PlayerAgent>> =
            vec![Box::new(PassAgent), Box::new(PassAgent)];
        let mut mp = vec![ManaPool::default(), ManaPool::default()];
        let templates = HashMap::new();
        let mut rng_adapter = crate::game_rng::ThreadRngAdapter;
        let mut ctx = EffectContext {
            game: &mut game,
            combat: None,
            agents: &mut agents,
            trigger_handler: &mut th,
            token_templates: &templates,
            mana_pools: &mut mp,
            parent_target_card: None,
            rng: &mut rng_adapter,
        };
        super::resolve(&mut ctx, &sa);

        assert_eq!(ctx.game.extra_turns.len(), 1);
        assert_eq!(ctx.game.extra_turns[0].player, p0);
        assert!(!ctx.game.extra_turns[0].skip_untap);
    }
}
