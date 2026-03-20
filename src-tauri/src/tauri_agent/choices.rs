use forge_engine_core::agent::BinaryChoiceKind;
use forge_engine_core::ids::{CardId, PlayerId};

use crate::game_view_dto::CardDto;
use crate::ids_codec::parse_card_id;
use crate::prompt::{AgentPromptInner, PlayerAction};

use super::TauriAgent;

pub(super) fn mulligan_decision(
    agent: &mut TauriAgent,
    _player: PlayerId,
    hand: &[CardId],
    mulligan_count: u32,
) -> bool {
    let hand_card_ids = TauriAgent::card_ids(hand);
    agent.send_prompt(AgentPromptInner::Mulligan {
        game_view: agent.view(),
        hand_card_ids,
        mulligan_count,
    });
    match agent.recv_action() {
        PlayerAction::MulliganDecision { keep } => keep,
        _ => true,
    }
}

pub(super) fn choose_cards_to_bottom(
    agent: &mut TauriAgent,
    _player: PlayerId,
    hand: &[CardId],
    count: usize,
) -> Vec<CardId> {
    let view = agent.view();
    let hand_card_ids = TauriAgent::card_ids(hand);
    let cards: Vec<CardDto> = hand
        .iter()
        .filter_map(|&cid| {
            let id_str = crate::ids_codec::card_id_str(cid);
            view.my_hand.iter().find(|c| c.id == id_str).cloned()
        })
        .collect();
    agent.send_prompt(AgentPromptInner::MulliganPutBack {
        game_view: view,
        hand_card_ids,
        cards,
        count,
    });
    match agent.recv_action() {
        PlayerAction::MulliganPutBackDecision { card_ids } => {
            card_ids.iter().filter_map(|s| parse_card_id(s)).collect()
        }
        _ => hand.iter().copied().take(count).collect(),
    }
}

pub(super) fn choose_mode(
    agent: &mut TauriAgent,
    _player: PlayerId,
    descriptions: &[String],
    min: usize,
    max: usize,
    card_name: Option<&str>,
) -> Vec<usize> {
    agent.send_prompt(AgentPromptInner::ChooseMode {
        game_view: agent.view(),
        options: descriptions.to_vec(),
        min_choices: min,
        max_choices: max,
        source_card_name: card_name.map(String::from),
    });
    match agent.recv_action() {
        PlayerAction::ModeDecision { chosen_indices } => chosen_indices,
        _ => (0..min.min(descriptions.len())).collect(),
    }
}

pub(super) fn choose_optional_trigger(
    agent: &mut TauriAgent,
    _player: PlayerId,
    description: &str,
    card_name: Option<&str>,
    _api: Option<&str>,
) -> bool {
    agent.send_prompt(AgentPromptInner::ChooseOptionalTrigger {
        game_view: agent.view(),
        description: description.to_string(),
        source_card_name: card_name.map(String::from),
        prompt_kind: Some("optional_trigger".to_string()),
        option_labels: Some(vec!["Decline".to_string(), "Accept".to_string()]),
        mode: None,
        api: None,
    });
    match agent.recv_action() {
        PlayerAction::OptionalTriggerDecision { accept } => accept,
        _ => true,
    }
}

pub(super) fn confirm_action(
    agent: &mut TauriAgent,
    _player: PlayerId,
    mode: Option<&str>,
    message: &str,
    options: &[String],
    card_name: Option<&str>,
    api: Option<&str>,
) -> bool {
    // Reuse the existing optional-trigger modal plumbing for generic confirms.
    let option_labels = if options.is_empty() {
        vec!["Decline".to_string(), "Accept".to_string()]
    } else {
        options.to_vec()
    };
    agent.send_prompt(AgentPromptInner::ChooseOptionalTrigger {
        game_view: agent.view(),
        description: message.to_string(),
        source_card_name: card_name.map(String::from),
        prompt_kind: Some("confirm_action".to_string()),
        option_labels: Some(option_labels),
        mode: mode.map(String::from),
        api: api.map(String::from),
    });
    match agent.recv_action() {
        PlayerAction::OptionalTriggerDecision { accept } => accept,
        _ => false,
    }
}

pub(super) fn confirm_payment(
    agent: &mut TauriAgent,
    _player: PlayerId,
    cost_kind: &str,
    message: &str,
    card_name: Option<&str>,
    api: Option<&str>,
) -> bool {
    agent.send_prompt(AgentPromptInner::ChooseOptionalTrigger {
        game_view: agent.view(),
        description: message.to_string(),
        source_card_name: card_name.map(String::from),
        prompt_kind: Some("confirm_payment".to_string()),
        option_labels: Some(vec!["Decline".to_string(), "Accept".to_string()]),
        mode: Some(cost_kind.to_string()),
        api: api.map(String::from),
    });
    match agent.recv_action() {
        PlayerAction::OptionalTriggerDecision { accept } => accept,
        _ => false,
    }
}

pub(super) fn choose_binary(
    agent: &mut TauriAgent,
    _player: PlayerId,
    question: &str,
    kind: BinaryChoiceKind,
    _default_choice: Option<bool>,
    card_name: Option<&str>,
    api: Option<&str>,
) -> bool {
    let (left, right) = kind.labels();
    // In this modal pipeline, `accept=true` means "second button";
    // reverse labels so `true` still maps to Java's first (left) choice.
    agent.send_prompt(AgentPromptInner::ChooseOptionalTrigger {
        game_view: agent.view(),
        description: question.to_string(),
        source_card_name: card_name.map(String::from),
        prompt_kind: Some("choose_binary".to_string()),
        option_labels: Some(vec![right.to_string(), left.to_string()]),
        mode: Some(kind.as_str().to_string()),
        api: api.map(String::from),
    });
    match agent.recv_action() {
        PlayerAction::OptionalTriggerDecision { accept } => accept,
        _ => false,
    }
}

pub(super) fn choose_color(
    agent: &mut TauriAgent,
    _player: PlayerId,
    valid_colors: &[String],
) -> Option<String> {
    agent.send_prompt(AgentPromptInner::ChooseColor {
        game_view: agent.view(),
        valid_colors: valid_colors.to_vec(),
        source_card_name: None,
    });
    match agent.recv_action() {
        PlayerAction::ColorDecision { color } => color,
        _ => valid_colors.first().cloned(),
    }
}

pub(super) fn choose_type(
    agent: &mut TauriAgent,
    _player: PlayerId,
    type_category: &str,
    valid_types: &[String],
) -> Option<String> {
    agent.send_prompt(AgentPromptInner::ChooseType {
        game_view: agent.view(),
        type_category: type_category.to_string(),
        valid_types: valid_types.to_vec(),
        source_card_name: None,
    });
    match agent.recv_action() {
        PlayerAction::TypeDecision { chosen_type } => chosen_type,
        _ => valid_types.first().cloned(),
    }
}

pub(super) fn choose_card_name(
    agent: &mut TauriAgent,
    _player: PlayerId,
    valid_names: &[String],
) -> Option<String> {
    agent.send_prompt(AgentPromptInner::ChooseCardName {
        game_view: agent.view(),
        valid_names: valid_names.to_vec(),
        source_card_name: None,
    });
    match agent.recv_action() {
        PlayerAction::CardNameDecision { chosen_name } => chosen_name,
        _ => valid_names.first().cloned(),
    }
}

pub(super) fn choose_x_value(
    agent: &mut TauriAgent,
    _player: PlayerId,
    max_x: u32,
    card_name: Option<&str>,
) -> u32 {
    agent.send_prompt(AgentPromptInner::ChooseNumber {
        game_view: agent.view(),
        min: 0,
        max: max_x as i32,
        source_card_name: card_name.map(String::from),
    });
    match agent.recv_action() {
        PlayerAction::NumberDecision { chosen_number } => {
            chosen_number.unwrap_or(max_x as i32).max(0) as u32
        }
        _ => max_x,
    }
}

pub(super) fn choose_number(
    agent: &mut TauriAgent,
    _player: PlayerId,
    min: i32,
    max: i32,
) -> Option<i32> {
    agent.send_prompt(AgentPromptInner::ChooseNumber {
        game_view: agent.view(),
        min,
        max,
        source_card_name: None,
    });
    match agent.recv_action() {
        PlayerAction::NumberDecision { chosen_number } => chosen_number,
        _ => Some(min),
    }
}

pub(super) fn choose_discard(
    agent: &mut TauriAgent,
    _player: PlayerId,
    hand: &[CardId],
    num: usize,
) -> Vec<CardId> {
    let hand_card_ids = TauriAgent::card_ids(hand);
    agent.send_prompt(AgentPromptInner::ChooseDiscard {
        game_view: agent.view(),
        hand_card_ids,
        num_to_discard: num,
    });
    match agent.recv_action() {
        PlayerAction::DiscardDecision { discarded_card_ids } => discarded_card_ids
            .iter()
            .filter_map(|id| parse_card_id(id))
            .collect(),
        _ => hand.iter().copied().take(num).collect(),
    }
}

pub(super) fn choose_cards_for_effect(
    agent: &mut TauriAgent,
    _player: PlayerId,
    valid: &[CardId],
    min: usize,
    max: usize,
) -> Vec<CardId> {
    let valid_card_ids = TauriAgent::card_ids(valid);
    let view = agent.view();

    // Build zone_cards from the snapshot view's zones (find matching DTOs)
    let all_cards: Vec<&CardDto> = view
        .battlefield
        .iter()
        .chain(view.my_hand.iter())
        .chain(view.graveyard.iter())
        .chain(view.exile.iter())
        .chain(view.opponent_graveyard.iter())
        .chain(view.opponent_exile.iter())
        .collect();
    let zone_cards: Vec<CardDto> = valid_card_ids
        .iter()
        .filter_map(|id| all_cards.iter().find(|c| c.id == *id).map(|c| (*c).clone()))
        .collect();

    agent.send_prompt(AgentPromptInner::ChooseCardsForEffect {
        game_view: view,
        valid_card_ids,
        zone_cards,
        min_choices: min,
        max_choices: max,
        source_card_name: None,
    });
    match agent.recv_action() {
        PlayerAction::ChooseCardsDecision { chosen_card_ids } => chosen_card_ids
            .iter()
            .filter_map(|id| parse_card_id(id))
            .collect(),
        _ => valid.iter().copied().take(max).collect(),
    }
}

pub(super) fn choose_single_card_for_zone_change(
    agent: &mut TauriAgent,
    _player: PlayerId,
    valid: &[CardId],
    select_prompt: &str,
    is_optional: bool,
) -> Option<CardId> {
    let valid_card_ids = TauriAgent::card_ids(valid);
    let view = agent.view();

    // Build zone_cards from all known zones + peeked library cards
    let peeked = std::mem::take(&mut agent.peeked_library_cards);
    let all_cards: Vec<&CardDto> = view
        .battlefield
        .iter()
        .chain(view.my_hand.iter())
        .chain(view.graveyard.iter())
        .chain(view.exile.iter())
        .chain(view.opponent_graveyard.iter())
        .chain(view.opponent_exile.iter())
        .chain(view.my_command_zone.iter())
        .collect();
    let mut zone_cards: Vec<CardDto> = valid_card_ids
        .iter()
        .filter_map(|id| {
            all_cards
                .iter()
                .find(|c| c.id == *id)
                .map(|c| (*c).clone())
                .or_else(|| peeked.iter().find(|c| c.id == *id).cloned())
        })
        .collect();
    // Deduplicate
    let mut seen = std::collections::HashSet::new();
    zone_cards.retain(|c| seen.insert(c.id.clone()));

    let min_choices = if is_optional { 0 } else { 1 };
    agent.send_prompt(AgentPromptInner::ChooseCardsForEffect {
        game_view: view,
        valid_card_ids,
        zone_cards,
        min_choices,
        max_choices: 1,
        source_card_name: Some(select_prompt.to_string()),
    });
    match agent.recv_action() {
        PlayerAction::ChooseCardsDecision { chosen_card_ids } => {
            chosen_card_ids.first().and_then(|id| parse_card_id(id))
        }
        _ => {
            if is_optional {
                None
            } else {
                valid.first().copied()
            }
        }
    }
}

pub(super) fn choose_cards_for_zone_change(
    agent: &mut TauriAgent,
    _player: PlayerId,
    valid: &[CardId],
    min: usize,
    max: usize,
    select_prompt: &str,
) -> Vec<CardId> {
    let valid_card_ids = TauriAgent::card_ids(valid);
    let view = agent.view();

    // Build zone_cards from all known zones + peeked library cards
    let peeked = std::mem::take(&mut agent.peeked_library_cards);
    let all_cards: Vec<&CardDto> = view
        .battlefield
        .iter()
        .chain(view.my_hand.iter())
        .chain(view.graveyard.iter())
        .chain(view.exile.iter())
        .chain(view.opponent_graveyard.iter())
        .chain(view.opponent_exile.iter())
        .chain(view.my_command_zone.iter())
        .collect();
    let mut zone_cards: Vec<CardDto> = valid_card_ids
        .iter()
        .filter_map(|id| {
            all_cards
                .iter()
                .find(|c| c.id == *id)
                .map(|c| (*c).clone())
                .or_else(|| peeked.iter().find(|c| c.id == *id).cloned())
        })
        .collect();
    let mut seen = std::collections::HashSet::new();
    zone_cards.retain(|c| seen.insert(c.id.clone()));

    agent.send_prompt(AgentPromptInner::ChooseCardsForEffect {
        game_view: view,
        valid_card_ids,
        zone_cards,
        min_choices: min,
        max_choices: max,
        source_card_name: Some(select_prompt.to_string()),
    });
    match agent.recv_action() {
        PlayerAction::ChooseCardsDecision { chosen_card_ids } => chosen_card_ids
            .iter()
            .filter_map(|id| parse_card_id(id))
            .collect(),
        _ => valid.iter().copied().take(max).collect(),
    }
}

pub(super) fn choose_explore_put_in_graveyard(
    agent: &mut TauriAgent,
    _player: PlayerId,
    revealed_card_name: &str,
    _revealed_cmc: i32,
    _mana_producing_lands: usize,
    _predicted_mana: usize,
    _lands_in_hand: usize,
) -> bool {
    let peeked = std::mem::take(&mut agent.peeked_library_cards);
    let revealed_card = peeked.into_iter().next();
    agent.send_prompt(AgentPromptInner::ExploreDecision {
        game_view: agent.view(),
        revealed_card_name: revealed_card_name.to_string(),
        revealed_card,
        source_card_name: None,
    });
    match agent.recv_action() {
        PlayerAction::ExploreResponse { put_in_graveyard } => put_in_graveyard,
        _ => false,
    }
}

pub(super) fn help_pay_assist(
    agent: &mut TauriAgent,
    _player: PlayerId,
    card_name: &str,
    max_generic: u32,
) -> u32 {
    agent.send_prompt(AgentPromptInner::HelpPayAssist {
        game_view: agent.view(),
        card_name: card_name.to_string(),
        max_generic,
    });
    match agent.recv_action() {
        PlayerAction::AssistDecision { amount_to_pay } => amount_to_pay.min(max_generic),
        _ => 0,
    }
}

pub(super) fn choose_random_discard(
    _agent: &mut TauriAgent,
    _player: PlayerId,
    hand: &[CardId],
    num: usize,
) -> Vec<CardId> {
    use rand::seq::SliceRandom;
    let mut v = hand.to_vec();
    v.shuffle(&mut rand::thread_rng());
    v.truncate(num);
    v
}

pub(super) fn choose_land_or_spell(_agent: &mut TauriAgent, _player: PlayerId) -> Option<bool> {
    None
}

/// Choose which replacement effect to apply when multiple effects match.
/// Reuses the ChooseMode prompt — structurally identical (pick one from a list).
pub(super) fn choose_single_replacement_effect(
    agent: &mut TauriAgent,
    _player: PlayerId,
    descriptions: &[String],
) -> usize {
    if descriptions.len() <= 1 {
        return 0;
    }
    agent.send_prompt(AgentPromptInner::ChooseMode {
        game_view: agent.view(),
        options: descriptions.to_vec(),
        min_choices: 1,
        max_choices: 1,
        source_card_name: Some("Replacement Effect".to_string()),
    });
    match agent.recv_action() {
        PlayerAction::ModeDecision { chosen_indices } => {
            chosen_indices.first().copied().unwrap_or(0)
        }
        _ => 0,
    }
}
