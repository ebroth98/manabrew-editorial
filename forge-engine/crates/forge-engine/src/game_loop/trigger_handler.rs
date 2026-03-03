use super::*;

impl GameLoop {
    pub(crate) fn process_triggers(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
    ) {
        let pending = self.trigger_handler.run_waiting_triggers(game);
        for mut pt in pending {
            let source_name = pt
                .entry
                .spell_ability
                .source
                .and_then(|id| game.cards.get(id.index()).map(|c| c.card_name.clone()))
                .unwrap_or_else(|| "Triggered ability".to_string());
            let player_name = game
                .player(pt.entry.spell_ability.activating_player)
                .name
                .clone();
            if pt.optional {
                // Prompt the deciding player, passing the SA's API type so AI agents
                // can make informed decisions (e.g. Java's PumpAi declines optional
                // non-targeted pump triggers).
                let api = pt.entry.spell_ability.api.as_deref();
                let accepted = agents[pt.decider.index()].choose_optional_trigger(
                    pt.decider,
                    &pt.description,
                    Some(&source_name),
                    api,
                );
                if !accepted {
                    continue; // Player declined the optional trigger
                }
            }

            // Set up targets for the triggered ability before putting on stack.
            // Mirrors Java's behavior where triggered ability targets are chosen
            // via chooseSingleEntityForEffect (always picks first alphabetically,
            // no RNG consumed). This is different from spell targeting which uses
            // setupDeterministicTargets with RNG.
            //
            // If the trigger requires targets but none are valid, skip it entirely.
            // Mirrors Java's `brains.doTrigger()` returning false when no valid
            // targets exist — the trigger is simply not played.
            if !setup_trigger_targets_fixed(game, &mut pt.entry.spell_ability) {
                continue;
            }

            let trigger_mode = pt
                .entry
                .spell_ability
                .trigger_source
                .and_then(|source_id| {
                    pt.entry.spell_ability.trigger_index.and_then(|idx| {
                        game.cards
                            .get(source_id.index())
                            .and_then(|c| c.triggers.get(idx))
                            .map(|t| trigger_mode_name(&t.mode))
                    })
                })
                .unwrap_or_else(|| "DelayedOrUnknown".to_string());
            let trigger_api = pt
                .entry
                .spell_ability
                .api
                .clone()
                .unwrap_or_else(|| "Unknown".to_string());
            let trigger_msg = if pt.description.is_empty() {
                format!(
                    "Trigger fired: mode={} | api={} | source={}",
                    trigger_mode, trigger_api, source_name
                )
            } else {
                format!(
                    "Trigger fired: mode={} | api={} | source={} | {}",
                    trigger_mode, trigger_api, source_name, pt.description
                )
            };
            agents[pt.entry.spell_ability.activating_player.index()].notify(&trigger_msg);
            game.stack.push(pt.entry);
            self.log_stack_push(&source_name, &player_name);
        }
    }
}

/// Set up targets for a triggered ability deterministically (first alphabetically).
///
/// Mirrors Java's `chooseSingleEntityForEffect` which always picks the first
/// valid target sorted alphabetically, without consuming any RNG. This is
/// different from spell targeting which goes through `setupDeterministicTargets`
/// and consumes RNG.
///
/// Walks the SA chain (including sub-abilities) to set up all targets.
/// Returns `false` if the *main* ability requires targets but none are valid
/// (mirrors Java's `brains.doTrigger()` returning false). Sub-abilities with
/// missing targets don't prevent the trigger from firing.
fn setup_trigger_targets_fixed(game: &GameState, sa: &mut SpellAbility) -> bool {
    // Set up targets for the main ability
    if !setup_single_sa_targets_fixed(game, sa) {
        return false; // Main ability needs targets but has none — skip trigger
    }

    // Walk sub-ability chain
    let mut current = sa.sub_ability.as_deref_mut();
    while let Some(sub) = current {
        setup_single_sa_targets_fixed(game, sub);
        current = sub.sub_ability.as_deref_mut();
    }
    true
}

/// Set up targets for a single SpellAbility node (no RNG, first alphabetically).
/// Returns `false` if the ability requires targets but none are valid.
fn setup_single_sa_targets_fixed(game: &GameState, sa: &mut SpellAbility) -> bool {
    let tr = match &sa.target_restrictions {
        Some(tr) => tr.clone(),
        None => return true, // No targeting required — always OK
    };

    let player = sa.activating_player;

    match &tr.target_kind {
        target_restrictions::TargetKind::Creature(ref filter) => {
            let mut valid: Vec<CardId> =
                target_restrictions::get_all_candidates_creature_filtered(
                    game,
                    filter.as_deref(),
                    player,
                )
                .into_iter()
                .filter(|&cid| {
                    target_restrictions::can_be_targeted_by_sa(game, cid, player, sa)
                })
                .collect();
            // Sort by name, pick first (matches Java's chooseSingleEntityForEffect)
            valid.sort_by(|&a, &b| game.card(a).card_name.cmp(&game.card(b).card_name));
            if let Some(&target) = valid.first() {
                sa.target_chosen.target_card = Some(target);
                true
            } else {
                false // Targeted but no valid targets
            }
        }
        target_restrictions::TargetKind::Permanent(ref filter) => {
            let mut valid: Vec<CardId> =
                target_restrictions::get_all_battlefield_permanents_filtered(
                    game,
                    filter.as_deref(),
                    player,
                )
                .into_iter()
                .filter(|&cid| {
                    target_restrictions::can_be_targeted_by_sa(game, cid, player, sa)
                })
                .collect();
            valid.sort_by(|&a, &b| game.card(a).card_name.cmp(&game.card(b).card_name));
            if let Some(&target) = valid.first() {
                sa.target_chosen.target_card = Some(target);
                true
            } else {
                false // Targeted but no valid targets
            }
        }
        target_restrictions::TargetKind::Player => {
            // Pick first alive player sorted by name
            let mut players = game.alive_players();
            players.sort_by(|&a, &b| game.player(a).name.cmp(&game.player(b).name));
            if let Some(&target) = players.first() {
                sa.target_chosen.target_player = Some(target);
                true
            } else {
                false
            }
        }
        target_restrictions::TargetKind::Any => {
            // Players first (by name), then creatures (by name)
            let mut players = game.alive_players();
            players.sort_by(|&a, &b| game.player(a).name.cmp(&game.player(b).name));
            if let Some(&target) = players.first() {
                sa.target_chosen.target_player = Some(target);
                true
            } else {
                let mut creatures = target_restrictions::get_all_candidates_creatures(game);
                creatures.sort_by(|&a, &b| game.card(a).card_name.cmp(&game.card(b).card_name));
                if let Some(&target) = creatures.first() {
                    sa.target_chosen.target_card = Some(target);
                    true
                } else {
                    false
                }
            }
        }
        _ => true, // Spell targeting and other kinds — don't block
    }
}

fn trigger_mode_name(mode: &crate::trigger::TriggerMode) -> String {
    let dbg = format!("{:?}", mode);
    dbg.split(|c: char| c == '{' || c.is_whitespace())
        .next()
        .unwrap_or("Unknown")
        .to_string()
}
