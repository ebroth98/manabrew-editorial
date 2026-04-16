use std::cell::RefCell;
use std::collections::{BTreeSet, HashSet};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use forge_carddb::CardDatabase;
use forge_engine_core::agent::{
    BinaryChoiceKind, GameEntity, ManaCostAction, PlayOption, PlayerAgent,
};
use forge_engine_core::card::CardInstance;
use forge_engine_core::combat::DefenderId;
use forge_engine_core::game::GameState;
use forge_engine_core::game_loop::GameLoop;
use forge_engine_core::ids::{CardId, PlayerId};
use forge_foundation::ZoneType;
use rand::rngs::StdRng;
use rand::SeedableRng;

use crate::callback_fmt::{CallbackArgDisplay, FmtCtx, ParityFormat};
use crate::deterministic_agent::{DeterministicAgent, VerboseMode};
use crate::java_random::JavaRandom;
use crate::parity_card_map::ParityCardMap;
use crate::protocol::{CallbackRecord, DecisionRecord, GameTrace, ParityLogEntry};
use crate::snapshot::snapshot_game;
use crate::utils::decks::{build_deck_from_spec, resolve_deck_spec};

pub const DEFAULT_DECKS_DIR: &str = "preset_decks";

pub(crate) struct ParityObserver {
    shared_log: Arc<Mutex<Vec<ParityLogEntry>>>,
    shared_snapshot_index: Arc<Mutex<usize>>,
}

impl ParityObserver {
    fn new(
        shared_log: Arc<Mutex<Vec<ParityLogEntry>>>,
        shared_snapshot_index: Arc<Mutex<usize>>,
    ) -> Self {
        Self {
            shared_log,
            shared_snapshot_index,
        }
    }

    pub(crate) fn on_callback(
        &self,
        name: &str,
        outcome: &str,
        player: u32,
        turn: u32,
        phase: &str,
        callback_args: Vec<String>,
    ) {
        let choice_logs = crate::parity_log::drain();

        let snapshot_index = *self.shared_snapshot_index.lock().unwrap();
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        self.shared_log
            .lock()
            .unwrap()
            .push(ParityLogEntry::Callback(CallbackRecord {
                snapshot_index,
                turn,
                phase: phase.to_string(),
                player,
                name: name.to_string(),
                outcome: outcome.to_string(),
                args: choice_logs,
                callback_args,
                timestamp_ms,
            }));
    }

    fn mark_snapshot(&self) {
        *self.shared_snapshot_index.lock().unwrap() += 1;
    }
}

struct CapturingAgent {
    player_id: PlayerId,
    inner: DeterministicAgent,
    shared_log: Arc<Mutex<Vec<ParityLogEntry>>>,
    shared_covered_cards: Arc<Mutex<BTreeSet<String>>>,
    parity_observer: Arc<ParityObserver>,
    parity_map: Arc<ParityCardMap>,
    capture_snapshots: bool,
    deep: bool,
    callback_snapshots: bool,
    current_turn: u32,
    current_phase: String,
    last_game_state: Option<GameState>,
    pending_pay_mana_cost_args: Option<Vec<String>>,
    #[allow(dead_code)]
    verbose: VerboseMode,
}

impl CapturingAgent {
    fn new(
        player_id: PlayerId,
        verbose: VerboseMode,
        prefer_actions: bool,
        shared_log: Arc<Mutex<Vec<ParityLogEntry>>>,
        covered: Arc<Mutex<BTreeSet<String>>>,
        snapshot_index: Arc<Mutex<usize>>,
        rng: Rc<RefCell<JavaRandom>>,
        game_rng: Rc<RefCell<JavaRandom>>,
        parity_map: Arc<ParityCardMap>,
        capture_snapshots: bool,
        deep: bool,
        callback_snapshots: bool,
    ) -> Self {
        let observer = Arc::new(ParityObserver::new(Arc::clone(&shared_log), snapshot_index));
        Self {
            player_id,
            inner: DeterministicAgent::new(
                player_id,
                verbose.clone(),
                rng,
                game_rng,
                prefer_actions,
                Arc::clone(&parity_map),
                Some(Arc::clone(&observer)),
            ),
            shared_log,
            shared_covered_cards: covered,
            parity_observer: observer,
            parity_map,
            capture_snapshots,
            deep,
            callback_snapshots,
            current_turn: 0,
            current_phase: "Unknown".to_string(),
            last_game_state: None,
            pending_pay_mana_cost_args: None,
            verbose,
        }
    }

    #[allow(dead_code)]
    fn is_verbose(&self) -> bool {
        self.verbose.is_active(self.current_turn)
    }

    /// Build a formatting context for the current game state.
    /// Returns `None` if no game state has been captured yet.
    fn fmt_ctx(&self) -> Option<FmtCtx<'_>> {
        self.last_game_state.as_ref().map(|game| FmtCtx {
            game,
            parity_map: &self.parity_map,
        })
    }

    fn save_snapshot(&self, kind: &str) {
        if !self.callback_snapshots {
            return;
        }
        let Some(ref game) = self.last_game_state else {
            return;
        };
        let snapshot = snapshot_game(game);
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let mut log = self.shared_log.lock().unwrap();
        log.push(ParityLogEntry::Snapshot(snapshot.clone()));
        log.push(ParityLogEntry::Decision(DecisionRecord {
            turn: snapshot.turn,
            phase: snapshot.phase.clone(),
            deciding_player: self.player_id.0,
            kind: kind.to_string(),
            options: vec![],
            choice: "CALLBACK_ENTRY".to_string(),
            timestamp_ms,
        }));
    }
}

macro_rules! parity_agent_callback {
    ($(fn $name:ident (&mut self $(, $arg:ident : $ty:ty )* ) -> $ret:ty => $kind:expr, defer_if $pat:pat => $pending:ident;)+) => {
        $(
            fn $name(&mut self $(, $arg: $ty)*) -> $ret {
                self.save_snapshot($kind);
                let fmt = self.fmt_ctx();
                let cb_args: Vec<String> = vec![$($arg.callback_arg_display(fmt.as_ref())),*];
                let result = self.inner.$name($($arg),*);
                if matches!(result, $pat) {
                    self.$pending = Some(cb_args);
                    return result;
                }
                let outcome = match self.fmt_ctx() {
                    Some(ctx) => result.parity_fmt(&ctx),
                    None => format!("{:?}", result),
                };
                self.parity_observer.on_callback(
                    $kind,
                    &outcome,
                    self.player_id.0,
                    self.current_turn,
                    &self.current_phase,
                    cb_args,
                );
                result
            }
        )+
    };
    ($(fn $name:ident (&mut self $(, $arg:ident : $ty:ty )* ) -> $ret:ty => $kind:expr;)+) => {
        $(
            fn $name(&mut self $(, $arg: $ty)*) -> $ret {
                self.save_snapshot($kind);
                let fmt = self.fmt_ctx();
                let cb_args: Vec<String> = vec![$($arg.callback_arg_display(fmt.as_ref())),*];
                let result = self.inner.$name($($arg),*);
                let outcome = match self.fmt_ctx() {
                    Some(ctx) => result.parity_fmt(&ctx),
                    None => format!("{:?}", result),
                };
                self.parity_observer.on_callback(
                    $kind,
                    &outcome,
                    self.player_id.0,
                    self.current_turn,
                    &self.current_phase,
                    cb_args,
                );
                result
            }
        )+
    };
}

impl PlayerAgent for CapturingAgent {
    fn notify(&mut self, event: forge_engine_core::agent::notification::GameNotification) {
        use forge_engine_core::agent::notification::GameNotification;
        match &event {
            GameNotification::Event(log_event) => {
                let message = &log_event.message;
                if let Some(card_name) = extract_coverage_card(message) {
                    self.shared_covered_cards
                        .lock()
                        .unwrap()
                        .insert(card_name.to_string());
                }
            }
            GameNotification::TurnChanged { turn_number, .. } => {
                self.current_turn = *turn_number;
                if self.capture_snapshots {
                    if let Some(ref game) = self.last_game_state {
                        let mut snap = snapshot_game(game);
                        snap.phase = "Untap".to_string();
                        let active = snap.active_player as usize;
                        for (i, p) in snap.players.iter_mut().enumerate() {
                            if i != active {
                                p.lands_played = 0;
                            }
                        }
                        self.shared_log
                            .lock()
                            .unwrap()
                            .push(ParityLogEntry::Snapshot(snap));
                        self.parity_observer.mark_snapshot();
                    }
                }
            }
            GameNotification::PhaseChanged { phase } => {
                self.current_phase = format!("{:?}", phase);
                if let Some(ref mut game) = self.last_game_state {
                    game.turn.phase = *phase;
                }
                if self.deep && self.player_id.0 == 0 {
                    if let Some(ref game) = self.last_game_state {
                        self.shared_log
                            .lock()
                            .unwrap()
                            .push(ParityLogEntry::Snapshot(snapshot_game(game)));
                        self.parity_observer.mark_snapshot();
                    }
                }
            }
            GameNotification::PriorityChanged { player } => {
                if let Some(ref mut game) = self.last_game_state {
                    game.turn.priority_player = *player;
                }
                if self.deep && self.player_id.0 == 0 {
                    if let Some(ref game) = self.last_game_state {
                        self.shared_log
                            .lock()
                            .unwrap()
                            .push(ParityLogEntry::Snapshot(snapshot_game(game)));
                        self.parity_observer.mark_snapshot();
                    }
                }
            }
            GameNotification::ManaPaymentResolved { player, actions } => {
                if *player == self.player_id {
                    if let Some(cb_args) = self.pending_pay_mana_cost_args.take() {
                        let outcome = match self.fmt_ctx() {
                            Some(ctx) => actions.parity_fmt(&ctx),
                            None => format!("{actions:?}"),
                        };
                        self.parity_observer.on_callback(
                            "pay_mana_cost",
                            &outcome,
                            self.player_id.0,
                            self.current_turn,
                            &self.current_phase,
                            cb_args,
                        );
                    }
                }
            }
            _ => {}
        }
        self.inner.notify(event.clone());
    }

    fn snapshot_state(
        &mut self,
        game: &GameState,
        mana_pools: &[forge_engine_core::mana::ManaPool],
    ) {
        self.inner.snapshot_state(game, mana_pools);
        self.last_game_state = Some(game.clone());
    }

    parity_agent_callback! {
        fn choose_targets_for(&mut self, sa: &mut forge_engine_core::spellability::SpellAbility, game: &GameState, mana_pools: &[forge_engine_core::mana::ManaPool]) -> bool => "choose_targets_for";
        fn mulligan_decision(&mut self, player: PlayerId, hand: &[CardId], mulligan_count: u32) -> bool => "mulligan_decision";
        fn choose_cards_to_bottom(&mut self, player: PlayerId, hand: &[CardId], count: usize) -> Vec<CardId> => "choose_cards_to_bottom";
        fn choose_action(&mut self, player: PlayerId, playable: &[PlayOption], tappable_lands: &[CardId], untappable_lands: &[CardId], activatable: &[(CardId, usize)]) -> forge_engine_core::player::actions::PlayerAction => "choose_action";
        fn choose_attackers(&mut self, player: PlayerId, available: &[CardId], possible_defenders: &[DefenderId]) -> Vec<(CardId, DefenderId)> => "choose_attackers";
        fn exert_attackers(&mut self, player: PlayerId, attackers: &[CardId]) -> Vec<CardId> => "exert_attackers";
        fn enlist_attackers(&mut self, player: PlayerId, attackers: &[CardId]) -> Vec<CardId> => "enlist_attackers";
        fn choose_blockers(&mut self, player: PlayerId, attackers: &[CardId], available_blockers: &[CardId], max_blockers: Option<usize>) -> Vec<(CardId, CardId)> => "choose_blockers";
        fn choose_blocker_for(&mut self, player: PlayerId, attackers: &[CardId], blocker: CardId) -> Option<CardId> => "choose_blocker_for";
        fn choose_damage_assignment_order(&mut self, player: PlayerId, attacker: CardId, blockers: &[CardId]) -> Vec<CardId> => "choose_damage_assignment_order";
        fn assign_combat_damage(&mut self, game: &GameState, player: PlayerId, attacker: CardId, blockers_in_order: &[CardId], defender: Option<forge_engine_core::combat::DefenderId>, damage_to_assign: i32) -> Vec<(Option<CardId>, i32)> => "assign_combat_damage";
        fn choose_target_player(&mut self, player: PlayerId, valid: &[PlayerId], sa: Option<&forge_engine_core::spellability::SpellAbility>) -> Option<PlayerId> => "choose_target_player";
        fn choose_target_card(&mut self, player: PlayerId, valid: &[CardId], sa: Option<&forge_engine_core::spellability::SpellAbility>) -> Option<CardId> => "choose_target_card";
        fn choose_target_card_from_zone(&mut self, player: PlayerId, zone: ZoneType, valid: &[CardId], sa: Option<&forge_engine_core::spellability::SpellAbility>) -> Option<CardId> => "choose_target_card_from_zone";
        fn choose_target_any(&mut self, player: PlayerId, valid_players: &[PlayerId], valid_cards: &[CardId], sa: Option<&forge_engine_core::spellability::SpellAbility>) -> forge_engine_core::agent::TargetChoice => "choose_target_any";
        fn choose_legend_keep(&mut self, player: PlayerId, duplicates: &[CardId]) -> CardId => "choose_legend_keep";
        fn choose_sacrifice(&mut self, player: PlayerId, valid: &[CardId], sa: Option<&forge_engine_core::spellability::SpellAbility>) -> Option<CardId> => "choose_sacrifice";
        fn choose_type(&mut self, player: PlayerId, type_category: &str, valid_types: &[String]) -> Option<String> => "choose_type";
        fn choose_scry(&mut self, player: PlayerId, cards: &[CardId]) -> Vec<CardId> => "choose_scry";
        fn choose_surveil(&mut self, player: PlayerId, cards: &[CardId]) -> Vec<CardId> => "choose_surveil";
        fn choose_dig(&mut self, player: PlayerId, valid: &[CardId], max: usize, optional: bool) -> Vec<CardId> => "choose_dig";
        fn choose_reorder_library(&mut self, player: PlayerId, cards: &[CardId]) -> Vec<CardId> => "choose_reorder_library";
        fn choose_discard(&mut self, player: PlayerId, hand: &[CardId], num: usize) -> Vec<CardId> => "choose_discard";
        fn choose_discard_any_number(&mut self, player: PlayerId, hand: &[CardId], min: usize, max: usize) -> Vec<CardId> => "choose_discard";
        fn choose_random_discard(&mut self, player: PlayerId, hand: &[CardId], num: usize) -> Vec<CardId> => "choose_random_discard";
        fn choose_cards_for_effect(&mut self, player: PlayerId, valid: &[CardId], min: usize, max: usize) -> Vec<CardId> => "choose_cards_for_effect";
        fn choose_entities_for_effect(&mut self, player: PlayerId, candidates: &[GameEntity], min: usize, max: usize) -> Vec<GameEntity> => "choose_entities_for_effect";
        fn choose_cards_for_zone_change(&mut self, player: PlayerId, valid: &[CardId], min: usize, max: usize, select_prompt: &str) -> Vec<CardId> => "choose_cards_for_zone_change";
        fn choose_target_spell(&mut self, player: PlayerId, valid: &[u32]) -> Option<u32> => "choose_target_spell";
        fn choose_mode(&mut self, player: PlayerId, descriptions: &[String], min: usize, max: usize, card_name: Option<&str>) -> Vec<usize> => "choose_mode";
        fn choose_spell_abilities_for_effect(&mut self, player: PlayerId, abilities: &[forge_engine_core::spellability::SpellAbility], num: usize) -> Vec<usize> => "choose_spell_abilities_for_effect";
        fn choose_single_entity_for_effect(&mut self, player: PlayerId, valid: &[CardId], is_optional: bool) -> Option<CardId> => "choose_single_entity_for_effect";
        fn get_ability_to_play(&mut self, player: PlayerId, abilities: &[forge_engine_core::spellability::SpellAbility]) -> Option<usize> => "get_ability_to_play";
        fn choose_x_value(&mut self, player: PlayerId, max_x: u32, card_name: Option<&str>) -> u32 => "choose_x_value";
        fn choose_optional_trigger(&mut self, player: PlayerId, description: &str, card_name: Option<&str>, api: Option<forge_engine_core::ability::api_type::ApiType>) -> bool => "choose_optional_trigger";
        fn choose_land_or_spell(&mut self, player: PlayerId) -> Option<bool> => "choose_land_or_spell";
        fn confirm_action(&mut self, player: PlayerId, mode: Option<&str>, message: &str, options: &[String], card_name: Option<&str>, api: Option<forge_engine_core::ability::api_type::ApiType>) -> bool => "confirm_action";
        fn confirm_payment(&mut self, player: PlayerId, cost_kind: &str, message: &str, card_name: Option<&str>, api: Option<forge_engine_core::ability::api_type::ApiType>) -> bool => "confirm_payment";
        fn pay_cost_to_prevent_effect(&mut self, player: PlayerId, paid: bool) -> bool => "pay_cost_to_prevent_effect";
        fn confirm_replacement_effect(&mut self, player: PlayerId, question: &str, effect_description: &str, card_name: Option<&str>) -> bool => "confirm_replacement_effect";
        fn choose_binary(&mut self, player: PlayerId, question: &str, kind: BinaryChoiceKind, default_choice: Option<bool>, card_name: Option<&str>, api: Option<forge_engine_core::ability::api_type::ApiType>) -> bool => "choose_binary";
        fn choose_color(&mut self, player: PlayerId, valid_colors: &[String]) -> Option<String> => "choose_color";
        fn choose_card_name(&mut self, player: PlayerId, valid_names: &[String]) -> Option<String> => "choose_card_name";
        fn choose_number(&mut self, player: PlayerId, min: i32, max: i32) -> Option<i32> => "choose_number";
        fn choose_number_from_list(&mut self, player: PlayerId, choices: &[i32], message: &str, card_name: Option<&str>) -> Option<i32> => "choose_number_from_list";
        fn choose_roll_to_ignore(&mut self, player: PlayerId, rolls: &[i32], card_name: Option<&str>) -> Option<i32> => "choose_roll_to_ignore";
        fn choose_roll_to_swap(&mut self, player: PlayerId, rolls: &[i32], card_name: Option<&str>) -> Option<i32> => "choose_roll_to_swap";
        fn choose_dice_to_reroll(&mut self, player: PlayerId, rolls: &[i32], card_name: Option<&str>) -> Vec<i32> => "choose_dice_to_reroll";
        fn choose_roll_to_modify(&mut self, player: PlayerId, rolls: &[i32], card_name: Option<&str>) -> Option<i32> => "choose_roll_to_modify";
        fn choose_roll_swap_value(&mut self, player: PlayerId, current_result: i32, power: i32, toughness: i32, card_name: Option<&str>) -> Option<forge_engine_core::agent::RollSwapChoice> => "choose_roll_swap_value";
        fn flip_coin_call(&mut self, player: PlayerId) -> bool => "flip_coin_call";
        fn choose_phyrexian_pay_life(&mut self, player: PlayerId, color: &str, card_name: Option<&str>) -> bool => "choose_phyrexian_pay_life";
        fn pay_combat_cost(&mut self, player: PlayerId, attacker: CardId, cost: i32, description: &str, tappable_lands: &[CardId], untappable_lands: &[CardId], mana_pool_total: i32) -> forge_engine_core::agent::CombatCostAction => "pay_combat_cost";
        fn choose_delve(&mut self, player: PlayerId, valid: &[CardId], max: usize, card_name: Option<&str>) -> Vec<CardId> => "choose_delve";
        fn choose_improvise(&mut self, player: PlayerId, untapped_artifacts: &[CardId], remaining_cost: &forge_foundation::ManaCost, card_name: Option<&str>) -> Vec<CardId> => "choose_improvise";
        fn choose_convoke(&mut self, player: PlayerId, untapped_creatures: &[CardId], remaining_cost: &forge_foundation::ManaCost, card_name: Option<&str>) -> Vec<CardId> => "choose_convoke";
        fn specify_mana_combo(&mut self, player: PlayerId, available_colors: &[String], amount: usize, card_name: Option<&str>) -> Vec<String> => "specify_mana_combo";
        fn choose_explore_put_in_graveyard(&mut self, player: PlayerId, revealed_card_name: &str, revealed_cmc: i32, mana_producing_lands: usize, predicted_mana: usize, lands_in_hand: usize) -> bool => "choose_explore_put_in_graveyard";
        fn choose_kicker(&mut self, player: PlayerId, kicker_cost: &str, card_name: Option<&str>) -> bool => "choose_kicker";
        fn help_pay_assist(&mut self, player: PlayerId, card_name: &str, max_generic: u32) -> u32 => "help_pay_assist";
        fn choose_buyback(&mut self, player: PlayerId, buyback_cost: &str, card_name: Option<&str>) -> bool => "choose_buyback";
        fn choose_multikicker(&mut self, player: PlayerId, cost: &str, max_kicks: u32, card_name: Option<&str>) -> u32 => "choose_multikicker";
        fn choose_replicate(&mut self, player: PlayerId, cost: &str, max_replicates: u32, card_name: Option<&str>) -> u32 => "choose_replicate";
        fn choose_alternative_cost(&mut self, player: PlayerId, options: &[String], card_name: Option<&str>) -> usize => "choose_alternative_cost";
        fn choose_single_replacement_effect(&mut self, player: PlayerId, descriptions: &[String]) -> usize => "choose_single_replacement_effect";
    }

    parity_agent_callback! {
        fn pay_mana_cost(&mut self, player: PlayerId, card_id: CardId, card_name: &str, mana_cost: &str, mana_cost_display: &str, mana_cost_checkpoint: &str, allow_reserved_source_reuse: bool, reserved_sacrifices: &[CardId], mana_ability_options: &[forge_engine_core::agent::ManaAbilityOption], tappable_lands: &[CardId], untappable_lands: &[CardId], mana_pool: &forge_engine_core::mana::ManaPool) -> ManaCostAction => "pay_mana_cost", defer_if ManaCostAction::Pay { auto: true } => pending_pay_mana_cost_args;
    }

    fn choose_single_card_for_zone_change(
        &mut self,
        player: PlayerId,
        valid: &[CardId],
        select_prompt: &str,
        is_optional: bool,
    ) -> Option<CardId> {
        // Java emits the raw card label (or "null") — no `Some(..)` wrapper —
        // so we can't go through the macro which uses the generic Option impl.
        self.save_snapshot("choose_single_card_for_zone_change");
        let fmt = self.fmt_ctx();
        let cb_args: Vec<String> = vec![
            player.callback_arg_display(fmt.as_ref()),
            valid.callback_arg_display(fmt.as_ref()),
            select_prompt.callback_arg_display(fmt.as_ref()),
            is_optional.callback_arg_display(fmt.as_ref()),
        ];
        let result =
            self.inner
                .choose_single_card_for_zone_change(player, valid, select_prompt, is_optional);
        let outcome = match (result, self.fmt_ctx()) {
            (Some(cid), Some(ctx)) => ctx.card(cid),
            (Some(cid), None) => format!("{cid:?}"),
            (None, _) => "null".to_string(),
        };
        self.parity_observer.on_callback(
            "choose_single_card_for_zone_change",
            &outcome,
            self.player_id.0,
            self.current_turn,
            &self.current_phase,
            cb_args,
        );
        result
    }

}

pub struct RunConfig {
    pub deck1: String,
    pub deck2: String,
    pub seed: u64,
    pub max_turns: u32,
    pub cards_dir: Option<String>,
    pub decks_dir: Option<String>,
    pub verbose: VerboseMode,
    pub prefer_actions: bool,
    pub deep: bool,
    pub loose_parity: bool,
    pub log_snapshots: bool,
    pub java_heap: String,
    /// Game variant: "Constructed", "Commander", "Oathbreaker", "TinyLeaders", "Brawl".
    pub variant: String,
    /// Commander card names for Commander variants.
    pub commanders: Vec<String>,
    /// Store full callback logs (for --full-log display).
    pub full_log: bool,
}

pub struct LoadedData {
    pub db: CardDatabase,
    pub token_templates: Vec<(String, CardInstance)>,
}

pub fn load_data(cards_dir: Option<&str>, verbose: bool) -> Result<LoadedData, String> {
    let _t_total = Instant::now();
    let cards_dir = cards_dir.unwrap_or("forge/forge-gui/res/cardsfolder");
    let cards_path = std::path::Path::new(cards_dir);

    if !cards_path.exists() {
        return Err(format!(
            "Cards directory not found: {}. Set --cards-dir to the Forge cardsfolder path.",
            cards_dir,
        ));
    }

    if verbose {
        eprintln!("[parity] Loading cards from {:?} ...", cards_path);
    }
    let _t_cards = Instant::now();
    let (db, result) = CardDatabase::load_from_directory(cards_path);
    if verbose {
        eprintln!(
            "[parity] Loaded {} cards ({} failed)",
            result.loaded, result.failed
        );
    }

    let mut token_templates = Vec::new();
    let token_dir_path = cards_path
        .parent()
        .map(|p| p.join("tokenscripts"))
        .unwrap_or_default();
    if token_dir_path.exists() {
        if verbose {
            eprintln!(
                "[parity] Loading token scripts from {:?} ...",
                token_dir_path
            );
        }
        let _t_tokens = Instant::now();
        let (token_db, token_result) = CardDatabase::load_from_directory(&token_dir_path);
        if verbose {
            eprintln!("[parity] Loaded {} token scripts", token_result.loaded);
        }
        for (script_name, rules) in token_db.iter() {
            let template = CardInstance::from_rules(rules, PlayerId(0));
            token_templates.push((script_name.clone(), template));
        }
    }

    // Load creature types from TypeLists.txt into the engine's global registry.
    // Mirrors Java's FModel.loadDynamicGamedata() → CardType.Constant.CREATURE_TYPES.
    {
        let type_list_path = cards_path
            .parent()
            .map(|p| p.join("lists").join("TypeLists.txt"))
            .unwrap_or_default();
        if !type_list_path.exists() {
            return Err(format!(
                "TypeLists.txt not found at {:?}. This file is required for creature type data.",
                type_list_path
            ));
        }
        if verbose {
            eprintln!("[parity] Loading type lists from {:?} ...", type_list_path);
        }
        let _t_types_read = Instant::now();
        let content = std::fs::read_to_string(&type_list_path).map_err(|e| {
            format!(
                "Failed to read TypeLists.txt at {:?}: {}",
                type_list_path, e
            )
        })?;
        let _t_types_parse = Instant::now();
        forge_engine_core::game::TypeRegistry::load(&content);
        if verbose {
            eprintln!(
                "[parity] Loaded {} creature types",
                forge_engine_core::game::TypeRegistry::creature_types().len()
            );
        }
    }

    Ok(LoadedData {
        db,
        token_templates,
    })
}

pub fn run_with_data(config: &RunConfig, data: &LoadedData) -> Result<GameTrace, String> {
    let _t_total = Instant::now();
    // Resolve deck lists — supports preset names, inline: specs, and file: specs
    let decks_dir = config.decks_dir.as_deref().unwrap_or(DEFAULT_DECKS_DIR);
    let _t_resolve = Instant::now();
    let deck1_spec = resolve_deck_spec(&config.deck1, decks_dir)?;
    let deck2_spec = resolve_deck_spec(&config.deck2, decks_dir)?;

    // Determine starting life based on variant
    let starting_life = match config.variant.as_str() {
        "Commander" => 40,
        "Oathbreaker" => 20,
        "TinyLeaders" => 25,
        "Brawl" => 30,
        _ => 20, // Constructed and other variants
    };

    // Set up game
    let p0 = PlayerId(0);
    let p1 = PlayerId(1);
    let mut game = GameState::new(&["Player1", "Player2"], starting_life);

    let _t_build = Instant::now();
    build_deck_from_spec(
        &mut game,
        &data.db,
        p0,
        &deck1_spec,
        config.verbose.is_any(),
    );
    build_deck_from_spec(
        &mut game,
        &data.db,
        p1,
        &deck2_spec,
        config.verbose.is_any(),
    );

    // Set up commanders if in a commander variant
    let is_commander_variant = matches!(
        config.variant.as_str(),
        "Commander" | "Oathbreaker" | "TinyLeaders" | "Brawl"
    );
    if is_commander_variant && !config.commanders.is_empty() {
        let mut unique_commanders: Vec<&str> = Vec::new();
        let mut seen = HashSet::new();
        for commander_name in &config.commanders {
            let key = commander_name.to_ascii_lowercase();
            if seen.insert(key) {
                unique_commanders.push(commander_name.as_str());
            }
        }

        // For each player, find commander cards already present in library and move to command zone.
        // Contract: --commander names must already be in the main deck list.
        for &pid in &[p0, p1] {
            for commander_name in &unique_commanders {
                let card_id = game
                    .cards_in_zone(ZoneType::Library, pid)
                    .iter()
                    .copied()
                    .find(|&cid| {
                        game.card(cid)
                            .card_name
                            .eq_ignore_ascii_case(commander_name)
                    })
                    .ok_or_else(|| {
                        format!(
                            "Commander \"{}\" not found in player {} main deck/library",
                            commander_name,
                            pid.0 + 1
                        )
                    })?;

                // Move to command zone and register as commander.
                game.move_card(card_id, ZoneType::Command, pid);
                game.player_register_commander(pid, card_id);
            }
        }
        // Set commander_damage_enabled based on variant
        let commander_damage_enabled = config.variant == "Commander";
        for &pid in &[p0, p1] {
            game.player_mut(pid).commander_damage_enabled = commander_damage_enabled;
        }
    }

    let mut game_loop = GameLoop::new(2);

    // Register token templates
    for (script_name, template) in &data.token_templates {
        game_loop.register_token(script_name.clone(), template.clone());
    }

    // Copy token art variant data from the card DB for game-RNG parity.
    // Java's Aggregates.random() on a Set consumes nextInt() per element,
    // so Rust must know how many art variants each token has per edition.
    game_loop.token_art_variants = data.db.token_art_variants().clone();
    game_loop.token_fallback = data.db.token_fallback().clone();
    game_loop.edition_dates = data.db.edition_dates().clone();

    // Shared storage for parity log entries captured by CapturingAgent
    let shared_log: Arc<Mutex<Vec<ParityLogEntry>>> = Arc::new(Mutex::new(Vec::new()));
    let shared_covered_cards: Arc<Mutex<BTreeSet<String>>> = Arc::new(Mutex::new(BTreeSet::new()));

    // Run game with fixed seed (for any engine-internal randomness)
    let mut rng = StdRng::seed_from_u64(config.seed);

    // Setup: shuffle libraries with Java-compatible RNG so opening hands match
    // the Java Forge engine, then draw 7 cards per player.
    //
    // Java's flow in match.startGame():
    //   1. prepareAllZones() — builds libraries (no RNG)
    //   2. player.shuffle(null) for each player — Collections.shuffle(list, rng)
    //   3. drawStartingHand() — moves top 7 cards to hand (no RNG)
    //
    // The game_rng mirrors Java's MyRandom — same seed, same consumption order.
    // It's used for both shuffling and game-level random effects (e.g.
    // Aggregates.random() in DiscardEffect Mode$ Random). After shuffling,
    // its state matches Java's MyRandom post-shuffle, so subsequent random
    // effects produce identical results.
    let game_rng = Rc::new(RefCell::new({
        let mut r = JavaRandom::new(config.seed as i64);
        r.label = "game";
        r
    }));
    {
        let _t_shuffle = Instant::now();
        let mut shuffle_rng = game_rng.borrow_mut();
        for &pid in &game.player_order.clone() {
            // Sort library by card name for deterministic pre-shuffle ordering,
            // matching Java's Match.preparePlayerZone which sorts after building
            // from ConcurrentHashMap-backed CardPool.
            let mut lib_cards: Vec<CardId> = game.cards_in_zone(ZoneType::Library, pid).to_vec();
            lib_cards.sort_by(|a, b| {
                game.cards[a.index()]
                    .card_name
                    .cmp(&game.cards[b.index()].card_name)
            });
            // Shuffle using the Java-compatible PRNG
            shuffle_rng.shuffle(&mut lib_cards);
            // Reverse so Java's index-0 "top" becomes Rust's last-element "top"
            // (Rust draws via pop(), Java draws via get(0))
            lib_cards.reverse();
            // Write back the shuffled order
            game.zone_mut(ZoneType::Library, pid).cards = lib_cards;
        }
    }

    // Match Java's determineFirstTurnPlayer() "coin flip".
    // Java calls Aggregates.random(game.getPlayers()) where game.getPlayers()
    // returns a PlayerCollection (extends FCollection<Player> implements List<Player>).
    // Since it implements List, Aggregates.random takes the List fast-path:
    //   src.get(MyRandom.getRandom().nextInt(len))
    // That's a single nextInt(numPlayers) call.
    // The result is overridden by DeterministicController.chooseStartingPlayer()
    // which always returns player 0 — we just need to consume the same RNG call.
    {
        let num_players = game.player_order.len() as i32;
        let _coin_flip = game_rng.borrow_mut().next_int(num_players);
    }
    for &pid in &game.player_order.clone() {
        game.draw_cards(pid, 7);
    }
    // Create a SEPARATE agent RNG seeded identically to Java's `new Random(seed)`.
    // This is distinct from the game RNG — both sides create a fresh Random(seed)
    // for agent decisions, ensuring the RNG state matches even though the game
    // RNG is consumed differently by each engine's internals.
    // IMPORTANT: Must be created BEFORE opening-hand actions since Java's
    // chooseSaToActivateFromOpeningHand consumes this RNG.
    let agent_rng = Rc::new(RefCell::new({
        let mut r = JavaRandom::new(config.seed as i64);
        r.label = "agent";
        r
    }));

    let parity_log_sink: Arc<Mutex<Vec<crate::protocol::ChoiceLogEntry>>> =
        Arc::new(Mutex::new(Vec::new()));
    crate::parity_log::set_sink(Arc::clone(&parity_log_sink));

    let parity_map = Arc::new(ParityCardMap::from_opening_state(&game));
    let shared_snapshot_index: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));

    // Create deterministic agents — player 0 uses CapturingAgent to collect
    // turn-start snapshots (matching Java's GameEventTurnBegan timing).
    // Both agents share the same agent RNG so consumption order matches Java.
    // Both agents share the same game RNG so random effects match Java's MyRandom.
    // Both agents share the same snapshot_index so callbacks from either player
    // reference the correct snapshot position (matching Java's approach).
    let mut agents: Vec<Box<dyn PlayerAgent>> = vec![
        Box::new(CapturingAgent::new(
            p0,
            config.verbose.clone(),
            config.prefer_actions,
            Arc::clone(&shared_log),
            Arc::clone(&shared_covered_cards),
            Arc::clone(&shared_snapshot_index),
            Rc::clone(&agent_rng),
            Rc::clone(&game_rng),
            Arc::clone(&parity_map),
            !config.deep,
            config.deep,
            false,
        )),
        Box::new(CapturingAgent::new(
            p1,
            config.verbose.clone(),
            config.prefer_actions,
            Arc::clone(&shared_log),
            Arc::clone(&shared_covered_cards),
            Arc::clone(&shared_snapshot_index),
            Rc::clone(&agent_rng),
            Rc::clone(&game_rng),
            Arc::clone(&parity_map),
            false,
            config.deep,
            false,
        )),
    ];

    game_loop.run_opening_hand_actions(&mut game, &mut agents);

    // Wire the Java-compatible RNG into the game loop so that effect-level
    // shuffles, coin flips, and dice rolls consume the same PRNG instance
    // as the agents, matching Java's single MyRandom consumption order.
    game_loop.game_rng = Box::new(crate::java_random::JavaGameRng(Rc::clone(&game_rng)));

    // Run turns — CapturingAgent captures turn-start snapshots automatically
    while !game.game_over && game.turn.turn_number <= config.max_turns {
        let _t_turn = Instant::now();
        game_loop.run_turn(&mut game, &mut agents, &mut rng);
    }

    crate::parity_log::clear_sink();

    let log: Vec<ParityLogEntry> = shared_log.lock().unwrap().clone();
    let covered_cards: Vec<String> = shared_covered_cards
        .lock()
        .unwrap()
        .iter()
        .cloned()
        .collect();

    Ok(GameTrace {
        seed: config.seed,
        deck1: config.deck1.clone(),
        deck2: config.deck2.clone(),
        max_turns: config.max_turns,
        variant: config.variant.clone(),
        commanders: config.commanders.clone(),
        log,
        covered_cards,
    })
}

pub fn run_rust_only(config: &RunConfig) -> Result<GameTrace, String> {
    let data = load_data(config.cards_dir.as_deref(), config.verbose.is_any())?;
    run_with_data(config, &data)
}

fn extract_coverage_card(message: &str) -> Option<&str> {
    message
        .strip_prefix("Played land: ")
        .or_else(|| message.strip_prefix("Cast: "))
        .map(str::trim)
        .filter(|s| !s.is_empty())
}
