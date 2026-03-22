//! Intensify — increase effect power (escalating).

use super::EffectContext;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    if let Some(sid) = sa.source {
        let current = ctx
            .game
            .card(sid)
            .svars
            .get("IntensifyCount")
            .and_then(|s| s.strip_prefix("Number$").and_then(|n| n.parse::<i32>().ok()))
            .unwrap_or(0);
        ctx.game.card_mut(sid).svars.insert(
            "IntensifyCount".to_string(),
            format!("Number${}", current + 1),
        );
    }
}
