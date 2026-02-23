use super::EffectContext;
use crate::ids::CardId;
use crate::spellability::SpellAbility;

/// Mirrors Java's `SetStateEffect.java`.
///
/// `DB$ SetState | Defined$ Self | Mode$ Transform`
///
/// Optionally gated by:
///   `ConditionDefined$ Remembered | ConditionPresent$ Card.Instant,Card.Sorcery | ConditionCompare$ EQ1`
///
/// If the condition passes, transforms the source DFC card to its other face.
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let mode = sa.params.get("Mode").map(String::as_str).unwrap_or("");
    if mode != "Transform" {
        return; // Only Transform mode is implemented.
    }

    let source_id = match sa.source {
        Some(id) => id,
        None => return,
    };

    // Evaluate optional condition.
    if let Some(cond_defined) = sa.params.get("ConditionDefined") {
        if cond_defined.eq_ignore_ascii_case("Remembered") {
            let cond_present = sa
                .params
                .get("ConditionPresent")
                .cloned()
                .unwrap_or_default();
            let cond_compare = sa
                .params
                .get("ConditionCompare")
                .cloned()
                .unwrap_or_default();

            let remembered: Vec<CardId> = ctx.game.card(source_id).remembered_cards.clone();
            let match_count = remembered
                .iter()
                .filter(|&&cid| matches_type_filter(ctx, cid, &cond_present))
                .count();

            if !evaluate_compare(&cond_compare, match_count) {
                return; // Condition not met.
            }
        }
    }

    // Perform the transform.
    ctx.game.card_mut(source_id).transform();

    // Fire Transformed trigger
    ctx.trigger_handler.run_trigger(
        crate::event::TriggerType::Transformed,
        crate::event::RunParams {
            card: Some(source_id),
            ..Default::default()
        },
        false,
    );

    // Re-scan active triggers so the new face's trigger list takes effect.
    ctx.trigger_handler.reset_active_triggers(ctx.game);
}

/// Check if a card matches a comma-separated type filter (OR semantics).
/// E.g. `"Card.Instant,Card.Sorcery"` → true if the card is an Instant or Sorcery.
fn matches_type_filter(ctx: &EffectContext, card_id: CardId, filter: &str) -> bool {
    if filter.is_empty() {
        return true;
    }
    for part in filter.split(',') {
        let type_name = part.trim().strip_prefix("Card.").unwrap_or(part.trim());
        let card = ctx.game.card(card_id);
        if card
            .type_line
            .core_types
            .iter()
            .any(|t| t.name().eq_ignore_ascii_case(type_name))
        {
            return true;
        }
    }
    false
}

/// Evaluate a `ConditionCompare` expression (e.g. `"EQ1"`, `"GT0"`) against a count.
fn evaluate_compare(compare: &str, count: usize) -> bool {
    if compare.len() < 3 {
        return true;
    }
    let op = &compare[..2];
    let num: usize = compare[2..].parse().unwrap_or(0);
    match op {
        "EQ" => count == num,
        "GT" => count > num,
        "GE" => count >= num,
        "LT" => count < num,
        "LE" => count <= num,
        "NE" => count != num,
        _ => true,
    }
}
