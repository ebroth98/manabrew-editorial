//! Card copy helpers (Java parity: `CardCopyService`).

use crate::card::Card;
use crate::ids::{CardId, PlayerId};

/// Copy a card, optionally assigning a new id/owner.
pub fn copy_card(
    copy_from: &Card,
    assign_new_id: bool,
    owner: Option<PlayerId>,
    new_id: Option<CardId>,
) -> Card {
    let mut out = copy_from.clone();
    if assign_new_id {
        if let Some(id) = new_id {
            out.id = id;
        }
    }
    if let Some(new_owner) = owner {
        out.owner = new_owner;
    }
    out
}

/// Java parity helper used by multiple copy paths.
pub fn copy_stats(
    input: &Card,
    new_owner: Option<PlayerId>,
    assign_new_id: bool,
    new_id: Option<CardId>,
) -> Card {
    copy_card(input, assign_new_id, new_owner, new_id)
}

/// Copy copiable characteristics from one card into another.
pub fn copy_copiable_characteristics(copy_from: &Card, to: &mut Card) {
    to.card_name = copy_from.card_name.clone();
    to.type_line = copy_from.type_line.clone();
    to.mana_cost = copy_from.mana_cost.clone();
    to.color = copy_from.color;
    to.base_power = copy_from.base_power;
    to.base_toughness = copy_from.base_toughness;
    to.keywords = copy_from.keywords.clone();
    to.abilities = copy_from.abilities.clone();
    to.triggers = copy_from.triggers.clone();
    to.svars = copy_from.svars.clone();
}
