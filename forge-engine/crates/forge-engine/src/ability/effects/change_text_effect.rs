//! ChangeText effect — modify text on a card (color words, land types, etc.)
//!
//! Ported from Java's `ChangeTextEffect.java`.
//! Change instances of one word to another in a card's text.

use super::EffectContext;
use crate::parsing::keys;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
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
