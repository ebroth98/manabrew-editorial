use super::{resolve_defined_player, EffectContext};
use crate::replacement::handler::{apply_replacements, ReplacementEvent};
use crate::replacement::ReplacementResult;
use crate::spellability::SpellAbility;

/// Resolve `SP$ GameWin` — a player wins the game.
///
/// Mirrors Java `GameWinEffect.java`.
///
/// # Card script examples
/// ```text
/// A:SP$ GameWin | Defined$ You
/// ```
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let controller = sa.activating_player;

    let defined = sa
        .params
        .get("Defined")
        .map(|s| s.as_str())
        .unwrap_or("You");

    let winner = resolve_defined_player(defined, controller, ctx.game).unwrap_or(controller);

    if !ctx.game.player(winner).is_alive() {
        return;
    }

    // Run GameWin replacement effects.
    let mut event = ReplacementEvent::GameWin { player: winner };
    let result = apply_replacements(ctx.game, &mut event);
    if result == ReplacementResult::Replaced {
        return;
    }

    // Set the winner
    ctx.game.player_mut(winner).has_won = true;

    // All other players lose
    let all_players: Vec<_> = ctx.game.player_order.clone();
    for &pid in &all_players {
        if pid != winner {
            ctx.game.player_mut(pid).has_lost = true;
        }
    }

    ctx.game.game_over = true;
    ctx.game.winner = Some(winner);
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
    fn game_win_sets_winner() {
        let mut game = GameState::new(&["Alice", "Bob"], 20);
        let p0 = PlayerId(0);

        let sa = SpellAbility::new_simple(None, p0, "SP$ GameWin | Defined$ You");

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

        assert!(ctx.game.game_over);
        assert_eq!(ctx.game.winner, Some(p0));
        assert!(ctx.game.player(p0).has_won);
        assert!(ctx.game.player(PlayerId(1)).has_lost);
    }
}
