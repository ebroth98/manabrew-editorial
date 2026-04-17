use std::sync::mpsc;
use std::time::Duration;

use forge_agent_interface::agent_impl::AgentTransport;
use forge_agent_interface::game_log_event::GameLogEntryDto;
use forge_agent_interface::game_snapshot_event::GameSnapshotEventDto;
use forge_agent_interface::prompt::{AgentPrompt, PlayerAction};

enum PromptSink {
    Local(mpsc::Sender<AgentPrompt>),
    Relay {
        player_index: usize,
        tx: mpsc::Sender<(usize, AgentPrompt)>,
    },
    Ai(mpsc::Sender<AgentPrompt>),
}

pub struct MpscTransport {
    prompt_sink: PromptSink,
    response_rx: mpsc::Receiver<PlayerAction>,
    notify_tx: Option<mpsc::Sender<GameLogEntryDto>>,
    snapshot_tx: Option<mpsc::Sender<GameSnapshotEventDto>>,
    response_timeout: Option<Duration>,
}

impl MpscTransport {
    pub fn new_local(
        prompt_tx: mpsc::Sender<AgentPrompt>,
        response_rx: mpsc::Receiver<PlayerAction>,
        notify_tx: mpsc::Sender<GameLogEntryDto>,
        snapshot_tx: mpsc::Sender<GameSnapshotEventDto>,
    ) -> Self {
        Self {
            prompt_sink: PromptSink::Local(prompt_tx),
            response_rx,
            notify_tx: Some(notify_tx),
            snapshot_tx: Some(snapshot_tx),
            response_timeout: None,
        }
    }

    pub fn new_relay(
        player_index: usize,
        prompt_tx: mpsc::Sender<(usize, AgentPrompt)>,
        response_rx: mpsc::Receiver<PlayerAction>,
    ) -> Self {
        Self {
            prompt_sink: PromptSink::Relay {
                player_index,
                tx: prompt_tx,
            },
            response_rx,
            notify_tx: None,
            snapshot_tx: None,
            response_timeout: Some(Duration::from_secs(120)),
        }
    }

    pub fn new_ai(
        prompt_tx: mpsc::Sender<AgentPrompt>,
        response_rx: mpsc::Receiver<PlayerAction>,
    ) -> Self {
        Self {
            prompt_sink: PromptSink::Ai(prompt_tx),
            response_rx,
            notify_tx: None,
            snapshot_tx: None,
            response_timeout: Some(Duration::from_secs(5)),
        }
    }
}

impl AgentTransport for MpscTransport {
    fn send_prompt(&self, prompt: AgentPrompt) {
        match &self.prompt_sink {
            PromptSink::Local(tx) | PromptSink::Ai(tx) => {
                let _ = tx.send(prompt);
            }
            PromptSink::Relay { player_index, tx } => {
                let _ = tx.send((*player_index, prompt));
            }
        }
    }

    fn recv_action(&self) -> PlayerAction {
        if let Some(timeout) = self.response_timeout {
            self.response_rx
                .recv_timeout(timeout)
                .unwrap_or(PlayerAction::PlayCard {
                    card_id: None,
                    mode: None,
                })
        } else {
            self.response_rx.recv().unwrap_or(PlayerAction::PlayCard {
                card_id: None,
                mode: None,
            })
        }
    }

    fn send_log(&self, entry: GameLogEntryDto) {
        if let Some(tx) = &self.notify_tx {
            let _ = tx.send(entry);
        }
    }

    fn send_snapshot(&self, snapshot: GameSnapshotEventDto) {
        if let Some(tx) = &self.snapshot_tx {
            let _ = tx.send(snapshot);
        }
    }

    fn is_human(&self) -> bool {
        !matches!(self.prompt_sink, PromptSink::Ai(_))
    }
}
