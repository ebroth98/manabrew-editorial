//! Java-parity helpers for `CardState` behavior.
//!
//! Rust does not model `CardState` as a separate runtime object; most state
//! lives directly on `Card`. This module provides CardState-named
//! operations as thin adapters over existing `Card` logic.

use forge_foundation::{CardTypeLine, ColorSet};

use crate::ability::activated::parse_activated_ability;
use crate::card::{card_copy_service, card_property, Card};
use crate::replacement::ReplacementEffect;
use crate::spellability::SpellAbility;
use crate::staticability::StaticAbility;
use crate::trigger::Trigger;

pub fn update_types(card: &mut Card) {
    card.type_line = CardTypeLine::parse(&card.type_line.to_string());
}

pub fn update_types_for_view(card: &mut Card) {
    let _ = card.type_line.to_string();
}

pub fn add_type(card: &mut Card, ty: &str) {
    card.type_line.add_type(ty);
}

pub fn remove_type(card: &mut Card, ty: &str) {
    card.type_line
        .supertypes
        .retain(|st| !st.name().eq_ignore_ascii_case(ty));
    card.type_line
        .core_types
        .retain(|ct| !ct.name().eq_ignore_ascii_case(ty));
    card.type_line
        .subtypes
        .retain(|s| !s.eq_ignore_ascii_case(ty));
}

pub fn remove_card_types(card: &mut Card) {
    card.type_line.core_types.clear();
    card.type_line.subtypes.clear();
}

pub fn calculate_perpetual_adjusted_mana_cost(card: &mut Card) {
    card.update_mana_cost_for_view();
}

pub fn add_color(card: &mut Card, color: ColorSet) {
    card.color = card.color.union(color);
}

pub fn has_keyword(card: &Card, keyword: &str) -> bool {
    card.keywords.contains_string_ignore_case(keyword)
        || card.granted_keywords.contains_string_ignore_case(keyword)
        || card.pump_keywords.contains_string_ignore_case(keyword)
}

pub fn has_intrinsic_keyword(card: &Card, keyword: &str) -> bool {
    card.keywords.contains_string_ignore_case(keyword)
}

pub fn update_keywords_cache(card: &mut Card) {
    // Only collapse duplicates of "redundant" keywords (Flying, Trample, ...).
    // Stackable keywords like Cascade or Annihilator must keep every instance
    // because they trigger once per copy on the source.
    let mut seen = std::collections::HashSet::new();
    let keywords = card.keywords.as_string_list();
    card.keywords.clear();
    for kw in keywords {
        let parsed = crate::keyword::keyword_collection::parse_keyword_string(&kw).0;
        let stackable = !parsed.is_multiple_redundant();
        let key = kw.to_ascii_lowercase();
        if stackable || seen.insert(key) {
            card.keywords.add(&kw);
        }
    }
}

pub fn add_intrinsic_keyword(card: &mut Card, keyword: &str) -> bool {
    if keyword.trim().is_empty() {
        return false;
    }
    card.keywords.add(keyword)
}

pub fn add_intrinsic_keywords<'a>(
    card: &mut Card,
    keywords: impl IntoIterator<Item = &'a str>,
) -> bool {
    let mut changed = false;
    for kw in keywords {
        changed |= add_intrinsic_keyword(card, kw);
    }
    changed
}

pub fn remove_intrinsic_keyword(card: &mut Card, keyword: &str) -> bool {
    card.keywords.remove(keyword)
}

pub fn apply_spell_ability(
    layer: &crate::card::card_trait_changes::CardTraitChanges,
    list: Vec<SpellAbility>,
) -> Vec<SpellAbility> {
    layer.apply_spell_ability(list)
}

pub fn apply_trigger(
    layer: &crate::card::card_trait_changes::CardTraitChanges,
    list: Vec<Trigger>,
) -> Vec<Trigger> {
    layer.apply_trigger(list)
}

pub fn apply_replacement_effect(
    layer: &crate::card::card_trait_changes::CardTraitChanges,
    list: Vec<ReplacementEffect>,
) -> Vec<ReplacementEffect> {
    layer.apply_replacement_effect(list)
}

pub fn apply_static_ability(
    layer: &crate::card::card_trait_changes::CardTraitChanges,
    list: Vec<StaticAbility>,
) -> Vec<StaticAbility> {
    layer.apply_static_ability(list)
}

pub fn apply_keywords(
    layer: &crate::card::card_trait_changes::CardTraitChanges,
    mut list: crate::keyword::keyword_collection::KeywordCollection,
) -> crate::keyword::keyword_collection::KeywordCollection {
    if layer.remove_all {
        list.clear();
    }
    list
}

pub fn copy(from: &Card, to: &mut Card) {
    copy_from(from, to);
}

pub fn has_spell_ability(card: &Card, sa: &SpellAbility) -> bool {
    card.activated_abilities
        .iter()
        .any(|ab| ab.ability_text == sa.ability_text)
}

pub fn add_spell_ability(card: &mut Card, sa: &SpellAbility) -> bool {
    card.abilities.push(sa.ability_text.clone());
    if let Some(parsed) = parse_activated_ability(&sa.ability_text, card.activated_abilities.len())
    {
        card.activated_abilities.push(parsed);
        return true;
    }
    false
}

pub fn has_trigger(card: &Card, trigger_id: u32) -> bool {
    card.triggers.iter().any(|t| t.id == trigger_id)
}

pub fn add_trigger(card: &mut Card, trig: Trigger) -> bool {
    card.triggers.push(trig);
    true
}

pub fn add_static_ability(card: &mut Card, st_ab: StaticAbility) -> bool {
    card.static_abilities.push(st_ab);
    true
}

pub fn remove_static_ability(card: &mut Card, mode: crate::staticability::StaticMode) -> bool {
    let before = card.static_abilities.len();
    card.static_abilities.retain(|sa| sa.mode != mode);
    before != card.static_abilities.len()
}

pub fn add_replacement_effect(card: &mut Card, re: ReplacementEffect) -> bool {
    card.replacement_effects.push(re);
    true
}

pub fn has_replacement_effect(card: &Card) -> bool {
    !card.replacement_effects.is_empty()
}

pub fn has_s_var(card: &Card, key: &str) -> bool {
    card.svars.contains_key(key)
}

pub fn remove_s_var(card: &mut Card, key: &str) {
    card.svars.remove(key);
}

pub fn copy_from(source: &Card, target: &mut Card) {
    target.changed_card_traits = source.changed_card_traits.clone();
    target.changed_card_traits_by_text = source.changed_card_traits_by_text.clone();

    target
        .svars
        .retain(|k, _| !k.starts_with("TextColor:") && !k.starts_with("TextType:"));
    target.copy_changed_text_from(source);

    target.update_type_cache();
}

pub fn add_abilities_from(source: &Card, target: &mut Card) {
    // Reuse copy-service for copiable characteristics.
    card_copy_service::copy_copiable_characteristics(source, target);

    target
        .activated_abilities
        .extend(source.activated_abilities.iter().cloned());
    target.triggers.extend(source.triggers.iter().cloned());
    target
        .replacement_effects
        .extend(source.replacement_effects.iter().cloned());
    target
        .static_abilities
        .extend(source.static_abilities.iter().cloned());
}

pub fn has_property(card: &Card, property: &str) -> bool {
    card_property::card_has_property(card, property, card.controller)
}

pub fn reset_original_host(card: &mut Card) {
    card.effect_source = None;
}

pub fn update_changed_text(card: &mut Card) {
    card.update_rules_view();
}

pub fn change_text_intrinsic(card: &mut Card) {
    card.update_changed_text();
}

pub fn has_chapter(card: &Card) -> bool {
    card.triggers.iter().any(|t| {
        t.params.has("Chapter")
            || t.params
                .get("Mode")
                .map(|m| m.eq_ignore_ascii_case("Chapter"))
                .unwrap_or(false)
            || t.description.to_ascii_lowercase().contains("chapter")
    })
}

pub fn set_type(card: &mut Card, type_line: &str) {
    card.type_line = CardTypeLine::parse(type_line);
}
