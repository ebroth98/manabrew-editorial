use forge_foundation::ZoneType;

use super::{matches_valid_cards, resolve_numeric_svar, EffectContext};
use crate::ids::CardId;
use crate::parsing::keys;
use crate::spellability::SpellAbility;

/// `SP$ DamageAll` — deal N damage to all matching permanents and/or players.
///
/// Mirrors Java's `DamageAllEffect.java`:
/// - `ValidCards$` selects which battlefield permanents receive damage.
/// - `ValidPlayers$` set to "Player" also deals damage to every player.
/// - `NumDmg$` specifies the amount (integer or SVar reference).
///
/// # Card script examples
/// ```text
/// A:SP$ DamageAll | NumDmg$ 2 | ValidCards$ Creature
/// A:SP$ DamageAll | ValidCards$ Creature.withFlying | ValidPlayers$ Player | NumDmg$ X
/// ```
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let num_dmg = resolve_numeric_svar(ctx.game, sa, "NumDmg", 0);
    if num_dmg <= 0 {
        return;
    }

    let valid_cards_filter = sa.params.get(keys::VALID_CARDS).map(|s| s.to_string()).unwrap_or_default();
    let valid_players = sa
        .params
        .get(keys::VALID_PLAYERS)
        .unwrap_or("")
        .to_string();
    let activating_player = sa.activating_player;

    let player_ids = ctx.game.player_order.clone();

    // Pass 1 — collect matching battlefield permanents
    let mut to_damage: Vec<CardId> = Vec::new();
    if !valid_cards_filter.is_empty() {
        for &pid in &player_ids {
            let zone_cards = ctx.game.cards_in_zone(ZoneType::Battlefield, pid).to_vec();
            for cid in zone_cards {
                if matches_valid_cards(ctx.game.card(cid), &valid_cards_filter, activating_player) {
                    to_damage.push(cid);
                }
            }
        }
    }

    // Check source card for Infect/Wither keywords
    let source = sa.source;
    let (source_has_infect_keyword, source_has_wither) = if let Some(src_id) = source {
        let src = ctx.game.card(src_id);
        (
            src.has_infect(),
            src.has_wither()
                || crate::staticability::static_ability_wither_damage::is_wither_damage(
                    &ctx.game.cards,
                    src,
                ),
        )
    } else {
        (false, false)
    };

    // Pass 2 — apply damage to collected permanents
    let source = sa.source;
    for card_id in to_damage {
        if ctx.game.card(card_id).zone == ZoneType::Battlefield {
            // Protection: prevents all damage from matching sources
            if let Some(src_id) = source {
                if crate::staticability::static_ability_colorless_damage_source::target_is_protected_from_source(
                    &ctx.game.cards,
                    ctx.game.card(card_id),
                    ctx.game.card(src_id),
                ) {
                    continue;
                }
            }

            // Track damage source for DamagedBy trigger filters
            if let Some(src_id) = source {
                if !ctx
                    .game
                    .card(card_id)
                    .damage_sources_this_turn
                    .contains(&src_id)
                {
                    ctx.game
                        .card_mut(card_id)
                        .damage_sources_this_turn
                        .push(src_id);
                }
            }
            if source_has_infect_keyword || source_has_wither {
                // Infect/Wither: damage to creatures as -1/-1 counters
                if !crate::staticability::static_ability_cant_put_counter::any_cant_put_counter_on_card(
                    &ctx.game.cards,
                    ctx.game.card(card_id),
                    &crate::card::CounterType::M1M1,
                ) {
                    ctx.game
                        .card_mut(card_id)
                        .add_counter(&crate::card::CounterType::M1M1, num_dmg);
                }
            } else {
                ctx.game.deal_damage_to_card(card_id, num_dmg);
            }

            // Fire DamageDone trigger per card
            ctx.trigger_handler.run_trigger(
                crate::event::TriggerType::DamageDone,
                crate::event::RunParams {
                    damage_source: source,
                    damage_target_card: Some(card_id),
                    damage_amount: Some(num_dmg),
                    is_combat_damage: Some(false),
                    ..Default::default()
                },
                false,
            );
        }
    }

    // Deal damage to each matching player if ValidPlayers$ is set
    if !valid_players.is_empty() {
        let is_opponent_only = valid_players.contains("Opponent");
        for pid in player_ids {
            if is_opponent_only && pid == activating_player {
                continue;
            }
            let source_has_infect = if let Some(src_id) = source {
                let src = ctx.game.card(src_id);
                source_has_infect_keyword
                    || crate::staticability::static_ability_infect_damage::is_infect_damage(
                        ctx.game,
                        &ctx.game.cards,
                        pid,
                        src.controller,
                    )
            } else {
                false
            };
            if source_has_infect {
                if !crate::staticability::static_ability_cant_put_counter::any_cant_put_counter_on_player(
                    &ctx.game.cards,
                    pid,
                    &crate::card::CounterType::Poison,
                ) {
                    ctx.game.player_mut(pid).poison_counters += num_dmg;
                }
            } else {
                ctx.game.deal_damage_to_player(pid, num_dmg);
            }

            // Fire DamageDone trigger per player
            ctx.trigger_handler.run_trigger(
                crate::event::TriggerType::DamageDone,
                crate::event::RunParams {
                    damage_source: source,
                    damage_target_player: Some(pid),
                    damage_amount: Some(num_dmg),
                    is_combat_damage: Some(false),
                    ..Default::default()
                },
                false,
            );
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

    #[test]
    fn damage_all_deals_to_each_creature() {
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
            "A:SP$ DamageAll | NumDmg$ 2 | ValidCards$ Creature",
        );
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

        assert_eq!(ctx.game.card(c1).damage, 2);
        assert_eq!(ctx.game.card(c2).damage, 2);
        // Players not affected (no ValidPlayers$)
        assert_eq!(ctx.game.player(p0).life, 20);
        assert_eq!(ctx.game.player(p1).life, 20);
    }

    #[test]
    fn damage_all_with_valid_players() {
        let mut game = GameState::new(&["Alice", "Bob"], 20);
        let p0 = PlayerId(0);
        let p1 = PlayerId(1);

        let sa = SpellAbility::new_simple(
            None,
            p0,
            "A:SP$ DamageAll | NumDmg$ 3 | ValidCards$ | ValidPlayers$ Player",
        );
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

        assert_eq!(ctx.game.player(p0).life, 17);
        assert_eq!(ctx.game.player(p1).life, 17);
    }
}
