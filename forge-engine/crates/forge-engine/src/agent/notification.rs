use crate::agent::GameLogEvent;
use crate::ids::{CardId, PlayerId};
use forge_foundation::PhaseType;

#[derive(Debug, Clone)]
pub enum GameNotification {
    Event(GameLogEvent),
    CardPlayed {
        player: PlayerId,
        card_id: CardId,
        card_name: String,
        set_code: String,
    },
    TurnChanged {
        active_player: PlayerId,
        turn_number: u32,
    },
    PhaseChanged {
        phase: PhaseType,
    },
    PriorityChanged {
        player: PlayerId,
    },
    StateChanged,
    SnapshotCreated {
        checkpoint_id: u64,
        label: String,
    },
}
