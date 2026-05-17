use std::sync::mpsc;

use forge_agent_interface::prompt::{AgentPrompt, PlayerAction};
use forge_bot::agent::{spawn_agent_responder, SimpleAi};

pub fn spawn_ai_prompt_responder(
    prompt_rx: mpsc::Receiver<AgentPrompt>,
    response_tx: mpsc::Sender<PlayerAction>,
) {
    spawn_agent_responder(Box::new(SimpleAi::new()), prompt_rx, response_tx);
}
