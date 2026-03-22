use forge_foundation::{CardTypeLine, ColorSet, ManaCost, ZoneType};

use super::trait_token_effect;
use super::{emit_zone_trigger, EffectContext};
use crate::card::Card;
use crate::event::{RunParams, TriggerType};
use crate::ids::CardId;
use crate::parsing::keys;
use crate::replacement::replacement_handler::{apply_replacements, ReplacementEvent};
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    // Create token creature(s) on the battlefield.
    // Mirrors Java TokenEffect / TokenEffectBase.
    let mut amount: usize =
        super::resolve_numeric_svar(ctx.game, sa, "TokenAmount", 1).max(0) as usize;
    let token_script = sa.token_script().unwrap_or("").to_string();
    let token_owner_str = sa
        .token_owner()
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

    // Run CreateToken replacement effects (e.g. Anointed Procession doubles tokens).
    let mut event = ReplacementEvent::CreateToken {
        player: token_controller,
        count: amount as i32,
    };
    apply_replacements(ctx.game, &mut event);
    if let ReplacementEvent::CreateToken {
        count: final_count, ..
    } = event
    {
        amount = final_count.max(0) as usize;
    }

    if !token_script.is_empty() {
        if let Some(template) = trait_token_effect::get_token_template(ctx.token_templates, &token_script).cloned() {
            create_tokens(ctx, sa, &template, amount, token_controller);
        } else {
            eprintln!(
                "[effects::token] Unknown token script '{}' — register it via game_loop.register_token()",
                token_script
            );
        }
    } else if has_inline_token_params(sa) {
        // Build token from inline parameters (TokenPower$, TokenToughness$, etc.)
        let template = build_inline_token(sa, token_controller);
        create_tokens(ctx, sa, &template, amount, token_controller);
    } else {
        eprintln!("[effects::token] Token effect missing TokenScript$ and inline params");
    }
}

/// Check if the SA has inline token definition params.
fn has_inline_token_params(sa: &SpellAbility) -> bool {
    sa.params.has(keys::TOKEN_POWER)
        || sa.params.has(keys::TOKEN_TOUGHNESS)
        || sa.params.has(keys::TOKEN_TYPES)
        || sa.params.has(keys::TOKEN_NAME)
}

/// Build a Card template from inline token params.
/// Mirrors Java's TokenEffectBase inline token construction.
fn build_inline_token(sa: &SpellAbility, owner: crate::ids::PlayerId) -> Card {
    let name = sa
        .params
        .get_cloned(keys::TOKEN_NAME)
        .unwrap_or_else(|| "Token".to_string());
    let power = sa
        .params
        .get(keys::TOKEN_POWER)
        .and_then(|s| s.parse::<i32>().ok());
    let toughness = sa
        .params
        .get(keys::TOKEN_TOUGHNESS)
        .and_then(|s| s.parse::<i32>().ok());
    let type_line = sa
        .params
        .get(keys::TOKEN_TYPES)
        .map(|s| CardTypeLine::parse(s))
        .unwrap_or_else(|| CardTypeLine::parse("Creature"));
    let colors = sa
        .params
        .get(keys::TOKEN_COLORS)
        .map(|s| parse_token_colors(s))
        .unwrap_or(ColorSet::COLORLESS);
    let keywords: Vec<String> = sa
        .params
        .get(keys::TOKEN_KEYWORDS)
        .map(|s| s.split('&').map(|k| k.trim().to_string()).collect())
        .unwrap_or_default();

    Card::new(
        CardId(0), // Will be reassigned by create_card
        name,
        owner,
        type_line,
        ManaCost::parse(""),
        colors,
        power,
        toughness,
        keywords,
        vec![],
    )
}

/// Parse color string for tokens (e.g. "White", "Black,Green", "Colorless").
fn parse_token_colors(s: &str) -> ColorSet {
    let mut result = ColorSet::COLORLESS;
    for part in s.split(',') {
        let c = match part.trim().to_lowercase().as_str() {
            "white" | "w" => ColorSet::WHITE,
            "blue" | "u" => ColorSet::BLUE,
            "black" | "b" => ColorSet::BLACK,
            "red" | "r" => ColorSet::RED,
            "green" | "g" => ColorSet::GREEN,
            _ => ColorSet::COLORLESS,
        };
        result = result.union(c);
    }
    result
}

/// Create N tokens from a template and put them on the battlefield.
fn create_tokens(
    ctx: &mut EffectContext,
    sa: &SpellAbility,
    template: &Card,
    amount: usize,
    token_controller: crate::ids::PlayerId,
) {
    // Java consumes 2 game-level RNG values when creating the token prototype:
    //   1. Aggregates.random() in TokenDb.getToken() — selects token art variant
    //   2. MyRandom.nextInt(artIndex) in PaperToken.getImageKey() — selects image
    // Both advance the seed once (artIndex is typically 1). Rust doesn't have
    // token art selection, but must consume the same RNG to stay in sync.
    ctx.rng.next_int(1); // match Aggregates.random() in TokenDb.getToken()
    ctx.rng.next_int(1); // match MyRandom.nextInt(artIndex) in PaperToken.getImageKey()

    for _ in 0..amount {
        let mut token = template.clone();
        token.set_owner(token_controller);
        token.set_controller(token_controller);
        token.set_is_token(true);
        let token_id = ctx.game.create_card(token);
        ctx.game
            .move_card(token_id, ZoneType::Battlefield, token_controller);
        // TokenTapped$ True: token enters the battlefield tapped.
        // Must be set AFTER move_card because enter_battlefield() resets tapped to false.
        // Mirrors Java TokenEffectBase line 131: if (sa.hasParam("TokenTapped")) tok.setTapped(true);
        if sa.is_param_true("TokenTapped") {
            ctx.game.tap(token_id);
        }
        ctx.trigger_handler
            .register_active_trigger(ctx.game, token_id);
        // Fire TokenCreated trigger
        ctx.trigger_handler.run_trigger(
            TriggerType::TokenCreated,
            RunParams {
                card: Some(token_id),
                player: Some(token_controller),
                ..Default::default()
            },
            false,
        );

        emit_zone_trigger(
            ctx.trigger_handler,
            token_id,
            ZoneType::None,
            ZoneType::Battlefield,
        );
    }
}
