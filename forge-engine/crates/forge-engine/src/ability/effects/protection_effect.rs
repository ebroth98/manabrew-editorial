use forge_foundation::ZoneType;

use super::EffectContext;
use crate::spellability::SpellAbility;

/// `SP$ Protection` — grant protection from a quality to a permanent.
///
/// Mirrors Java's `ProtectEffect.java`.
/// - `Gains$` — the protection keyword to grant (e.g. "Protection from chosen color").
/// - `Choices$` — if present, player chooses what to protect from.
///
/// # Card script examples
/// ```text
/// A:SP$ Protection | Gains$ Protection from chosen color | Choices$ White,Blue,Black,Red,Green
/// A:SP$ Protection | Gains$ Protection from red
/// ```
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let controller = sa.activating_player;

    // Determine target
    let target = sa.target_chosen.target_card.or_else(|| {
        match sa.params.get("Defined").map(|s| s.as_str()) {
            Some("Self") => sa.source,
            Some("ParentTarget") => ctx.parent_target_card,
            _ => sa.source,
        }
    });

    let card_id = match target {
        Some(id) if ctx.game.card(id).zone == ZoneType::Battlefield => id,
        _ => return,
    };

    let gains = sa.params.get("Gains").cloned().unwrap_or_default();

    // Check if we need to choose a color/type
    if gains.contains("chosen color") {
        let choices = sa.params.get("Choices")
            .map(|s| s.split(',').map(|c| c.trim().to_string()).collect::<Vec<_>>())
            .unwrap_or_else(|| vec![
                "White".into(), "Blue".into(), "Black".into(), "Red".into(), "Green".into(),
            ]);

        let chosen = ctx.agents[controller.index()].choose_color(controller, &choices);
        if let Some(color) = chosen {
            let prot_kw = format!("Protection from {}", color.to_lowercase());
            ctx.game.card_mut(card_id).pump_keywords.push(prot_kw);
        }
    } else {
        // Static protection grant
        ctx.game.card_mut(card_id).pump_keywords.push(gains);
    }
}
