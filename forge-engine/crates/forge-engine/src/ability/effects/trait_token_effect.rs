//! TokenEffectBase — abstract base for token creation.
//!
//! Mirrors Java's `TokenEffectBase.java`.
//! Provides shared logic for creating token creatures, used by
//! `TokenEffect`, `IncubateEffect`, and similar effects.

use crate::card::Card;
use crate::ids::PlayerId;
use crate::spellability::SpellAbility;

use super::EffectContext;

/// Parsed token creation parameters.
pub struct TokenCreateParams {
    /// Number of tokens to create.
    pub amount: usize,
    /// The token script string(s).
    pub scripts: Vec<String>,
    /// The player who owns the tokens.
    pub owner: PlayerId,
}

/// Parse common token creation parameters from a spell ability.
pub fn parse_token_params(ctx: &EffectContext, sa: &SpellAbility) -> Option<TokenCreateParams> {
    let amount = super::resolve_numeric_svar(ctx.game, sa, "TokenAmount", 1).max(0) as usize;

    let scripts: Vec<String> = sa
        .token_script()
        .map(|s| s.split(',').map(|t| t.trim().to_string()).collect())
        .unwrap_or_default();

    if scripts.is_empty() {
        return None;
    }

    let owner = sa.activating_player;

    Some(TokenCreateParams {
        amount,
        scripts,
        owner,
    })
}

/// Look up a token template by script name from the template map.
pub fn get_token_template<'a>(
    templates: &'a std::collections::HashMap<String, Card>,
    script: &str,
) -> Option<&'a Card> {
    // Try exact match first, then case-insensitive
    templates.get(script).or_else(|| {
        let lower = script.to_ascii_lowercase();
        templates
            .iter()
            .find(|(k, _)| k.to_ascii_lowercase() == lower)
            .map(|(_, v)| v)
    })
}
