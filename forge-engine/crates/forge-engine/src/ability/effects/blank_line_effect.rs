//! BlankLine — no-op formatting effect used in card scripts.

use super::EffectContext;
use crate::spellability::SpellAbility;

pub fn resolve(_ctx: &mut EffectContext, _sa: &SpellAbility) {}
