use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use crate::auto_pay;
use forge_engine_core::agent::{
    BinaryChoiceKind, GameEntity, ManaCostAction, PlayCardMode, PlayOption, PlayerAgent,
    TargetChoice,
};
use forge_engine_core::spellability::SpellAbility;
use forge_engine_core::combat::DefenderId;
use forge_engine_core::game::GameState;
use forge_engine_core::ids::{CardId, PlayerId};
use forge_engine_core::mana::ManaPool;
use forge_engine_core::player::actions::{AbilityRef, PlayerAction};
use forge_engine_core::replacement::replacement_handler::{apply_replacements, ReplacementEvent};
use forge_engine_core::spellability::AlternativeCost;
use forge_foundation::PhaseType;

use crate::choice_space;
use crate::combat_choice_space;
use crate::gui_repro;
use crate::java_random::JavaRandom;
use crate::parity_card_map::ParityCardMap;
use crate::parity_order;

#[allow(dead_code)]
const ANSI_RESET: &str = "\x1b[0m";
#[allow(dead_code)]
const ANSI_DIM_GRAY: &str = "\x1b[90m";
#[allow(dead_code)]
const ANSI_YELLOW: &str = "\x1b[33m";
const PREFER_ACTION_WEIGHT: usize = 3;

#[derive(Clone, Debug)]
pub enum VerboseMode {
    Off,
    All,
    Turns(Vec<u32>),
}

impl VerboseMode {
    /// Parse from an optional CLI value.
    /// `None` / not present → `Off`, `Some(None)` (bare `--verbose`) → `All`,
    /// `Some(Some("21,22"))` → `Turns([21, 22])`.
    pub fn from_flag(present: bool, value: Option<&str>) -> Self {
        if !present {
            return VerboseMode::Off;
        }
        match value {
            None => VerboseMode::All,
            Some(s) if s.is_empty() => VerboseMode::All,
            Some(s) => {
                let turns: Vec<u32> = s.split(',').filter_map(|t| t.trim().parse().ok()).collect();
                if turns.is_empty() {
                    VerboseMode::All
                } else {
                    VerboseMode::Turns(turns)
                }
            }
        }
    }

    pub fn is_active(&self, current_turn: u32) -> bool {
        match self {
            VerboseMode::Off => false,
            VerboseMode::All => true,
            VerboseMode::Turns(turns) => turns.contains(&current_turn),
        }
    }

    /// True only for bare `--verbose` (all turns). Turn-specific modes
    /// should not trigger general progress logging.
    pub fn is_any(&self) -> bool {
        matches!(self, VerboseMode::All)
    }

    pub fn to_java_arg(&self) -> Option<String> {
        match self {
            VerboseMode::Off => None,
            VerboseMode::All => Some(String::new()),
            VerboseMode::Turns(turns) => Some(
                turns
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
            ),
        }
    }
}

pub struct DeterministicAgent {
    player_id: PlayerId,
    pub log: Vec<String>,
    pub verbose: VerboseMode,
    current_turn: u32,
    last_game_snapshot: Option<GameSnapshot>,
    rng: Rc<RefCell<JavaRandom>>,
    game_rng: Rc<RefCell<JavaRandom>>,
    prefer_actions: bool,
    parity_map: Arc<ParityCardMap>,
    parity_observer: Option<Arc<crate::runner::ParityObserver>>,
}

struct GameSnapshot {
    game: GameState,
    card_names: Vec<(CardId, String)>,
    card_is_land: Vec<(CardId, bool)>,
    ability_is_mana: Vec<((CardId, usize), bool)>,
}

#[derive(Clone, Copy)]
enum ActionChoice {
    Card(PlayOption),
    Ability(CardId, usize),
}

#[allow(private_interfaces)]
impl DeterministicAgent {
    pub fn new(
        player_id: PlayerId,
        verbose: VerboseMode,
        rng: Rc<RefCell<JavaRandom>>,
        game_rng: Rc<RefCell<JavaRandom>>,
        prefer_actions: bool,
        parity_map: Arc<ParityCardMap>,
        parity_observer: Option<Arc<crate::runner::ParityObserver>>,
    ) -> Self {
        Self {
            player_id,
            log: Vec::new(),
            verbose,
            current_turn: 0,
            last_game_snapshot: None,
            rng,
            game_rng,
            prefer_actions,
            parity_map,
            parity_observer,
        }
    }

    pub fn rng_call_count(&self) -> u64 {
        self.rng.borrow().call_count
    }

    pub fn rng(&self) -> Rc<RefCell<JavaRandom>> {
        Rc::clone(&self.rng)
    }

    /// Look up a card name from the cached snapshot.
    fn card_name(&self, id: CardId) -> String {
        if let Some(ref snap) = self.last_game_snapshot {
            for (cid, name) in &snap.card_names {
                if *cid == id {
                    return name.clone();
                }
            }
        }
        format!("Card({})", id.0)
    }

    /// Check if a card is a land from the cached snapshot.
    fn is_land(&self, id: CardId) -> bool {
        if let Some(ref snap) = self.last_game_snapshot {
            for (cid, land) in &snap.card_is_land {
                if *cid == id {
                    return *land;
                }
            }
        }
        false
    }

    fn is_mana_ability(&self, card_id: CardId, ability_idx: usize) -> bool {
        if let Some(ref snap) = self.last_game_snapshot {
            for ((cid, idx), is_mana) in &snap.ability_is_mana {
                if *cid == card_id && *idx == ability_idx {
                    return *is_mana;
                }
            }
        }
        false
    }

    fn ability_sort_text(&self, card_id: CardId, ability_idx: usize) -> String {
        if let Some(ref snap) = self.last_game_snapshot {
            let card = snap.game.card(card_id);
            if let Some(ab) = card
                .activated_abilities
                .iter()
                .find(|ab| ab.ability_index == ability_idx)
            {
                return ab.ability_text.clone();
            }
        }
        String::new()
    }

    fn target_owner_controller_key(&self, id: CardId) -> (u32, u32) {
        if let Some(ref snap) = self.last_game_snapshot {
            let card = snap.game.card(id);
            (card.owner.0, card.controller.0)
        } else {
            (u32::MAX, u32::MAX)
        }
    }

    fn predicted_damage_to_card(
        &self,
        game: &GameState,
        target: CardId,
        amount: i32,
        source: CardId,
        is_combat: bool,
    ) -> i32 {
        if amount <= 0 {
            return 0;
        }
        let mut sim = game.clone();
        let mut event = ReplacementEvent::DamageToCard {
            target,
            amount,
            source: Some(source),
            is_combat,
        };
        let _ = apply_replacements(&mut sim, &mut event);
        match event {
            ReplacementEvent::DamageToCard { amount, .. } => amount.max(0),
            _ => 0,
        }
    }

    fn damage_needed_to_kill(
        &self,
        game: &GameState,
        target: CardId,
        max_damage: i32,
        source: CardId,
        is_combat: bool,
    ) -> i32 {
        let target_card = game.card(target);
        let source_card = game.card(source);
        let mut kill_damage = (target_card.toughness() - target_card.damage).max(0);

        if target_card.has_keyword("Indestructible")
            && !source_card.has_wither()
            && !source_card.has_infect()
        {
            return max_damage + 1;
        }
        if source_card.has_deathtouch() && target_card.is_creature() {
            kill_damage = 1;
        }

        for damage in 1..=max_damage {
            if self.predicted_damage_to_card(game, target, damage, source, is_combat) >= kill_damage
            {
                return damage;
            }
        }

        max_damage + 1
    }

    fn play_option_label(&self, play: PlayOption) -> String {
        if self.is_land(play.card_id) {
            return format!("LAND:{}", self.card_name(play.card_id));
        }
        let fb_tag = match play.mode {
            PlayCardMode::Alternative(AlternativeCost::Flashback) => "[FB]",
            _ => "",
        };
        format!("SPELL:{}{}", self.card_name(play.card_id), fb_tag)
    }

    fn play_option_sort_text(play: PlayOption) -> &'static str {
        match play.mode {
            PlayCardMode::Normal => "0",
            PlayCardMode::Alternative(AlternativeCost::Flashback) => "Flashback",
            PlayCardMode::Alternative(AlternativeCost::Spectacle) => "Spectacle",
            PlayCardMode::Alternative(AlternativeCost::Evoke) => "Evoke",
            PlayCardMode::Alternative(AlternativeCost::Dash) => "Dash",
            PlayCardMode::Alternative(AlternativeCost::Blitz) => "Blitz",
            PlayCardMode::Alternative(AlternativeCost::Escape) => "Escape",
            PlayCardMode::Alternative(AlternativeCost::Overload) => "Overload",
            PlayCardMode::Alternative(AlternativeCost::Madness) => "Madness",
            PlayCardMode::Alternative(AlternativeCost::Foretell) => "Foretell",
            PlayCardMode::Alternative(AlternativeCost::Emerge) => "Emerge",
            PlayCardMode::Alternative(AlternativeCost::Suspend) => "Suspend",
            PlayCardMode::Alternative(AlternativeCost::Morph)
            | PlayCardMode::Alternative(AlternativeCost::Megamorph) => "Morph",
            PlayCardMode::Alternative(AlternativeCost::Bestow) => "Bestow",
            PlayCardMode::Alternative(AlternativeCost::Warp) => "0",
            PlayCardMode::Alternative(AlternativeCost::SacrificeAlt) => "0",
            PlayCardMode::Alternative(AlternativeCost::Plot) => "Plot",
            PlayCardMode::Alternative(AlternativeCost::Awaken) => "Awaken",
            PlayCardMode::Alternative(AlternativeCost::Disturb) => "Disturb",
            PlayCardMode::Alternative(AlternativeCost::Harmonize) => "Harmonize",
            PlayCardMode::Alternative(AlternativeCost::Freerunning) => "Freerunning",
            PlayCardMode::Alternative(AlternativeCost::Impending) => "Impending",
            PlayCardMode::Alternative(AlternativeCost::Mayhem) => "Mayhem",
            PlayCardMode::Alternative(AlternativeCost::MTMtE) => "MTMtE",
            PlayCardMode::Alternative(AlternativeCost::Mutate) => "Mutate",
            PlayCardMode::Alternative(AlternativeCost::Prowl) => "Prowl",
            PlayCardMode::Alternative(AlternativeCost::Sneak) => "Sneak",
            PlayCardMode::Alternative(AlternativeCost::Surge) => "Surge",
            PlayCardMode::Alternative(AlternativeCost::WebSlinging) => "WebSlinging",
            PlayCardMode::Alternative(AlternativeCost::Plotted) => "Plotted",
            // Host-card `Mode$ AlternativeCost` actions are represented in Rust
            // as `StaticAlternative`; parity uses the same explicit label.
            PlayCardMode::StaticAlternative => "StaticAlternative",
            PlayCardMode::ForetellExile => "ForetellExile",
        }
    }

    /// Fallback tiebreaker for card play modes. Mirrors Java's use of
    /// `sa.toUnsuppressedString()` as the 5th sort key field.
    /// When variant is the same (e.g., Normal and Warp both return "0"),
    /// this ensures a deterministic ordering.
    fn play_option_fallback(play: PlayOption) -> &'static str {
        match play.mode {
            PlayCardMode::Normal => "Normal",
            PlayCardMode::Alternative(AlternativeCost::Warp) => "Warp",
            PlayCardMode::StaticAlternative => "StaticAlternative",
            // Other modes already have unique variant strings, so fallback rarely matters.
            _ => "",
        }
    }

    fn action_sort_key(&self, choice: &ActionChoice) -> String {
        match *choice {
            ActionChoice::Card(play) => {
                let label = self.play_option_label(play);
                format!(
                    "{}|0|{}|{}|{}",
                    label,
                    self.parity_map.id(play.card_id),
                    Self::play_option_sort_text(play),
                    Self::play_option_fallback(play),
                )
            }
            ActionChoice::Ability(card_id, ability_idx) => format!(
                "AB:{}|1|{}|{:05}|{}",
                self.card_name(card_id),
                self.parity_map.id(card_id),
                ability_idx,
                self.ability_sort_text(card_id, ability_idx),
            ),
        }
    }

    fn legal_attackers_for_blocker(&self, blocker: CardId, attackers: &[CardId]) -> Vec<CardId> {
        let Some(ref snap) = self.last_game_snapshot else {
            return attackers.to_vec();
        };
        attackers
            .iter()
            .copied()
            .filter(|&attacker| {
                forge_engine_core::combat::can_creature_block(&snap.game, blocker, attacker)
            })
            .collect()
    }

    /// Pick a random index in [0, len) from the shared RNG.
    fn pick(&self, len: usize) -> usize {
        choice_space::pick_index(len, &mut self.rng.borrow_mut())
    }

    fn is_verbose(&self) -> bool {
        self.verbose.is_active(self.current_turn)
    }

    fn emit_callback(&self, name: &str, outcome: &str) {
        if let Some(ref observer) = self.parity_observer {
            observer.on_callback(
                name,
                outcome,
                self.player_id.0,
                self.current_turn,
                &format!("{:?}", self.last_game_snapshot.as_ref().map(|s| &s.game.turn.phase).unwrap_or(&PhaseType::Untap)),
                Vec::new(),
            );
        }
    }
}

impl PlayerAgent for DeterministicAgent {
    fn snapshot_state(&mut self, game: &GameState, _mana_pools: &[ManaPool]) {
        // Assign parity IDs for all currently existing cards as soon as we
        // observe state, so later parity_id reads are not first-touch dependent.
        self.parity_map.sync_with_game(game);

        let card_names: Vec<(CardId, String)> = game
            .cards
            .iter()
            .map(|c| {
                let name = if c.face_down {
                    String::new()
                } else {
                    c.card_name.clone()
                };
                (c.id, name)
            })
            .collect();
        let card_is_land: Vec<(CardId, bool)> =
            game.cards.iter().map(|c| (c.id, c.is_land())).collect();
        let ability_is_mana: Vec<((CardId, usize), bool)> = game
            .cards
            .iter()
            .flat_map(|c| {
                c.activated_abilities
                    .iter()
                    .map(move |ab| ((c.id, ab.ability_index), ab.is_mana_ability))
            })
            .collect();
        self.last_game_snapshot = Some(GameSnapshot {
            game: game.clone(),
            card_names,
            card_is_land,
            ability_is_mana,
        });
    }

    fn choose_targets_for(
        &mut self,
        sa: &mut forge_engine_core::spellability::SpellAbility,
        game: &GameState,
        mana_pools: &[ManaPool],
    ) -> bool {
        self.snapshot_state(game, mana_pools);
        if let Some(tr) = sa.target_restrictions.as_ref() {
            let min_targets = tr.get_min_targets(game, sa);
            let current_targets = sa.target_chosen.all_target_cards().len() as i32
                + i32::from(sa.target_chosen.target_player.is_some())
                + i32::from(sa.target_chosen.target_stack_entry.is_some());
            if current_targets == 0 && min_targets <= 0 {
                return true;
            }
        }
        let result = forge_engine_core::spellability::choose_targets_by_kind(
            self, sa, game, mana_pools,
        );

        // Log the actual targets chosen for parity debugging.
        let mut target_names = Vec::new();
        if let Some(pid) = sa.target_chosen.target_player {
            target_names.push(format!("Player({})", pid.0));
        }
        if let Some(cid) = sa.target_chosen.target_card {
            target_names.push(format!("{}@{}", self.card_name(cid), self.parity_map.id(cid)));
        }
        for &cid in sa.target_chosen.divided_map.keys() {
            target_names.push(format!("{}@{}", self.card_name(cid), self.parity_map.id(cid)));
        }
        if let Some(stack_id) = sa.target_chosen.target_stack_entry {
            target_names.push(format!("Stack({})", stack_id));
        }
        if !target_names.is_empty() {
            self.emit_callback(
                "choose_targets_for(inner)",
                &format!("[{}]", target_names.join(", ")),
            );
        }
        result
    }

    fn mulligan_decision(
        &mut self,
        _player: PlayerId,
        _hand: &[CardId],
        _mulligan_count: u32,
    ) -> bool {
        true
    }

    fn choose_action(
        &mut self,
        _player: PlayerId,
        playable: &[PlayOption],
        _tappable_lands: &[CardId],
        _untappable_lands: &[CardId],
        activatable: &[(CardId, usize)],
    ) -> PlayerAction {
        if playable.is_empty() && activatable.is_empty() {
            return PlayerAction::PassPriority;
        }

        // Match Java harness ActionSpace: omit explicit mana abilities from the
        // deterministic main action space.
        let filtered_activatable: Vec<(CardId, usize)> = activatable
            .iter()
            .copied()
            .filter(|(card_id, ability_idx)| !self.is_mana_ability(*card_id, *ability_idx))
            .collect();
        let choices: Vec<ActionChoice> = playable
            .iter()
            .copied()
            .into_iter()
            .map(ActionChoice::Card)
            .chain(
                filtered_activatable
                    .iter()
                    .copied()
                    .map(|(card_id, idx)| ActionChoice::Ability(card_id, idx)),
            )
            .collect();
        let choices = choice_space::sort_native(&choices, |a, b| {
            self.action_sort_key(a).cmp(&self.action_sort_key(b))
        });
        if choices.is_empty() {
            return PlayerAction::PassPriority;
        }
        // Pick randomly:
        // - default: each action + pass are equally likely
        // - prefer-actions: each action has weight PREFER_ACTION_WEIGHT, pass has weight 1
        let chosen_idx = if self.prefer_actions {
            let idx = choice_space::pick_weighted_index_with_pass(
                choices.len(),
                PREFER_ACTION_WEIGHT,
                &mut self.rng.borrow_mut(),
            );
            if idx >= choices.len() {
                return PlayerAction::PassPriority;
            }
            idx
        } else {
            let idx = choice_space::pick_index_with_pass(choices.len(), &mut self.rng.borrow_mut());
            if idx >= choices.len() {
                return PlayerAction::PassPriority;
            }
            idx
        };

        match choices[chosen_idx] {
            ActionChoice::Card(chosen) => PlayerAction::CastSpell(chosen),
            ActionChoice::Ability(card_id, ability_idx) => {
                PlayerAction::ActivateAbility(AbilityRef {
                    card_id,
                    ability_index: ability_idx,
                })
            }
        }
    }

    fn pay_mana_cost(
        &mut self,
        _player: PlayerId,
        card_id: CardId,
        _card_name: &str,
        mana_cost: &str,
        _mana_cost_display: &str,
        mana_cost_checkpoint: &str,
        allow_reserved_source_reuse: bool,
        _mana_ability_options: &[forge_engine_core::agent::ManaAbilityOption],
        _tappable_lands: &[CardId],
        _untappable_lands: &[CardId],
        mana_pool: &ManaPool,
    ) -> ManaCostAction {
        let Some(ref snap) = self.last_game_snapshot else {
            return ManaCostAction::Cancel;
        };
        let callback_cost = if mana_cost.contains('X') {
            mana_cost_checkpoint
        } else {
            mana_cost
        };
        auto_pay::next_mana_cost_action(
            &snap.game,
            mana_pool,
            self.player_id,
            card_id,
            callback_cost,
            allow_reserved_source_reuse,
        )
        .action
    }

    fn choose_attackers(
        &mut self,
        _player: PlayerId,
        available: &[CardId],
        possible_defenders: &[DefenderId],
    ) -> Vec<(CardId, DefenderId)> {
        let mut attackers = Vec::new();
        if !possible_defenders.is_empty() {
            let sorted_available = choice_space::sort_native(available, |a, b| {
                let an = self.card_name(*a);
                let bn = self.card_name(*b);
                an.cmp(&bn)
                    .then_with(|| self.parity_map.id(*a).cmp(&self.parity_map.id(*b)))
            });
            for &id in &sorted_available {
                let roll = choice_space::pick_index(2, &mut self.rng.borrow_mut());
                if self.is_verbose() {
                    eprintln!(
                        "[parity-agent p{}] atk roll {} -> {}",
                        self.player_id.0,
                        self.card_name(id),
                        roll
                    );
                }
                if roll == 1 {
                    let def_idx = choice_space::pick_index(
                        possible_defenders.len(),
                        &mut self.rng.borrow_mut(),
                    );
                    if self.is_verbose() {
                        eprintln!(
                            "[parity-agent p{}] atk defender {} idx={}/{}",
                            self.player_id.0,
                            self.card_name(id),
                            def_idx,
                            possible_defenders.len()
                        );
                    }
                    attackers.push((id, possible_defenders[def_idx]));
                }
            }
        }
        if !attackers.is_empty() {
            let names: Vec<String> = attackers
                .iter()
                .map(|&(id, _)| self.card_name(id))
                .collect();
            let _joined = names.join(", ");
        } else {
        }
        attackers
    }

    fn exert_attackers(&mut self, _player: PlayerId, attackers: &[CardId]) -> Vec<CardId> {
        if attackers.is_empty() {
            return vec![];
        }
        let mut out = Vec::new();
        let mut rng = self.rng.borrow_mut();
        for &attacker in attackers {
            if gui_repro::pick_bool(&mut rng) {
                out.push(attacker);
            }
        }
        out
    }

    fn enlist_attackers(&mut self, _player: PlayerId, attackers: &[CardId]) -> Vec<CardId> {
        if attackers.is_empty() {
            return vec![];
        }
        choice_space::pick_one(attackers, &mut self.rng.borrow_mut())
            .into_iter()
            .collect()
    }

    fn choose_blockers(
        &mut self,
        _player: PlayerId,
        attackers: &[CardId],
        available_blockers: &[CardId],
        max_blockers: Option<usize>,
    ) -> Vec<(CardId, CardId)> {
        let sorted_attackers = choice_space::sort_native(attackers, |a, b| {
            let an = self.card_name(*a);
            let bn = self.card_name(*b);
            an.cmp(&bn)
                .then_with(|| self.parity_map.id(*a).cmp(&self.parity_map.id(*b)))
        });
        let sorted_blockers = choice_space::sort_native(available_blockers, |a, b| {
            let an = self.card_name(*a);
            let bn = self.card_name(*b);
            an.cmp(&bn)
                .then_with(|| self.parity_map.id(*a).cmp(&self.parity_map.id(*b)))
        });

        let mut pairs = Vec::new();
        for &blocker in &sorted_blockers {
            // When BlockRestrict limit is reached, Java still iterates remaining
            // blockers with 0 legal options (consuming RNG for forced PASS).
            // Mirror this by continuing iteration but with empty legal attackers.
            let at_limit = max_blockers.map_or(false, |max| pairs.len() >= max);
            let legal_attackers = if at_limit {
                Vec::new() // no legal targets → forced PASS (consumes RNG)
            } else {
                self.legal_attackers_for_blocker(blocker, &sorted_attackers)
            };
            let choice = choice_space::pick_index_with_pass(
                legal_attackers.len(),
                &mut self.rng.borrow_mut(),
            );
            if choice > 0 && choice <= legal_attackers.len() {
                pairs.push((blocker, legal_attackers[choice - 1]));
            }
        }
        if pairs.is_empty() {
            return pairs;
        }
        pairs
    }

    fn choose_blocker_for(
        &mut self,
        _player: PlayerId,
        attackers: &[CardId],
        blocker: CardId,
    ) -> Option<CardId> {
        let sorted_attackers = choice_space::sort_native(attackers, |a, b| {
            let an = self.card_name(*a);
            let bn = self.card_name(*b);
            an.cmp(&bn)
                .then_with(|| self.parity_map.id(*a).cmp(&self.parity_map.id(*b)))
        });
        let legal_attackers = self.legal_attackers_for_blocker(blocker, &sorted_attackers);
        if legal_attackers.is_empty() {
            // Java DeterministicController always rolls `nextInt(options.size() + 1)`.
            // When options is empty, that's `nextInt(1)` (consumes RNG, always 0).
            let _ = choice_space::pick_index_with_pass(0, &mut self.rng.borrow_mut());
            return None;
        }
        let attacker = combat_choice_space::pick_single_blocker_target(
            &legal_attackers,
            &mut self.rng.borrow_mut(),
        );
        if attacker.is_none() {
            return None;
        }
        let attacker = attacker.unwrap();
        Some(attacker)
    }

    fn choose_damage_assignment_order(
        &mut self,
        _player: PlayerId,
        _attacker: CardId,
        blockers: &[CardId],
    ) -> Vec<CardId> {
        parity_order::sort_cards_by_name_then_id(
            blockers,
            |cid| self.card_name(cid),
            |cid| self.parity_map.id(cid),
        )
    }

    fn assign_combat_damage(
        &mut self,
        game: &GameState,
        _player: PlayerId,
        attacker: CardId,
        blockers_in_order: &[CardId],
        defender_id: Option<DefenderId>,
        damage_to_assign: i32,
    ) -> Vec<(Option<CardId>, i32)> {
        let mut out: Vec<(Option<CardId>, i32)> = Vec::new();
        if damage_to_assign <= 0 {
            return out;
        }

        let has_trample = game.card(attacker).has_trample();
        let can_assign_defender = has_trample && defender_id.is_some();
        let mut damage_left = damage_to_assign;
        let mut last_target: Option<CardId> = None;

        for &blocker in blockers_in_order {
            if damage_left <= 0 {
                break;
            }
            if game.card(blocker).zone != forge_foundation::ZoneType::Battlefield {
                continue;
            }
            if forge_engine_core::staticability::static_ability_colorless_damage_source::target_is_protected_from_source(
                &game.cards,
                game.card(blocker),
                game.card(attacker),
            ) {
                continue;
            }
            last_target = Some(blocker);
            let blocker_card = game.card(blocker);
            let lethal = if blocker_card.type_line.is_planeswalker() {
                blocker_card.counter_count(&forge_engine_core::card::CounterType::Loyalty)
            } else {
                self.damage_needed_to_kill(game, blocker, damage_left, attacker, true)
            };
            let assign = lethal.min(damage_left);
            if assign > 0 {
                out.push((Some(blocker), assign));
                damage_left -= assign;
            }
        }

        if damage_left > 0 {
            if can_assign_defender {
                out.push((None, damage_left));
            } else if let Some(last) = last_target {
                if let Some((_, d)) = out
                    .iter_mut()
                    .find(|(assignee, _)| assignee.map(|id| id == last).unwrap_or(false))
                {
                    *d += damage_left;
                } else {
                    out.push((Some(last), damage_left));
                }
            }
        }

        out
    }

    fn choose_target_spell(&mut self, _player: PlayerId, valid: &[u32]) -> Option<u32> {
        if valid.is_empty() {
            return None;
        }
        let target = choice_space::pick_one(valid, &mut self.rng.borrow_mut())?;
        Some(target)
    }

    fn choose_target_player(
        &mut self,
        _player: PlayerId,
        valid: &[PlayerId],
        _sa: Option<&forge_engine_core::spellability::SpellAbility>,
    ) -> Option<PlayerId> {
        if valid.is_empty() {
            return None;
        }
        let target = choice_space::pick_one(valid, &mut self.rng.borrow_mut())?;
        Some(target)
    }

    fn choose_target_card(
        &mut self,
        _player: PlayerId,
        valid: &[CardId],
        _sa: Option<&forge_engine_core::spellability::SpellAbility>,
    ) -> Option<CardId> {
        if valid.is_empty() {
            return None;
        }
        // Keep target ordering aligned with Java parity harness:
        // sort by card name, then owner/controller, then parity id.
        let sorted = choice_space::sort_native(valid, |a, b| {
            self.card_name(*a)
                .cmp(&self.card_name(*b))
                .then_with(|| {
                    self.target_owner_controller_key(*a)
                        .cmp(&self.target_owner_controller_key(*b))
                })
                .then_with(|| self.parity_map.id(*a).cmp(&self.parity_map.id(*b)))
        });
        let target = choice_space::pick_one(&sorted, &mut self.rng.borrow_mut())?;
        Some(target)
    }

    fn choose_target_card_from_zone(
        &mut self,
        player: PlayerId,
        _zone: forge_foundation::ZoneType,
        valid: &[CardId],
        sa: Option<&forge_engine_core::spellability::SpellAbility>,
    ) -> Option<CardId> {
        self.choose_target_card(player, valid, sa)
    }

    fn choose_target_any(
        &mut self,
        _player: PlayerId,
        valid_players: &[PlayerId],
        valid_cards: &[CardId],
        _sa: Option<&forge_engine_core::spellability::SpellAbility>,
    ) -> TargetChoice {
        let mut sorted: Vec<TargetChoice> = valid_players
            .iter()
            .copied()
            .map(TargetChoice::Player)
            .chain(valid_cards.iter().copied().map(TargetChoice::Card))
            .collect();
        // Keep target ordering aligned with Java parity harness:
        // players first by id/name, then cards by name, owner/controller, parity id.
        sorted.sort_by(|a, b| match (a, b) {
            (TargetChoice::Player(pa), TargetChoice::Player(pb)) => pa.0.cmp(&pb.0),
            (TargetChoice::Player(_), TargetChoice::Card(_)) => std::cmp::Ordering::Less,
            (TargetChoice::Card(_), TargetChoice::Player(_)) => std::cmp::Ordering::Greater,
            (TargetChoice::Card(ca), TargetChoice::Card(cb)) => self
                .card_name(*ca)
                .cmp(&self.card_name(*cb))
                .then_with(|| {
                    self.target_owner_controller_key(*ca)
                        .cmp(&self.target_owner_controller_key(*cb))
                })
                .then_with(|| self.parity_map.id(*ca).cmp(&self.parity_map.id(*cb))),
            _ => std::cmp::Ordering::Equal,
        });

        let total = sorted.len();

        if total == 0 {
            return TargetChoice::None;
        }

        let idx = self.pick(total);
        match sorted[idx] {
            TargetChoice::Player(pid) => TargetChoice::Player(pid),
            TargetChoice::Card(cid) => TargetChoice::Card(cid),
            TargetChoice::None => TargetChoice::None,
        }
    }

    fn choose_optional_trigger(
        &mut self,
        _player: PlayerId,
        _description: &str,
        _card_name: Option<&str>,
        _api: Option<forge_engine_core::ability::api_type::ApiType>,
    ) -> bool {
        let accept = choice_space::pick_bool(&mut self.rng.borrow_mut());
        accept
    }

    fn confirm_action(
        &mut self,
        _player: PlayerId,
        _mode: Option<&str>,
        _message: &str,
        _options: &[String],
        _card_name: Option<&str>,
        _api: Option<forge_engine_core::ability::api_type::ApiType>,
    ) -> bool {
        let accept = choice_space::pick_bool(&mut self.rng.borrow_mut());
        accept
    }

    fn confirm_replacement_effect(
        &mut self,
        _player: PlayerId,
        _question: &str,
        _effect_description: &str,
        _card_name: Option<&str>,
    ) -> bool {
        let accept = choice_space::pick_bool(&mut self.rng.borrow_mut());
        accept
    }

    fn confirm_payment(
        &mut self,
        _player: PlayerId,
        _cost_kind: &str,
        _message: &str,
        _card_name: Option<&str>,
        _api: Option<forge_engine_core::ability::api_type::ApiType>,
    ) -> bool {
        let accept = choice_space::pick_bool(&mut self.rng.borrow_mut());
        accept
    }

    fn choose_binary(
        &mut self,
        _player: PlayerId,
        _question: &str,
        _kind: BinaryChoiceKind,
        _default_choice: Option<bool>,
        _card_name: Option<&str>,
        _api: Option<forge_engine_core::ability::api_type::ApiType>,
    ) -> bool {
        let chosen_left = choice_space::pick_bool(&mut self.rng.borrow_mut());
        chosen_left
    }

    // ── Fixed overrides that sort alphabetically (matching Java) but use no RNG ──

    fn choose_legend_keep(&mut self, _player: PlayerId, duplicates: &[CardId]) -> CardId {
        // Sort by (card_name, parity_id) for deterministic cross-engine parity.
        // Both Java and Rust sort identically to avoid HashMap ordering issues.
        let sorted = choice_space::sort_native(duplicates, |a, b| {
            self.card_name(*a)
                .cmp(&self.card_name(*b))
                .then_with(|| self.parity_map.id(*a).cmp(&self.parity_map.id(*b)))
        });
        choice_space::pick_one(&sorted, &mut self.rng.borrow_mut()).unwrap_or(duplicates[0])
    }

    fn choose_sacrifice(
        &mut self,
        _player: PlayerId,
        valid: &[CardId],
        _sa: Option<&forge_engine_core::spellability::SpellAbility>,
    ) -> Option<CardId> {
        if valid.is_empty() {
            return None;
        }
        let sorted = choice_space::sort_native(valid, |a, b| {
            self.card_name(*a)
                .cmp(&self.card_name(*b))
                .then_with(|| self.parity_map.id(*a).cmp(&self.parity_map.id(*b)))
        });
        choice_space::pick_one(&sorted, &mut self.rng.borrow_mut())
    }

    fn choose_discard(&mut self, _player: PlayerId, hand: &[CardId], num: usize) -> Vec<CardId> {
        if hand.is_empty() || num == 0 {
            return vec![];
        }
        // Sort by (card_name, parity_id) for deterministic cross-engine parity.
        let sorted = choice_space::sort_native(hand, |a, b| {
            self.card_name(*a)
                .cmp(&self.card_name(*b))
                .then_with(|| self.parity_map.id(*a).cmp(&self.parity_map.id(*b)))
        });
        gui_repro::pick_many_unique(&sorted, num, num, &mut self.rng.borrow_mut())
    }

    fn choose_random_discard(
        &mut self,
        _player: PlayerId,
        hand: &[CardId],
        num: usize,
    ) -> Vec<CardId> {
        if hand.is_empty() || num == 0 {
            return vec![];
        }
        // Reservoir sampling with the game RNG, mirroring Java's Aggregates.random()
        // which uses MyRandom.getRandom().nextInt(i) for reservoir replacement.
        // We use game_rng (not agent rng) to match Java's architecture where
        // Aggregates.random() uses MyRandom (the game-level RNG) rather than
        // the agent's decision RNG.
        // IMPORTANT: Do NOT sort — Java iterates cards in zone order (the order
        // they were added to hand), not alphabetically. Sorting would change the
        // reservoir sampling input sequence and produce different results.
        let count = num.min(hand.len());
        let mut rng = self.game_rng.borrow_mut();
        let mut result: Vec<CardId> = hand[..count].to_vec();
        for i in count..hand.len() {
            let j = choice_space::pick_index(i + 1, &mut rng);
            if j < count {
                result[j] = hand[i];
            }
        }
        result
    }

    fn choose_dig(
        &mut self,
        _player: PlayerId,
        valid: &[CardId],
        max: usize,
        optional: bool,
    ) -> Vec<CardId> {
        if valid.is_empty() || max == 0 {
            return vec![];
        }
        // Java DigEffect: min = (anyNumber || optional) ? 0 : max
        // When not optional, the player must take exactly `max` cards.
        let min = if optional { 0 } else { max };
        // Sort by (card_name, parity_id) for deterministic cross-engine parity.
        let sorted = choice_space::sort_native(valid, |a, b| {
            self.card_name(*a)
                .cmp(&self.card_name(*b))
                .then_with(|| self.parity_map.id(*a).cmp(&self.parity_map.id(*b)))
        });
        gui_repro::pick_many_unique(&sorted, min, max, &mut self.rng.borrow_mut())
    }

    fn choose_land_or_spell(&mut self, _player: PlayerId) -> Option<bool> {
        // TODO: engine does not currently expose a typed choice list here.
        None
    }

    fn choose_color(&mut self, _player: PlayerId, valid_colors: &[String]) -> Option<String> {
        gui_repro::choose_color(valid_colors, &mut self.rng.borrow_mut())
    }

    fn choose_type(
        &mut self,
        _player: PlayerId,
        _type_category: &str,
        valid_types: &[String],
    ) -> Option<String> {
        gui_repro::choose_type(valid_types, &mut self.rng.borrow_mut())
    }

    fn choose_card_name(&mut self, _player: PlayerId, valid_names: &[String]) -> Option<String> {
        gui_repro::choose_card_name(valid_names, &mut self.rng.borrow_mut())
    }

    fn choose_number(&mut self, _player: PlayerId, min: i32, max: i32) -> Option<i32> {
        Some(gui_repro::choose_number(
            min,
            max,
            &mut self.rng.borrow_mut(),
        ))
    }

    fn choose_x_value(&mut self, _player: PlayerId, max_x: u32, _card_name: Option<&str>) -> u32 {
        max_x
    }

    /// Always pay life for phyrexian mana — matches Java's
    /// ComputerUtilMana.payManaCost() which auto-pays phyrexian
    /// shards with life when no colored mana source is available.
    fn choose_phyrexian_pay_life(
        &mut self,
        _player: PlayerId,
        _color: &str,
        _card_name: Option<&str>,
    ) -> bool {
        true
    }

    fn choose_cards_for_effect(
        &mut self,
        _player: PlayerId,
        valid: &[CardId],
        min: usize,
        max: usize,
    ) -> Vec<CardId> {
        if valid.is_empty() {
            return vec![];
        }
        // Sort valid cards by (card_name, parity_id) for deterministic cross-engine parity.
        let sorted = choice_space::sort_native(valid, |a, b| {
            self.card_name(*a)
                .cmp(&self.card_name(*b))
                .then_with(|| self.parity_map.id(*a).cmp(&self.parity_map.id(*b)))
        });
        gui_repro::pick_many_unique(&sorted, min, max, &mut self.rng.borrow_mut())
    }

    fn choose_entities_for_effect(
        &mut self,
        _player: PlayerId,
        candidates: &[GameEntity],
        min: usize,
        max: usize,
    ) -> Vec<GameEntity> {
        if candidates.is_empty() {
            return vec![];
        }
        // Sort entities canonically: players first (by id), cards second (by name + parity_id).
        let mut sorted = candidates.to_vec();
        sorted.sort_by(|a, b| {
            let key = |e: &GameEntity| -> (u8, String, u32) {
                match e {
                    GameEntity::Player(pid) => (0, format!("P{}", pid.0), 0),
                    GameEntity::Card(cid) => (1, self.card_name(*cid), self.parity_map.id(*cid)),
                }
            };
            key(a).cmp(&key(b))
        });
        gui_repro::pick_many_unique(&sorted, min, max, &mut self.rng.borrow_mut())
    }

    fn choose_single_card_for_zone_change(
        &mut self,
        player: PlayerId,
        valid: &[CardId],
        _select_prompt: &str,
        _is_optional: bool,
    ) -> Option<CardId> {
        let sorted = choice_space::sort_native(valid, |a, b| {
            self.card_name(*a)
                .cmp(&self.card_name(*b))
                .then_with(|| self.parity_map.id(*a).cmp(&self.parity_map.id(*b)))
        });
        let result = self
            .choose_cards_for_effect(player, &sorted, 1, 1)
            .into_iter()
            .next();
        result
    }

    fn choose_cards_for_zone_change(
        &mut self,
        player: PlayerId,
        valid: &[CardId],
        min: usize,
        max: usize,
        _select_prompt: &str,
    ) -> Vec<CardId> {
        let sorted = choice_space::sort_native(valid, |a, b| {
            self.card_name(*a)
                .cmp(&self.card_name(*b))
                .then_with(|| self.parity_map.id(*a).cmp(&self.parity_map.id(*b)))
        });
        self.choose_cards_for_effect(player, &sorted, min, max)
    }

    fn choose_mode(
        &mut self,
        _player: PlayerId,
        descriptions: &[String],
        min: usize,
        max: usize,
        _card_name: Option<&str>,
    ) -> Vec<usize> {
        if descriptions.is_empty() {
            return vec![];
        }
        let mut rng = self.rng.borrow_mut();
        let count = gui_repro::pick_count(min, max, descriptions.len(), &mut rng);
        let mut pool: Vec<usize> = (0..descriptions.len()).collect();
        let mut out = Vec::with_capacity(count);
        for _ in 0..count {
            if pool.is_empty() {
                break;
            }
            let idx = choice_space::pick_index(pool.len(), &mut rng);
            out.push(pool.remove(idx));
        }
        out
    }

    fn choose_spell_abilities_for_effect(
        &mut self,
        _player: PlayerId,
        abilities: &[SpellAbility],
        num: usize,
    ) -> Vec<usize> {
        if abilities.is_empty() || num == 0 {
            return vec![];
        }
        let count = num.min(abilities.len());
        let mut pool: Vec<usize> = (0..abilities.len()).collect();
        let mut out = Vec::with_capacity(count);
        let mut rng = self.rng.borrow_mut();
        for _ in 0..count {
            if pool.is_empty() {
                break;
            }
            let idx = choice_space::pick_index(pool.len(), &mut rng);
            out.push(pool.remove(idx));
        }
        out
    }

    fn choose_single_entity_for_effect(
        &mut self,
        _player: PlayerId,
        valid: &[CardId],
        _is_optional: bool,
    ) -> Option<CardId> {
        if valid.is_empty() {
            return None;
        }
        let sorted = choice_space::sort_native(valid, |a, b| {
            self.card_name(*a)
                .cmp(&self.card_name(*b))
                .then_with(|| self.parity_map.id(*a).cmp(&self.parity_map.id(*b)))
        });
        choice_space::pick_one(&sorted, &mut self.rng.borrow_mut())
    }

    fn get_ability_to_play(
        &mut self,
        _player: PlayerId,
        abilities: &[SpellAbility],
    ) -> Option<usize> {
        if abilities.is_empty() {
            return None;
        }
        let idx = choice_space::pick_index(abilities.len(), &mut self.rng.borrow_mut());
        Some(idx)
    }

    fn choose_scry(&mut self, _player: PlayerId, cards: &[CardId]) -> Vec<CardId> {
        let mut out = Vec::new();
        let mut rng = self.rng.borrow_mut();
        for &cid in cards {
            if gui_repro::pick_bool(&mut rng) {
                out.push(cid);
            }
        }
        out
    }

    fn choose_surveil(&mut self, _player: PlayerId, cards: &[CardId]) -> Vec<CardId> {
        let mut out = Vec::new();
        let mut rng = self.rng.borrow_mut();
        for &cid in cards {
            if gui_repro::pick_bool(&mut rng) {
                out.push(cid);
            }
        }
        out
    }

    fn choose_reorder_library(&mut self, _player: PlayerId, cards: &[CardId]) -> Vec<CardId> {
        // Java's DeterministicController.orderMoveToZoneList returns cards as-is
        // (no RNG consumed), so we must do the same to stay in sync.
        cards.to_vec()
    }

    fn notify(&mut self, event: forge_engine_core::agent::notification::GameNotification) {
        use forge_engine_core::agent::notification::GameNotification;
        match &event {
            GameNotification::Event(log_event) => {
                if self.log.len() >= 500 {
                    self.log.remove(0);
                }
                self.log.push(log_event.message.clone());
                if self.is_verbose() {
                    eprintln!(
                        "[parity-agent-rust p{}] notify: {}",
                        self.player_id.0, log_event.message
                    );
                }
            }
            GameNotification::TurnChanged {
                active_player,
                turn_number,
            } => {
                self.current_turn = *turn_number;
                if self.is_verbose() {
                    eprintln!(
                        "[parity-agent-rust p{}] === Turn {} (P{} active) ===",
                        self.player_id.0, turn_number, active_player.0
                    );
                }
            }
            GameNotification::PhaseChanged { phase } => {
                if self.is_verbose() {
                    eprintln!(
                        "[parity-agent-rust p{}] --- Phase: {:?} ---",
                        self.player_id.0, phase
                    );
                }
            }
            _ => {}
        }
    }

    fn choose_single_replacement_effect(
        &mut self,
        _player: PlayerId,
        descriptions: &[String],
    ) -> usize {
        let sorted = parity_order::sort_replacement_descriptions_with_indices(descriptions);
        if sorted.is_empty() {
            return 0;
        }
        let picked = choice_space::pick_index(sorted.len(), &mut self.rng.borrow_mut());
        sorted[picked].0
    }
}
