use super::EffectContext;
use crate::spellability::SpellAbility;

/// `SP$ NameCard` — the activating player names a card.
/// Stores the result in `source.named_cards` for subsequent effects.
///
/// Mirrors Java's `NameCardEffect.java`.
/// - `ChooseFromList$` — comma-separated list of card names to choose from.
/// - `ChooseFromDefinedCards$` — use remembered/defined cards as choices.
///
/// # Card script examples
/// ```text
/// A:SP$ NameCard
/// A:SP$ NameCard | ChooseFromList$ Lightning Bolt,Counterspell,Dark Ritual
/// ```
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let controller = sa.activating_player;

    // Build the valid names list
    let valid_names: Vec<String> = if let Some(list) = sa.params.get(crate::parsing::keys::CHOOSE_FROM_LIST) {
        list.split(',').map(|s| s.trim().to_string()).collect()
    } else if sa.params.has(crate::parsing::keys::CHOOSE_FROM_DEFINED_CARDS) {
        // Use remembered cards from source
        if let Some(source_id) = sa.source {
            ctx.game
                .card(source_id)
                .remembered_cards
                .iter()
                .map(|&cid| ctx.game.card(cid).card_name.clone())
                .collect()
        } else {
            vec![]
        }
    } else {
        // Open naming — for now provide an empty list (frontend handles free text input)
        vec![]
    };

    let chosen = ctx.agents[controller.index()].choose_card_name(controller, &valid_names);

    if let Some(chosen_name) = chosen {
        if let Some(source_id) = sa.source {
            ctx.game.card_mut(source_id).named_cards.push(chosen_name);
        }
    }
}
