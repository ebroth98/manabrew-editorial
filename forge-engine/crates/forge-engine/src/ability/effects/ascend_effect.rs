//! Ascend effect — gain the City's Blessing if you control 10+ permanents.
//!
//! Ported 1:1 from Java's `AscendEffect.java`.
//! Ascend: If you control ten or more permanents, you get the city's blessing
//! for the rest of the game.

use forge_foundation::ZoneType;

use super::EffectContext;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let controller = sa.activating_player;
    let players = if let Some(def) = sa.defined_player() {
        super::resolve_defined_players(def, controller, ctx.game)
    } else {
        vec![controller]
    };

    for pid in players {
        if ctx.game.player(pid).has_lost {
            continue;
        }

        // Count permanents on battlefield
        let permanent_count = ctx.game.cards_in_zone(ZoneType::Battlefield, pid).len();

        if permanent_count >= 10 {
            // Grant city's blessing (permanent for rest of game)
            // In Java this is Player.setBlessing(true). We track via a flag.
            // The blessing is checked by card scripts via "Player.hasCityBlessing"
            ctx.game.player_mut(pid).has_city_blessing = true;
        }
    }
}
