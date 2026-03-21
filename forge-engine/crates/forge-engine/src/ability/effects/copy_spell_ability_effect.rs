use super::EffectContext;
use crate::replacement::replacement_handler::{apply_replacements, ReplacementEvent};
use crate::replacement::ReplacementResult;
use crate::spellability::SpellAbility;

/// `SP$ CopySpellAbility` — copy the top spell on the stack.
///
/// Mirrors Java's `CopySpellAbilityEffect.java` (basic version).
/// Creates a clone of the topmost spell on the stack with the same targets.
/// Full retargeting support deferred.
///
/// # Card script examples
/// ```text
/// A:SP$ CopySpellAbility | Defined$ TopStack
/// A:SP$ CopySpellAbility | Defined$ TriggeredSpellAbility
/// ```
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let controller = sa.activating_player;

    // Run CopySpell replacement effects before copying.
    let mut event = ReplacementEvent::CopySpell {
        player: controller,
        count: 1,
    };
    let result = apply_replacements(ctx.game, &mut event);
    if result == ReplacementResult::Skipped || result == ReplacementResult::Replaced {
        return;
    }

    // Find the spell to copy — default: top of stack (excluding self)
    let stack_entry = {
        let stack_entries: Vec<_> = ctx.game.stack.iter().collect();
        // Find the topmost spell that isn't *this* effect's source
        stack_entries.iter().rev().find_map(|entry| {
            if Some(entry.id) != sa.params.get("StackId").and_then(|s| s.parse().ok()) {
                Some((*entry).clone())
            } else {
                None
            }
        })
    };

    let original = match stack_entry {
        Some(entry) => entry,
        None => return, // Nothing to copy
    };

    // Clone the spell ability with same targets
    let mut copy = original.spell_ability.clone();
    copy.activating_player = controller;

    // Push the copy onto the stack (it will resolve like a normal spell)
    let copy_entry = crate::spellability::StackEntry {
        id: 0, // will be assigned by push()
        spell_ability: copy,
        is_creature_spell: original.is_creature_spell,
        is_permanent_spell: original.is_permanent_spell,
        cast_from_zone: None,
        optional_trigger_decider: None,
        optional_trigger_description: None,
        optional_trigger_source_name: None,
    };

    ctx.game.stack.push(copy_entry);
}
