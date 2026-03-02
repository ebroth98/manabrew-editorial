use forge_foundation::mana::ManaAtom;
use forge_foundation::{ManaCost, ManaCostShard};
use indexmap::IndexMap;
use std::collections::HashMap;

use crate::cost::{can_pay_ignoring_mana, CostPart};
use crate::game::GameState;
use crate::ids::{CardId, PlayerId};

use super::mana_cost_being_paid::{can_pay_for_shard_with_color, ManaCostBeingPaid};
use super::{
    all_basic_subtype_atoms, atom_short, basic_land_mana_atom, produced_to_atoms, tap_land_for_mana,
    ManaPool,
};

#[derive(Debug, Clone)]
struct ManaAbilityRef {
    card_id: CardId,
    ability_index: Option<usize>,
    atoms: Vec<u16>,
    amount: i32,
    mana_text: String,
    source_order: usize,
}

impl ManaAbilityRef {
    fn can_pay_shard(&self, shard: ManaCostShard) -> bool {
        self.atoms
            .iter()
            .any(|&a| can_pay_for_shard_with_color(shard, a))
    }
}

/// Auto-tap lands to produce the required mana.
/// Mirrors Java ComputerUtilMana flow: groupSourcesByManaColor ->
/// groupAndOrderToPayShards -> sortManaAbilities -> getNextShardToPay -> chooseManaAbility.
pub fn auto_tap_lands(
    game: &mut GameState,
    pool: &mut ManaPool,
    player: PlayerId,
    cost: &ManaCost,
    current_spell: Option<CardId>,
) -> Vec<CardId> {
    let mut tapped_lands: Vec<CardId> = Vec::new();

    let mut unpaid = ManaCostBeingPaid::from_mana_cost(cost);
    pay_cost_from_pool(&mut unpaid, pool);
    if unpaid.is_paid() {
        return tapped_lands;
    }

    let colors_most_common = colors_most_common_in_hand(game, player, current_spell);
    let mana_ability_map = group_sources_by_mana_color(game, player);
    if mana_ability_map.is_empty() {
        return tapped_lands;
    }

    let mut sources_for_shards = group_and_order_to_pay_shards(&mana_ability_map, &unpaid);
    if sources_for_shards.is_empty() {
        return tapped_lands;
    }

    sort_mana_abilities(
        game,
        player,
        current_spell,
        &mut sources_for_shards,
        &colors_most_common,
    );

    while !unpaid.is_paid() {
        let Some(to_pay) = get_next_shard_to_pay(&unpaid, &sources_for_shards) else {
            break;
        };

        let ma_list = sources_for_shards.get(&to_pay).cloned().unwrap_or_default();
        if ma_list.is_empty() {
            break;
        }

        let Some(sa_payment) = choose_mana_ability(current_spell, to_pay, &ma_list) else {
            break;
        };

        let Some(chosen_atom) = choose_atom_for_shard(&sa_payment, to_pay, &colors_most_common) else {
            break;
        };

        tap_land_for_mana(
            game,
            pool,
            player,
            sa_payment.card_id,
            chosen_atom,
            &mut tapped_lands,
        );

        let _ = unpaid.try_pay_mana(chosen_atom, chosen_atom as u8);
        for _ in 1..sa_payment.amount.max(1) {
            pool.add(chosen_atom, 1);
            let _ = unpaid.try_pay_mana(chosen_atom, chosen_atom as u8);
        }

        // Lands can only be used once after tap; remove all entries from same host card.
        for abilities in sources_for_shards.values_mut() {
            abilities.retain(|a| a.card_id != sa_payment.card_id);
        }
    }

    tapped_lands
}

fn pay_cost_from_pool(unpaid: &mut ManaCostBeingPaid, pool: &ManaPool) {
    let colors = [
        (ManaAtom::WHITE, pool.white),
        (ManaAtom::BLUE, pool.blue),
        (ManaAtom::BLACK, pool.black),
        (ManaAtom::RED, pool.red),
        (ManaAtom::GREEN, pool.green),
        (ManaAtom::COLORLESS, pool.colorless),
    ];

    for (atom, count) in colors {
        for _ in 0..count.max(0) {
            if unpaid.is_paid() {
                return;
            }
            let _ = unpaid.try_pay_mana(atom, atom as u8);
        }
    }
}

fn get_next_shard_to_pay(
    unpaid: &ManaCostBeingPaid,
    sources_for_shards: &IndexMap<ManaCostShard, Vec<ManaAbilityRef>>,
) -> Option<ManaCostShard> {
    let mut shards_to_pay = unpaid.get_distinct_shards();
    shards_to_pay.sort_by_key(|shard| sources_for_shards.get(shard).map_or(0, |v| v.len()));
    unpaid.get_shard_to_pay_by_priority(&shards_to_pay, ManaAtom::COLORS_SUPERPOSITION as u8)
}

fn choose_mana_ability(
    current_spell: Option<CardId>,
    to_pay: ManaCostShard,
    ma_list: &[ManaAbilityRef],
) -> Option<ManaAbilityRef> {
    for ma in ma_list {
        // Java avoids paying from the same host card when selecting payment ability.
        if Some(ma.card_id) == current_spell {
            continue;
        }
        if ma.can_pay_shard(to_pay) {
            return Some(ma.clone());
        }
    }
    None
}

fn choose_atom_for_shard(
    mana_ab: &ManaAbilityRef,
    shard: ManaCostShard,
    colors_most_common: &[u16],
) -> Option<u16> {
    if shard.is_colorless() {
        if mana_ab.atoms.contains(&ManaAtom::COLORLESS) {
            return Some(ManaAtom::COLORLESS);
        }
    }

    if shard == ManaCostShard::Generic || shard.is_generic() {
        let _ = colors_most_common;
        return mana_ab.atoms.first().copied();
    }

    mana_ab
        .atoms
        .iter()
        .copied()
        .find(|&a| can_pay_for_shard_with_color(shard, a))
}

fn group_and_order_to_pay_shards(
    mana_ability_map: &IndexMap<i32, Vec<ManaAbilityRef>>,
    cost: &ManaCostBeingPaid,
) -> IndexMap<ManaCostShard, Vec<ManaAbilityRef>> {
    let mut res: IndexMap<ManaCostShard, Vec<ManaAbilityRef>> = IndexMap::new();

    if (cost.get_generic_mana_amount() > 0 || cost.has_any_kind(ManaAtom::OR_2_GENERIC))
        && mana_ability_map.contains_key(&(ManaAtom::GENERIC as i32))
    {
        res.insert(
            ManaCostShard::Generic,
            mana_ability_map
                .get(&(ManaAtom::GENERIC as i32))
                .cloned()
                .unwrap_or_default(),
        );
    }

    for shard in cost.get_distinct_shards() {
        if shard.is_or_2_generic() {
            let color_key = shard.color_mask() as i32;
            if let Some(list) = mana_ability_map.get(&color_key) {
                res.entry(shard).or_default().extend(list.clone());
            }
            if let Some(list) = mana_ability_map.get(&(ManaAtom::GENERIC as i32)) {
                res.entry(shard).or_default().extend(list.clone());
            }
            continue;
        }

        if shard == ManaCostShard::Generic {
            continue;
        }

        for (color_key, list) in mana_ability_map {
            let key_color = (*color_key as u16) & (ManaAtom::COLORS_SUPERPOSITION | ManaAtom::COLORLESS);
            if can_pay_for_shard_with_color(shard, key_color) {
                let bucket = res.entry(shard).or_default();
                for ma in list {
                    if !bucket
                        .iter()
                        .any(|x| x.card_id == ma.card_id && x.ability_index == ma.ability_index)
                    {
                        bucket.push(ma.clone());
                    }
                }
            }
        }
    }

    res
}

fn sort_mana_abilities(
    game: &GameState,
    player: PlayerId,
    current_spell: Option<CardId>,
    mana_ability_map: &mut IndexMap<ManaCostShard, Vec<ManaAbilityRef>>,
    colors_most_common: &[u16],
) {
    let mut mana_card_score: HashMap<CardId, i32> = HashMap::new();
    let mut ordered_cards: Vec<CardId> = Vec::new();

    for abilities in mana_ability_map.values() {
        for ability in abilities {
            if mana_card_score.contains_key(&ability.card_id) {
                continue;
            }
            let score = score_mana_producing_card(game, ability.card_id, player);
            mana_card_score.insert(ability.card_id, score);
            ordered_cards.push(ability.card_id);
        }
    }

    ordered_cards.sort_by_key(|cid| mana_card_score.get(cid).copied().unwrap_or(0));

    let shards: Vec<ManaCostShard> = mana_ability_map.keys().copied().collect();
    for shard in shards {
        let Some(existing) = mana_ability_map.get(&shard).cloned() else {
            continue;
        };
        let mut new_abilities = existing.clone();
        let existing_index: HashMap<(CardId, Option<usize>), usize> = existing
            .iter()
            .enumerate()
            .map(|(i, a)| ((a.card_id, a.ability_index), i))
            .collect();

        new_abilities.sort_by(|a, b| {
            let idx_a = ordered_cards.iter().position(|&c| c == a.card_id).unwrap_or(usize::MAX);
            let idx_b = ordered_cards.iter().position(|&c| c == b.card_id).unwrap_or(usize::MAX);
            let mut pre_order = (idx_a as isize) - (idx_b as isize);

            if pre_order != 0 {
                if shard.is_generic()
                    && mana_card_score.get(&a.card_id) == mana_card_score.get(&b.card_id)
                {
                    for &col in colors_most_common {
                        let a_can = a.atoms.contains(&col);
                        let b_can = b.atoms.contains(&col);
                        if a_can && !b_can {
                            return std::cmp::Ordering::Greater;
                        }
                        if !a_can && b_can {
                            return std::cmp::Ordering::Less;
                        }
                    }
                }

                let a_pos = existing_index
                    .get(&(a.card_id, a.ability_index))
                    .copied()
                    .unwrap_or(usize::MAX);
                let b_pos = existing_index
                    .get(&(b.card_id, b.ability_index))
                    .copied()
                    .unwrap_or(usize::MAX);
                pre_order += (a_pos as isize) - (b_pos as isize);
                return pre_order.cmp(&0);
            }

            let shard_mana = shard.short_string();
            let pay_with_a = a.mana_text.contains(shard_mana);
            let pay_with_b = b.mana_text.contains(shard_mana);
            if pay_with_a && !pay_with_b {
                return std::cmp::Ordering::Less;
            }
            if pay_with_b && !pay_with_a {
                return std::cmp::Ordering::Greater;
            }

            a.ability_index
                .cmp(&b.ability_index)
                .then(a.source_order.cmp(&b.source_order))
        });

        // Java excludes same-host payment in chooseManaAbility, keep list intact here.
        let _ = current_spell;
        mana_ability_map.insert(shard, new_abilities);
    }
}

fn group_sources_by_mana_color(game: &GameState, player: PlayerId) -> IndexMap<i32, Vec<ManaAbilityRef>> {
    let mut mana_map: IndexMap<i32, Vec<ManaAbilityRef>> = IndexMap::new();
    let mut source_order = 0usize;

    for card_id in get_available_mana_sources(game, player) {
        let card = game.card(card_id);
        let mut explicit_mana_added = false;

        for ab in &card.activated_abilities {
            if !ab.is_mana_ability {
                continue;
            }
            if ab.cost.parts.iter().any(|p| matches!(p, CostPart::Mana(_))) {
                continue;
            }
            if !can_pay_ignoring_mana(&ab.cost, game, card_id, player) {
                continue;
            }
            let Some(produced) = ab.params.get("Produced") else {
                continue;
            };
            if produced == "Combo ColorIdentity" {
                continue;
            }

            let atoms = produced_to_atoms(produced, &card.chosen_colors);
            if atoms.is_empty() {
                continue;
            }

            explicit_mana_added = true;
            let ma = ManaAbilityRef {
                card_id,
                ability_index: Some(ab.ability_index),
                atoms: atoms.clone(),
                amount: parse_mana_ability_amount(ab),
                mana_text: ability_mana_text_for_score(produced, &card.chosen_colors),
                source_order,
            };
            source_order += 1;
            add_mana_ability_to_color_map(&mut mana_map, &ma);
        }

        if !explicit_mana_added {
            let mut atoms = all_basic_subtype_atoms(card);
            if atoms.is_empty() {
                if let Some(a) = basic_land_mana_atom(card) {
                    atoms.push(a);
                }
            }
            for atom in atoms {
                let ma = ManaAbilityRef {
                    card_id,
                    ability_index: None,
                    atoms: vec![atom],
                    amount: 1,
                    mana_text: atom_short(atom).to_string(),
                    source_order,
                };
                source_order += 1;
                add_mana_ability_to_color_map(&mut mana_map, &ma);
            }
        }
    }

    mana_map
}

fn add_mana_ability_to_color_map(map: &mut IndexMap<i32, Vec<ManaAbilityRef>>, ma: &ManaAbilityRef) {
    map.entry(ManaAtom::GENERIC as i32)
        .or_default()
        .push(ma.clone());

    for &atom in &ma.atoms {
        map.entry(atom as i32).or_default().push(ma.clone());
    }
}

fn get_available_mana_sources(game: &GameState, player: PlayerId) -> Vec<CardId> {
    let battlefield: Vec<CardId> = game
        .cards_in_zone(forge_foundation::ZoneType::Battlefield, player)
        .iter()
        .copied()
        .filter(|&cid| !game.card(cid).tapped)
        .collect();

    let mut other_sources: Vec<CardId> = Vec::new();
    let mut colorless_sources: Vec<CardId> = Vec::new();
    let mut one_sources: Vec<CardId> = Vec::new();
    let mut two_sources: Vec<CardId> = Vec::new();
    let mut three_sources: Vec<CardId> = Vec::new();
    let mut four_sources: Vec<CardId> = Vec::new();
    let mut five_plus_sources: Vec<CardId> = Vec::new();
    let mut any_color_sources: Vec<CardId> = Vec::new();

    for cid in battlefield {
        let card = game.card(cid);

        let mut has_any_mana_ability = false;
        let mut usable_mana_abilities = 0usize;
        let mut produces_any_color = false;
        let mut unique_atoms: Vec<u16> = Vec::new();

        for ab in &card.activated_abilities {
            if !ab.is_mana_ability {
                continue;
            }
            has_any_mana_ability = true;
            if ab.cost.parts.iter().any(|p| matches!(p, CostPart::Mana(_))) {
                continue;
            }
            if !can_pay_ignoring_mana(&ab.cost, game, cid, player) {
                continue;
            }
            let Some(produced) = ab.params.get("Produced") else {
                continue;
            };
            if produced.eq_ignore_ascii_case("Any") {
                produces_any_color = true;
            }
            for atom in produced_to_atoms(produced, &card.chosen_colors) {
                if !unique_atoms.contains(&atom) {
                    unique_atoms.push(atom);
                }
            }
            usable_mana_abilities += 1;
        }

        if usable_mana_abilities == 0 {
            if card.is_land() && !has_any_mana_ability {
                unique_atoms = all_basic_subtype_atoms(card);
                if unique_atoms.is_empty() {
                    if let Some(a) = basic_land_mana_atom(card) {
                        unique_atoms.push(a);
                    }
                }
                usable_mana_abilities = unique_atoms.len();
            }
        }

        if usable_mana_abilities == 0 {
            continue;
        }

        if card.is_creature() {
            other_sources.push(cid);
        } else if produces_any_color {
            any_color_sources.push(cid);
        } else if usable_mana_abilities <= 1 {
            if unique_atoms.len() == 1 && unique_atoms[0] == ManaAtom::COLORLESS {
                colorless_sources.push(cid);
            } else {
                one_sources.push(cid);
            }
        } else if usable_mana_abilities == 2 {
            two_sources.push(cid);
        } else if usable_mana_abilities == 3 {
            three_sources.push(cid);
        } else if usable_mana_abilities == 4 {
            four_sources.push(cid);
        } else {
            five_plus_sources.push(cid);
        }
    }

    let mut sorted: Vec<CardId> = Vec::new();
    sorted.extend(colorless_sources);
    sorted.extend(one_sources);
    sorted.extend(two_sources);
    sorted.extend(three_sources);
    sorted.extend(four_sources);
    sorted.extend(five_plus_sources);
    sorted.extend(any_color_sources);
    sorted.extend(other_sources);
    sorted
}

fn colors_most_common_in_hand(
    game: &GameState,
    player: PlayerId,
    current_spell: Option<CardId>,
) -> Vec<u16> {
    let mut max_pips = [0_i32; 5];
    for &card_id in game.cards_in_zone(forge_foundation::ZoneType::Hand, player) {
        if Some(card_id) == current_spell {
            continue;
        }
        let card = game.card(card_id);
        if card.is_land() {
            continue;
        }

        let mut pips = [0_i32; 5];
        for shard in card.mana_cost.shards() {
            let atoms = shard.shard() & ManaAtom::COLORS_SUPERPOSITION;
            if (atoms & ManaAtom::WHITE) != 0 {
                pips[0] += 1;
            }
            if (atoms & ManaAtom::BLUE) != 0 {
                pips[1] += 1;
            }
            if (atoms & ManaAtom::BLACK) != 0 {
                pips[2] += 1;
            }
            if (atoms & ManaAtom::RED) != 0 {
                pips[3] += 1;
            }
            if (atoms & ManaAtom::GREEN) != 0 {
                pips[4] += 1;
            }
        }

        for i in 0..5 {
            max_pips[i] = max_pips[i].max(pips[i]);
        }
    }

    let mut ordered = vec![
        (ManaAtom::WHITE, max_pips[0]),
        (ManaAtom::BLUE, max_pips[1]),
        (ManaAtom::BLACK, max_pips[2]),
        (ManaAtom::RED, max_pips[3]),
        (ManaAtom::GREEN, max_pips[4]),
    ];

    ordered.sort_by(|a, b| b.1.cmp(&a.1));
    ordered
        .into_iter()
        .filter_map(|(atom, count)| if count > 0 { Some(atom) } else { None })
        .collect()
}

fn score_mana_producing_card(game: &GameState, card_id: CardId, player: PlayerId) -> i32 {
    let card = game.card(card_id);
    let mut score = 0;
    let mut has_mana_ability = false;

    for ab in &card.activated_abilities {
        if ab.is_mana_ability {
            let produced = ab.params.get("Produced").map(|s| s.as_str());
            score += score_mana_ability(game, card_id, ab, produced);
            has_mana_ability = true;
        } else if can_pay_ignoring_mana(&ab.cost, game, card_id, player) {
            score += 13;
        }
    }

    if !has_mana_ability && card.is_land() {
        let mut subtype_atoms = all_basic_subtype_atoms(card);
        if subtype_atoms.is_empty() {
            if let Some(a) = basic_land_mana_atom(card) {
                subtype_atoms.push(a);
            }
        }
        for atom in subtype_atoms {
            score += score_implicit_land_mana_ability(atom);
        }
    }

    if card.can_attack() {
        score += 13;
    }
    if card.can_block() {
        score += 13;
    }

    score
}

fn score_mana_ability(
    game: &GameState,
    card_id: CardId,
    ab: &crate::ability::activated::ActivatedAbility,
    produced_override: Option<&str>,
) -> i32 {
    let mut score = 0;
    let card = game.card(card_id);

    if let Some(produced) = produced_override.or_else(|| ab.params.get("Produced").map(String::as_str))
    {
        let mana_text = ability_mana_text_for_score(produced, &card.chosen_colors);
        if mana_text == "Any" {
            score += 7;
        } else {
            score += mana_text.len() as i32;
            if !mana_text.contains('C') {
                score += 1;
            }
        }
    } else {
        score += 1;
    }

    for part in &ab.cost.parts {
        match part {
            CostPart::PayLife(_) => score += 3,
            CostPart::Sacrifice { type_filter, .. } => {
                score += 6;
                if type_filter != "CARDNAME" {
                    score += 40;
                }
            }
            CostPart::Discard { .. } => score += 6,
            _ => {}
        }
        score += 1;
    }

    score
}

fn score_implicit_land_mana_ability(atom: u16) -> i32 {
    let mut score = 0;
    let text = atom_short(atom);
    score += text.len() as i32;
    if atom != ManaAtom::COLORLESS {
        score += 1;
    }
    score += 1;
    score
}

fn ability_mana_text_for_score(produced: &str, chosen_colors: &[String]) -> String {
    if produced.eq_ignore_ascii_case("Any") {
        return "Any".to_string();
    }

    let atoms = produced_to_atoms(produced, chosen_colors);
    if atoms.is_empty() {
        return String::new();
    }

    atoms
        .into_iter()
        .map(atom_short)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Auto-tap untapped lands to produce `needed` additional generic mana.
/// Used for paying commander tax on top of the regular cost.
pub fn auto_tap_lands_generic(
    game: &mut GameState,
    pool: &mut ManaPool,
    player: PlayerId,
    needed: i32,
) -> Vec<CardId> {
    let deficit = (needed - pool.total()).max(0);
    if deficit <= 0 {
        return Vec::new();
    }

    let mut remaining = deficit;
    let mut tapped_lands: Vec<CardId> = Vec::new();

    for card_id in get_available_mana_sources(game, player) {
        if remaining <= 0 {
            break;
        }
        let card = game.card(card_id);
        let mut atoms = all_basic_subtype_atoms(card);
        if atoms.is_empty() {
            if let Some(a) = basic_land_mana_atom(card) {
                atoms.push(a);
            }
        }

        let atom = if atoms.contains(&ManaAtom::COLORLESS) {
            ManaAtom::COLORLESS
        } else {
            atoms.first().copied().unwrap_or(ManaAtom::COLORLESS)
        };

        tap_land_for_mana(game, pool, player, card_id, atom, &mut tapped_lands);
        remaining -= 1;
    }

    tapped_lands
}
fn parse_mana_ability_amount(ab: &crate::ability::activated::ActivatedAbility) -> i32 {
    ab.params
        .get("Amount")
        .and_then(|s| s.parse::<i32>().ok())
        .filter(|&n| n > 0)
        .unwrap_or(1)
}
