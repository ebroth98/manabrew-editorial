//! Decision strategies for a bot. The lifecycle in `BotState` is fixed; the
//! AI plugged into it is not.
//!
//! To add a new agent: define a struct implementing [`BotAgent`], add a
//! variant to [`AgentKind`], and wire it in [`AgentKind::build`]. The room
//! picks which agent to spawn via the `agent` field of the bot config.

use std::sync::mpsc;
use std::thread;

use forge_agent_interface::prompt::{AgentPrompt, PlayerAction};
use serde::{Deserialize, Serialize};

pub mod simple_ai;
pub use simple_ai::SimpleAi;

/// A bot's decision strategy. Each call to `decide` may consult per-bot state
/// (anti-loop memoization, opening-hand plans, …) — hence `&mut self`.
pub trait BotAgent: Send {
    fn decide(&mut self, prompt: AgentPrompt) -> Option<PlayerAction>;
}

/// Wire-level selector for which built-in agent the bot should use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum AgentKind {
    #[default]
    Simple,
}

impl AgentKind {
    pub fn build(self) -> Box<dyn BotAgent + Send> {
        match self {
            AgentKind::Simple => Box::<SimpleAi>::default(),
        }
    }
}

/// Run a `BotAgent` on a dedicated thread, pumping prompts from `prompt_rx`
/// and sending responses to `response_tx`. Used by engine hosts (Tauri,
/// self-hosted-node) to plug an AI into an in-process engine session — the
/// relay-side equivalent is `forge_bot::run_bot`.
pub fn spawn_agent_responder(
    mut agent: Box<dyn BotAgent + Send>,
    prompt_rx: mpsc::Receiver<AgentPrompt>,
    response_tx: mpsc::Sender<PlayerAction>,
) {
    thread::spawn(move || {
        while let Ok(prompt) = prompt_rx.recv() {
            if let Some(action) = agent.decide(prompt) {
                if response_tx.send(action).is_err() {
                    break;
                }
            }
        }
    });
}
