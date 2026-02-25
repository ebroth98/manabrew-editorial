use std::sync::mpsc;
use std::time::Duration;

use forge_engine_core::agent::{MainPhaseAction, PlayerAgent, TargetChoice};
use forge_engine_core::game::GameState;
use forge_engine_core::ids::{CardId, PlayerId};
use forge_engine_core::mana::ManaPool;
use forge_foundation::{PhaseType, ZoneType};

use crate::game_view_dto::{card_to_dto, CardDto, GameViewDto};
use crate::prompt::{
    ActivatableAbilityInfo, AgentPrompt, AgentPromptInner, BlockAssignment, DisplayEvent,
    PlayerAction, TargetAnyChoice,
};

const RESPONSE_TIMEOUT: Duration = Duration::from_secs(120);

/// Parse "stack-42" → 42u32 (stack entry ID).
fn parse_spell_id(s: &str) -> Option<u32> {
    s.strip_prefix("stack-").and_then(|n| n.parse::<u32>().ok())
}

/// A PlayerAgent that sends prompts to a remote player via the server relay
/// and blocks waiting for their response.
///
/// One instance is created per non-host player in a multiplayer game.
pub struct RemotePlayerAgent {
    pub player_id: PlayerId,
    pub player_index: usize,
    pub game_id: String,
    /// Sends (player_index, prompt) to the remote prompt forwarder thread.
    pub prompt_tx: mpsc::Sender<(usize, AgentPrompt)>,
    /// Receives responses from route_remote_response() when the remote player acts.
    pub response_rx: mpsc::Receiver<PlayerAction>,
    latest_view: Option<GameViewDto>,
    pending_display_events: Vec<DisplayEvent>,
    peeked_library_cards: Vec<CardDto>,
    /// Cached per-ability descriptions and is_mana_ability flags, populated in snapshot_state.
    ability_descriptions: std::collections::HashMap<(u32, usize), (String, bool)>,
}

impl RemotePlayerAgent {
    pub fn new(
        player_id: PlayerId,
        player_index: usize,
        game_id: String,
        prompt_tx: mpsc::Sender<(usize, AgentPrompt)>,
        response_rx: mpsc::Receiver<PlayerAction>,
    ) -> Self {
        Self {
            player_id,
            player_index,
            game_id,
            prompt_tx,
            response_rx,
            latest_view: None,
            pending_display_events: Vec::new(),
            peeked_library_cards: Vec::new(),
            ability_descriptions: std::collections::HashMap::new(),
        }
    }

    fn send_prompt(&mut self, inner: AgentPromptInner) {
        let display_events = std::mem::take(&mut self.pending_display_events);
        let _ = self.prompt_tx.send((
            self.player_index,
            AgentPrompt {
                display_events,
                inner,
            },
        ));
    }

    fn recv_action(&self) -> PlayerAction {
        self.response_rx
            .recv_timeout(RESPONSE_TIMEOUT)
            .unwrap_or(PlayerAction::PlayCard { card_id: None })
    }

    fn view(&self) -> GameViewDto {
        self.latest_view.clone().unwrap_or_else(|| GameViewDto {
            game_id: self.game_id.clone(),
            turn: 0,
            step: "main1".into(),
            combat_assignments: vec![],
            active_player_id: String::new(),
            priority_player_id: String::new(),
            players: vec![],
            my_hand: vec![],
            battlefield: vec![],
            stack: vec![],
            exile: vec![],
            graveyard: vec![],
            opponent_graveyard: vec![],
            opponent_exile: vec![],
            my_command_zone: vec![],
            opponent_command_zone: vec![],
            game_over: false,
            winner_id: None,
            monarch_id: None,
            initiative_holder_id: None,
        })
    }

    fn parse_card_id(s: &str) -> Option<CardId> {
        s.strip_prefix("card-")
            .and_then(|n| n.parse::<u32>().ok())
            .map(CardId)
    }

    fn parse_player_id(s: &str) -> Option<PlayerId> {
        s.strip_prefix("player-")
            .and_then(|n| n.parse::<u32>().ok())
            .map(PlayerId)
    }
}

impl PlayerAgent for RemotePlayerAgent {
    fn snapshot_state(&mut self, game: &GameState, mana_pools: &[ManaPool]) {
        self.latest_view = Some(GameViewDto::from_engine(
            game,
            mana_pools,
            self.player_id,
            &self.game_id,
            &[],
            &[],
        ));

        // Cache per-ability descriptions from battlefield cards
        self.ability_descriptions.clear();
        let battlefield = game.cards_in_zone(forge_foundation::ZoneType::Battlefield, self.player_id);
        for &card_id in battlefield {
            let card = game.card(card_id);
            for ab in &card.activated_abilities {
                let desc = ab.params.get("SpellDescription")
                    .cloned()
                    .unwrap_or_else(|| ab.ability_text.clone());
                self.ability_descriptions.insert(
                    (card_id.0, ab.ability_index),
                    (desc, ab.is_mana_ability),
                );
            }
        }
    }

    fn mulligan_decision(&mut self, _player: PlayerId, hand: &[CardId]) -> bool {
        let hand_card_ids: Vec<String> = hand.iter().map(|c| format!("card-{}", c.0)).collect();
        self.send_prompt(AgentPromptInner::Mulligan {
            game_view: self.view(),
            hand_card_ids,
        });
        match self.recv_action() {
            PlayerAction::MulliganDecision { keep } => keep,
            _ => true,
        }
    }

    fn choose_action(
        &mut self,
        _player: PlayerId,
        playable: &[CardId],
        tappable_lands: &[CardId],
        untappable_lands: &[CardId],
        activatable: &[(CardId, usize)],
    ) -> MainPhaseAction {
        let playable_card_ids: Vec<String> =
            playable.iter().map(|c| format!("card-{}", c.0)).collect();
        let mut tappable_land_ids: Vec<String> = tappable_lands
            .iter()
            .map(|c| format!("card-{}", c.0))
            .collect();
        let untappable_land_ids: Vec<String> = untappable_lands
            .iter()
            .map(|c| format!("card-{}", c.0))
            .collect();

        // Build activatable ability info and merge mana-ability cards into tappable list
        let view_ref = self.view();
        let mut activatable_ability_ids = Vec::new();
        for &(card_id, ability_idx) in activatable {
            let id_str = format!("card-{}", card_id.0);
            let (description, is_mana) = self
                .ability_descriptions
                .get(&(card_id.0, ability_idx))
                .cloned()
                .unwrap_or_else(|| {
                    let text = view_ref
                        .battlefield
                        .iter()
                        .find(|c| c.id == id_str)
                        .map(|c| c.text.clone())
                        .unwrap_or_default();
                    (text, false)
                });
            activatable_ability_ids.push(ActivatableAbilityInfo {
                card_id: id_str.clone(),
                ability_index: ability_idx,
                description,
                is_mana_ability: is_mana,
            });
            if !tappable_land_ids.contains(&id_str) {
                tappable_land_ids.push(id_str);
            }
        }

        let mut view = view_ref;
        for card in &mut view.my_hand {
            card.is_playable = playable_card_ids.contains(&card.id);
        }

        self.send_prompt(AgentPromptInner::ChooseAction {
            game_view: view,
            playable_card_ids,
            tappable_land_ids,
            untappable_land_ids,
            activatable_ability_ids,
        });
        match self.recv_action() {
            PlayerAction::PlayCard { card_id } => card_id
                .and_then(|id| Self::parse_card_id(&id))
                .map(MainPhaseAction::Play)
                .unwrap_or(MainPhaseAction::Pass),
            PlayerAction::TapLand { card_id } => {
                let parsed = Self::parse_card_id(&card_id);
                match parsed {
                    Some(cid) => {
                        // Prefer ActivateAbility if card has an activatable ability
                        // (handles dual lands, non-basic lands with AB$ Mana, and non-land mana sources)
                        if let Some(&(id, idx)) = activatable.iter().find(|(id, _)| *id == cid) {
                            MainPhaseAction::ActivateAbility(id, idx)
                        } else {
                            // Basic land without AB$ Mana — use ActivateMana
                            MainPhaseAction::ActivateMana(cid)
                        }
                    }
                    None => MainPhaseAction::Pass,
                }
            }
            PlayerAction::ActivateAbility {
                card_id,
                ability_index,
            } => Self::parse_card_id(&card_id)
                .map(|cid| MainPhaseAction::ActivateAbility(cid, ability_index))
                .unwrap_or(MainPhaseAction::Pass),
            PlayerAction::UntapLand { card_id } => Self::parse_card_id(&card_id)
                .map(MainPhaseAction::UntapMana)
                .unwrap_or(MainPhaseAction::Pass),
            _ => MainPhaseAction::Pass,
        }
    }

    fn choose_attackers(&mut self, _player: PlayerId, available: &[CardId]) -> Vec<CardId> {
        let available_attacker_ids: Vec<String> =
            available.iter().map(|c| format!("card-{}", c.0)).collect();
        let mut view = self.view();
        for card in &mut view.battlefield {
            card.is_choosable = available_attacker_ids.contains(&card.id);
        }
        self.send_prompt(AgentPromptInner::ChooseAttackers {
            game_view: view,
            available_attacker_ids,
        });
        match self.recv_action() {
            PlayerAction::DeclareAttackers { attacker_ids } => attacker_ids
                .iter()
                .filter_map(|id| Self::parse_card_id(id))
                .collect(),
            _ => Vec::new(),
        }
    }

    fn choose_blockers(
        &mut self,
        _player: PlayerId,
        attackers: &[CardId],
        available_blockers: &[CardId],
    ) -> Vec<(CardId, CardId)> {
        let attacker_ids: Vec<String> = attackers.iter().map(|c| format!("card-{}", c.0)).collect();
        let available_blocker_ids: Vec<String> = available_blockers
            .iter()
            .map(|c| format!("card-{}", c.0))
            .collect();
        let mut view = self.view();
        for card in &mut view.battlefield {
            card.is_choosable = available_blocker_ids.contains(&card.id);
        }
        self.send_prompt(AgentPromptInner::ChooseBlockers {
            game_view: view,
            attacker_ids,
            available_blocker_ids,
        });
        match self.recv_action() {
            PlayerAction::DeclareBlockers { assignments } => assignments
                .iter()
                .filter_map(
                    |BlockAssignment {
                         blocker_id,
                         attacker_id,
                     }| {
                        let b = Self::parse_card_id(blocker_id)?;
                        let a = Self::parse_card_id(attacker_id)?;
                        Some((b, a))
                    },
                )
                .collect(),
            _ => Vec::new(),
        }
    }

    fn choose_target_player(&mut self, _player: PlayerId, valid: &[PlayerId]) -> Option<PlayerId> {
        let valid_player_ids: Vec<String> =
            valid.iter().map(|p| format!("player-{}", p.0)).collect();
        self.send_prompt(AgentPromptInner::ChooseTargetPlayer {
            game_view: self.view(),
            valid_player_ids,
        });
        match self.recv_action() {
            PlayerAction::TargetPlayer { player_id } => {
                player_id.and_then(|id| Self::parse_player_id(&id))
            }
            _ => valid.first().copied(),
        }
    }

    fn choose_target_card(&mut self, _player: PlayerId, valid: &[CardId]) -> Option<CardId> {
        let valid_card_ids: Vec<String> = valid.iter().map(|c| format!("card-{}", c.0)).collect();
        let mut view = self.view();
        for card in &mut view.battlefield {
            card.is_choosable = valid_card_ids.contains(&card.id);
        }
        self.send_prompt(AgentPromptInner::ChooseTargetCard {
            game_view: view,
            valid_card_ids,
        });
        match self.recv_action() {
            PlayerAction::TargetCard { card_id } => card_id.and_then(|id| Self::parse_card_id(&id)),
            _ => valid.first().copied(),
        }
    }

    fn choose_target_card_from_zone(
        &mut self,
        _player: PlayerId,
        zone: ZoneType,
        valid: &[CardId],
    ) -> Option<CardId> {
        let valid_card_ids: Vec<String> = valid.iter().map(|c| format!("card-{}", c.0)).collect();
        let view = self.view();

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

        self.send_prompt(AgentPromptInner::ChooseTargetCardFromZone {
            game_view: view,
            valid_card_ids,
            zone: format!("{:?}", zone),
            zone_cards,
        });
        match self.recv_action() {
            PlayerAction::TargetCard { card_id } => card_id.and_then(|id| Self::parse_card_id(&id)),
            _ => valid.first().copied(),
        }
    }

    fn choose_target_any(
        &mut self,
        _player: PlayerId,
        valid_players: &[PlayerId],
        valid_cards: &[CardId],
    ) -> TargetChoice {
        let valid_player_ids: Vec<String> = valid_players
            .iter()
            .map(|p| format!("player-{}", p.0))
            .collect();
        let valid_card_ids: Vec<String> = valid_cards
            .iter()
            .map(|c| format!("card-{}", c.0))
            .collect();
        let mut view = self.view();
        for card in &mut view.battlefield {
            card.is_choosable = valid_card_ids.contains(&card.id);
        }
        self.send_prompt(AgentPromptInner::ChooseTargetAny {
            game_view: view,
            valid_player_ids,
            valid_card_ids,
        });
        match self.recv_action() {
            PlayerAction::TargetAny { target } => match target {
                TargetAnyChoice::Player { player_id } => Self::parse_player_id(&player_id)
                    .map(TargetChoice::Player)
                    .unwrap_or(TargetChoice::None),
                TargetAnyChoice::Card { card_id } => Self::parse_card_id(&card_id)
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

    fn choose_sacrifice(&mut self, _player: PlayerId, valid: &[CardId]) -> Option<CardId> {
        let valid_card_ids: Vec<String> = valid.iter().map(|c| format!("card-{}", c.0)).collect();
        let mut view = self.view();
        for card in &mut view.battlefield {
            card.is_choosable = valid_card_ids.contains(&card.id);
        }
        self.send_prompt(AgentPromptInner::ChooseTargetCard {
            game_view: view,
            valid_card_ids,
        });
        match self.recv_action() {
            PlayerAction::TargetCard { card_id } => card_id.and_then(|id| Self::parse_card_id(&id)),
            _ => valid.first().copied(),
        }
    }

    fn on_library_peek(&mut self, game: &GameState, cards: &[CardId]) {
        self.peeked_library_cards = cards
            .iter()
            .map(|&id| card_to_dto(game, id, &[], &[], "library"))
            .collect();
    }

    fn choose_scry(&mut self, _player: PlayerId, cards: &[CardId]) -> Vec<CardId> {
        let card_ids: Vec<String> = cards.iter().map(|c| format!("card-{}", c.0)).collect();
        let peeked = std::mem::take(&mut self.peeked_library_cards);
        self.send_prompt(AgentPromptInner::Scry {
            game_view: self.view(),
            card_ids,
            cards: peeked,
        });
        match self.recv_action() {
            PlayerAction::ScryDecision { bottom_card_ids } => bottom_card_ids
                .iter()
                .filter_map(|id| Self::parse_card_id(id))
                .collect(),
            _ => vec![],
        }
    }

    fn choose_surveil(&mut self, _player: PlayerId, cards: &[CardId]) -> Vec<CardId> {
        let card_ids: Vec<String> = cards.iter().map(|c| format!("card-{}", c.0)).collect();
        let peeked = std::mem::take(&mut self.peeked_library_cards);
        self.send_prompt(AgentPromptInner::Surveil {
            game_view: self.view(),
            card_ids,
            cards: peeked,
        });
        match self.recv_action() {
            PlayerAction::SurveilDecision { graveyard_card_ids } => graveyard_card_ids
                .iter()
                .filter_map(|id| Self::parse_card_id(id))
                .collect(),
            _ => vec![],
        }
    }

    fn choose_dig(
        &mut self,
        _player: PlayerId,
        valid: &[CardId],
        max: usize,
        optional: bool,
    ) -> Vec<CardId> {
        let card_ids: Vec<String> = valid.iter().map(|c| format!("card-{}", c.0)).collect();
        let peeked = std::mem::take(&mut self.peeked_library_cards);
        let valid_peeked: Vec<CardDto> = peeked
            .into_iter()
            .filter(|dto| card_ids.contains(&dto.id))
            .collect();
        self.send_prompt(AgentPromptInner::Dig {
            game_view: self.view(),
            card_ids,
            cards: valid_peeked,
            num_to_take: max,
            optional,
        });
        match self.recv_action() {
            PlayerAction::DigDecision { chosen_card_ids } => chosen_card_ids
                .iter()
                .filter_map(|id| Self::parse_card_id(id))
                .collect(),
            _ => valid.iter().copied().take(max).collect(),
        }
    }

    fn choose_discard(&mut self, _player: PlayerId, hand: &[CardId], num: usize) -> Vec<CardId> {
        let hand_card_ids: Vec<String> = hand.iter().map(|c| format!("card-{}", c.0)).collect();
        self.send_prompt(AgentPromptInner::ChooseDiscard {
            game_view: self.view(),
            hand_card_ids,
            num_to_discard: num,
        });
        match self.recv_action() {
            PlayerAction::DiscardDecision { discarded_card_ids } => discarded_card_ids
                .iter()
                .filter_map(|id| Self::parse_card_id(id))
                .collect(),
            _ => hand.iter().copied().take(num).collect(),
        }
    }

    fn choose_target_spell(&mut self, _player: PlayerId, valid: &[u32]) -> Option<u32> {
        let valid_spell_ids: Vec<String> = valid.iter().map(|id| format!("stack-{}", id)).collect();
        self.send_prompt(AgentPromptInner::ChooseTargetSpell {
            game_view: self.view(),
            valid_spell_ids,
        });
        match self.recv_action() {
            PlayerAction::TargetSpell { spell_id } => spell_id.and_then(|id| parse_spell_id(&id)),
            _ => valid.first().copied(),
        }
    }

    fn choose_color(&mut self, _player: PlayerId, valid_colors: &[String]) -> Option<String> {
        self.send_prompt(AgentPromptInner::ChooseColor {
            game_view: self.view(),
            valid_colors: valid_colors.to_vec(),
            source_card_name: None,
        });
        match self.recv_action() {
            PlayerAction::ColorDecision { color } => color,
            _ => valid_colors.first().cloned(),
        }
    }

    fn choose_cards_for_effect(
        &mut self,
        _player: PlayerId,
        valid: &[CardId],
        min: usize,
        max: usize,
    ) -> Vec<CardId> {
        let valid_card_ids: Vec<String> = valid.iter().map(|c| format!("card-{}", c.0)).collect();
        let view = self.view();

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

        self.send_prompt(AgentPromptInner::ChooseCardsForEffect {
            game_view: view,
            valid_card_ids,
            zone_cards,
            min_choices: min,
            max_choices: max,
            source_card_name: None,
        });
        match self.recv_action() {
            PlayerAction::ChooseCardsDecision { chosen_card_ids } => chosen_card_ids
                .iter()
                .filter_map(|id| Self::parse_card_id(id))
                .collect(),
            _ => valid.iter().copied().take(max).collect(),
        }
    }

    fn choose_type(&mut self, _player: PlayerId, type_category: &str, valid_types: &[String]) -> Option<String> {
        self.send_prompt(AgentPromptInner::ChooseType {
            game_view: self.view(),
            type_category: type_category.to_string(),
            valid_types: valid_types.to_vec(),
            source_card_name: None,
        });
        match self.recv_action() {
            PlayerAction::TypeDecision { chosen_type } => chosen_type,
            _ => valid_types.first().cloned(),
        }
    }

    fn choose_card_name(&mut self, _player: PlayerId, valid_names: &[String]) -> Option<String> {
        self.send_prompt(AgentPromptInner::ChooseCardName {
            game_view: self.view(),
            valid_names: valid_names.to_vec(),
            source_card_name: None,
        });
        match self.recv_action() {
            PlayerAction::CardNameDecision { chosen_name } => chosen_name,
            _ => valid_names.first().cloned(),
        }
    }

    fn choose_number(&mut self, _player: PlayerId, min: i32, max: i32) -> Option<i32> {
        self.send_prompt(AgentPromptInner::ChooseNumber {
            game_view: self.view(),
            min,
            max,
            source_card_name: None,
        });
        match self.recv_action() {
            PlayerAction::NumberDecision { chosen_number } => chosen_number,
            _ => Some(min),
        }
    }

    fn choose_land_or_spell(&mut self, _player: PlayerId) -> Option<bool> {
        None
    }

    fn notify(&mut self, _message: &str) {
        // Notifications for remote players are sent via StateUpdate prompts
    }

    fn notify_card_played(
        &mut self,
        player: PlayerId,
        card_id: CardId,
        card_name: &str,
        set_code: &str,
    ) {
        self.pending_display_events.push(DisplayEvent::CardPlayed {
            card_id: format!("card-{}", card_id.0),
            card_name: card_name.to_string(),
            set_code: set_code.to_string(),
            player_id: format!("player-{}", player.0),
        });
        self.send_prompt(AgentPromptInner::StateUpdate {
            game_view: self.view(),
        });
    }

    fn notify_turn_changed(&mut self, active_player: PlayerId, turn_number: u32) {
        let player_id = format!("player-{}", active_player.0);
        let active_player_name = self
            .latest_view
            .as_ref()
            .and_then(|v| v.players.iter().find(|p| p.id == player_id))
            .map(|p| p.name.clone())
            .unwrap_or_else(|| format!("Player {}", active_player.0));
        self.pending_display_events.push(DisplayEvent::TurnChanged {
            active_player_id: player_id,
            active_player_name,
            turn_number,
        });
        self.send_prompt(AgentPromptInner::StateUpdate {
            game_view: self.view(),
        });
    }

    fn notify_phase_changed(&mut self, _phase: PhaseType) {
        self.send_prompt(AgentPromptInner::StateUpdate {
            game_view: self.view(),
        });
    }

    fn notify_state_changed(&mut self) {
        self.send_prompt(AgentPromptInner::StateUpdate {
            game_view: self.view(),
        });
    }
}
