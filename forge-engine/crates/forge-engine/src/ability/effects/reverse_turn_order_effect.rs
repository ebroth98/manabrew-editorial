use super::EffectContext;
use crate::spellability::SpellAbility;

/// Resolve `SP$ ReverseTurnOrder` — reverse the player turn order.
///
/// Mirrors Java `ReverseTurnOrderEffect.java`.
///
/// # Card script examples
/// ```text
/// A:SP$ ReverseTurnOrder
/// ```
pub fn resolve(ctx: &mut EffectContext, _sa: &SpellAbility) {
    ctx.game.player_order.reverse();
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
    fn reverse_turn_order() {
        let mut game = GameState::new(&["Alice", "Bob"], 20);
        let p0 = PlayerId(0);
        assert_eq!(game.player_order, vec![PlayerId(0), PlayerId(1)]);

        let sa = SpellAbility::new_simple(None, p0, "SP$ ReverseTurnOrder");

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

        assert_eq!(ctx.game.player_order, vec![PlayerId(1), PlayerId(0)]);
    }
}
