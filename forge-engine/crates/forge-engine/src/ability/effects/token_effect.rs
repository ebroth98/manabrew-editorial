use forge_foundation::{CardTypeLine, ColorSet, ManaCost, ZoneType};

use super::trait_token_effect;
use super::{emit_zone_trigger, EffectContext};
use crate::card::card_zone_table::CardZoneTable;
use crate::card::Card;
use crate::event::{RunParams, TriggerType};
use crate::ids::{CardId, PlayerId};
use crate::parsing::keys;
use crate::replacement::replacement_handler::{apply_replacements_with_agents, ReplacementEvent};
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    // Create token creature(s) on the battlefield.
    // Mirrors Java TokenEffect / TokenEffectBase.
    let amount: usize = super::resolve_numeric_svar(ctx.game, sa, "TokenAmount", 1).max(0) as usize;
    let token_script = sa.token_script().unwrap_or("").to_string();
    let token_owners = resolve_token_owners(ctx, sa);
    if token_owners.is_empty() {
        return;
    }

    // Run CreateToken replacement effects (e.g. Anointed Procession doubles tokens).
    let mut all_created_tokens: Vec<CardId> = Vec::new();
    if !token_script.is_empty() {
        if let Some(template) =
            trait_token_effect::get_token_template(ctx.token_templates, &token_script).cloned()
        {
            for token_controller in token_owners {
                let final_amount = replaced_token_amount(ctx, amount, token_controller);
                // Always call create_tokens even when amount is 0 — the function
                // consumes 2 RNG values for Java token-art parity that must fire
                // regardless of count.
                let created = create_tokens(ctx, sa, &template, final_amount, token_controller);
                all_created_tokens.extend(created);
            }
        } else {
            eprintln!(
                "[effects::token] Unknown token script '{}' — register it via game_loop.register_token()",
                token_script
            );
        }
    } else if has_inline_token_params(sa) {
        // Build token from inline parameters (TokenPower$, TokenToughness$, etc.)
        for token_controller in token_owners {
            let final_amount = replaced_token_amount(ctx, amount, token_controller);
            let template = build_inline_token(sa, token_controller);
            let created = create_tokens(ctx, sa, &template, final_amount, token_controller);
            all_created_tokens.extend(created);
        }
    } else {
        eprintln!("[effects::token] Token effect missing TokenScript$ and inline params");
    }

    // Fire ChangesZoneAll for the batch of tokens entering the battlefield.
    // Mirrors Java's CardZoneTable.triggerChangesZoneAll() at the end of
    // TokenEffect.  This is needed for triggers like Woodland Champion
    // (Mode$ ChangesZoneAll | Destination$ Battlefield).
    if !all_created_tokens.is_empty() {
        let mut table = CardZoneTable::default();
        for &tid in &all_created_tokens {
            table.put(Some(ZoneType::None), Some(ZoneType::Battlefield), tid);
        }
        table.trigger_changes_zone_all(ctx.trigger_handler, ctx.game, Some(sa));
    }
}

fn resolve_token_owners(ctx: &EffectContext, sa: &SpellAbility) -> Vec<PlayerId> {
    if let Some(defined) = sa.token_owner() {
        let players = crate::ability::ability_utils::resolve_defined_players_with_sa(
            defined,
            sa,
            sa.activating_player,
            ctx.game,
        );
        if !players.is_empty() {
            return players;
        }
    }
    vec![sa.activating_player]
}

fn replaced_token_amount(
    ctx: &mut EffectContext,
    amount: usize,
    token_controller: PlayerId,
) -> usize {
    let mut event = ReplacementEvent::CreateToken {
        player: token_controller,
        count: amount as i32,
        is_effect: true,
    };
    apply_replacements_with_agents(&mut *ctx.game, ctx.agents, &mut event);
    if let ReplacementEvent::CreateToken {
        count: final_count, ..
    } = event
    {
        final_count.max(0) as usize
    } else {
        amount
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
) -> Vec<CardId> {
    // Sync game RNG with Java's token art selection (Aggregates.random + getImageKey).
    let script_key = sa.token_script().unwrap_or("").to_string();
    ctx.sync_token_art_rng(&script_key, sa);

    let mut created = Vec::with_capacity(amount);
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
        apply_token_attacking_marker(ctx, sa, token_id);
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
        created.push(token_id);
    }
    created
}

fn apply_token_attacking_marker(ctx: &mut EffectContext, sa: &SpellAbility, token_id: CardId) {
    let _ = super::add_to_combat(ctx, sa, token_id, "TokenAttacking");
}
