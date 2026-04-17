use forge_foundation::ZoneType;

use super::{
    emit_zone_trigger, matches_change_type, parse_param, parse_zone_type, resolve_defined_player,
    EffectContext,
};
use crate::agent::{notify_all_agents, GameLogEvent};
use crate::parsing::keys;
use crate::spellability::SpellAbility;

/// Mirrors Java's `DigEffect.java`.
///
/// `SP$ Dig | DigNum$ N | ChangeNum$ K | DestinationZone$ Hand | DestinationZone2$ Library`
///
/// Looks at the top N cards of the target player's library.
/// The activating player chooses up to K of them and moves them to DestinationZone (default Hand).
/// The rest go to DestinationZone2 (default Library bottom).
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let dig_num = parse_param(&sa.ability_text, "DigNum$ ").unwrap_or(1) as usize;
    let optional = sa.params.has(keys::OPTIONAL);
    let skip_reorder = sa.param_is_true("SkipReorder");
    let rest_random_order = sa.param_is_true("RestRandomOrder");
    let change_all = sa
        .params
        .get(keys::CHANGE_NUM)
        .map(|s| s.eq_ignore_ascii_case("All"))
        .unwrap_or(false);
    let any_number = sa
        .params
        .get(keys::CHANGE_NUM)
        .map(|s| s.eq_ignore_ascii_case("Any"))
        .unwrap_or(false);
    let change_num = if change_all || any_number {
        dig_num
    } else {
        parse_param(&sa.ability_text, "ChangeNum$ ").unwrap_or(1) as usize
    };

    let dest_zone1 = sa
        .params
        .get(keys::DESTINATION_ZONE)
        .and_then(|s| parse_zone_type(s))
        .unwrap_or(ZoneType::Hand);
    let dest_zone2 = sa
        .params
        .get(keys::DESTINATION_ZONE_2)
        .and_then(|s| parse_zone_type(s))
        .unwrap_or(ZoneType::Library);

    // Library position for zone2 placement: -1 = bottom, 0 = top
    let lib_position2: i32 = sa
        .params
        .get(keys::LIBRARY_POSITION_2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(-1);

    let change_valid = sa
        .params
        .get(keys::CHANGE_VALID)
        .map(|s| s.to_string())
        .unwrap_or_default();

    // Determine the player whose library we dig through.
    let dig_player = sa
        .target_chosen
        .target_player
        .or_else(|| {
            sa.params
                .get(keys::DEFINED)
                .and_then(|d| resolve_defined_player(d, sa.activating_player, ctx.game))
        })
        .unwrap_or(sa.activating_player);

    let lib_len = ctx.game.cards_in_zone(ZoneType::Library, dig_player).len();
    if lib_len == 0 {
        return;
    }

    let count = dig_num.min(lib_len);

    // Take top N cards off the library.
    let mut top_n: Vec<_> = {
        let zone = ctx.game.zone_mut(ZoneType::Library, dig_player);
        let len = zone.cards.len();
        zone.cards.split_off(len - count)
    };
    // Java DigEffect iterates top cards in top-first order.
    // Our library uses index 0 = bottom, so split_off returns deepest->top.
    // Reverse to expose the same chooser order Java uses.
    top_n.reverse();

    // Filter valid choices by ChangeValid$ (e.g. "Creature").
    let valid: Vec<_> = if change_valid.is_empty() {
        top_n.clone()
    } else {
        top_n
            .iter()
            .copied()
            .filter(|&id| matches_change_type(ctx.game.card(id), &change_valid, &[]))
            .collect()
    };

    // Java DigEffect only prompts for optional skip when PromptToSkipOptionalAbility is set.
    // Otherwise Optional$ True is modeled by allowing 0 selected cards in choose_dig.
    let may_be_skipped = sa.params.has(keys::PROMPT_TO_SKIP_OPTIONAL_ABILITY);
    if optional && may_be_skipped && !valid.is_empty() {
        let source_name = sa.source.map(|cid| ctx.game.card(cid).card_name.clone());
        let prompt = sa
            .params
            .get(keys::OPTIONAL_ABILITY_PROMPT)
            .unwrap_or("Would you like to proceed with this optional ability?");
        let accepted = ctx.agents[dig_player.index()].confirm_action(
            dig_player,
            None,
            prompt,
            &[],
            source_name.as_deref(),
            Some(crate::ability::api_type::ApiType::Dig),
        );
        if !accepted {
            // Put cards back into library — reverse to restore original deepest→top order.
            top_n.reverse();
            let zone = ctx.game.zone_mut(ZoneType::Library, dig_player);
            zone.cards.extend(top_n);
            return;
        }
    }

    // Let UI agents pre-build card info for the revealed cards.
    ctx.agents[sa.activating_player.index()].on_library_peek(ctx.game, &top_n);
    if sa.params.is_true(keys::REVEAL) {
        for &card_id in &top_n {
            notify_all_agents(
                ctx.agents,
                GameLogEvent::rule("Reveal Library cards")
                    .with_player(dig_player)
                    .with_card(card_id),
            );
        }
    }

    // Ask the chooser (activating player) which cards to take.
    // Java DigEffect skips the prompt entirely when no valid cards exist,
    // so we must also skip to avoid consuming extra RNG.
    let max_take = change_num.min(valid.len());
    let chosen = if change_all {
        valid.clone()
    } else if valid.is_empty() {
        Vec::new()
    } else {
        ctx.agents[sa.activating_player.index()].choose_dig(
            sa.activating_player,
            &valid,
            max_take,
            optional || any_number,
        )
    };

    let mut chosen: Vec<_> = chosen
        .into_iter()
        .filter(|id| valid.contains(id))
        .take(max_take)
        .collect();
    // Java reverses moved cards before moving them so the final destination
    // order matches the chooser's intended top-first order.
    chosen.reverse();

    let mut rest: Vec<_> = top_n
        .iter()
        .copied()
        .filter(|id| !chosen.contains(id))
        .collect();

    if !rest_random_order
        && !skip_reorder
        && rest.len() > 1
        && (dest_zone2 == ZoneType::Library || dest_zone2 == ZoneType::Graveyard)
    {
        ctx.agents[sa.activating_player.index()].snapshot_state(ctx.game, ctx.mana_pools);
        ctx.agents[sa.activating_player.index()].on_library_peek(ctx.game, &rest);
        let reordered = ctx.agents[sa.activating_player.index()]
            .choose_reorder_library(sa.activating_player, &rest);
        if reordered.len() == rest.len() && rest.iter().all(|id| reordered.contains(id)) {
            rest = reordered;
        }
    }

    // Move chosen cards to dest_zone1.
    for &id in &chosen {
        let owner = ctx.game.card(id).owner;
        let dest_owner = if dest_zone1 == ZoneType::Battlefield {
            sa.activating_player
        } else {
            owner
        };
        ctx.move_card(id, dest_zone1, dest_owner);
        if sa.param_is_true(keys::IMPRINT) {
            if let Some(source_id) = sa.source {
                ctx.game.card_mut(source_id).add_imprinted_card(id);
            }
        }
        if sa.is_remember_changed() {
            if let Some(source_id) = sa.source {
                ctx.game.card_mut(source_id).add_remembered_card(id);
            }
        }
        if dest_zone1 == ZoneType::Battlefield {
            let _ = super::add_to_combat(ctx, sa, id, keys::ATTACKING);
        }
        emit_zone_trigger(ctx.trigger_handler, id, ZoneType::Library, dest_zone1);
    }

    // Move rest to dest_zone2.
    for &id in &rest {
        let owner = ctx.game.card(id).owner;
        if dest_zone2 == ZoneType::Library {
            // Put back into the library at the specified position.
            // lib_position2 == -1 means bottom (index 0), 0 means top.
            if lib_position2 == 0 {
                // top of library
                ctx.game.zone_mut(ZoneType::Library, owner).cards.push(id);
                ctx.game.card_mut(id).set_zone(ZoneType::Library);
            } else {
                // bottom of library
                ctx.game
                    .zone_mut(ZoneType::Library, owner)
                    .cards
                    .insert(0, id);
                ctx.game.card_mut(id).set_zone(ZoneType::Library);
            }
        } else {
            let dest_owner = if dest_zone2 == ZoneType::Battlefield {
                sa.activating_player
            } else {
                owner
            };
            ctx.move_card(id, dest_zone2, dest_owner);
            emit_zone_trigger(ctx.trigger_handler, id, ZoneType::Library, dest_zone2);
        }
    }
}

#[cfg(test)]
mod tests {
    use forge_foundation::{CardTypeLine, ColorSet, ManaCost, ZoneType};

    use crate::ability::effects::EffectContext;
    use crate::agent::{PassAgent, PlayerAgent};
    use crate::card::Card;
    use crate::combat::DefenderId;
    use crate::game::GameState;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::ManaPool;
    use crate::spellability::SpellAbility;
    use crate::trigger::handler::TriggerHandler;
    use std::collections::HashMap;

    fn make_land(game: &mut GameState, owner: PlayerId) -> CardId {
        let c = Card::new(
            CardId(0),
            "Island".into(),
            owner,
            CardTypeLine::parse("Basic Land Island"),
            ManaCost::parse(""),
            ColorSet::COLORLESS,
            None,
            None,
            vec![],
            vec![],
        );
        game.create_card(c)
    }

    /// Agent that always picks the first card offered during dig.
    struct TakeFirstAgent;
    impl PlayerAgent for TakeFirstAgent {
        fn mulligan_decision(&mut self, _: PlayerId, _: &[CardId], _: u32) -> bool {
            true
        }
        fn choose_action(
            &mut self,
            _: PlayerId,
            _: &[crate::agent::PlayOption],
            _: &[CardId],
            _: &[CardId],
            _: &[(CardId, usize)],
        ) -> crate::player::actions::PlayerAction {
            crate::player::actions::PlayerAction::PassPriority
        }
        fn choose_attackers(
            &mut self,
            _: PlayerId,
            _: &[CardId],
            _: &[DefenderId],
        ) -> Vec<(CardId, DefenderId)> {
            vec![]
        }
        fn choose_blockers(
            &mut self,
            _: PlayerId,
            _: &[CardId],
            _: &[CardId],
            _: Option<usize>,
        ) -> Vec<(CardId, CardId)> {
            vec![]
        }
        fn choose_target_player(
            &mut self,
            _: PlayerId,
            v: &[PlayerId],
            _sa: Option<&crate::spellability::SpellAbility>,
        ) -> Option<PlayerId> {
            v.first().copied()
        }
        fn choose_target_card(
            &mut self,
            _: PlayerId,
            v: &[CardId],
            _sa: Option<&crate::spellability::SpellAbility>,
        ) -> Option<CardId> {
            v.first().copied()
        }
        fn choose_target_any(
            &mut self,
            _: PlayerId,
            vp: &[PlayerId],
            vc: &[CardId],
            _sa: Option<&crate::spellability::SpellAbility>,
        ) -> crate::agent::TargetChoice {
            vp.first()
                .copied()
                .map(crate::agent::TargetChoice::Player)
                .or_else(|| vc.first().copied().map(crate::agent::TargetChoice::Card))
                .unwrap_or(crate::agent::TargetChoice::None)
        }
        fn choose_land_or_spell(&mut self, _: PlayerId) -> Option<bool> {
            None
        }
        fn choose_dig(
            &mut self,
            _player: PlayerId,
            cards: &[CardId],
            max: usize,
            _optional: bool,
        ) -> Vec<CardId> {
            cards.iter().copied().take(max).collect()
        }
        fn choose_targets_for(
            &mut self,
            _sa: &mut SpellAbility,
            _game: &GameState,
            _mana_pools: &[ManaPool],
        ) -> bool {
            false
        }
    }

    #[test]
    fn dig_moves_chosen_to_hand() {
        let mut game = GameState::new(&["Alice", "Bob"], 20);
        let p0 = PlayerId(0);

        let a = make_land(&mut game, p0);
        let b = make_land(&mut game, p0);
        let c = make_land(&mut game, p0);
        // Library (bottom→top): a, b, c  → c is on top
        game.zone_mut(ZoneType::Library, p0).cards = vec![a, b, c];

        // Dig 3, take 1 to hand, rest go to graveyard.
        let sa = SpellAbility::new_simple(
            None,
            p0,
            "SP$ Dig | DigNum$ 3 | ChangeNum$ 1 | DestinationZone2$ Graveyard | NoReveal$ True",
        );
        let mut trigger_handler = TriggerHandler::new();
        let mut agents: Vec<Box<dyn PlayerAgent>> =
            vec![Box::new(TakeFirstAgent), Box::new(PassAgent)];
        let mut mana_pools = vec![ManaPool::default(), ManaPool::default()];
        let token_templates = HashMap::new();
        let templates_variants: HashMap<(String, String), usize> = HashMap::new();
        let token_fallback: HashMap<String, String> = HashMap::new();
        let edition_dates: HashMap<String, String> = HashMap::new();
        let mut rng_adapter = crate::game_rng::ThreadRngAdapter;
        let mut ctx = EffectContext {
            game: &mut game,
            combat: None,
            agents: &mut agents,
            trigger_handler: &mut trigger_handler,
            token_templates: &token_templates,
            token_art_variants: &templates_variants,
            token_fallback: &token_fallback,
            edition_dates: &edition_dates,
            mana_pools: &mut mana_pools,
            parent_target_card: None,
            rng: &mut rng_adapter,
        };

        super::resolve(&mut ctx, &sa);

        // 1 card goes to hand, 2 go to graveyard.
        assert_eq!(ctx.game.cards_in_zone(ZoneType::Hand, p0).len(), 1);
        assert_eq!(ctx.game.cards_in_zone(ZoneType::Graveyard, p0).len(), 2);
        assert_eq!(ctx.game.cards_in_zone(ZoneType::Library, p0).len(), 0);
    }
}
