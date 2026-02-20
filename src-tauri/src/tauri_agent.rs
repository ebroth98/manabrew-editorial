use std::sync::mpsc;

use forge_engine_core::agent::{MainPhaseAction, PlayerAgent, TargetChoice};
use forge_engine_core::game::GameState;
use forge_engine_core::ids::{CardId, PlayerId};
use forge_engine_core::mana_pool::ManaPool;

use crate::game_view_dto::GameViewDto;
use crate::prompt::{AgentPrompt, BlockAssignment, PlayerAction, TargetAnyChoice};

/// A PlayerAgent that sends prompts to the frontend and blocks waiting for a response.
pub struct TauriAgent {
    pub human_player: PlayerId,
    pub game_id: String,
    pub prompt_tx: mpsc::Sender<AgentPrompt>,
    pub response_rx: mpsc::Receiver<PlayerAction>,
    pub notify_tx: mpsc::Sender<String>,
    latest_view: Option<GameViewDto>,
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
        }
    }

    fn send_prompt(&self, prompt: AgentPrompt) {
        let _ = self.prompt_tx.send(prompt);
    }

    fn recv_action(&self) -> PlayerAction {
        self.response_rx.recv().unwrap_or(PlayerAction::PlayCard { card_id: None })
    }

    fn view(&self) -> GameViewDto {
        self.latest_view.clone().unwrap_or_else(|| {
            // Fallback: empty view
            GameViewDto {
                game_id: self.game_id.clone(),
                turn: 0,
                step: "main1".into(),
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
        self.send_prompt(AgentPrompt::Mulligan {
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
    ) -> MainPhaseAction {
        let playable_card_ids: Vec<String> = playable.iter().map(|c| format!("card-{}", c.0)).collect();
        let tappable_land_ids: Vec<String> = tappable_lands.iter().map(|c| format!("card-{}", c.0)).collect();
        let untappable_land_ids: Vec<String> = untappable_lands.iter().map(|c| format!("card-{}", c.0)).collect();

        // Update the view with playable info
        let mut view = self.view();
        for card in &mut view.my_hand {
            card.is_playable = playable_card_ids.contains(&card.id);
        }

        self.send_prompt(AgentPrompt::ChooseAction {
            game_view: view,
            playable_card_ids,
            tappable_land_ids,
            untappable_land_ids,
        });
        match self.recv_action() {
            PlayerAction::PlayCard { card_id } => {
                card_id.and_then(|id| Self::parse_card_id(&id))
                    .map(MainPhaseAction::Play)
                    .unwrap_or(MainPhaseAction::Pass)
            }
            PlayerAction::TapLand { card_id } => {
                Self::parse_card_id(&card_id)
                    .map(MainPhaseAction::ActivateMana)
                    .unwrap_or(MainPhaseAction::Pass)
            }
            PlayerAction::UntapLand { card_id } => {
                Self::parse_card_id(&card_id)
                    .map(MainPhaseAction::UntapMana)
                    .unwrap_or(MainPhaseAction::Pass)
            }
            _ => MainPhaseAction::Pass,
        }
    }

    fn choose_attackers(&mut self, _player: PlayerId, available: &[CardId]) -> Vec<CardId> {
        let available_attacker_ids: Vec<String> = available.iter().map(|c| format!("card-{}", c.0)).collect();
        let mut view = self.view();
        for card in &mut view.battlefield {
            card.is_choosable = available_attacker_ids.contains(&card.id);
        }
        self.send_prompt(AgentPrompt::ChooseAttackers {
            game_view: view,
            available_attacker_ids,
        });
        match self.recv_action() {
            PlayerAction::DeclareAttackers { attacker_ids } => {
                attacker_ids.iter().filter_map(|id| Self::parse_card_id(id)).collect()
            }
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
        let available_blocker_ids: Vec<String> = available_blockers.iter().map(|c| format!("card-{}", c.0)).collect();
        let mut view = self.view();
        for card in &mut view.battlefield {
            card.is_choosable = available_blocker_ids.contains(&card.id);
        }
        self.send_prompt(AgentPrompt::ChooseBlockers {
            game_view: view,
            attacker_ids,
            available_blocker_ids,
        });
        match self.recv_action() {
            PlayerAction::DeclareBlockers { assignments } => {
                assignments.iter().filter_map(|BlockAssignment { blocker_id, attacker_id }| {
                    let b = Self::parse_card_id(blocker_id)?;
                    let a = Self::parse_card_id(attacker_id)?;
                    Some((b, a))
                }).collect()
            }
            _ => Vec::new(),
        }
    }

    fn choose_target_player(&mut self, _player: PlayerId, valid: &[PlayerId]) -> Option<PlayerId> {
        let valid_player_ids: Vec<String> = valid.iter().map(|p| format!("player-{}", p.0)).collect();
        self.send_prompt(AgentPrompt::ChooseTargetPlayer {
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
        self.send_prompt(AgentPrompt::ChooseTargetCard {
            game_view: view,
            valid_card_ids,
        });
        match self.recv_action() {
            PlayerAction::TargetCard { card_id } => {
                card_id.and_then(|id| Self::parse_card_id(&id))
            }
            _ => valid.first().copied(),
        }
    }

    fn choose_target_any(
        &mut self,
        _player: PlayerId,
        valid_players: &[PlayerId],
        valid_cards: &[CardId],
    ) -> TargetChoice {
        let valid_player_ids: Vec<String> = valid_players.iter().map(|p| format!("player-{}", p.0)).collect();
        let valid_card_ids: Vec<String> = valid_cards.iter().map(|c| format!("card-{}", c.0)).collect();
        let mut view = self.view();
        for card in &mut view.battlefield {
            card.is_choosable = valid_card_ids.contains(&card.id);
        }
        self.send_prompt(AgentPrompt::ChooseTargetAny {
            game_view: view,
            valid_player_ids,
            valid_card_ids,
        });
        match self.recv_action() {
            PlayerAction::TargetAny { target } => match target {
                TargetAnyChoice::Player { player_id } => {
                    Self::parse_player_id(&player_id)
                        .map(TargetChoice::Player)
                        .unwrap_or(TargetChoice::None)
                }
                TargetAnyChoice::Card { card_id } => {
                    Self::parse_card_id(&card_id)
                        .map(TargetChoice::Card)
                        .unwrap_or(TargetChoice::None)
                }
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

    fn choose_land_or_spell(&mut self, _player: PlayerId) -> Option<bool> {
        None
    }

    fn notify(&mut self, message: &str) {
        let _ = self.notify_tx.send(message.to_string());
    }
}
