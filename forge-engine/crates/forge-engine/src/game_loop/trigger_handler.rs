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

            // ── Mirrors Java's trigger processing order ──
            // In Java (deterministic harness), the flow is:
            //   1. prepareSingleSaDeterministic:
            //      a. CharmEffect.makeChoices(sa) — select charm modes
            //      b. sa.setupTargets() — pick targets for the chosen mode
            //      (consumes RNG for mode selection + target selection)
            //   2. playStackDeterministic: adds trigger to stack
            //   3. WrappedAbility.resolve(): confirmTrigger (consumes RNG for ACCEPT/DECLINE)
            //
            // We must match this RNG consumption order:
            //   charm mode selection → target setup → [stack] → optional confirm at resolution.

            // For Charm-type triggers (e.g. Modular uses SP$ Charm | Choices$ ModularMove),
            // select the charm mode BEFORE target setup, so that setupTargets can find
            // targets on the chosen sub-ability.  Mirrors Java's CharmEffect.makeChoices().
            if pt.entry.spell_ability.api.as_deref() == Some("Charm") {
                if !crate::ability::effects::charm_effect::make_choices_precast(
                    game,
                    agents,
                    &mut pt.entry.spell_ability,
                ) {
                    continue; // No valid charm modes — drop the trigger
                }
            }

            // Set up trigger targets via the shared SpellAbility targeting path.
            // This mirrors Java's `prepareSingleSaDeterministic` → `setupTargets()`,
            // which happens BEFORE the trigger is put on the stack.
            // If target setup fails (no valid targets), the trigger is silently
            // dropped — matching Java's behavior.
            if !pt
                .entry
                .spell_ability
                .setup_targets(game, agents, &self.mana_pools)
            {
                continue;
            }

            // For optional triggers, store the decider info on the StackEntry.
            // The actual ACCEPT/DECLINE prompt happens at resolution time
            // (in resolve_stack), matching Java's WrappedAbility.resolve() →
            // confirmTrigger() flow.
            if pt.optional {
                pt.entry.optional_trigger_decider = Some(pt.decider);
                pt.entry.optional_trigger_description = Some(pt.description.clone());
                pt.entry.optional_trigger_source_name = Some(source_name.clone());
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
            let is_optional = pt.optional;
            game.stack.push(pt.entry);
            self.log_stack_push(&source_name, &player_name);
            if std::env::var("FORGE_TRIGGER_TRACE").is_ok() {
                eprintln!("[trigger-trace] PUSHED trigger to stack: {} optional={} api={}", source_name, is_optional, trigger_api);
            }
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
