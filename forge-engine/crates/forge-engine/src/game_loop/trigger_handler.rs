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

            // Set up trigger targets via the shared SpellAbility targeting path.
            // This mirrors Java's controller flow (`chooseTargetsFor`), including
            // optional target semantics (e.g. TargetMin$ 0 may choose no target).
            if !pt
                .entry
                .spell_ability
                .setup_targets(game, agents, &self.mana_pools)
            {
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
            let mut event = crate::agent::GameLogEvent::stack(trigger_msg)
                .with_player(pt.entry.spell_ability.activating_player);
            if let Some(source_id) = pt.entry.spell_ability.source {
                event = event.with_source_card(source_id);
            }
            if let Some(target_id) = pt.entry.spell_ability.target_chosen.target_card {
                event = event.with_target_card(target_id);
            }
            crate::agent::notify_all_agents(agents, event);
            game.stack.push(pt.entry);
            self.log_stack_push(&source_name, &player_name);
        }
    }
}

fn trigger_mode_name(mode: &crate::trigger::TriggerMode) -> String {
    let dbg = format!("{:?}", mode);
    dbg.split(|c: char| c == '{' || c.is_whitespace())
        .next()
        .unwrap_or("Unknown")
        .to_string()
}
