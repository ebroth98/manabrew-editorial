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
use crate::game_view_dto::{CardDto, GameViewDto};
use crate::ids_codec::{card_id_str, parse_card_id, parse_player_id, player_id_str};
use crate::prompt::{
    ActivatableAbilityInfo, AgentPrompt, AgentPromptInner, DisplayEvent, PlayOptionDto,
    PlayerAction,
};

mod choices;
mod combat;
mod costs;
mod library;
mod targeting;

/// A PlayerAgent that sends prompts to the frontend and blocks waiting for a response.
pub struct TauriAgent {
    pub player_id: PlayerId,
    pub game_id: String,
    prompt_sink: PromptSink,
    pub response_rx: mpsc::Receiver<PlayerAction>,
    pub notify_tx: Option<mpsc::Sender<GameLogEntryDto>>,
    pub snapshot_tx: Option<mpsc::Sender<GameSnapshotEventDto>>,
    response_timeout: Option<Duration>,
    pub(crate) latest_view: Option<GameViewDto>,
    /// Display events accumulated between prompts — drained and attached to each outgoing prompt.
    pub(crate) pending_display_events: Vec<DisplayEvent>,
    /// Card DTOs pre-built by on_library_peek() for Scry/Surveil/Dig prompts.
    pub(crate) peeked_library_cards: Vec<CardDto>,
    /// Cached per-ability descriptions and is_mana_ability flags, populated in snapshot_state.
    /// Key: (card_id.0, ability_index) → (description, is_mana_ability)
    ability_descriptions: std::collections::HashMap<(u32, usize), (String, bool)>,
    pub(crate) pending_restore_checkpoint: Option<u64>,
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
    pub(crate) fn send_prompt(&mut self, inner: AgentPromptInner) {
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

    pub(crate) fn recv_action(&self) -> PlayerAction {
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

    pub(crate) fn view(&self) -> GameViewDto {
        self.latest_view.clone().unwrap_or_else(|| {
            // Fallback: empty view
            GameViewDto::empty(self.game_id.clone())
        })
    }

    pub(crate) fn card_ids(cards: &[CardId]) -> Vec<String> {
        cards.iter().map(|&c| card_id_str(c)).collect()
    }

    pub(crate) fn player_ids(players: &[PlayerId]) -> Vec<String> {
        players.iter().map(|&p| player_id_str(p)).collect()
    }

    pub(crate) fn defender_ids_to_dtos(
        defenders: &[DefenderId],
    ) -> Vec<crate::prompt::DefenderIdDto> {
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

    fn play_option_to_dto(play: &PlayOption) -> PlayOptionDto {
        use forge_engine_core::agent::PlayCardMode;
        let card_id = card_id_str(play.card_id);
        let (mode, mode_label) = match &play.mode {
            PlayCardMode::Normal => ("normal".to_string(), "Cast normally".to_string()),
            PlayCardMode::Alternative(alt) => {
                let name = format!("{:?}", alt);
                (
                    format!("alternative:{}", name.to_lowercase()),
                    format!("Cast with {}", name),
                )
            }
            PlayCardMode::GainLifeAlt => (
                "gainLifeAlt".to_string(),
                "Cast with alternate cost".to_string(),
            ),
            PlayCardMode::StaticAlternative => (
                "staticAlternative".to_string(),
                "Cast with alternative cost".to_string(),
            ),
            PlayCardMode::ForetellExile => (
                "foretellExile".to_string(),
                "Foretell (exile face-down)".to_string(),
            ),
        };
        PlayOptionDto {
            card_id,
            mode,
            mode_label,
        }
    }

    fn parse_play_mode(mode_str: &str) -> Option<forge_engine_core::agent::PlayCardMode> {
        use forge_engine_core::agent::PlayCardMode;
        use forge_engine_core::spellability::AlternativeCost;
        match mode_str {
            "normal" => Some(PlayCardMode::Normal),
            "gainLifeAlt" => Some(PlayCardMode::GainLifeAlt),
            "foretellExile" => Some(PlayCardMode::ForetellExile),
            s if s.starts_with("alternative:") => {
                let alt_name = &s["alternative:".len()..];
                let alt = match alt_name {
                    "flashback" => AlternativeCost::Flashback,
                    "evoke" => AlternativeCost::Evoke,
                    "dash" => AlternativeCost::Dash,
                    "escape" => AlternativeCost::Escape,
                    "madness" => AlternativeCost::Madness,
                    "overload" => AlternativeCost::Overload,
                    "spectacle" => AlternativeCost::Spectacle,
                    "emerge" => AlternativeCost::Emerge,
                    "blitz" => AlternativeCost::Blitz,
                    "foretell" => AlternativeCost::Foretell,
                    "suspend" => AlternativeCost::Suspend,
                    _ => return None,
                };
                Some(PlayCardMode::Alternative(alt))
            }
            _ => None,
        }
    }

    pub(crate) fn parse_defender_id(id: &str, possible: &[DefenderId]) -> Option<DefenderId> {
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

    pub(crate) fn mark_battlefield_choosable(view: &mut GameViewDto, valid_card_ids: &[String]) {
        for card in &mut view.battlefield {
            card.is_choosable = valid_card_ids.contains(&card.id);
        }
    }

    pub(crate) fn recv_card_choice_or_first(&self, valid: &[CardId]) -> Option<CardId> {
        match self.recv_action() {
            PlayerAction::TargetCard { card_id } => card_id.and_then(|id| parse_card_id(&id)),
            _ => valid.first().copied(),
        }
    }

    pub(crate) fn recv_player_choice_or_first(&self, valid: &[PlayerId]) -> Option<PlayerId> {
        match self.recv_action() {
            PlayerAction::TargetPlayer { player_id } => {
                player_id.and_then(|id| parse_player_id(&id))
            }
            _ => valid.first().copied(),
        }
    }

    pub(crate) fn recv_spell_choice_or_first(&self, valid: &[u32]) -> Option<u32> {
        match self.recv_action() {
            PlayerAction::TargetSpell { spell_id } => {
                spell_id.and_then(|id| crate::ids_codec::parse_stack_id(&id))
            }
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
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| ab.ability_text.clone());
                self.ability_descriptions
                    .insert((card_id.0, ab.ability_index), (desc, ab.is_mana_ability));
            }
        }
    }

    fn mulligan_decision(
        &mut self,
        player: PlayerId,
        hand: &[CardId],
        mulligan_count: u32,
    ) -> bool {
        choices::mulligan_decision(self, player, hand, mulligan_count)
    }

    fn choose_cards_to_bottom(
        &mut self,
        player: PlayerId,
        hand: &[CardId],
        count: usize,
    ) -> Vec<CardId> {
        choices::choose_cards_to_bottom(self, player, hand, count)
    }

    fn choose_action(
        &mut self,
        _player: PlayerId,
        playable: &[PlayOption],
        tappable_lands: &[CardId],
        untappable_lands: &[CardId],
        activatable: &[(CardId, usize)],
    ) -> MainPhaseAction {
        let playable_card_ids: Vec<String> = playable
            .iter()
            .map(|play| card_id_str(play.card_id))
            .collect();
        let playable_options: Vec<PlayOptionDto> = playable
            .iter()
            .map(|play| Self::play_option_to_dto(play))
            .collect();
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
            playable_options,
            tappable_land_ids,
            untappable_land_ids,
            activatable_ability_ids,
        });
        match self.recv_action() {
            PlayerAction::RestoreSnapshot { checkpoint_id } => {
                self.pending_restore_checkpoint = Some(checkpoint_id);
                MainPhaseAction::Pass
            }
            PlayerAction::PlayCard { card_id, mode } => card_id
                .and_then(|id| {
                    let cid = parse_card_id(&id)?;
                    // If mode is specified, find the exact PlayOption matching card+mode
                    if let Some(mode_str) = &mode {
                        if let Some(parsed_mode) = Self::parse_play_mode(mode_str) {
                            return playable
                                .iter()
                                .copied()
                                .find(|play| play.card_id == cid && play.mode == parsed_mode);
                        }
                    }
                    // Fallback: first matching PlayOption for this card
                    playable.iter().copied().find(|play| play.card_id == cid)
                })
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
        player: PlayerId,
        available: &[CardId],
        possible_defenders: &[DefenderId],
    ) -> Vec<(CardId, DefenderId)> {
        combat::choose_attackers(self, player, available, possible_defenders)
    }

    fn choose_blockers(
        &mut self,
        player: PlayerId,
        attackers: &[CardId],
        available_blockers: &[CardId],
        max_blockers: Option<usize>,
    ) -> Vec<(CardId, CardId)> {
        combat::choose_blockers(self, player, attackers, available_blockers, max_blockers)
    }

    fn choose_damage_assignment_order(
        &mut self,
        player: PlayerId,
        attacker: CardId,
        blockers: &[CardId],
    ) -> Vec<CardId> {
        combat::choose_damage_assignment_order(self, player, attacker, blockers)
    }

    fn assign_combat_damage(
        &mut self,
        game: &GameState,
        player: PlayerId,
        attacker: CardId,
        blockers_in_order: &[CardId],
        defender_id: Option<DefenderId>,
        damage_to_assign: i32,
    ) -> Vec<(Option<CardId>, i32)> {
        let attacker_has_deathtouch = game.card(attacker).has_deathtouch();
        combat::choose_combat_damage_assignment(
            self,
            player,
            attacker,
            blockers_in_order,
            defender_id,
            damage_to_assign,
            attacker_has_deathtouch,
        )
    }

    fn choose_target_player(&mut self, player: PlayerId, valid: &[PlayerId]) -> Option<PlayerId> {
        targeting::choose_target_player(self, player, valid)
    }

    fn choose_target_card(&mut self, player: PlayerId, valid: &[CardId]) -> Option<CardId> {
        targeting::choose_target_card(self, player, valid)
    }

    fn choose_target_card_from_zone(
        &mut self,
        player: PlayerId,
        zone: ZoneType,
        valid: &[CardId],
    ) -> Option<CardId> {
        targeting::choose_target_card_from_zone(self, player, zone, valid)
    }

    fn choose_target_any(
        &mut self,
        player: PlayerId,
        valid_players: &[PlayerId],
        valid_cards: &[CardId],
    ) -> TargetChoice {
        targeting::choose_target_any(self, player, valid_players, valid_cards)
    }

    fn choose_sacrifice(&mut self, player: PlayerId, valid: &[CardId]) -> Option<CardId> {
        targeting::choose_sacrifice(self, player, valid)
    }

    fn on_library_peek(&mut self, game: &forge_engine_core::game::GameState, cards: &[CardId]) {
        library::on_library_peek(self, game, cards)
    }

    fn choose_scry(&mut self, player: PlayerId, cards: &[CardId]) -> Vec<CardId> {
        library::choose_scry(self, player, cards)
    }

    fn choose_surveil(&mut self, player: PlayerId, cards: &[CardId]) -> Vec<CardId> {
        library::choose_surveil(self, player, cards)
    }

    fn choose_dig(
        &mut self,
        player: PlayerId,
        valid: &[CardId],
        max: usize,
        optional: bool,
    ) -> Vec<CardId> {
        library::choose_dig(self, player, valid, max, optional)
    }

    fn choose_discard(&mut self, player: PlayerId, hand: &[CardId], num: usize) -> Vec<CardId> {
        choices::choose_discard(self, player, hand, num)
    }

    fn choose_target_spell(&mut self, player: PlayerId, valid: &[u32]) -> Option<u32> {
        targeting::choose_target_spell(self, player, valid)
    }

    fn choose_mode(
        &mut self,
        player: PlayerId,
        descriptions: &[String],
        min: usize,
        max: usize,
        card_name: Option<&str>,
    ) -> Vec<usize> {
        choices::choose_mode(self, player, descriptions, min, max, card_name)
    }

    fn choose_single_replacement_effect(
        &mut self,
        player: PlayerId,
        descriptions: &[String],
    ) -> usize {
        choices::choose_single_replacement_effect(self, player, descriptions)
    }

    fn choose_optional_trigger(
        &mut self,
        player: PlayerId,
        description: &str,
        card_name: Option<&str>,
        api: Option<forge_engine_core::ability::api_type::ApiType>,
    ) -> bool {
        choices::choose_optional_trigger(self, player, description, card_name, api)
    }

    fn confirm_action(
        &mut self,
        player: PlayerId,
        mode: Option<&str>,
        message: &str,
        options: &[String],
        card_name: Option<&str>,
        api: Option<forge_engine_core::ability::api_type::ApiType>,
    ) -> bool {
        choices::confirm_action(self, player, mode, message, options, card_name, api)
    }

    fn confirm_payment(
        &mut self,
        player: PlayerId,
        cost_kind: &str,
        message: &str,
        card_name: Option<&str>,
        api: Option<forge_engine_core::ability::api_type::ApiType>,
    ) -> bool {
        choices::confirm_payment(self, player, cost_kind, message, card_name, api)
    }

    fn choose_binary(
        &mut self,
        player: PlayerId,
        question: &str,
        kind: BinaryChoiceKind,
        default_choice: Option<bool>,
        card_name: Option<&str>,
        api: Option<forge_engine_core::ability::api_type::ApiType>,
    ) -> bool {
        choices::choose_binary(self, player, question, kind, default_choice, card_name, api)
    }

    fn choose_phyrexian_pay_life(
        &mut self,
        player: PlayerId,
        color: &str,
        card_name: Option<&str>,
    ) -> bool {
        costs::choose_phyrexian_pay_life(self, player, color, card_name)
    }

    fn choose_kicker(
        &mut self,
        player: PlayerId,
        kicker_cost: &str,
        card_name: Option<&str>,
    ) -> bool {
        costs::choose_kicker(self, player, kicker_cost, card_name)
    }

    fn choose_buyback(
        &mut self,
        player: PlayerId,
        buyback_cost: &str,
        card_name: Option<&str>,
    ) -> bool {
        costs::choose_buyback(self, player, buyback_cost, card_name)
    }

    fn choose_multikicker(
        &mut self,
        player: PlayerId,
        cost: &str,
        max_kicks: u32,
        card_name: Option<&str>,
    ) -> u32 {
        costs::choose_multikicker(self, player, cost, max_kicks, card_name)
    }

    fn choose_replicate(
        &mut self,
        player: PlayerId,
        cost: &str,
        max_replicates: u32,
        card_name: Option<&str>,
    ) -> u32 {
        costs::choose_replicate(self, player, cost, max_replicates, card_name)
    }

    fn choose_alternative_cost(
        &mut self,
        player: PlayerId,
        options: &[String],
        card_name: Option<&str>,
    ) -> usize {
        costs::choose_alternative_cost(self, player, options, card_name)
    }

    fn choose_color(&mut self, player: PlayerId, valid_colors: &[String]) -> Option<String> {
        choices::choose_color(self, player, valid_colors)
    }

    fn choose_cards_for_effect(
        &mut self,
        player: PlayerId,
        valid: &[CardId],
        min: usize,
        max: usize,
    ) -> Vec<CardId> {
        choices::choose_cards_for_effect(self, player, valid, min, max)
    }

    fn choose_single_card_for_zone_change(
        &mut self,
        player: PlayerId,
        valid: &[CardId],
        select_prompt: &str,
        is_optional: bool,
    ) -> Option<CardId> {
        choices::choose_single_card_for_zone_change(self, player, valid, select_prompt, is_optional)
    }

    fn choose_cards_for_zone_change(
        &mut self,
        player: PlayerId,
        valid: &[CardId],
        min: usize,
        max: usize,
        select_prompt: &str,
    ) -> Vec<CardId> {
        choices::choose_cards_for_zone_change(self, player, valid, min, max, select_prompt)
    }

    fn choose_type(
        &mut self,
        player: PlayerId,
        type_category: &str,
        valid_types: &[String],
    ) -> Option<String> {
        choices::choose_type(self, player, type_category, valid_types)
    }

    fn choose_card_name(&mut self, player: PlayerId, valid_names: &[String]) -> Option<String> {
        choices::choose_card_name(self, player, valid_names)
    }

    fn choose_x_value(&mut self, player: PlayerId, max_x: u32, card_name: Option<&str>) -> u32 {
        choices::choose_x_value(self, player, max_x, card_name)
    }

    fn choose_number(&mut self, player: PlayerId, min: i32, max: i32) -> Option<i32> {
        choices::choose_number(self, player, min, max)
    }

    fn pay_combat_cost(
        &mut self,
        player: PlayerId,
        attacker: CardId,
        cost: i32,
        description: &str,
        tappable_lands: &[CardId],
        untappable_lands: &[CardId],
        mana_pool_total: i32,
    ) -> CombatCostAction {
        combat::pay_combat_cost(
            self,
            player,
            attacker,
            cost,
            description,
            tappable_lands,
            untappable_lands,
            mana_pool_total,
        )
    }

    fn choose_delve(
        &mut self,
        player: PlayerId,
        valid: &[CardId],
        max: usize,
        card_name: Option<&str>,
    ) -> Vec<CardId> {
        costs::choose_delve(self, player, valid, max, card_name)
    }

    fn choose_improvise(
        &mut self,
        player: PlayerId,
        untapped_artifacts: &[CardId],
        remaining_cost: &forge_foundation::ManaCost,
        card_name: Option<&str>,
    ) -> Vec<CardId> {
        costs::choose_improvise(self, player, untapped_artifacts, remaining_cost, card_name)
    }

    fn choose_convoke(
        &mut self,
        player: PlayerId,
        untapped_creatures: &[CardId],
        remaining_cost: &forge_foundation::ManaCost,
        card_name: Option<&str>,
    ) -> Vec<CardId> {
        costs::choose_convoke(self, player, untapped_creatures, remaining_cost, card_name)
    }

    fn pay_mana_cost(
        &mut self,
        player: PlayerId,
        card_id: CardId,
        card_name: &str,
        mana_cost: &str,
        tappable_lands: &[CardId],
        untappable_lands: &[CardId],
        mana_pool: &ManaPool,
    ) -> ManaCostAction {
        costs::pay_mana_cost(
            self,
            player,
            card_id,
            card_name,
            mana_cost,
            tappable_lands,
            untappable_lands,
            mana_pool,
        )
    }

    fn is_human(&self) -> bool {
        !matches!(self.prompt_sink, PromptSink::Ai(_))
    }

    fn specify_mana_combo(
        &mut self,
        player: PlayerId,
        available_colors: &[String],
        amount: usize,
        card_name: Option<&str>,
    ) -> Vec<String> {
        costs::specify_mana_combo(self, player, available_colors, amount, card_name)
    }

    fn exert_attackers(&mut self, player: PlayerId, attackers: &[CardId]) -> Vec<CardId> {
        combat::exert_attackers(self, player, attackers)
    }

    fn enlist_attackers(&mut self, player: PlayerId, attackers: &[CardId]) -> Vec<CardId> {
        combat::enlist_attackers(self, player, attackers)
    }

    fn choose_reorder_library(&mut self, player: PlayerId, cards: &[CardId]) -> Vec<CardId> {
        library::choose_reorder_library(self, player, cards)
    }

    fn choose_explore_put_in_graveyard(
        &mut self,
        player: PlayerId,
        revealed_card_name: &str,
        revealed_cmc: i32,
        mana_producing_lands: usize,
        predicted_mana: usize,
        lands_in_hand: usize,
    ) -> bool {
        choices::choose_explore_put_in_graveyard(
            self,
            player,
            revealed_card_name,
            revealed_cmc,
            mana_producing_lands,
            predicted_mana,
            lands_in_hand,
        )
    }

    fn help_pay_assist(&mut self, player: PlayerId, card_name: &str, max_generic: u32) -> u32 {
        choices::help_pay_assist(self, player, card_name, max_generic)
    }

    fn choose_random_discard(
        &mut self,
        player: PlayerId,
        hand: &[CardId],
        num: usize,
    ) -> Vec<CardId> {
        choices::choose_random_discard(self, player, hand, num)
    }

    fn choose_land_or_spell(&mut self, player: PlayerId) -> Option<bool> {
        choices::choose_land_or_spell(self, player)
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
