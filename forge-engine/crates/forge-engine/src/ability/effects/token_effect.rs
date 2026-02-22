use forge_foundation::ZoneType;

use super::{emit_zone_trigger, EffectContext};
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    // Create token creature(s) on the battlefield.
    // Mirrors Java TokenEffect / TokenEffectBase.
    let amount: usize = sa
        .params
        .get("TokenAmount")
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    let token_script = sa.params.get("TokenScript").cloned().unwrap_or_default();
    let token_owner_str = sa
        .params
        .get("TokenOwner")
        .map(|s| s.to_lowercase())
        .unwrap_or_else(|| "you".to_string());

    let token_controller = if token_owner_str.contains("opponent") {
        ctx.game
            .player_order
            .iter()
            .find(|&&p| p != sa.activating_player)
            .copied()
            .unwrap_or(sa.activating_player)
    } else {
        sa.activating_player
    };

    if token_script.is_empty() {
        eprintln!("[effects::token] Token effect missing TokenScript$ param");
    } else if let Some(template) = ctx.token_templates.get(&token_script).cloned() {
        for _ in 0..amount {
            let mut token = template.clone();
            token.owner = token_controller;
            token.controller = token_controller;
            token.is_token = true;
            let token_id = ctx.game.create_card(token);
            ctx.game
                .move_card(token_id, ZoneType::Battlefield, token_controller);
            ctx.trigger_handler
                .register_active_trigger(ctx.game, token_id);
            emit_zone_trigger(
                ctx.trigger_handler,
                token_id,
                ZoneType::None,
                ZoneType::Battlefield,
            );
        }
    } else {
        eprintln!(
            "[effects::token] Unknown token script '{}' — register it via game_loop.register_token()",
            token_script
        );
    }
}
