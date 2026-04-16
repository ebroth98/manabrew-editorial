use super::EffectContext;
use crate::spellability::SpellAbility;

/// Resolve `SP$ GameDraw` — the game ends in a draw.
///
/// Mirrors Java `GameDrawEffect.java`.
///
/// # Card script examples
/// ```text
/// A:SP$ GameDraw
/// ```
pub fn resolve(ctx: &mut EffectContext, _sa: &SpellAbility) {
    // All players lose — no winner
    let all_players: Vec<_> = ctx.game.player_order.clone();
    for &pid in &all_players {
        ctx.game
            .player_mark_lost(pid, crate::replacement::GameLossReason::IntentionalDraw);
    }

    ctx.game.game_over = true;
    ctx.game.winner = None;
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
    fn game_draw_ends_game_with_no_winner() {
        let mut game = GameState::new(&["Alice", "Bob"], 20);
        let p0 = PlayerId(0);

        let sa = SpellAbility::new_simple(None, p0, "SP$ GameDraw");

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

        assert!(ctx.game.game_over);
        assert_eq!(ctx.game.winner, None);
        assert!(ctx.game.player(p0).has_lost);
        assert!(ctx.game.player(PlayerId(1)).has_lost);
    }
}
