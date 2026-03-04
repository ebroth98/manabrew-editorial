use std::sync::mpsc;
use std::thread;

use crate::prompt::{
    AgentPrompt, AgentPromptInner, BlockAssignment, PlayerAction, TargetAnyChoice,
};

pub fn spawn_ai_prompt_responder(
    prompt_rx: mpsc::Receiver<AgentPrompt>,
    response_tx: mpsc::Sender<PlayerAction>,
) {
    thread::spawn(move || {
        while let Ok(prompt) = prompt_rx.recv() {
            let maybe_action = match prompt.inner {
                AgentPromptInner::Mulligan { .. } => {
                    Some(PlayerAction::MulliganDecision { keep: true })
                }
                AgentPromptInner::MulliganPutBack {
                    hand_card_ids,
                    count,
                    ..
                } => Some(PlayerAction::MulliganPutBackDecision {
                    card_ids: hand_card_ids.into_iter().take(count).collect(),
                }),
                AgentPromptInner::ChooseAction {
                    playable_card_ids, ..
                } => Some(PlayerAction::PlayCard {
                    card_id: playable_card_ids.first().cloned(),
                }),
                AgentPromptInner::ChooseAttackers {
                    available_attacker_ids,
                    possible_defender_ids,
                    ..
                } => {
                    let default_defender = possible_defender_ids
                        .first()
                        .map(|d| d.id.clone())
                        .unwrap_or_else(|| "player-1".to_string());
                    Some(PlayerAction::DeclareAttackers {
                        assignments: available_attacker_ids
                            .into_iter()
                            .map(|id| crate::prompt::AttackAssignment {
                                attacker_id: id,
                                defender_id: default_defender.clone(),
                            })
                            .collect(),
                    })
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
                    Some(PlayerAction::DeclareBlockers { assignments })
                }
                AgentPromptInner::ChooseTargetPlayer {
                    valid_player_ids, ..
                } => Some(PlayerAction::TargetPlayer {
                    player_id: valid_player_ids.first().cloned(),
                }),
                AgentPromptInner::ChooseTargetCard { valid_card_ids, .. }
                | AgentPromptInner::ChooseTargetCardFromZone { valid_card_ids, .. } => {
                    Some(PlayerAction::TargetCard {
                        card_id: valid_card_ids.first().cloned(),
                    })
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
                    Some(PlayerAction::TargetAny { target })
                }
                AgentPromptInner::Scry { .. } => Some(PlayerAction::ScryDecision {
                    bottom_card_ids: Vec::new(),
                }),
                AgentPromptInner::Surveil { .. } => Some(PlayerAction::SurveilDecision {
                    graveyard_card_ids: Vec::new(),
                }),
                AgentPromptInner::Dig {
                    card_ids,
                    num_to_take,
                    ..
                } => Some(PlayerAction::DigDecision {
                    chosen_card_ids: card_ids.into_iter().take(num_to_take).collect(),
                }),
                AgentPromptInner::ChooseDiscard {
                    hand_card_ids,
                    num_to_discard,
                    ..
                } => Some(PlayerAction::DiscardDecision {
                    discarded_card_ids: hand_card_ids.into_iter().take(num_to_discard).collect(),
                }),
                AgentPromptInner::ChooseTargetSpell {
                    valid_spell_ids, ..
                } => Some(PlayerAction::TargetSpell {
                    spell_id: valid_spell_ids.first().cloned(),
                }),
                AgentPromptInner::ChooseMode {
                    options,
                    min_choices,
                    ..
                } => Some(PlayerAction::ModeDecision {
                    chosen_indices: (0..min_choices.min(options.len())).collect(),
                }),
                AgentPromptInner::ChooseOptionalTrigger { .. } => {
                    Some(PlayerAction::OptionalTriggerDecision { accept: true })
                }
                AgentPromptInner::ChooseKicker { .. } => {
                    Some(PlayerAction::KickerDecision { kicked: false })
                }
                AgentPromptInner::ChooseBuyback { .. } => Some(PlayerAction::BuybackDecision {
                    buyback_paid: false,
                }),
                AgentPromptInner::ChooseMultikicker { .. } => {
                    Some(PlayerAction::MultikickerDecision { kick_count: 0 })
                }
                AgentPromptInner::ChooseReplicate { .. } => {
                    Some(PlayerAction::ReplicateDecision { replicate_count: 0 })
                }
                AgentPromptInner::ChooseAlternativeCost { .. } => {
                    Some(PlayerAction::AlternativeCostDecision { chosen_index: 0 })
                }
                AgentPromptInner::ChooseColor { valid_colors, .. } => {
                    Some(PlayerAction::ColorDecision {
                        color: valid_colors.first().cloned(),
                    })
                }
                AgentPromptInner::ChooseType { valid_types, .. } => {
                    Some(PlayerAction::TypeDecision {
                        chosen_type: valid_types.first().cloned(),
                    })
                }
                AgentPromptInner::ChooseNumber { min, .. } => Some(PlayerAction::NumberDecision {
                    chosen_number: Some(min),
                }),
                AgentPromptInner::ChooseCardName { valid_names, .. } => {
                    Some(PlayerAction::CardNameDecision {
                        chosen_name: valid_names.first().cloned(),
                    })
                }
                AgentPromptInner::ChooseCardsForEffect {
                    valid_card_ids,
                    max_choices,
                    ..
                } => Some(PlayerAction::ChooseCardsDecision {
                    chosen_card_ids: valid_card_ids.into_iter().take(max_choices).collect(),
                }),
                AgentPromptInner::ChooseDamageAssignmentOrder { blocker_ids, .. } => {
                    Some(PlayerAction::DamageAssignmentOrderDecision {
                        ordered_blocker_ids: blocker_ids,
                    })
                }
                AgentPromptInner::PayCombatCost {
                    tappable_land_ids,
                    mana_pool_total,
                    cost,
                    ..
                } => {
                    if mana_pool_total >= cost {
                        Some(PlayerAction::PayCombatCost)
                    } else if !tappable_land_ids.is_empty() {
                        Some(PlayerAction::TapLand {
                            card_id: tappable_land_ids[0].clone(),
                        })
                    } else {
                        Some(PlayerAction::DeclineCombatCost)
                    }
                }
                AgentPromptInner::StateUpdate { .. } | AgentPromptInner::GameOver { .. } => None,
            };

            if let Some(action) = maybe_action {
                if response_tx.send(action).is_err() {
                    break;
                }
            }
        }
    });
}
