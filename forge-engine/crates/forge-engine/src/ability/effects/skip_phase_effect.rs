use super::{resolve_defined_players, EffectContext};
use crate::spellability::SpellAbility;

/// Resolve `SP$ SkipPhase` — make a player skip their next occurrence of a phase.
///
/// Mirrors Java `SkipPhaseEffect.java`.
/// Sets per-player phase skip flags. The game loop checks these before
/// entering each phase and skips accordingly.
///
/// # Card script examples
/// ```text
/// A:SP$ SkipPhase | Defined$ Opponent | Phase$ Draw
/// A:SP$ SkipPhase | Defined$ You | Phase$ Combat
/// A:SP$ SkipPhase | Defined$ Opponent | Phase$ Untap
/// ```
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let controller = sa.activating_player;

    let phase = sa
        .params
        .get("Phase")
        .or_else(|| sa.params.get("Step"))
        .map(|s| s.as_str())
        .unwrap_or("");

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
        match phase {
            "Draw" => ctx.game.player_mut(target).skip_next_draw = true,
            "Combat" | "BeginCombat" => ctx.game.player_mut(target).skip_next_combat = true,
            "Untap" => ctx.game.player_mut(target).skip_next_untap = true,
            _ => {}
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
    fn skip_draw_phase() {
        let mut game = GameState::new(&["Alice", "Bob"], 20);
        let p0 = PlayerId(0);
        let p1 = PlayerId(1);

        let sa =
            SpellAbility::new_simple(None, p0, "SP$ SkipPhase | Defined$ Opponent | Phase$ Draw");

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

        assert!(ctx.game.player(p1).skip_next_draw);
        assert!(!ctx.game.player(p1).skip_next_combat);
    }

    #[test]
    fn skip_combat_phase() {
        let mut game = GameState::new(&["Alice", "Bob"], 20);
        let p0 = PlayerId(0);

        let sa = SpellAbility::new_simple(None, p0, "SP$ SkipPhase | Defined$ You | Phase$ Combat");

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

        assert!(ctx.game.player(p0).skip_next_combat);
    }
}
