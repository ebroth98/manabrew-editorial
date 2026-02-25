use super::EffectContext;
use crate::spellability::SpellAbility;

/// Resolve `SP$ EndTurn` — end the current turn.
///
/// Mirrors Java `EndTurnEffect.java`.
/// Sets the `end_turn_requested` flag on `GameState`. The game loop checks
/// this flag in the turn state machine to skip remaining phases and jump
/// directly to the Cleanup step.
///
/// # Card script examples
/// ```text
/// A:SP$ EndTurn
/// ```
pub fn resolve(ctx: &mut EffectContext, _sa: &SpellAbility) {
    // Clear the stack (exile all spells/abilities)
    while ctx.game.stack.pop().is_some() {}
    // Signal the game loop to skip to cleanup
    ctx.game.end_turn_requested = true;
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
    fn end_turn_sets_flag() {
        let mut game = GameState::new(&["Alice", "Bob"], 20);
        let p0 = PlayerId(0);

        let sa = SpellAbility::new_simple(None, p0, "SP$ EndTurn");

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

        assert!(ctx.game.end_turn_requested);
        // Stack should be empty after EndTurn
        assert!(ctx.game.stack.is_empty());
    }
}
