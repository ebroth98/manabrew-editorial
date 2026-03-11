use forge_foundation::mana::ManaAtom;
use forge_foundation::ZoneType;

use super::{mana_atom_from_produced, EffectContext};
use crate::mana::{color_name_to_mana_atom, Mana};
use crate::spellability::SpellAbility;

/// Resolve DB$ ManaReflected — produce mana of a color/type that reflects other cards.
/// Mirrors Java's ManaReflectedEffect.java.
///
/// Key params:
/// - ReflectProperty$: "Is" (card colors), "Produce" (mana abilities), "Produced" (trigger mana)
/// - ColorOrType$: "Color" (5 colors) or "Type" (6 = 5 colors + colorless)
/// - Valid$: filter for which cards to check
/// - Amount$: how many mana to produce (default 1)
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let player = sa.activating_player;
    let source_id = match sa.source {
        Some(id) => id,
        None => return,
    };

    let reflect_property = sa
        .params
        .get("ReflectProperty")
        .cloned()
        .unwrap_or_else(|| "Is".to_string());
    let color_or_type = sa
        .params
        .get("ColorOrType")
        .cloned()
        .unwrap_or_else(|| "Color".to_string());
    let valid = sa
        .params
        .get("Valid")
        .cloned()
        .unwrap_or_else(|| "Card".to_string());

    let amount = super::resolve_numeric_svar(ctx.game, sa, "Amount", 1);
    if amount <= 0 {
        return;
    }

    // Determine available colors based on ReflectProperty
    let available_colors = match reflect_property.as_str() {
        "Is" => get_colors_from_cards(ctx, source_id, &valid, &color_or_type),
        "Produce" => get_colors_from_mana_abilities(ctx, source_id, &valid, &color_or_type),
        "Produced" => get_colors_from_produced(sa, &color_or_type),
        _ => get_colors_from_cards(ctx, source_id, &valid, &color_or_type),
    };

    if available_colors.is_empty() {
        return;
    }

    // Read metadata from the ability
    let restriction = sa.params.get("RestrictValid").cloned();
    let source_is_snow = ctx.game.card(source_id).type_line.is_snow();

    // Sort available colors in WUBRG(C) order to match Java's ColorSet iteration.
    let wubrg_order: &[u16] = &[
        ManaAtom::WHITE,
        ManaAtom::BLUE,
        ManaAtom::BLACK,
        ManaAtom::RED,
        ManaAtom::GREEN,
        ManaAtom::COLORLESS,
    ];
    let mut sorted_colors: Vec<u16> = Vec::new();
    for &atom in wubrg_order {
        if available_colors.contains(&atom) {
            sorted_colors.push(atom);
        }
    }
    if sorted_colors.is_empty() {
        sorted_colors = available_colors;
    }

    // Convert to color names and let the agent choose (mirrors Java's chooseColor).
    let color_names: Vec<String> = sorted_colors
        .iter()
        .map(|&atom| match atom {
            ManaAtom::WHITE => "White".to_string(),
            ManaAtom::BLUE => "Blue".to_string(),
            ManaAtom::BLACK => "Black".to_string(),
            ManaAtom::RED => "Red".to_string(),
            ManaAtom::GREEN => "Green".to_string(),
            ManaAtom::COLORLESS => "Colorless".to_string(),
            _ => "Colorless".to_string(),
        })
        .collect();

    let best_color = if let Some(chosen_name) =
        ctx.agents[player.index()].choose_color(player, &color_names)
    {
        match chosen_name.as_str() {
            "White" => ManaAtom::WHITE,
            "Blue" => ManaAtom::BLUE,
            "Black" => ManaAtom::BLACK,
            "Red" => ManaAtom::RED,
            "Green" => ManaAtom::GREEN,
            "Colorless" => ManaAtom::COLORLESS,
            _ => sorted_colors[0],
        }
    } else {
        sorted_colors[0]
    };

    // Produce `amount` mana of the chosen color
    for _ in 0..amount {
        let mut m = Mana::simple(best_color);
        m.source_card = Some(source_id);
        m.is_snow = source_is_snow;
        m.restriction = restriction.clone();
        ctx.mana_pools[player.index()].add_mana(m);
    }
}

/// ReflectProperty$ Is — collect colors present on valid cards.
fn get_colors_from_cards(
    ctx: &EffectContext,
    source_id: crate::ids::CardId,
    valid: &str,
    color_or_type: &str,
) -> Vec<u16> {
    let source = ctx.game.card(source_id);
    let controller = source.controller;
    let mut colors = Vec::new();

    // Handle Defined.Imprinted, Defined.ExiledWith, etc.
    let cards_to_check: Vec<crate::ids::CardId> = if valid.starts_with("Defined.") {
        let def = &valid[8..];
        match def {
            "Imprinted" | "ExiledWith" => {
                // Check cards exiled by this card (imprinted)
                ctx.game
                    .cards
                    .iter()
                    .filter(|c| {
                        c.zone == ZoneType::Exile && c.remembered_cards.contains(&source_id)
                    })
                    .map(|c| c.id)
                    .collect()
            }
            "Self" => vec![source_id],
            _ => vec![],
        }
    } else {
        // Filter battlefield cards matching Valid$
        collect_valid_cards(ctx, source_id, valid, controller)
    };

    for &cid in &cards_to_check {
        let card = ctx.game.card(cid);
        for color in card.color.iter() {
            let atom = match color.long_name() {
                "white" => ManaAtom::WHITE,
                "blue" => ManaAtom::BLUE,
                "black" => ManaAtom::BLACK,
                "red" => ManaAtom::RED,
                "green" => ManaAtom::GREEN,
                _ => continue,
            };
            if !colors.contains(&atom) {
                colors.push(atom);
            }
        }
    }

    // If ColorOrType$ is "Type", also allow colorless
    if color_or_type == "Type" && !colors.contains(&ManaAtom::COLORLESS) {
        colors.push(ManaAtom::COLORLESS);
    }

    colors
}

/// ReflectProperty$ Produce — collect mana types that mana abilities on valid cards could produce.
fn get_colors_from_mana_abilities(
    ctx: &EffectContext,
    source_id: crate::ids::CardId,
    valid: &str,
    color_or_type: &str,
) -> Vec<u16> {
    let source = ctx.game.card(source_id);
    let controller = source.controller;
    let mut colors = Vec::new();

    let cards_to_check = collect_valid_cards(ctx, source_id, valid, controller);

    for &cid in &cards_to_check {
        // Skip self to avoid infinite recursion
        if cid == source_id {
            continue;
        }
        let card = ctx.game.card(cid);
        for ab in &card.activated_abilities {
            if !ab.is_mana_ability {
                continue;
            }
            if let Some(produced) = ab.params.get("Produced") {
                // Parse produced colors
                for tok in produced.replace(',', " ").split_whitespace() {
                    let lower = tok.to_lowercase();
                    let atom = match lower.as_str() {
                        "w" | "white" => Some(ManaAtom::WHITE),
                        "u" | "blue" => Some(ManaAtom::BLUE),
                        "b" | "black" => Some(ManaAtom::BLACK),
                        "r" | "red" => Some(ManaAtom::RED),
                        "g" | "green" => Some(ManaAtom::GREEN),
                        "c" | "colorless" => Some(ManaAtom::COLORLESS),
                        "any" => {
                            // "Any" means all 5 colors
                            for &a in &[
                                ManaAtom::WHITE,
                                ManaAtom::BLUE,
                                ManaAtom::BLACK,
                                ManaAtom::RED,
                                ManaAtom::GREEN,
                            ] {
                                if !colors.contains(&a) {
                                    colors.push(a);
                                }
                            }
                            None
                        }
                        _ => {
                            // Try Combo patterns like "Combo White Blue"
                            if tok.starts_with("Combo") {
                                // Already handled by split
                                None
                            } else {
                                color_name_to_mana_atom(tok)
                                    .and_then(|_| mana_atom_from_produced(tok))
                            }
                        }
                    };
                    if let Some(a) = atom {
                        if !colors.contains(&a) {
                            colors.push(a);
                        }
                    }
                }
            }
        }
    }

    // If ColorOrType$ is "Type", also allow colorless
    if color_or_type == "Type" && !colors.contains(&ManaAtom::COLORLESS) {
        colors.push(ManaAtom::COLORLESS);
    }

    colors
}

/// ReflectProperty$ Produced — mirror the mana that was just produced (for mana doublers).
/// Used by triggered abilities like Mana Flare, Mirari's Wake.
fn get_colors_from_produced(sa: &SpellAbility, color_or_type: &str) -> Vec<u16> {
    // The produced mana comes from the triggering event.
    // In our trigger system, the producing mana ability's Produced$ is stored
    // in the parent ability chain. For simplicity, look at the SA's own params
    // or fall back to all 5 colors.
    let mut colors = Vec::new();

    // Check if there's a Produced$ on the SA itself
    if let Some(produced) = sa.params.get("Produced") {
        for tok in produced.replace(',', " ").split_whitespace() {
            if let Some(atom) = mana_atom_from_produced(tok) {
                if !colors.contains(&atom) {
                    colors.push(atom);
                }
            }
        }
    }

    // If no colors found, allow all (the trigger handler should have set produced)
    if colors.is_empty() {
        colors = vec![
            ManaAtom::WHITE,
            ManaAtom::BLUE,
            ManaAtom::BLACK,
            ManaAtom::RED,
            ManaAtom::GREEN,
        ];
        if color_or_type == "Type" {
            colors.push(ManaAtom::COLORLESS);
        }
    }

    colors
}

/// Collect card IDs on the battlefield matching a Valid$ filter string.
fn collect_valid_cards(
    ctx: &EffectContext,
    source_id: crate::ids::CardId,
    valid: &str,
    controller: crate::ids::PlayerId,
) -> Vec<crate::ids::CardId> {
    let mut result = Vec::new();

    // Parse comma-separated valid alternatives (OR)
    for part in valid.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        for card in ctx.game.cards.iter() {
            if card.zone != ZoneType::Battlefield {
                continue;
            }
            if matches_valid_for_reflect(card, part, source_id, controller, ctx) {
                if !result.contains(&card.id) {
                    result.push(card.id);
                }
            }
        }
    }

    result
}

/// Simple valid card matching for ManaReflected.
fn matches_valid_for_reflect(
    card: &crate::card::CardInstance,
    valid: &str,
    source_id: crate::ids::CardId,
    controller: crate::ids::PlayerId,
    _ctx: &EffectContext,
) -> bool {
    // Split by "." for qualifiers: e.g. "Land.YouCtrl", "Creature.Legendary+YouCtrl"
    let parts: Vec<&str> = valid.split('.').collect();
    let base_type = parts[0];

    // Check base type
    let type_ok = match base_type {
        "Card" => true,
        "Land" => card.type_line.is_land(),
        "Creature" => card.is_creature(),
        "Permanent" => true, // anything on battlefield is a permanent
        "Artifact" => card.type_line.is_artifact(),
        "Enchantment" => card.type_line.is_enchantment(),
        "Planeswalker" => card.type_line.is_planeswalker(),
        "Gate" => card.type_line.has_subtype("Gate"),
        _ => {
            // Check as subtype
            card.type_line.has_subtype(base_type)
        }
    };
    if !type_ok {
        return false;
    }

    // Check qualifiers
    for &qualifier in parts.iter().skip(1) {
        // Handle + compound qualifiers
        for q in qualifier.split('+') {
            match q {
                "YouCtrl" => {
                    if card.controller != controller {
                        return false;
                    }
                }
                "OppCtrl" => {
                    if card.controller == controller {
                        return false;
                    }
                }
                "Self" => {
                    if card.id != source_id {
                        return false;
                    }
                }
                "Legendary" => {
                    if !card.type_line.is_legendary() {
                        return false;
                    }
                }
                _ => {
                    // Check as subtype
                    if !card.type_line.has_subtype(q) {
                        // Unknown qualifier, skip
                    }
                }
            }
        }
    }

    true
}
