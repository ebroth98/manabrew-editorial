//! Static ability parsing and types.
//!
//! Mirrors the Java Forge `forge/game/staticability/` package, specifically
//! `StaticAbility.java` and `StaticAbilityContinuous.java`.
//!
//! Card scripts encode static abilities as `S$`-prefixed lines, e.g.:
//! ```text
//! S$ Mode$ Continuous | Affected$ Creature.YouControl | AddPower$ 1 | AddToughness$ 1 | Description$ Creatures you control get +1/+1.
//! S$ Mode$ ETBTapped | Description$ This permanent enters the battlefield tapped.
//! S$ Mode$ CantAttack | Affected$ Creature.YouControl | Description$ Creatures you control can't attack.
//! ```

use std::collections::HashMap;

use forge_foundation::ColorSet;
use forge_foundation::ZoneType;
use serde::{Deserialize, Serialize};

use crate::card::Card;
use crate::card_trait_base::{CardTrait, CardTraitBase};
use crate::core::HasSVars;
use crate::game::GameState;
use crate::ids::{CardId, PlayerId};
use crate::parsing::keys;
use crate::parsing::Params;

const STATIC_ZONE_KEYS: &[&str] = &[keys::ACTIVE_ZONES, keys::EFFECT_ZONE];

const STATIC_CONDITION_KEYS: &[&str] = &[
    keys::PHASES,
    keys::CONDITION,
    keys::PLAYER_TURN,
    "TopCardOfLibraryIs",
    "ClassLevel",
    "CheckSecondSVar",
    "CheckThirdSVar",
    "CheckFourthSVar",
];

// ── Mode ────────────────────────────────────────────────────────────────────

/// The mode of a static ability.
///
/// Mirrors Java `StaticAbilityMode` enum. Each variant corresponds to a
/// `Mode$ <Value>` entry in the card script.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StaticMode {
    /// `Mode$ Continuous` — layer-based continuous effects (anthems, keyword
    /// grants, P/T setting). The most common category; handled by the layer
    /// system in `layer.rs`.
    Continuous,

    /// `Mode$ CantAttack` — affected creatures cannot attack.
    CantAttack,

    /// `Mode$ CantBlock` — affected creatures cannot block.
    CantBlock,

    /// `Mode$ ETBTapped` — this permanent (or matching permanents) enters the
    /// battlefield tapped. Applied once at ETB time, not continuously.
    ETBTapped,

    /// `Mode$ CantBeCast` — matching spells cannot be cast.
    CantBeCast,
    /// `Mode$ CantBeActivated` — matching abilities cannot be activated.
    CantBeActivated,
    /// `Mode$ CantPlayLand` — matching lands cannot be played.
    CantPlayLand,

    /// `Mode$ ReduceCost` — reduce the mana cost of matching spells.
    ReduceCost,

    /// `Mode$ IncreaseCost` — increase the mana cost of matching spells.
    IncreaseCost,

    /// `Mode$ SetCost` — raise cost to a minimum (Trinisphere). Used with `RaiseTo$`.
    SetCost,
    CantTarget,
    CantAttach,
    MustAttack,
    MustBlock,
    Panharmonicon,
    CantGainLosePayLife,
    CantDraw,
    CantExile,
    CantSacrifice,
    CantRegenerate,
    DisableTriggers,
    CantPutCounter,
    CastWithFlash,
    BlockRestrict,
    AttackRestrict,
    CanAttackDefender,
    IgnoreHexproof,
    IgnoreShroud,
    IgnoreLegendRule,
    MustTarget,
    AssignCombatDamageAsUnblocked,
    AssignNoCombatDamage,
    CombatDamageToughness,
    NoCleanupDamage,
    InfectDamage,
    WitherDamage,
    ColorlessDamageSource,
    CountersRemain,
    MaxCounter,
    /// `Mode$ CantAttackUnless` — attacker must pay a cost to attack (Propaganda, Ghostly Prison).
    CantAttackUnless,
    /// `Mode$ OptionalAttackCost` — optional attack payment like Exert/Enlist.
    OptionalAttackCost,
    /// `Mode$ CantBlockUnless` — blocker must pay a cost to block (War Cadence).
    CantBlockUnless,
    /// `Mode$ CantBlockBy` — restricts which blockers can block an attacker
    /// (Flying, Fear, Intimidate, Skulk, or card-specific restrictions).
    CantBlockBy,
    /// `Mode$ ManaConvert` — spend mana as though it were mana of any color/type.
    ManaConvert,
    /// `Mode$ UnspentMana` — mana of specified type doesn't empty from pool.
    UnspentMana,
    /// `Mode$ ManaBurn` — losing unspent mana causes life loss (Yurlok of Scorch Thrash).
    ManaBurn,
    ActivateAbilityAsIfHaste,
    CanAdapt,
    AlternativeCost,
    CantAttackBlock,
    CantBeCopied,
    CantBeSuspected,
    CantBecomeMonarch,
    CantChangeDayTime,
    CantCrew,
    CantDiscard,
    CantPhaseIn,
    CantPhaseOut,
    CantTransform,
    CantVenture,
    Devotion,
    CanExhaust,
    FlipCoinMod,
    GainLifeRadiation,
    IgnoreLandwalk,
    NumLoyaltyAct,
    PlotZone,
    SurveilNum,
    TapPowerValue,
    TurnReversed,
    PhaseReversed,
    UntapOtherPlayer,
    CanBlockIfReach,
    BlockTapped,
    CanAttackIfHaste,
    MinMaxBlocker,
    AttackVigilance,
    CantPreventDamage,
    CantGainLife,
    CantLoseLife,
    CantPayLife,
    CantChangeLife,

    /// Any mode not yet recognised — stored but not applied.
    Other(String),
}

// ── Layer ────────────────────────────────────────────────────────────────────

/// CR 613 layer ordering for continuous effects.
///
/// Effects are applied in ascending numeric order. Timestamp ordering within
/// the same layer is preserved by the order in which effects are collected
/// (battlefield entry order in `GameState.cards`).
///
/// Reference: <https://magic.wizards.com/en/rules> CR 613
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Layer {
    /// Layer 2 — control-changing effects (`GainControl$`).
    Control = 2,
    /// Layer 4 — type-changing effects (`AddType$`, `RemoveType$`).
    Type = 4,
    /// Layer 5 — color-changing effects (`AddColor$`).
    Color = 5,
    /// Layer 6 — ability-adding / removing (`AddKeyword$`).
    Ability = 6,
    /// Layer 7b — P/T set to an absolute value (`SetPower$`, `SetToughness$`).
    /// Note: 7a (CDAs) are not yet implemented.
    SetPT = 71,
    /// Layer 7c — P/T modifications: bonuses and penalties (`AddPower$`, `AddToughness$`).
    ModifyPT = 72,
    // Layer 7d (counters) is handled intrinsically by `Card::power()`
    // and `Card::toughness()` — no special layer entry needed.
}

// ── StaticAbility ────────────────────────────────────────────────────────────

/// A parsed static ability from an `S$` line in a card script.
///
/// Params are stored exactly as they appear in the script so that new param
/// types can be added without changing this struct.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticAbility {
    #[serde(default)]
    pub base: Box<CardTraitBase>,
    pub mode: StaticMode,
    /// Parsed key→value parameters from the pipe-separated script line.
    /// Keys do NOT include the trailing `$`.
    pub params: Params,
    pub ignore_effect_cards: Vec<CardId>,
    pub ignore_effect_players: Vec<PlayerId>,
    pub may_play_turn: i32,
    /// Mirrors `CardTraitBase.sVars` in Java. Populated by the card factory
    /// with the host card's SVar map so that `$`-expressions evaluated under
    /// the ability resolve against the card's SVars (not the ability's
    /// mapParams).
    #[serde(default)]
    pub svars: HashMap<String, String>,
}

impl StaticAbility {
    fn sync_trait_base_params(&mut self) {
        let map = self
            .params
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        self.base.set_map_params(map);
        self.base.set_svars(self.svars.clone());
    }

    /// Return the CR 613 layer this `Continuous` ability belongs to.
    ///
    /// Returns `None` for non-`Continuous` abilities or continuous abilities
    /// whose effect type is not yet recognised.
    pub fn continuous_layer(&self) -> Option<Layer> {
        if self.mode != StaticMode::Continuous {
            return None;
        }
        // Presence of specific params determines the layer (mirrors Java
        // `StaticAbilityContinuous.getLayer()`).
        if self.params.has(keys::ADD_POWER) || self.params.has(keys::ADD_TOUGHNESS) {
            Some(Layer::ModifyPT)
        } else if self.params.has(keys::SET_POWER) || self.params.has(keys::SET_TOUGHNESS) {
            Some(Layer::SetPT)
        } else if self.params.has(keys::ADD_KEYWORD) {
            Some(Layer::Ability)
        } else if self.params.has(keys::GAIN_CONTROL) {
            Some(Layer::Control)
        } else if self.params.has(keys::ADD_TYPE) || self.params.has(keys::REMOVE_TYPE) {
            Some(Layer::Type)
        } else if self.params.has(keys::ADD_COLOR) {
            Some(Layer::Color)
        } else {
            None
        }
    }

    pub fn check_mode(&self, mode: &StaticMode) -> bool {
        match (&self.mode, mode) {
            (StaticMode::Other(a), StaticMode::Other(b)) => a.eq_ignore_ascii_case(b),
            // CantAttackBlock matches both CantAttack and CantBlock queries
            (StaticMode::CantAttackBlock, StaticMode::CantAttack) => true,
            (StaticMode::CantAttackBlock, StaticMode::CantBlock) => true,
            _ => self.mode == *mode,
        }
    }

    pub fn zones_check(&self, source_zone: ZoneType) -> bool {
        if !self.params.contains_any_key(STATIC_ZONE_KEYS) {
            return source_zone == ZoneType::Battlefield;
        }

        let _perf_scope = crate::perf::ParamsLookupScopeGuard::enter(
            crate::perf::ParamsLookupScope::StaticAbility,
        );
        if let Some(active) = self.params.get(keys::ACTIVE_ZONES) {
            let zones: Vec<ZoneType> = active
                .split(',')
                .filter_map(|z| ZoneType::from_str_compat(z.trim()))
                .collect();
            if zones.is_empty() {
                return false;
            }
            return zones.contains(&source_zone);
        }
        if let Some(effect_zone) = self.params.get(keys::EFFECT_ZONE) {
            if effect_zone.eq_ignore_ascii_case("All") {
                return true;
            }
            let zones: Vec<ZoneType> = effect_zone
                .split(',')
                .filter_map(|z| ZoneType::from_str_compat(z.trim()))
                .collect();
            if zones.is_empty() {
                return false;
            }
            return zones.contains(&source_zone);
        }
        source_zone == ZoneType::Battlefield
    }

    pub fn check_conditions(&self, source: &Card, game: &GameState) -> bool {
        let _perf_scope = crate::perf::ParamsLookupScopeGuard::enter(
            crate::perf::ParamsLookupScope::StaticAbility,
        );
        if !self.zones_check(source.zone) {
            return false;
        }
        if source.phased_out {
            return false;
        }
        if !crate::card::valid_filter::meets_common_requirements(game, &self.params, source) {
            return false;
        }

        if !self.params.contains_any_key(STATIC_CONDITION_KEYS) {
            return true;
        }

        if let Some(phases) = self.params.get(keys::PHASES) {
            let current = format!("{:?}", game.turn.phase);
            if !phases
                .split(',')
                .map(str::trim)
                .any(|p| p.eq_ignore_ascii_case(&current))
            {
                return false;
            }
        }

        if let Some(condition) = self.params.get(keys::CONDITION) {
            if condition.eq_ignore_ascii_case("MaxSpeed")
                && game.player(source.controller).speed != 4
            {
                return false;
            }
        }

        if let Some(player_turn) = self.params.get(keys::PLAYER_TURN) {
            let active = game.turn.active_player;
            let defined = crate::ability::effects::helpers::resolve_defined_players(
                player_turn,
                source.controller,
                game,
            );
            let ok = defined.contains(&active);
            if !ok {
                return false;
            }
        }

        if let Some(valid_top) = self.params.get("TopCardOfLibraryIs") {
            let top = game
                .zone(ZoneType::Library, source.controller)
                .peek_top()
                .map(|cid| game.card(cid));
            let Some(top_card) = top else {
                return false;
            };
            if !crate::card::valid_filter::matches_valid_card_opt(Some(valid_top), top_card, source)
            {
                return false;
            }
        }

        if let Some(class_level) = self.params.get("ClassLevel") {
            let min = class_level.parse::<i32>().unwrap_or(0);
            if source.class_level < min {
                return false;
            }
        }

        if !crate::card::valid_filter::check_named_svar_condition(
            game,
            &self.params,
            source,
            source,
            "CheckSecondSVar",
            "SecondSVarCompare",
        ) {
            return false;
        }
        if !crate::card::valid_filter::check_named_svar_condition(
            game,
            &self.params,
            source,
            source,
            "CheckThirdSVar",
            "ThirdSVarCompare",
        ) {
            return false;
        }
        if !crate::card::valid_filter::check_named_svar_condition(
            game,
            &self.params,
            source,
            source,
            "CheckFourthSVar",
            "FourthSVarCompare",
        ) {
            return false;
        }

        true
    }

    pub fn check_conditions_full(
        &self,
        mode: &StaticMode,
        source: &Card,
        game: &GameState,
    ) -> bool {
        self.check_mode(mode) && self.check_conditions(source, game)
    }

    pub fn is_active_for(&self, mode: StaticMode, source_zone: ZoneType) -> bool {
        self.check_mode(&mode) && self.zones_check(source_zone)
    }

    pub fn add_ignore_effect_players(&mut self, player: PlayerId) {
        if !self.ignore_effect_players.contains(&player) {
            self.ignore_effect_players.push(player);
        }
    }

    pub fn clear_ignore_effects(&mut self) {
        self.ignore_effect_cards.clear();
        self.ignore_effect_players.clear();
    }

    pub fn inc_may_play_turn(&mut self) {
        self.may_play_turn += 1;
    }

    pub fn reset_may_play_turn(&mut self) {
        self.may_play_turn = 0;
    }

    pub fn copy(&self) -> Self {
        self.clone()
    }
}

// ── CardFilter ───────────────────────────────────────────────────────────────

/// Filter for which permanents are affected by a static ability.
///
/// Parsed from the `Affected$` or `ValidCards$` parameter, which mirrors the
/// `Card.isValid()` logic in Java Forge's `StaticAbilityContinuous`.
///
/// Format: `BaseType[.Qualifier][+Qualifier...]`
///
/// Examples:
/// - `"Creature.YouControl"` — creatures you control
/// - `"Creature.White+YouCtrl"` — white creatures you control (Honor of the Pure)
/// - `"Creature.Other+YouControl"` — creatures you control other than this card
/// - `"Creature.Goblin+YouControl"` — Goblins you control
/// - `"Permanent.YouControl"` — all permanents you control
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CardFilter {
    /// Only match cards with the `Creature` core type.
    pub creatures_only: bool,
    /// Only match cards controlled by the ability source's controller.
    pub controller_only: bool,
    /// Only match cards owned by the ability source's controller (`YouOwn`).
    pub owner_only: bool,
    /// Exclude the source card itself (`Other` qualifier).
    pub other_only: bool,
    /// Only match commanders.
    pub commander_only: bool,
    /// Only match cards with this subtype (e.g. `"Goblin"`, `"Warrior"`).
    pub subtype: Option<String>,
    /// Only match non-land permanents.
    pub nonland_only: bool,
    /// Only match land permanents.
    pub land_only: bool,
    /// Only match cards that include this color (e.g. White for Honor of the Pure).
    /// `None` means no color restriction.
    pub required_color: Option<ColorSet>,
    /// Only match colorless cards (`Colorless` qualifier).
    pub colorless_only: bool,
    /// Only match creatures currently attacking the source's controller
    /// (`attackingYou` qualifier, e.g. Watchdog).
    pub attacking_you: bool,
    /// Only match cards with this exact name (`named<CardName>` qualifier).
    pub card_name: Option<String>,
    /// Only match token permanents.
    pub token_only: bool,
    /// Only match cards that share a color with the source card.
    pub shares_color_with_source: bool,
    /// Only match cards that share a color with the source's equipped creature.
    pub shares_color_with_equipped: bool,
    /// Only match cards that share a creature type with the source card.
    pub shares_creature_type_with_source: bool,
    /// Only match cards that share a creature type with the source's equipped creature.
    pub shares_creature_type_with_equipped: bool,
}

impl CardFilter {
    /// Parse an `Affected$` / `ValidCards$` value string into a `CardFilter`.
    pub fn parse(s: &str) -> Self {
        let mut f = CardFilter::default();
        // The string may be "BaseType.Q1.Q2+Q3+Q4".
        // Split on '+' first, then on '.' within each segment.
        let mut parts = s.split('+');
        // First segment contains the base type (possibly with dot qualifiers).
        let base = parts.next().unwrap_or("").trim();
        for seg in base.split('.') {
            Self::apply_segment(&mut f, seg.trim());
        }
        // Remaining '+'-separated parts are all qualifiers.
        for part in parts {
            Self::apply_segment(&mut f, part.trim());
        }
        f
    }

    fn apply_segment(f: &mut CardFilter, seg: &str) {
        match seg {
            "Creature" => f.creatures_only = true,
            // "Permanent" and "Card" impose no additional restriction.
            "Permanent" | "Card" | "" => {}
            "nonLand" | "NonLand" => f.nonland_only = true,
            "Land" => f.land_only = true,
            "YouControl" | "YouCtrl" => f.controller_only = true,
            "YouOwn" => f.owner_only = true,
            "Other" => f.other_only = true,
            "IsCommander" => f.commander_only = true,
            // Color qualifiers (e.g. "Creature.White+YouCtrl" for Honor of the Pure).
            "White" => f.required_color = Some(ColorSet::WHITE),
            "Blue" => f.required_color = Some(ColorSet::BLUE),
            "Black" => f.required_color = Some(ColorSet::BLACK),
            "Red" => f.required_color = Some(ColorSet::RED),
            "Green" => f.required_color = Some(ColorSet::GREEN),
            "Colorless" => f.colorless_only = true,
            "attackingYou" => f.attacking_you = true,
            "token" | "Token" => f.token_only = true,
            "SharesColorWith" => f.shares_color_with_source = true,
            "sharesCreatureTypeWith" => f.shares_creature_type_with_source = true,
            "SharesColorWith Equipped" => f.shares_color_with_equipped = true,
            "sharesCreatureTypeWith Equipped" => f.shares_creature_type_with_equipped = true,
            s if s.starts_with("named") => {
                f.card_name = Some(s["named".len()..].to_string());
            }
            s => {
                // Unknown tokens are treated as subtype filters (e.g. "Goblin").
                if f.subtype.is_none() {
                    f.subtype = Some(s.to_string());
                }
            }
        }
    }

    /// Returns `true` if `card` passes this filter given `source` is the
    /// static ability's host card.
    pub fn matches(&self, card: &Card, source: &Card) -> bool {
        if self.creatures_only && !card.is_creature() {
            return false;
        }
        if self.controller_only && card.controller != source.controller {
            return false;
        }
        if self.owner_only && card.owner != source.controller {
            return false;
        }
        if self.other_only && card.id == source.id {
            return false;
        }
        if self.commander_only && !card.is_commander {
            return false;
        }
        if let Some(ref sub) = self.subtype {
            if !card.has_subtype(sub) {
                return false;
            }
        }
        if self.nonland_only && card.is_land() {
            return false;
        }
        if self.land_only && !card.is_land() {
            return false;
        }
        if let Some(required) = self.required_color {
            if !card.color.shares_color_with(required) {
                return false;
            }
        }
        if self.colorless_only && !card.color.is_colorless() {
            return false;
        }
        if self.attacking_you && card.attacking_player != Some(source.controller) {
            return false;
        }
        if let Some(ref name) = self.card_name {
            if card.card_name != *name {
                return false;
            }
        }
        if self.token_only && !card.is_token {
            return false;
        }
        if self.shares_color_with_source {
            if card.color.is_colorless() || source.color.is_colorless() {
                return false;
            }
            if !card.color.shares_color_with(source.color) {
                return false;
            }
        }
        if self.shares_creature_type_with_source && !shares_creature_type_with(card, source) {
            return false;
        }
        true
    }

    /// Context-aware matching for predicates that require game lookups
    /// (e.g. `SharesColorWith Equipped`).
    pub fn matches_with_game(&self, card: &Card, source: &Card, game: &GameState) -> bool {
        if !self.matches(card, source) {
            return false;
        }
        if self.shares_color_with_equipped {
            let Some(equipped_id) = source.attached_to else {
                return false;
            };
            let equipped = game.card(equipped_id);
            if card.color.is_colorless() || equipped.color.is_colorless() {
                return false;
            }
            if !card.color.shares_color_with(equipped.color) {
                return false;
            }
        }
        if self.shares_creature_type_with_equipped {
            let Some(equipped_id) = source.attached_to else {
                return false;
            };
            let equipped = game.card(equipped_id);
            if !shares_creature_type_with(card, equipped) {
                return false;
            }
        }
        true
    }
}

fn shares_creature_type_with(a: &Card, b: &Card) -> bool {
    a.shares_creature_type_with(b)
}

// ── Parser ───────────────────────────────────────────────────────────────────

/// Parse a raw `S$` (or `S:`) ability line from a card script into a
/// [`StaticAbility`].
///
/// Returns `None` if the line does not start with the `S$` / `S:` prefix or
/// has no recognisable `Mode$` param.
///
/// # Format
///
/// ```text
/// S$ Mode$ Continuous | Affected$ Creature.YouControl | AddPower$ 1 | AddToughness$ 1
/// S$ Mode$ ETBTapped | Description$ Enters tapped.
/// ```
///
/// Reference: Java `StaticAbility.java` in `forge/game/staticability/`.
pub fn parse_static_ability(raw: &str) -> Option<StaticAbility> {
    let trimmed = raw.trim();
    // Accept "S$ ..." or "S: ..." prefixes (both appear in Forge card files).
    let body = if let Some(rest) = trimmed.strip_prefix("S$ ") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("S:") {
        rest.trim_start()
    } else {
        return None;
    };

    // Parse "|"-separated "Key$ Value" pairs using central parser.
    let params = Params::from_raw(body);

    let mode = match params.get(keys::MODE) {
        Some("Continuous") => StaticMode::Continuous,
        Some("CantAttack") => StaticMode::CantAttack,
        Some("CantBlock") => StaticMode::CantBlock,
        Some("ETBTapped") => StaticMode::ETBTapped,
        Some("CantBeCast") => StaticMode::CantBeCast,
        Some("CantBeActivated") => StaticMode::CantBeActivated,
        Some("CantPlayLand") => StaticMode::CantPlayLand,
        Some("ReduceCost") => StaticMode::ReduceCost,
        Some("IncreaseCost") | Some("RaiseCost") => StaticMode::IncreaseCost,
        Some("SetCost") => StaticMode::SetCost,
        Some("CantTarget") => StaticMode::CantTarget,
        Some("CantAttach") => StaticMode::CantAttach,
        Some("MustAttack") => StaticMode::MustAttack,
        Some("MustBlock") => StaticMode::MustBlock,
        Some("Panharmonicon") => StaticMode::Panharmonicon,
        Some("CantGainLosePayLife") => StaticMode::CantGainLosePayLife,
        Some("CantDraw") => StaticMode::CantDraw,
        Some("CantExile") => StaticMode::CantExile,
        Some("CantSacrifice") => StaticMode::CantSacrifice,
        Some("CantRegenerate") => StaticMode::CantRegenerate,
        Some("DisableTriggers") => StaticMode::DisableTriggers,
        Some("CantPutCounter") => StaticMode::CantPutCounter,
        Some("CastWithFlash") => StaticMode::CastWithFlash,
        Some("BlockRestrict") => StaticMode::BlockRestrict,
        Some("AttackRestrict") => StaticMode::AttackRestrict,
        Some("CanAttackDefender") => StaticMode::CanAttackDefender,
        Some("IgnoreHexproof") => StaticMode::IgnoreHexproof,
        Some("IgnoreShroud") => StaticMode::IgnoreShroud,
        Some("IgnoreLegendRule") => StaticMode::IgnoreLegendRule,
        Some("MustTarget") => StaticMode::MustTarget,
        Some("AssignCombatDamageAsUnblocked") => StaticMode::AssignCombatDamageAsUnblocked,
        Some("AssignNoCombatDamage") => StaticMode::AssignNoCombatDamage,
        Some("CombatDamageToughness") => StaticMode::CombatDamageToughness,
        Some("NoCleanupDamage") => StaticMode::NoCleanupDamage,
        Some("InfectDamage") => StaticMode::InfectDamage,
        Some("WitherDamage") => StaticMode::WitherDamage,
        Some("ColorlessDamageSource") => StaticMode::ColorlessDamageSource,
        Some("CountersRemain") => StaticMode::CountersRemain,
        Some("MaxCounter") => StaticMode::MaxCounter,
        Some("CantAttackUnless") => StaticMode::CantAttackUnless,
        Some("OptionalAttackCost") => StaticMode::OptionalAttackCost,
        Some("CantBlockUnless") => StaticMode::CantBlockUnless,
        Some("CantBlockBy") => StaticMode::CantBlockBy,
        Some("ManaConvert") => StaticMode::ManaConvert,
        Some("UnspentMana") => StaticMode::UnspentMana,
        Some("ManaBurn") => StaticMode::ManaBurn,
        Some("ActivateAbilityAsIfHaste") => StaticMode::ActivateAbilityAsIfHaste,
        Some("CanAdapt") => StaticMode::CanAdapt,
        Some("AlternativeCost") => StaticMode::AlternativeCost,
        Some("CantAttackBlock") | Some("CantAttack,CantBlock") | Some("CantBlock,CantAttack") => {
            StaticMode::CantAttackBlock
        }
        Some("CantBeCopied") => StaticMode::CantBeCopied,
        Some("CantBeSuspected") => StaticMode::CantBeSuspected,
        Some("CantBecomeMonarch") => StaticMode::CantBecomeMonarch,
        Some("CantChangeDayTime") => StaticMode::CantChangeDayTime,
        Some("CantCrew") => StaticMode::CantCrew,
        Some("CantDiscard") => StaticMode::CantDiscard,
        Some("CantPhaseIn") => StaticMode::CantPhaseIn,
        Some("CantPhaseOut") => StaticMode::CantPhaseOut,
        Some("CantTransform") => StaticMode::CantTransform,
        Some("CantVenture") => StaticMode::CantVenture,
        Some("Devotion") => StaticMode::Devotion,
        Some("CanExhaust") => StaticMode::CanExhaust,
        Some("FlipCoinMod") => StaticMode::FlipCoinMod,
        Some("GainLifeRadiation") => StaticMode::GainLifeRadiation,
        Some("IgnoreLandwalk") => StaticMode::IgnoreLandwalk,
        Some("NumLoyaltyAct") => StaticMode::NumLoyaltyAct,
        Some("PlotZone") => StaticMode::PlotZone,
        Some("SurveilNum") => StaticMode::SurveilNum,
        Some("TapPowerValue") => StaticMode::TapPowerValue,
        Some("TurnReversed") => StaticMode::TurnReversed,
        Some("PhaseReversed") => StaticMode::PhaseReversed,
        Some("UntapOtherPlayer") => StaticMode::UntapOtherPlayer,
        Some("CanBlockIfReach") => StaticMode::CanBlockIfReach,
        Some("BlockTapped") => StaticMode::BlockTapped,
        Some("CanAttackIfHaste") => StaticMode::CanAttackIfHaste,
        Some("MinMaxBlocker") => StaticMode::MinMaxBlocker,
        Some("AttackVigilance") => StaticMode::AttackVigilance,
        Some("CantPreventDamage") => StaticMode::CantPreventDamage,
        Some("CantGainLife") => StaticMode::CantGainLife,
        Some("CantLoseLife") => StaticMode::CantLoseLife,
        Some("CantChangeLife") => StaticMode::CantChangeLife,
        Some("CantPayLife") => StaticMode::CantPayLife,
        Some(other) => StaticMode::Other(other.to_string()),
        None => return None,
    };

    let mut st_ab = StaticAbility {
        base: Box::new(CardTraitBase::default()),
        mode,
        params,
        ignore_effect_cards: Vec::new(),
        ignore_effect_players: Vec::new(),
        may_play_turn: 0,
        svars: HashMap::new(),
    };
    st_ab.sync_trait_base_params();
    Some(st_ab)
}

impl HasSVars for StaticAbility {
    fn get_svar(&self, name: &str) -> Option<&str> {
        self.svars.get(name).map(String::as_str)
    }

    fn set_svar(&mut self, name: String, value: String) {
        self.svars.insert(name.clone(), value.clone());
        self.base.set_svar(name, value);
    }

    fn set_svars(&mut self, new_svars: HashMap<String, String>) {
        self.svars = new_svars.clone();
        self.base.set_svars(new_svars);
    }

    fn get_svars(&self) -> &HashMap<String, String> {
        &self.svars
    }

    fn remove_svar(&mut self, var: &str) {
        self.svars.remove(var);
        self.base.remove_svar(var);
    }
}

impl CardTrait for StaticAbility {
    fn base(&self) -> &CardTraitBase {
        &self.base
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use forge_foundation::{CardTypeLine, ColorSet, ManaCost};

    use crate::card::Card;
    use crate::ids::{CardId, PlayerId};

    fn make_creature(id: u32, owner: u32, subtypes: &[&str]) -> Card {
        let type_str = if subtypes.is_empty() {
            "Creature".to_string()
        } else {
            format!("Creature - {}", subtypes.join(" "))
        };
        Card::new(
            CardId(id),
            "Test".to_string(),
            PlayerId(owner),
            CardTypeLine::parse(&type_str),
            ManaCost::parse("1 G"),
            ColorSet::GREEN,
            Some(2),
            Some(2),
            vec![],
            vec![],
        )
    }

    fn make_land(id: u32, owner: u32) -> Card {
        Card::new(
            CardId(id),
            "Forest".to_string(),
            PlayerId(owner),
            CardTypeLine::parse("Basic Land - Forest"),
            ManaCost::parse(""),
            ColorSet::GREEN,
            None,
            None,
            vec![],
            vec![],
        )
    }

    // ── Parser tests ─────────────────────────────────────────────────────

    #[test]
    fn parse_continuous_anthem() {
        let raw = "S$ Mode$ Continuous | Affected$ Creature.YouControl | AddPower$ 1 | AddToughness$ 1 | Description$ Creatures you control get +1/+1.";
        let sa = parse_static_ability(raw).expect("should parse");
        assert_eq!(sa.mode, StaticMode::Continuous);
        assert_eq!(sa.params.get("AddPower"), Some("1"));
        assert_eq!(sa.params.get("AddToughness"), Some("1"));
        assert_eq!(sa.continuous_layer(), Some(Layer::ModifyPT));
    }

    #[test]
    fn parse_etb_tapped() {
        let raw = "S$ Mode$ ETBTapped | Description$ This permanent enters the battlefield tapped.";
        let sa = parse_static_ability(raw).expect("should parse");
        assert_eq!(sa.mode, StaticMode::ETBTapped);
        assert!(sa.continuous_layer().is_none());
    }

    #[test]
    fn parse_cant_attack() {
        let raw = "S$ Mode$ CantAttack | Affected$ Creature.YouControl | Description$ Creatures you control can't attack.";
        let sa = parse_static_ability(raw).expect("should parse");
        assert_eq!(sa.mode, StaticMode::CantAttack);
    }

    #[test]
    fn parse_keyword_grant() {
        let raw = "S$ Mode$ Continuous | Affected$ Creature.YouControl | AddKeyword$ Flying | Description$ Creatures you control have flying.";
        let sa = parse_static_ability(raw).expect("should parse");
        assert_eq!(sa.continuous_layer(), Some(Layer::Ability));
        assert_eq!(sa.params.get("AddKeyword"), Some("Flying"));
    }

    #[test]
    fn parse_set_pt() {
        let raw =
            "S$ Mode$ Continuous | Affected$ Creature.YouControl | SetPower$ 0 | SetToughness$ 1";
        let sa = parse_static_ability(raw).expect("should parse");
        assert_eq!(sa.continuous_layer(), Some(Layer::SetPT));
    }

    #[test]
    fn parse_s_colon_prefix() {
        // Some older Forge card scripts use "S:" instead of "S$".
        let raw =
            "S: Mode$ Continuous | Affected$ Creature.YouControl | AddPower$ 2 | AddToughness$ 2";
        let sa = parse_static_ability(raw).expect("should parse S: prefix");
        assert_eq!(sa.mode, StaticMode::Continuous);
    }

    #[test]
    fn non_static_line_returns_none() {
        assert!(parse_static_ability("AB$ Mana | Cost$ T | Produced$ G").is_none());
        assert!(parse_static_ability("T$ Mode$ ChangesZone").is_none());
        assert!(parse_static_ability("").is_none());
    }

    // ── CardFilter tests ─────────────────────────────────────────────────

    #[test]
    fn filter_creature_you_control() {
        let f = CardFilter::parse("Creature.YouControl");
        assert!(f.creatures_only);
        assert!(f.controller_only);
        assert!(!f.other_only);
        assert!(f.subtype.is_none());
    }

    #[test]
    fn filter_creature_other_you_control() {
        let f = CardFilter::parse("Creature.Other+YouControl");
        assert!(f.creatures_only);
        assert!(f.controller_only);
        assert!(f.other_only);
    }

    #[test]
    fn filter_goblin_subtype() {
        let f = CardFilter::parse("Creature.Goblin+YouControl");
        assert!(f.creatures_only);
        assert!(f.controller_only);
        assert_eq!(f.subtype, Some("Goblin".to_string()));
    }

    #[test]
    fn filter_matches_creature() {
        let source = make_creature(0, 0, &[]);
        let target = make_creature(1, 0, &[]);
        let f = CardFilter::parse("Creature.YouControl");
        assert!(f.matches(&target, &source));
    }

    #[test]
    fn filter_excludes_opponent_creatures() {
        let source = make_creature(0, 0, &[]);
        let mut opp = make_creature(1, 1, &[]); // different controller
        opp.controller = PlayerId(1);
        let f = CardFilter::parse("Creature.YouControl");
        assert!(!f.matches(&opp, &source));
    }

    #[test]
    fn filter_excludes_self_with_other() {
        let source = make_creature(0, 0, &[]);
        let f = CardFilter::parse("Creature.Other+YouControl");
        assert!(!f.matches(&source, &source));
    }

    #[test]
    fn filter_excludes_land_with_nonland() {
        let source = make_creature(0, 0, &[]);
        let land = make_land(1, 0);
        let f = CardFilter::parse("Permanent.nonLand+YouControl");
        assert!(!f.matches(&land, &source));
    }

    #[test]
    fn filter_subtype_goblin() {
        let source = make_creature(0, 0, &[]);
        let goblin = make_creature(1, 0, &["Goblin"]);
        let bear = make_creature(2, 0, &["Bear"]);
        let f = CardFilter::parse("Creature.Goblin+YouControl");
        assert!(f.matches(&goblin, &source));
        assert!(!f.matches(&bear, &source));
    }

    // ── Color filter tests ───────────────────────────────────────────────

    fn make_white_creature(id: u32, owner: u32) -> Card {
        Card::new(
            CardId(id),
            "White Knight".to_string(),
            PlayerId(owner),
            CardTypeLine::parse("Creature - Human Knight"),
            ManaCost::parse("W W"),
            ColorSet::WHITE,
            Some(2),
            Some(2),
            vec![],
            vec![],
        )
    }

    #[test]
    fn filter_color_white_parses() {
        let f = CardFilter::parse("Creature.White+YouCtrl");
        assert!(f.creatures_only);
        assert!(f.controller_only);
        assert_eq!(f.required_color, Some(ColorSet::WHITE));
        assert!(
            f.subtype.is_none(),
            "White should not be treated as a subtype"
        );
    }

    #[test]
    fn filter_honor_of_the_pure_matches_white_creature() {
        // Simulate Honor of the Pure: "Creature.White+YouCtrl"
        let source = make_white_creature(0, 0); // Honor of the Pure controlled by player 0
        let white_ally = make_white_creature(1, 0);
        let green_ally = make_creature(2, 0, &[]); // green creature, same controller
        let white_opponent = make_white_creature(3, 1); // white but opponent controls it
        let mut white_opponent = white_opponent;
        white_opponent.controller = PlayerId(1);

        let f = CardFilter::parse("Creature.White+YouCtrl");
        assert!(f.matches(&white_ally, &source), "white ally should match");
        assert!(
            !f.matches(&green_ally, &source),
            "green creature should not match"
        );
        assert!(
            !f.matches(&white_opponent, &source),
            "opponent's white creature should not match"
        );
    }

    #[test]
    fn filter_color_white_does_not_match_colorless() {
        let source = make_white_creature(0, 0);
        let colorless = Card::new(
            CardId(1),
            "Darksteel Myr".to_string(),
            PlayerId(0),
            CardTypeLine::parse("Artifact Creature - Myr"),
            ManaCost::parse("3"),
            ColorSet::COLORLESS,
            Some(0),
            Some(1),
            vec![],
            vec![],
        );
        let f = CardFilter::parse("Creature.White+YouCtrl");
        assert!(
            !f.matches(&colorless, &source),
            "colorless artifact should not be white"
        );
    }
}
