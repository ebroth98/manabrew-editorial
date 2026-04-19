use super::{resolve_numeric_svar, EffectContext};
use crate::agent::PlayerAgent;
use crate::game::GameState;
use crate::ids::PlayerId;
use crate::parsing::keys;
use crate::parsing::Params;
use crate::spellability::{build_spell_ability, SpellAbility};

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
    let use_preselected_modes = should_use_preselected_modes(ctx.game, source_id, &mode_texts);
    // Check if modes were pre-selected (Spree — chosen during casting before payment)
    let pre_selected = if use_preselected_modes {
        ctx.game.card_mut(source_id).chosen_modes.take()
    } else {
        ctx.game.card_mut(source_id).chosen_modes = None;
        None
    };
    let mut chosen_indices: Vec<usize> = if let Some(pre) = pre_selected {
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

    // Java `CharmEffect.chainAbilities` sorts chosen modes by their declared
    // `CharmOrder` (1-based `Choices$` list index) before resolving, so Destroy
    // always runs before GainLife even if the player picked GainLife first.
    // Mirror that here — agent pick order must not leak into target prompts.
    chosen_indices.sort();

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

    let use_preselected_modes = should_use_preselected_modes(game, source_id, &mode_texts);
    let pre_selected = if use_preselected_modes {
        game.card_mut(source_id).chosen_modes.take()
    } else {
        game.card_mut(source_id).chosen_modes = None;
        None
    };
    let mut chosen_indices: Vec<usize> = if let Some(pre) = pre_selected {
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

    // Mirror Java's `CharmEffect.chainAbilities`: resolve modes in the order
    // they were declared (CharmOrder), not in the order the player picked
    // them. Otherwise mode target prompts fire in agent-pick order and
    // cascade RNG divergences vs Java.
    chosen_indices.sort();

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

/// Collect valid mode indices for a charm/modal spell.
/// Mirrors Java's `CharmEffect.makePossibleOptions(SpellAbility)`.
///
/// Returns the indices (into the mode list) of modes that have valid targets.
pub fn make_possible_options(
    game: &GameState,
    source_id: crate::ids::CardId,
    player: PlayerId,
    choices_str: &str,
) -> Vec<usize> {
    let mode_svars: Vec<&str> = choices_str.split(',').map(|s| s.trim()).collect();
    let svars = game.card(source_id).svars.clone();
    let mode_texts: Vec<String> = mode_svars
        .iter()
        .filter_map(|svar| svars.get(*svar).cloned())
        .collect();

    mode_texts
        .iter()
        .enumerate()
        .filter(|(_, text)| mode_has_valid_targets_in_game(game, text, player, source_id))
        .map(|(i, _)| i)
        .collect()
}

/// Build a formatted description string for charm mode choices.
/// Mirrors Java's `CharmEffect.makeFormatedDescription(SpellAbility)`.
///
/// Returns a description listing all available modes with their descriptions.
pub fn make_formated_description(
    game: &GameState,
    source_id: crate::ids::CardId,
    choices_str: &str,
) -> String {
    let mode_svars: Vec<&str> = choices_str.split(',').map(|s| s.trim()).collect();
    let svars = game.card(source_id).svars.clone();
    let mode_texts: Vec<String> = mode_svars
        .iter()
        .filter_map(|svar| svars.get(*svar).cloned())
        .collect();

    let mut description = String::new();
    description.push_str("Choose one —\n");
    for (i, text) in mode_texts.iter().enumerate() {
        let params = Params::from_raw(text);
        let mode_desc = params
            .get_cloned(keys::SPELL_DESCRIPTION)
            .unwrap_or_else(|| text.clone());
        description.push_str(&format!("• {}\n", mode_desc));
        let _ = i; // index for potential numbering
    }
    description
}

/// Make charm mode choices during pre-cast.
/// Mirrors Java's `CharmEffect.makeChoices(SpellAbility)`.
///
/// This is a wrapper around `make_choices_precast` for structural parity.
pub fn make_choices(
    game: &mut GameState,
    agents: &mut [Box<dyn PlayerAgent>],
    sa: &mut SpellAbility,
) -> bool {
    make_choices_precast(game, agents, sa)
}

/// Chain a list of sub-abilities (modes) onto a root spell ability.
/// Mirrors Java's `CharmEffect.chainAbilities(SpellAbility, List<AbilitySub>)`.
///
/// Appends each mode ability as a sub-ability at the end of the SA chain.
pub fn chain_abilities(
    game: &GameState,
    sa: &mut SpellAbility,
    mode_texts: &[String],
    player: PlayerId,
    source_id: crate::ids::CardId,
) {
    for mode_text in mode_texts {
        let mut mode_sa = build_spell_ability(game, source_id, mode_text, player);
        mode_sa.source = Some(source_id);
        mode_sa.trigger_remembered_amount = sa.trigger_remembered_amount;
        append_subability(sa, mode_sa);
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

fn should_use_preselected_modes(
    game: &GameState,
    source_id: crate::ids::CardId,
    mode_texts: &[String],
) -> bool {
    let source = game.card(source_id);
    if source.has_keyword("Spree") || source.has_keyword("Tiered") {
        return true;
    }

    mode_texts.iter().any(|text| {
        let params = Params::from_raw(text);
        params.has(keys::MODE_COST)
    })
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

    // Match Java CharmEffect.makePossibleOptions(): only drop a targeted mode
    // when it requires at least one target and the full targeting engine finds
    // zero legal candidates.
    if tr.get_min_targets(game, &sa) <= 0 {
        return true;
    }

    tr.has_candidates(game, player, sa.source)
}

fn setup_mode_targets(ctx: &mut EffectContext, mode_sa: &mut SpellAbility, player: PlayerId) {
    if !mode_sa.uses_targeting() {
        return;
    }
    mode_sa.targeting_player = Some(player);
    ctx.agents[player.index()].choose_targets_for(mode_sa, ctx.game, ctx.mana_pools);
}

#[cfg(test)]
mod tests {
    use forge_foundation::{CardTypeLine, ColorSet, ManaCost, ZoneType};
    use std::collections::{BTreeMap, HashMap};

    use crate::ability::effects::EffectContext;
    use crate::agent::PassAgent;
    use crate::card::Card;
    use crate::game::GameState;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::ManaPool;
    use crate::spellability::SpellAbility;
    use crate::trigger::handler::TriggerHandler;

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

        let charm_card = Card::new(
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
        // We can't set svars in Card::new directly, so we use create_card + mutate
        let cid = game.create_card(charm_card);
        game.card_mut(cid).set_svars_map(svars);

        // Put a card in each player's library so draw succeeds
        let dummy_a = game.create_card(Card::new(
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

        // PassAgent.choose_mode picks first min modes → ModeA (draw for p0)
        let p0_hand_before = ctx.game.cards_in_zone(ZoneType::Hand, p0).len();
        super::resolve(&mut ctx, &sa);
        let p0_hand_after = ctx.game.cards_in_zone(ZoneType::Hand, p0).len();
        // p0 should have drawn 1 card
        assert_eq!(p0_hand_after, p0_hand_before + 1);
        // p1 should not have drawn
        assert_eq!(ctx.game.cards_in_zone(ZoneType::Hand, p1).len(), 0);
    }

    #[test]
    fn charm_precast_does_not_reuse_stale_chosen_modes() {
        let mut game = GameState::new(&["Alice", "Bob"], 20);
        let p0 = PlayerId(0);
        let p1 = PlayerId(1);

        let mut svars = BTreeMap::new();
        svars.insert(
            "BraveTheStench".to_string(),
            "DB$ Pump | ValidTgts$ Creature.OppCtrl | TgtPrompt$ Select target creature an opponent controls. | NumAtt$ -1 | NumDef$ -1 | IsCurse$ True | SpellDescription$ Brave the Stench".to_string(),
        );
        svars.insert(
            "SearchTheBody".to_string(),
            "DB$ Token | TokenScript$ c_a_treasure_sac | TokenOwner$ You | SpellDescription$ Search the Body".to_string(),
        );

        let ghast = Card::new(
            CardId(0),
            "Shambling Ghast".into(),
            p1,
            CardTypeLine::parse("Creature Zombie"),
            ManaCost::parse("B"),
            ColorSet::from_names("b"),
            Some(1),
            Some(1),
            vec!["A:SP$ Charm | Choices$ BraveTheStench,SearchTheBody".to_string()],
            vec![],
        );
        let ghast_id = game.create_card(ghast);
        game.card_mut(ghast_id).set_svars_map(svars);

        let patient_zero = game.create_card(Card::new(
            CardId(0),
            "Patient Zero".into(),
            p0,
            CardTypeLine::parse("Creature Zombie"),
            ManaCost::parse("1 B"),
            ColorSet::from_names("b"),
            Some(2),
            Some(2),
            vec![],
            vec![],
        ));
        game.move_card(patient_zero, ZoneType::Battlefield, p0);

        let mut sa = SpellAbility::new_simple(
            Some(ghast_id),
            p1,
            "A:SP$ Charm | Choices$ BraveTheStench,SearchTheBody",
        );
        // Simulate a prior life of the same card instance choosing "Search the Body".
        game.card_mut(ghast_id).set_chosen_modes(vec![1]);
        let mut agents: Vec<Box<dyn crate::agent::PlayerAgent>> =
            vec![Box::new(PassAgent), Box::new(PassAgent)];

        assert!(super::make_choices_precast(&mut game, &mut agents, &mut sa));
        assert!(game.card(ghast_id).chosen_modes.is_none());
        assert_eq!(
            sa.sub_ability.as_ref().and_then(|sub| sub.api),
            Some(crate::ability::api_type::ApiType::Pump)
        );
    }
}
