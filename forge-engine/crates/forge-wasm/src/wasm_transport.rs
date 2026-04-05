//! WASM transport for game agent communication.
//!
//! Uses SharedArrayBuffer + Atomics.wait() to block the worker thread
//! while waiting for human player input from the main thread.
//!
//! Protocol:
//! - Signal slot (Int32Array index 0):
//!   0 = idle, 1 = prompt ready (worker→main), 2 = response ready (main→worker)
//! - Data length slot (Int32Array index 1): byte length of JSON payload
//! - Data region (Uint8Array offset 8..): JSON payload bytes

use forge_agent_interface::agent_impl::AgentTransport;
use forge_agent_interface::game_log_event::GameLogEntryDto;
use forge_agent_interface::game_snapshot_event::GameSnapshotEventDto;
use forge_agent_interface::prompt::{AgentPrompt, PlayerAction};

use js_sys::{Int32Array, SharedArrayBuffer, Uint8Array};
use wasm_bindgen::prelude::*;

const SIGNAL_IDLE: i32 = 0;
const SIGNAL_PROMPT_READY: i32 = 1;
const SIGNAL_RESPONSE_READY: i32 = 2;
const SIGNAL_PROMPT_ACKNOWLEDGED: i32 = 3;

/// Header size in bytes (2 x i32 = 8 bytes for signal + length).
const HEADER_BYTES: u32 = 8;

/// Default buffer size: 256KB should handle even complex game views.
pub const DEFAULT_BUFFER_SIZE: u32 = 256 * 1024;

/// WASM transport that blocks on Atomics.wait() for human player responses.
pub struct WasmTransport {
    signal: Int32Array,
    data: Uint8Array,
    is_human: bool,
}

impl WasmTransport {
    /// Create a new transport backed by a SharedArrayBuffer.
    ///
    /// The SAB must be at least `HEADER_BYTES` + max prompt/response JSON size.
    /// The same SAB must be shared with the main thread for communication.
    pub fn new(sab: &SharedArrayBuffer, is_human: bool) -> Self {
        let signal = Int32Array::new_with_byte_offset_and_length(
            &JsValue::from(sab.clone()),
            0,
            2, // 2 i32 slots: signal + length
        );
        let data = Uint8Array::new_with_byte_offset_and_length(
            &JsValue::from(sab.clone()),
            HEADER_BYTES,
            sab.byte_length() - HEADER_BYTES,
        );
        Self {
            signal,
            data,
            is_human,
        }
    }

    /// Write JSON bytes into the data region and set the length.
    fn write_data(&self, json_bytes: &[u8]) {
        let len = json_bytes.len() as i32;
        // Set length in slot 1
        js_sys::Atomics::store(&self.signal, 1, len).unwrap_or(0);
        // Copy JSON into data region
        let js_array = Uint8Array::new_with_length(json_bytes.len() as u32);
        js_array.copy_from(json_bytes);
        self.data.set(&js_array, 0);
    }

    /// Read JSON bytes from the data region.
    fn read_data(&self) -> Vec<u8> {
        let len = js_sys::Atomics::load(&self.signal, 1).unwrap_or(0) as usize;
        let mut buf = vec![0u8; len];
        self.data.slice(0, len as u32).copy_to(&mut buf);
        buf
    }
}

impl AgentTransport for WasmTransport {
    fn send_prompt(&self, prompt: AgentPrompt) {
        let json = serde_json::to_vec(&prompt).unwrap_or_default();
        self.write_data(&json);
        js_sys::Atomics::store(&self.signal, 0, SIGNAL_PROMPT_READY).unwrap_or(0);
        js_sys::Atomics::notify(&self.signal, 0).unwrap_or(0);
    }

    fn recv_action(&self) -> PlayerAction {
        loop {
            let current = js_sys::Atomics::load(&self.signal, 0).unwrap_or(0);
            if current == SIGNAL_RESPONSE_READY {
                break;
            }
            let _ = js_sys::Atomics::wait(&self.signal, 0, current);
        }

        let json_bytes = self.read_data();

        js_sys::Atomics::store(&self.signal, 0, SIGNAL_IDLE).unwrap_or(0);

        serde_json::from_slice(&json_bytes).unwrap_or(PlayerAction::PlayCard {
            card_id: None,
            mode: None,
        })
    }

    fn send_log(&self, _entry: GameLogEntryDto) {
        // TODO: Forward log entries to main thread via a separate channel or postMessage
    }

    fn send_snapshot(&self, _snapshot: GameSnapshotEventDto) {
        // TODO: Forward snapshots to main thread
    }

    fn is_human(&self) -> bool {
        self.is_human
    }
}

/// AI transport that auto-responds without blocking.
/// Uses the shared crate's AI responder logic.
pub struct WasmAiTransport;

impl AgentTransport for WasmAiTransport {
    fn send_prompt(&self, prompt: AgentPrompt) {
        PENDING_AI_PROMPT.with(|cell| {
            *cell.borrow_mut() = Some(prompt);
        });
    }

    fn recv_action(&self) -> PlayerAction {
        let prompt = PENDING_AI_PROMPT.with(|cell| cell.borrow_mut().take());
        match prompt {
            Some(p) => ai_respond(&p.inner),
            None => PlayerAction::PlayCard {
                card_id: None,
                mode: None,
            },
        }
    }

    fn send_log(&self, _entry: GameLogEntryDto) {}
    fn send_snapshot(&self, _snapshot: GameSnapshotEventDto) {}

    fn is_human(&self) -> bool {
        false
    }
}

use std::cell::RefCell;

thread_local! {
    static PENDING_AI_PROMPT: RefCell<Option<AgentPrompt>> = const { RefCell::new(None) };
}

/// Simple AI response logic — picks first available action.
/// Mirrors the Tauri ai_agent.rs logic.
fn ai_respond(inner: &forge_agent_interface::prompt::AgentPromptInner) -> PlayerAction {
    use forge_agent_interface::prompt::*;
    use forge_engine_core::player::actions::PlayerAction as EnginePlayerAction;

    match inner {
        AgentPromptInner::Mulligan { .. } => PlayerAction::MulliganDecision { keep: true },
        AgentPromptInner::MulliganPutBack {
            hand_card_ids,
            count,
            ..
        } => PlayerAction::MulliganPutBackDecision {
            card_ids: hand_card_ids.iter().take(*count).cloned().collect(),
        },
        AgentPromptInner::ChooseAction {
            available_player_actions,
            ..
        } => {
            // AI priority: cast spells > pass
            let action = available_player_actions
                .iter()
                .copied()
                .find(|a| matches!(a, EnginePlayerAction::CastSpell(_)))
                .or_else(|| {
                    available_player_actions
                        .iter()
                        .copied()
                        .find(|a| matches!(a, EnginePlayerAction::PassPriority))
                })
                .or_else(|| available_player_actions.first().copied());
            action
                .map(|a| PlayerAction::EngineAction { action: a })
                .unwrap_or(PlayerAction::PlayCard {
                    card_id: None,
                    mode: None,
                })
        },
        AgentPromptInner::ChooseAttackers {
            available_attacker_ids,
            possible_defender_ids,
            ..
        } => {
            let default_defender = possible_defender_ids
                .first()
                .map(|d| d.id.clone())
                .unwrap_or_else(|| "player-1".to_string());
            PlayerAction::DeclareAttackers {
                assignments: available_attacker_ids
                    .iter()
                    .map(|id| AttackAssignment {
                        attacker_id: id.clone(),
                        defender_id: default_defender.clone(),
                    })
                    .collect(),
            }
        }
        AgentPromptInner::ChooseBlockers {
            attacker_ids,
            available_blocker_ids,
            ..
        } => {
            let assignments =
                if !attacker_ids.is_empty() && !available_blocker_ids.is_empty() {
                    vec![BlockAssignment {
                        blocker_id: available_blocker_ids[0].clone(),
                        attacker_id: attacker_ids[0].clone(),
                    }]
                } else {
                    Vec::new()
                };
            PlayerAction::DeclareBlockers { assignments }
        }
        AgentPromptInner::ChooseTargetPlayer {
            valid_player_ids, ..
        } => PlayerAction::TargetPlayer {
            player_id: valid_player_ids.first().cloned(),
        },
        AgentPromptInner::ChooseTargetCard { valid_card_ids, .. }
        | AgentPromptInner::ChooseTargetCardFromZone { valid_card_ids, .. } => {
            PlayerAction::TargetCard {
                card_id: valid_card_ids.first().cloned(),
            }
        }
        AgentPromptInner::ChooseTargetAny {
            valid_player_ids,
            valid_card_ids,
            ..
        } => {
            let target = if let Some(card_id) = valid_card_ids.first() {
                TargetAnyChoice::Card {
                    card_id: card_id.clone(),
                }
            } else if let Some(player_id) = valid_player_ids.first() {
                TargetAnyChoice::Player {
                    player_id: player_id.clone(),
                }
            } else {
                TargetAnyChoice::None
            };
            PlayerAction::TargetAny { target }
        }
        AgentPromptInner::Scry { .. } => PlayerAction::ScryDecision {
            bottom_card_ids: Vec::new(),
        },
        AgentPromptInner::Surveil { .. } => PlayerAction::SurveilDecision {
            graveyard_card_ids: Vec::new(),
        },
        AgentPromptInner::Dig {
            card_ids,
            num_to_take,
            ..
        } => PlayerAction::DigDecision {
            chosen_card_ids: card_ids.iter().take(*num_to_take).cloned().collect(),
        },
        AgentPromptInner::ChooseDiscard {
            hand_card_ids,
            num_to_discard,
            ..
        } => PlayerAction::DiscardDecision {
            discarded_card_ids: hand_card_ids.iter().take(*num_to_discard).cloned().collect(),
        },
        AgentPromptInner::ChooseTargetSpell {
            valid_spell_ids, ..
        } => PlayerAction::TargetSpell {
            spell_id: valid_spell_ids.first().cloned(),
        },
        AgentPromptInner::ChooseMode {
            options,
            min_choices,
            ..
        } => PlayerAction::ModeDecision {
            chosen_indices: (0..*min_choices.min(&options.len())).collect(),
        },
        AgentPromptInner::ChooseOptionalTrigger { .. } => {
            PlayerAction::OptionalTriggerDecision { accept: true }
        }
        AgentPromptInner::ChoosePhyrexian { .. } => {
            PlayerAction::PhyrexianDecision { pay_life: false }
        }
        AgentPromptInner::ChooseKicker { .. } => {
            PlayerAction::KickerDecision { kicked: false }
        }
        AgentPromptInner::ChooseBuyback { .. } => PlayerAction::BuybackDecision {
            buyback_paid: false,
        },
        AgentPromptInner::ChooseMultikicker { .. } => {
            PlayerAction::MultikickerDecision { kick_count: 0 }
        }
        AgentPromptInner::ChooseReplicate { .. } => {
            PlayerAction::ReplicateDecision {
                replicate_count: 0,
            }
        }
        AgentPromptInner::ChooseAlternativeCost { .. } => {
            PlayerAction::AlternativeCostDecision { chosen_index: 0 }
        }
        AgentPromptInner::ChooseColor { valid_colors, .. } => PlayerAction::ColorDecision {
            color: valid_colors.first().cloned(),
        },
        AgentPromptInner::ChooseType { valid_types, .. } => PlayerAction::TypeDecision {
            chosen_type: valid_types.first().cloned(),
        },
        AgentPromptInner::ChooseNumber { min, .. } => PlayerAction::NumberDecision {
            chosen_number: Some(*min),
        },
        AgentPromptInner::ChooseCardName { valid_names, .. } => PlayerAction::CardNameDecision {
            chosen_name: valid_names.first().cloned(),
        },
        AgentPromptInner::ChooseCardsForEffect {
            valid_card_ids,
            max_choices,
            ..
        } => PlayerAction::ChooseCardsDecision {
            chosen_card_ids: valid_card_ids.iter().take(*max_choices).cloned().collect(),
        },
        AgentPromptInner::ChooseDamageAssignmentOrder { blocker_ids, .. } => {
            PlayerAction::DamageAssignmentOrderDecision {
                ordered_blocker_ids: blocker_ids.clone(),
            }
        }
        AgentPromptInner::ChooseCombatDamageAssignment {
            blocker_ids,
            total_damage,
            ..
        } => {
            let mut assignments = Vec::new();
            if let Some(first) = blocker_ids.first() {
                assignments.push(CombatDamageAssignmentEntry {
                    assignee_id: first.clone(),
                    damage: (*total_damage).max(0),
                });
            }
            PlayerAction::CombatDamageAssignmentDecision { assignments }
        }
        AgentPromptInner::PayCombatCost {
            tappable_land_ids,
            mana_pool_total,
            cost,
            ..
        } => {
            if *mana_pool_total >= *cost {
                PlayerAction::PayCombatCost
            } else if !tappable_land_ids.is_empty() {
                PlayerAction::TapLand {
                    card_id: tappable_land_ids[0].clone(),
                    ability_index: None,
                }
            } else {
                PlayerAction::DeclineCombatCost
            }
        }
        AgentPromptInner::PayManaCost {
            game_view,
            mana_cost,
            tappable_land_ids,
            mana_ability_options,
            ..
        } => forge_agent_interface::auto_pay::choose_pay_mana_cost_action(
            game_view,
            mana_cost,
            tappable_land_ids,
            mana_ability_options,
        )
        .unwrap_or(PlayerAction::CancelManaCost),
        AgentPromptInner::ChooseDelve {
            valid_card_ids,
            max_cards,
            ..
        } => PlayerAction::DelveDecision {
            chosen_card_ids: valid_card_ids.iter().take(*max_cards).cloned().collect(),
        },
        AgentPromptInner::ChooseConvoke { .. } => PlayerAction::ConvokeDecision {
            chosen_card_ids: vec![],
        },
        AgentPromptInner::ChooseImprovise { .. } => PlayerAction::ImproviseDecision {
            chosen_card_ids: vec![],
        },
        AgentPromptInner::SpecifyManaCombo {
            available_colors,
            amount,
            ..
        } => {
            let color = available_colors
                .first()
                .cloned()
                .unwrap_or_else(|| "C".to_string());
            PlayerAction::ManaComboDecision {
                chosen_colors: vec![color; *amount],
            }
        }
        AgentPromptInner::ChooseExertAttackers { .. } => PlayerAction::ExertDecision {
            chosen_attacker_ids: vec![],
        },
        AgentPromptInner::ChooseEnlistAttackers { .. } => PlayerAction::EnlistDecision {
            chosen_attacker_ids: vec![],
        },
        AgentPromptInner::ReorderLibrary { card_ids, .. } => {
            PlayerAction::ReorderLibraryDecision {
                ordered_card_ids: card_ids.clone(),
            }
        }
        AgentPromptInner::ExploreDecision { .. } => PlayerAction::ExploreResponse {
            put_in_graveyard: false,
        },
        AgentPromptInner::HelpPayAssist { .. } => {
            PlayerAction::AssistDecision { amount_to_pay: 0 }
        }
        AgentPromptInner::StateUpdate { .. } | AgentPromptInner::GameOver { .. } => {
            // No action needed for display-only prompts
            PlayerAction::PlayCard {
                card_id: None,
                mode: None,
            }
        }
    }
}
