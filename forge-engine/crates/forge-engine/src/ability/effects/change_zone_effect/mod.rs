//! ChangeZone effect — moves cards between zones.
//!
//! Ported from Java's `ChangeZoneEffect.java`. Sub-modules:
//! - [`helpers`] — matching, pre/post move, destination resolution, search restrictions
//! - [`known`] — known-origin resolve (Battlefield, Graveyard, targeted cards)
//! - [`hidden`] — hidden-origin resolve (Library/Hand searches)
//! - [`search`] — search sub-routines (single, multi, each, random, player choice)
//! - [`stack`] — stack removal (bouncing/exiling spells)
//! - [`move_cards`] — shared move + post-processing logic

pub(super) mod helpers;
mod hidden;
mod known;
pub(super) mod move_cards;
pub(super) mod search;
mod stack;

use forge_foundation::ZoneType;

use super::{parse_zone_type, EffectContext};
use crate::parsing::keys;
use crate::spellability::SpellAbility;

/// Struct form so this directory-module effect can participate in the
/// `SpellAbilityEffect` trait hierarchy alongside the single-file effects.
#[forge_engine_macros::spell_effect(ChangeZoneEffect)]
fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    resolve(ctx, sa);
}

/// Configure the spell ability during construction.
/// Mirrors Java `ChangeZoneEffect.buildSpellAbility` — calls
/// `adjustChangeZoneTarget` to set the target zone to the origin zone.
pub fn build_spell_ability(sa: &mut SpellAbility) {
    if let Some(origin_str) = sa.params.get(crate::parsing::keys::ORIGIN) {
        if let Some(ref mut tr) = sa.target_restrictions {
            if !tr.can_tgt_player() {
                if let Some(zone) = parse_zone_type(origin_str) {
                    tr.tgt_zone = vec![zone];
                }
            }
        }
    }
}

/// Top-level resolve dispatcher — mirrors Java's `resolve()` which splits on
/// `sa.isHidden()` into hidden-origin (library search) vs known-origin paths.
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let origin_str = sa.origin().unwrap_or("");
    let dest_zone = match parse_zone_type(sa.destination().unwrap_or("")) {
        Some(z) => z,
        None => return,
    };

    // Multi-origin: Origin$ can be comma-separated (e.g. "Library,Graveyard")
    let origins: Vec<ZoneType> = origin_str
        .split(',')
        .filter_map(|s| parse_zone_type(s.trim()))
        .collect();

    if origins.is_empty() {
        return;
    }

    let primary_origin = origins[0];

    // Java parity: sa.isHidden() && !sa.isNinjutsu() → hidden path
    if (primary_origin.is_hidden() || sa.is_hidden()) && !sa.param_is_true(keys::NINJUTSU) {
        hidden::resolve_hidden_origin(ctx, sa, primary_origin, dest_zone);
    } else {
        known::resolve_known_origin(ctx, sa, primary_origin, dest_zone);
    }
}
