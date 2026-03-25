use super::*;

impl GameLoop {
    pub(crate) fn process_triggers(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
    ) {
        let pushed = crate::trigger::process_waiting_triggers(
            &mut self.trigger_handler,
            &self.mana_pools,
            game,
            agents,
        );
        for log in pushed {
            self.log_stack_push(&log.source_name, &log.player_name);
            if std::env::var("FORGE_TRIGGER_TRACE").is_ok() {
                eprintln!(
                    "[trigger-trace] PUSHED trigger to stack: {} optional={} api={}",
                    log.source_name, log.optional, log.trigger_api
                );
            }
        }
    }
}
