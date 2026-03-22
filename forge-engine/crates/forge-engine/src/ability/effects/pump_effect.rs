use forge_foundation::ZoneType;

use super::{parse_param, resolve_numeric_svar, EffectContext};
use crate::card::perpetual::perpetual_interface::PerpetualInterface;
use crate::card::perpetual::{perpetual_keywords, perpetual_pt_boost};
use crate::parsing::keys;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    // Try direct integer first, then fall back to SVar resolution (for Count$Kicked etc.)
    let att_bonus = parse_param(&sa.ability_text, "NumAtt$ ")
        .unwrap_or_else(|| resolve_numeric_svar(ctx.game, sa, "NumAtt", 0));
    let def_bonus = parse_param(&sa.ability_text, "NumDef$ ")
        .unwrap_or_else(|| resolve_numeric_svar(ctx.game, sa, "NumDef", 0));

    // Parse KW$ parameter for keyword grants (e.g. "KW$ Haste" or "KW$ Flying & Trample")
    let keywords: Vec<String> = sa
        .params
        .get(keys::KW)
        .map(|kw_str| {
            kw_str
                .split('&')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();

    let is_perpetual = sa
        .params
        .get("Duration")
        .map(|d| d.eq_ignore_ascii_case("Perpetual"))
        .unwrap_or(false);
    let resolve_ts = if is_perpetual {
        Some(ctx.game.next_effect_timestamp())
    } else {
        None
    };

    // Overload: apply pump to ALL valid creatures instead of the chosen target.
    if sa.overloaded {
        let valid_tgts = sa.params.get(keys::VALID_TGTS).map(|s| s.to_string()).unwrap_or_default();
        let all_bf: Vec<crate::ids::CardId> = ctx
            .game
            .player_order
            .clone()
            .iter()
            .flat_map(|&pid| ctx.game.cards_in_zone(ZoneType::Battlefield, pid).to_vec())
            .collect();
        for cid in all_bf {
            if ctx.game.card(cid).zone != ZoneType::Battlefield {
                continue;
            }
            if !super::matches_valid_cards(ctx.game.card(cid), &valid_tgts, sa.activating_player) {
                continue;
            }
            if is_perpetual {
                let ts = resolve_ts.expect("perpetual resolve timestamp must exist");
                let card = ctx.game.card_mut(cid);
                perpetual_pt_boost::PerpetualPtBoost {
                    timestamp: ts,
                    power: att_bonus,
                    toughness: def_bonus,
                }
                .apply_effect(card);
                for kw in &keywords {
                    perpetual_keywords::PerpetualKeywords {
                        timestamp: ts,
                        add_keywords: vec![kw.clone()],
                        remove_keywords: Vec::new(),
                        remove_all: false,
                    }
                    .apply_effect(card);
                }
            } else {
                ctx.game.card_mut(cid).add_pt_boost(att_bonus, def_bonus);
                for kw in &keywords {
                    ctx.game.card_mut(cid).add_pump_keyword(kw);
                }
            }
        }
        return;
    }

    // Java PumpEffect resolves non-targeted pump abilities through
    // SpellAbilityEffect.getTargetCards(sa), which defaults `Defined` to `Self`
    // when the ability has no targets. Mirror that fallback here so abilities
    // like Guardian of New Benalia correctly affect their source.
    let target_card = sa.target_chosen.target_card.or_else(|| match sa.defined() {
        Some("Self") => sa.source,
        Some("ParentTarget") => ctx.parent_target_card,
        Some(_) => None,
        None if !sa.uses_targeting() => sa.source,
        None => None,
    });

    if let Some(target_card) = target_card {
        if ctx.game.card(target_card).zone == ZoneType::Battlefield {
            if is_perpetual {
                let ts = resolve_ts.expect("perpetual resolve timestamp must exist");
                let card = ctx.game.card_mut(target_card);
                perpetual_pt_boost::PerpetualPtBoost {
                    timestamp: ts,
                    power: att_bonus,
                    toughness: def_bonus,
                }
                .apply_effect(card);
                for kw in &keywords {
                    perpetual_keywords::PerpetualKeywords {
                        timestamp: ts,
                        add_keywords: vec![kw.clone()],
                        remove_keywords: Vec::new(),
                        remove_all: false,
                    }
                    .apply_effect(card);
                }
            } else {
                ctx.game.card_mut(target_card).power_modifier += att_bonus;
                ctx.game.card_mut(target_card).toughness_modifier += def_bonus;
                for kw in &keywords {
                    ctx.game
                        .card_mut(target_card)
                        .pump_keywords
                        .add(kw);
                }
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
    use crate::card::Card;
    use crate::game::GameState;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::ManaPool;
    use crate::spellability::SpellAbility;
    use crate::trigger::handler::TriggerHandler;

    fn make_creature(game: &mut GameState, owner: PlayerId, name: &str) -> CardId {
        let c = Card::new(
            CardId(0),
            name.into(),
            owner,
            CardTypeLine::parse("Creature - Human Soldier"),
            ManaCost::parse("1 W"),
            ColorSet::WHITE,
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
        templates: &'a HashMap<String, Card>,
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
    fn non_targeted_pump_defaults_to_self_like_java() {
        let mut game = GameState::new(&["Alice", "Bob"], 20);
        let p0 = PlayerId(0);
        let guardian = make_creature(&mut game, p0, "Guardian of New Benalia");
        game.move_card(guardian, ZoneType::Battlefield, p0);

        let sa = SpellAbility::new_simple(
            Some(guardian),
            p0,
            "AB$ Pump | KW$ Indestructible | SpellDescription$ CARDNAME gains indestructible until end of turn.",
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

        assert!(ctx.game.card(guardian).has_indestructible());
    }

    #[test]
    fn targeted_pump_does_not_fall_back_to_source() {
        let mut game = GameState::new(&["Alice", "Bob"], 20);
        let p0 = PlayerId(0);
        let source = make_creature(&mut game, p0, "Source");
        game.move_card(source, ZoneType::Battlefield, p0);

        let sa = SpellAbility::new_simple(
            Some(source),
            p0,
            "SP$ Pump | ValidTgts$ Creature | KW$ Indestructible",
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

        assert!(!ctx.game.card(source).has_indestructible());
    }
}
