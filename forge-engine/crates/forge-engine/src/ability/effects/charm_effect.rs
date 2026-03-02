use forge_foundation::ZoneType;

use super::{matches_valid_cards, EffectContext};
use crate::agent::TargetChoice;
use crate::ids::PlayerId;
use crate::spellability::target_restrictions::{
    get_all_battlefield_permanents_filtered, get_all_candidates_creature_filtered,
    get_all_candidates_spells, get_valid_cards_in_zone, TargetKind,
};
use crate::spellability::{build_spell_ability, SpellAbility};
use crate::trigger::parse_pipe_params;

/// `SP$ Charm` — modal spell: player chooses N effects from a list.
///
/// Mirrors Java's `CharmEffect.java`.
///
/// # Card script format
/// ```text
/// A:SP$ Charm | Choices$ Mode1,Mode2,Mode3 | [CharmNum$ 2] | [MinCharmNum$ 1]
/// SVar:Mode1:DB$ Draw | NumCards$ 1 | SpellDescription$ Draw a card.
/// SVar:Mode2:DB$ Destroy | ValidTgts$ Creature | SpellDescription$ Destroy target creature.
/// ```
///
/// Modes are resolved at resolution time (shortcut vs MTG rules cast-time targeting),
/// which is acceptable for single-player implementations.
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let source_id = match sa.source {
        Some(id) => id,
        None => return,
    };

    let choices_str = sa.params.get("Choices").cloned().unwrap_or_default();
    if choices_str.is_empty() {
        return;
    }

    let charm_num: usize = sa
        .params
        .get("CharmNum")
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    let min_charm_num: usize = sa
        .params
        .get("MinCharmNum")
        .and_then(|s| s.parse().ok())
        .unwrap_or(charm_num);

    let player = sa.activating_player;

    // Collect SVar names for each mode
    let mode_svars: Vec<&str> = choices_str.split(',').map(|s| s.trim()).collect();

    // Get mode texts from the source card's SVars
    let svars = ctx.game.card(source_id).svars.clone();
    let mode_texts: Vec<String> = mode_svars
        .iter()
        .filter_map(|svar| svars.get(*svar).cloned())
        .collect();

    if mode_texts.is_empty() {
        return;
    }

    // Extract SpellDescription$ for each mode (human-readable label)
    let mode_descriptions: Vec<String> = mode_texts
        .iter()
        .map(|text| {
            let params = parse_pipe_params(text);
            params
                .get("SpellDescription")
                .cloned()
                .unwrap_or_else(|| text.clone())
        })
        .collect();

    // Filter modes to only those with valid targets (matching Java's CharmEffect
    // which passes only `possible` modes to chooseModeForAbility).
    let valid_mode_indices: Vec<usize> = mode_texts
        .iter()
        .enumerate()
        .filter(|(_, text)| mode_has_valid_targets(ctx, text, player, source_id))
        .map(|(i, _)| i)
        .collect();

    if valid_mode_indices.is_empty() {
        return; // No modes have valid targets — spell fizzles
    }

    let valid_descriptions: Vec<String> = valid_mode_indices
        .iter()
        .map(|&i| mode_descriptions[i].clone())
        .collect();

    // Check if Entwine was paid (SA flag) — if so, auto-select all modes
    let entwine_paid = sa.params.get("Entwine").map(|_| true).unwrap_or(false) || sa.kicked; // Entwine is sometimes represented as kicked

    // Check source card for Entwine/Escalate keywords
    let has_entwine = ctx.game.card(source_id).get_entwine_cost().is_some();
    let has_escalate = ctx.game.card(source_id).get_escalate_cost().is_some();

    // If Escalate, allow choosing more modes (up to all)
    let charm_num = if has_escalate {
        mode_texts.len()
    } else {
        charm_num
    };

    // Ask the activating player to choose mode(s)
    let card_name = ctx.game.card(source_id).card_name.clone();
    let chosen_indices: Vec<usize> = if entwine_paid || (has_entwine && sa.kicked) {
        // Entwine: all valid modes (mapped back to original indices)
        valid_mode_indices.clone()
    } else {
        let agent_choices = ctx.agents[player.index()].choose_mode(
            player,
            &valid_descriptions,
            min_charm_num,
            charm_num.min(valid_mode_indices.len()),
            Some(&card_name),
        );
        // Map agent choices (indices into valid_descriptions) back to original mode indices
        agent_choices
            .into_iter()
            .filter_map(|i| valid_mode_indices.get(i).copied())
            .collect()
    };

    // Resolve each chosen mode in declaration order
    for idx in chosen_indices {
        if idx >= mode_texts.len() {
            continue;
        }
        let mode_text = &mode_texts[idx];

        // Build the mode's SpellAbility (recursively includes SubAbility$ chain)
        let mut mode_sa = build_spell_ability(ctx.game, source_id, mode_text, player);
        mode_sa.source = Some(source_id);

        // Walk the sub-ability chain: set targets then resolve each node
        let mut cur_opt: Option<SpellAbility> = Some(mode_sa);
        while let Some(mut cur_sa) = cur_opt {
            setup_mode_targets(ctx, &mut cur_sa, player);
            super::resolve_effect(ctx, &cur_sa);
            // Descend into sub-ability (unbox)
            cur_opt = cur_sa.sub_ability.map(|b| *b);
            if ctx.game.game_over {
                break;
            }
        }

        if ctx.game.game_over {
            break;
        }
    }
}

/// Check whether a charm mode has valid targets (or needs no targets).
///
/// Mirrors Java's pre-filtering of `possible` modes in CharmEffect before
/// calling `chooseModeForAbility`. Modes without targeting requirements are
/// always valid. Modes requiring specific targets are valid only if at least
/// one legal candidate exists.
fn mode_has_valid_targets(
    ctx: &EffectContext,
    mode_text: &str,
    player: PlayerId,
    source_id: crate::ids::CardId,
) -> bool {
    let sa = build_spell_ability(ctx.game, source_id, mode_text, player);
    let tr = match &sa.target_restrictions {
        Some(tr) => tr,
        None => return true, // No targeting = always valid
    };
    match &tr.target_kind {
        TargetKind::Player => true,
        TargetKind::Spell => !get_all_candidates_spells(ctx.game).is_empty(),
        TargetKind::Creature(ref filter) => {
            !get_all_candidates_creature_filtered(ctx.game, filter.as_deref(), player).is_empty()
        }
        TargetKind::Permanent(ref filter) => {
            !get_all_battlefield_permanents_filtered(ctx.game, filter.as_deref(), player).is_empty()
        }
        TargetKind::CardInZone { zone, filter } => ctx.game.player_order.iter().any(|&pid| {
            !get_valid_cards_in_zone(ctx.game, *zone, pid, filter.as_deref()).is_empty()
        }),
        TargetKind::Any => {
            let filter = tr
                .valid_tgts
                .first()
                .map(String::as_str)
                .unwrap_or("Permanent");
            ctx.game.player_order.iter().any(|&pid| {
                ctx.game
                    .cards_in_zone(ZoneType::Battlefield, pid)
                    .iter()
                    .any(|&cid| matches_valid_cards(ctx.game.card(cid), filter, player))
            }) || !ctx.game.alive_players().is_empty()
        }
        TargetKind::None => true,
    }
}

/// Set up targeting for a charm mode SpellAbility at resolution time.
///
/// Reads the mode's `TargetRestrictions` and calls the appropriate agent
/// method to choose targets, then stores the result in `target_chosen`.
fn setup_mode_targets(ctx: &mut EffectContext, mode_sa: &mut SpellAbility, player: PlayerId) {
    let tr = match &mode_sa.target_restrictions {
        Some(tr) => tr.clone(),
        None => return,
    };

    match &tr.target_kind {
        TargetKind::Player => {
            let players = ctx.game.alive_players();
            if let Some(p) = ctx.agents[player.index()].choose_target_player(player, &players) {
                mode_sa.target_chosen.target_player = Some(p);
            }
        }

        TargetKind::Spell => {
            let stack_ids = get_all_candidates_spells(ctx.game);
            if let Some(id) = ctx.agents[player.index()].choose_target_spell(player, &stack_ids) {
                mode_sa.target_chosen.target_stack_entry = Some(id);
            }
        }

        TargetKind::Creature(ref filter) => {
            let valid = get_all_candidates_creature_filtered(ctx.game, filter.as_deref(), player);
            if let Some(card) = ctx.agents[player.index()].choose_target_card(player, &valid) {
                mode_sa.target_chosen.target_card = Some(card);
            }
        }

        TargetKind::Permanent(ref filter) => {
            let valid =
                get_all_battlefield_permanents_filtered(ctx.game, filter.as_deref(), player);
            if let Some(card) = ctx.agents[player.index()].choose_target_card(player, &valid) {
                mode_sa.target_chosen.target_card = Some(card);
            }
        }

        TargetKind::CardInZone { zone, filter } => {
            let zone = *zone;
            let filter = filter.clone();
            let mut valid = Vec::new();
            for &pid in &ctx.game.player_order.clone() {
                let zone_cards = get_valid_cards_in_zone(ctx.game, zone, pid, filter.as_deref());
                valid.extend(zone_cards);
            }
            if let Some(card) =
                ctx.agents[player.index()].choose_target_card_from_zone(player, zone, &valid)
            {
                mode_sa.target_chosen.target_card = Some(card);
            }
        }

        // Generic fallback: use valid_tgts[0] with matches_valid_cards for battlefield
        TargetKind::Any => {
            let filter = tr
                .valid_tgts
                .first()
                .map(String::as_str)
                .unwrap_or("Permanent");
            let mut valid = Vec::new();
            for &pid in &ctx.game.player_order.clone() {
                let zone_cards = ctx.game.cards_in_zone(ZoneType::Battlefield, pid).to_vec();
                for cid in zone_cards {
                    if matches_valid_cards(ctx.game.card(cid), filter, player) {
                        valid.push(cid);
                    }
                }
            }
            if valid.is_empty() {
                // No battlefield cards match → try targeting a player
                let players = ctx.game.alive_players();
                if let Some(p) = ctx.agents[player.index()].choose_target_player(player, &players) {
                    mode_sa.target_chosen.target_player = Some(p);
                }
            } else {
                let choice = ctx.agents[player.index()].choose_target_any(
                    player,
                    &ctx.game.alive_players(),
                    &valid,
                );
                match choice {
                    TargetChoice::Player(p) => {
                        mode_sa.target_chosen.target_player = Some(p);
                    }
                    TargetChoice::Card(c) => {
                        mode_sa.target_chosen.target_card = Some(c);
                    }
                    TargetChoice::None => {}
                }
            }
        }

        TargetKind::None => {}
    }
}

#[cfg(test)]
mod tests {
    use forge_foundation::{CardTypeLine, ColorSet, ManaCost, ZoneType};
    use std::collections::{BTreeMap, HashMap};

    use crate::ability::effects::EffectContext;
    use crate::agent::PassAgent;
    use crate::card::CardInstance;
    use crate::game::GameState;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::ManaPool;
    use crate::spellability::SpellAbility;
    use crate::trigger::handler::TriggerHandler;

    fn make_spell_ability_with_svars(
        player: PlayerId,
        ability_text: &str,
        svars: &[(&str, &str)],
    ) -> SpellAbility {
        let mut sa = SpellAbility::new_simple(None, player, ability_text);
        // Inject SVars into a fake source card — we embed them in params here
        // for the unit test (in real games, svars come from CardInstance.svars).
        let _ = svars; // svars are normally on the card; this test uses a custom setup
        sa
    }

    #[test]
    fn charm_choose_mode_zero_choices_noops() {
        let mut game = GameState::new(&["Alice", "Bob"], 20);
        let p0 = PlayerId(0);

        // A charm with no Choices$ should be a no-op
        let sa = SpellAbility::new_simple(None, p0, "A:SP$ Charm");
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
        // Should not panic
        super::resolve(&mut ctx, &sa);
    }

    /// Integration test: charm with two draw modes, PassAgent picks first mode.
    /// Uses a live card with SVars so `build_spell_ability` can look them up.
    #[test]
    fn charm_resolves_chosen_non_targeted_mode() {
        let mut game = GameState::new(&["Alice", "Bob"], 20);
        let p0 = PlayerId(0);
        let p1 = PlayerId(1);

        // Build a fake "charm" card with two modes stored as SVars
        let mut svars = BTreeMap::new();
        // Mode A: draw a card (uses Defined$ You)
        svars.insert(
            "ModeA".to_string(),
            "DB$ Draw | NumCards$ 1 | Defined$ You | SpellDescription$ Draw a card.".to_string(),
        );
        // Mode B: draw a card for opponent (Defined$ Opponent)
        svars.insert(
            "ModeB".to_string(),
            "DB$ Draw | NumCards$ 1 | Defined$ Opponent | SpellDescription$ Opponent draws."
                .to_string(),
        );

        let charm_card = CardInstance::new(
            CardId(0),
            "Test Charm".into(),
            p0,
            CardTypeLine::parse("Instant"),
            ManaCost::parse("U B"),
            ColorSet::from_names("u"),
            None,
            None,
            vec!["A:SP$ Charm | Choices$ ModeA,ModeB".to_string()],
            vec![],
        );
        // We can't set svars in CardInstance::new directly, so we use create_card + mutate
        let cid = game.create_card(charm_card);
        game.card_mut(cid).svars = svars;

        // Put a card in each player's library so draw succeeds
        let dummy_a = game.create_card(CardInstance::new(
            CardId(0),
            "Dummy A".into(),
            p0,
            CardTypeLine::parse("Creature"),
            ManaCost::parse(""),
            ColorSet::COLORLESS,
            Some(1),
            Some(1),
            vec![],
            vec![],
        ));
        game.move_card(dummy_a, ZoneType::Library, p0);

        let sa = SpellAbility::new_simple(Some(cid), p0, "A:SP$ Charm | Choices$ ModeA,ModeB");

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

        // PassAgent.choose_mode picks first min modes → ModeA (draw for p0)
        let p0_hand_before = ctx.game.cards_in_zone(ZoneType::Hand, p0).len();
        super::resolve(&mut ctx, &sa);
        let p0_hand_after = ctx.game.cards_in_zone(ZoneType::Hand, p0).len();
        // p0 should have drawn 1 card
        assert_eq!(p0_hand_after, p0_hand_before + 1);
        // p1 should not have drawn
        assert_eq!(ctx.game.cards_in_zone(ZoneType::Hand, p1).len(), 0);
    }
}
