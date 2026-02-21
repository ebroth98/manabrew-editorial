use std::collections::BTreeMap;

use super::{mana_atom_from_produced, EffectContext};
use crate::spellability::StackEntry;

pub fn resolve(
    ctx: &mut EffectContext,
    params: &BTreeMap<String, String>,
    entry: &StackEntry,
) {
    // Mana ability resolved on stack (shouldn't normally happen, but handle gracefully)
    if let Some(produced) = params.get("Produced") {
        if let Some(atom) = mana_atom_from_produced(produced) {
            ctx.mana_pools[entry.controller.index()].add(atom, 1);
        }
    }
}
