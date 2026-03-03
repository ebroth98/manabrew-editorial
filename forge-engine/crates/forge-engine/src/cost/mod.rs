use forge_foundation::{ManaCost, ZoneType};
use serde::{Deserialize, Serialize};

use crate::ability::effects::{matches_change_type, parse_counter_type};
use crate::card::CounterType;
use crate::game::GameState;
use crate::ids::{CardId, PlayerId};
use crate::mana::ManaPool;
use crate::staticability::static_ability_cant_gain_lose_pay_life::cant_pay_life;

/// A single component of an ability cost.
/// Mirrors Java's CostPart hierarchy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CostPart {
    /// Tap the source permanent. {T}
    Tap,
    /// Pay mana.
    Mana(ManaCost),
    /// Pay life. Mirrors CostPayLife.
    PayLife(i32),
    /// Sacrifice permanents. type_filter "CARDNAME" means sacrifice self.
    Sacrifice { amount: i32, type_filter: String },
    /// Discard cards. type_filter "CARDNAME" means discard self.
    Discard { amount: i32, type_filter: String },
    /// Remove counters from the source permanent (e.g. SubCounter<1/DREAM/NICKNAME>).
    SubCounter { amount: i32, counter_type: CounterType },
    /// Add counters to the source permanent (e.g. AddCounter<1/LOYALTY>). Mirrors CostPutCounter.
    AddCounter { amount: i32, counter_type: CounterType },
    /// Exile cards from a specific zone (own zone) as cost. Mirrors CostExile.
    Exile {
        amount: i32,
        type_filter: String,
        from: ZoneType,
    },
    /// Exile cards from any player's graveyard as cost (ExileAnyGrave). Mirrors CostExile zoneMode=-1.
    ExileFromAnyGrave {
        amount: i32,
        type_filter: String,
    },
    /// Return permanents to owner's hand as cost. Mirrors CostReturn.
    Return { amount: i32, type_filter: String },
    /// Tap other permanents of a type as cost (tapXType<n/filter>). Mirrors CostTapType.
    TapType { amount: i32, type_filter: String },
    /// Untap permanents as cost. Mirrors CostUntap.
    Untap,
    /// Untap other permanents of a type as cost (untapYType<n/filter>). Mirrors CostUntapType.
    UntapType { amount: i32, type_filter: String },
    /// Pay energy counters. Mirrors CostPayEnergy.
    PayEnergy(i32),
    /// Deal damage to the source's controller as cost. Mirrors CostDamage.
    DamageYou(i32),
    /// Draw cards as cost. Mirrors CostDraw.
    Draw(i32),
    /// Mill cards as cost. Mirrors CostMill.
    Mill(i32),
    /// Reveal cards from hand as cost. Mirrors CostReveal.
    Reveal { amount: i32, type_filter: String },
    /// Exert the source permanent as cost. Mirrors CostExert.
    Exert,
    /// Opponent gains life as cost. Mirrors CostGainLife.
    GainLife(i32),
    /// Gain control of permanents matching type_filter as cost. Mirrors CostGainControl.
    GainControl { amount: i32, type_filter: String },
    /// Remove any counter type from permanents matching type_filter. Mirrors CostRemoveAnyCounter.
    /// `counter_type` is None means any counter type.
    RemoveAnyCounter { amount: i32, type_filter: String, counter_type: Option<CounterType> },
    /// Unattach the source equipment from whatever it is equipping. Mirrors CostUnattach.
    Unattach,
    /// Move cards from exile to graveyard as cost. Mirrors CostExiledMoveToGrave.
    ExiledMoveToGrave { amount: i32, type_filter: String },
}

impl CostPart {
    /// Payment ordering — mirrors Java's CostPart.paymentOrder.
    /// Lower numbers are paid first.
    fn payment_order(&self) -> i32 {
        match self {
            CostPart::Tap => -1,
            CostPart::Untap => -1,
            CostPart::Mana(_) => 0,
            CostPart::PayEnergy(_) => 5,
            CostPart::SubCounter { .. } => 6,
            CostPart::AddCounter { .. } => 6,
            CostPart::PayLife(_) => 7,
            CostPart::DamageYou(_) => 8,
            CostPart::GainLife(_) => 9,
            CostPart::Reveal { .. } => 11,
            CostPart::Draw(_) => 12,
            CostPart::Mill(_) => 13,
            CostPart::Discard { .. } => 14,
            CostPart::Sacrifice { .. } => 15,
            CostPart::Exile { .. } => 16,
            CostPart::ExileFromAnyGrave { .. } => 16,
            CostPart::Return { .. } => 17,
            CostPart::TapType { .. } => 18,
            CostPart::UntapType { .. } => 18,
            CostPart::GainControl { .. } => 8,
            CostPart::RemoveAnyCounter { .. } => 8,
            CostPart::Unattach => 10,
            CostPart::ExiledMoveToGrave { .. } => 15,
            CostPart::Exert => 20,
        }
    }
}

/// The complete cost to activate an ability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cost {
    pub parts: Vec<CostPart>,
    pub has_tap: bool,
}

/// Parse a Cost$ value from the DSL.
///
/// Examples:
/// - `"T"` → tap
/// - `"1 G"` → mana cost {1}{G}
/// - `"T 1 G"` → tap + mana
/// - `"Sac<1/CARDNAME>"` → sacrifice self
/// - `"PayLife<3>"` → pay 3 life
/// - `"Exile<1/Creature>"` → exile a creature from battlefield
/// - `"ExileFromHand<1/Card>"` → exile from hand
/// - `"Return<1/CARDNAME>"` → return self to hand
/// - `"tapXType<1/Creature>"` → tap a creature
/// - `"AddCounter<1/LOYALTY>"` → add a loyalty counter
/// - `"SubCounter<1/LOYALTY/CARDNAME>"` → remove loyalty counter
pub fn parse_cost(raw: &str) -> Cost {
    let mut parts = Vec::new();
    let mut has_tap = false;
    let mut mana_tokens: Vec<&str> = Vec::new();

    // Split on spaces, but keep <...> groups together
    let tokens = split_cost_tokens(raw);

    for token in &tokens {
        if *token == "T" {
            parts.push(CostPart::Tap);
            has_tap = true;
        } else if *token == "Q" {
            // Q = untap cost
            parts.push(CostPart::Untap);
        } else if token.starts_with("Sac<") {
            // Parse Sac<amount/filter>
            if let Some(inner) = token.strip_prefix("Sac<").and_then(|s| s.strip_suffix('>')) {
                let (amount, filter) = parse_amount_filter(inner);
                parts.push(CostPart::Sacrifice {
                    amount,
                    type_filter: filter,
                });
            }
        } else if token.starts_with("Discard<") {
            // Parse Discard<amount/filter>
            if let Some(inner) = token.strip_prefix("Discard<").and_then(|s| s.strip_suffix('>')) {
                let (amount, filter) = parse_amount_filter(inner);
                parts.push(CostPart::Discard {
                    amount,
                    type_filter: filter,
                });
            }
        } else if token.starts_with("PayLife<") {
            if let Some(inner) = token
                .strip_prefix("PayLife<")
                .and_then(|s| s.strip_suffix('>'))
            {
                let amount = inner.parse::<i32>().unwrap_or(0);
                parts.push(CostPart::PayLife(amount));
            }
        } else if token.starts_with("SubCounter<") {
            // Parse SubCounter<amount/counterType/source>.
            // We currently support paying from source only (CARDNAME/NICKNAME).
            if let Some(inner) = token
                .strip_prefix("SubCounter<")
                .and_then(|s| s.strip_suffix('>'))
            {
                let mut it = inner.split('/');
                let amount = it.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(1);
                let counter_type_str = it.next().unwrap_or("P1P1");
                let source = it.next().unwrap_or("CARDNAME");
                if source.eq_ignore_ascii_case("CARDNAME")
                    || source.eq_ignore_ascii_case("NICKNAME")
                {
                    parts.push(CostPart::SubCounter {
                        amount,
                        counter_type: parse_counter_type(counter_type_str),
                    });
                }
            }
        } else if token.starts_with("AddCounter<") {
            // Parse AddCounter<amount/counterType[/target]>
            if let Some(inner) = token
                .strip_prefix("AddCounter<")
                .and_then(|s| s.strip_suffix('>'))
            {
                let mut it = inner.split('/');
                let amount = it.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(1);
                let counter_type_str = it.next().unwrap_or("LOYALTY");
                // third segment is target; we default to source (CARDNAME)
                parts.push(CostPart::AddCounter {
                    amount,
                    counter_type: parse_counter_type(counter_type_str),
                });
            }
        } else if token.starts_with("PayEnergy<") {
            if let Some(inner) = token
                .strip_prefix("PayEnergy<")
                .and_then(|s| s.strip_suffix('>'))
            {
                let amount = inner.parse::<i32>().unwrap_or(1);
                parts.push(CostPart::PayEnergy(amount));
            }
        } else if token.starts_with("Exile<") {
            // Exile<amount/filter> — from battlefield
            if let Some(inner) = token.strip_prefix("Exile<").and_then(|s| s.strip_suffix('>')) {
                let (amount, filter) = parse_amount_filter(inner);
                parts.push(CostPart::Exile {
                    amount,
                    type_filter: filter,
                    from: ZoneType::Battlefield,
                });
            }
        } else if token.starts_with("ExileFromHand<") {
            if let Some(inner) = token
                .strip_prefix("ExileFromHand<")
                .and_then(|s| s.strip_suffix('>'))
            {
                let (amount, filter) = parse_amount_filter(inner);
                parts.push(CostPart::Exile {
                    amount,
                    type_filter: filter,
                    from: ZoneType::Hand,
                });
            }
        } else if token.starts_with("ExileFromGrave<") {
            if let Some(inner) = token
                .strip_prefix("ExileFromGrave<")
                .and_then(|s| s.strip_suffix('>'))
            {
                let (amount, filter) = parse_amount_filter(inner);
                parts.push(CostPart::Exile {
                    amount,
                    type_filter: filter,
                    from: ZoneType::Graveyard,
                });
            }
        } else if token.starts_with("ExileFromTop<") {
            if let Some(inner) = token
                .strip_prefix("ExileFromTop<")
                .and_then(|s| s.strip_suffix('>'))
            {
                let (amount, filter) = parse_amount_filter(inner);
                parts.push(CostPart::Exile {
                    amount,
                    type_filter: filter,
                    from: ZoneType::Library,
                });
            }
        } else if token.starts_with("ExileAnyGrave<") {
            // ExileAnyGrave<amount/filter> — exile from ANY player's graveyard
            if let Some(inner) = token
                .strip_prefix("ExileAnyGrave<")
                .and_then(|s| s.strip_suffix('>'))
            {
                let (amount, filter) = parse_amount_filter(inner);
                parts.push(CostPart::ExileFromAnyGrave {
                    amount,
                    type_filter: filter,
                });
            }
        } else if token.starts_with("ExileSameGrave<") {
            // ExileSameGrave<amount/filter> — exile from the same graveyard (treated same as ExileFromGrave in our model)
            if let Some(inner) = token
                .strip_prefix("ExileSameGrave<")
                .and_then(|s| s.strip_suffix('>'))
            {
                let (amount, filter) = parse_amount_filter(inner);
                parts.push(CostPart::ExileFromAnyGrave {
                    amount,
                    type_filter: filter,
                });
            }
        } else if token.starts_with("Return<") {
            // Return<amount/filter> — return permanent(s) to hand
            if let Some(inner) = token.strip_prefix("Return<").and_then(|s| s.strip_suffix('>')) {
                let (amount, filter) = parse_amount_filter(inner);
                parts.push(CostPart::Return {
                    amount,
                    type_filter: filter,
                });
            }
        } else if token.starts_with("tapXType<") {
            // tapXType<amount/filter[/desc]>
            if let Some(inner) = token
                .strip_prefix("tapXType<")
                .and_then(|s| s.strip_suffix('>'))
            {
                let (amount, filter) = parse_amount_filter(inner);
                parts.push(CostPart::TapType {
                    amount,
                    type_filter: filter,
                });
            }
        } else if token.starts_with("untapYType<") {
            // untapYType<amount/filter[/desc]>
            if let Some(inner) = token
                .strip_prefix("untapYType<")
                .and_then(|s| s.strip_suffix('>'))
            {
                let (amount, filter) = parse_amount_filter(inner);
                parts.push(CostPart::UntapType {
                    amount,
                    type_filter: filter,
                });
            }
        } else if token.starts_with("DamageYou<") {
            if let Some(inner) = token
                .strip_prefix("DamageYou<")
                .and_then(|s| s.strip_suffix('>'))
            {
                let amount = inner.parse::<i32>().unwrap_or(1);
                parts.push(CostPart::DamageYou(amount));
            }
        } else if token.starts_with("Draw<") {
            if let Some(inner) = token.strip_prefix("Draw<").and_then(|s| s.strip_suffix('>')) {
                let amount = inner.parse::<i32>().unwrap_or(1);
                parts.push(CostPart::Draw(amount));
            }
        } else if token.starts_with("Mill<") {
            if let Some(inner) = token.strip_prefix("Mill<").and_then(|s| s.strip_suffix('>')) {
                let amount = inner.parse::<i32>().unwrap_or(1);
                parts.push(CostPart::Mill(amount));
            }
        } else if token.starts_with("Reveal<") {
            if let Some(inner) = token.strip_prefix("Reveal<").and_then(|s| s.strip_suffix('>')) {
                let (amount, filter) = parse_amount_filter(inner);
                parts.push(CostPart::Reveal {
                    amount,
                    type_filter: filter,
                });
            }
        } else if token.starts_with("Exert<") {
            // Exert<amount/filter[/desc]> — exert the source creature
            parts.push(CostPart::Exert);
        } else if token.starts_with("GainLife<") {
            if let Some(inner) = token
                .strip_prefix("GainLife<")
                .and_then(|s| s.strip_suffix('>'))
            {
                let amount = inner.parse::<i32>().unwrap_or(1);
                parts.push(CostPart::GainLife(amount));
            }
        } else if token.starts_with("GainControl<") {
            // GainControl<amount/filter[/desc]>
            if let Some(inner) = token
                .strip_prefix("GainControl<")
                .and_then(|s| s.strip_suffix('>'))
            {
                let (amount, filter) = parse_amount_filter(inner);
                parts.push(CostPart::GainControl {
                    amount,
                    type_filter: filter,
                });
            }
        } else if token.starts_with("RemoveAnyCounter<") {
            // RemoveAnyCounter<amount/counterType/typeFilter[/desc]>
            // counterType can be "Any" meaning any counter.
            if let Some(inner) = token
                .strip_prefix("RemoveAnyCounter<")
                .and_then(|s| s.strip_suffix('>'))
            {
                let mut it = inner.split('/');
                let amount = it.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(1);
                let counter_str = it.next().unwrap_or("Any");
                let type_filter = it.next().unwrap_or("Permanent").to_string();
                let counter_type = if counter_str.eq_ignore_ascii_case("Any") || counter_str.is_empty() {
                    None
                } else {
                    Some(parse_counter_type(counter_str))
                };
                parts.push(CostPart::RemoveAnyCounter {
                    amount,
                    type_filter,
                    counter_type,
                });
            }
        } else if token.starts_with("Unattach<") {
            // Unattach<type[/desc]> — unattach the source equipment from its host
            parts.push(CostPart::Unattach);
        } else if token.starts_with("ExiledMoveToGrave<") {
            // ExiledMoveToGrave<amount/filter[/desc]>
            if let Some(inner) = token
                .strip_prefix("ExiledMoveToGrave<")
                .and_then(|s| s.strip_suffix('>'))
            {
                let (amount, filter) = parse_amount_filter(inner);
                parts.push(CostPart::ExiledMoveToGrave {
                    amount,
                    type_filter: filter,
                });
            }
        } else {
            // Accumulate as mana token
            mana_tokens.push(token);
        }
    }

    // If we have mana tokens, combine them into a ManaCost
    if !mana_tokens.is_empty() {
        let mana_str = mana_tokens.join(" ");
        let mana_cost = ManaCost::parse(&mana_str);
        if mana_cost.cmc() > 0 || !mana_str.is_empty() {
            parts.push(CostPart::Mana(mana_cost));
        }
    }

    // Sort by payment order
    parts.sort_by_key(|p| p.payment_order());

    Cost { parts, has_tap }
}

/// Parse `"amount/filter"` inner content, returning (amount, filter).
/// If there's no slash, defaults to amount=1 and filter=inner.
fn parse_amount_filter(inner: &str) -> (i32, String) {
    if let Some(slash_idx) = inner.find('/') {
        let amt = inner[..slash_idx].parse::<i32>().unwrap_or(1);
        // Strip any trailing description (second slash)
        let rest = &inner[slash_idx + 1..];
        let filter = if let Some(desc_idx) = rest.find('/') {
            rest[..desc_idx].to_string()
        } else {
            rest.to_string()
        };
        (amt, filter)
    } else {
        (1, inner.to_string())
    }
}

/// Split cost string on spaces, keeping `<...>` groups together.
fn split_cost_tokens(raw: &str) -> Vec<&str> {
    let mut tokens = Vec::new();
    let mut start = 0;
    let mut depth = 0;
    let bytes = raw.as_bytes();

    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'<' => depth += 1,
            b'>' => {
                if depth > 0 {
                    depth -= 1;
                }
            }
            b' ' if depth == 0 => {
                let token = raw[start..i].trim();
                if !token.is_empty() {
                    tokens.push(token);
                }
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    // Last token
    let token = raw[start..].trim();
    if !token.is_empty() {
        tokens.push(token);
    }
    tokens
}

/// Find valid sacrifice targets on the battlefield for a player, filtered by type.
/// Mirrors Java's `CostSacrifice.getMaxAmountX()` + `CardLists.getValidCards()`.
pub fn get_sacrifice_targets(game: &GameState, player: PlayerId, type_filter: &str) -> Vec<CardId> {
    game.cards_in_zone(ZoneType::Battlefield, player)
        .to_vec()
        .into_iter()
        .filter(|&cid| matches_change_type(game.card(cid), type_filter, &[]))
        .collect()
}

/// Find valid exile/return targets in a given zone for a player, filtered by type.
pub fn get_zone_targets(
    game: &GameState,
    player: PlayerId,
    zone: ZoneType,
    type_filter: &str,
) -> Vec<CardId> {
    game.cards_in_zone(zone, player)
        .to_vec()
        .into_iter()
        .filter(|&cid| {
            if type_filter == "Card" || type_filter.is_empty() {
                true
            } else {
                matches_change_type(game.card(cid), type_filter, &[])
            }
        })
        .collect()
}

/// Find cards in exile across all players matching a type filter.
/// Used by ExiledMoveToGrave and can_pay checks.
pub fn get_exiled_targets(game: &GameState, type_filter: &str) -> Vec<CardId> {
    game.players
        .iter()
        .flat_map(|p| game.cards_in_zone(ZoneType::Exile, p.id).to_vec())
        .filter(|&cid| {
            type_filter == "Card"
                || type_filter.is_empty()
                || matches_change_type(game.card(cid), type_filter, &[])
        })
        .collect()
}

/// Find valid tap-type targets (untapped permanents matching filter, excluding source).
pub fn get_tap_type_targets(
    game: &GameState,
    player: PlayerId,
    type_filter: &str,
    exclude: CardId,
) -> Vec<CardId> {
    game.cards_in_zone(ZoneType::Battlefield, player)
        .to_vec()
        .into_iter()
        .filter(|&cid| {
            if cid == exclude {
                return false;
            }
            let card = game.card(cid);
            if card.tapped {
                return false;
            }
            if type_filter == "Card" || type_filter.is_empty() {
                true
            } else {
                matches_change_type(card, type_filter, &[])
            }
        })
        .collect()
}

/// Check if a cost can be paid by the given player for the given source card.
/// `available_mana` is the total mana available (pool + untapped sources).
pub fn can_pay(
    cost: &Cost,
    game: &GameState,
    available_mana: &ManaPool,
    source: CardId,
    player: PlayerId,
) -> bool {
    can_pay_inner(cost, game, Some(available_mana), source, player)
}

/// Check if a cost can be paid ignoring mana requirements.
/// Used for mana ability availability checks (to avoid circular dependency).
pub fn can_pay_ignoring_mana(
    cost: &Cost,
    game: &GameState,
    source: CardId,
    player: PlayerId,
) -> bool {
    can_pay_inner(cost, game, None, source, player)
}

/// Shared implementation for cost payability checks.
/// When `available_mana` is None, mana costs are skipped.
fn can_pay_inner(
    cost: &Cost,
    game: &GameState,
    available_mana: Option<&ManaPool>,
    source: CardId,
    player: PlayerId,
) -> bool {
    let card = game.card(source);

    for part in &cost.parts {
        match part {
            CostPart::Tap => {
                if card.tapped {
                    return false;
                }
                if card.is_creature() && card.summoning_sick && !card.has_haste() {
                    return false;
                }
            }
            CostPart::Untap => {
                if !card.tapped {
                    return false;
                }
            }
            CostPart::Mana(mana_cost) => {
                if let Some(pool) = available_mana {
                    if !pool.can_pay(mana_cost) {
                        return false;
                    }
                }
            }
            CostPart::PayLife(amount) => {
                // Static abilities (Platinum Emperion, etc.) can prevent life payment.
                if cant_pay_life(game, player, true, None) {
                    return false;
                }
                // Player needs at least `amount` life — paying lethal life is legal in MTG.
                if game.player(player).life < *amount {
                    return false;
                }
            }
            CostPart::Sacrifice {
                type_filter,
                amount,
            } => {
                if type_filter == "CARDNAME" {
                    if card.zone != ZoneType::Battlefield {
                        return false;
                    }
                } else {
                    let targets = get_sacrifice_targets(game, player, type_filter);
                    if (targets.len() as i32) < *amount {
                        return false;
                    }
                }
            }
            CostPart::Discard {
                type_filter,
                amount,
            } => {
                if type_filter == "CARDNAME" {
                    // Discard the source card itself — it must be in hand.
                    if card.zone != ZoneType::Hand {
                        return false;
                    }
                } else if type_filter == "Card" || type_filter.is_empty() {
                    // Any card — just need enough cards in hand.
                    let hand_size = game.cards_in_zone(ZoneType::Hand, player).len() as i32;
                    if hand_size < *amount {
                        return false;
                    }
                } else {
                    // Type-filtered discard — count matching cards in hand.
                    // Mirrors Java CostDiscard.getMaxAmountX() filtering by getType().
                    let matching = game
                        .cards_in_zone(ZoneType::Hand, player)
                        .iter()
                        .filter(|&&cid| matches_change_type(game.card(cid), type_filter, &[]))
                        .count() as i32;
                    if matching < *amount {
                        return false;
                    }
                }
            }
            CostPart::SubCounter {
                amount,
                counter_type,
            } => {
                if card.zone != ZoneType::Battlefield {
                    return false;
                }
                if card.counter_count(counter_type) < *amount {
                    return false;
                }
            }
            CostPart::AddCounter { .. } => {
                // AddCounter (put counter on source) is always payable if source is on battlefield.
                if card.zone != ZoneType::Battlefield {
                    return false;
                }
            }
            CostPart::Exile { amount, type_filter, from } => {
                if type_filter == "CARDNAME" {
                    if card.zone != *from {
                        return false;
                    }
                } else {
                    let targets = get_zone_targets(game, player, *from, type_filter);
                    if (targets.len() as i32) < *amount {
                        return false;
                    }
                }
            }
            CostPart::Return { amount, type_filter } => {
                if type_filter == "CARDNAME" {
                    if card.zone != ZoneType::Battlefield {
                        return false;
                    }
                } else {
                    let targets = get_sacrifice_targets(game, player, type_filter);
                    if (targets.len() as i32) < *amount {
                        return false;
                    }
                }
            }
            CostPart::TapType { amount, type_filter } => {
                let targets = get_tap_type_targets(game, player, type_filter, source);
                if (targets.len() as i32) < *amount {
                    return false;
                }
            }
            CostPart::UntapType { amount, type_filter } => {
                // Untap tapped permanents matching type
                let count = game
                    .cards_in_zone(ZoneType::Battlefield, player)
                    .iter()
                    .filter(|&&cid| {
                        cid != source
                            && game.card(cid).tapped
                            && (type_filter == "Card"
                                || type_filter.is_empty()
                                || matches_change_type(game.card(cid), type_filter, &[]))
                    })
                    .count() as i32;
                if count < *amount {
                    return false;
                }
            }
            CostPart::PayEnergy(amount) => {
                if game.player(player).energy_counters < *amount {
                    return false;
                }
            }
            CostPart::DamageYou(_) => {
                // Mirrors Java CostDamage.canPay() — always returns true.
                // The player may die as a state-based action after payment; that's legal.
            }
            CostPart::Draw(_) => {
                // Drawing is always possible (library may be empty but that's a loss condition)
            }
            CostPart::Mill(_) => {
                // Same as draw: always considered payable
            }
            CostPart::Reveal { amount, type_filter } => {
                let count = get_zone_targets(game, player, ZoneType::Hand, type_filter).len() as i32;
                if count < *amount {
                    return false;
                }
            }
            CostPart::Exert => {
                // Exerting self: source must be on battlefield and not already exerted.
                if card.zone != ZoneType::Battlefield {
                    return false;
                }
            }
            CostPart::GainLife(_) => {
                // Opponent gaining life is always payable
            }
            CostPart::GainControl { amount, type_filter } => {
                // Scan all players' battlefields — can gain control of any matching permanent.
                // Mirrors Java CostGainControl.canPay() which calls getCardsIn(ZoneType.Battlefield)
                // across all players.
                let count = game
                    .players
                    .iter()
                    .flat_map(|p| game.cards_in_zone(ZoneType::Battlefield, p.id))
                    .filter(|&&cid| matches_change_type(game.card(cid), type_filter, &[]))
                    .count() as i32;
                if count < *amount {
                    return false;
                }
            }
            CostPart::RemoveAnyCounter { amount, type_filter, counter_type } => {
                // Sum counters of the given type (or any type) across all matching permanents
                let total: i32 = game
                    .cards_in_zone(ZoneType::Battlefield, player)
                    .iter()
                    .filter(|&&cid| {
                        type_filter == "Permanent"
                            || type_filter.is_empty()
                            || matches_change_type(game.card(cid), type_filter, &[])
                    })
                    .map(|&cid| {
                        let c = game.card(cid);
                        match counter_type {
                            Some(ct) => c.counter_count(ct),
                            None => c.counters.values().sum(),
                        }
                    })
                    .sum();
                if total < *amount {
                    return false;
                }
            }
            CostPart::Unattach => {
                // Source must be on the battlefield and currently equipping something
                if card.zone != ZoneType::Battlefield {
                    return false;
                }
                if card.attached_to.is_none() {
                    return false;
                }
            }
            CostPart::ExileFromAnyGrave { amount, type_filter } => {
                // Cards in ANY player's graveyard matching filter.
                let count = game
                    .players
                    .iter()
                    .flat_map(|p| game.cards_in_zone(ZoneType::Graveyard, p.id))
                    .filter(|&&cid| {
                        type_filter == "Card"
                            || type_filter.is_empty()
                            || matches_change_type(game.card(cid), type_filter, &[])
                    })
                    .count() as i32;
                if count < *amount {
                    return false;
                }
            }
            CostPart::ExiledMoveToGrave { amount, type_filter } => {
                // Count cards in exile across all players matching the filter
                let exiled = get_exiled_targets(game, type_filter).len() as i32;
                if exiled < *amount {
                    return false;
                }
            }
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tap_only() {
        let cost = parse_cost("T");
        assert!(cost.has_tap);
        assert_eq!(cost.parts.len(), 1);
        assert!(matches!(cost.parts[0], CostPart::Tap));
    }

    #[test]
    fn parse_mana_only() {
        let cost = parse_cost("1 G");
        assert!(!cost.has_tap);
        assert_eq!(cost.parts.len(), 1);
        match &cost.parts[0] {
            CostPart::Mana(mc) => assert_eq!(mc.cmc(), 2),
            _ => panic!("expected Mana cost part"),
        }
    }

    #[test]
    fn parse_tap_and_mana() {
        let cost = parse_cost("T 1 G");
        assert!(cost.has_tap);
        assert_eq!(cost.parts.len(), 2);
        // Tap should come first (payment_order = -1)
        assert!(matches!(cost.parts[0], CostPart::Tap));
        assert!(matches!(cost.parts[1], CostPart::Mana(_)));
    }

    #[test]
    fn parse_sacrifice() {
        let cost = parse_cost("Sac<1/CARDNAME>");
        assert_eq!(cost.parts.len(), 1);
        match &cost.parts[0] {
            CostPart::Sacrifice {
                amount,
                type_filter,
            } => {
                assert_eq!(*amount, 1);
                assert_eq!(type_filter, "CARDNAME");
            }
            _ => panic!("expected Sacrifice cost part"),
        }
    }

    #[test]
    fn parse_pay_life() {
        let cost = parse_cost("PayLife<3>");
        assert_eq!(cost.parts.len(), 1);
        match &cost.parts[0] {
            CostPart::PayLife(n) => assert_eq!(*n, 3),
            _ => panic!("expected PayLife cost part"),
        }
    }

    #[test]
    fn parse_compound_cost() {
        let cost = parse_cost("T Sac<1/CARDNAME>");
        assert!(cost.has_tap);
        assert_eq!(cost.parts.len(), 2);
        // Tap first (order -1), then sacrifice (order 15)
        assert!(matches!(cost.parts[0], CostPart::Tap));
        assert!(matches!(cost.parts[1], CostPart::Sacrifice { .. }));
    }

    #[test]
    fn parse_sacrifice_creature() {
        let cost = parse_cost("B Sac<1/Creature>");
        assert_eq!(cost.parts.len(), 2);
        // Mana (order 0) before Sacrifice (order 15)
        assert!(matches!(cost.parts[0], CostPart::Mana(_)));
        match &cost.parts[1] {
            CostPart::Sacrifice {
                amount,
                type_filter,
            } => {
                assert_eq!(*amount, 1);
                assert_eq!(type_filter, "Creature");
            }
            _ => panic!("expected Sacrifice cost part"),
        }
    }

    #[test]
    fn payment_order_sorting() {
        // PayLife, Tap, Mana, Sacrifice — should sort to: Tap, Mana, PayLife, Sacrifice
        let cost = parse_cost("PayLife<2> T 1 G Sac<1/CARDNAME>");
        assert_eq!(cost.parts.len(), 4);
        assert!(matches!(cost.parts[0], CostPart::Tap));
        assert!(matches!(cost.parts[1], CostPart::Mana(_)));
        assert!(matches!(cost.parts[2], CostPart::PayLife(_)));
        assert!(matches!(cost.parts[3], CostPart::Sacrifice { .. }));
    }

    #[test]
    fn parse_exile_from_hand() {
        let cost = parse_cost("ExileFromHand<1/Card>");
        assert_eq!(cost.parts.len(), 1);
        match &cost.parts[0] {
            CostPart::Exile { amount, type_filter, from } => {
                assert_eq!(*amount, 1);
                assert_eq!(type_filter, "Card");
                assert_eq!(*from, ZoneType::Hand);
            }
            _ => panic!("expected Exile cost part"),
        }
    }

    #[test]
    fn parse_add_counter() {
        let cost = parse_cost("AddCounter<1/LOYALTY>");
        assert_eq!(cost.parts.len(), 1);
        match &cost.parts[0] {
            CostPart::AddCounter { amount, counter_type } => {
                assert_eq!(*amount, 1);
                assert_eq!(*counter_type, CounterType::Loyalty);
            }
            _ => panic!("expected AddCounter cost part"),
        }
    }

    #[test]
    fn parse_return() {
        let cost = parse_cost("Return<1/CARDNAME>");
        assert_eq!(cost.parts.len(), 1);
        match &cost.parts[0] {
            CostPart::Return { amount, type_filter } => {
                assert_eq!(*amount, 1);
                assert_eq!(type_filter, "CARDNAME");
            }
            _ => panic!("expected Return cost part"),
        }
    }

    #[test]
    fn parse_tap_type() {
        let cost = parse_cost("tapXType<2/Creature>");
        assert_eq!(cost.parts.len(), 1);
        match &cost.parts[0] {
            CostPart::TapType { amount, type_filter } => {
                assert_eq!(*amount, 2);
                assert_eq!(type_filter, "Creature");
            }
            _ => panic!("expected TapType cost part"),
        }
    }

    #[test]
    fn parse_pay_energy() {
        let cost = parse_cost("PayEnergy<3>");
        assert_eq!(cost.parts.len(), 1);
        match &cost.parts[0] {
            CostPart::PayEnergy(n) => assert_eq!(*n, 3),
            _ => panic!("expected PayEnergy cost part"),
        }
    }
}
