use forge_foundation::ZoneType;

use super::{matches_valid_cards, parse_param, EffectContext};
use crate::ids::CardId;
use crate::spellability::SpellAbility;

/// `SP$ PumpAll` — modify P/T of all matching permanents until end of turn.
///
/// Mirrors Java's `PumpAllEffect.java`:
/// - `NumAtt$` / `NumDef$` specify power/toughness change (signed: "+2", "-2").
/// - `ValidCards$` selects which permanents are affected.
/// - Duration is always "until end of turn" (EOT cleanup in `step_cleanup`
///   zeroes `power_modifier` / `toughness_modifier` on all battlefield creatures).
///   Perpetual pump is not yet supported.
///
/// Positive values are a pump (Giant Growth effect); negative values are a
/// debuff (Rising Miasma -2/-2).
///
/// # Card script examples
/// ```text
/// A:SP$ PumpAll | ValidCards$ Creature.YouCtrl | NumAtt$ +2 | NumDef$ +2
/// A:SP$ PumpAll | ValidCards$ Creature | NumAtt$ -2 | NumDef$ -2
/// ```
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    // parse_param strips leading '+' sign via Rust's i32::from_str which accepts it.
    let att_bonus = parse_param(&sa.ability_text, "NumAtt$ ").unwrap_or(0);
    let def_bonus = parse_param(&sa.ability_text, "NumDef$ ").unwrap_or(0);

    if att_bonus == 0 && def_bonus == 0 {
        return;
    }

    let valid_cards_filter = sa
        .params
        .get("ValidCards")
        .cloned()
        .unwrap_or_else(|| "Creature".to_string());
    let activating_player = sa.activating_player;

    // Pass 1 — collect matching battlefield permanents
    let player_ids = ctx.game.player_order.clone();
    let mut to_pump: Vec<CardId> = Vec::new();
    for &pid in &player_ids {
        let zone_cards = ctx.game.cards_in_zone(ZoneType::Battlefield, pid).to_vec();
        for cid in zone_cards {
            if matches_valid_cards(ctx.game.card(cid), &valid_cards_filter, activating_player) {
                to_pump.push(cid);
            }
        }
    }

    // Pass 2 — apply temporary modifiers (zeroed at cleanup by step_cleanup)
    for card_id in to_pump {
        if ctx.game.card(card_id).zone == ZoneType::Battlefield {
            ctx.game.card_mut(card_id).power_modifier += att_bonus;
            ctx.game.card_mut(card_id).toughness_modifier += def_bonus;
        }
    }
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

    fn make_creature(game: &mut GameState, owner: PlayerId) -> CardId {
        let c = CardInstance::new(
            CardId(0),
            "Bear".into(),
            owner,
            CardTypeLine::parse("Creature - Bear"),
            ManaCost::parse("1 G"),
            ColorSet::GREEN,
            Some(2),
            Some(2),
            vec![],
            vec![],
        );
        game.create_card(c)
    }

    fn make_ctx<'a>(
        game: &'a mut GameState,
        agents: &'a mut Vec<Box<dyn crate::agent::PlayerAgent>>,
        th: &'a mut TriggerHandler,
        mp: &'a mut Vec<ManaPool>,
        templates: &'a HashMap<String, CardInstance>,
    ) -> EffectContext<'a> {
        EffectContext {
            game,
            agents,
            trigger_handler: th,
            token_templates: templates,
            mana_pools: mp,
            parent_target_card: None,
        }
    }

    #[test]
    fn pump_all_boosts_all_creatures() {
        let mut game = GameState::new(&["Alice", "Bob"], 20);
        let p0 = PlayerId(0);
        let p1 = PlayerId(1);
        let c1 = make_creature(&mut game, p0);
        let c2 = make_creature(&mut game, p1);
        game.move_card(c1, ZoneType::Battlefield, p0);
        game.move_card(c2, ZoneType::Battlefield, p1);

        let sa = SpellAbility::new_simple(
            None,
            p0,
            "A:SP$ PumpAll | ValidCards$ Creature | NumAtt$ +2 | NumDef$ +2",
        );
        let mut th = TriggerHandler::new();
        let mut agents: Vec<Box<dyn crate::agent::PlayerAgent>> =
            vec![Box::new(PassAgent), Box::new(PassAgent)];
        let mut mp = vec![ManaPool::default(), ManaPool::default()];
        let templates = HashMap::new();
        let mut ctx = make_ctx(&mut game, &mut agents, &mut th, &mut mp, &templates);
        super::resolve(&mut ctx, &sa);

        assert_eq!(ctx.game.card(c1).power(), 4);   // 2+2
        assert_eq!(ctx.game.card(c1).toughness(), 4);
        assert_eq!(ctx.game.card(c2).power(), 4);
        assert_eq!(ctx.game.card(c2).toughness(), 4);
    }

    #[test]
    fn pump_all_debuff_reduces_pt() {
        let mut game = GameState::new(&["Alice", "Bob"], 20);
        let p0 = PlayerId(0);
        let c1 = make_creature(&mut game, p0);
        game.move_card(c1, ZoneType::Battlefield, p0);

        // Rising Miasma: -2/-2 to all
        let sa = SpellAbility::new_simple(
            None,
            p0,
            "A:SP$ PumpAll | ValidCards$ Creature | NumAtt$ -2 | NumDef$ -2",
        );
        let mut th = TriggerHandler::new();
        let mut agents: Vec<Box<dyn crate::agent::PlayerAgent>> =
            vec![Box::new(PassAgent), Box::new(PassAgent)];
        let mut mp = vec![ManaPool::default(), ManaPool::default()];
        let templates = HashMap::new();
        let mut ctx = make_ctx(&mut game, &mut agents, &mut th, &mut mp, &templates);
        super::resolve(&mut ctx, &sa);

        assert_eq!(ctx.game.card(c1).power(), 0);    // 2-2
        assert_eq!(ctx.game.card(c1).toughness(), 0);
    }

    #[test]
    fn pump_all_you_ctrl_only_affects_your_creatures() {
        let mut game = GameState::new(&["Alice", "Bob"], 20);
        let p0 = PlayerId(0);
        let p1 = PlayerId(1);
        let mine = make_creature(&mut game, p0);
        let theirs = make_creature(&mut game, p1);
        game.move_card(mine, ZoneType::Battlefield, p0);
        game.move_card(theirs, ZoneType::Battlefield, p1);

        // Righteous Charge: creatures you control get +2/+2
        let sa = SpellAbility::new_simple(
            None,
            p0,
            "A:SP$ PumpAll | ValidCards$ Creature.YouCtrl | NumAtt$ +2 | NumDef$ +2",
        );
        let mut th = TriggerHandler::new();
        let mut agents: Vec<Box<dyn crate::agent::PlayerAgent>> =
            vec![Box::new(PassAgent), Box::new(PassAgent)];
        let mut mp = vec![ManaPool::default(), ManaPool::default()];
        let templates = HashMap::new();
        let mut ctx = make_ctx(&mut game, &mut agents, &mut th, &mut mp, &templates);
        super::resolve(&mut ctx, &sa);

        assert_eq!(ctx.game.card(mine).power(), 4);    // boosted
        assert_eq!(ctx.game.card(theirs).power(), 2);  // unchanged
    }
}
