use super::{mana_atom_from_produced, EffectContext};
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    // Mana ability resolved on stack (shouldn't normally happen, but handle gracefully)
    if let Some(produced) = sa.params.get("Produced") {
        if let Some(atom) = mana_atom_from_produced(produced) {
            let amount = super::resolve_numeric_svar(ctx.game, sa, "Amount", 1);
            if amount > 0 {
                ctx.mana_pools[sa.activating_player.index()].add(atom, amount);
            }
        }
    }
}
