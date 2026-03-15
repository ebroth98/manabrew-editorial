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

use std::collections::BTreeMap;

use forge_foundation::ColorSet;
use serde::{Deserialize, Serialize};

use crate::card::CardInstance;

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
    // Layer 7d (counters) is handled intrinsically by `CardInstance::power()`
    // and `CardInstance::toughness()` — no special layer entry needed.
}

// ── StaticAbility ────────────────────────────────────────────────────────────

/// A parsed static ability from an `S$` line in a card script.
///
/// Params are stored exactly as they appear in the script so that new param
/// types can be added without changing this struct.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticAbility {
    pub mode: StaticMode,
    /// Raw key→value pairs parsed from the pipe-separated script line.
    /// Keys do NOT include the trailing `$`.
    pub params: BTreeMap<String, String>,
}

impl StaticAbility {
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
        if self.params.contains_key("AddPower") || self.params.contains_key("AddToughness") {
            Some(Layer::ModifyPT)
        } else if self.params.contains_key("SetPower") || self.params.contains_key("SetToughness") {
            Some(Layer::SetPT)
        } else if self.params.contains_key("AddKeyword") {
            Some(Layer::Ability)
        } else if self.params.contains_key("GainControl") {
            Some(Layer::Control)
        } else if self.params.contains_key("AddType") || self.params.contains_key("RemoveType") {
            Some(Layer::Type)
        } else if self.params.contains_key("AddColor") {
            Some(Layer::Color)
        } else {
            None
        }
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
    /// Exclude the source card itself (`Other` qualifier).
    pub other_only: bool,
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
            "Other" => f.other_only = true,
            // Color qualifiers (e.g. "Creature.White+YouCtrl" for Honor of the Pure).
            "White" => f.required_color = Some(ColorSet::WHITE),
            "Blue" => f.required_color = Some(ColorSet::BLUE),
            "Black" => f.required_color = Some(ColorSet::BLACK),
            "Red" => f.required_color = Some(ColorSet::RED),
            "Green" => f.required_color = Some(ColorSet::GREEN),
            "Colorless" => f.colorless_only = true,
            "attackingYou" => f.attacking_you = true,
            "token" | "Token" => f.token_only = true,
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
    pub fn matches(&self, card: &CardInstance, source: &CardInstance) -> bool {
        if self.creatures_only && !card.is_creature() {
            return false;
        }
        if self.controller_only && card.controller != source.controller {
            return false;
        }
        if self.other_only && card.id == source.id {
            return false;
        }
        if let Some(ref sub) = self.subtype {
            if !card.type_line.has_subtype(sub) {
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
        true
    }
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

    // Parse "|"-separated "Key$ Value" pairs.
    let mut params: BTreeMap<String, String> = BTreeMap::new();
    for segment in body.split('|') {
        let seg = segment.trim();
        if let Some(idx) = seg.find("$ ") {
            let key = seg[..idx].trim().to_string();
            let val = seg[idx + 2..].trim().to_string();
            params.insert(key, val);
        }
    }

    let mode = match params.get("Mode").map(String::as_str) {
        Some("Continuous") => StaticMode::Continuous,
        Some("CantAttack") => StaticMode::CantAttack,
        Some("CantBlock") => StaticMode::CantBlock,
        Some("ETBTapped") => StaticMode::ETBTapped,
        Some("CantBeCast") => StaticMode::CantBeCast,
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
        Some("CantGainLife") => StaticMode::Other("CantGainLife".to_string()),
        Some("CantLoseLife") => StaticMode::Other("CantLoseLife".to_string()),
        Some("CantChangeLife") => StaticMode::Other("CantChangeLife".to_string()),
        Some("CantPayLife") => StaticMode::Other("CantPayLife".to_string()),
        Some(other) => StaticMode::Other(other.to_string()),
        None => return None,
    };

    Some(StaticAbility { mode, params })
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use forge_foundation::{CardTypeLine, ColorSet, ManaCost};

    use crate::card::CardInstance;
    use crate::ids::{CardId, PlayerId};

    fn make_creature(id: u32, owner: u32, subtypes: &[&str]) -> CardInstance {
        let type_str = if subtypes.is_empty() {
            "Creature".to_string()
        } else {
            format!("Creature - {}", subtypes.join(" "))
        };
        CardInstance::new(
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

    fn make_land(id: u32, owner: u32) -> CardInstance {
        CardInstance::new(
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
        assert_eq!(sa.params["AddPower"], "1");
        assert_eq!(sa.params["AddToughness"], "1");
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
        assert_eq!(sa.params["AddKeyword"], "Flying");
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

    fn make_white_creature(id: u32, owner: u32) -> CardInstance {
        CardInstance::new(
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
        let colorless = CardInstance::new(
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
