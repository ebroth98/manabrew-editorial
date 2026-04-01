//! Token-level parser for individual cost tokens.
//!
//! Extracted from the monolithic `parse_cost` if/else chain in `mod.rs`.
//! Each prefix gets its own small parse function; `parse_cost_token` dispatches.

use forge_foundation::{ManaCost, ZoneType};

use super::{CostPart, RevealFrom};
use crate::ability::effects::parse_counter_type;

/// Result of attempting to parse a single cost token.
pub(super) enum TokenResult {
    /// Successfully parsed into a CostPart.
    Part(CostPart),
    /// Token sets the tap flag.
    Tap,
    /// Token sets the mandatory flag.
    Mandatory,
    /// Token is a mana symbol (accumulate for later).
    Mana,
}

/// Parse a single cost token into a `TokenResult`.
pub(super) fn parse_cost_token(token: &str) -> TokenResult {
    // Exact matches first
    if token == "T" {
        return TokenResult::Tap;
    }
    if token == "Q" || token == "Untap" {
        return TokenResult::Part(CostPart::Untap);
    }
    if token == "Mandatory" {
        return TokenResult::Mandatory;
    }
    if token == "Forage" {
        return TokenResult::Part(CostPart::Forage);
    }
    if token.starts_with("PromiseGift") {
        return TokenResult::Part(CostPart::PromiseGift);
    }

    // Prefix-based dispatch
    if let Some(part) = try_parse_prefixed(token) {
        return TokenResult::Part(part);
    }

    TokenResult::Mana
}

// ---------------------------------------------------------------------------
// Prefix dispatch
// ---------------------------------------------------------------------------

/// Strip `prefix` and trailing `>`, then call `parser` on the inner content.
fn try_strip_parse(
    token: &str,
    prefix: &str,
    parser: fn(&str) -> Option<CostPart>,
) -> Option<CostPart> {
    token
        .strip_prefix(prefix)
        .and_then(|s| s.strip_suffix('>'))
        .and_then(parser)
}

/// Try every known prefix in order, returning the first match.
fn try_parse_prefixed(token: &str) -> Option<CostPart> {
    // The order mirrors the original if/else chain so that more-specific
    // prefixes (e.g. "ExileFromHand<") are tried before shorter ones
    // (e.g. "Exile<") where necessary. Prefixes that share no common
    // start can appear in any order.

    try_strip_parse(token, "Mana<", parse_mana_cost)
        .or_else(|| try_strip_parse(token, "Sac<", parse_sacrifice))
        .or_else(|| try_strip_parse(token, "Discard<", parse_discard))
        .or_else(|| try_strip_parse(token, "PayLife<", parse_pay_life))
        .or_else(|| try_strip_parse(token, "SubCounter<", parse_sub_counter))
        .or_else(|| try_strip_parse(token, "AddCounter<", parse_add_counter))
        .or_else(|| try_strip_parse(token, "PayEnergy<", parse_pay_energy))
        .or_else(|| try_strip_parse(token, "PayShards<", parse_pay_shards))
        .or_else(|| try_strip_parse(token, "ChooseColor<", parse_choose_color))
        .or_else(|| try_strip_parse(token, "ChooseCreatureType<", parse_choose_creature_type))
        .or_else(|| try_strip_parse(token, "FlipCoin<", parse_flip_coin))
        .or_else(|| try_strip_parse(token, "RollDice<", parse_roll_dice))
        // Exile variants — longer prefixes first
        .or_else(|| try_strip_parse(token, "ExileFromHand<", parse_exile_from_hand))
        .or_else(|| try_strip_parse(token, "ExileFromGrave<", parse_exile_from_grave))
        .or_else(|| try_strip_parse(token, "ExileFromTop<", parse_exile_from_top))
        .or_else(|| try_strip_parse(token, "ExileFromStack<", parse_exile_from_stack))
        .or_else(|| try_strip_parse(token, "ExileAnyGrave<", parse_exile_any_grave))
        .or_else(|| try_strip_parse(token, "ExileSameGrave<", parse_exile_same_grave))
        .or_else(|| try_strip_parse(token, "ExileCtrlOrGrave<", parse_exile_ctrl_or_grave))
        .or_else(|| try_strip_parse(token, "ExiledMoveToGrave<", parse_exiled_move_to_grave))
        .or_else(|| try_strip_parse(token, "Exile<", parse_exile_battlefield))
        .or_else(|| try_strip_parse(token, "Return<", parse_return))
        .or_else(|| try_strip_parse(token, "tapXType<", parse_tap_type))
        .or_else(|| try_strip_parse(token, "untapYType<", parse_untap_type))
        .or_else(|| try_strip_parse(token, "DamageYou<", parse_damage_you))
        .or_else(|| try_strip_parse(token, "Draw<", parse_draw))
        .or_else(|| try_strip_parse(token, "Mill<", parse_mill))
        .or_else(|| try_strip_parse(token, "Reveal<", parse_reveal))
        .or_else(|| try_strip_parse(token, "ChooseCard<", parse_choose_card))
        .or_else(|| try_strip_parse(token, "RevealFromExile<", parse_reveal_from_exile))
        .or_else(|| try_strip_parse(token, "RevealOrChoose<", parse_reveal_or_choose))
        .or_else(|| try_strip_parse(token, "RevealChosen<", parse_reveal_chosen))
        .or_else(|| try_strip_parse(token, "BeholdExile<", parse_behold_exile))
        .or_else(|| try_strip_parse(token, "Behold<", parse_behold))
        .or_else(|| parse_exert(token))
        .or_else(|| try_strip_parse(token, "GainLife<", parse_gain_life))
        .or_else(|| try_strip_parse(token, "GainControl<", parse_gain_control))
        .or_else(|| try_strip_parse(token, "RemoveAnyCounter<", parse_remove_any_counter))
        .or_else(|| try_strip_parse(token, "Unattach<", parse_unattach))
        .or_else(|| try_strip_parse(token, "Waterbend<", parse_waterbend))
        .or_else(|| try_strip_parse(token, "AddMana<", parse_add_mana))
        .or_else(|| try_strip_parse(token, "CollectEvidence<", parse_collect_evidence))
        .or_else(|| {
            try_strip_parse(
                token,
                "PutCardToLibFromHand<",
                parse_put_card_to_lib_from_hand,
            )
        })
        .or_else(|| {
            try_strip_parse(
                token,
                "PutCardToLibFromSameGrave<",
                parse_put_card_to_lib_from_same_grave,
            )
        })
        .or_else(|| {
            try_strip_parse(
                token,
                "PutCardToLibFromGrave<",
                parse_put_card_to_lib_from_grave,
            )
        })
        .or_else(|| {
            try_strip_parse(
                token,
                "PutCardToLibFromBattlefield<",
                parse_put_card_to_lib_from_battlefield,
            )
        })
        .or_else(|| try_strip_parse(token, "Enlist<", parse_enlist))
        .or_else(|| try_strip_parse(token, "Blight<", parse_blight))
}

// ---------------------------------------------------------------------------
// Individual parsers
// ---------------------------------------------------------------------------

fn parse_mana_cost(inner: &str) -> Option<CostPart> {
    let split: Vec<&str> = inner.splitn(2, '\\').collect();
    let mana_text = split[0];
    let restriction = if split.len() > 1 {
        Some(split[1])
    } else {
        None
    };
    let mana_cost = ManaCost::parse(mana_text);

    let mut x_min = 0i32;
    let mut is_exiled_creature_cost = false;
    let mut is_enchanted_creature_cost = false;
    let mut is_cost_pay_any_number_of_times = false;

    if let Some(r) = restriction {
        if r.starts_with("XMin") {
            x_min = r[4..].parse::<i32>().unwrap_or(0);
        }
        is_exiled_creature_cost = r.eq_ignore_ascii_case("Exiled");
        is_enchanted_creature_cost = r.eq_ignore_ascii_case("EnchantedCost");
        is_cost_pay_any_number_of_times = r.eq_ignore_ascii_case("NumTimes");
    }

    Some(CostPart::Mana {
        cost: mana_cost,
        x_min,
        is_exiled_creature_cost,
        is_enchanted_creature_cost,
        is_cost_pay_any_number_of_times,
        max_waterbend: None,
    })
}

fn parse_sacrifice(inner: &str) -> Option<CostPart> {
    let (amount, filter) = super::parse_amount_filter(inner);
    Some(CostPart::Sacrifice {
        amount,
        type_filter: filter,
    })
}

fn parse_discard(inner: &str) -> Option<CostPart> {
    let (amount, filter) = super::parse_amount_filter(inner);
    Some(CostPart::Discard {
        amount,
        type_filter: filter,
    })
}

fn parse_pay_life(inner: &str) -> Option<CostPart> {
    let amount = inner.parse::<i32>().unwrap_or(0);
    Some(CostPart::PayLife(amount))
}

fn parse_sub_counter(inner: &str) -> Option<CostPart> {
    let mut it = inner.split('/');
    let amount = it.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(1);
    let counter_type_str = it.next().unwrap_or("P1P1");
    let source = it.next().unwrap_or("CARDNAME");
    if source.eq_ignore_ascii_case("CARDNAME") || source.eq_ignore_ascii_case("NICKNAME") {
        Some(CostPart::SubCounter {
            amount,
            counter_type: parse_counter_type(counter_type_str),
        })
    } else {
        None
    }
}

fn parse_add_counter(inner: &str) -> Option<CostPart> {
    let mut it = inner.split('/');
    let amount = it.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(1);
    let counter_type_str = it.next().unwrap_or("LOYALTY");
    Some(CostPart::AddCounter {
        amount,
        counter_type: parse_counter_type(counter_type_str),
    })
}

fn parse_pay_energy(inner: &str) -> Option<CostPart> {
    let amount = inner.parse::<i32>().unwrap_or(1);
    Some(CostPart::PayEnergy(amount))
}

fn parse_pay_shards(inner: &str) -> Option<CostPart> {
    let amount = super::parse_i32_or_x(inner, 1);
    Some(CostPart::PayShards(amount))
}

fn parse_choose_color(inner: &str) -> Option<CostPart> {
    let amount = super::parse_i32_or_x(inner, 1);
    Some(CostPart::ChooseColor(amount))
}

fn parse_choose_creature_type(inner: &str) -> Option<CostPart> {
    let amount = super::parse_i32_or_x(inner, 1);
    Some(CostPart::ChooseCreatureType(amount))
}

fn parse_flip_coin(inner: &str) -> Option<CostPart> {
    let amount = super::parse_i32_or_x(inner, 1);
    Some(CostPart::FlipCoin(amount))
}

fn parse_roll_dice(inner: &str) -> Option<CostPart> {
    let mut it = inner.splitn(4, '/');
    let amount = it.next().map(|s| super::parse_i32_or_x(s, 1)).unwrap_or(1);
    let sides = it.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(6);
    let result_svar = it.next().unwrap_or("").to_string();
    Some(CostPart::RollDice {
        amount,
        sides,
        result_svar,
    })
}

fn parse_exile_battlefield(inner: &str) -> Option<CostPart> {
    let (amount, filter) = super::parse_amount_filter(inner);
    Some(CostPart::Exile {
        amount,
        type_filter: filter,
        from: ZoneType::Battlefield,
    })
}

fn parse_exile_from_hand(inner: &str) -> Option<CostPart> {
    let (amount, filter) = super::parse_amount_filter(inner);
    Some(CostPart::Exile {
        amount,
        type_filter: filter,
        from: ZoneType::Hand,
    })
}

fn parse_exile_from_grave(inner: &str) -> Option<CostPart> {
    let (amount, filter) = super::parse_amount_filter(inner);
    Some(CostPart::Exile {
        amount,
        type_filter: filter,
        from: ZoneType::Graveyard,
    })
}

fn parse_exile_from_top(inner: &str) -> Option<CostPart> {
    let (amount, filter) = super::parse_amount_filter(inner);
    Some(CostPart::Exile {
        amount,
        type_filter: filter,
        from: ZoneType::Library,
    })
}

fn parse_exile_any_grave(inner: &str) -> Option<CostPart> {
    let (amount, filter) = super::parse_amount_filter(inner);
    Some(CostPart::ExileFromAnyGrave {
        amount,
        type_filter: filter,
    })
}

fn parse_exile_same_grave(inner: &str) -> Option<CostPart> {
    let (amount, filter) = super::parse_amount_filter_dynamic(inner);
    Some(CostPart::ExileFromSameGrave {
        amount,
        type_filter: filter,
    })
}

fn parse_exile_ctrl_or_grave(inner: &str) -> Option<CostPart> {
    let (amount, filter) = super::parse_amount_filter(inner);
    Some(CostPart::ExileCtrlOrGrave {
        amount,
        type_filter: filter,
    })
}

fn parse_exile_from_stack(inner: &str) -> Option<CostPart> {
    let (amount, filter) = super::parse_amount_filter_dynamic(inner);
    Some(CostPart::ExileFromStack {
        amount,
        type_filter: filter,
    })
}

fn parse_return(inner: &str) -> Option<CostPart> {
    let (amount, filter) = super::parse_amount_filter(inner);
    Some(CostPart::Return {
        amount,
        type_filter: filter,
    })
}

fn parse_tap_type(inner: &str) -> Option<CostPart> {
    let (amount, filter) = super::parse_amount_filter(inner);
    let (final_filter, min_total_power) = if let Some(idx) = filter.find("+withTotalPowerGE") {
        let power_str = &filter[idx + "+withTotalPowerGE".len()..];
        let power: i32 = power_str
            .trim_start_matches('{')
            .trim_end_matches('}')
            .parse()
            .unwrap_or(0);
        (filter[..idx].to_string(), Some(power))
    } else {
        (filter, None)
    };
    Some(CostPart::TapType {
        amount,
        type_filter: final_filter,
        min_total_power,
    })
}

fn parse_untap_type(inner: &str) -> Option<CostPart> {
    let (amount, filter) = super::parse_amount_filter(inner);
    Some(CostPart::UntapType {
        amount,
        type_filter: filter,
        // Default to true; post-processing in parse_cost sets the real value
        // based on whether the cost also has an Untap (Q) part.
        can_untap_source: true,
    })
}

fn parse_damage_you(inner: &str) -> Option<CostPart> {
    let amount = inner.parse::<i32>().unwrap_or(1);
    Some(CostPart::DamageYou(amount))
}

fn parse_draw(inner: &str) -> Option<CostPart> {
    let amount = inner.parse::<i32>().unwrap_or(1);
    Some(CostPart::Draw(amount))
}

fn parse_mill(inner: &str) -> Option<CostPart> {
    let amount = inner.parse::<i32>().unwrap_or(1);
    Some(CostPart::Mill(amount))
}

fn parse_reveal(inner: &str) -> Option<CostPart> {
    let (amount, filter) = super::parse_amount_filter(inner);
    Some(CostPart::Reveal {
        amount,
        type_filter: filter,
        from: RevealFrom::All,
    })
}

fn parse_choose_card(inner: &str) -> Option<CostPart> {
    let (amount, filter) = super::parse_amount_filter(inner);
    Some(CostPart::Reveal {
        amount,
        type_filter: filter,
        from: RevealFrom::Hand,
    })
}

fn parse_reveal_from_exile(inner: &str) -> Option<CostPart> {
    let (amount, filter) = super::parse_amount_filter(inner);
    Some(CostPart::Reveal {
        amount,
        type_filter: filter,
        from: RevealFrom::Exile,
    })
}

fn parse_reveal_or_choose(inner: &str) -> Option<CostPart> {
    let (amount, filter) = super::parse_amount_filter(inner);
    Some(CostPart::Reveal {
        amount,
        type_filter: filter,
        from: RevealFrom::HandOrBattlefield,
    })
}

fn parse_reveal_chosen(inner: &str) -> Option<CostPart> {
    let reveal_type = inner.split('/').next().unwrap_or("Player").to_string();
    Some(CostPart::RevealChosen { reveal_type })
}

fn parse_behold(inner: &str) -> Option<CostPart> {
    let (amount, filter) = super::parse_amount_filter_dynamic(inner);
    Some(CostPart::Behold {
        amount,
        type_filter: filter,
        exile: false,
    })
}

fn parse_behold_exile(inner: &str) -> Option<CostPart> {
    let (amount, filter) = super::parse_amount_filter_dynamic(inner);
    Some(CostPart::Behold {
        amount,
        type_filter: filter,
        exile: true,
    })
}

/// Exert is special: it can appear as `Exert<...>` (with angle brackets)
/// or potentially bare. The original code had a fallback else-branch.
fn parse_exert(token: &str) -> Option<CostPart> {
    if !token.starts_with("Exert<") {
        return None;
    }
    if let Some(inner) = token
        .strip_prefix("Exert<")
        .and_then(|s| s.strip_suffix('>'))
    {
        let (amount, filter) = super::parse_amount_filter_dynamic(inner);
        Some(CostPart::Exert {
            amount,
            type_filter: filter,
        })
    } else {
        // Exert without proper angle brackets — default
        Some(CostPart::Exert {
            amount: 1,
            type_filter: "CARDNAME".to_string(),
        })
    }
}

fn parse_gain_life(inner: &str) -> Option<CostPart> {
    let amount = inner
        .split('/')
        .next()
        .and_then(|s| s.trim().parse::<i32>().ok())
        .unwrap_or(1);
    Some(CostPart::GainLife(amount))
}

fn parse_gain_control(inner: &str) -> Option<CostPart> {
    let (amount, filter) = super::parse_amount_filter(inner);
    Some(CostPart::GainControl {
        amount,
        type_filter: filter,
    })
}

fn parse_remove_any_counter(inner: &str) -> Option<CostPart> {
    let mut it = inner.split('/');
    let amount = it.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(1);
    let counter_str = it.next().unwrap_or("Any");
    let type_filter = it.next().unwrap_or("Permanent").to_string();
    let counter_type = if counter_str.eq_ignore_ascii_case("Any") || counter_str.is_empty() {
        None
    } else {
        Some(parse_counter_type(counter_str))
    };
    Some(CostPart::RemoveAnyCounter {
        amount,
        type_filter,
        counter_type,
    })
}

/// Parse `Unattach<Type/Description>`.
/// Java: `abCostParse(parse, 2)` splits into [type, description].
/// The constructor passes `("1", type, desc)` to `CostPartWithList`.
fn parse_unattach(inner: &str) -> Option<CostPart> {
    let (type_filter, description) = if let Some(slash_idx) = inner.find('/') {
        let tf = inner[..slash_idx].to_string();
        let desc = inner[slash_idx + 1..].to_string();
        (tf, if desc.is_empty() { None } else { Some(desc) })
    } else {
        (inner.to_string(), None)
    };
    Some(CostPart::Unattach {
        type_filter,
        description,
    })
}

fn parse_waterbend(inner: &str) -> Option<CostPart> {
    let amount = inner.parse::<i32>().unwrap_or(0);
    Some(CostPart::Waterbend { amount })
}

fn parse_add_mana(inner: &str) -> Option<CostPart> {
    let (amount, mana_type) = super::parse_amount_filter(inner);
    Some(CostPart::AddMana { amount, mana_type })
}

fn parse_exiled_move_to_grave(inner: &str) -> Option<CostPart> {
    let (amount, filter) = super::parse_amount_filter(inner);
    Some(CostPart::ExiledMoveToGrave {
        amount,
        type_filter: filter,
    })
}

fn parse_collect_evidence(inner: &str) -> Option<CostPart> {
    let amount = super::parse_i32_or_x(inner, 1);
    Some(CostPart::CollectEvidence(amount))
}

fn parse_put_card_to_lib_from_hand(inner: &str) -> Option<CostPart> {
    parse_put_card_to_lib(inner, ZoneType::Hand, false)
}

fn parse_put_card_to_lib_from_grave(inner: &str) -> Option<CostPart> {
    parse_put_card_to_lib(inner, ZoneType::Graveyard, false)
}

fn parse_put_card_to_lib_from_same_grave(inner: &str) -> Option<CostPart> {
    parse_put_card_to_lib(inner, ZoneType::Graveyard, true)
}

fn parse_put_card_to_lib_from_battlefield(inner: &str) -> Option<CostPart> {
    parse_put_card_to_lib(inner, ZoneType::Battlefield, false)
}

fn parse_put_card_to_lib(inner: &str, from: ZoneType, same_zone: bool) -> Option<CostPart> {
    let mut it = inner.splitn(4, '/');
    let amount = it.next().map(|s| super::parse_i32_or_x(s, 1)).unwrap_or(1);
    let lib_pos = it.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
    let type_filter = it.next().unwrap_or("Card").to_string();
    Some(CostPart::PutCardToLib {
        amount,
        lib_pos,
        type_filter,
        from,
        same_zone,
    })
}

fn parse_enlist(inner: &str) -> Option<CostPart> {
    let pieces: Vec<&str> = inner.split('/').collect();
    let amount = pieces
        .first()
        .map(|s| super::parse_i32_or_x(s, 1))
        .unwrap_or(1);
    let filter = if pieces.len() >= 3 {
        pieces[2]
    } else {
        pieces.get(1).copied().unwrap_or("")
    }
    .to_string();
    Some(CostPart::Enlist {
        amount,
        type_filter: filter,
    })
}

fn parse_blight(inner: &str) -> Option<CostPart> {
    let amount = super::parse_i32_or_x(inner, 1);
    Some(CostPart::Blight(amount))
}
