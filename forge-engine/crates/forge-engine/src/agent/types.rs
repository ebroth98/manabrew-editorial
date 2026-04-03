use serde::{Deserialize, Serialize};

use crate::ids::{CardId, PlayerId};
use crate::spellability::AlternativeCost;

/// A game entity that can be a player or a card (permanent).
/// Used by effects like Proliferate that operate on mixed entity lists.
/// Mirrors Java's `GameEntity` hierarchy used in `chooseEntitiesForEffect`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameEntity {
    Player(PlayerId),
    Card(CardId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayOption {
    pub card_id: CardId,
    pub mode: PlayCardMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayCardMode {
    Normal,
    Alternative(AlternativeCost),
    /// Alternative cost granted by `Mode$ AlternativeCost` static abilities.
    StaticAlternative,
    ForetellExile,
}

/// A target choice that can be a player, a card, or nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetChoice {
    Player(PlayerId),
    Card(CardId),
    None,
}

/// The action a player takes during a main phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainPhaseAction {
    /// Pass priority / end main phase.
    Pass,
    /// Play a card from hand / graveyard / exile / command with a specific cast mode.
    Play(PlayOption),
    /// Tap an untapped land on the battlefield to add its mana to the pool.
    ActivateMana(CardId),
    /// Untap a tapped land and remove its mana from the pool (undo tap).
    UntapMana(CardId),
    /// Activate an ability on a permanent. (source card, ability index)
    ActivateAbility(CardId, usize),
}

/// The action a player takes when asked to pay an attack cost (Propaganda, Ghostly Prison).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatCostAction {
    /// Tap an untapped land to add mana to the pool.
    TapLand(CardId),
    /// Untap a tapped land and remove its mana from the pool (undo).
    UntapLand(CardId),
    /// Pay the cost from the mana pool.
    Pay,
    /// Decline to pay — remove this attacker.
    Decline,
}

/// The action a player takes when interactively paying a mana cost for a spell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManaCostAction {
    /// Tap an untapped land to add mana to the pool.
    TapLand {
        card_id: CardId,
        mana_ability_index: Option<usize>,
        express_choice: Option<u16>,
    },
    /// Untap a tapped land and remove its mana from the pool (undo).
    UntapLand(CardId),
    /// Confirm payment from the mana pool.
    Pay,
    /// Cancel casting this spell.
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManaAbilityOption {
    pub card_id: CardId,
    pub ability_index: usize,
    pub description: String,
}

/// Java-parity binary choice kinds (`PlayerController.BinaryChoiceType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryChoiceKind {
    HeadsOrTails,
    TapOrUntap,
    PlayOrDraw,
    OddsOrEvens,
    UntapOrLeaveTapped,
    LeftOrRight,
    AddOrRemove,
    IncreaseOrDecrease,
}

impl BinaryChoiceKind {
    /// Canonical button labels for each binary choice kind.
    pub fn labels(self) -> (&'static str, &'static str) {
        match self {
            BinaryChoiceKind::HeadsOrTails => ("Heads", "Tails"),
            BinaryChoiceKind::TapOrUntap => ("Tap", "Untap"),
            BinaryChoiceKind::PlayOrDraw => ("Play", "Draw"),
            BinaryChoiceKind::OddsOrEvens => ("Odds", "Evens"),
            BinaryChoiceKind::UntapOrLeaveTapped => ("Untap", "Leave tapped"),
            BinaryChoiceKind::LeftOrRight => ("Left", "Right"),
            BinaryChoiceKind::AddOrRemove => ("Add", "Remove"),
            BinaryChoiceKind::IncreaseOrDecrease => ("Increase", "Decrease"),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            BinaryChoiceKind::HeadsOrTails => "HeadsOrTails",
            BinaryChoiceKind::TapOrUntap => "TapOrUntap",
            BinaryChoiceKind::PlayOrDraw => "PlayOrDraw",
            BinaryChoiceKind::OddsOrEvens => "OddsOrEvens",
            BinaryChoiceKind::UntapOrLeaveTapped => "UntapOrLeaveTapped",
            BinaryChoiceKind::LeftOrRight => "LeftOrRight",
            BinaryChoiceKind::AddOrRemove => "AddOrRemove",
            BinaryChoiceKind::IncreaseOrDecrease => "IncreaseOrDecrease",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollSwapChoice {
    Power,
    Toughness,
}
