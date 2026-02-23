use forge_foundation::ZoneType;

use super::EffectContext;
use crate::spellability::SpellAbility;

/// SP$ ControlGain — gain control of target permanent until end of turn or permanently.
///
/// Mirrors Java's `ControlGainEffect.resolve()`.
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let target_card = match sa.target_chosen.target_card {
        Some(c) => c,
        None => return,
    };

    // Verify target is still on the battlefield
    if ctx.game.card(target_card).zone != ZoneType::Battlefield {
        return;
    }

    let new_controller = sa.activating_player;

    // Check if the card can be controlled by the new controller
    if !ctx
        .game
        .card(target_card)
        .can_be_controlled_by(new_controller)
    {
        return;
    }

    // Change controller
    ctx.game.change_controller(target_card, new_controller);

    // Handle Untap parameter
    if sa.params.contains_key("Untap") {
        ctx.game.untap(target_card);
    }

    // Handle AddKWs parameter (add keywords)
    if let Some(kws_str) = sa.params.get("AddKWs") {
        let keywords: Vec<String> = kws_str.split(" & ").map(|s| s.to_string()).collect();
        for kw in keywords {
            ctx.game.card_mut(target_card).granted_keywords.push(kw);
        }
    }
}
