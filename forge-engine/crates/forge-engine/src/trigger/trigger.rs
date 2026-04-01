use forge_foundation::{PhaseType, ZoneType};
use serde::{Deserialize, Serialize};

use crate::card::valid_filter;
use crate::event::{AbilityValue, RunParams, TriggerType};
use crate::game::GameState;
use crate::ids::{CardId, PlayerId};
use crate::parsing::{keys, Params};
use crate::spellability::{build_spell_ability, SpellAbility};

/// Mirrors Java's abstract Trigger class.
/// In Java, each TriggerType has a subclass (TriggerChangesZone, TriggerPhase, etc.)
/// with a performTest() override. In Rust, TriggerMode enum dispatch replaces this.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trigger {
    pub id: u32,
    pub mode: TriggerMode,
    /// Raw parsed parameters — mirrors Java's mapParams: Map<String,String>.
    pub params: Params,
    /// Zones where host card must be for trigger to be active.
    /// Default: [Battlefield].
    pub active_zones: Vec<ZoneType>,
    /// SVar name to execute — mirrors Java's Execute$ → overridingAbility.
    pub execute: String,
    /// Whether trigger is optional (has OptionalDecider$).
    pub optional: bool,
    /// Trigger description text.
    pub description: String,
    /// Whether this trigger is intrinsic to the card.
    pub intrinsic: bool,
    /// Java parity: `Static$ True` triggers are matched before normal triggers.
    pub static_trigger: bool,
    /// Java parity: remembers objects captured by the trigger.
    #[serde(default)]
    pub trigger_remembered: Vec<AbilityValue>,
}

/// Replaces Java's Trigger subclass hierarchy.
/// Each variant holds the parsed parameters specific to that trigger type,
/// and perform_test() dispatches to variant-specific matching logic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TriggerMode {
    ChangesZone {
        origin: Option<ZoneType>,
        destination: Option<ZoneType>,
        valid_card: Option<String>,
        excluded_origins: Option<Vec<ZoneType>>,
        excluded_destinations: Option<Vec<ZoneType>>,
        valid_cause: Option<String>,
        check_on_triggered_card: Option<String>,
        fizzle: Option<bool>,
        not_this_ability: bool,
        condition_you_cast_this_turn: Option<String>,
    },
    Phase {
        phase: Option<PhaseType>,
        valid_player: Option<String>,
    },
    SpellCast {
        valid_card: Option<String>,
        valid_activating_player: Option<String>,
    },
    AbilityCast {
        valid_card: Option<String>,
        valid_activating_player: Option<String>,
    },
    SpellAbilityCast {
        valid_card: Option<String>,
        valid_activating_player: Option<String>,
    },
    Attacks {
        valid_card: Option<String>,
        /// If true, only triggers when the creature attacks alone (Exalted).
        alone: bool,
    },
    /// A creature fought another creature.
    Fight {
        valid_card: Option<String>,
    },
    /// One or more creatures fought (batch event).
    FightOnce {
        valid_card: Option<String>,
    },
    DamageDone {
        valid_source: Option<String>,
        valid_target: Option<String>,
        combat_damage_only: bool,
    },
    /// A spell was countered (SP$ Counter).
    Countered {
        valid_card: Option<String>,
        valid_cause: Option<String>,
        valid_sa: Option<String>,
    },
    // ── New trigger modes (issue #19) ──
    /// A creature blocks an attacker.
    Blocks {
        valid_card: Option<String>,
        valid_blocked: Option<String>,
    },
    /// An attacker is blocked.
    AttackerBlocked {
        valid_card: Option<String>,
    },
    /// An attacker is not blocked.
    AttackerUnblocked {
        valid_card: Option<String>,
    },
    /// A player gained life.
    LifeGained {
        valid_player: Option<String>,
        valid_source: Option<String>,
        first_time_only: bool,
        spell_only: bool,
    },
    /// A player lost life.
    LifeLost {
        valid_player: Option<String>,
        first_time_only: bool,
    },
    /// A counter was added to a permanent.
    CounterAdded {
        valid_card: Option<String>,
        counter_type: Option<String>,
    },
    /// A counter was removed from a permanent.
    CounterRemoved {
        valid_card: Option<String>,
        counter_type: Option<String>,
    },
    /// A permanent was sacrificed.
    Sacrificed {
        valid_card: Option<String>,
        valid_player: Option<String>,
    },
    /// A creature adapted.
    Adapt {
        valid_card: Option<String>,
    },
    /// A creature became renowned.
    BecomeRenowned {
        valid_card: Option<String>,
    },
    /// A creature evolved.
    Evolved {
        valid_card: Option<String>,
    },
    /// A card was discarded.
    Discarded {
        valid_card: Option<String>,
        valid_player: Option<String>,
        valid_cause: Option<String>,
    },
    /// A scheme was abandoned.
    Abandoned {
        valid_card: Option<String>,
    },
    /// A card was drawn.
    Drawn {
        valid_card: Option<String>,
        valid_player: Option<String>,
        /// `Number$ N` — only fires when this is exactly the Nth card drawn this turn.
        number: Option<i32>,
    },
    /// Echo upkeep check.
    PayEcho {
        valid_card: Option<String>,
        paid: Option<bool>,
    },
    /// A class level was gained.
    ClassLevelGained {
        valid_card: Option<String>,
        class_level: Option<i32>,
    },
    /// A card was milled.
    Milled {
        valid_card: Option<String>,
        valid_player: Option<String>,
    },
    /// Cards were milled in one event.
    MilledAll {
        valid_card: Option<String>,
    },
    /// Cards were milled once for a player.
    MilledOnce {
        valid_card: Option<String>,
        valid_player: Option<String>,
    },
    /// A permanent was tapped.
    Taps {
        valid_card: Option<String>,
        valid_cause: Option<String>,
        valid_player: Option<String>,
        attacker: Option<bool>,
        require_first_time: bool,
    },
    /// A permanent was untapped.
    Untaps {
        valid_card: Option<String>,
    },
    /// A DFC was transformed.
    Transformed {
        valid_card: Option<String>,
    },
    /// A face-down creature was turned face up (Morph/Megamorph).
    TurnFaceUp {
        valid_card: Option<String>,
    },
    /// An aura/equipment was attached.
    Attached {
        valid_card: Option<String>,
    },
    /// An aura/equipment was unattached.
    Unattached {
        valid_card: Option<String>,
    },
    /// A land was played.
    LandPlayed {
        valid_card: Option<String>,
    },
    /// A permanent became the target of a spell or ability.
    BecomesTarget {
        valid_source: Option<String>,
        valid_target: Option<String>,
        require_first_time: bool,
        require_valiant: bool,
    },
    /// A permanent was tapped for mana.
    TapsForMana {
        valid_card: Option<String>,
        activator: Option<String>,
        produced: Option<String>,
    },
    /// An activated ability was activated.
    AbilityActivated {
        valid_card: Option<String>,
        valid_activating_player: Option<String>,
    },
    /// A creature explored.
    Explored {
        valid_card: Option<String>,
        valid_explored: Option<String>,
    },
    /// A creature became monstrous.
    BecomeMonstrous {
        valid_card: Option<String>,
    },
    /// A player became the monarch.
    BecomeMonarch {
        valid_player: Option<String>,
    },
    /// A player paid life.
    PayLife {
        valid_player: Option<String>,
    },
    /// Manifest Dread event.
    ManifestDread {
        valid_player: Option<String>,
    },
    /// A player lost the game.
    LosesGame {
        valid_player: Option<String>,
    },
    /// A player discovered.
    Discover {
        valid_player: Option<String>,
    },
    /// A gift was given.
    GiveGift {
        valid_player: Option<String>,
    },
    /// Cards were conjured in batch.
    ConjureAll {
        valid_player: Option<String>,
        valid_card: Option<String>,
    },
    /// Elementalbend trigger.
    Elementalbend {
        valid_player: Option<String>,
    },
    /// A planar die roll event.
    PlanarDice {
        valid_player: Option<String>,
        result: Option<String>,
    },
    /// A player investigated.
    Investigated {
        valid_player: Option<String>,
        first_time_only: bool,
    },
    /// A player proliferated.
    Proliferate {
        valid_player: Option<String>,
    },
    /// A player completed a dungeon.
    CompletedDungeon {
        valid_player: Option<String>,
    },
    /// A player committed a crime.
    CommitCrime {
        valid_player: Option<String>,
    },
    /// The Ring tempted a player.
    RingTemptsYou {
        valid_player: Option<String>,
        valid_card: Option<String>,
    },
    /// A new game started.
    NewGame,
    /// Day/night state changed.
    DayTimeChanges,
    /// A card became plotted.
    BecomesPlotted {
        valid_card: Option<String>,
    },
    /// A card specialized.
    Specializes {
        valid_card: Option<String>,
    },
    /// A card trained.
    Trains {
        valid_card: Option<String>,
    },
    /// A card devoured creatures.
    Devoured {
        valid_card: Option<String>,
    },
    /// A player visited an attraction.
    VisitAttraction {
        valid_player: Option<String>,
        valid_card: Option<String>,
    },
    /// A room was entered.
    EnteredRoom {
        valid_card: Option<String>,
        valid_room: Option<String>,
    },
    /// Cards were sought in batch.
    SeekAll {
        valid_player: Option<String>,
    },
    /// A card became crewed.
    BecomesCrewed {
        valid_card: Option<String>,
        valid_crew: Option<String>,
        first_time_crewed: bool,
        valid_crew_amount: Option<String>,
    },
    /// A card was championed.
    Championed {
        valid_card: Option<String>,
        valid_source: Option<String>,
    },
    /// A clash happened.
    Clashed {
        valid_player: Option<String>,
        won: Option<bool>,
    },
    /// A card was mentored by a source.
    Mentored {
        valid_card: Option<String>,
        valid_source: Option<String>,
    },
    /// A room/door became fully unlocked.
    FullyUnlock {
        valid_card: Option<String>,
        valid_player: Option<String>,
    },
    /// A spell ability resolved.
    AbilityResolves {
        valid_spell_ability: Option<String>,
        valid_source: Option<String>,
    },
    /// A spell ability triggered another trigger.
    AbilityTriggered {
        valid_mode: Option<String>,
        valid_destination: Option<String>,
        valid_spell_ability: Option<String>,
        valid_source: Option<String>,
        valid_cause: Option<String>,
        triggered_own_ability: bool,
    },
    /// A door was unlocked.
    UnlockDoor {
        valid_card: Option<String>,
        valid_player: Option<String>,
        this_door: bool,
    },
    /// Cards phased out (batch event).
    PhaseOutAll {
        valid_cards: Option<String>,
    },
    /// Vote trigger.
    Vote,
    /// Cumulative upkeep was paid (or not).
    PayCumulativeUpkeep {
        valid_card: Option<String>,
        paid: Option<bool>,
    },
    /// Damage was dealt to a player/creature for the first time this turn.
    DamageDealtOnce {
        valid_source: Option<String>,
        valid_target: Option<String>,
        combat_damage_only: bool,
    },
    /// A permanent was destroyed.
    Destroyed {
        valid_card: Option<String>,
        valid_causer: Option<String>,
    },
    /// A card was exiled.
    Exiled {
        valid_card: Option<String>,
    },
    /// A token was created.
    TokenCreated {
        valid_card: Option<String>,
    },
    /// A spell was copied (Storm, Replicate, etc.) — for Magecraft triggers.
    SpellCopied {
        valid_card: Option<String>,
        valid_activating_player: Option<String>,
    },
    SpellCopy {
        valid_card: Option<String>,
        valid_activating_player: Option<String>,
    },
    SpellAbilityCopy {
        valid_card: Option<String>,
        valid_activating_player: Option<String>,
    },
    SpellCastOrCopy {
        valid_card: Option<String>,
        valid_activating_player: Option<String>,
    },
    // ── New trigger modes (issue #54) ──
    AttackersDeclared {
        valid_player: Option<String>,
        valid_attackers: Option<String>,
        valid_attackers_amount: Option<String>,
        attacked_target: Option<String>,
        /// Mirrors Java's TriggerType enum: both `AttackersDeclared` and
        /// `AttackersDeclaredOneTarget` use the same `TriggerAttackersDeclared`
        /// class but are registered as separate event types. When true, this
        /// trigger only matches `TriggerType::AttackersDeclaredOneTarget` events.
        one_target: bool,
    },
    BlockersDeclared,
    ChangesZoneAll {
        origin: Option<ZoneType>,
        destination: Option<ZoneType>,
        valid_card: Option<String>,
        valid_cause: Option<String>,
        first_time_only: bool,
        valid_amount: Option<String>,
    },
    ChangesController {
        valid_card: Option<String>,
    },
    TurnBegin {
        valid_player: Option<String>,
    },
    DamageDoneOnce {
        valid_source: Option<String>,
        valid_target: Option<String>,
        combat_damage_only: bool,
    },
    DamageDoneOnceByController {
        valid_source: Option<String>,
        valid_target: Option<String>,
        combat_damage_only: bool,
    },
    SpellCastAll {
        valid_card: Option<String>,
        valid_activating_player: Option<String>,
    },
    LifeLostAll {
        valid_player: Option<String>,
    },
    CounterAddedOnce {
        valid_card: Option<String>,
        counter_type: Option<String>,
        valid_source: Option<String>,
    },
    CounterAddedAll {
        counter_type: Option<String>,
        valid: Option<String>,
    },
    CounterPlayerAddedAll {
        valid_source: Option<String>,
        valid_object: Option<String>,
        valid_object_to_source: Option<String>,
    },
    CounterTypeAddedAll {
        valid_object: Option<String>,
        first_time_only: bool,
    },
    DiscardedAll {
        valid_card: Option<String>,
        valid_player: Option<String>,
    },
    SacrificedOnce {
        valid_card: Option<String>,
        valid_player: Option<String>,
    },
    Cycled {
        valid_card: Option<String>,
        valid_player: Option<String>,
    },
    PhasedIn {
        valid_card: Option<String>,
    },
    PhasedOut {
        valid_card: Option<String>,
    },
    Always,
    Immediate,
    Surveil {
        valid_player: Option<String>,
    },
    Scry {
        valid_player: Option<String>,
    },
    Foretell {
        valid_card: Option<String>,
    },
    SearchedLibrary {
        valid_player: Option<String>,
    },
    Shuffled {
        valid_player: Option<String>,
    },
    ManaAdded {
        valid_source: Option<String>,
        valid_sa: Option<String>,
        player: Option<String>,
        produced: Option<String>,
    },
    TokenCreatedOnce {
        valid_card: Option<String>,
        only_first: Option<String>,
    },
    TapAll {
        valid_card: Option<String>,
    },
    UntapAll {
        valid_card: Option<String>,
    },
    BecomesTargetOnce {
        valid_card: Option<String>,
    },
    AttackerBlockedByCreature {
        valid_card: Option<String>,
        valid_blocked: Option<String>,
    },
    AttackerBlockedOnce {
        valid_card: Option<String>,
    },
    AttackerUnblockedOnce {
        valid_card: Option<String>,
    },
    SpellCastOnce {
        valid_card: Option<String>,
        valid_activating_player: Option<String>,
    },
    SpellCastOfType {
        valid_card: Option<String>,
        valid_activating_player: Option<String>,
    },
    DamageAll {
        valid_source: Option<String>,
        valid_target: Option<String>,
    },
    DamagePreventedOnce {
        valid_card: Option<String>,
    },
    ExcessDamage {
        valid_source: Option<String>,
        valid_target: Option<String>,
    },
    ExcessDamageAll {
        valid_target: Option<String>,
        combat_damage_only: bool,
    },
    CounterRemovedOnce {
        valid_card: Option<String>,
        counter_type: Option<String>,
    },
    /// A creature was exerted (Mirrors Java TriggerExerted).
    Exerted {
        valid_card: Option<String>,
    },
    /// A player collected evidence.
    CollectEvidence {
        valid_player: Option<String>,
    },
    /// A player foraged.
    Forage {
        valid_player: Option<String>,
    },
    /// A creature enlisted another creature.
    Enlisted {
        valid_card: Option<String>,
        valid_enlisted: Option<String>,
    },
    /// A player flipped a coin.
    FlippedCoin {
        valid_player: Option<String>,
        valid_result: Option<String>,
    },
    /// A player rolled a die.
    RolledDie {
        valid_player: Option<String>,
        valid_result: Option<String>,
        valid_sides: Option<String>,
        number: Option<i32>,
        natural: bool,
        rolled_to_visit_attractions: bool,
    },
    /// A player completed a die-roll action.
    RolledDieOnce {
        valid_player: Option<String>,
        valid_result: Option<String>,
        valid_sides: Option<String>,
        rolled_to_visit_attractions: bool,
    },
    /// Mana was expended (Expend N mechanic). Fires when cumulative mana spent reaches Amount.
    ManaExpend {
        valid_player: Option<String>,
        amount: i32,
    },
    /// A creature was exploited (Exploit keyword).
    Exploited {
        valid_card: Option<String>,
        valid_source: Option<String>,
    },
    /// A creature mutated onto another (IKO).
    Mutates {
        valid_card: Option<String>,
    },
    /// A scheme was set in motion (Archenemy).
    SetInMotion {
        valid_card: Option<String>,
    },
    /// A Case enchantment was solved.
    CaseSolved {
        valid_card: Option<String>,
        valid_player: Option<String>,
    },
    /// A player claimed a prize from an attraction.
    ClaimPrize {
        valid_player: Option<String>,
        valid_card: Option<String>,
    },
    /// A player took the initiative.
    TakesInitiative {
        valid_player: Option<String>,
    },
    /// A player planeswalked from a plane.
    PlaneswalkedFrom {
        valid_card: Option<String>,
    },
    /// A player planeswalked to a plane.
    PlaneswalkedTo {
        valid_card: Option<String>,
    },
    /// A contraption was cranked.
    CrankContraption {
        valid_card: Option<String>,
        valid_player: Option<String>,
    },
    /// Chaos ensues (Planechase).
    ChaosEnsues {
        valid_player: Option<String>,
    },
    /// A permanent became saddled.
    BecomesSaddled {
        valid_saddled: Option<String>,
        first_time_saddled: bool,
    },
    /// A permanent was crewed/saddled/stationed.
    CrewedSaddled {
        valid_card: Option<String>,
        valid_crew: Option<String>,
    },
}

impl Trigger {
    /// Java parity shim for Trigger.resetIDs().
    pub fn reset_i_ds(next_id: &mut u32) {
        *next_id = 50_000;
    }

    /// Minimal `ABILITY` replacement parity used by stack text paths.
    pub fn replace_ability_text(
        &self,
        desc: &str,
        game: &GameState,
        host_card: CardId,
        activating_player: PlayerId,
    ) -> String {
        if !desc.contains("ABILITY") {
            return desc.to_string();
        }
        let ability_desc = self
            .ensure_ability(game, host_card, activating_player)
            .map(|sa| {
                if sa.description.is_empty() {
                    sa.ability_text
                } else {
                    sa.description
                }
            })
            .unwrap_or_else(|| "<take no action>".to_string());
        desc.replace("ABILITY", ability_desc.trim())
    }

    /// Mirrors Java Trigger.phasesCheck() for common phase/turn params.
    pub fn phases_check(&self, game: &GameState, host_card: CardId) -> bool {
        let phase = game.turn.phase;
        let host_controller = game.card(host_card).controller;

        if let Some(phase_text) = self.params.get(keys::PHASE) {
            let mut any_match = false;
            for token in phase_text.split(',').map(|s| s.trim()) {
                if let Some(parsed) = parse_phase(token) {
                    if parsed == phase {
                        any_match = true;
                        break;
                    }
                }
            }
            if !any_match {
                return false;
            }
            if let Some(phase_count) = self.params.get("PhaseCount") {
                let expected = phase_count.parse::<i32>().unwrap_or(1);
                let current = if phase == PhaseType::Main2 { 2 } else { 1 };
                if current != expected {
                    return false;
                }
            }
        }

        if self.params.has(keys::PLAYER_TURN) && game.turn.active_player != host_controller {
            return false;
        }
        if self.params.has("NotPlayerTurn") && game.turn.active_player == host_controller {
            return false;
        }
        if self.params.has("OpponentTurn") {
            let active = game.turn.active_player;
            let is_opponent_turn = active != host_controller;
            if !is_opponent_turn {
                return false;
            }
        }
        if self.params.has("FirstUpkeep")
            && !(phase == PhaseType::Upkeep && game.turn.turn_number >= 1)
        {
            return false;
        }
        if self.params.has("FirstUpkeepThisGame")
            && !(phase == PhaseType::Upkeep && game.turn.turn_number == 1)
        {
            return false;
        }
        if self.params.has("FirstCombat") && phase != PhaseType::CombatBegin {
            return false;
        }
        if let Some(turn_count) = self.params.get("TurnCount") {
            let expected = turn_count.parse::<u32>().unwrap_or(game.turn.turn_number);
            if game.turn.turn_number != expected {
                return false;
            }
        }
        true
    }

    /// Mirrors Java Trigger.requirementsCheck() subset used in current engine.
    pub fn requirements_check(&self, game: &GameState, host_card: CardId) -> bool {
        if self.params.has("APlayerHasMoreLifeThanEachOther") {
            let mut highest = i32::MIN;
            let mut count = 0;
            for p in &game.players {
                if p.life > highest {
                    highest = p.life;
                    count = 1;
                } else if p.life == highest {
                    count += 1;
                }
            }
            if count != 1 {
                return false;
            }
        }
        if self.params.has("APlayerHasMostCardsInHand") {
            let mut largest = i32::MIN;
            let mut count = 0;
            for p in &game.players {
                let hand_count = game.cards_in_zone(ZoneType::Hand, p.id).len() as i32;
                if hand_count > largest {
                    largest = hand_count;
                    count = 1;
                } else if hand_count == largest {
                    count += 1;
                }
            }
            if count != 1 {
                return false;
            }
        }
        let host = game.card(host_card);
        if !valid_filter::check_is_present(game, &self.params, host) {
            return false;
        }
        self.check_resolved_limit(game, host_card)
    }

    /// Mirrors Java Trigger.checkResolvedLimit() (approximation with per-card counter).
    pub fn check_resolved_limit(&self, game: &GameState, host_card: CardId) -> bool {
        if let Some(limit) = self
            .params
            .get("ResolvedLimit")
            .and_then(|v| v.parse::<u32>().ok())
        {
            return game.card(host_card).ability_resolved_this_turn < limit;
        }
        true
    }

    /// Mirrors Java Trigger.checkActivationLimit().
    pub fn check_activation_limit(&self, game: &GameState, host_card: CardId) -> bool {
        if let Some(limit) = self
            .params
            .get("ActivationLimit")
            .and_then(|v| v.parse::<u32>().ok())
        {
            if game.card(host_card).ability_activated_this_turn >= limit {
                return false;
            }
        }
        if let Some(limit) = self
            .params
            .get(keys::GAME_ACTIVATION_LIMIT)
            .and_then(|v| v.parse::<u32>().ok())
        {
            let used = game
                .card(host_card)
                .activations_this_game
                .values()
                .copied()
                .sum::<u32>();
            if used >= limit {
                return false;
            }
        }
        true
    }

    /// Mirrors Java Trigger.meetsRequirementsOnTriggeredObjects() subset.
    pub fn meets_requirements_on_triggered_objects(
        &self,
        game: &GameState,
        run_params: &RunParams,
        host_card: CardId,
    ) -> bool {
        let Some(condition) = self.params.get(keys::CONDITION) else {
            return true;
        };
        match condition {
            "Evolve" => {
                let Some(moved) = run_params.card else {
                    return false;
                };
                let moved_card = game.card(moved);
                let host = game.card(host_card);
                moved_card.is_creature()
                    && host.is_creature()
                    && (moved_card.power() > host.power()
                        || moved_card.toughness() > host.toughness())
            }
            "LifePaid" => run_params
                .spell_ability
                .as_ref()
                .map(|sa| {
                    sa.params
                        .get("LifeAmount")
                        .and_then(|v| v.parse::<i32>().ok())
                        .unwrap_or(0)
                        > 0
                })
                .unwrap_or(false),
            "Sacrificed" => run_params
                .spell_ability
                .as_ref()
                .map(|sa| {
                    !sa.paid_hash
                        .get("Sacrificed")
                        .cloned()
                        .unwrap_or_default()
                        .is_empty()
                })
                .unwrap_or(false),
            _ => true,
        }
    }

    pub fn add_remembered<T: Into<AbilityValue>>(&mut self, item: T) {
        self.trigger_remembered.push(item.into());
    }

    pub fn is_static(&self) -> bool {
        self.static_trigger
    }

    /// Mirrors Java's `Trigger.isManaAbility()`.
    /// A trigger is a mana ability only if its mode is TapsForMana or ManaAdded
    /// AND the resulting SpellAbility itself is a mana ability (has a mana part).
    pub fn is_mana_ability(
        &self,
        game: &GameState,
        host_card: CardId,
        activating_player: PlayerId,
    ) -> bool {
        if !matches!(
            self.mode,
            TriggerMode::TapsForMana { .. } | TriggerMode::ManaAdded { .. }
        ) {
            return false;
        }
        self.ensure_ability(game, host_card, activating_player)
            .map_or(false, |sa| sa.is_mana_ability)
    }

    pub fn add_remembered_many<T: Into<AbilityValue>, I: IntoIterator<Item = T>>(
        &mut self,
        items: I,
    ) {
        for item in items {
            self.add_remembered(item);
        }
    }

    pub fn copy(&self, next_id: &mut u32, keep_id: bool) -> Self {
        let mut out = self.clone();
        if !keep_id {
            out.id = *next_id;
            *next_id = next_id.saturating_add(1);
        }
        out
    }

    /// Tracks trigger activation on the host card.
    pub fn trigger_run(&self, game: &mut GameState, host_card: CardId) {
        game.card_mut(host_card).add_ability_activated();
    }

    /// Ensures trigger execute ability can be built from host SVar.
    pub fn ensure_ability(
        &self,
        game: &GameState,
        host_card: CardId,
        activating_player: PlayerId,
    ) -> Option<SpellAbility> {
        if self.execute.is_empty() {
            return None;
        }
        let host = game.card(host_card);
        let ability_text = host.svars.get(&self.execute)?;
        Some(build_spell_ability(
            game,
            host_card,
            ability_text,
            activating_player,
        ))
    }

    pub fn set_triggering_objects(
        &self,
        sa: &mut SpellAbility,
        params: &RunParams,
        game: &GameState,
        host_card: CardId,
        host_controller: PlayerId,
    ) {
        add_ability_trigger_metadata(sa, self, params, game, host_card, host_controller);
        add_common_trigger_objects(sa, params);
        self.mode
            .set_triggering_objects(sa, params, game, host_card, host_controller);
    }

    pub fn build_triggered_spell_ability(
        &self,
        game: &GameState,
        host_card: CardId,
        host_controller: PlayerId,
        trigger_index: usize,
        params: &RunParams,
    ) -> SpellAbility {
        let svar_text = game
            .card(host_card)
            .svars
            .get(&self.execute)
            .cloned()
            .unwrap_or_default();
        let mut sa = build_spell_ability(game, host_card, &svar_text, host_controller);
        sa.is_trigger = true;
        sa.trigger_source = Some(host_card);
        sa.trigger_source_zone_timestamp = Some(game.card(host_card).zone_timestamp);
        sa.source_trigger_id = Some(self.id);
        sa.trigger_index = Some(trigger_index);
        sa.trigger_remembered = self.trigger_remembered.clone();
        self.set_triggering_objects(&mut sa, params, game, host_card, host_controller);
        self.configure_triggered_spell_ability(&mut sa, params, game, &svar_text);
        sa
    }

    fn configure_triggered_spell_ability(
        &self,
        sa: &mut SpellAbility,
        params: &RunParams,
        game: &GameState,
        svar_text: &str,
    ) {
        if let Some(pid) = params.damage_target_player {
            sa.target_chosen.target_player = Some(pid);
        }
        if sa.target_chosen.target_player.is_none() && svar_text.contains("TriggeredPlayer") {
            if let Some(pid) = params.player {
                sa.target_chosen.target_player = Some(pid);
            }
        }
        if let Some(pid) = params.defending_player {
            if sa.target_chosen.target_player.is_none() && svar_text.contains("DefendingPlayer") {
                sa.target_chosen.target_player = Some(pid);
            }
        }
        if let Some(cid) = params.damage_target_card {
            sa.target_chosen.target_card = Some(cid);
        }
        if let Some(cause_cid) = params.cause_card {
            if let Some(entry) = game.stack.find_by_source_card(cause_cid) {
                sa.target_chosen.target_stack_entry = Some(entry.id);
            }
        }
        if let Some(attacker_id) = params.attacker {
            if svar_text.contains("TriggeredAttacker") {
                sa.target_chosen.target_card = Some(attacker_id);
            }
        }
        if let Some(blocker_id) = params.blocker {
            if svar_text.contains("TriggeredBlocker") {
                sa.target_chosen.target_card = Some(blocker_id);
            }
        }
        if let Some(lki_p1p1) = params.lki_p1p1_counters {
            if self.execute.contains("Modular") || svar_text.contains("Modular") {
                sa.trigger_remembered_amount = lki_p1p1;
            }
        }
    }
}

fn trigger_mode_name(mode: &TriggerMode) -> String {
    let dbg = format!("{mode:?}");
    dbg.split(|c: char| c == '{' || c.is_whitespace())
        .next()
        .unwrap_or("Unknown")
        .to_string()
}

fn destination_names(params: &RunParams) -> Option<String> {
    if let Some(destinations) = params.destinations.as_ref() {
        return Some(destinations.clone());
    }
    if let Some(zone_changes) = params.zone_changes.as_ref() {
        let mut ordered = Vec::new();
        for change in zone_changes {
            let name = format!("{:?}", change.destination);
            if !ordered.contains(&name) {
                ordered.push(name);
            }
        }
        if !ordered.is_empty() {
            return Some(ordered.join(","));
        }
    }
    params
        .destination
        .map(|destination| format!("{destination:?}"))
}

fn cause_cards_for_ability_triggered(
    trigger: &Trigger,
    params: &RunParams,
    game: &GameState,
    host_card: CardId,
    host_controller: PlayerId,
) -> Vec<CardId> {
    match &trigger.mode {
        TriggerMode::ChangesZone { .. } => params.card.into_iter().collect(),
        TriggerMode::ChangesZoneAll { .. } => params.cards.clone().unwrap_or_default(),
        TriggerMode::Attacks { .. } => params.attacker.into_iter().collect(),
        TriggerMode::AttackersDeclared {
            valid_attackers, ..
        } => params
            .attacker_ids
            .clone()
            .unwrap_or_default()
            .into_iter()
            .filter(|&attacker| {
                valid_attackers.as_ref().is_none_or(|filter| {
                    crate::trigger::matches_valid_card(
                        filter,
                        attacker,
                        host_card,
                        host_controller,
                        game,
                    )
                })
            })
            .collect(),
        _ => params
            .cards
            .clone()
            .or_else(|| params.card.map(|card| vec![card]))
            .unwrap_or_default(),
    }
}

fn add_ability_trigger_metadata(
    sa: &mut SpellAbility,
    trigger: &Trigger,
    params: &RunParams,
    game: &GameState,
    host_card: CardId,
    host_controller: PlayerId,
) {
    sa.add_triggering_object("AbilityTriggeredMode", &trigger_mode_name(&trigger.mode));
    if let Some(destinations) = destination_names(params) {
        sa.add_triggering_object("AbilityTriggeredDestinations", &destinations);
    }
    let cause_cards =
        cause_cards_for_ability_triggered(trigger, params, game, host_card, host_controller);
    if !cause_cards.is_empty() {
        let csv = cause_cards
            .iter()
            .map(|cid| cid.0.to_string())
            .collect::<Vec<_>>()
            .join(",");
        sa.add_triggering_object("AbilityTriggeredCauseCards", &csv);
    }
}

fn add_common_trigger_objects(sa: &mut SpellAbility, params: &RunParams) {
    if let Some(card_id) = params.card {
        sa.add_triggering_object("Card", &card_id.0.to_string());
        sa.add_triggering_object("NewCard", &card_id.0.to_string());
    }
    if let Some(card_id) = params.card_lki {
        sa.add_triggering_object("CardLKI", &card_id.0.to_string());
    }
    if let Some(player_id) = params.activator.or(params.cause_player) {
        sa.add_triggering_object("Activator", &player_id.0.to_string());
    }
    if let Some(player_id) = params.player {
        sa.add_triggering_object("Player", &player_id.0.to_string());
    }
    if let Some(player_id) = params.attacking_player {
        sa.add_triggering_object("AttackingPlayer", &player_id.0.to_string());
    }
    if let Some(player_id) = params.defending_player {
        sa.add_triggering_object("DefendingPlayer", &player_id.0.to_string());
    }
    if let Some(card_id) = params.causer.or(params.cause_card) {
        sa.add_triggering_object("Causer", &card_id.0.to_string());
    }
    if let Some(card_id) = params.source_card.or(params.spell_card) {
        sa.add_triggering_object("Source", &card_id.0.to_string());
    }
    if let Some(card_id) = params.attacker {
        sa.add_triggering_object("Attacker", &card_id.0.to_string());
    }
    if let Some(card_id) = params.blocker {
        sa.add_triggering_object("Blocker", &card_id.0.to_string());
    }
    if let Some(card_id) = params.attacked_card {
        sa.add_triggering_object("Attacked", &card_id.0.to_string());
    }
    if let Some(player_id) = params.attacked_player {
        sa.add_triggering_object("AttackedTarget", &player_id.0.to_string());
    }
    if let Some(card_id) = params.target_card {
        let value = card_id.0.to_string();
        sa.add_triggering_object("Target", &value);
        sa.add_triggering_object("TargetCard", &value);
    }
    if let Some(player_id) = params.target_player {
        let value = player_id.0.to_string();
        sa.add_triggering_object("Target", &value);
        sa.add_triggering_object("TargetPlayer", &value);
    }
    // Java DamageDone triggers expose the damaged entity as AbilityKey.Target.
    // Mirror that here so TriggeredTarget resolves against trigger objects
    // instead of depending on target_chosen fallback behavior.
    if params.target_player.is_none() {
        if let Some(player_id) = params.damage_target_player {
            let value = player_id.0.to_string();
            sa.add_triggering_object("Target", &value);
            sa.add_triggering_object("TargetPlayer", &value);
        }
    }
    if params.target_card.is_none() {
        if let Some(card_id) = params.damage_target_card {
            let value = card_id.0.to_string();
            sa.add_triggering_object("Target", &value);
            sa.add_triggering_object("TargetCard", &value);
        }
    }
    if let Some(card_id) = params.explored {
        sa.add_triggering_object("Explored", &card_id.0.to_string());
    }
    if let Some(cards) = params.cards.as_deref() {
        let csv = cards
            .iter()
            .map(|card_id| card_id.0.to_string())
            .collect::<Vec<_>>()
            .join(",");
        if !csv.is_empty() {
            sa.add_triggering_object("Cards", &csv);
        }
    }
    if let Some(cards) = params.attacker_ids.as_deref() {
        let csv = cards
            .iter()
            .map(|card_id| card_id.0.to_string())
            .collect::<Vec<_>>()
            .join(",");
        if !csv.is_empty() {
            sa.add_triggering_object("Attackers", &csv);
        }
    }
    if let Some(value) = params.life_amount {
        sa.add_triggering_object("LifeAmount", &value.to_string());
    }
    if let Some(value) = params.natural_result {
        sa.add_triggering_object("NaturalResult", &value.to_string());
    }
    if let Some(value) = params.card_state_name.as_deref() {
        sa.add_triggering_object("CardState", value);
    }
    if let Some(value) = params.room_name.as_deref() {
        sa.add_triggering_object("RoomName", value);
    }
    if let Some(value) = params.spell_ability.as_ref() {
        sa.add_triggering_spell_ability("SpellAbility", value.clone());
    }
    if let Some(value) = params.source_sa.as_ref() {
        sa.add_triggering_spell_ability("SourceSA", value.clone());
    }
    if let Some(value) = params.ability_mana.as_ref() {
        sa.add_triggering_spell_ability("AbilityMana", value.clone());
    }
    if let Some(value) = params.cause.as_ref() {
        sa.add_triggering_spell_ability("Cause", value.clone());
    }
    if let Some(results) = params.die_results.as_deref() {
        let csv = results
            .iter()
            .map(i32::to_string)
            .collect::<Vec<_>>()
            .join(",");
        if !csv.is_empty() {
            sa.add_triggering_object("Result", &csv);
        }
    } else if let Some(value) = params.die_result {
        sa.add_triggering_object("Result", &value.to_string());
    }
    if let Some(value) = params.die_sides {
        sa.add_triggering_object("Sides", &value.to_string());
    }
    if let Some(value) = params.number {
        sa.add_triggering_object("Number", &value.to_string());
    }
}

/// Check an optional ValidCard$ filter against a card from RunParams.
/// Returns false if the filter exists but the card doesn't match.
pub(crate) fn check_card_filter(
    filter: &Option<String>,
    card_id: Option<CardId>,
    host_card: CardId,
    host_controller: PlayerId,
    game: &GameState,
) -> bool {
    if let Some(filter) = filter {
        if let Some(cid) = card_id {
            matches_valid_card(filter, cid, host_card, host_controller, game)
        } else {
            false
        }
    } else {
        true
    }
}

/// Check an optional ValidPlayer$ filter against a player from RunParams.
/// Returns false if the filter exists but the player doesn't match.
pub(crate) fn check_player_filter(
    filter: &Option<String>,
    player_id: Option<PlayerId>,
    host_controller: PlayerId,
) -> bool {
    if let Some(filter) = filter {
        if let Some(pid) = player_id {
            matches_valid_player(filter, pid, host_controller)
        } else {
            false
        }
    } else {
        true
    }
}

/// Check an optional CounterType$ filter against a counter type from RunParams.
pub(crate) fn check_counter_type_filter(
    expected: &Option<String>,
    actual: &Option<String>,
) -> bool {
    if let Some(expected) = expected {
        if let Some(actual) = actual {
            actual.eq_ignore_ascii_case(expected)
        } else {
            false
        }
    } else {
        true
    }
}

/// Check a damage target filter that can match either a card or player target.
/// `strict_card_filter` controls whether card-specific filters (Card., Creature., etc.)
/// reject player targets.
pub(crate) fn check_damage_target(
    filter: &Option<String>,
    run_params: &RunParams,
    host_card: CardId,
    host_controller: PlayerId,
    game: &GameState,
    strict_card_filter: bool,
) -> bool {
    let filter = match filter {
        Some(f) => f,
        None => return true,
    };
    if let Some(target_card) = run_params.damage_target_card {
        matches_valid_card(filter, target_card, host_card, host_controller, game)
    } else if strict_card_filter {
        let is_card_filter = filter.starts_with("Card.")
            || filter.starts_with("Creature.")
            || filter.starts_with("Permanent.")
            || filter.starts_with("Artifact.")
            || filter.starts_with("Enchantment.")
            || filter.starts_with("Planeswalker.");
        if is_card_filter {
            false
        } else if let Some(target_player) = run_params.damage_target_player {
            matches_valid_player(filter, target_player, host_controller)
        } else {
            false
        }
    } else if let Some(target_player) = run_params.damage_target_player {
        matches_valid_player(filter, target_player, host_controller)
    } else {
        false
    }
}

/// Check zone matches (origin and/or destination).
pub(crate) fn check_zone_filter(expected: &Option<ZoneType>, actual: Option<ZoneType>) -> bool {
    if let Some(expected) = expected {
        actual == Some(*expected)
    } else {
        true
    }
}

impl TriggerMode {
    /// Mirrors Java's Trigger.performTest() — each subclass overrides.
    /// In Rust, enum match replaces virtual dispatch.
    pub fn perform_test(
        &self,
        run_params: &RunParams,
        game: &GameState,
        host_card: CardId,
        host_controller: PlayerId,
        current_trigger_id: Option<u32>,
    ) -> bool {
        match self {
            TriggerMode::Attacks { .. } => {
                return super::trigger_attacks::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::AttackerBlocked { .. } => {
                return super::trigger_attacker_blocked::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::AttackerUnblocked { .. } => {
                return super::trigger_attacker_unblocked::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::AttackerBlockedByCreature { .. } => {
                return super::trigger_attacker_blocked_by_creature::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::AttackerBlockedOnce { .. } => {
                return super::trigger_attacker_blocked_once::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::AttackerUnblockedOnce { .. } => {
                return super::trigger_attacker_unblocked_once::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::Blocks { .. } => {
                return super::trigger_blocks::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::Phase { .. } => {
                return super::trigger_phase::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::ChangesZone { .. } => {
                return super::trigger_changes_zone::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                    current_trigger_id,
                )
            }
            TriggerMode::ChangesZoneAll { .. } => {
                return super::trigger_changes_zone_all::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::Drawn { .. } => {
                return super::trigger_drawn::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::LifeGained { .. } => {
                return super::trigger_life_gained::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::LifeLost { .. } => {
                return super::trigger_life_lost::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::LifeLostAll { .. } => {
                return super::trigger_life_lost_all::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::CounterAdded { .. } => {
                return super::trigger_counter_added::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::CounterAddedOnce { .. } => {
                return super::trigger_counter_added_once::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::CounterRemoved { .. } => {
                return super::trigger_counter_removed::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::CounterRemovedOnce { .. } => {
                return super::trigger_counter_removed_once::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::Countered { .. } => {
                return super::trigger_countered::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::Fight { .. } => {
                return super::trigger_fight::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::FightOnce { .. } => {
                return super::trigger_fight_once::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::DamageDealtOnce { .. } => {
                return super::trigger_damage_dealt_once::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::AttackersDeclared { .. } => {
                return super::trigger_attackers_declared::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::BlockersDeclared => {
                return super::trigger_blockers_declared::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::Sacrificed { .. } => {
                return super::trigger_sacrificed::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::SacrificedOnce { .. } => {
                return super::trigger_sacrificed_once::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::BecomesTarget { .. } => {
                return super::trigger_becomes_target::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::Taps { .. } => {
                return super::trigger_taps::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::Untaps { .. } => {
                return super::trigger_untaps::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::TapsForMana { .. } => {
                return super::trigger_taps_for_mana::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::TapAll { .. } => {
                return super::trigger_tap_all::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::UntapAll { .. } => {
                return super::trigger_untap_all::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::LandPlayed { .. } => {
                return super::trigger_land_played::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::Destroyed { .. } => {
                return super::trigger_destroyed::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::Exiled { .. } => {
                return super::trigger_exiled::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::Explored { .. } => {
                return super::trigger_explores::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::Transformed { .. } => {
                return super::trigger_transformed::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::TurnFaceUp { .. } => {
                return super::trigger_turn_face_up::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::TokenCreated { .. } => {
                return super::trigger_token_created::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::TokenCreatedOnce { .. } => {
                return super::trigger_token_created_once::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::TurnBegin { .. } => {
                return super::trigger_turn_begin::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::BecomeMonarch { .. } => {
                return super::trigger_become_monarch::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::Surveil { .. } => {
                return super::trigger_surveil::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::Scry { .. } => {
                return super::trigger_scry::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::ManaAdded { .. } => {
                return super::trigger_mana_added::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::ManaExpend { .. } => {
                return super::trigger_mana_expend::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::Mutates { .. } => {
                return super::trigger_mutates::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::SetInMotion { .. } => {
                return super::trigger_set_in_motion::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::CaseSolved { .. } => {
                return super::trigger_case_solved::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::ClaimPrize { .. } => {
                return super::trigger_claim_prize::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::TakesInitiative { .. } => {
                return super::trigger_takes_initiative::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::Discarded { .. } => {
                return super::trigger_discarded::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::PayCumulativeUpkeep { .. } => {
                return super::trigger_pay_cumulative_upkeep::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::ChaosEnsues { .. } => {
                return super::trigger_chaos_ensues::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::BecomesSaddled { .. } => {
                return super::trigger_becomes_saddled::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::PlaneswalkedFrom { .. } => {
                return super::trigger_planeswalked_from::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::PlaneswalkedTo { .. } => {
                return super::trigger_planeswalked_to::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::CrankContraption { .. } => {
                return super::trigger_crank_contraption::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::Adapt { .. } => {
                return super::trigger_adapt::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::BecomeRenowned { .. } => {
                return super::trigger_become_renowned::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::Evolved { .. } => {
                return super::trigger_evolved::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::Investigated { .. } => {
                return super::trigger_investigated::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::Proliferate { .. } => {
                return super::trigger_proliferate::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::CompletedDungeon { .. } => {
                return super::trigger_completed_dungeon::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::CommitCrime { .. } => {
                return super::trigger_commit_crime::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::RingTemptsYou { .. } => {
                return super::trigger_ring_tempts_you::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::PayLife { .. } => {
                return super::trigger_pay_life::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::PayEcho { .. } => {
                return super::trigger_pay_echo::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::ClassLevelGained { .. } => {
                return super::trigger_class_level_gained::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::BecomesPlotted { .. } => {
                return super::trigger_becomes_plotted::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::NewGame => {
                return super::trigger_new_game::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::DayTimeChanges => {
                return super::trigger_day_time_changes::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::LosesGame { .. } => {
                return super::trigger_loses_game::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::Discover { .. } => {
                return super::trigger_discover::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::Elementalbend { .. } => {
                return super::trigger_elementalbend::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::PlanarDice { .. } => {
                return super::trigger_planar_dice::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::PhaseOutAll { .. } => {
                return super::trigger_phase_out_all::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::Vote => {
                return super::trigger_vote::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::GiveGift { .. } => {
                return super::trigger_give_gift::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::VisitAttraction { .. } => {
                return super::trigger_visit_attraction::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::EnteredRoom { .. } => {
                return super::trigger_entered_room::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::MilledAll { .. } => {
                return super::trigger_milled_all::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::MilledOnce { .. } => {
                return super::trigger_milled_once::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::Abandoned { .. } => {
                return super::trigger_abandoned::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::ManifestDread { .. } => {
                return super::trigger_manifest_dread::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::Specializes { .. } => {
                return super::trigger_specializes::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::Trains { .. } => {
                return super::trigger_trains::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::Devoured { .. } => {
                return super::trigger_devoured::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::ConjureAll { .. } => {
                return super::trigger_conjure_all::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::SeekAll { .. } => {
                return super::trigger_seek_all::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::BecomesCrewed { .. } => {
                return super::trigger_becomes_crewed::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::Championed { .. } => {
                return super::trigger_championed::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::Clashed { .. } => {
                return super::trigger_clashed::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::Mentored { .. } => {
                return super::trigger_mentored::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::FullyUnlock { .. } => {
                return super::trigger_fully_unlock::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::AbilityResolves { .. } => {
                return super::trigger_ability_resolves::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::AbilityTriggered { .. } => {
                return super::trigger_ability_triggered::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::UnlockDoor { .. } => {
                return super::trigger_unlock_door::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::CounterAddedAll { .. } => {
                return super::trigger_counter_added_all::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::CounterPlayerAddedAll { .. } => {
                return super::trigger_counter_player_added_all::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::CounterTypeAddedAll { .. } => {
                return super::trigger_counter_type_added_all::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::DamageDoneOnceByController { .. } => {
                return super::trigger_damage_done_once_by_controller::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::ExcessDamageAll { .. } => {
                return super::trigger_excess_damage_all::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::CrewedSaddled { .. } => {
                return super::trigger_crewed_saddled::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::SpellCast { .. }
            | TriggerMode::AbilityCast { .. }
            | TriggerMode::SpellAbilityCast { .. }
            | TriggerMode::SpellCastAll { .. }
            | TriggerMode::SpellCastOnce { .. }
            | TriggerMode::SpellCastOfType { .. }
            | TriggerMode::SpellCopied { .. }
            | TriggerMode::SpellCopy { .. }
            | TriggerMode::SpellAbilityCopy { .. }
            | TriggerMode::SpellCastOrCopy { .. } => {
                return super::trigger_spell_ability_cast_or_copy::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::DamageDone { .. } => super::trigger_damage_done::perform_test(
                self,
                run_params,
                game,
                host_card,
                host_controller,
            ),
            TriggerMode::DamageDoneOnce { .. } => super::trigger_damage_done_once::perform_test(
                self,
                run_params,
                game,
                host_card,
                host_controller,
            ),
            TriggerMode::DamageAll { .. } => super::trigger_damage_all::perform_test(
                self,
                run_params,
                game,
                host_card,
                host_controller,
            ),
            TriggerMode::ExcessDamage { .. } => super::trigger_excess_damage::perform_test(
                self,
                run_params,
                game,
                host_card,
                host_controller,
            ),
            TriggerMode::Milled { .. } => super::trigger_milled::perform_test(
                self,
                run_params,
                game,
                host_card,
                host_controller,
            ),
            TriggerMode::DiscardedAll { .. } => super::trigger_discarded_all::perform_test(
                self,
                run_params,
                game,
                host_card,
                host_controller,
            ),
            TriggerMode::Cycled { .. } => super::trigger_cycled::perform_test(
                self,
                run_params,
                game,
                host_card,
                host_controller,
            ),
            TriggerMode::FlippedCoin { .. } => super::trigger_flipped_coin::perform_test(
                self,
                run_params,
                game,
                host_card,
                host_controller,
            ),
            TriggerMode::RolledDie { .. } => super::trigger_rolled_die::perform_test(
                self,
                run_params,
                game,
                host_card,
                host_controller,
            ),
            TriggerMode::RolledDieOnce { .. } => super::trigger_rolled_die_once::perform_test(
                self,
                run_params,
                game,
                host_card,
                host_controller,
            ),
            TriggerMode::AbilityActivated { .. } => super::trigger_ability_activated::perform_test(
                self,
                run_params,
                game,
                host_card,
                host_controller,
            ),
            TriggerMode::Attached { .. } => super::trigger_attached::perform_test(
                self,
                run_params,
                game,
                host_card,
                host_controller,
            ),
            TriggerMode::Unattached { .. } => super::trigger_unattach::perform_test(
                self,
                run_params,
                game,
                host_card,
                host_controller,
            ),
            TriggerMode::ChangesController { .. } => {
                super::trigger_changes_controller::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::PhasedIn { .. } => super::trigger_phase_in::perform_test(
                self,
                run_params,
                game,
                host_card,
                host_controller,
            ),
            TriggerMode::PhasedOut { .. } => super::trigger_phase_out::perform_test(
                self,
                run_params,
                game,
                host_card,
                host_controller,
            ),
            TriggerMode::Foretell { .. } => super::trigger_foretell::perform_test(
                self,
                run_params,
                game,
                host_card,
                host_controller,
            ),
            TriggerMode::BecomesTargetOnce { .. } => {
                super::trigger_becomes_target_once::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::BecomeMonstrous { .. } => super::trigger_become_monstrous::perform_test(
                self,
                run_params,
                game,
                host_card,
                host_controller,
            ),
            TriggerMode::DamagePreventedOnce { .. } => {
                super::trigger_damage_prevented_once::perform_test(
                    self,
                    run_params,
                    game,
                    host_card,
                    host_controller,
                )
            }
            TriggerMode::Exerted { .. } => super::trigger_exerted::perform_test(
                self,
                run_params,
                game,
                host_card,
                host_controller,
            ),
            TriggerMode::SearchedLibrary { .. } => super::trigger_searched_library::perform_test(
                self,
                run_params,
                game,
                host_card,
                host_controller,
            ),
            TriggerMode::Shuffled { .. } => super::trigger_shuffled::perform_test(
                self,
                run_params,
                game,
                host_card,
                host_controller,
            ),
            TriggerMode::CollectEvidence { .. } => super::trigger_collect_evidence::perform_test(
                self,
                run_params,
                game,
                host_card,
                host_controller,
            ),
            TriggerMode::Forage { .. } => super::trigger_forage::perform_test(
                self,
                run_params,
                game,
                host_card,
                host_controller,
            ),
            TriggerMode::Exploited { .. } => super::trigger_exploited::perform_test(
                self,
                run_params,
                game,
                host_card,
                host_controller,
            ),
            TriggerMode::Always => super::trigger_always::perform_test(
                self,
                run_params,
                game,
                host_card,
                host_controller,
            ),
            TriggerMode::Immediate => super::trigger_immediate::perform_test(),
            TriggerMode::Enlisted {
                valid_card,
                valid_enlisted,
            } => {
                // Mirrors Java TriggerEnlisted.performTest():
                // ValidCard$ checks the enlisting creature, ValidEnlisted$ checks the enlisted one.
                check_card_filter(
                    valid_card,
                    run_params.card,
                    host_card,
                    host_controller,
                    game,
                ) && check_card_filter(
                    valid_enlisted,
                    run_params.enlisted,
                    host_card,
                    host_controller,
                    game,
                )
            }
            _ => unreachable!("missing TriggerMode::perform_test dispatch for {:?}", self),
        }
    }

    pub fn trigger_type(&self) -> TriggerType {
        match self {
            TriggerMode::ChangesZone { .. } => TriggerType::ChangesZone,
            TriggerMode::Phase { .. } => TriggerType::Phase,
            TriggerMode::SpellCast { .. } => TriggerType::SpellCast,
            TriggerMode::AbilityCast { .. } => TriggerType::AbilityCast,
            TriggerMode::SpellAbilityCast { .. } => TriggerType::SpellAbilityCast,
            TriggerMode::Attacks { .. } => TriggerType::Attacks,
            TriggerMode::Fight { .. } => TriggerType::Fight,
            TriggerMode::FightOnce { .. } => TriggerType::FightOnce,
            TriggerMode::DamageDone { .. } => TriggerType::DamageDone,
            TriggerMode::Countered { .. } => TriggerType::Countered,
            TriggerMode::Blocks { .. } => TriggerType::Blocks,
            TriggerMode::AttackerBlocked { .. } => TriggerType::AttackerBlocked,
            TriggerMode::AttackerUnblocked { .. } => TriggerType::AttackerUnblocked,
            TriggerMode::LifeGained { .. } => TriggerType::LifeGained,
            TriggerMode::LifeLost { .. } => TriggerType::LifeLost,
            TriggerMode::PayLife { .. } => TriggerType::PayLife,
            TriggerMode::LosesGame { .. } => TriggerType::LosesGame,
            TriggerMode::Discover { .. } => TriggerType::Discover,
            TriggerMode::Elementalbend { .. } => TriggerType::Elementalbend,
            TriggerMode::Clashed { .. } => TriggerType::Clashed,
            TriggerMode::ManifestDread { .. } => TriggerType::ManifestDread,
            TriggerMode::ConjureAll { .. } => TriggerType::ConjureAll,
            TriggerMode::SeekAll { .. } => TriggerType::SeekAll,
            TriggerMode::CounterAdded { .. } => TriggerType::CounterAdded,
            TriggerMode::CounterRemoved { .. } => TriggerType::CounterRemoved,
            TriggerMode::Sacrificed { .. } => TriggerType::Sacrificed,
            TriggerMode::Discarded { .. } => TriggerType::Discarded,
            TriggerMode::Abandoned { .. } => TriggerType::Abandoned,
            TriggerMode::Adapt { .. } => TriggerType::Adapt,
            TriggerMode::BecomeRenowned { .. } => TriggerType::BecomeRenowned,
            TriggerMode::Evolved { .. } => TriggerType::Evolved,
            TriggerMode::Drawn { .. } => TriggerType::Drawn,
            TriggerMode::Milled { .. } => TriggerType::Milled,
            TriggerMode::MilledAll { .. } => TriggerType::MilledAll,
            TriggerMode::MilledOnce { .. } => TriggerType::MilledOnce,
            TriggerMode::PayEcho { .. } => TriggerType::PayEcho,
            TriggerMode::ClassLevelGained { .. } => TriggerType::ClassLevelGained,
            TriggerMode::Taps { .. } => TriggerType::Taps,
            TriggerMode::Untaps { .. } => TriggerType::Untaps,
            TriggerMode::Transformed { .. } => TriggerType::Transformed,
            TriggerMode::TurnFaceUp { .. } => TriggerType::TurnFaceUp,
            TriggerMode::Attached { .. } => TriggerType::Attached,
            TriggerMode::Unattached { .. } => TriggerType::Unattached,
            TriggerMode::LandPlayed { .. } => TriggerType::LandPlayed,
            TriggerMode::BecomesTarget { .. } => TriggerType::BecomesTarget,
            TriggerMode::BecomesCrewed { .. } => TriggerType::BecomesCrewed,
            TriggerMode::Championed { .. } => TriggerType::Championed,
            TriggerMode::Mentored { .. } => TriggerType::Mentored,
            TriggerMode::TapsForMana { .. } => TriggerType::TapsForMana,
            TriggerMode::AbilityActivated { .. } => TriggerType::AbilityActivated,
            TriggerMode::Explored { .. } => TriggerType::Explored,
            TriggerMode::BecomeMonstrous { .. } => TriggerType::BecomeMonstrous,
            TriggerMode::BecomeMonarch { .. } => TriggerType::BecomeMonarch,
            TriggerMode::Investigated { .. } => TriggerType::Investigated,
            TriggerMode::Proliferate { .. } => TriggerType::Proliferate,
            TriggerMode::CompletedDungeon { .. } => TriggerType::CompletedDungeon,
            TriggerMode::CommitCrime { .. } => TriggerType::CommitCrime,
            TriggerMode::RingTemptsYou { .. } => TriggerType::RingTemptsYou,
            TriggerMode::PlanarDice { .. } => TriggerType::PlanarDice,
            TriggerMode::NewGame => TriggerType::NewGame,
            TriggerMode::DayTimeChanges => TriggerType::DayTimeChanges,
            TriggerMode::BecomesPlotted { .. } => TriggerType::BecomesPlotted,
            TriggerMode::Specializes { .. } => TriggerType::Specializes,
            TriggerMode::Trains { .. } => TriggerType::Trains,
            TriggerMode::Devoured { .. } => TriggerType::Devoured,
            TriggerMode::FullyUnlock { .. } => TriggerType::FullyUnlock,
            TriggerMode::AbilityResolves { .. } => TriggerType::AbilityResolves,
            TriggerMode::AbilityTriggered { .. } => TriggerType::AbilityTriggered,
            TriggerMode::UnlockDoor { .. } => TriggerType::UnlockDoor,
            TriggerMode::CounterAddedAll { .. } => TriggerType::CounterAddedAll,
            TriggerMode::CounterPlayerAddedAll { .. } => TriggerType::CounterPlayerAddedAll,
            TriggerMode::CounterTypeAddedAll { .. } => TriggerType::CounterTypeAddedAll,
            TriggerMode::CrewedSaddled { .. } => TriggerType::Crewed,
            TriggerMode::DamageDoneOnceByController { .. } => {
                TriggerType::DamageDoneOnceByController
            }
            TriggerMode::ExcessDamageAll { .. } => TriggerType::ExcessDamageAll,
            TriggerMode::PhaseOutAll { .. } => TriggerType::PhaseOutAll,
            TriggerMode::Vote => TriggerType::Vote,
            TriggerMode::GiveGift { .. } => TriggerType::GiveGift,
            TriggerMode::VisitAttraction { .. } => TriggerType::VisitAttraction,
            TriggerMode::EnteredRoom { .. } => TriggerType::EnteredRoom,
            TriggerMode::PayCumulativeUpkeep { .. } => TriggerType::PayCumulativeUpkeep,
            TriggerMode::DamageDealtOnce { .. } => TriggerType::DamageDealtOnce,
            TriggerMode::Destroyed { .. } => TriggerType::Destroyed,
            TriggerMode::Exiled { .. } => TriggerType::Exiled,
            TriggerMode::TokenCreated { .. } => TriggerType::TokenCreated,
            TriggerMode::SpellCopied { .. } => TriggerType::SpellCopied,
            TriggerMode::SpellCopy { .. } => TriggerType::SpellCopy,
            TriggerMode::SpellAbilityCopy { .. } => TriggerType::SpellAbilityCopy,
            TriggerMode::SpellCastOrCopy { .. } => TriggerType::SpellCastOrCopy,
            TriggerMode::AttackersDeclared {
                one_target: true, ..
            } => TriggerType::AttackersDeclaredOneTarget,
            TriggerMode::AttackersDeclared {
                one_target: false, ..
            } => TriggerType::AttackersDeclared,
            TriggerMode::BlockersDeclared => TriggerType::BlockersDeclared,
            TriggerMode::ChangesController { .. } => TriggerType::ChangesController,
            TriggerMode::TurnBegin { .. } => TriggerType::TurnBegin,
            TriggerMode::Cycled { .. } => TriggerType::Cycled,
            TriggerMode::PhasedIn { .. } => TriggerType::PhasedIn,
            TriggerMode::PhasedOut { .. } => TriggerType::PhasedOut,
            TriggerMode::Always => TriggerType::Always,
            TriggerMode::Immediate => TriggerType::Immediate,
            TriggerMode::Surveil { .. } => TriggerType::Surveil,
            TriggerMode::Scry { .. } => TriggerType::Scry,
            TriggerMode::Foretell { .. } => TriggerType::Foretell,
            TriggerMode::SearchedLibrary { .. } => TriggerType::SearchedLibrary,
            TriggerMode::Shuffled { .. } => TriggerType::Shuffled,
            TriggerMode::ManaAdded { .. } => TriggerType::ManaAdded,
            TriggerMode::DamageDoneOnce { .. } => TriggerType::DamageDoneOnce,
            TriggerMode::DamageAll { .. } => TriggerType::DamageAll,
            TriggerMode::ExcessDamage { .. } => TriggerType::ExcessDamage,
            TriggerMode::DamagePreventedOnce { .. } => TriggerType::DamagePreventedOnce,
            TriggerMode::SpellCastAll { .. } => TriggerType::SpellCastAll,
            TriggerMode::SpellCastOnce { .. } => TriggerType::SpellCastOnce,
            TriggerMode::SpellCastOfType { .. } => TriggerType::SpellCastOfType,
            TriggerMode::LifeLostAll { .. } => TriggerType::LifeLostAll,
            TriggerMode::CounterAddedOnce { .. } => TriggerType::CounterAddedOnce,
            TriggerMode::CounterRemovedOnce { .. } => TriggerType::CounterRemovedOnce,
            TriggerMode::Exerted { .. } => TriggerType::Exerted,
            TriggerMode::CollectEvidence { .. } => TriggerType::CollectEvidence,
            TriggerMode::Forage { .. } => TriggerType::Forage,
            TriggerMode::Enlisted { .. } => TriggerType::Enlisted,
            TriggerMode::FlippedCoin { .. } => TriggerType::FlippedCoin,
            TriggerMode::RolledDie { .. } => TriggerType::RolledDie,
            TriggerMode::RolledDieOnce { .. } => TriggerType::RolledDieOnce,
            TriggerMode::DiscardedAll { .. } => TriggerType::DiscardedAll,
            TriggerMode::SacrificedOnce { .. } => TriggerType::SacrificedOnce,
            TriggerMode::ChangesZoneAll { .. } => TriggerType::ChangesZoneAll,
            TriggerMode::TapAll { .. } => TriggerType::TapAll,
            TriggerMode::UntapAll { .. } => TriggerType::UntapAll,
            TriggerMode::BecomesTargetOnce { .. } => TriggerType::BecomesTargetOnce,
            TriggerMode::TokenCreatedOnce { .. } => TriggerType::TokenCreatedOnce,
            TriggerMode::AttackerBlockedOnce { .. } => TriggerType::AttackerBlockedOnce,
            TriggerMode::AttackerBlockedByCreature { .. } => TriggerType::AttackerBlockedByCreature,
            TriggerMode::AttackerUnblockedOnce { .. } => TriggerType::AttackerUnblockedOnce,
            TriggerMode::ManaExpend { .. } => TriggerType::ManaExpend,
            TriggerMode::Exploited { .. } => TriggerType::Exploited,
            TriggerMode::Mutates { .. } => TriggerType::Mutates,
            TriggerMode::SetInMotion { .. } => TriggerType::SetInMotion,
            TriggerMode::CaseSolved { .. } => TriggerType::CaseSolved,
            TriggerMode::ClaimPrize { .. } => TriggerType::ClaimPrize,
            TriggerMode::TakesInitiative { .. } => TriggerType::TakeInitiative,
            TriggerMode::PlaneswalkedFrom { .. } => TriggerType::Planeswalk,
            TriggerMode::PlaneswalkedTo { .. } => TriggerType::Planeswalk,
            TriggerMode::CrankContraption { .. } => TriggerType::CrankAdvanced,
            TriggerMode::ChaosEnsues { .. } => TriggerType::ChaosEnsues,
            TriggerMode::BecomesSaddled { .. } => TriggerType::BecomesSaddled,
        }
    }

    pub fn set_triggering_objects(
        &self,
        sa: &mut SpellAbility,
        params: &RunParams,
        game: &GameState,
        host_card: CardId,
        host_controller: PlayerId,
    ) {
        match self {
            TriggerMode::ChangesZone { .. } => {
                super::trigger_changes_zone::set_triggering_objects(sa, params)
            }
            TriggerMode::AttackersDeclared { .. } => {
                super::trigger_attackers_declared::set_triggering_objects(sa, params)
            }
            TriggerMode::Explored { .. } => {
                super::trigger_explores::set_triggering_objects(sa, params)
            }
            TriggerMode::Destroyed { .. } => {
                super::trigger_destroyed::set_triggering_objects(sa, params)
            }
            TriggerMode::Vote => {
                super::trigger_vote::set_triggering_objects(
                    sa,
                    params,
                    host_card,
                    host_controller,
                    game,
                );
            }
            _ => {}
        }
    }
}

pub(crate) fn matches_valid_sa(filter: &str, sa: &crate::spellability::SpellAbility) -> bool {
    let f = filter.trim();
    if f.is_empty() {
        return true;
    }
    if f.eq_ignore_ascii_case("Spell") {
        return sa.is_spell;
    }
    if f.eq_ignore_ascii_case("Ability") {
        return !sa.is_spell;
    }
    true
}

/// Matches a card against a ValidCard$ filter string.
/// Handles: Card.Self, Creature.Other, Creature.YouCtrl, Creature,
/// Instant,Sorcery (comma = OR), type filters.
pub(crate) fn matches_valid_card(
    filter: &str,
    card_id: CardId,
    host_card: CardId,
    _host_controller: PlayerId,
    game: &GameState,
) -> bool {
    let card = game.card(card_id);
    let host = game.card(host_card);
    valid_filter::matches_valid_card(filter, card, host)
}

/// Matches a player against a ValidPlayer$ filter string.
pub(crate) fn matches_valid_player(
    filter: &str,
    player: PlayerId,
    host_controller: PlayerId,
) -> bool {
    valid_filter::matches_valid_player(filter, player, host_controller)
}

/// Check if a count matches a ValidAttackersAmount filter like "GE1", "EQ3", etc.
pub(crate) fn matches_amount(filter: &str, count: usize) -> bool {
    let (op, num_str) = if filter.len() >= 3 {
        (&filter[..2], &filter[2..])
    } else {
        return count > 0; // fallback
    };
    let n: usize = num_str.parse().unwrap_or(0);
    match op {
        "GE" => count >= n,
        "GT" => count > n,
        "LE" => count <= n,
        "LT" => count < n,
        "EQ" => count == n,
        "NE" => count != n,
        _ => count > 0,
    }
}

/// Parse a zone name to ZoneType.
pub(crate) fn parse_zone(s: &str) -> Option<ZoneType> {
    match s {
        "Battlefield" => Some(ZoneType::Battlefield),
        "Graveyard" => Some(ZoneType::Graveyard),
        "Hand" => Some(ZoneType::Hand),
        "Library" => Some(ZoneType::Library),
        "Exile" => Some(ZoneType::Exile),
        "Stack" => Some(ZoneType::Stack),
        "Command" => Some(ZoneType::Command),
        "Any" => None, // None means "any zone"
        _ => None,
    }
}

/// Parse a phase name to PhaseType.
pub(crate) fn parse_phase(s: &str) -> Option<PhaseType> {
    match s {
        "Untap" => Some(PhaseType::Untap),
        "Upkeep" => Some(PhaseType::Upkeep),
        "Draw" => Some(PhaseType::Draw),
        "Main1" => Some(PhaseType::Main1),
        "Main2" => Some(PhaseType::Main2),
        "CombatBegin" | "BeginCombat" => Some(PhaseType::CombatBegin),
        "CombatEnd" | "EndCombat" | "EndOfCombat" => Some(PhaseType::CombatEnd),
        "EndOfTurn" | "End" => Some(PhaseType::EndOfTurn),
        "Cleanup" => Some(PhaseType::Cleanup),
        _ => None,
    }
}

/// Mirrors Java's TriggerHandler.parseTrigger().
/// Parses raw "Mode$ ChangesZone | Origin$ Any | ..." into Trigger struct.
pub fn parse_trigger(raw: &str, next_id: &mut u32) -> Option<Trigger> {
    let params = Params::from_raw(raw);

    let mode_str = params.get(keys::MODE)?;
    let mode = match mode_str {
        "ChangesZone" => crate::trigger::trigger_changes_zone::parse_mode(&params),
        "Phase" => crate::trigger::trigger_phase::parse_mode(&params),
        "SpellCast" | "AbilityCast" | "SpellAbilityCast" => {
            crate::trigger::trigger_spell_ability_cast_or_copy::parse_mode(mode_str, &params)
        }
        "Attacks" => crate::trigger::trigger_attacks::parse_mode(&params),
        "Fight" => {
            let valid_card = params.get_cloned(keys::VALID_CARD);
            TriggerMode::Fight { valid_card }
        }
        "FightOnce" => {
            let valid_card = params.get_cloned(keys::VALID_CARD);
            TriggerMode::FightOnce { valid_card }
        }
        "DamageDone" => {
            let valid_source = params.get_cloned(keys::VALID_SOURCE);
            let valid_target = params.get_cloned(keys::VALID_TARGET);
            let combat_damage_only = params.is_true(keys::COMBAT_DAMAGE);
            TriggerMode::DamageDone {
                valid_source,
                valid_target,
                combat_damage_only,
            }
        }
        "Countered" => {
            let valid_card = params.get_cloned(keys::VALID_CARD);
            let valid_cause = params.get_cloned(keys::VALID_CAUSE);
            let valid_sa = params.get_cloned(keys::VALID_SA);
            TriggerMode::Countered {
                valid_card,
                valid_cause,
                valid_sa,
            }
        }
        // ── New trigger modes (issue #19) ──
        "Blocks" => crate::trigger::trigger_blocks::parse_mode(&params),
        "AttackerBlocked" => {
            let valid_card = params.get_cloned(keys::VALID_CARD);
            TriggerMode::AttackerBlocked { valid_card }
        }
        "AttackerUnblocked" => {
            let valid_card = params.get_cloned(keys::VALID_CARD);
            TriggerMode::AttackerUnblocked { valid_card }
        }
        "LifeGained" => crate::trigger::trigger_life_gained::parse_mode(&params),
        "LifeLost" => crate::trigger::trigger_life_lost::parse_mode(&params),
        "CounterAdded" => crate::trigger::trigger_counter_added::parse_mode(&params),
        "CounterRemoved" => crate::trigger::trigger_counter_removed::parse_mode(&params),
        "Sacrificed" => crate::trigger::trigger_sacrificed::parse_mode(&params),
        "Drawn" => crate::trigger::trigger_drawn::parse_mode(&params),
        "Milled" => {
            let valid_card = params.get_cloned(keys::VALID_CARD);
            let valid_player = params.get_cloned(keys::VALID_PLAYER);
            TriggerMode::Milled {
                valid_card,
                valid_player,
            }
        }
        "Taps" => crate::trigger::trigger_taps::parse_mode(&params),
        "Untaps" => crate::trigger::trigger_untaps::parse_mode(&params),
        "Transformed" => crate::trigger::trigger_transformed::parse_mode(&params),
        "TurnFaceUp" => crate::trigger::trigger_turn_face_up::parse_mode(&params),
        "Attached" => {
            let valid_card = params.get_cloned(keys::VALID_CARD);
            TriggerMode::Attached { valid_card }
        }
        "Unattached" => {
            let valid_card = params.get_cloned(keys::VALID_CARD);
            TriggerMode::Unattached { valid_card }
        }
        "LandPlayed" => crate::trigger::trigger_land_played::parse_mode(&params),
        "BecomesTarget" => crate::trigger::trigger_becomes_target::parse_mode(&params),
        "TapsForMana" => crate::trigger::trigger_taps_for_mana::parse_mode(&params),
        "AbilityActivated" => crate::trigger::trigger_ability_activated::parse_mode(&params),
        "Explored" | "Explores" => crate::trigger::trigger_explores::parse_mode(&params),
        "BecomeMonstrous" => {
            let valid_card = params.get_cloned(keys::VALID_CARD);
            TriggerMode::BecomeMonstrous { valid_card }
        }
        "BecomeMonarch" => crate::trigger::trigger_become_monarch::parse_mode(&params),
        "DamageDealtOnce" => crate::trigger::trigger_damage_dealt_once::parse_mode(&params),
        "Destroyed" => crate::trigger::trigger_destroyed::parse_mode(&params),
        "Exiled" => crate::trigger::trigger_exiled::parse_mode(&params),
        "CollectEvidence" => {
            let valid_player = params.get_cloned(keys::VALID_PLAYER);
            TriggerMode::CollectEvidence { valid_player }
        }
        "Forage" => {
            let valid_player = params.get_cloned(keys::VALID_PLAYER);
            TriggerMode::Forage { valid_player }
        }
        "Enlisted" => {
            let valid_card = params.get_cloned(keys::VALID_CARD);
            let valid_enlisted = params.get_cloned(keys::VALID_ENLISTED);
            TriggerMode::Enlisted {
                valid_card,
                valid_enlisted,
            }
        }
        "FlippedCoin" => {
            let valid_player = params.get_cloned(keys::VALID_PLAYER);
            let valid_result = params.get_cloned(keys::VALID_RESULT);
            TriggerMode::FlippedCoin {
                valid_player,
                valid_result,
            }
        }
        "RolledDie" => {
            let valid_player = params.get_cloned(keys::VALID_PLAYER);
            let valid_result = params.get_cloned(keys::VALID_RESULT);
            let valid_sides = params.get_cloned(keys::VALID_SIDES);
            let number = params.get("Number").and_then(|n| n.parse::<i32>().ok());
            let natural = params.is_true("Natural");
            let rolled_to_visit_attractions = params.has("RolledToVisitAttractions");
            TriggerMode::RolledDie {
                valid_player,
                valid_result,
                valid_sides,
                number,
                natural,
                rolled_to_visit_attractions,
            }
        }
        "RolledDieOnce" => {
            let valid_player = params.get_cloned(keys::VALID_PLAYER);
            let valid_result = params.get_cloned(keys::VALID_RESULT);
            let valid_sides = params.get_cloned(keys::VALID_SIDES);
            let rolled_to_visit_attractions = params.has("RolledToVisitAttractions");
            TriggerMode::RolledDieOnce {
                valid_player,
                valid_result,
                valid_sides,
                rolled_to_visit_attractions,
            }
        }
        "TokenCreated" => crate::trigger::trigger_token_created::parse_mode(&params),
        "SpellCastOrCopy" | "SpellCopied" | "SpellAbilityCopy" | "SpellCopy" => {
            crate::trigger::trigger_spell_ability_cast_or_copy::parse_mode(mode_str, &params)
        }
        // ── New trigger modes (issue #54) ──
        "AttackersDeclared" | "AttackersDeclaredOneTarget" => {
            let valid_player = params
                .get(keys::ATTACKING_PLAYER)
                .or_else(|| params.get(keys::VALID_PLAYER))
                .map(|s| s.to_string());
            let valid_attackers = params.get_cloned(keys::VALID_ATTACKERS);
            let valid_attackers_amount = params.get_cloned(keys::VALID_ATTACKERS_AMOUNT);
            let attacked_target = params.get_cloned("AttackedTarget");
            let one_target = mode_str == "AttackersDeclaredOneTarget";
            TriggerMode::AttackersDeclared {
                valid_player,
                valid_attackers,
                valid_attackers_amount,
                attacked_target,
                one_target,
            }
        }
        "BlockersDeclared" => crate::trigger::trigger_blockers_declared::parse_mode(&params),
        "ChangesZoneAll" => crate::trigger::trigger_changes_zone_all::parse_mode(&params),
        "ChangesController" => {
            let valid_card = params.get_cloned(keys::VALID_CARD);
            TriggerMode::ChangesController { valid_card }
        }
        "TurnBegin" | "NewTurn" => {
            let valid_player = params.get_cloned(keys::VALID_PLAYER);
            TriggerMode::TurnBegin { valid_player }
        }
        "DamageDoneOnce" => {
            let valid_source = params.get_cloned(keys::VALID_SOURCE);
            let valid_target = params.get_cloned(keys::VALID_TARGET);
            let combat_damage_only = params.is_true(keys::COMBAT_DAMAGE);
            TriggerMode::DamageDoneOnce {
                valid_source,
                valid_target,
                combat_damage_only,
            }
        }
        "DamageDoneOnceByController" => {
            let valid_source = params.get_cloned(keys::VALID_SOURCE);
            let valid_target = params.get_cloned(keys::VALID_TARGET);
            let combat_damage_only = params.is_true(keys::COMBAT_DAMAGE);
            TriggerMode::DamageDoneOnceByController {
                valid_source,
                valid_target,
                combat_damage_only,
            }
        }
        "SpellCastAll" => {
            crate::trigger::trigger_spell_ability_cast_or_copy::parse_mode(mode_str, &params)
        }
        "LifeLostAll" => {
            let valid_player = params.get_cloned(keys::VALID_PLAYER);
            TriggerMode::LifeLostAll { valid_player }
        }
        "CounterAddedOnce" => {
            let valid_card = params.get_cloned(keys::VALID_CARD);
            let counter_type = params.get_cloned(keys::COUNTER_TYPE);
            let valid_source = params.get_cloned(keys::VALID_SOURCE);
            TriggerMode::CounterAddedOnce {
                valid_card,
                counter_type,
                valid_source,
            }
        }
        "CounterAddedAll" => {
            let counter_type = params.get_cloned(keys::COUNTER_TYPE);
            let valid = params
                .get(keys::VALID)
                .or_else(|| params.get(keys::VALID_CARD))
                .map(|s| s.to_string());
            TriggerMode::CounterAddedAll {
                counter_type,
                valid,
            }
        }
        "CounterPlayerAddedAll" => {
            let valid_source = params.get_cloned("ValidSource");
            let valid_object = params.get_cloned("ValidObject");
            let valid_object_to_source = params.get_cloned("ValidObjectToSource");
            TriggerMode::CounterPlayerAddedAll {
                valid_source,
                valid_object,
                valid_object_to_source,
            }
        }
        "CounterTypeAddedAll" => {
            let valid_object = params.get_cloned("ValidObject");
            let first_time_only = params.has("FirstTime");
            TriggerMode::CounterTypeAddedAll {
                valid_object,
                first_time_only,
            }
        }
        "DiscardedAll" => {
            let valid_card = params.get_cloned(keys::VALID_CARD);
            let valid_player = params.get_cloned(keys::VALID_PLAYER);
            TriggerMode::DiscardedAll {
                valid_card,
                valid_player,
            }
        }
        "Discarded" => {
            let valid_card = params.get_cloned(keys::VALID_CARD);
            let valid_player = params.get_cloned(keys::VALID_PLAYER);
            let valid_cause = params.get_cloned(keys::VALID_CAUSE);
            TriggerMode::Discarded {
                valid_card,
                valid_player,
                valid_cause,
            }
        }
        "SacrificedOnce" => {
            let valid_card = params.get_cloned(keys::VALID_CARD);
            let valid_player = params.get_cloned(keys::VALID_PLAYER);
            TriggerMode::SacrificedOnce {
                valid_card,
                valid_player,
            }
        }
        "Cycled" | "Cycling" => {
            let valid_card = params.get_cloned(keys::VALID_CARD);
            let valid_player = params.get_cloned(keys::VALID_PLAYER);
            TriggerMode::Cycled {
                valid_card,
                valid_player,
            }
        }
        "PhasedIn" | "PhaseIn" => {
            let valid_card = params.get_cloned(keys::VALID_CARD);
            TriggerMode::PhasedIn { valid_card }
        }
        "PhasedOut" | "PhaseOut" => {
            let valid_card = params.get_cloned(keys::VALID_CARD);
            TriggerMode::PhasedOut { valid_card }
        }
        "Always" => TriggerMode::Always,
        "Immediate" => TriggerMode::Immediate,
        "Surveil" => {
            let valid_player = params.get_cloned(keys::VALID_PLAYER);
            TriggerMode::Surveil { valid_player }
        }
        "Scry" => {
            let valid_player = params.get_cloned(keys::VALID_PLAYER);
            TriggerMode::Scry { valid_player }
        }
        "Foretell" | "Foretold" => {
            let valid_card = params.get_cloned(keys::VALID_CARD);
            TriggerMode::Foretell { valid_card }
        }
        "SearchedLibrary" => {
            let valid_player = params.get_cloned(keys::VALID_PLAYER);
            TriggerMode::SearchedLibrary { valid_player }
        }
        "Shuffled" => {
            let valid_player = params.get_cloned(keys::VALID_PLAYER);
            TriggerMode::Shuffled { valid_player }
        }
        "ManaAdded" => {
            let valid_source = params.get_cloned("ValidSource");
            let valid_sa = params.get_cloned(keys::VALID_SA);
            let player = params.get_cloned(keys::PLAYER);
            let produced = params.get_cloned(keys::PRODUCED);
            TriggerMode::ManaAdded {
                valid_source,
                valid_sa,
                player,
                produced,
            }
        }
        "TokenCreatedOnce" => {
            let valid_card = params
                .get("ValidToken")
                .or_else(|| params.get(keys::VALID_CARD))
                .map(|s| s.to_string());
            let only_first = params.get_cloned("OnlyFirst");
            TriggerMode::TokenCreatedOnce {
                valid_card,
                only_first,
            }
        }
        "TapAll" => {
            let valid_card = params.get_cloned(keys::VALID_CARD);
            TriggerMode::TapAll { valid_card }
        }
        "UntapAll" => {
            let valid_card = params.get_cloned(keys::VALID_CARD);
            TriggerMode::UntapAll { valid_card }
        }
        "BecomesTargetOnce" => {
            let valid_card = params.get_cloned(keys::VALID_CARD);
            TriggerMode::BecomesTargetOnce { valid_card }
        }
        "AttackerBlockedByCreature" => {
            let valid_card = params.get_cloned(keys::VALID_CARD);
            let valid_blocked = params.get_cloned(keys::VALID_BLOCKED);
            TriggerMode::AttackerBlockedByCreature {
                valid_card,
                valid_blocked,
            }
        }
        "AttackerBlockedOnce" => {
            let valid_card = params.get_cloned(keys::VALID_CARD);
            TriggerMode::AttackerBlockedOnce { valid_card }
        }
        "AttackerUnblockedOnce" => {
            let valid_card = params.get_cloned(keys::VALID_CARD);
            TriggerMode::AttackerUnblockedOnce { valid_card }
        }
        "SpellCastOnce" | "SpellCastOfType" => {
            crate::trigger::trigger_spell_ability_cast_or_copy::parse_mode(mode_str, &params)
        }
        "DamageAll" => {
            let valid_source = params.get_cloned(keys::VALID_SOURCE);
            let valid_target = params.get_cloned(keys::VALID_TARGET);
            TriggerMode::DamageAll {
                valid_source,
                valid_target,
            }
        }
        "DamagePreventedOnce" => {
            let valid_card = params.get_cloned(keys::VALID_CARD);
            TriggerMode::DamagePreventedOnce { valid_card }
        }
        "ExcessDamage" => {
            let valid_source = params.get_cloned(keys::VALID_SOURCE);
            let valid_target = params.get_cloned(keys::VALID_TARGET);
            TriggerMode::ExcessDamage {
                valid_source,
                valid_target,
            }
        }
        "ExcessDamageAll" => {
            let valid_target = params.get_cloned(keys::VALID_TARGET);
            let combat_damage_only = params.is_true(keys::COMBAT_DAMAGE);
            TriggerMode::ExcessDamageAll {
                valid_target,
                combat_damage_only,
            }
        }
        "CounterRemovedOnce" => {
            let valid_card = params.get_cloned(keys::VALID_CARD);
            let counter_type = params.get_cloned(keys::COUNTER_TYPE);
            TriggerMode::CounterRemovedOnce {
                valid_card,
                counter_type,
            }
        }
        "Exerted" => {
            let valid_card = params.get_cloned(keys::VALID_CARD);
            TriggerMode::Exerted { valid_card }
        }
        "ManaExpend" => {
            let valid_player = params.get_cloned(keys::PLAYER);
            let amount = params.as_i32(keys::AMOUNT).unwrap_or(1);
            TriggerMode::ManaExpend {
                valid_player,
                amount,
            }
        }
        "Mutates" => {
            let valid_card = params.get_cloned(keys::VALID_CARD);
            TriggerMode::Mutates { valid_card }
        }
        "SetInMotion" => {
            let valid_card = params.get_cloned(keys::VALID_CARD);
            TriggerMode::SetInMotion { valid_card }
        }
        "CaseSolved" => {
            let valid_card = params.get_cloned(keys::VALID_CARD);
            let valid_player = params.get_cloned(keys::VALID_PLAYER);
            TriggerMode::CaseSolved {
                valid_card,
                valid_player,
            }
        }
        "ClaimPrize" => {
            let valid_player = params.get_cloned(keys::VALID_PLAYER);
            let valid_card = params.get_cloned(keys::VALID_CARD);
            TriggerMode::ClaimPrize {
                valid_player,
                valid_card,
            }
        }
        "TakesInitiative" | "TakeInitiative" => {
            let valid_player = params.get_cloned(keys::VALID_PLAYER);
            TriggerMode::TakesInitiative { valid_player }
        }
        "Adapt" => {
            let valid_card = params.get_cloned(keys::VALID_CARD);
            TriggerMode::Adapt { valid_card }
        }
        "BecomeRenowned" => {
            let valid_card = params.get_cloned(keys::VALID_CARD);
            TriggerMode::BecomeRenowned { valid_card }
        }
        "Evolved" => {
            let valid_card = params.get_cloned(keys::VALID_CARD);
            TriggerMode::Evolved { valid_card }
        }
        "BecomesPlotted" => {
            let valid_card = params.get_cloned(keys::VALID_CARD);
            TriggerMode::BecomesPlotted { valid_card }
        }
        "Investigated" => {
            let valid_player = params.get_cloned(keys::VALID_PLAYER);
            let first_time_only = params.has("FirstTime");
            TriggerMode::Investigated {
                valid_player,
                first_time_only,
            }
        }
        "Proliferate" => {
            let valid_player = params.get_cloned(keys::VALID_PLAYER);
            TriggerMode::Proliferate { valid_player }
        }
        "CompletedDungeon" | "DungeonCompleted" => {
            let valid_player = params.get_cloned(keys::VALID_PLAYER);
            TriggerMode::CompletedDungeon { valid_player }
        }
        "CommitCrime" => {
            let valid_player = params.get_cloned(keys::VALID_PLAYER);
            TriggerMode::CommitCrime { valid_player }
        }
        "GiveGift" => {
            let valid_player = params.get_cloned(keys::VALID_PLAYER);
            TriggerMode::GiveGift { valid_player }
        }
        "RingTemptsYou" => {
            let valid_player = params.get_cloned(keys::VALID_PLAYER);
            let valid_card = params.get_cloned(keys::VALID_CARD);
            TriggerMode::RingTemptsYou {
                valid_player,
                valid_card,
            }
        }
        "PayLife" => {
            let valid_player = params.get_cloned(keys::VALID_PLAYER);
            TriggerMode::PayLife { valid_player }
        }
        "PayEcho" => {
            let valid_card = params.get_cloned(keys::VALID_CARD);
            let paid = params.get("Paid").map(|v| v.eq_ignore_ascii_case("true"));
            TriggerMode::PayEcho { valid_card, paid }
        }
        "ClassLevelGained" => {
            let valid_card = params.get_cloned(keys::VALID_CARD);
            let class_level = params.as_i32("ClassLevel");
            TriggerMode::ClassLevelGained {
                valid_card,
                class_level,
            }
        }
        "NewGame" => crate::trigger::trigger_new_game::parse_mode(&params),
        "DayTimeChanges" => TriggerMode::DayTimeChanges,
        "LosesGame" => {
            let valid_player = params.get_cloned(keys::VALID_PLAYER);
            TriggerMode::LosesGame { valid_player }
        }
        "Discover" => {
            let valid_player = params.get_cloned(keys::VALID_PLAYER);
            TriggerMode::Discover { valid_player }
        }
        "Elementalbend" | "ElementalBend" | "Airbend" | "Earthbend" | "Firebend" | "Waterbend" => {
            let valid_player = params.get_cloned(keys::VALID_PLAYER);
            TriggerMode::Elementalbend { valid_player }
        }
        "PlanarDice" => {
            let valid_player = params.get_cloned(keys::VALID_PLAYER);
            let result = params.get_cloned("Result");
            TriggerMode::PlanarDice {
                valid_player,
                result,
            }
        }
        "PhaseOutAll" => {
            let valid_cards = params
                .get(keys::VALID_CARDS)
                .or_else(|| params.get(keys::VALID_CARD))
                .map(|s| s.to_string());
            TriggerMode::PhaseOutAll { valid_cards }
        }
        "VisitAttraction" => {
            let valid_player = params.get_cloned(keys::VALID_PLAYER);
            let valid_card = params.get_cloned(keys::VALID_CARD);
            TriggerMode::VisitAttraction {
                valid_player,
                valid_card,
            }
        }
        "EnteredRoom" | "RoomEntered" => {
            let valid_card = params.get_cloned(keys::VALID_CARD);
            let valid_room = params.get_cloned("ValidRoom");
            TriggerMode::EnteredRoom {
                valid_card,
                valid_room,
            }
        }
        "MilledAll" => {
            let valid_card = params.get_cloned(keys::VALID_CARD);
            TriggerMode::MilledAll { valid_card }
        }
        "MilledOnce" => {
            let valid_card = params.get_cloned(keys::VALID_CARD);
            let valid_player = params.get_cloned(keys::VALID_PLAYER);
            TriggerMode::MilledOnce {
                valid_card,
                valid_player,
            }
        }
        "Abandoned" => crate::trigger::trigger_abandoned::parse_mode(&params),
        "ManifestDread" => {
            let valid_player = params.get_cloned(keys::VALID_PLAYER);
            TriggerMode::ManifestDread { valid_player }
        }
        "Specializes" => {
            let valid_card = params.get_cloned(keys::VALID_CARD);
            TriggerMode::Specializes { valid_card }
        }
        "Trains" => {
            let valid_card = params.get_cloned(keys::VALID_CARD);
            TriggerMode::Trains { valid_card }
        }
        "Devoured" => {
            let valid_card = params
                .get("ValidDevoured")
                .or_else(|| params.get(keys::VALID_CARD))
                .map(|s| s.to_string());
            TriggerMode::Devoured { valid_card }
        }
        "ConjureAll" => {
            let valid_player = params.get_cloned(keys::VALID_PLAYER);
            let valid_card = params.get_cloned(keys::VALID_CARD);
            TriggerMode::ConjureAll {
                valid_player,
                valid_card,
            }
        }
        "SeekAll" => {
            let valid_player = params.get_cloned(keys::VALID_PLAYER);
            TriggerMode::SeekAll { valid_player }
        }
        "BecomesCrewed" => {
            let valid_card = params.get_cloned(keys::VALID_CARD);
            let valid_crew = params.get_cloned("ValidCrew");
            let first_time_crewed = params.has("FirstTimeCrewed");
            let valid_crew_amount = params.get_cloned("ValidCrewAmount");
            TriggerMode::BecomesCrewed {
                valid_card,
                valid_crew,
                first_time_crewed,
                valid_crew_amount,
            }
        }
        "Championed" => {
            let valid_card = params
                .get("ValidCard")
                .or_else(|| params.get("ValidChampioned"))
                .map(|s| s.to_string());
            let valid_source = params.get("ValidSource").map(|s| s.to_string());
            TriggerMode::Championed {
                valid_card,
                valid_source,
            }
        }
        "Clashed" => {
            let valid_player = params.get_cloned(keys::VALID_PLAYER);
            let won = params.get("Won").map(|v| v.eq_ignore_ascii_case("True"));
            TriggerMode::Clashed { valid_player, won }
        }
        "Mentored" => {
            let valid_card = params.get_cloned(keys::VALID_CARD);
            let valid_source = params.get_cloned("ValidSource");
            TriggerMode::Mentored {
                valid_card,
                valid_source,
            }
        }
        "FullyUnlock" => crate::trigger::trigger_fully_unlock::parse_mode(&params),
        "AbilityResolves" => {
            let valid_spell_ability = params.get_cloned("ValidSpellAbility");
            let valid_source = params.get_cloned("ValidSource");
            TriggerMode::AbilityResolves {
                valid_spell_ability,
                valid_source,
            }
        }
        "AbilityTriggered" => {
            let valid_mode = params.get_cloned("ValidMode");
            let valid_destination = params.get_cloned("ValidDestination");
            let valid_spell_ability = params.get_cloned("ValidSpellAbility");
            let valid_source = params.get_cloned("ValidSource");
            let valid_cause = params.get_cloned("ValidCause");
            let triggered_own_ability = params.has("TriggeredOwnAbility");
            TriggerMode::AbilityTriggered {
                valid_mode,
                valid_destination,
                valid_spell_ability,
                valid_source,
                valid_cause,
                triggered_own_ability,
            }
        }
        "UnlockDoor" => crate::trigger::trigger_unlock_door::parse_mode(&params),
        "Vote" => crate::trigger::trigger_vote::parse_mode(&params),
        "PlaneswalkedFrom" => {
            let valid_card = params
                .get(keys::VALID_CARDS)
                .or_else(|| params.get(keys::VALID_CARD))
                .map(|s| s.to_string());
            TriggerMode::PlaneswalkedFrom { valid_card }
        }
        "PlaneswalkedTo" => {
            let valid_card = params
                .get(keys::VALID_CARDS)
                .or_else(|| params.get(keys::VALID_CARD))
                .map(|s| s.to_string());
            TriggerMode::PlaneswalkedTo { valid_card }
        }
        "Planeswalk" => {
            let valid_card = params
                .get(keys::VALID_CARDS)
                .or_else(|| params.get(keys::VALID_CARD))
                .map(|s| s.to_string());
            TriggerMode::PlaneswalkedTo { valid_card }
        }
        "CrankContraption" | "CrankAdvanced" => {
            let valid_card = params.get_cloned(keys::VALID_CARD);
            let valid_player = params.get_cloned(keys::VALID_PLAYER);
            TriggerMode::CrankContraption {
                valid_card,
                valid_player,
            }
        }
        "PayCumulativeUpkeep" => {
            let valid_card = params.get_cloned(keys::VALID_CARD);
            let paid = params.get("Paid").map(|v| v.eq_ignore_ascii_case("true"));
            TriggerMode::PayCumulativeUpkeep { valid_card, paid }
        }
        "ChaosEnsues" => {
            let valid_player = params.get_cloned(keys::VALID_PLAYER);
            TriggerMode::ChaosEnsues { valid_player }
        }
        "BecomesSaddled" => {
            let valid_saddled = params.get_cloned("ValidSaddled");
            let first_time_saddled = params.has("FirstTimeSaddled");
            TriggerMode::BecomesSaddled {
                valid_saddled,
                first_time_saddled,
            }
        }
        "Crewed" | "Saddled" | "Stationed" => {
            let valid_card = params.get_cloned(keys::VALID_CARD);
            let valid_crew = params
                .get("ValidCrew")
                .or_else(|| params.get("ValidSaddled"))
                .map(|s| s.to_string());
            TriggerMode::CrewedSaddled {
                valid_card,
                valid_crew,
            }
        }
        "Unattach" => {
            let valid_card = params.get_cloned(keys::VALID_CARD);
            TriggerMode::Unattached { valid_card }
        }
        "Exploited" => {
            let valid_card = params.get_cloned(keys::VALID_CARD);
            let valid_source = params.get_cloned(keys::VALID_SOURCE);
            TriggerMode::Exploited {
                valid_card,
                valid_source,
            }
        }
        _ => return None,
    };

    // Parse active zones (default: Battlefield)
    let active_zones = params
        .get(keys::TRIGGER_ZONES)
        .map(|s| {
            s.split(',')
                .filter_map(|z| parse_zone(z.trim()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| vec![ZoneType::Battlefield]);

    let execute = params.get_cloned(keys::EXECUTE).unwrap_or_default();
    let optional = params.has(keys::OPTIONAL_DECIDER);
    let description = params
        .get_cloned(keys::TRIGGER_DESCRIPTION)
        .unwrap_or_default();
    let static_trigger = params.has("Static");

    let id = *next_id;
    *next_id += 1;

    Some(Trigger {
        id,
        mode,
        params,
        active_zones,
        execute,
        optional,
        description,
        intrinsic: true,
        static_trigger,
        trigger_remembered: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::RunParams;
    use crate::ids::{CardId, PlayerId};
    use crate::spellability::SpellAbility;

    #[test]
    fn parse_pipe_params_basic() {
        let params = Params::from_raw("Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigDraw");
        assert_eq!(params.get("Mode"), Some("ChangesZone"));
        assert_eq!(params.get("Origin"), Some("Any"));
        assert_eq!(params.get("Destination"), Some("Battlefield"));
        assert_eq!(params.get("ValidCard"), Some("Card.Self"));
        assert_eq!(params.get("Execute"), Some("TrigDraw"));
    }

    #[test]
    fn parse_trigger_changes_zone() {
        let mut next_id = 0;
        let trigger = parse_trigger(
            "Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigDraw | TriggerDescription$ When CARDNAME enters the battlefield, draw two cards.",
            &mut next_id,
        ).unwrap();

        assert_eq!(trigger.id, 0);
        assert_eq!(trigger.execute, "TrigDraw");
        assert!(matches!(
            trigger.mode,
            TriggerMode::ChangesZone {
                origin: None,
                destination: Some(ZoneType::Battlefield),
                ..
            }
        ));
        assert_eq!(trigger.active_zones, vec![ZoneType::Battlefield]);
    }

    #[test]
    fn parse_trigger_spell_cast() {
        let mut next_id = 0;
        let trigger = parse_trigger(
            "Mode$ SpellCast | ValidCard$ Instant,Sorcery | ValidActivatingPlayer$ You | Execute$ TrigDmg | TriggerDescription$ Whenever you cast an instant or sorcery spell, deal 2 damage.",
            &mut next_id,
        ).unwrap();

        assert!(matches!(trigger.mode, TriggerMode::SpellCast { .. }));
        assert_eq!(trigger.execute, "TrigDmg");
    }

    #[test]
    fn parse_trigger_ability_cast_is_distinct_mode() {
        let mut next_id = 0;
        let trigger = parse_trigger(
            "Mode$ AbilityCast | ValidCard$ Card.Self | Execute$ TrigAbility",
            &mut next_id,
        )
        .unwrap();
        assert!(matches!(trigger.mode, TriggerMode::AbilityCast { .. }));
    }

    #[test]
    fn parse_trigger_spell_ability_cast_is_distinct_mode() {
        let mut next_id = 0;
        let trigger = parse_trigger(
            "Mode$ SpellAbilityCast | ValidCard$ Instant | Execute$ TrigSpellAbility",
            &mut next_id,
        )
        .unwrap();
        assert!(matches!(trigger.mode, TriggerMode::SpellAbilityCast { .. }));
    }

    #[test]
    fn parse_trigger_spell_copy_family_modes_are_distinct() {
        let mut next_id = 0;
        let t_spell_copy = parse_trigger(
            "Mode$ SpellCopy | ValidCard$ Instant | Execute$ TrigCopy",
            &mut next_id,
        )
        .unwrap();
        let t_sa_copy = parse_trigger(
            "Mode$ SpellAbilityCopy | ValidCard$ Instant | Execute$ TrigSACopy",
            &mut next_id,
        )
        .unwrap();
        let t_cast_or_copy = parse_trigger(
            "Mode$ SpellCastOrCopy | ValidCard$ Instant | Execute$ TrigBoth",
            &mut next_id,
        )
        .unwrap();
        assert!(matches!(t_spell_copy.mode, TriggerMode::SpellCopy { .. }));
        assert!(matches!(
            t_sa_copy.mode,
            TriggerMode::SpellAbilityCopy { .. }
        ));
        assert!(matches!(
            t_cast_or_copy.mode,
            TriggerMode::SpellCastOrCopy { .. }
        ));
    }

    #[test]
    fn parse_trigger_phase() {
        let mut next_id = 0;
        let trigger = parse_trigger(
            "Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | Execute$ TrigUpkeep | TriggerDescription$ At the beginning of your upkeep.",
            &mut next_id,
        ).unwrap();

        assert!(matches!(
            trigger.mode,
            TriggerMode::Phase {
                phase: Some(PhaseType::Upkeep),
                ..
            }
        ));
    }

    #[test]
    fn parse_trigger_other_creature_etb() {
        let mut next_id = 0;
        let trigger = parse_trigger(
            "Mode$ ChangesZone | Destination$ Battlefield | ValidCard$ Creature.Other | Execute$ TrigGain | TriggerDescription$ Whenever another creature enters the battlefield, gain 1 life.",
            &mut next_id,
        ).unwrap();

        assert!(matches!(
            trigger.mode,
            TriggerMode::ChangesZone {
                origin: None,
                destination: Some(ZoneType::Battlefield),
                ..
            }
        ));

        if let TriggerMode::ChangesZone { valid_card, .. } = &trigger.mode {
            assert_eq!(valid_card.as_deref(), Some("Creature.Other"));
        }
    }

    // ── Tests for new trigger types (issue #19) ──

    #[test]
    fn parse_trigger_attacks() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ Attacks | ValidCard$ Creature.Self | Execute$ TrigAtk | TriggerDescription$ When this creature attacks.",
            &mut id,
        ).unwrap();
        assert!(matches!(t.mode, TriggerMode::Attacks { .. }));
        if let TriggerMode::Attacks { valid_card, alone } = &t.mode {
            assert_eq!(valid_card.as_deref(), Some("Creature.Self"));
            assert!(!alone);
        }
    }

    #[test]
    fn parse_trigger_fight() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ Fight | ValidCard$ Creature.Self | Execute$ TrigFight",
            &mut id,
        )
        .unwrap();
        assert!(matches!(t.mode, TriggerMode::Fight { .. }));
        if let TriggerMode::Fight { valid_card } = &t.mode {
            assert_eq!(valid_card.as_deref(), Some("Creature.Self"));
        }
    }

    #[test]
    fn parse_trigger_fight_once() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ FightOnce | ValidCard$ Creature.YouCtrl | Execute$ TrigFight",
            &mut id,
        )
        .unwrap();
        assert!(matches!(t.mode, TriggerMode::FightOnce { .. }));
        if let TriggerMode::FightOnce { valid_card } = &t.mode {
            assert_eq!(valid_card.as_deref(), Some("Creature.YouCtrl"));
        }
    }

    #[test]
    fn parse_trigger_damage_done() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ DamageDone | ValidSource$ Creature.Self | CombatDamage$ True | Execute$ TrigDmg",
            &mut id,
        )
        .unwrap();
        if let TriggerMode::DamageDone {
            valid_source,
            combat_damage_only,
            ..
        } = &t.mode
        {
            assert_eq!(valid_source.as_deref(), Some("Creature.Self"));
            assert!(*combat_damage_only);
        } else {
            panic!("Expected DamageDone mode");
        }
    }

    #[test]
    fn damage_done_player_populates_target_trigger_objects() {
        let mut sa = SpellAbility::new_simple(None, PlayerId(0), "");
        add_common_trigger_objects(
            &mut sa,
            &RunParams {
                damage_source: Some(CardId(1)),
                damage_target_player: Some(PlayerId(1)),
                ..Default::default()
            },
        );

        assert_eq!(sa.trigger_objects.get("Target").map(String::as_str), Some("1"));
        assert_eq!(
            sa.trigger_objects.get("TargetPlayer").map(String::as_str),
            Some("1")
        );
    }

    #[test]
    fn damage_done_card_populates_target_trigger_objects() {
        let mut sa = SpellAbility::new_simple(None, PlayerId(0), "");
        add_common_trigger_objects(
            &mut sa,
            &RunParams {
                damage_source: Some(CardId(1)),
                damage_target_card: Some(CardId(7)),
                ..Default::default()
            },
        );

        assert_eq!(sa.trigger_objects.get("Target").map(String::as_str), Some("7"));
        assert_eq!(
            sa.trigger_objects.get("TargetCard").map(String::as_str),
            Some("7")
        );
    }

    #[test]
    fn parse_trigger_blocks() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ Blocks | ValidCard$ Creature.Self | ValidBlocked$ Creature | Execute$ TrigBlock",
            &mut id,
        )
        .unwrap();
        if let TriggerMode::Blocks {
            valid_card,
            valid_blocked,
        } = &t.mode
        {
            assert_eq!(valid_card.as_deref(), Some("Creature.Self"));
            assert_eq!(valid_blocked.as_deref(), Some("Creature"));
        } else {
            panic!("Expected Blocks mode");
        }
    }

    #[test]
    fn parse_trigger_attacker_blocked() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ AttackerBlocked | ValidCard$ Creature.Self | Execute$ TrigBlocked",
            &mut id,
        )
        .unwrap();
        assert!(matches!(t.mode, TriggerMode::AttackerBlocked { .. }));
    }

    #[test]
    fn parse_trigger_attacker_unblocked() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ AttackerUnblocked | ValidCard$ Creature.Self | Execute$ TrigUnblocked",
            &mut id,
        )
        .unwrap();
        assert!(matches!(t.mode, TriggerMode::AttackerUnblocked { .. }));
    }

    #[test]
    fn parse_trigger_life_gained() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ LifeGained | ValidPlayer$ You | Execute$ TrigGain | TriggerDescription$ Whenever you gain life.",
            &mut id,
        ).unwrap();
        if let TriggerMode::LifeGained { valid_player, .. } = &t.mode {
            assert_eq!(valid_player.as_deref(), Some("You"));
        } else {
            panic!("Expected LifeGained mode");
        }
    }

    #[test]
    fn parse_trigger_life_lost() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ LifeLost | ValidPlayer$ Opponent | Execute$ TrigLost",
            &mut id,
        )
        .unwrap();
        if let TriggerMode::LifeLost { valid_player, .. } = &t.mode {
            assert_eq!(valid_player.as_deref(), Some("Opponent"));
        } else {
            panic!("Expected LifeLost mode");
        }
    }

    #[test]
    fn parse_trigger_counter_added() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ CounterAdded | ValidCard$ Card.Self | CounterType$ P1P1 | Execute$ TrigCounter",
            &mut id,
        )
        .unwrap();
        if let TriggerMode::CounterAdded {
            valid_card,
            counter_type,
        } = &t.mode
        {
            assert_eq!(valid_card.as_deref(), Some("Card.Self"));
            assert_eq!(counter_type.as_deref(), Some("P1P1"));
        } else {
            panic!("Expected CounterAdded mode");
        }
    }

    #[test]
    fn parse_trigger_counter_removed() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ CounterRemoved | ValidCard$ Creature | CounterType$ M1M1 | Execute$ TrigRemove",
            &mut id,
        )
        .unwrap();
        if let TriggerMode::CounterRemoved {
            valid_card,
            counter_type,
        } = &t.mode
        {
            assert_eq!(valid_card.as_deref(), Some("Creature"));
            assert_eq!(counter_type.as_deref(), Some("M1M1"));
        } else {
            panic!("Expected CounterRemoved mode");
        }
    }

    #[test]
    fn parse_trigger_sacrificed() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ Sacrificed | ValidCard$ Creature | ValidPlayer$ You | Execute$ TrigSac",
            &mut id,
        )
        .unwrap();
        if let TriggerMode::Sacrificed {
            valid_card,
            valid_player,
        } = &t.mode
        {
            assert_eq!(valid_card.as_deref(), Some("Creature"));
            assert_eq!(valid_player.as_deref(), Some("You"));
        } else {
            panic!("Expected Sacrificed mode");
        }
    }

    #[test]
    fn parse_trigger_drawn() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ Drawn | ValidPlayer$ You | Execute$ TrigDraw | TriggerDescription$ Whenever you draw a card.",
            &mut id,
        ).unwrap();
        if let TriggerMode::Drawn { valid_player, .. } = &t.mode {
            assert_eq!(valid_player.as_deref(), Some("You"));
        } else {
            panic!("Expected Drawn mode");
        }
    }

    #[test]
    fn parse_trigger_milled() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ Milled | ValidCard$ Card | ValidPlayer$ Opponent | Execute$ TrigMill",
            &mut id,
        )
        .unwrap();
        if let TriggerMode::Milled {
            valid_card,
            valid_player,
        } = &t.mode
        {
            assert_eq!(valid_card.as_deref(), Some("Card"));
            assert_eq!(valid_player.as_deref(), Some("Opponent"));
        } else {
            panic!("Expected Milled mode");
        }
    }

    #[test]
    fn parse_trigger_taps() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ Taps | ValidCard$ Creature | Execute$ TrigTap",
            &mut id,
        )
        .unwrap();
        assert!(matches!(t.mode, TriggerMode::Taps { .. }));
    }

    #[test]
    fn parse_trigger_untaps() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ Untaps | ValidCard$ Creature.Self | Execute$ TrigUntap",
            &mut id,
        )
        .unwrap();
        assert!(matches!(t.mode, TriggerMode::Untaps { .. }));
    }

    #[test]
    fn parse_trigger_transformed() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ Transformed | ValidCard$ Card.Self | Execute$ TrigTransform",
            &mut id,
        )
        .unwrap();
        assert!(matches!(t.mode, TriggerMode::Transformed { .. }));
    }

    #[test]
    fn parse_trigger_attached() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ Attached | ValidCard$ Card.Self | Execute$ TrigAttach",
            &mut id,
        )
        .unwrap();
        assert!(matches!(t.mode, TriggerMode::Attached { .. }));
    }

    #[test]
    fn parse_trigger_unattached() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ Unattached | ValidCard$ Card.Self | Execute$ TrigDetach",
            &mut id,
        )
        .unwrap();
        assert!(matches!(t.mode, TriggerMode::Unattached { .. }));
    }

    #[test]
    fn parse_trigger_land_played() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ LandPlayed | ValidCard$ Land.YouCtrl | Execute$ TrigLandfall | TriggerDescription$ Landfall.",
            &mut id,
        ).unwrap();
        if let TriggerMode::LandPlayed { valid_card } = &t.mode {
            assert_eq!(valid_card.as_deref(), Some("Land.YouCtrl"));
        } else {
            panic!("Expected LandPlayed mode");
        }
    }

    #[test]
    fn parse_trigger_becomes_target() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ BecomesTarget | ValidSource$ Spell | ValidTarget$ Creature.Self | Execute$ TrigTarget",
            &mut id,
        )
        .unwrap();
        assert!(matches!(t.mode, TriggerMode::BecomesTarget { .. }));
    }

    #[test]
    fn parse_trigger_taps_for_mana() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ TapsForMana | ValidCard$ Land | Execute$ TrigMana | TriggerDescription$ Whenever a land is tapped for mana.",
            &mut id,
        ).unwrap();
        if let TriggerMode::TapsForMana { valid_card, .. } = &t.mode {
            assert_eq!(valid_card.as_deref(), Some("Land"));
        } else {
            panic!("Expected TapsForMana mode");
        }
    }

    #[test]
    fn parse_trigger_ability_activated() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ AbilityActivated | ValidCard$ Creature | ValidActivatingPlayer$ You | Execute$ TrigAct",
            &mut id,
        ).unwrap();
        if let TriggerMode::AbilityActivated {
            valid_card,
            valid_activating_player,
        } = &t.mode
        {
            assert_eq!(valid_card.as_deref(), Some("Creature"));
            assert_eq!(valid_activating_player.as_deref(), Some("You"));
        } else {
            panic!("Expected AbilityActivated mode");
        }
    }

    #[test]
    fn parse_trigger_explored() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ Explores | ValidCard$ Creature.Self | ValidExplored$ Land | Execute$ TrigExplore",
            &mut id,
        )
        .unwrap();
        if let TriggerMode::Explored {
            valid_card,
            valid_explored,
        } = &t.mode
        {
            assert_eq!(valid_card.as_deref(), Some("Creature.Self"));
            assert_eq!(valid_explored.as_deref(), Some("Land"));
        } else {
            panic!("Expected Explored mode");
        }
    }

    #[test]
    fn parse_trigger_become_monarch() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ BecomeMonarch | ValidPlayer$ You | Execute$ TrigMonarch",
            &mut id,
        )
        .unwrap();
        if let TriggerMode::BecomeMonarch { valid_player } = &t.mode {
            assert_eq!(valid_player.as_deref(), Some("You"));
        } else {
            panic!("Expected BecomeMonarch mode");
        }
    }

    #[test]
    fn parse_trigger_damage_dealt_once() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ DamageDealtOnce | ValidSource$ Creature.Self | CombatDamage$ True | Execute$ TrigOnce",
            &mut id,
        ).unwrap();
        if let TriggerMode::DamageDealtOnce {
            valid_source,
            combat_damage_only,
            ..
        } = &t.mode
        {
            assert_eq!(valid_source.as_deref(), Some("Creature.Self"));
            assert!(*combat_damage_only);
        } else {
            panic!("Expected DamageDealtOnce mode");
        }
    }

    #[test]
    fn parse_trigger_destroyed() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ Destroyed | ValidCard$ Creature | ValidCauser$ Card.Self | Execute$ TrigDestroy",
            &mut id,
        )
        .unwrap();
        if let TriggerMode::Destroyed {
            valid_card,
            valid_causer,
        } = &t.mode
        {
            assert_eq!(valid_card.as_deref(), Some("Creature"));
            assert_eq!(valid_causer.as_deref(), Some("Card.Self"));
        } else {
            panic!("Expected Destroyed mode");
        }
    }

    #[test]
    fn parse_trigger_static_flag() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ TurnFaceUp | ValidCard$ Card.Self | Static$ True | Execute$ TrigStatic",
            &mut id,
        )
        .unwrap();
        assert!(t.is_static());
    }

    #[test]
    fn parse_trigger_exiled() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ Exiled | ValidCard$ Card | Execute$ TrigExile",
            &mut id,
        )
        .unwrap();
        assert!(matches!(t.mode, TriggerMode::Exiled { .. }));
    }

    #[test]
    fn parse_trigger_token_created() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ TokenCreated | ValidCard$ Creature | Execute$ TrigToken",
            &mut id,
        )
        .unwrap();
        assert!(matches!(t.mode, TriggerMode::TokenCreated { .. }));
    }

    #[test]
    fn parse_trigger_optional_decider() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ ChangesZone | Destination$ Battlefield | ValidCard$ Creature.Other | OptionalDecider$ You | Execute$ TrigMay | TriggerDescription$ May draw.",
            &mut id,
        ).unwrap();
        assert!(t.optional);
        assert_eq!(t.description, "May draw.");
    }

    #[test]
    fn parse_trigger_countered() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ Countered | ValidCard$ Card | Execute$ TrigCountered",
            &mut id,
        )
        .unwrap();
        assert!(matches!(t.mode, TriggerMode::Countered { .. }));
    }

    #[test]
    fn parse_trigger_custom_zones() {
        let mut id = 0;
        let t = parse_trigger(
            "Mode$ Drawn | ValidPlayer$ You | Execute$ TrigDraw | TriggerZones$ Battlefield,Graveyard",
            &mut id,
        ).unwrap();
        assert_eq!(
            t.active_zones,
            vec![ZoneType::Battlefield, ZoneType::Graveyard]
        );
    }
}
