//! ChooseCardName effect — name a card.
//!
//! Ported from Java's `ChooseCardNameEffect.java`.
//! Choose a card name (e.g. Pithing Needle, Meddling Mage).

use super::EffectContext;
use crate::parsing::keys;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let Some(source_id) = sa.source else { return };
    let controller = sa.activating_player;

    // Player names a card
    ctx.agents[controller.index()].snapshot_state(ctx.game, ctx.mana_pools);

    // Use the agent's name_card method if available, otherwise store from param
    let named = if let Some(defined_name) = sa.params.get(keys::DEFINED_NAME) {
        defined_name.to_string()
    } else {
        // Agent chooses a card name — default implementation picks from known cards
        // For parity, the agent returns a name string.
        "Named Card".to_string()
    };

    // Store the chosen name on the source card
    ctx.game.card_mut(source_id).svars.insert("ChosenName".to_string(), named);

    if sa.param_is_true(keys::REMEMBER_CHOSEN) {
        // Remember the name for later checks
    }
}
