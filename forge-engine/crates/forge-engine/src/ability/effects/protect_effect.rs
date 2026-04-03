use forge_foundation::ZoneType;

use super::EffectContext;
use crate::card::card_util;
use crate::spellability::SpellAbility;

/// End-of-turn revert for Protection. Mirrors the `GameCommand.run()` in Java
/// `ProtectEffect` that removes the granted protection keyword when the
/// effect duration expires.
pub fn run(
    game: &mut crate::game::GameState,
    card_id: crate::ids::CardId,
    keyword: &str,
) {
    if game.card(card_id).zone == ZoneType::Battlefield {
        game.card_mut(card_id).pump_keywords.remove(keyword);
    }
}

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
        match sa.params.get(crate::parsing::keys::DEFINED) {
            Some("Self") => sa.source,
            Some("ParentTarget") => ctx.parent_target_card,
            _ => sa.source,
        }
    });

    let gains = sa
        .params
        .get(crate::parsing::keys::GAINS)
        .unwrap_or("")
        .to_string();

    // Mirrors Java ProtectEffect: `isChoice = sa.getParam("Gains").contains("Choice")`
    // Handles both `Gains$ Choice` (Gods Willing) and `Gains$ Protection from chosen color`.
    let is_choice = gains.contains("Choice") || gains.contains("chosen color");

    let mut targets = target.into_iter().collect::<Vec<_>>();
    targets.extend(card_util::get_radiance(ctx.game, sa).iter().copied());
    targets.retain(|&id| ctx.game.card(id).zone == ZoneType::Battlefield);
    targets.sort_unstable_by_key(|cid| cid.0);
    targets.dedup();
    if targets.is_empty() {
        return;
    }

    if is_choice {
        // Build the protection choices from `Choices$` parameter.
        // `AnyColor` expands to the 5 Magic colors.
        let choices = sa
            .params
            .get(crate::parsing::keys::CHOICES)
            .map(|s| {
                if s.contains("AnyColor") {
                    vec![
                        "White".into(),
                        "Blue".into(),
                        "Black".into(),
                        "Red".into(),
                        "Green".into(),
                    ]
                } else {
                    s.split(',')
                        .map(|c| c.trim().to_string())
                        .collect::<Vec<_>>()
                }
            })
            .unwrap_or_else(|| {
                vec![
                    "White".into(),
                    "Blue".into(),
                    "Black".into(),
                    "Red".into(),
                    "Green".into(),
                ]
            });

        let chosen = ctx.agents[controller.index()].choose_color(controller, &choices);
        if let Some(color) = chosen {
            let prot_kw = format!("Protection from {}", color.to_lowercase());
            for card_id in targets {
                ctx.game.card_mut(card_id).add_pump_keyword(&prot_kw);
            }
        }
    } else {
        // Static protection grant
        for card_id in targets {
            ctx.game.card_mut(card_id).add_pump_keyword(&gains);
        }
    }
}
