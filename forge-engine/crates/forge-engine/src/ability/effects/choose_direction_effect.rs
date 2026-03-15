use super::EffectContext;
use crate::agent::BinaryChoiceKind;
use crate::spellability::SpellAbility;

/// `SP$ ChooseDirection` — choose left or right and remember it on source.
///
/// Mirrors Java `ChooseDirectionEffect.java`.
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let controller = sa.activating_player;
    let Some(source_id) = sa.source else { return };
    let source_name = ctx.game.card(source_id).card_name.clone();
    let choose_left = ctx.agents[controller.index()].choose_binary(
        controller,
        "Choose direction",
        BinaryChoiceKind::LeftOrRight,
        None,
        Some(&source_name),
        sa.api.as_deref(),
    );
    ctx.game.card_mut(source_id).svars.insert(
        "ChosenDirection".to_string(),
        if choose_left {
            "Left".to_string()
        } else {
            "Right".to_string()
        },
    );
}
