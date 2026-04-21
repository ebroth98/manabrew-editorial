//! ChangeText effect — modify text on a card (color words, land types, etc.)
//!
//! Ported from Java's `ChangeTextEffect.java`.
//! Change instances of one word to another in a card's text.

use super::EffectContext;
use crate::parsing::keys;
use crate::spellability::SpellAbility;

/// End-of-turn revert for text changes. Mirrors the anonymous `GameCommand.run()`
/// in Java `ChangeTextEffect` that calls `removeChangedTextColorWord` /
/// `removeChangedTextTypeWord` at end of turn.
///
/// Removes all `ChangedText_*` SVars from the given card, reverting any
/// color-word or type-word substitutions applied by `resolve`.
pub fn run(game: &mut crate::game::GameState, card_id: crate::ids::CardId) {
    let svars_to_remove: Vec<String> = game
        .card(card_id)
        .svars
        .keys()
        .filter(|k| k.starts_with("ChangedText_"))
        .cloned()
        .collect();
    for key in svars_to_remove {
        game.card_mut(card_id).svars.remove(&key);
    }
}

/// Struct form of this effect so it can participate in the
/// `SpellAbilityEffect` trait hierarchy — mirrors Java's
/// `ChangeTextEffect` class extending `SpellAbilityEffect`.
pub struct ChangeTextEffect;

impl crate::ability::spell_ability_effect::SpellAbilityEffect for ChangeTextEffect {
    fn resolve(ctx: &mut EffectContext, sa: &crate::spellability::SpellAbility) {
    let Some(source_id) = sa.source else { return };

    let original = sa
        .params
        .get(keys::ORIGINAL)
        .map(|s| s.to_string())
        .unwrap_or_default();
    let replacement = sa
        .params
        .get(keys::REPLACEMENT)
        .map(|s| s.to_string())
        .unwrap_or_default();

    if original.is_empty() || replacement.is_empty() {
        return;
    }

    // Store the text change in the card's SVars for continuous effect processing
    // Java uses Card.addChangedTextColorWord / addChangedTextTypeWord
    ctx.game
        .card_mut(source_id)
        .svars
        .insert(format!("ChangedText_{}", original), replacement);
    }
}
