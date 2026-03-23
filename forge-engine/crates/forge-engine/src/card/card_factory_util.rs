//! Partial parity module for Java `CardFactoryUtil`.

use std::collections::HashSet;

use crate::card::Card;
use crate::replacement::ReplacementEffect;
use crate::spellability::SpellAbility;
use crate::staticability::StaticAbility;
use crate::trigger::Trigger;

use super::{KEYWORD_ALT_COST_GAINLIFE_PREFIX, KEYWORD_ALT_COST_SACRIFICE_PREFIX};

/// Parse `Mode$ AlternativeCost | Cost$ Sac<N/Type>` from a static ability raw string
/// and return `Some("AltCostSacrifice:N:Type")` keyword.
pub(crate) fn parse_sacrifice_alt_cost_keyword(raw: &str) -> Option<String> {
    if !raw.contains("AlternativeCost") {
        return None;
    }
    raw.split('|').find_map(|part| {
        let p = part.trim();
        if let Some(rest) = p.strip_prefix("Cost$") {
            let cost = rest.trim();
            if let Some(inner) = cost
                .strip_prefix("Sac<")
                .and_then(|s| s.strip_suffix('>'))
            {
                let mut split = inner.splitn(2, '/');
                let amount = split.next().and_then(|s| s.trim().parse::<i32>().ok())?;
                let type_filter = split.next().unwrap_or("").trim().to_string();
                return Some(format!(
                    "{}{}:{}",
                    KEYWORD_ALT_COST_SACRIFICE_PREFIX, amount, type_filter
                ));
            }
        }
        None
    })
}

/// Parse `Mode$ AlternativeCost | Cost$ GainLife<N/...> | IsPresent$ ...` from a
/// static ability raw string and return `Some("AltCostGainLife:N:condition")` keyword.
pub(crate) fn parse_gainlife_alt_cost_keyword(raw: &str) -> Option<String> {
    if !raw.contains("AlternativeCost") {
        return None;
    }
    let life_amount = raw.split('|').find_map(|part| {
        let p = part.trim();
        if let Some(rest) = p.strip_prefix("Cost$") {
            let cost = rest.trim();
            if let Some(inner) = cost
                .strip_prefix("GainLife<")
                .and_then(|s| s.split('>').next())
            {
                let n = inner
                    .split('/')
                    .next()
                    .and_then(|s| s.trim().parse::<i32>().ok())?;
                return Some(n);
            }
        }
        None
    })?;
    let condition = raw
        .split('|')
        .find_map(|part| {
            let p = part.trim();
            p.strip_prefix("IsPresent$").map(|s| s.trim().to_string())
        })
        .unwrap_or_default();
    Some(format!(
        "{}{}:{}",
        KEYWORD_ALT_COST_GAINLIFE_PREFIX, life_amount, condition
    ))
}

pub fn ability_cast_face_down(card: &Card, _intrinsic: bool, key: &str) -> SpellAbility {
    SpellAbility::new_simple(Some(card.id), card.controller, &format!("FaceDown:{key}"))
}

pub fn resolve(sa: &SpellAbility, card: &mut Card) {
    if let Some(raw) = sa.params.get("AddKeyword") {
        for kw in raw.split('&').map(str::trim).filter(|s| !s.is_empty()) {
            card.add_intrinsic_keyword(kw);
        }
    }
    if let Some(raw) = sa.params.get("AddType") {
        for ty in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            card.add_type(ty);
        }
    }
}

pub fn can_play(sa: &SpellAbility, card: &Card) -> bool {
    sa.source == Some(card.id) && sa.activating_player == card.controller
}

pub fn ability_unlock_room(card: &Card) -> SpellAbility {
    SpellAbility::new_simple(Some(card.id), card.controller, "UnlockRoom")
}

pub fn ability_morph_up(card: &Card, cost_str: &str, mega: bool, _intrinsic: bool) -> SpellAbility {
    SpellAbility::new_simple(
        Some(card.id),
        card.controller,
        &format!("MorphUp:{cost_str}:{mega}"),
    )
}

pub fn ability_disguise_up(card: &Card, cost_str: &str, _intrinsic: bool) -> SpellAbility {
    SpellAbility::new_simple(
        Some(card.id),
        card.controller,
        &format!("DisguiseUp:{cost_str}"),
    )
}

pub fn ability_turn_face_up(card: &Card, key: &str, desc: &str) -> SpellAbility {
    SpellAbility::new_simple(
        Some(card.id),
        card.controller,
        &format!("TurnFaceUp:{key}:{desc}"),
    )
}

pub fn handle_hidden_agenda(_player: crate::ids::PlayerId, _card: &mut Card) -> bool {
    false
}

pub fn extract_operators(expression: &str) -> String {
    expression
        .chars()
        .filter(|c| matches!(c, '+' | '-' | '*' | '/' | '<' | '>' | '=' | '!'))
        .collect()
}

pub fn sort_colors_from_list(list: &[Card]) -> [i32; 5] {
    let mut out = [0; 5];
    for c in list {
        if c.color.has_white() {
            out[0] += 1;
        }
        if c.color.has_blue() {
            out[1] += 1;
        }
        if c.color.has_black() {
            out[2] += 1;
        }
        if c.color.has_red() {
            out[3] += 1;
        }
        if c.color.has_green() {
            out[4] += 1;
        }
    }
    out
}

pub fn shared_keywords(
    keywords: impl IntoIterator<Item = String>,
    restrictions: &[String],
) -> Vec<String> {
    let restrictions_lc: HashSet<String> =
        restrictions.iter().map(|s| s.to_ascii_lowercase()).collect();
    keywords
        .into_iter()
        .filter(|kw| {
            if restrictions_lc.is_empty() {
                return true;
            }
            restrictions_lc
                .iter()
                .any(|r| kw.to_ascii_lowercase().contains(r))
        })
        .collect()
}

pub fn add_ability_factory_abilities(card: &mut Card, abilities: &[String]) {
    for raw in abilities {
        let sa = crate::spellability::build_spell_ability_from_host_card(card, raw, card.controller);
        card.add_spell_ability(&sa);
    }
}

pub fn setup_keyworded_abilities(card: &mut Card) {
    card.generate_keyword_abilities();
    card.generate_keyword_triggers();
}

pub fn make_etb_counter(_kw: &str, _card: &Card, _intrinsic: bool) -> Option<ReplacementEffect> {
    None
}

pub fn add_trigger_ability(card: &mut Card, trig: Trigger) {
    card.add_trigger(trig);
}

pub fn add_replacement_effect(card: &mut Card, re: ReplacementEffect) {
    card.add_replacement_effect(re);
}

pub fn add_spell_ability(card: &mut Card, sa: &SpellAbility) {
    card.add_spell_ability(sa);
}

pub fn add_static_ability(card: &mut Card, st: StaticAbility) {
    card.add_static_ability(st);
}

pub fn setup_siege_abilities(card: &mut Card) {
    card.update_triggers();
}

pub fn setup_adventure_ability(_card: &mut Card) -> Option<ReplacementEffect> {
    None
}

pub fn setup_omen_ability(_card: &mut Card) -> Option<ReplacementEffect> {
    None
}

pub fn run() {
    let _ = extract_operators("X+Y");
}
