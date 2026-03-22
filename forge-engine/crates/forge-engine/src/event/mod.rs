use forge_foundation::{PhaseType, ZoneType};
use serde::{Deserialize, Serialize};

use crate::ids::{CardId, PlayerId};

/// Event types — mirrors Java TriggerType enum (subset).
/// Expanded to 25 core trigger types (issue #19).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TriggerType {
    ChangesZone,
    Phase,
    SpellCast,
    Attacks,
    DamageDone,
    /// Two creatures fought each other (SP$ Fight).
    Fight,
    /// One or more creatures fought (batched).
    FightOnce,
    /// A card was discarded (SP$ Discard).
    Discarded,
    /// A spell was countered (SP$ Counter).
    Countered,
    // ── New trigger types (issue #19) ──
    /// A creature blocks an attacker.
    Blocks,
    /// An attacker is blocked by at least one creature.
    AttackerBlocked,
    /// An attacker is not blocked.
    AttackerUnblocked,
    /// A player gained life.
    LifeGained,
    /// A player lost life.
    LifeLost,
    /// A counter was added to a permanent.
    CounterAdded,
    /// A counter was removed from a permanent.
    CounterRemoved,
    /// A permanent was sacrificed.
    Sacrificed,
    /// A card was drawn.
    Drawn,
    /// A card was milled (library → graveyard).
    Milled,
    /// A permanent was tapped.
    Taps,
    /// A permanent was untapped.
    Untaps,
    /// A DFC was transformed.
    Transformed,
    /// An aura/equipment was attached.
    Attached,
    /// An aura/equipment was unattached.
    Unattached,
    /// A land was played.
    LandPlayed,
    /// A permanent became the target of a spell or ability.
    BecomesTarget,
    /// A permanent was tapped for mana.
    TapsForMana,
    /// An activated ability was activated.
    AbilityActivated,
    /// A creature explored.
    Explored,
    /// A creature became monstrous.
    BecomeMonstrous,
    /// A player became the monarch.
    BecomeMonarch,
    /// Damage was dealt to a player/creature for the first time this turn.
    DamageDealtOnce,
    /// A permanent was destroyed.
    Destroyed,
    /// A card was exiled.
    Exiled,
    /// A token was created.
    TokenCreated,
    /// A spell was copied (Storm, Replicate, etc.) — used by Magecraft.
    SpellCopied,
    /// A player took the initiative.
    TakeInitiative,
    /// A permanent phased out.
    PhasedOut,
    /// A permanent phased in.
    PhasedIn,
    // ── New trigger types (issue #54) ──
    /// All attackers declared at once (batch).
    AttackersDeclared,
    /// All blockers declared at once (batch).
    BlockersDeclared,
    /// A card changed zones (batch/all variant).
    ChangesZoneAll,
    /// A permanent changed controller.
    ChangesController,
    /// A turn began.
    TurnBegin,
    /// Damage done once (first-time batch).
    DamageDoneOnce,
    /// Any spell cast (all players).
    SpellCastAll,
    /// Any player lost life (all players).
    LifeLostAll,
    /// Counter added once (batch).
    CounterAddedOnce,
    /// Any card discarded (all players).
    DiscardedAll,
    /// A permanent sacrificed once (batch).
    SacrificedOnce,
    /// A card was cycled.
    Cycled,
    /// Always fires (every phase).
    Always,
    /// Fires immediately when registered.
    Immediate,
    /// A player surveilled.
    Surveil,
    /// A player scried.
    Scry,
    /// A card was foretold.
    Foretell,
    /// A player searched their library.
    SearchedLibrary,
    /// A player shuffled their library.
    Shuffled,
    /// Mana was added to a player's pool.
    ManaAdded,
    /// A token was created (batch).
    TokenCreatedOnce,
    /// Any permanent tapped (all).
    TapAll,
    /// Any permanent untapped (all).
    UntapAll,
    /// A permanent became a target (batch).
    BecomesTargetOnce,
    /// An attacker was blocked by a specific creature.
    AttackerBlockedByCreature,
    /// An attacker was blocked (batch).
    AttackerBlockedOnce,
    /// An attacker was unblocked (batch).
    AttackerUnblockedOnce,
    /// Any spell cast (batch).
    SpellCastOnce,
    /// A spell of a specific type was cast.
    SpellCastOfType,
    /// Damage dealt (all instances).
    DamageAll,
    /// Damage was prevented (batch).
    DamagePreventedOnce,
    /// Excess damage dealt.
    ExcessDamage,
    /// Any player gained life (all players).
    LifeGainedAll,
    /// A counter was removed (batch).
    CounterRemovedOnce,
    /// A creature was exerted.
    Exerted,
    /// A player collected evidence.
    CollectEvidence,
    /// A player foraged.
    Forage,
    /// A creature enlisted another creature.
    Enlisted,
    /// A player flipped a coin.
    FlippedCoin,
    /// A player rolled a die.
    RolledDie,
    /// A player completed a die-roll action.
    RolledDieOnce,
    /// Mana was expended (cumulative per-turn tracking for Expend mechanic).
    ManaExpend,
    /// A face-down card was turned face-up (Morph/Megamorph/Manifest).
    TurnFaceUp,
    /// A creature was exploited (Exploit keyword).
    Exploited,
    /// Cumulative upkeep was paid (or not). Mirrors Java TriggerType.PayCumulativeUpkeep.
    PayCumulativeUpkeep,
    // ── Niche trigger types (Planechase, Unfinity, Ikoria, etc.) ──
    /// Chaos ensues (Planechase chaos die result).
    ChaosEnsues,
    /// A player planeswalked to a new plane (Planechase).
    Planeswalk,
    /// A player claimed the prize from an attraction (Unfinity).
    ClaimPrize,
    /// A player advanced their crank counter (Unstable).
    CrankAdvanced,
    /// A Case enchantment was solved (MKM).
    CaseSolved,
    /// A creature became saddled (OTJ).
    BecomesSaddled,
    /// A scheme was set in motion (Archenemy).
    SetInMotion,
    /// A creature mutated onto another (IKO).
    Mutates,
}

/// Typed event parameter keys — mirrors Java AbilityKey enum.
/// In Java this is Map<AbilityKey, Object>. In Rust we use a struct
/// because Rust has no Object type (justified deviation).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunParams {
    pub card: Option<CardId>,
    pub card_lki: Option<CardId>,
    pub origin: Option<ZoneType>,
    pub destination: Option<ZoneType>,
    pub cause_player: Option<PlayerId>,
    pub player: Option<PlayerId>,
    pub phase: Option<PhaseType>,
    pub damage_source: Option<CardId>,
    pub damage_target_player: Option<PlayerId>,
    pub damage_target_card: Option<CardId>,
    pub damage_amount: Option<i32>,
    pub is_combat_damage: Option<bool>,
    pub attacker: Option<CardId>,
    pub defending_player: Option<PlayerId>,
    pub spell_card: Option<CardId>,
    pub spell_controller: Option<PlayerId>,
    /// Second card involved (e.g. second creature in a Fight trigger).
    pub card2: Option<CardId>,
    /// SpellAbility that was countered
    pub spell_ability: Option<crate::spellability::SpellAbility>,
    /// Cause of the event (e.g. counterspell)
    pub cause: Option<crate::spellability::SpellAbility>,
    // ── New fields (issue #19) ──
    /// Blocking creature (for Blocks trigger).
    pub blocker: Option<CardId>,
    /// Attacker being blocked (for Blocks trigger).
    pub blocked_attacker: Option<CardId>,
    /// Life amount gained or lost (for LifeGained/LifeLost triggers).
    pub life_amount: Option<i32>,
    /// Counter type name (for CounterAdded/CounterRemoved triggers).
    pub counter_type: Option<String>,
    /// Number of counters added/removed.
    pub counter_amount: Option<i32>,
    // ── New fields (issue #54) ──
    /// Batch of attacker IDs (for AttackersDeclared).
    pub attacker_ids: Option<Vec<CardId>>,
    /// Batch of blocker IDs (for BlockersDeclared).
    pub blocker_ids: Option<Vec<CardId>>,
    /// Original controller before a control change.
    pub original_controller: Option<PlayerId>,
    /// Cumulative mana expend amount (for ManaExpend trigger).
    pub mana_expend_amount: Option<i32>,
    /// Enlisted card (for TriggerMode::Enlisted).
    pub enlisted: Option<CardId>,
    /// The spell/ability card that caused the event (for BecomesTarget — the targeting spell).
    pub cause_card: Option<CardId>,
    /// Coin-flip outcome (true = win/heads).
    pub coin_flip_won: Option<bool>,
    /// Rolled die result (modified).
    pub die_result: Option<i32>,
    /// Number of sides on the rolled die.
    pub die_sides: Option<i32>,
    /// Number of attackers declared this combat (for Exalted `Alone$ True` check).
    pub num_attackers: Option<usize>,
    /// The creature that was exploited (for Exploited trigger).
    pub exploited_card: Option<CardId>,
    /// LKI +1/+1 counter count on a card that just left the battlefield.
    /// Used by Modular triggers to know how many counters to move.
    pub lki_p1p1_counters: Option<i32>,
    /// Whether cumulative upkeep was paid (for PayCumulativeUpkeep trigger).
    pub cumulative_upkeep_paid: Option<bool>,
    /// Snapshot of drawn_this_turn at the time a Drawn event fires.
    /// Used by `Number$ N` triggers to compare against the exact draw count
    /// at fire time (not at deferred match time).
    pub drawn_this_turn_snapshot: Option<i32>,
}

impl RunParams {
    /// Get a card ID from run-params by AbilityKey.
    /// Provides a generic accessor so code can use `AbilityKey` enum values
    /// to pull data from the typed struct.
    pub fn get_card(&self, key: crate::ability::AbilityKey) -> Option<CardId> {
        use crate::ability::AbilityKey;
        match key {
            AbilityKey::Card => self.card,
            AbilityKey::CardLKI => self.card_lki,
            AbilityKey::DamageSource => self.damage_source,
            AbilityKey::DamageTarget => self.damage_target_card,
            AbilityKey::Attacker => self.attacker,
            AbilityKey::Blocker => self.blocker,
            AbilityKey::Source => self.spell_card,
            AbilityKey::NewCard => self.card2,
            AbilityKey::Enlisted => self.enlisted,
            AbilityKey::Exploited => self.exploited_card,
            AbilityKey::Cause => self.cause_card,
            _ => None,
        }
    }

    /// Get a player ID from run-params by AbilityKey.
    pub fn get_player(&self, key: crate::ability::AbilityKey) -> Option<PlayerId> {
        use crate::ability::AbilityKey;
        match key {
            AbilityKey::Player => self.player,
            AbilityKey::Activator => self.cause_player,
            AbilityKey::DefendingPlayer => self.defending_player,
            AbilityKey::AttackingPlayer => self.spell_controller,
            AbilityKey::OriginalController => self.original_controller,
            AbilityKey::DamageTarget => self.damage_target_player,
            _ => None,
        }
    }

    /// Get an integer amount from run-params by AbilityKey.
    pub fn get_amount(&self, key: crate::ability::AbilityKey) -> Option<i32> {
        use crate::ability::AbilityKey;
        match key {
            AbilityKey::DamageAmount => self.damage_amount,
            AbilityKey::LifeAmount => self.life_amount,
            AbilityKey::CounterAmount => self.counter_amount,
            _ => None,
        }
    }
}
