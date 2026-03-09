use std::sync::mpsc;
use std::time::Duration;

use forge_engine_core::agent::{
    BinaryChoiceKind, CombatCostAction, GameLogEvent, MainPhaseAction, ManaCostAction, PlayOption,
    PlayerAgent, TargetChoice,
};
use forge_engine_core::combat::DefenderId;
use forge_engine_core::game::GameState;
use forge_engine_core::ids::{CardId, PlayerId};
use forge_engine_core::mana::ManaPool;
use forge_foundation::{PhaseType, ZoneType};

use crate::game_log_event::GameLogEntryDto;
use crate::game_snapshot_event::GameSnapshotEventDto;
use crate::game_view_dto::{card_to_dto, CardDto, GameViewDto};
use crate::ids_codec::{
    card_id_str, parse_card_id, parse_player_id, parse_stack_id, player_id_str, stack_id_str,
};
use crate::prompt::{
    ActivatableAbilityInfo, AgentPrompt, AgentPromptInner, BlockAssignment, DisplayEvent,
    PlayerAction, TargetAnyChoice,
};

/// A PlayerAgent that sends prompts to the frontend and blocks waiting for a response.
pub struct TauriAgent {
    pub player_id: PlayerId,
    pub game_id: String,
    prompt_sink: PromptSink,
    pub response_rx: mpsc::Receiver<PlayerAction>,
    pub notify_tx: Option<mpsc::Sender<GameLogEntryDto>>,
    pub snapshot_tx: Option<mpsc::Sender<GameSnapshotEventDto>>,
    response_timeout: Option<Duration>,
    latest_view: Option<GameViewDto>,
    /// Display events accumulated between prompts — drained and attached to each outgoing prompt.
    pending_display_events: Vec<DisplayEvent>,
    /// Card DTOs pre-built by on_library_peek() for Scry/Surveil/Dig prompts.
    peeked_library_cards: Vec<CardDto>,
    /// Cached per-ability descriptions and is_mana_ability flags, populated in snapshot_state.
    /// Key: (card_id.0, ability_index) → (description, is_mana_ability)
    ability_descriptions: std::collections::HashMap<(u32, usize), (String, bool)>,
    pending_restore_checkpoint: Option<u64>,
}

enum PromptSink {
    Local(mpsc::Sender<AgentPrompt>),
    Relay {
        player_index: usize,
        tx: mpsc::Sender<(usize, AgentPrompt)>,
    },
    Ai(mpsc::Sender<AgentPrompt>),
}

impl TauriAgent {
    pub fn new_local(
        player_id: PlayerId,
        game_id: String,
        prompt_tx: mpsc::Sender<AgentPrompt>,
        response_rx: mpsc::Receiver<PlayerAction>,
        notify_tx: mpsc::Sender<GameLogEntryDto>,
        snapshot_tx: mpsc::Sender<GameSnapshotEventDto>,
    ) -> Self {
        Self {
            player_id,
            game_id,
            prompt_sink: PromptSink::Local(prompt_tx),
            response_rx,
            notify_tx: Some(notify_tx),
            snapshot_tx: Some(snapshot_tx),
            response_timeout: None,
            latest_view: None,
            pending_display_events: Vec::new(),
            peeked_library_cards: Vec::new(),
            ability_descriptions: std::collections::HashMap::new(),
            pending_restore_checkpoint: None,
        }
    }

    pub fn new_relay(
        player_id: PlayerId,
        player_index: usize,
        game_id: String,
        prompt_tx: mpsc::Sender<(usize, AgentPrompt)>,
        response_rx: mpsc::Receiver<PlayerAction>,
    ) -> Self {
        Self {
            player_id,
            game_id,
            prompt_sink: PromptSink::Relay {
                player_index,
                tx: prompt_tx,
            },
            response_rx,
            notify_tx: None,
            snapshot_tx: None,
            response_timeout: Some(Duration::from_secs(120)),
            latest_view: None,
            pending_display_events: Vec::new(),
            peeked_library_cards: Vec::new(),
            ability_descriptions: std::collections::HashMap::new(),
            pending_restore_checkpoint: None,
        }
    }

    pub fn new_ai(
        player_id: PlayerId,
        game_id: String,
        prompt_tx: mpsc::Sender<AgentPrompt>,
        response_rx: mpsc::Receiver<PlayerAction>,
    ) -> Self {
        Self {
            player_id,
            game_id,
            prompt_sink: PromptSink::Ai(prompt_tx),
            response_rx,
            notify_tx: None,
            snapshot_tx: None,
            response_timeout: Some(Duration::from_secs(5)),
            latest_view: None,
            pending_display_events: Vec::new(),
            peeked_library_cards: Vec::new(),
            ability_descriptions: std::collections::HashMap::new(),
            pending_restore_checkpoint: None,
        }
    }

    /// Send a prompt to the frontend, bundling any accumulated display events.
    fn send_prompt(&mut self, inner: AgentPromptInner) {
        let display_events = std::mem::take(&mut self.pending_display_events);
        let prompt = AgentPrompt {
            display_events,
            inner,
        };
        match &self.prompt_sink {
            PromptSink::Local(tx) => {
                let _ = tx.send(prompt);
            }
            PromptSink::Relay { player_index, tx } => {
                let _ = tx.send((*player_index, prompt));
            }
            PromptSink::Ai(tx) => {
                let _ = tx.send(prompt);
            }
        }
    }

    fn recv_action(&self) -> PlayerAction {
        if let Some(timeout) = self.response_timeout {
            self.response_rx
                .recv_timeout(timeout)
                .unwrap_or(PlayerAction::PlayCard { card_id: None })
        } else {
            self.response_rx
                .recv()
                .unwrap_or(PlayerAction::PlayCard { card_id: None })
        }
    }

    fn view(&self) -> GameViewDto {
        self.latest_view.clone().unwrap_or_else(|| {
            // Fallback: empty view
            GameViewDto::empty(self.game_id.clone())
        })
    }

    fn card_ids(cards: &[CardId]) -> Vec<String> {
        cards.iter().map(|&c| card_id_str(c)).collect()
    }

    fn player_ids(players: &[PlayerId]) -> Vec<String> {
        players.iter().map(|&p| player_id_str(p)).collect()
    }

    fn defender_ids_to_dtos(defenders: &[DefenderId]) -> Vec<crate::prompt::DefenderIdDto> {
        defenders
            .iter()
            .map(|d| match d {
                DefenderId::Player(pid) => crate::prompt::DefenderIdDto {
                    id: format!("player-{}", pid.0),
                    label: format!("Player {}", pid.0),
                },
                DefenderId::Permanent(cid) => crate::prompt::DefenderIdDto {
                    id: format!("card-{}", cid.0),
                    label: format!("Permanent {}", cid.0),
                },
            })
            .collect()
    }

    fn parse_defender_id(id: &str, possible: &[DefenderId]) -> Option<DefenderId> {
        if let Some(rest) = id.strip_prefix("player-") {
            let idx: u32 = rest.parse().ok()?;
            possible
                .iter()
                .find(|d| matches!(d, DefenderId::Player(p) if p.0 == idx))
                .copied()
        } else if let Some(rest) = id.strip_prefix("card-") {
            let idx: u32 = rest.parse().ok()?;
            possible
                .iter()
                .find(|d| matches!(d, DefenderId::Permanent(c) if c.0 == idx))
                .copied()
        } else {
            None
        }
    }

    fn mark_battlefield_choosable(view: &mut GameViewDto, valid_card_ids: &[String]) {
        for card in &mut view.battlefield {
            card.is_choosable = valid_card_ids.contains(&card.id);
        }
    }

    fn recv_card_choice_or_first(&self, valid: &[CardId]) -> Option<CardId> {
        match self.recv_action() {
            PlayerAction::TargetCard { card_id } => card_id.and_then(|id| parse_card_id(&id)),
            _ => valid.first().copied(),
        }
    }

    fn recv_player_choice_or_first(&self, valid: &[PlayerId]) -> Option<PlayerId> {
        match self.recv_action() {
            PlayerAction::TargetPlayer { player_id } => {
                player_id.and_then(|id| parse_player_id(&id))
            }
            _ => valid.first().copied(),
        }
    }

    fn recv_spell_choice_or_first(&self, valid: &[u32]) -> Option<u32> {
        match self.recv_action() {
            PlayerAction::TargetSpell { spell_id } => spell_id.and_then(|id| parse_stack_id(&id)),
            _ => valid.first().copied(),
        }
    }
}

impl PlayerAgent for TauriAgent {
    fn snapshot_state(&mut self, game: &GameState, mana_pools: &[ManaPool]) {
        self.latest_view = Some(GameViewDto::from_engine(
            game,
            mana_pools,
            self.player_id,
            &self.game_id,
            &[], // playable/choosable filled at prompt time
            &[],
        ));

        // Cache per-ability descriptions from battlefield cards
        self.ability_descriptions.clear();
        let battlefield =
            game.cards_in_zone(forge_foundation::ZoneType::Battlefield, self.player_id);
        for &card_id in battlefield {
            let card = game.card(card_id);
            for ab in &card.activated_abilities {
                let desc = ab
                    .params
                    .get("SpellDescription")
                    .cloned()
                    .unwrap_or_else(|| ab.ability_text.clone());
                self.ability_descriptions
                    .insert((card_id.0, ab.ability_index), (desc, ab.is_mana_ability));
            }
        }
    }

    fn mulligan_decision(
        &mut self,
        _player: PlayerId,
        hand: &[CardId],
        mulligan_count: u32,
    ) -> bool {
        let hand_card_ids = Self::card_ids(hand);
        self.send_prompt(AgentPromptInner::Mulligan {
            game_view: self.view(),
            hand_card_ids,
            mulligan_count,
        });
        match self.recv_action() {
            PlayerAction::MulliganDecision { keep } => keep,
            _ => true,
        }
    }

    fn choose_cards_to_bottom(
        &mut self,
        _player: PlayerId,
        hand: &[CardId],
        count: usize,
    ) -> Vec<CardId> {
        let view = self.view();
        let hand_card_ids = Self::card_ids(hand);
        let cards: Vec<CardDto> = hand
            .iter()
            .filter_map(|&cid| {
                let id_str = card_id_str(cid);
                view.my_hand.iter().find(|c| c.id == id_str).cloned()
            })
            .collect();
        self.send_prompt(AgentPromptInner::MulliganPutBack {
            game_view: view,
            hand_card_ids,
            cards,
            count,
        });
        match self.recv_action() {
            PlayerAction::MulliganPutBackDecision { card_ids } => {
                card_ids.iter().filter_map(|s| parse_card_id(s)).collect()
            }
            _ => hand.iter().copied().take(count).collect(),
        }
    }

    fn choose_action(
        &mut self,
        _player: PlayerId,
        playable: &[PlayOption],
        tappable_lands: &[CardId],
        untappable_lands: &[CardId],
        activatable: &[(CardId, usize)],
    ) -> MainPhaseAction {
        let playable_card_ids: Vec<String> =
            playable.iter().map(|play| card_id_str(play.card_id)).collect();
        let mut tappable_land_ids: Vec<String> =
            tappable_lands.iter().map(|&c| card_id_str(c)).collect();
        let untappable_land_ids: Vec<String> =
            untappable_lands.iter().map(|&c| card_id_str(c)).collect();

        // Build activatable ability info and merge mana-ability cards into tappable list
        let view_ref = self.view();
        let mut activatable_ability_ids = Vec::new();
        for &(card_id, ability_idx) in activatable {
            let id_str = card_id_str(card_id);
            let (description, is_mana) = self
                .ability_descriptions
                .get(&(card_id.0, ability_idx))
                .cloned()
                .unwrap_or_else(|| {
                    // Fallback: use card text from view
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
            // Add non-land activatable cards to tappable list so they get the TAP button
            if !tappable_land_ids.contains(&id_str) {
                tappable_land_ids.push(id_str);
            }
        }

        // Update the view with playable info (hand, graveyard, command zone)
        let mut view = view_ref;
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
            activatable_ability_ids,
        });
        match self.recv_action() {
            PlayerAction::RestoreSnapshot { checkpoint_id } => {
                self.pending_restore_checkpoint = Some(checkpoint_id);
                MainPhaseAction::Pass
            }
            PlayerAction::PlayCard { card_id } => card_id
                .and_then(|id| parse_card_id(&id))
                .and_then(|cid| playable.iter().copied().find(|play| play.card_id == cid))
                .map(MainPhaseAction::Play)
                .unwrap_or(MainPhaseAction::Pass),
            PlayerAction::TapLand { card_id } => {
                let parsed = parse_card_id(&card_id);
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
            } => parse_card_id(&card_id)
                .map(|cid| MainPhaseAction::ActivateAbility(cid, ability_index))
                .unwrap_or(MainPhaseAction::Pass),
            PlayerAction::UntapLand { card_id } => parse_card_id(&card_id)
                .map(MainPhaseAction::UntapMana)
                .unwrap_or(MainPhaseAction::Pass),
            _ => MainPhaseAction::Pass,
        }
    }

    fn choose_attackers(
        &mut self,
        _player: PlayerId,
        available: &[CardId],
        possible_defenders: &[DefenderId],
    ) -> Vec<(CardId, DefenderId)> {
        let available_attacker_ids = Self::card_ids(available);
        let possible_defender_dtos = Self::defender_ids_to_dtos(possible_defenders);
        let mut view = self.view();
        Self::mark_battlefield_choosable(&mut view, &available_attacker_ids);
        self.send_prompt(AgentPromptInner::ChooseAttackers {
            game_view: view,
            available_attacker_ids,
            possible_defender_ids: possible_defender_dtos,
        });
        let default_defender = possible_defenders
            .first()
            .copied()
            .unwrap_or(DefenderId::Player(PlayerId(1)));
        match self.recv_action() {
            PlayerAction::RestoreSnapshot { checkpoint_id } => {
                self.pending_restore_checkpoint = Some(checkpoint_id);
                Vec::new()
            }
            PlayerAction::DeclareAttackers { assignments } => assignments
                .iter()
                .filter_map(|a| {
                    let attacker = parse_card_id(&a.attacker_id)?;
                    let defender = Self::parse_defender_id(&a.defender_id, possible_defenders)
                        .unwrap_or(default_defender);
                    Some((attacker, defender))
                })
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
        let attacker_ids = Self::card_ids(attackers);
        let available_blocker_ids = Self::card_ids(available_blockers);
        let mut view = self.view();
        Self::mark_battlefield_choosable(&mut view, &available_blocker_ids);
        self.send_prompt(AgentPromptInner::ChooseBlockers {
            game_view: view,
            attacker_ids,
            available_blocker_ids,
        });
        match self.recv_action() {
            PlayerAction::RestoreSnapshot { checkpoint_id } => {
                self.pending_restore_checkpoint = Some(checkpoint_id);
                Vec::new()
            }
            PlayerAction::DeclareBlockers { assignments } => assignments
                .iter()
                .filter_map(
                    |BlockAssignment {
                         blocker_id,
                         attacker_id,
                     }| {
                        let b = parse_card_id(blocker_id)?;
                        let a = parse_card_id(attacker_id)?;
                        Some((b, a))
                    },
                )
                .collect(),
            _ => Vec::new(),
        }
    }

    fn choose_damage_assignment_order(
        &mut self,
        _player: PlayerId,
        attacker: CardId,
        blockers: &[CardId],
    ) -> Vec<CardId> {
        let attacker_id = crate::ids_codec::card_id_str(attacker);
        let blocker_ids: Vec<String> = blockers
            .iter()
            .map(|&b| crate::ids_codec::card_id_str(b))
            .collect();
        let blocker_cards: Vec<CardDto> = Vec::new(); // Blocker info available from gameView
        let view = self.view();
        self.send_prompt(AgentPromptInner::ChooseDamageAssignmentOrder {
            game_view: view,
            attacker_id,
            blocker_ids,
            blocker_cards,
        });
        match self.recv_action() {
            PlayerAction::DamageAssignmentOrderDecision {
                ordered_blocker_ids,
            } => {
                let parsed: Vec<CardId> = ordered_blocker_ids
                    .iter()
                    .filter_map(|s| parse_card_id(s))
                    .collect();
                if parsed.len() == blockers.len() {
                    parsed
                } else {
                    blockers.to_vec()
                }
            }
            _ => blockers.to_vec(),
        }
    }

    fn choose_target_player(&mut self, _player: PlayerId, valid: &[PlayerId]) -> Option<PlayerId> {
        let valid_player_ids = Self::player_ids(valid);
        self.send_prompt(AgentPromptInner::ChooseTargetPlayer {
            game_view: self.view(),
            valid_player_ids,
        });
        self.recv_player_choice_or_first(valid)
    }

    fn choose_target_card(&mut self, _player: PlayerId, valid: &[CardId]) -> Option<CardId> {
        let valid_card_ids = Self::card_ids(valid);
        let mut view = self.view();
        Self::mark_battlefield_choosable(&mut view, &valid_card_ids);
        self.send_prompt(AgentPromptInner::ChooseTargetCard {
            game_view: view,
            valid_card_ids,
        });
        self.recv_card_choice_or_first(valid)
    }

    fn choose_target_card_from_zone(
        &mut self,
        _player: PlayerId,
        zone: ZoneType,
        valid: &[CardId],
    ) -> Option<CardId> {
        let valid_card_ids = Self::card_ids(valid);
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
        self.recv_card_choice_or_first(valid)
    }

    fn choose_target_any(
        &mut self,
        _player: PlayerId,
        valid_players: &[PlayerId],
        valid_cards: &[CardId],
    ) -> TargetChoice {
        let valid_player_ids = Self::player_ids(valid_players);
        let valid_card_ids = Self::card_ids(valid_cards);
        let mut view = self.view();
        Self::mark_battlefield_choosable(&mut view, &valid_card_ids);
        self.send_prompt(AgentPromptInner::ChooseTargetAny {
            game_view: view,
            valid_player_ids,
            valid_card_ids,
        });
        match self.recv_action() {
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

    fn choose_sacrifice(&mut self, _player: PlayerId, valid: &[CardId]) -> Option<CardId> {
        let valid_card_ids = Self::card_ids(valid);
        let mut view = self.view();
        Self::mark_battlefield_choosable(&mut view, &valid_card_ids);
        self.send_prompt(AgentPromptInner::ChooseTargetCard {
            game_view: view,
            valid_card_ids,
        });
        self.recv_card_choice_or_first(valid)
    }

    fn on_library_peek(&mut self, game: &forge_engine_core::game::GameState, cards: &[CardId]) {
        self.peeked_library_cards = cards
            .iter()
            .map(|&id| card_to_dto(game, id, &[], &[], "library"))
            .collect();
    }

    fn choose_scry(&mut self, _player: PlayerId, cards: &[CardId]) -> Vec<CardId> {
        let card_ids = Self::card_ids(cards);
        let peeked = std::mem::take(&mut self.peeked_library_cards);
        self.send_prompt(AgentPromptInner::Scry {
            game_view: self.view(),
            card_ids,
            cards: peeked,
        });
        match self.recv_action() {
            PlayerAction::ScryDecision { bottom_card_ids } => bottom_card_ids
                .iter()
                .filter_map(|id| parse_card_id(id))
                .collect(),
            _ => vec![],
        }
    }

    fn choose_surveil(&mut self, _player: PlayerId, cards: &[CardId]) -> Vec<CardId> {
        let card_ids = Self::card_ids(cards);
        let peeked = std::mem::take(&mut self.peeked_library_cards);
        self.send_prompt(AgentPromptInner::Surveil {
            game_view: self.view(),
            card_ids,
            cards: peeked,
        });
        match self.recv_action() {
            PlayerAction::SurveilDecision { graveyard_card_ids } => graveyard_card_ids
                .iter()
                .filter_map(|id| parse_card_id(id))
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
        let card_ids = Self::card_ids(valid);
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
                .filter_map(|id| parse_card_id(id))
                .collect(),
            _ => valid.iter().copied().take(max).collect(),
        }
    }

    fn choose_discard(&mut self, _player: PlayerId, hand: &[CardId], num: usize) -> Vec<CardId> {
        let hand_card_ids = Self::card_ids(hand);
        self.send_prompt(AgentPromptInner::ChooseDiscard {
            game_view: self.view(),
            hand_card_ids,
            num_to_discard: num,
        });
        match self.recv_action() {
            PlayerAction::DiscardDecision { discarded_card_ids } => discarded_card_ids
                .iter()
                .filter_map(|id| parse_card_id(id))
                .collect(),
            _ => hand.iter().copied().take(num).collect(),
        }
    }

    fn choose_target_spell(&mut self, _player: PlayerId, valid: &[u32]) -> Option<u32> {
        let valid_spell_ids: Vec<String> = valid.iter().map(|&id| stack_id_str(id)).collect();
        self.send_prompt(AgentPromptInner::ChooseTargetSpell {
            game_view: self.view(),
            valid_spell_ids,
        });
        self.recv_spell_choice_or_first(valid)
    }

    fn choose_mode(
        &mut self,
        _player: PlayerId,
        descriptions: &[String],
        min: usize,
        max: usize,
        card_name: Option<&str>,
    ) -> Vec<usize> {
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

    fn choose_optional_trigger(
        &mut self,
        _player: PlayerId,
        description: &str,
        card_name: Option<&str>,
        _api: Option<&str>,
    ) -> bool {
        self.send_prompt(AgentPromptInner::ChooseOptionalTrigger {
            game_view: self.view(),
            description: description.to_string(),
            source_card_name: card_name.map(String::from),
            prompt_kind: Some("optional_trigger".to_string()),
            option_labels: Some(vec!["Decline".to_string(), "Accept".to_string()]),
            mode: None,
            api: None,
        });
        match self.recv_action() {
            PlayerAction::OptionalTriggerDecision { accept } => accept,
            _ => true,
        }
    }

    fn confirm_action(
        &mut self,
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
        self.send_prompt(AgentPromptInner::ChooseOptionalTrigger {
            game_view: self.view(),
            description: message.to_string(),
            source_card_name: card_name.map(String::from),
            prompt_kind: Some("confirm_action".to_string()),
            option_labels: Some(option_labels),
            mode: mode.map(String::from),
            api: api.map(String::from),
        });
        match self.recv_action() {
            PlayerAction::OptionalTriggerDecision { accept } => accept,
            _ => false,
        }
    }

    fn confirm_payment(
        &mut self,
        _player: PlayerId,
        cost_kind: &str,
        message: &str,
        card_name: Option<&str>,
        api: Option<&str>,
    ) -> bool {
        self.send_prompt(AgentPromptInner::ChooseOptionalTrigger {
            game_view: self.view(),
            description: message.to_string(),
            source_card_name: card_name.map(String::from),
            prompt_kind: Some("confirm_payment".to_string()),
            option_labels: Some(vec!["Decline".to_string(), "Accept".to_string()]),
            mode: Some(cost_kind.to_string()),
            api: api.map(String::from),
        });
        match self.recv_action() {
            PlayerAction::OptionalTriggerDecision { accept } => accept,
            _ => false,
        }
    }

    fn choose_binary(
        &mut self,
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
        self.send_prompt(AgentPromptInner::ChooseOptionalTrigger {
            game_view: self.view(),
            description: question.to_string(),
            source_card_name: card_name.map(String::from),
            prompt_kind: Some("choose_binary".to_string()),
            option_labels: Some(vec![right.to_string(), left.to_string()]),
            mode: Some(kind.as_str().to_string()),
            api: api.map(String::from),
        });
        match self.recv_action() {
            PlayerAction::OptionalTriggerDecision { accept } => accept,
            _ => false,
        }
    }

    fn choose_phyrexian_pay_life(
        &mut self,
        _player: PlayerId,
        color: &str,
        card_name: Option<&str>,
    ) -> bool {
        self.send_prompt(AgentPromptInner::ChoosePhyrexian {
            game_view: self.view(),
            phyrexian_color: color.to_string(),
            source_card_name: card_name.map(String::from),
        });
        match self.recv_action() {
            PlayerAction::PhyrexianDecision { pay_life } => pay_life,
            _ => false,
        }
    }

    fn choose_kicker(
        &mut self,
        _player: PlayerId,
        kicker_cost: &str,
        card_name: Option<&str>,
    ) -> bool {
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

    fn choose_buyback(
        &mut self,
        _player: PlayerId,
        buyback_cost: &str,
        card_name: Option<&str>,
    ) -> bool {
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

    fn choose_multikicker(
        &mut self,
        _player: PlayerId,
        cost: &str,
        max_kicks: u32,
        card_name: Option<&str>,
    ) -> u32 {
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

    fn choose_replicate(
        &mut self,
        _player: PlayerId,
        cost: &str,
        max_replicates: u32,
        card_name: Option<&str>,
    ) -> u32 {
        self.send_prompt(AgentPromptInner::ChooseReplicate {
            game_view: self.view(),
            cost: cost.to_string(),
            max_replicates,
            source_card_name: card_name.map(String::from),
        });
        match self.recv_action() {
            PlayerAction::ReplicateDecision { replicate_count } => {
                replicate_count.min(max_replicates)
            }
            _ => 0,
        }
    }

    fn choose_alternative_cost(
        &mut self,
        _player: PlayerId,
        options: &[String],
        card_name: Option<&str>,
    ) -> usize {
        self.send_prompt(AgentPromptInner::ChooseAlternativeCost {
            game_view: self.view(),
            options: options.to_vec(),
            source_card_name: card_name.map(String::from),
        });
        match self.recv_action() {
            PlayerAction::AlternativeCostDecision { chosen_index } => {
                chosen_index.min(options.len().saturating_sub(1))
            }
            _ => 0,
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
        let valid_card_ids = Self::card_ids(valid);
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
                .filter_map(|id| parse_card_id(id))
                .collect(),
            _ => valid.iter().copied().take(max).collect(),
        }
    }

    fn choose_type(
        &mut self,
        _player: PlayerId,
        type_category: &str,
        valid_types: &[String],
    ) -> Option<String> {
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

    fn choose_x_value(&mut self, _player: PlayerId, max_x: u32, card_name: Option<&str>) -> u32 {
        self.send_prompt(AgentPromptInner::ChooseNumber {
            game_view: self.view(),
            min: 0,
            max: max_x as i32,
            source_card_name: card_name.map(String::from),
        });
        match self.recv_action() {
            PlayerAction::NumberDecision { chosen_number } => {
                chosen_number.unwrap_or(max_x as i32).max(0) as u32
            }
            _ => max_x,
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

    fn pay_combat_cost(
        &mut self,
        _player: PlayerId,
        attacker: CardId,
        cost: i32,
        description: &str,
        tappable_lands: &[CardId],
        untappable_lands: &[CardId],
        mana_pool_total: i32,
    ) -> CombatCostAction {
        let attacker_id = card_id_str(attacker);
        let attacker_name = self
            .latest_view
            .as_ref()
            .and_then(|v| v.battlefield.iter().find(|c| c.id == attacker_id))
            .map(|c| c.name.clone())
            .unwrap_or_default();
        let tappable_land_ids = Self::card_ids(tappable_lands);
        let untappable_land_ids = Self::card_ids(untappable_lands);

        self.send_prompt(AgentPromptInner::PayCombatCost {
            game_view: self.view(),
            attacker_id,
            attacker_name,
            cost,
            description: description.to_string(),
            tappable_land_ids,
            untappable_land_ids,
            mana_pool_total,
        });
        match self.recv_action() {
            PlayerAction::TapLand { card_id } => parse_card_id(&card_id)
                .map(CombatCostAction::TapLand)
                .unwrap_or(CombatCostAction::Decline),
            PlayerAction::UntapLand { card_id } => parse_card_id(&card_id)
                .map(CombatCostAction::UntapLand)
                .unwrap_or(CombatCostAction::Decline),
            PlayerAction::PayCombatCost => CombatCostAction::Pay,
            _ => CombatCostAction::Decline,
        }
    }

    fn choose_delve(
        &mut self,
        _player: PlayerId,
        valid: &[CardId],
        max: usize,
        card_name: Option<&str>,
    ) -> Vec<CardId> {
        let valid_ids = Self::card_ids(valid);
        // Build CardDtos for the graveyard cards
        let zone_cards: Vec<_> = valid
            .iter()
            .filter_map(|&cid| {
                self.latest_view.as_ref().and_then(|v| {
                    v.graveyard
                        .iter()
                        .chain(v.opponent_graveyard.iter())
                        .find(|c| c.id == card_id_str(cid))
                        .cloned()
                })
            })
            .collect();

        self.send_prompt(AgentPromptInner::ChooseDelve {
            game_view: self.view(),
            valid_card_ids: valid_ids,
            zone_cards,
            max_cards: max,
            source_card_name: card_name.map(|s| s.to_string()),
        });
        match self.recv_action() {
            PlayerAction::DelveDecision { chosen_card_ids } => chosen_card_ids
                .iter()
                .filter_map(|id| parse_card_id(id))
                .filter(|cid| valid.contains(cid))
                .take(max)
                .collect(),
            _ => vec![],
        }
    }

    fn choose_improvise(
        &mut self,
        _player: PlayerId,
        untapped_artifacts: &[CardId],
        remaining_cost: &forge_foundation::ManaCost,
        card_name: Option<&str>,
    ) -> Vec<CardId> {
        let valid_ids = Self::card_ids(untapped_artifacts);

        self.send_prompt(AgentPromptInner::ChooseImprovise {
            game_view: self.view(),
            valid_card_ids: valid_ids,
            remaining_cost: remaining_cost.to_string(),
            source_card_name: card_name.map(|s| s.to_string()),
        });
        match self.recv_action() {
            PlayerAction::ImproviseDecision { chosen_card_ids } => chosen_card_ids
                .iter()
                .filter_map(|id| parse_card_id(id))
                .filter(|cid| untapped_artifacts.contains(cid))
                .collect(),
            _ => vec![],
        }
    }

    fn choose_convoke(
        &mut self,
        _player: PlayerId,
        untapped_creatures: &[CardId],
        remaining_cost: &forge_foundation::ManaCost,
        card_name: Option<&str>,
    ) -> Vec<CardId> {
        let valid_ids = Self::card_ids(untapped_creatures);

        self.send_prompt(AgentPromptInner::ChooseConvoke {
            game_view: self.view(),
            valid_card_ids: valid_ids,
            remaining_cost: remaining_cost.to_string(),
            source_card_name: card_name.map(|s| s.to_string()),
        });
        match self.recv_action() {
            PlayerAction::ConvokeDecision { chosen_card_ids } => chosen_card_ids
                .iter()
                .filter_map(|id| parse_card_id(id))
                .filter(|cid| untapped_creatures.contains(cid))
                .collect(),
            _ => vec![],
        }
    }

    fn pay_mana_cost(
        &mut self,
        _player: PlayerId,
        card_id: CardId,
        card_name: &str,
        mana_cost: &str,
        tappable_lands: &[CardId],
        untappable_lands: &[CardId],
        mana_pool: &ManaPool,
    ) -> ManaCostAction {
        let card_id_s = card_id_str(card_id);
        let tappable_land_ids = Self::card_ids(tappable_lands);
        let untappable_land_ids = Self::card_ids(untappable_lands);

        self.send_prompt(AgentPromptInner::PayManaCost {
            game_view: self.view(),
            card_id: card_id_s,
            card_name: card_name.to_string(),
            mana_cost: mana_cost.to_string(),
            tappable_land_ids,
            untappable_land_ids,
            mana_pool_total: mana_pool.total(),
        });
        match self.recv_action() {
            PlayerAction::TapLand { card_id } => parse_card_id(&card_id)
                .map(ManaCostAction::TapLand)
                .unwrap_or(ManaCostAction::Cancel),
            PlayerAction::UntapLand { card_id } => parse_card_id(&card_id)
                .map(ManaCostAction::UntapLand)
                .unwrap_or(ManaCostAction::Cancel),
            PlayerAction::PayManaCost => ManaCostAction::Pay,
            _ => ManaCostAction::Cancel,
        }
    }

    fn is_human(&self) -> bool {
        true
    }

    fn specify_mana_combo(
        &mut self,
        _player: PlayerId,
        available_colors: &[String],
        amount: usize,
        card_name: Option<&str>,
    ) -> Vec<String> {
        self.send_prompt(AgentPromptInner::SpecifyManaCombo {
            game_view: self.view(),
            available_colors: available_colors.to_vec(),
            amount,
            source_card_name: card_name.map(String::from),
        });
        let action = self.recv_action();
        match action {
            PlayerAction::ManaComboDecision { chosen_colors } => {
                // Validate: only return valid colors, pad/truncate to amount
                let mut result: Vec<String> = chosen_colors
                    .into_iter()
                    .filter(|c| available_colors.contains(c))
                    .collect();
                // Pad with first available color if needed
                while result.len() < amount {
                    result.push(available_colors.first().cloned().unwrap_or("C".to_string()));
                }
                result.truncate(amount);
                result
            }
            _ => {
                // Default: all first color
                vec![available_colors.first().cloned().unwrap_or("C".to_string()); amount]
            }
        }
    }

    fn choose_land_or_spell(&mut self, _player: PlayerId) -> Option<bool> {
        None
    }

    fn notify(&mut self, message: &str) {
        if let Some(tx) = &self.notify_tx {
            let _ = tx.send(GameLogEntryDto::from_message(message));
        }
    }

    fn notify_event(&mut self, event: GameLogEvent) {
        if let Some(tx) = &self.notify_tx {
            let _ = tx.send(GameLogEntryDto::from_event(event));
        }
    }

    fn notify_snapshot_created(&mut self, checkpoint_id: u64, label: &str) {
        if let (Some(tx), Some(view)) = (&self.snapshot_tx, self.latest_view.clone()) {
            let _ = tx.send(GameSnapshotEventDto::new(
                checkpoint_id,
                label.to_string(),
                view,
            ));
        }
    }

    fn take_restore_request(&mut self) -> Option<u64> {
        self.pending_restore_checkpoint.take()
    }

    fn notify_card_played(
        &mut self,
        player: PlayerId,
        card_id: CardId,
        card_name: &str,
        set_code: &str,
    ) {
        self.pending_display_events.push(DisplayEvent::CardPlayed {
            card_id: card_id_str(card_id),
            card_name: card_name.to_string(),
            set_code: set_code.to_string(),
            player_id: player_id_str(player),
        });
        // Flush immediately so the frontend receives one event per card play.
        self.send_prompt(AgentPromptInner::StateUpdate {
            game_view: self.view(),
        });
    }

    fn notify_turn_changed(&mut self, active_player: PlayerId, turn_number: u32) {
        let player_id = player_id_str(active_player);
        let active_player_name = self
            .latest_view
            .as_ref()
            .and_then(|v| v.players.iter().find(|p| p.id == player_id))
            .map(|p| p.name.clone())
            .unwrap_or_else(|| format!("Player {}", active_player.0));
        if let Some(tx) = &self.notify_tx {
            let _ = tx.send(GameLogEntryDto::from_event(
                forge_engine_core::agent::GameLogEvent::rule(format!(
                    "TURN {} — {}",
                    turn_number, active_player_name
                ))
                .with_player(active_player),
            ));
        }
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
