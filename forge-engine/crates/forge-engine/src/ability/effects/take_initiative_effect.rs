use super::{resolve_defined_player, EffectContext};
use crate::spellability::SpellAbility;

/// Resolve `SP$ TakeInitiative` — a player takes the initiative.
///
/// Mirrors Java `TakeInitiativeEffect.java`.
/// Sets `game.initiative_holder` to the target player and fires a TakeInitiative trigger.
/// Note: Full dungeon advancement is not implemented (requires Venture subsystem).
///
/// # Card script examples
/// ```text
/// A:SP$ TakeInitiative | Defined$ You
/// ```
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let controller = sa.activating_player;

    let defined = sa.params.get("Defined").unwrap_or("You");

    let target = resolve_defined_player(defined, controller, ctx.game).unwrap_or(controller);

    if !ctx.game.player(target).is_alive() {
        return;
    }

    ctx.game
        .player_take_initiative(target, Some(ctx.trigger_handler));
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
    fn take_initiative_sets_state() {
        let mut game = GameState::new(&["Alice", "Bob"], 20);
        let p0 = PlayerId(0);
        assert!(game.initiative_holder.is_none());

        let sa = SpellAbility::new_simple(None, p0, "SP$ TakeInitiative | Defined$ You");

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

        assert_eq!(ctx.game.initiative_holder, Some(p0));
    }
}
