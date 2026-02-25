use std::sync::mpsc;

use forge_engine_core::agent::{MainPhaseAction, PlayerAgent, TargetChoice};
use forge_engine_core::game::GameState;
use forge_engine_core::ids::{CardId, PlayerId};
use forge_engine_core::mana::ManaPool;
use forge_foundation::{PhaseType, ZoneType};

use crate::game_view_dto::{card_to_dto, CardDto, GameViewDto};
use crate::prompt::{
    AgentPrompt, AgentPromptInner, BlockAssignment, DisplayEvent, PlayerAction, TargetAnyChoice,
};

/// Parse "stack-42" → 42u32 (stack entry ID).
/// Matches the DTO format used in `game_view_dto.rs` (`format!("stack-{}", entry.id)`).
fn parse_spell_id(s: &str) -> Option<u32> {
    s.strip_prefix("stack-").and_then(|n| n.parse::<u32>().ok())
}

/// A PlayerAgent that sends prompts to the frontend and blocks waiting for a response.
pub struct TauriAgent {
    pub human_player: PlayerId,
    pub game_id: String,
    pub prompt_tx: mpsc::Sender<AgentPrompt>,
    pub response_rx: mpsc::Receiver<PlayerAction>,
    pub notify_tx: mpsc::Sender<String>,
    latest_view: Option<GameViewDto>,
    /// Display events accumulated between prompts — drained and attached to each outgoing prompt.
    pending_display_events: Vec<DisplayEvent>,
    /// Card DTOs pre-built by on_library_peek() for Scry/Surveil/Dig prompts.
    peeked_library_cards: Vec<CardDto>,
}

impl TauriAgent {
    pub fn new(
        human_player: PlayerId,
        game_id: String,
        prompt_tx: mpsc::Sender<AgentPrompt>,
        response_rx: mpsc::Receiver<PlayerAction>,
        notify_tx: mpsc::Sender<String>,
    ) -> Self {
        Self {
            human_player,
            game_id,
            prompt_tx,
            response_rx,
            notify_tx,
            latest_view: None,
            pending_display_events: Vec::new(),
            peeked_library_cards: Vec::new(),
        }
    }

    /// Send a prompt to the frontend, bundling any accumulated display events.
    fn send_prompt(&mut self, inner: AgentPromptInner) {
        let display_events = std::mem::take(&mut self.pending_display_events);
        let _ = self.prompt_tx.send(AgentPrompt {
            display_events,
            inner,
        });
    }

    fn recv_action(&self) -> PlayerAction {
        self.response_rx
            .recv()
            .unwrap_or(PlayerAction::PlayCard { card_id: None })
    }

    fn view(&self) -> GameViewDto {
        self.latest_view.clone().unwrap_or_else(|| {
            // Fallback: empty view
            GameViewDto {
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
            }
        })
    }

    /// Parse "card-42" → CardId(42)
    fn parse_card_id(s: &str) -> Option<CardId> {
        s.strip_prefix("card-")
            .and_then(|n| n.parse::<u32>().ok())
            .map(CardId)
    }

    /// Parse "player-0" → PlayerId(0)
    fn parse_player_id(s: &str) -> Option<PlayerId> {
        s.strip_prefix("player-")
            .and_then(|n| n.parse::<u32>().ok())
            .map(PlayerId)
    }
}

impl PlayerAgent for TauriAgent {
    fn snapshot_state(&mut self, game: &GameState, mana_pools: &[ManaPool]) {
        self.latest_view = Some(GameViewDto::from_engine(
            game,
            mana_pools,
            self.human_player,
            &self.game_id,
            &[], // playable/choosable filled at prompt time
            &[],
        ));
    }

    fn mulligan_decision(&mut self, _player: PlayerId, hand: &[CardId]) -> bool {
        let hand_card_ids: Vec<String> = hand.iter().map(|c| format!("card-{}", c.0)).collect();
        self.send_prompt(AgentPromptInner::Mulligan {
            game_view: self.view(),
            hand_card_ids,
        });
        match self.recv_action() {
            PlayerAction::MulliganDecision { keep } => keep,
            _ => true, // default: keep
        }
    }

    fn choose_action(
        &mut self,
        _player: PlayerId,
        playable: &[CardId],
        tappable_lands: &[CardId],
        untappable_lands: &[CardId],
        _activatable: &[(CardId, usize)],
    ) -> MainPhaseAction {
        let playable_card_ids: Vec<String> =
            playable.iter().map(|c| format!("card-{}", c.0)).collect();
        let tappable_land_ids: Vec<String> = tappable_lands
            .iter()
            .map(|c| format!("card-{}", c.0))
            .collect();
        let untappable_land_ids: Vec<String> = untappable_lands
            .iter()
            .map(|c| format!("card-{}", c.0))
            .collect();

        // Update the view with playable info (hand, graveyard, command zone)
        let mut view = self.view();
        for card in &mut view.my_hand {
            card.is_playable = playable_card_ids.contains(&card.id);
        }
        for card in &mut view.graveyard {
            card.is_playable = playable_card_ids.contains(&card.id);
        }
        for card in &mut view.my_command_zone {
            card.is_playable = playable_card_ids.contains(&card.id);
        }

        self.send_prompt(AgentPromptInner::ChooseAction {
            game_view: view,
            playable_card_ids,
            tappable_land_ids,
            untappable_land_ids,
        });
        match self.recv_action() {
            PlayerAction::PlayCard { card_id } => card_id
                .and_then(|id| Self::parse_card_id(&id))
                .map(MainPhaseAction::Play)
                .unwrap_or(MainPhaseAction::Pass),
            PlayerAction::TapLand { card_id } => Self::parse_card_id(&card_id)
                .map(MainPhaseAction::ActivateMana)
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

    fn choose_sacrifice(&mut self, player: PlayerId, valid: &[CardId]) -> Option<CardId> {
        // Non-human players: just pick the first valid card (AI behavior)
        if player != self.human_player {
            return valid.first().copied();
        }
        // Human player: reuse the ChooseTargetCard prompt so the frontend can pick
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

    fn on_library_peek(&mut self, game: &forge_engine_core::game::GameState, cards: &[CardId]) {
        // Only build DTOs for the human player — AI peeks are silent.
        self.peeked_library_cards = cards
            .iter()
            .map(|&id| card_to_dto(game, id, &[], &[], "library"))
            .collect();
    }

    fn choose_scry(&mut self, player: PlayerId, cards: &[CardId]) -> Vec<CardId> {
        if player != self.human_player {
            return vec![]; // AI: keep all on top
        }
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

    fn choose_surveil(&mut self, player: PlayerId, cards: &[CardId]) -> Vec<CardId> {
        if player != self.human_player {
            return vec![]; // AI: keep all on top
        }
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
        player: PlayerId,
        valid: &[CardId],
        max: usize,
        optional: bool,
    ) -> Vec<CardId> {
        if player != self.human_player {
            // AI: take first `max` cards
            return valid.iter().copied().take(max).collect();
        }
        let card_ids: Vec<String> = valid.iter().map(|c| format!("card-{}", c.0)).collect();
        let peeked = std::mem::take(&mut self.peeked_library_cards);
        // Filter peeked to only valid cards (ChangeValid$ may have narrowed the list).
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

    fn choose_discard(&mut self, player: PlayerId, hand: &[CardId], num: usize) -> Vec<CardId> {
        if player != self.human_player {
            return hand.iter().copied().take(num).collect();
        }
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

    fn choose_target_spell(&mut self, player: PlayerId, valid: &[u32]) -> Option<u32> {
        if player != self.human_player {
            return valid.first().copied();
        }
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

    fn choose_mode(
        &mut self,
        player: PlayerId,
        descriptions: &[String],
        min: usize,
        max: usize,
        card_name: Option<&str>,
    ) -> Vec<usize> {
        if player != self.human_player {
            // AI: pick first `min` modes
            return (0..min.min(descriptions.len())).collect();
        }
        self.send_prompt(AgentPromptInner::ChooseMode {
            game_view: self.view(),
            options: descriptions.to_vec(),
            min_choices: min,
            max_choices: max,
            source_card_name: card_name.map(String::from),
        });
        match self.recv_action() {
            PlayerAction::ModeDecision { chosen_indices } => chosen_indices,
            _ => (0..min.min(descriptions.len())).collect(),
        }
    }

    fn choose_optional_trigger(&mut self, player: PlayerId, description: &str, card_name: Option<&str>) -> bool {
        if player != self.human_player {
            return true; // AI always accepts optional triggers
        }
        self.send_prompt(AgentPromptInner::ChooseOptionalTrigger {
            game_view: self.view(),
            description: description.to_string(),
            source_card_name: card_name.map(String::from),
        });
        match self.recv_action() {
            PlayerAction::OptionalTriggerDecision { accept } => accept,
            _ => true,
        }
    }

    fn choose_kicker(&mut self, player: PlayerId, kicker_cost: &str, card_name: Option<&str>) -> bool {
        if player != self.human_player {
            return false; // AI default: don't kick
        }
        self.send_prompt(AgentPromptInner::ChooseKicker {
            game_view: self.view(),
            kicker_cost: kicker_cost.to_string(),
            source_card_name: card_name.map(String::from),
        });
        match self.recv_action() {
            PlayerAction::KickerDecision { kicked } => kicked,
            _ => false,
        }
    }

    fn choose_buyback(&mut self, player: PlayerId, buyback_cost: &str, card_name: Option<&str>) -> bool {
        if player != self.human_player {
            return false;
        }
        self.send_prompt(AgentPromptInner::ChooseBuyback {
            game_view: self.view(),
            buyback_cost: buyback_cost.to_string(),
            source_card_name: card_name.map(String::from),
        });
        match self.recv_action() {
            PlayerAction::BuybackDecision { buyback_paid } => buyback_paid,
            _ => false,
        }
    }

    fn choose_multikicker(&mut self, player: PlayerId, cost: &str, max_kicks: u32, card_name: Option<&str>) -> u32 {
        if player != self.human_player {
            return 0;
        }
        self.send_prompt(AgentPromptInner::ChooseMultikicker {
            game_view: self.view(),
            cost: cost.to_string(),
            max_kicks,
            source_card_name: card_name.map(String::from),
        });
        match self.recv_action() {
            PlayerAction::MultikickerDecision { kick_count } => kick_count.min(max_kicks),
            _ => 0,
        }
    }

    fn choose_replicate(&mut self, player: PlayerId, cost: &str, max_replicates: u32, card_name: Option<&str>) -> u32 {
        if player != self.human_player {
            return 0;
        }
        self.send_prompt(AgentPromptInner::ChooseReplicate {
            game_view: self.view(),
            cost: cost.to_string(),
            max_replicates,
            source_card_name: card_name.map(String::from),
        });
        match self.recv_action() {
            PlayerAction::ReplicateDecision { replicate_count } => replicate_count.min(max_replicates),
            _ => 0,
        }
    }

    fn choose_alternative_cost(&mut self, player: PlayerId, options: &[String], card_name: Option<&str>) -> usize {
        if player != self.human_player {
            return 0; // AI: always pick normal cost
        }
        self.send_prompt(AgentPromptInner::ChooseAlternativeCost {
            game_view: self.view(),
            options: options.to_vec(),
            source_card_name: card_name.map(String::from),
        });
        match self.recv_action() {
            PlayerAction::AlternativeCostDecision { chosen_index } => chosen_index.min(options.len().saturating_sub(1)),
            _ => 0,
        }
    }

    fn choose_color(&mut self, player: PlayerId, valid_colors: &[String]) -> Option<String> {
        if player != self.human_player {
            return valid_colors.first().cloned();
        }
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
        player: PlayerId,
        valid: &[CardId],
        min: usize,
        max: usize,
    ) -> Vec<CardId> {
        if player != self.human_player {
            return valid.iter().copied().take(max).collect();
        }
        let valid_card_ids: Vec<String> = valid.iter().map(|c| format!("card-{}", c.0)).collect();
        let view = self.view();

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

    fn choose_land_or_spell(&mut self, _player: PlayerId) -> Option<bool> {
        None
    }

    fn notify(&mut self, message: &str) {
        let _ = self.notify_tx.send(message.to_string());
    }

    fn notify_card_played(&mut self, player: PlayerId, card_id: CardId, card_name: &str, set_code: &str) {
        self.pending_display_events.push(DisplayEvent::CardPlayed {
            card_id: format!("card-{}", card_id.0),
            card_name: card_name.to_string(),
            set_code: set_code.to_string(),
            player_id: format!("player-{}", player.0),
        });
        // Flush immediately so the frontend receives one event per card play.
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
        // Flush immediately so the frontend receives one event per turn change.
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
