use forge_foundation::ZoneType;

use super::EffectContext;
use crate::spellability::SpellAbility;

/// Resolve `SP$ PowerExchange` — swap power between two creatures.
///
/// Mirrors Java `PowerExchangeEffect.java`.
///
/// # Card script examples
/// ```text
/// A:SP$ PowerExchange | ValidTgts$ Creature | TgtPrompt$ Select target creature
/// ```
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let source = sa.source;
    let target = sa.target_chosen.target_card;

    let (c1, c2) = match (source, target) {
        (Some(s), Some(t)) => (s, t),
        _ => return,
    };

    if ctx.game.card(c1).zone != ZoneType::Battlefield
        || ctx.game.card(c2).zone != ZoneType::Battlefield
    {
        return;
    }

    // Both must be creatures
    if !ctx.game.card(c1).is_creature() || !ctx.game.card(c2).is_creature() {
        return;
    }

    let p1 = ctx.game.card(c1).power();
    let p2 = ctx.game.card(c2).power();

    // Calculate the modifier deltas needed to swap powers
    let c1_base = ctx
        .game
        .card(c1)
        .static_set_power
        .unwrap_or(ctx.game.card(c1).base_power.unwrap_or(0));
    let c2_base = ctx
        .game
        .card(c2)
        .static_set_power
        .unwrap_or(ctx.game.card(c2).base_power.unwrap_or(0));

    // Set power modifiers so effective power = the other's power
    ctx.game.card_mut(c1).power_modifier = p2 - c1_base - ctx.game.card(c1).static_power_modifier;
    ctx.game.card_mut(c2).power_modifier = p1 - c2_base - ctx.game.card(c2).static_power_modifier;
}

#[cfg(test)]
mod tests {
    use forge_foundation::{CardTypeLine, ColorSet, ManaCost, ZoneType};
    use std::collections::HashMap;

    use crate::ability::effects::EffectContext;
    use crate::agent::PassAgent;
    use crate::card::CardInstance;
    use crate::game::GameState;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::ManaPool;
    use crate::spellability::SpellAbility;
    use crate::trigger::handler::TriggerHandler;

    #[test]
    fn power_exchange_swaps() {
        let mut game = GameState::new(&["Alice", "Bob"], 20);
        let p0 = PlayerId(0);

        let c1 = game.create_card(CardInstance::new(
            CardId(0),
            "Bear".into(),
            p0,
            CardTypeLine::parse("Creature - Bear"),
            ManaCost::parse("1 G"),
            ColorSet::GREEN,
            Some(2),
            Some(2),
            vec![],
            vec![],
        ));
        game.move_card(c1, ZoneType::Battlefield, p0);

        let c2 = game.create_card(CardInstance::new(
            CardId(0),
            "Dragon".into(),
            p0,
            CardTypeLine::parse("Creature - Dragon"),
            ManaCost::parse("4 R R"),
            ColorSet::RED,
            Some(5),
            Some(5),
            vec![],
            vec![],
        ));
        game.move_card(c2, ZoneType::Battlefield, p0);

        assert_eq!(game.card(c1).power(), 2);
        assert_eq!(game.card(c2).power(), 5);

        let mut sa = SpellAbility::new_simple(Some(c1), p0, "SP$ PowerExchange");
        sa.target_chosen.target_card = Some(c2);

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

        assert_eq!(ctx.game.card(c1).power(), 5);
        assert_eq!(ctx.game.card(c2).power(), 2);
    }
}
