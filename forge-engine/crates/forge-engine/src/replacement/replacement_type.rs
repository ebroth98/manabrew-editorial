//! Replacement effect event types.
//!
//! Mirrors Java `ReplacementType.java` in `forge/game/replacement/`.
//! Each variant corresponds to an `Event$ <Value>` entry in card scripts.

use serde::{Deserialize, Serialize};

use crate::parsing::{keys, Params};

use super::replacement_effect::{ReplacementEffect, ReplacementLayer};

/// The type of game event a replacement effect intercepts.
///
/// Mirrors Java `ReplacementType` enum (all 36 variants).
///
/// Reference: Java `ReplacementType.java` in `forge/game/replacement/`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplacementType {
    /// `Event$ AddCounter` — a counter being added to a permanent or player.
    AddCounter,

    /// `Event$ AssembleContraption` — assembling a contraption (Unstable).
    AssembleContraption,

    /// `Event$ AssignDealDamage` — assigning/dealing damage (before it resolves).
    AssignDealDamage,

    /// `Event$ Attached` — an aura/equipment becoming attached.
    Attached,

    /// `Event$ BeginPhase` — a phase beginning.
    BeginPhase,

    /// `Event$ BeginTurn` — a turn beginning.
    BeginTurn,

    /// `Event$ Cascade` — cascade trigger.
    Cascade,

    /// `Event$ Counter` — countering a spell.
    Counter,

    /// `Event$ CopySpell` — copying a spell.
    CopySpell,

    /// `Event$ CreateToken` — creating token(s).
    CreateToken,

    /// `Event$ DamageDone` — damage being dealt to a card or player.
    DamageDone,

    /// `Event$ DealtDamage` — after damage has been dealt.
    DealtDamage,

    /// `Event$ DeclareBlocker` — declaring a blocker.
    DeclareBlocker,

    /// `Event$ Destroy` — a permanent being destroyed.
    Destroy,

    /// `Event$ Draw` — a single card draw.
    Draw,

    /// `Event$ DrawCards` — multiple card draws at once.
    DrawCards,

    /// `Event$ Explore` — a creature exploring.
    Explore,

    /// `Event$ GainLife` — a player gaining life.
    GainLife,

    /// `Event$ GameLoss` — a player losing the game (e.g. Platinum Angel).
    GameLoss,

    /// `Event$ GameWin` — a player winning the game.
    GameWin,

    /// `Event$ Learn` — the learn action.
    Learn,

    /// `Event$ LifeReduced` — life being reduced (distinct from damage).
    LifeReduced,

    /// `Event$ LoseMana` — mana being lost from a pool.
    LoseMana,

    /// `Event$ Mill` — cards being milled.
    Mill,

    /// `Event$ Moved` — a card moving between zones (ETB, dies, exile, etc.).
    Moved,

    /// `Event$ PayLife` — a player paying life.
    PayLife,

    /// `Event$ PlanarDiceResult` — result of a planar die roll.
    PlanarDiceResult,

    /// `Event$ Planeswalk` — planeswalking.
    Planeswalk,

    /// `Event$ ProduceMana` — mana being produced (for doublers like Mirari's Wake).
    ProduceMana,

    /// `Event$ Proliferate` — proliferating.
    Proliferate,

    /// `Event$ RemoveCounter` — removing a counter.
    RemoveCounter,

    /// `Event$ RollDice` — rolling a die.
    RollDice,

    /// `Event$ RollPlanarDice` — rolling the planar die.
    RollPlanarDice,

    /// `Event$ Scry` — scrying.
    Scry,

    /// `Event$ SetInMotion` — setting a scheme in motion (Archenemy).
    SetInMotion,

    /// `Event$ Tap` — tapping a permanent.
    Tap,

    /// `Event$ Transform` — transforming a permanent.
    Transform,

    /// `Event$ TurnFaceUp` — turning a face-down permanent face up.
    TurnFaceUp,

    /// `Event$ Untap` — untapping a permanent.
    Untap,

    /// Any event type not yet recognised — stored but not applied.
    Other(String),
}

impl ReplacementType {
    /// Alias for `from_event_str`. Mirrors Java `ReplacementType.smartValueOf()`.
    pub fn smart_value_of(value: &str) -> Self {
        Self::from_event_str(value)
    }

    /// Factory function that creates a `ReplacementEffect` from a `ReplacementType` and params.
    ///
    /// Parses `ActiveZones$` and `Layer$` from the given params, then assembles a
    /// `ReplacementEffect`. Mirrors the Java constructor path
    /// `ReplacementType.createReplacementEffect(Map<String,String>)`.
    pub fn create_replacement(event: &ReplacementType, params: &Params) -> ReplacementEffect {
        use forge_foundation::ZoneType;

        let layer = params
            .get(keys::LAYER)
            .and_then(|s| ReplacementLayer::from_layer_str(s))
            .unwrap_or(ReplacementLayer::Other);

        let active_zones = params
            .get(keys::ACTIVE_ZONES)
            .map(|s| {
                s.split(|c: char| c == ',' || c == ' ')
                    .filter_map(|tok| match tok.trim() {
                        "Battlefield" => Some(ZoneType::Battlefield),
                        "Graveyard" => Some(ZoneType::Graveyard),
                        "Hand" => Some(ZoneType::Hand),
                        "Library" => Some(ZoneType::Library),
                        "Exile" => Some(ZoneType::Exile),
                        "Command" => Some(ZoneType::Command),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        ReplacementEffect {
            event: event.clone(),
            layer,
            params: params.clone(),
            active_zones,
            suppressed: false,
        }
    }

    /// Parse an `Event$` value string into a `ReplacementType`.
    /// Mirrors Java `ReplacementType.smartValueOf()`.
    pub fn from_event_str(s: &str) -> Self {
        match s.trim() {
            "AddCounter" => Self::AddCounter,
            "AssembleContraption" => Self::AssembleContraption,
            "AssignDealDamage" => Self::AssignDealDamage,
            "Attached" => Self::Attached,
            "BeginPhase" => Self::BeginPhase,
            "BeginTurn" => Self::BeginTurn,
            "Cascade" => Self::Cascade,
            "Counter" => Self::Counter,
            "CopySpell" => Self::CopySpell,
            "CreateToken" => Self::CreateToken,
            "DamageDone" => Self::DamageDone,
            "DealtDamage" => Self::DealtDamage,
            "DeclareBlocker" => Self::DeclareBlocker,
            "Destroy" => Self::Destroy,
            "Draw" => Self::Draw,
            "DrawCards" => Self::DrawCards,
            "Explore" => Self::Explore,
            "GainLife" => Self::GainLife,
            "GameLoss" => Self::GameLoss,
            "GameWin" => Self::GameWin,
            "Learn" => Self::Learn,
            "LifeReduced" => Self::LifeReduced,
            "LoseMana" => Self::LoseMana,
            "Mill" => Self::Mill,
            "Moved" => Self::Moved,
            "PayLife" => Self::PayLife,
            "PlanarDiceResult" => Self::PlanarDiceResult,
            "Planeswalk" => Self::Planeswalk,
            "ProduceMana" => Self::ProduceMana,
            "Proliferate" => Self::Proliferate,
            "RemoveCounter" => Self::RemoveCounter,
            "RollDice" => Self::RollDice,
            "RollPlanarDice" => Self::RollPlanarDice,
            "Scry" => Self::Scry,
            "SetInMotion" => Self::SetInMotion,
            "Tap" => Self::Tap,
            "Transform" => Self::Transform,
            "TurnFaceUp" => Self::TurnFaceUp,
            "Untap" => Self::Untap,
            other => Self::Other(other.to_string()),
        }
    }
}
