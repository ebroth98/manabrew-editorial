use forge_foundation::ZoneType;

use super::{matches_valid_cards, parse_param, resolve_numeric_svar, EffectContext};
use crate::ids::CardId;
use crate::spellability::SpellAbility;

/// `SP$ PumpAll` — modify P/T of all matching permanents until end of turn (or perpetually).
///
/// Mirrors Java's `PumpAllEffect.java`:
/// - `NumAtt$` / `NumDef$` specify power/toughness change (signed: "+2", "-2").
/// - `ValidCards$` selects which permanents are affected.
/// - `PumpZone$` specifies the zone to look in (default: Battlefield, supports Hand).
/// - `Duration$ Perpetual` stores the bonus in `perpetual_power_modifier` /
///   `perpetual_toughness_modifier` so it persists across zone changes.
/// - Without `Duration$ Perpetual`, uses temporary `power_modifier` / `toughness_modifier`
///   (zeroed at cleanup by `step_cleanup`).
///
/// Positive values are a pump (Giant Growth effect); negative values are a
/// debuff (Rising Miasma -2/-2).
///
/// # Card script examples
/// ```text
/// A:SP$ PumpAll | ValidCards$ Creature.YouCtrl | NumAtt$ +2 | NumDef$ +2
/// A:SP$ PumpAll | ValidCards$ Creature | NumAtt$ -2 | NumDef$ -2
/// DB$ PumpAll | PumpZone$ Hand | ValidCards$ Creature.YouOwn | NumAtt$ +1 | NumDef$ +1 | Duration$ Perpetual
/// ```
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    // parse_param strips leading '+' sign via Rust's i32::from_str which accepts it.
    // Fall back to SVar resolution for Count$Kicked etc.
    let att_bonus = parse_param(&sa.ability_text, "NumAtt$ ")
        .unwrap_or_else(|| resolve_numeric_svar(ctx.game, sa, "NumAtt", 0));
    let def_bonus = parse_param(&sa.ability_text, "NumDef$ ")
        .unwrap_or_else(|| resolve_numeric_svar(ctx.game, sa, "NumDef", 0));

    // Parse KW$ parameter for keyword grants (e.g. "KW$ Haste" or "KW$ Flying & Trample")
    let keywords: Vec<String> = sa
        .params
        .get("KW")
        .map(|kw_str| {
            kw_str
                .split('&')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();

    if att_bonus == 0 && def_bonus == 0 && keywords.is_empty() {
        return;
    }

    let valid_cards_filter = sa
        .params
        .get("ValidCards")
        .map(|s| s.to_string())
        .unwrap_or_else(|| "Creature".to_string());
    let activating_player = sa.activating_player;

    // Determine the zone to look for cards in (default: Battlefield).
    let pump_zone_str = sa
        .params
        .get("PumpZone")
        .unwrap_or("Battlefield");
    let pump_zone = match pump_zone_str {
        s if s.eq_ignore_ascii_case("Hand") => ZoneType::Hand,
        _ => ZoneType::Battlefield,
    };

    // Perpetual effects persist across zone changes (stored in perpetual_*_modifier).
    let is_perpetual = sa
        .params
        .get("Duration")
        .map(|d| d.eq_ignore_ascii_case("Perpetual"))
        .unwrap_or(false);

    // Pass 1 — collect matching cards in the target zone
    let player_ids = ctx.game.player_order.clone();
    let mut to_pump: Vec<CardId> = Vec::new();
    for &pid in &player_ids {
        let zone_cards = ctx.game.cards_in_zone(pump_zone, pid).to_vec();
        for cid in zone_cards {
            if matches_valid_cards(ctx.game.card(cid), &valid_cards_filter, activating_player) {
                to_pump.push(cid);
            }
        }
    }

    // Pass 2 — apply modifiers
    for card_id in to_pump {
        if ctx.game.card(card_id).zone != pump_zone {
            continue; // already moved
        }
        if is_perpetual {
            ctx.game.card_mut(card_id).perpetual_power_modifier += att_bonus;
            ctx.game.card_mut(card_id).perpetual_toughness_modifier += def_bonus;
        } else {
            ctx.game.card_mut(card_id).power_modifier += att_bonus;
            ctx.game.card_mut(card_id).toughness_modifier += def_bonus;
            for kw in &keywords {
                ctx.game.card_mut(card_id).pump_keywords.add(kw);
            }
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
        rng: &'a mut dyn crate::game_rng::GameRng,
    ) -> EffectContext<'a> {
        EffectContext {
            game,
            agents,
            trigger_handler: th,
            token_templates: templates,
            mana_pools: mp,
            parent_target_card: None,
            rng,
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
        let mut rng_adapter = crate::game_rng::ThreadRngAdapter;
        let mut ctx = make_ctx(
            &mut game,
            &mut agents,
            &mut th,
            &mut mp,
            &templates,
            &mut rng_adapter,
        );
        super::resolve(&mut ctx, &sa);

        assert_eq!(ctx.game.card(c1).power(), 4); // 2+2
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
        let mut rng_adapter = crate::game_rng::ThreadRngAdapter;
        let mut ctx = make_ctx(
            &mut game,
            &mut agents,
            &mut th,
            &mut mp,
            &templates,
            &mut rng_adapter,
        );
        super::resolve(&mut ctx, &sa);

        assert_eq!(ctx.game.card(c1).power(), 0); // 2-2
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
        let mut rng_adapter = crate::game_rng::ThreadRngAdapter;
        let mut ctx = make_ctx(
            &mut game,
            &mut agents,
            &mut th,
            &mut mp,
            &templates,
            &mut rng_adapter,
        );
        super::resolve(&mut ctx, &sa);

        assert_eq!(ctx.game.card(mine).power(), 4); // boosted
        assert_eq!(ctx.game.card(theirs).power(), 2); // unchanged
    }
}
