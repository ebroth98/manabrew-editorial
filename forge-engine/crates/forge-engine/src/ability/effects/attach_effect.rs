use forge_foundation::ZoneType;

use super::EffectContext;
use crate::spellability::SpellAbility;

/// SP$ Attach / AB$ Attach — attach source Equipment/Aura to target creature.
///
/// Mirrors Java's `AttachEffect.resolve()`.
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let target = match sa.target_chosen.target_card {
        Some(c) => c,
        None => return,
    };

    // Source is the card being attached (the Equipment or Aura)
    let aura_id = match sa.source {
        Some(s) => s,
        None => return,
    };

    // Both must be on the battlefield
    if ctx.game.card(aura_id).zone != ZoneType::Battlefield
        || ctx.game.card(target).zone != ZoneType::Battlefield
    {
        return;
    }
    if crate::staticability::static_ability_cant_attach::cant_attach(
        &ctx.game.cards,
        ctx.game.card(aura_id),
        ctx.game.card(target),
        false,
    ) {
        return;
    }

    ctx.game.attach_to(aura_id, target);

    // Fire Attached trigger
    ctx.trigger_handler.run_trigger(
        crate::event::TriggerType::Attached,
        crate::event::RunParams {
            card: Some(aura_id),
            ..Default::default()
        },
        false,
    );
}
