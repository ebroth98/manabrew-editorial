//! Card assembly pipeline — separates parsing from behavior rewrites.
//!
//! Splits the old monolithic `Card::from_rules` into three phases:
//!
//! 1. **`parse_card_components`** — pure parsing, no side effects
//! 2. **`synthesize_derived`** — behavior rewrites (SpellCastOrCopy duplication,
//!    AlternativeCost keyword extraction, Exert trigger synthesis)
//! 3. **`assemble_card`** — combines components into a `Card`
//!
//! This mirrors Java's separation between `AbilityFactory` (parsing),
//! `TriggerHandler` (registration), and `Card` (construction).

use forge_carddb::CardRules;

use crate::ids::{CardId, PlayerId};
use crate::parsing::{keys, parse_or_warn};
use crate::replacement::{parse_replacement_effect, ReplacementEffect};
use crate::staticability::{parse_static_ability, StaticAbility};
use crate::trigger::{parse_trigger, Trigger};

use super::card_factory_util::{
    parse_gainlife_alt_cost_keyword, parse_sacrifice_alt_cost_keyword,
};
use super::{Card, CardOtherPart};

// ── Phase 1: Parse ──────────────────────────────────────────────────────────

/// Parsed components from a card face's raw text, before any rewrites.
pub(crate) struct ParsedComponents {
    pub triggers: Vec<Trigger>,
    pub static_abilities: Vec<StaticAbility>,
    pub replacement_effects: Vec<ReplacementEffect>,
    /// Raw trigger lines that contained `Mode$ SpellCastOrCopy`,
    /// needed for Magecraft duplication in Phase 2.
    pub spell_cast_or_copy_raw: Vec<String>,
    /// Synthetic keywords extracted from AlternativeCost static abilities.
    pub alt_cost_keywords: Vec<String>,
}

/// Phase 1: Parse triggers, static abilities, and replacement effects from
/// a card face's raw text lines.
pub(crate) fn parse_card_components(
    face: &forge_carddb::CardFace,
) -> ParsedComponents {
    let mut next_trigger_id = 0u32;
    let mut triggers = Vec::new();
    let mut spell_cast_or_copy_raw = Vec::new();

    for raw in &face.triggers {
        if let Some(trig) = parse_trigger(raw, &mut next_trigger_id) {
            triggers.push(trig);
            if raw.contains("Mode$ SpellCastOrCopy") {
                spell_cast_or_copy_raw.push(raw.clone());
            }
        }
    }

    // Parse static abilities from S: lines (need S$ prefix for the parser)
    let mut static_abilities = Vec::new();
    let mut alt_cost_keywords = Vec::new();
    for raw in &face.static_abilities {
        if let Some(kw) = parse_gainlife_alt_cost_keyword(raw) {
            alt_cost_keywords.push(kw);
        }
        if let Some(kw) = parse_sacrifice_alt_cost_keyword(raw) {
            alt_cost_keywords.push(kw);
        }
        let prefixed = format!("S$ {}", raw);
        if let Some(sa) = parse_static_ability(&prefixed) {
            static_abilities.push(sa);
        }
    }

    // Parse replacement effects from R: lines (need R$ prefix for the parser)
    let replacement_effects: Vec<ReplacementEffect> = face
        .replacements
        .iter()
        .filter_map(|raw| {
            let prefixed = format!("R$ {}", raw);
            parse_or_warn(parse_replacement_effect(&prefixed), "ReplacementEffect", raw)
        })
        .collect();

    ParsedComponents {
        triggers,
        static_abilities,
        replacement_effects,
        spell_cast_or_copy_raw,
        alt_cost_keywords,
    }
}

// ── Phase 2: Synthesize ─────────────────────────────────────────────────────

/// Phase 2: Apply behavior rewrites that derive new triggers/keywords from
/// parsed components.
///
/// - Duplicate `SpellCastOrCopy` triggers as `SpellCopied` (Magecraft)
/// - Synthesize `Exerted` triggers from `OptionalAttackCost` with Exert cost
pub(crate) fn synthesize_derived(
    components: &mut ParsedComponents,
    existing_trigger_count: usize,
) {
    let mut next_trig_id = (existing_trigger_count + components.triggers.len()) as u32;

    // Duplicate SpellCastOrCopy triggers as SpellCopied (for Magecraft)
    for raw in &components.spell_cast_or_copy_raw {
        let converted = raw.replace("Mode$ SpellCastOrCopy", "Mode$ SpellCopied");
        if let Some(trig) = parse_trigger(&converted, &mut next_trig_id) {
            components.triggers.push(trig);
        }
    }

    // OptionalAttackCost with Exert + Trigger$: register an Exerted trigger
    for sa in &components.static_abilities {
        if sa.mode != crate::staticability::StaticMode::OptionalAttackCost {
            continue;
        }
        let has_exert = sa
            .params
            .get(keys::COST)
            .map(|c| c.contains("Exert"))
            .unwrap_or(false);
        if has_exert {
            if let Some(svar_name) = sa.params.get(keys::TRIGGER) {
                let raw = format!(
                    "Mode$ Exerted | ValidCard$ Card.Self | Execute$ {} | TriggerZones$ Battlefield",
                    svar_name
                );
                if let Some(mut trig) = parse_trigger(&raw, &mut next_trig_id) {
                    trig.execute = svar_name.to_string();
                    components.triggers.push(trig);
                }
            }
        }
    }
}

// ── Phase 3: Assemble ───────────────────────────────────────────────────────

/// Phase 3: Combine parsed + synthesized components into a `Card`.
pub(crate) fn assemble_card(
    rules: &CardRules,
    owner: PlayerId,
    components: ParsedComponents,
) -> Card {
    let face = &rules.main_part;

    let mut card = Card::new(
        CardId(0),
        face.name.clone(),
        owner,
        face.type_line.clone(),
        face.mana_cost.clone(),
        face.resolved_color(),
        face.int_power,
        face.int_toughness,
        face.keywords.clone(),
        face.abilities.clone(),
    );

    // Append parsed triggers to keyword-generated ones.
    for trig in components.triggers {
        card.add_trigger(trig);
    }

    // Merge card-text SVars (keyword-generated SVars already set by constructor)
    for (k, v) in &face.svars {
        card.svars.entry(k.clone()).or_insert_with(|| v.clone());
    }

    // Add parsed static abilities and replacement effects.
    for sa in components.static_abilities {
        card.add_static_ability(sa);
    }
    for re in components.replacement_effects {
        card.add_replacement_effect(re);
    }

    // Add synthetic alt-cost keywords.
    for kw in components.alt_cost_keywords {
        card.add_intrinsic_keyword(&kw);
    }

    // Double-faced cards
    if rules.split_type.is_dual_faced() {
        if let Some(ref back_face) = rules.other_part {
            let mut back_trigger_id = 0u32;
            let back_triggers: Vec<_> = back_face
                .triggers
                .iter()
                .filter_map(|raw| {
                    parse_or_warn(parse_trigger(raw, &mut back_trigger_id), "Trigger", raw)
                })
                .collect();

            card.other_part = Some(CardOtherPart {
                name: back_face.name.clone(),
                type_line: back_face.type_line.clone(),
                mana_cost: back_face.mana_cost.clone(),
                color: back_face.resolved_color(),
                base_power: back_face.int_power,
                base_toughness: back_face.int_toughness,
                keywords: crate::keyword::keyword_collection::KeywordCollection::from_strings(&back_face.keywords),
                abilities: back_face.abilities.clone(),
                triggers: back_triggers,
                svars: back_face.svars.clone(),
            });
        }
    }

    card
}
