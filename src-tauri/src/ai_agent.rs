use std::sync::mpsc;

use forge_agent_interface::prompt::{AgentPrompt, PlayerAction};

pub fn spawn_ai_prompt_responder(
    prompt_rx: mpsc::Receiver<AgentPrompt>,
    response_tx: mpsc::Sender<PlayerAction>,
) {
    forge_agent_interface::simple_ai::spawn_simple_ai_prompt_responder(prompt_rx, response_tx);
}
