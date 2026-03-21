use super::{resolve_numeric_svar, EffectContext};
use crate::agent::PlayerAgent;
use crate::agent::TargetChoice;
use crate::game::GameState;
use crate::ids::PlayerId;
use crate::parsing::keys;
use crate::spellability::target_restrictions::{
    get_all_battlefield_permanents_filtered, get_all_candidates_creature_filtered,
    get_all_candidates_spells, get_valid_cards_in_zone, TargetKind,
};
use crate::spellability::{build_spell_ability, SpellAbility};
use crate::parsing::Params;

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
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    // Java chains chosen charm modes onto the root SpellAbility during casting
    // (via make_choices_precast), then the stack resolver walks the full
    // sub-ability chain. If sub-abilities are already present, just return —
    // the stack's resolve_ability loop will walk and resolve each sub-ability.
    if sa.sub_ability.is_some() {
        return;
    }

    let source_id = match sa.source {
        Some(id) => id,
        None => return,
    };

    let choices_str = sa.params.get_cloned(keys::CHOICES).unwrap_or_default();
    if choices_str.is_empty() {
        return;
    }

    let charm_num = resolve_numeric_svar(ctx.game, sa, keys::CHARM_NUM, 1).max(0) as usize;
    let min_charm_num =
        resolve_numeric_svar(ctx.game, sa, keys::MIN_CHARM_NUM, charm_num as i32).max(0) as usize;

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
            let params = Params::from_raw(text);
            params
                .get_cloned(keys::SPELL_DESCRIPTION)
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
    let entwine_paid = sa.params.has(keys::ENTWINE) || sa.kicked; // Entwine is sometimes represented as kicked

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
    // Check if modes were pre-selected (Spree — chosen during casting before payment)
    let pre_selected = ctx.game.card_mut(source_id).chosen_modes.take();
    let chosen_indices: Vec<usize> = if let Some(pre) = pre_selected {
        // Spree: modes already chosen before payment
        pre
    } else if entwine_paid || (has_entwine && sa.kicked) {
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
        // Propagate trigger context from parent SA to mode SA so that
        // effects like Modular can read trigger_remembered_amount.
        mode_sa.trigger_remembered_amount = sa.trigger_remembered_amount;

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

pub fn make_choices_precast(
    game: &mut GameState,
    agents: &mut [Box<dyn PlayerAgent>],
    sa: &mut SpellAbility,
) -> bool {
    let source_id = match sa.source {
        Some(id) => id,
        None => return true,
    };

    let choices_str = sa.params.get_cloned(keys::CHOICES).unwrap_or_default();
    if choices_str.is_empty() {
        return true;
    }

    let player = sa.activating_player;
    let mode_svars: Vec<&str> = choices_str.split(',').map(|s| s.trim()).collect();
    let svars = game.card(source_id).svars.clone();
    let mode_texts: Vec<String> = mode_svars
        .iter()
        .filter_map(|svar| svars.get(*svar).cloned())
        .collect();
    if mode_texts.is_empty() {
        return false;
    }

    let mode_descriptions: Vec<String> = mode_texts
        .iter()
        .map(|text| {
            let params = Params::from_raw(text);
            params
                .get_cloned(keys::SPELL_DESCRIPTION)
                .unwrap_or_else(|| text.clone())
        })
        .collect();

    let valid_mode_indices: Vec<usize> = mode_texts
        .iter()
        .enumerate()
        .filter(|(_, text)| mode_has_valid_targets_in_game(game, text, player, source_id))
        .map(|(i, _)| i)
        .collect();
    if valid_mode_indices.is_empty() {
        return false;
    }

    let valid_descriptions: Vec<String> = valid_mode_indices
        .iter()
        .map(|&i| mode_descriptions[i].clone())
        .collect();

    let has_entwine = game.card(source_id).get_entwine_cost().is_some();
    let has_escalate = game.card(source_id).get_escalate_cost().is_some();
    let can_repeat = sa.params.has(keys::CAN_REPEAT_MODES);

    let mut charm_num = resolve_numeric_svar(game, sa, keys::CHARM_NUM, 1).max(0) as usize;
    let min_charm_num =
        resolve_numeric_svar(game, sa, keys::MIN_CHARM_NUM, charm_num as i32).max(0) as usize;
    if has_escalate {
        charm_num = mode_texts.len();
    }
    if !can_repeat && min_charm_num > valid_mode_indices.len() {
        return false;
    }

    let pre_selected = game.card_mut(source_id).chosen_modes.take();
    let chosen_indices: Vec<usize> = if let Some(pre) = pre_selected {
        pre
    } else if sa.params.has(keys::ENTWINE) || (has_entwine && sa.kicked) {
        valid_mode_indices.clone()
    } else {
        let card_name = game.card(source_id).card_name.clone();
        let chosen = agents[player.index()].choose_mode(
            player,
            &valid_descriptions,
            min_charm_num,
            charm_num.min(valid_mode_indices.len()),
            Some(&card_name),
        );
        chosen
            .into_iter()
            .filter_map(|i| valid_mode_indices.get(i).copied())
            .collect()
    };

    if chosen_indices.len() < min_charm_num {
        return false;
    }

    game.card_mut(source_id).chosen_modes = Some(chosen_indices.clone());
    sa.sub_ability = None;
    let parent_trigger_remembered = sa.trigger_remembered_amount;
    for idx in chosen_indices {
        if idx >= mode_texts.len() {
            continue;
        }
        let mut mode_sa = build_spell_ability(game, source_id, &mode_texts[idx], player);
        mode_sa.source = Some(source_id);
        // Propagate trigger context from parent SA so effects like Modular
        // can access trigger_remembered_amount at resolution time.
        mode_sa.trigger_remembered_amount = parent_trigger_remembered;
        append_subability(sa, mode_sa);
    }

    true
}

/// Pre-cast legality check for Charm mode selection.
///
/// Mirrors Java `CharmEffect.makeChoices` behavior enough to decide whether the
/// cast should proceed at all: if not enough legal modes exist, casting fails.
pub(crate) fn can_make_choices_precast(
    game: &GameState,
    player: PlayerId,
    source_id: crate::ids::CardId,
    charm_sa_text: &str,
) -> bool {
    let sa_params = Params::from_raw(charm_sa_text);
    let Some(choices_str) = sa_params.get(keys::CHOICES) else {
        return true;
    };

    let mode_svars: Vec<&str> = choices_str.split(',').map(|s| s.trim()).collect();
    if mode_svars.is_empty() {
        return false;
    }

    let svars = game.card(source_id).svars.clone();
    let mode_texts: Vec<String> = mode_svars
        .iter()
        .filter_map(|svar| svars.get(*svar).cloned())
        .collect();
    if mode_texts.is_empty() {
        return false;
    }

    let mut charm_num: usize = sa_params
        .get(keys::CHARM_NUM)
        .and_then(|s| {
            s.parse().ok().or_else(|| {
                // If not a plain integer, try resolving as an SVar from the source card.
                game.card(source_id)
                    .svars
                    .get(s.trim())
                    .and_then(|v| v.parse().ok())
            })
        })
        .unwrap_or(1);
    let min_charm_num: usize = sa_params
        .get(keys::MIN_CHARM_NUM)
        .and_then(|s| {
            s.parse().ok().or_else(|| {
                game.card(source_id)
                    .svars
                    .get(s.trim())
                    .and_then(|v| v.parse().ok())
            })
        })
        .unwrap_or(charm_num);
    let can_repeat = sa_params.has(keys::CAN_REPEAT_MODES);

    let valid_count = mode_texts
        .iter()
        .filter(|text| mode_has_valid_targets_in_game(game, text, player, source_id))
        .count();

    if valid_count == 0 {
        return false;
    }

    if !can_repeat && min_charm_num > valid_count {
        return false;
    }

    if !can_repeat {
        charm_num = charm_num.min(valid_count);
    }

    charm_num >= min_charm_num
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
    mode_has_valid_targets_in_game(ctx.game, mode_text, player, source_id)
}

fn append_subability(root: &mut SpellAbility, mode_sa: SpellAbility) {
    let mut slot = &mut root.sub_ability;
    loop {
        match slot {
            Some(node) => slot = &mut node.sub_ability,
            None => {
                *slot = Some(Box::new(mode_sa));
                return;
            }
        }
    }
}

fn mode_has_valid_targets_in_game(
    game: &GameState,
    mode_text: &str,
    player: PlayerId,
    source_id: crate::ids::CardId,
) -> bool {
    let sa = build_spell_ability(game, source_id, mode_text, player);
    let tr = match &sa.target_restrictions {
        Some(tr) => tr,
        None => return true, // No targeting = always valid
    };
    match &tr.target_kind {
        TargetKind::Player => true,
        TargetKind::Spell => !get_all_candidates_spells(game).is_empty(),
        TargetKind::Creature(ref filter) => {
            !crate::spellability::target_restrictions::apply_other_source_filter(
                get_all_candidates_creature_filtered(game, filter.as_deref(), player),
                filter.as_deref(),
                sa.source,
            )
            .is_empty()
        }
        TargetKind::Permanent(ref filter) => {
            !crate::spellability::target_restrictions::apply_other_source_filter(
                get_all_battlefield_permanents_filtered(game, filter.as_deref(), player),
                filter.as_deref(),
                sa.source,
            )
            .is_empty()
        }
        TargetKind::CardInZone { zone, filter } => game.player_order.iter().any(|&pid| {
            !get_valid_cards_in_zone(game, *zone, pid, filter.as_deref(), sa.source).is_empty()
        }),
        TargetKind::Any => {
            if crate::spellability::target_restrictions::any_target_allows_players(&tr.valid_tgts)
                && !game.alive_players().is_empty()
            {
                return true;
            }
            crate::spellability::target_restrictions::get_all_candidates_any_filtered(
                game,
                &tr.valid_tgts,
                player,
            )
            .into_iter()
            .any(|cid| {
                crate::spellability::target_restrictions::can_be_targeted_by_sa(
                    game, cid, player, &sa,
                )
            })
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
            let valid = crate::spellability::target_restrictions::apply_other_source_filter(
                get_all_candidates_creature_filtered(ctx.game, filter.as_deref(), player),
                filter.as_deref(),
                mode_sa.source,
            );
            if let Some(card) = ctx.agents[player.index()].choose_target_card(player, &valid) {
                mode_sa.target_chosen.target_card = Some(card);
            }
        }

        TargetKind::Permanent(ref filter) => {
            let valid = crate::spellability::target_restrictions::apply_other_source_filter(
                get_all_battlefield_permanents_filtered(ctx.game, filter.as_deref(), player),
                filter.as_deref(),
                mode_sa.source,
            );
            if let Some(card) = ctx.agents[player.index()].choose_target_card(player, &valid) {
                mode_sa.target_chosen.target_card = Some(card);
            }
        }

        TargetKind::CardInZone { zone, filter } => {
            let zone = *zone;
            let filter = filter.clone();
            let mut valid = Vec::new();
            for &pid in &ctx.game.player_order.clone() {
                let zone_cards =
                    get_valid_cards_in_zone(ctx.game, zone, pid, filter.as_deref(), mode_sa.source);
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
            let valid_players =
                if crate::spellability::target_restrictions::any_target_allows_players(
                    &tr.valid_tgts,
                ) {
                    ctx.game.alive_players()
                } else {
                    Vec::new()
                };
            let valid_cards =
                crate::spellability::target_restrictions::get_all_candidates_any_filtered(
                    ctx.game,
                    &tr.valid_tgts,
                    player,
                )
                .into_iter()
                .filter(|&cid| {
                    crate::spellability::target_restrictions::can_be_targeted_by_sa(
                        ctx.game, cid, player, mode_sa,
                    )
                })
                .collect::<Vec<_>>();
            let choice =
                ctx.agents[player.index()].choose_target_any(player, &valid_players, &valid_cards);
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
