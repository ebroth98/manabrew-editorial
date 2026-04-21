use forge_engine_core::agent::TargetChoice;
use forge_engine_core::ids::{CardId, PlayerId};
use forge_foundation::ZoneType;

use crate::game_view_dto::{CardDto, TargetingIntent};
use crate::ids_codec::card_id_str;
use crate::ids_codec::parse_card_id;
use crate::ids_codec::parse_player_id;
use crate::ids_codec::stack_id_str;
use crate::prompt::{AgentPromptInner, PlayerAction, TargetAnyChoice};

use super::{AgentTransport, PromptAgent};

pub(super) fn choose_target_player<T: AgentTransport>(
    agent: &mut PromptAgent<T>,
    _player: PlayerId,
    valid: &[PlayerId],
    source: Option<CardId>,
    hostile: bool,
    intent: TargetingIntent,
) -> Option<PlayerId> {
    let valid_player_ids = PromptAgent::<T>::player_ids(valid);
    let source_card_id = source.map(card_id_str);
    agent.send_prompt(AgentPromptInner::ChooseTargetPlayer {
        game_view: agent.view(),
        valid_player_ids,
        source_card_id,
        hostile,
        intent,
    });
    agent.recv_player_choice_or_first(valid)
}

pub(super) fn choose_target_card<T: AgentTransport>(
    agent: &mut PromptAgent<T>,
    _player: PlayerId,
    valid: &[CardId],
    source: Option<CardId>,
    hostile: bool,
    intent: TargetingIntent,
) -> Option<CardId> {
    let valid_card_ids = PromptAgent::<T>::card_ids(valid);
    let mut view = agent.view();
    PromptAgent::<T>::mark_battlefield_choosable(&mut view, &valid_card_ids);
    let source_card_id = source.map(card_id_str);
    agent.send_prompt(AgentPromptInner::ChooseTargetCard {
        game_view: view,
        valid_card_ids,
        source_card_id,
        hostile,
        intent,
    });
    agent.recv_card_choice_or_first(valid)
}

pub(super) fn choose_target_card_from_zone<T: AgentTransport>(
    agent: &mut PromptAgent<T>,
    _player: PlayerId,
    zone: ZoneType,
    valid: &[CardId],
    source: Option<CardId>,
    _hostile: bool,
    intent: TargetingIntent,
) -> Option<CardId> {
    let valid_card_ids = PromptAgent::<T>::card_ids(valid);
    let view = agent.view();

    // Build the list of cards in the specified zone
    let zone_cards: Vec<CardDto> = match zone {
        ZoneType::Graveyard => view
            .graveyard
            .iter()
            .filter(|c| valid_card_ids.contains(&c.id))
            .cloned()
            .collect(),
        ZoneType::Exile => view
            .exile
            .iter()
            .filter(|c| valid_card_ids.contains(&c.id))
            .cloned()
            .collect(),
        ZoneType::Hand => view
            .my_hand
            .iter()
            .filter(|c| valid_card_ids.contains(&c.id))
            .cloned()
            .collect(),
        _ => vec![],
    };

    let source_card_id = source.map(card_id_str);
    agent.send_prompt(AgentPromptInner::ChooseTargetCardFromZone {
        game_view: view,
        valid_card_ids,
        zone: format!("{:?}", zone),
        zone_cards,
        source_card_id,
        intent,
    });
    agent.recv_card_choice_or_first(valid)
}

pub(super) fn choose_target_any<T: AgentTransport>(
    agent: &mut PromptAgent<T>,
    _player: PlayerId,
    valid_players: &[PlayerId],
    valid_cards: &[CardId],
    source: Option<CardId>,
    hostile: bool,
    intent: TargetingIntent,
) -> TargetChoice {
    let valid_player_ids = PromptAgent::<T>::player_ids(valid_players);
    let valid_card_ids = PromptAgent::<T>::card_ids(valid_cards);
    let mut view = agent.view();
    PromptAgent::<T>::mark_battlefield_choosable(&mut view, &valid_card_ids);
    let source_card_id = source.map(card_id_str);
    agent.send_prompt(AgentPromptInner::ChooseTargetAny {
        game_view: view,
        valid_player_ids,
        valid_card_ids,
        source_card_id,
        hostile,
        intent,
    });
    match agent.recv_action() {
        PlayerAction::TargetAny { target } => match target {
            TargetAnyChoice::Player { player_id } => parse_player_id(&player_id)
                .map(TargetChoice::Player)
                .unwrap_or(TargetChoice::None),
            TargetAnyChoice::Card { card_id } => parse_card_id(&card_id)
                .map(TargetChoice::Card)
                .unwrap_or(TargetChoice::None),
            TargetAnyChoice::None => TargetChoice::None,
        },
        _ => {
            if let Some(&pid) = valid_players.first() {
                TargetChoice::Player(pid)
            } else if let Some(&cid) = valid_cards.first() {
                TargetChoice::Card(cid)
            } else {
                TargetChoice::None
            }
        }
    }
}

pub(super) fn choose_target_spell<T: AgentTransport>(
    agent: &mut PromptAgent<T>,
    _player: PlayerId,
    valid: &[u32],
    source: Option<CardId>,
) -> Option<u32> {
    let valid_spell_ids: Vec<String> = valid.iter().map(|&id| stack_id_str(id)).collect();
    let source_card_id = source.map(card_id_str);
    agent.send_prompt(AgentPromptInner::ChooseTargetSpell {
        game_view: agent.view(),
        valid_spell_ids,
        source_card_id,
        intent: TargetingIntent::Counter,
    });
    agent.recv_spell_choice_or_first(valid)
}

pub(super) fn choose_sacrifice<T: AgentTransport>(
    agent: &mut PromptAgent<T>,
    _player: PlayerId,
    valid: &[CardId],
    source: Option<CardId>,
) -> Option<CardId> {
    let valid_card_ids = PromptAgent::<T>::card_ids(valid);
    let mut view = agent.view();
    PromptAgent::<T>::mark_battlefield_choosable(&mut view, &valid_card_ids);
    let source_card_id = source.map(card_id_str);
    agent.send_prompt(AgentPromptInner::ChooseTargetCard {
        game_view: view,
        valid_card_ids,
        source_card_id,
        hostile: true,
        intent: TargetingIntent::Sacrifice,
    });
    agent.recv_card_choice_or_first(valid)
}
