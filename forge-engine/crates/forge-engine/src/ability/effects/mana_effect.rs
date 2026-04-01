use forge_foundation::mana::ManaAtom;

use super::{mana_atom_from_produced, EffectContext};
use crate::parsing::keys;
use crate::spellability::SpellAbility;

/// Build/configure the spell ability after construction.
/// Mirrors Java's `ManaEffect.buildSpellAbility(SpellAbility)`.
///
/// For mana effects, marks the ability as a mana ability if it doesn't
/// use targeting.
pub fn build_spell_ability(sa: &mut crate::spellability::SpellAbility) {
    // Mana abilities that don't target are mana abilities (can be activated
    // any time you need mana, don't use the stack).
    if !sa.uses_targeting() {
        sa.is_mana_ability = true;
    }
}

/// Handle special mana production types.
/// Mirrors Java's `ManaEffect.handleSpecialMana(SpellAbility, String)`.
///
/// This is a public wrapper around `resolve_special_mana` for structural parity.
pub fn handle_special_mana(
    ctx: &mut EffectContext,
    sa: &crate::spellability::SpellAbility,
    source_id: crate::ids::CardId,
    player: crate::ids::PlayerId,
    special: &str,
) -> Vec<String> {
    resolve_special_mana(ctx, sa, source_id, player, special)
}

/// Resolve DB$ Mana — produce mana as a sub-ability effect.
/// Mirrors Java's ManaEffect.java for the stack-based resolution path.
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let player = sa.activating_player;
    let source_id = match sa.source {
        Some(id) => id,
        None => return,
    };

    let produced = match sa.params.get(keys::PRODUCED) {
        Some(p) => p.to_string(),
        None => return,
    };

    // Read metadata params from the ability
    let restriction = sa.params.get_cloned(keys::RESTRICT_VALID);
    let adds_no_counter = sa
        .params
        .get(keys::ADDS_NO_COUNTER)
        .map_or(false, |v| v == "True");
    let adds_keywords = sa.params.get_cloned(keys::ADDS_KEYWORDS);
    let adds_keywords_valid = sa.params.get_cloned(keys::ADDS_KEYWORDS_VALID);
    let adds_counters = sa.params.get_cloned(keys::ADDS_COUNTERS);
    let adds_counters_valid = sa.params.get_cloned(keys::ADDS_COUNTERS_VALID);
    let triggers_when_spent = sa.params.get_cloned(keys::TRIGGERS_WHEN_SPENT);

    // Handle Special mana production types (mirrors Java handleSpecialMana)
    if produced.starts_with("Special") {
        let special = produced.strip_prefix("Special ").unwrap_or("");
        let mana_tokens = resolve_special_mana(ctx, sa, source_id, player, special);
        if mana_tokens.is_empty() {
            return;
        }
        let source_is_snow = ctx.game.card(source_id).type_line.is_snow();
        for tok in &mana_tokens {
            if let Some(atom) = mana_atom_from_produced(tok) {
                let mut m = crate::mana::Mana::simple(atom);
                m.source_card = Some(source_id);
                m.is_snow = source_is_snow;
                m.restriction = restriction.clone();
                m.adds_no_counter = adds_no_counter;
                m.adds_keywords = adds_keywords.clone();
                m.adds_keywords_valid = adds_keywords_valid.clone();
                m.adds_counters = adds_counters.clone();
                m.adds_counters_valid = adds_counters_valid.clone();
                m.triggers_when_spent = triggers_when_spent.clone();
                ctx.mana_pools[player.index()].add_mana(m);
            }
        }
        return;
    }

    // Determine mana string to produce
    let mana_string: Option<String> = if produced.contains("Any") {
        // "Any" — all 5 colors available
        let available = vec![
            "W".to_string(),
            "U".to_string(),
            "B".to_string(),
            "R".to_string(),
            "G".to_string(),
        ];
        Some(available.first().cloned().unwrap_or("W".to_string()))
    } else if produced.starts_with("Combo") || produced.contains(',') {
        // Combo or comma-separated choices — normalize to color letters
        let options: Vec<&str> = if let Some(rest) = produced.strip_prefix("Combo ") {
            rest.split_whitespace().collect()
        } else {
            produced.split(',').map(|s| s.trim()).collect()
        };
        let available: Vec<String> = options
            .iter()
            .filter_map(|name| {
                let lower = name.to_lowercase();
                match lower.as_str() {
                    "white" | "w" => Some("W".to_string()),
                    "blue" | "u" => Some("U".to_string()),
                    "black" | "b" => Some("B".to_string()),
                    "red" | "r" => Some("R".to_string()),
                    "green" | "g" => Some("G".to_string()),
                    "colorless" | "c" => Some("C".to_string()),
                    _ => mana_atom_from_produced(name).map(|_| name.to_string()),
                }
            })
            .collect();
        available
            .first()
            .cloned()
            .or_else(|| Some(produced.clone()))
    } else if produced == "Chosen" {
        // Use card's chosen color
        let chosen = ctx
            .game
            .card(source_id)
            .chosen_colors
            .first()
            .cloned()
            .unwrap_or_else(|| "White".to_string());
        let lower = chosen.to_lowercase();
        match lower.as_str() {
            "white" => Some("W".to_string()),
            "blue" => Some("U".to_string()),
            "black" => Some("B".to_string()),
            "red" => Some("R".to_string()),
            "green" => Some("G".to_string()),
            _ => Some("C".to_string()),
        }
    } else {
        // Raw produced string (e.g. "W", "B B", "C")
        Some(produced.clone())
    };

    // Apply Amount$ multiplier, with specifyManaCombo for multi-amount any/combo mana
    let mut final_mana = match mana_string {
        Some(ms) => ms,
        None => return,
    };
    let amount = super::resolve_numeric_svar(ctx.game, sa, "Amount", 1);
    if amount <= 0 {
        return;
    }
    let is_combo =
        produced.contains("Any") || produced.starts_with("Combo") || produced.contains(',');
    if amount > 1 && is_combo {
        // Multi-amount combo: let agent choose color distribution
        let available: Vec<String> = if produced.contains("Any") {
            vec!["W", "U", "B", "R", "G"]
                .into_iter()
                .map(String::from)
                .collect()
        } else {
            let options: Vec<&str> = if let Some(rest) = produced.strip_prefix("Combo ") {
                rest.split_whitespace().collect()
            } else {
                produced.split(',').map(|s| s.trim()).collect()
            };
            options
                .iter()
                .filter_map(|name| {
                    let lower = name.to_lowercase();
                    match lower.as_str() {
                        "white" | "w" => Some("W".to_string()),
                        "blue" | "u" => Some("U".to_string()),
                        "black" | "b" => Some("B".to_string()),
                        "red" | "r" => Some("R".to_string()),
                        "green" | "g" => Some("G".to_string()),
                        "colorless" | "c" => Some("C".to_string()),
                        _ => None,
                    }
                })
                .collect()
        };
        let card_name = ctx.game.card(source_id).card_name.clone();
        let chosen = ctx.agents[player.index()].specify_mana_combo(
            player,
            &available,
            amount as usize,
            Some(&card_name),
        );
        final_mana = chosen.join(" ");
    } else if amount > 1 {
        let base = final_mana.clone();
        for _ in 1..amount {
            final_mana.push(' ');
            final_mana.push_str(&base);
        }
    }

    // Apply ProduceMana replacement effects (mana doublers)
    {
        use crate::replacement::replacement_handler::{apply_replacements, ReplacementEvent};
        let mut event = ReplacementEvent::ProduceMana {
            source: source_id,
            activator: player,
            mana: final_mana.clone(),
        };
        let result = apply_replacements(ctx.game, &mut event);
        if result == crate::replacement::ReplacementResult::Updated {
            if let ReplacementEvent::ProduceMana { mana: new_mana, .. } = event {
                final_mana = new_mana;
            }
        }
    }

    // Check if source is snow
    let source_is_snow = ctx.game.card(source_id).type_line.is_snow();

    // Add the mana to the pool with full metadata
    for tok in final_mana.split_whitespace() {
        if let Some(atom) = mana_atom_from_produced(tok) {
            let mut m = crate::mana::Mana::simple(atom);
            m.source_card = Some(source_id);
            m.is_snow = source_is_snow;
            m.restriction = restriction.clone();
            m.adds_no_counter = adds_no_counter;
            m.adds_keywords = adds_keywords.clone();
            m.adds_keywords_valid = adds_keywords_valid.clone();
            m.adds_counters = adds_counters.clone();
            m.adds_counters_valid = adds_counters_valid.clone();
            m.triggers_when_spent = triggers_when_spent.clone();
            ctx.mana_pools[player.index()].add_mana(m);
        }
    }
}

/// Resolve special mana production types (mirrors Java ManaEffect.handleSpecialMana).
/// Returns a list of mana letter tokens to produce (e.g. ["W", "U", "B"]).
pub fn resolve_special_mana(
    ctx: &mut EffectContext,
    sa: &SpellAbility,
    source_id: crate::ids::CardId,
    player: crate::ids::PlayerId,
    special: &str,
) -> Vec<String> {
    use forge_foundation::ZoneType;

    if special.starts_with("EachColorAmong_Valid") {
        // EachColorAmong_Valid <ValidFilter> — one mana per unique color among matching permanents
        let filter = special
            .strip_prefix("EachColorAmong_Valid ")
            .unwrap_or("Permanent.YouCtrl");
        return each_color_among_valid(ctx, source_id, player, filter);
    }

    if special == "EachColorAmong_ExiledWith" {
        // One mana per unique color among cards exiled by this card
        let mut colors = Vec::new();
        for card in ctx.game.cards.iter() {
            if card.zone != ZoneType::Exile {
                continue;
            }
            // Check if exiled by source (via remembered_cards or exiled_with)
            if !card.remembered_cards.contains(&source_id) {
                continue;
            }
            for color in card.color.iter() {
                let letter = color_to_letter(color.long_name());
                if !letter.is_empty() && !colors.contains(&letter) {
                    colors.push(letter);
                }
            }
        }
        return colors;
    }

    if special.starts_with("EachColoredManaSymbol") {
        // EachColoredManaSymbol_Milled — one mana per colored symbol in milled card's cost
        // The milled card should be the last card that entered the graveyard
        if special.contains("Milled") {
            let gy = ctx.game.cards_in_zone(ZoneType::Graveyard, player);
            if let Some(&last_card_id) = gy.last() {
                let milled = ctx.game.card(last_card_id);
                let mut tokens = Vec::new();
                for &shard in milled.mana_cost.shards() {
                    let atoms = shard.shard();
                    if (atoms & ManaAtom::WHITE) != 0 {
                        tokens.push("W".to_string());
                    }
                    if (atoms & ManaAtom::BLUE) != 0 {
                        tokens.push("U".to_string());
                    }
                    if (atoms & ManaAtom::BLACK) != 0 {
                        tokens.push("B".to_string());
                    }
                    if (atoms & ManaAtom::RED) != 0 {
                        tokens.push("R".to_string());
                    }
                    if (atoms & ManaAtom::GREEN) != 0 {
                        tokens.push("G".to_string());
                    }
                }
                return tokens;
            }
        }
        return vec![];
    }

    if special == "DoubleManaInPool" {
        // Double the amount of each type of mana in pool
        let pool = &ctx.mana_pools[player.index()];
        let mut tokens = Vec::new();
        for &(atom, letter) in &[
            (ManaAtom::WHITE, "W"),
            (ManaAtom::BLUE, "U"),
            (ManaAtom::BLACK, "B"),
            (ManaAtom::RED, "R"),
            (ManaAtom::GREEN, "G"),
            (ManaAtom::COLORLESS, "C"),
        ] {
            let count = pool.count_color(atom);
            for _ in 0..count {
                tokens.push(letter.to_string());
            }
        }
        return tokens;
    }

    if special == "EnchantedManaCost" {
        // Produce mana matching the enchanted permanent's mana cost colors
        let source = ctx.game.card(source_id);
        if let Some(attached_to) = source.attached_to {
            let enchanted = ctx.game.card(attached_to);
            let mut tokens = Vec::new();
            for &shard in enchanted.mana_cost.shards() {
                let atoms = shard.shard();
                if (atoms & ManaAtom::WHITE) != 0 {
                    tokens.push("W".to_string());
                }
                if (atoms & ManaAtom::BLUE) != 0 {
                    tokens.push("U".to_string());
                }
                if (atoms & ManaAtom::BLACK) != 0 {
                    tokens.push("B".to_string());
                }
                if (atoms & ManaAtom::RED) != 0 {
                    tokens.push("R".to_string());
                }
                if (atoms & ManaAtom::GREEN) != 0 {
                    tokens.push("G".to_string());
                }
            }
            // Also add generic as colorless
            let generic = enchanted.mana_cost.generic_cost();
            for _ in 0..generic {
                tokens.push("C".to_string());
            }
            return tokens;
        }
        return vec![];
    }

    if special.starts_with("LastNotedType") {
        // Produce mana of the last noted type (Jeweled Amulet, Ice Cauldron)
        let source = ctx.game.card(source_id);
        if let Some(noted) = source.chosen_colors.first() {
            let letter = color_to_letter(&noted.to_lowercase());
            if !letter.is_empty() {
                // Check for Amount$ or default to 1
                let amount = super::resolve_numeric_svar(ctx.game, sa, "Amount", 1);
                let mut tokens = Vec::new();
                for _ in 0..amount {
                    tokens.push(letter.clone());
                }
                return tokens;
            }
        }
        return vec![];
    }

    vec![]
}

/// Collect one mana per unique color among permanents matching the valid filter.
fn each_color_among_valid(
    ctx: &EffectContext,
    source_id: crate::ids::CardId,
    controller: crate::ids::PlayerId,
    filter: &str,
) -> Vec<String> {
    use forge_foundation::ZoneType;

    let mut colors = Vec::new();

    for card in ctx.game.cards.iter() {
        if card.zone != ZoneType::Battlefield {
            continue;
        }
        if !matches_simple_valid(card, filter, source_id, controller) {
            continue;
        }
        for color in card.color.iter() {
            let letter = color_to_letter(color.long_name());
            if !letter.is_empty() && !colors.contains(&letter) {
                colors.push(letter);
            }
        }
    }

    colors
}

/// Simple valid filter matching for special mana (e.g. "Permanent.YouCtrl+MonoColor").
fn matches_simple_valid(
    card: &crate::card::Card,
    filter: &str,
    source_id: crate::ids::CardId,
    controller: crate::ids::PlayerId,
) -> bool {
    let parts: Vec<&str> = filter.split('.').collect();
    let base = parts[0];

    let type_ok = match base {
        "Permanent" | "Card" => true,
        "Creature" => card.is_creature(),
        "Land" => card.type_line.is_land(),
        "Artifact" => card.type_line.is_artifact(),
        "Enchantment" => card.type_line.is_enchantment(),
        _ => card.type_line.has_subtype(base),
    };
    if !type_ok {
        return false;
    }

    for &qualifier in parts.iter().skip(1) {
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
                "MonoColor" => {
                    // Card must be exactly one color
                    if card.color.count_colors() != 1 {
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
                _ => {}
            }
        }
    }

    true
}

fn color_to_letter(color_name: &str) -> String {
    match color_name {
        "white" => "W".to_string(),
        "blue" => "U".to_string(),
        "black" => "B".to_string(),
        "red" => "R".to_string(),
        "green" => "G".to_string(),
        _ => String::new(),
    }
}
