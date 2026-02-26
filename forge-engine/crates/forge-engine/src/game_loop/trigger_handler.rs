use super::*;

impl GameLoop {
    pub(crate) fn process_triggers(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
    ) {
        let pending = self.trigger_handler.run_waiting_triggers(game);
        for pt in pending {
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
                // Prompt the deciding player
                let accepted = agents[pt.decider.index()].choose_optional_trigger(
                    pt.decider,
                    &pt.description,
                    Some(&source_name),
                );
                if !accepted {
                    continue; // Player declined the optional trigger
                }
            }
            game.stack.push(pt.entry);
            self.log_stack_push(&source_name, &player_name);
        }
    }
}
