use forge_engine_core::agent::ManaCostAction;
use forge_engine_core::cost::{can_pay_ignoring_mana, CostPart};
use forge_engine_core::game::GameState;
use forge_engine_core::ids::{CardId, PlayerId};
use forge_engine_core::mana::mana_cost_being_paid::{
    can_pay_for_shard_with_color, ManaCostBeingPaid,
};
use forge_engine_core::mana::{basic_land_mana_atom, produced_to_atoms, ManaPool};
use forge_engine_core::parsing::keys;
use forge_foundation::mana::ManaAtom;
use forge_foundation::{ManaCost, ManaCostShard, ZoneType};

pub struct NextManaCostAction {
    pub action: ManaCostAction,
}

#[derive(Clone)]
struct ManaAbilityCandidate {
    card_id: CardId,
    ability_index: Option<usize>,
    atoms: Vec<u16>,
    mana_text: String,
    source_order: usize,
}

impl ManaAbilityCandidate {
    fn can_pay_shard(&self, shard: ManaCostShard) -> bool {
        if shard == ManaCostShard::Generic {
            return true;
        }
        self.atoms
            .iter()
            .any(|&atom| can_pay_for_shard_with_color(shard, atom))
    }

    fn chosen_atom_for_shard(&self, shard: ManaCostShard) -> Option<u16> {
        if shard == ManaCostShard::Generic || shard.is_generic() {
            return self.atoms.first().copied();
        }
        self.atoms
            .iter()
            .copied()
            .find(|&atom| can_pay_for_shard_with_color(shard, atom))
            .or_else(|| self.atoms.first().copied())
    }
}

/// Parse callback-facing mana strings (e.g. "{1}{G}") into Forge token format.
pub fn parse_callback_mana_cost(mana_cost: &str) -> ManaCost {
    if mana_cost.contains('{') {
        let normalized = mana_cost.replace("}{", " ").replace(['{', '}'], "");
        ManaCost::parse(normalized.trim())
    } else {
        ManaCost::parse(mana_cost)
    }
}

pub fn next_mana_cost_action(
    game: &GameState,
    mana_pool: &ManaPool,
    player: PlayerId,
    source: CardId,
    mana_cost: &str,
    _allow_reserved_source_reuse: bool,
) -> NextManaCostAction {
    let cost = parse_callback_mana_cost(mana_cost);
    let mut unpaid = ManaCostBeingPaid::from_mana_cost(&cost);
    pay_cost_from_pool(&mut unpaid, mana_pool);

    if unpaid.is_paid() {
        return NextManaCostAction {
            action: ManaCostAction::Pay,
        };
    }

    let candidates = collect_playable_mana_abilities(game, player, source);
    if let Some((candidate, chosen_atom)) = choose_candidate(&unpaid, &candidates) {
        return NextManaCostAction {
            action: ManaCostAction::TapLand {
                card_id: candidate.card_id,
                mana_ability_index: candidate.ability_index,
                express_choice: Some(chosen_atom),
            },
        };
    }

    if unpaid.contains_only_phyrexian_mana()
        && game.player(player).life >= required_phyrexian_life(&unpaid)
    {
        return NextManaCostAction {
            action: ManaCostAction::Pay,
        };
    }

    NextManaCostAction {
        action: ManaCostAction::Cancel,
    }
}

fn choose_candidate(
    unpaid: &ManaCostBeingPaid,
    candidates: &[ManaAbilityCandidate],
) -> Option<(ManaAbilityCandidate, u16)> {
    for shard in shard_priority(unpaid, candidates) {
        if let Some(candidate) = choose_least_versatile_candidate(candidates, shard, unpaid) {
            if let Some(chosen_atom) = candidate.chosen_atom_for_shard(shard) {
                return Some((candidate, chosen_atom));
            }
        }
    }
    None
}

fn shard_priority(
    unpaid: &ManaCostBeingPaid,
    candidates: &[ManaAbilityCandidate],
) -> Vec<ManaCostShard> {
    let mut colored = Vec::new();
    let mut generic = None;

    for shard in unpaid.get_distinct_shards() {
        if shard == ManaCostShard::X {
            continue;
        }
        if shard == ManaCostShard::Generic {
            generic = Some(shard);
        } else if !colored.contains(&shard) {
            colored.push(shard);
        }
    }

    colored.sort_by_key(|&shard| {
        candidates
            .iter()
            .filter(|candidate| candidate.can_pay_shard(shard))
            .count()
    });

    if let Some(generic_shard) = generic {
        colored.push(generic_shard);
    }

    colored
}

fn choose_least_versatile_candidate(
    candidates: &[ManaAbilityCandidate],
    shard: ManaCostShard,
    unpaid: &ManaCostBeingPaid,
) -> Option<ManaAbilityCandidate> {
    let mut fallback = None;

    for candidate in candidates {
        if !candidate.can_pay_shard(shard) {
            continue;
        }
        if fallback.is_none() {
            fallback = Some(candidate.clone());
        }
        if !is_sole_source_for_other_shard(candidate, shard, candidates, unpaid) {
            return Some(candidate.clone());
        }
    }

    fallback
}

fn is_sole_source_for_other_shard(
    candidate: &ManaAbilityCandidate,
    current_shard: ManaCostShard,
    candidates: &[ManaAbilityCandidate],
    unpaid: &ManaCostBeingPaid,
) -> bool {
    for other in unpaid.get_distinct_shards() {
        if other == current_shard || other == ManaCostShard::Generic || other == ManaCostShard::X {
            continue;
        }
        if !candidate.can_pay_shard(other) {
            continue;
        }
        let sources_for_other = candidates
            .iter()
            .filter(|other_candidate| other_candidate.can_pay_shard(other))
            .count();
        if sources_for_other <= 1 {
            return true;
        }
    }
    false
}

fn collect_playable_mana_abilities(
    game: &GameState,
    player: PlayerId,
    source_being_paid: CardId,
) -> Vec<ManaAbilityCandidate> {
    let mut candidates = Vec::new();
    let mut source_order = 0usize;

    for &card_id in game.cards_in_zone(ZoneType::Battlefield, player) {
        if card_id == source_being_paid {
            continue;
        }
        let card = game.card(card_id);
        let mut explicit_mana_added = false;

        for ab in &card.activated_abilities {
            if !ab.is_mana_ability {
                continue;
            }
            if ab
                .cost
                .parts
                .iter()
                .any(|part| matches!(part, CostPart::Mana { .. }))
            {
                continue;
            }
            if !can_pay_ignoring_mana(&ab.cost, game, card_id, player) {
                continue;
            }

            if ab.params.get(keys::AB) == Some("ManaReflected") {
                let atoms = compute_reflected_atoms(game, player, card_id, ab);
                if atoms.is_empty() {
                    continue;
                }
                explicit_mana_added = true;
                candidates.push(ManaAbilityCandidate {
                    card_id,
                    ability_index: Some(ab.ability_index),
                    atoms,
                    mana_text: "Any".to_string(),
                    source_order,
                });
                source_order += 1;
                continue;
            }

            let Some(produced) = ab.params.get(keys::PRODUCED) else {
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
            candidates.push(ManaAbilityCandidate {
                card_id,
                ability_index: Some(ab.ability_index),
                atoms,
                mana_text: ability_mana_text_for_score(produced, &card.chosen_colors),
                source_order,
            });
            source_order += 1;
        }

        if !explicit_mana_added && card.is_land() && !card.tapped {
            let mut atoms = all_basic_subtype_atoms(card);
            if atoms.is_empty() {
                if let Some(atom) = basic_land_mana_atom(card) {
                    atoms.push(atom);
                }
            }
            for atom in atoms {
                candidates.push(ManaAbilityCandidate {
                    card_id,
                    ability_index: None,
                    atoms: vec![atom],
                    mana_text: atom_short(atom).to_string(),
                    source_order,
                });
                source_order += 1;
            }
        }
    }

    candidates.sort_by_key(|candidate| {
        autopay_source_score(game, candidate) * 1000 + candidate.source_order as i32
    });
    candidates
}

fn autopay_source_score(game: &GameState, candidate: &ManaAbilityCandidate) -> i32 {
    let card = game.card(candidate.card_id);
    let mut score = 0;

    if candidate.mana_text == "Any" {
        score += 7;
    } else {
        score += candidate.mana_text.split_whitespace().count() as i32;
        if !candidate.mana_text.contains('C') {
            score += 1;
        }
    }

    if let Some(ability_index) = candidate.ability_index {
        if let Some(ab) = card.activated_abilities.get(ability_index) {
            score += ab.cost.parts.len() as i32;
            if ab
                .cost
                .parts
                .iter()
                .any(|part| matches!(part, CostPart::Sacrifice { .. }))
            {
                // Preserve sacrifice fodder like Spawn/Treasure whenever a
                // non-sacrificial source can pay the same shard.
                score += 50;
            }
        }
    } else {
        score += 1;
    }

    if card.is_creature() {
        if card.can_attack() {
            score += 13;
        }
        if card.can_block() {
            score += 13;
        }
    }

    score
}

fn ability_mana_text_for_score(produced: &str, chosen_colors: &[String]) -> String {
    if produced.eq_ignore_ascii_case("Any") {
        return "Any".to_string();
    }

    let atoms = produced_to_atoms(produced, chosen_colors);
    atoms
        .into_iter()
        .map(atom_short)
        .collect::<Vec<_>>()
        .join(" ")
}

fn all_basic_subtype_atoms(card: &forge_engine_core::card::Card) -> Vec<u16> {
    let mut atoms = Vec::new();
    let subtypes = [
        ("Plains", ManaAtom::WHITE),
        ("Island", ManaAtom::BLUE),
        ("Swamp", ManaAtom::BLACK),
        ("Mountain", ManaAtom::RED),
        ("Forest", ManaAtom::GREEN),
    ];
    for (subtype, atom) in subtypes {
        if card.type_line.has_subtype(subtype) {
            atoms.push(atom);
        }
    }
    atoms
}

fn compute_reflected_atoms(
    game: &GameState,
    player: PlayerId,
    source_id: CardId,
    ab: &forge_engine_core::ability::activated::ActivatedAbility,
) -> Vec<u16> {
    let reflect_prop = ab.params.get(keys::REFLECT_PROPERTY).unwrap_or("Is");
    let valid = ab.params.get(keys::VALID).unwrap_or("Card");
    let include_colorless = ab.params.get(keys::COLOR_OR_TYPE) == Some("Type");
    let mut reflected = Vec::new();

    for &other_id in game.cards_in_zone(ZoneType::Battlefield, player) {
        if other_id == source_id {
            continue;
        }
        let other = game.card(other_id);
        let matches = if valid.contains("Land") {
            other.is_land() && other.controller == player
        } else {
            other.controller == player
        };
        if !matches {
            continue;
        }

        if reflect_prop == "Produce" {
            for other_ab in &other.activated_abilities {
                if !other_ab.is_mana_ability {
                    continue;
                }
                if let Some(produced) = other_ab.params.get(keys::PRODUCED) {
                    for atom in produced_to_atoms(produced, &other.chosen_colors) {
                        if !reflected.contains(&atom) {
                            reflected.push(atom);
                        }
                    }
                }
            }
            for atom in all_basic_subtype_atoms(other) {
                if !reflected.contains(&atom) {
                    reflected.push(atom);
                }
            }
            if reflected.is_empty() {
                if let Some(atom) = basic_land_mana_atom(other) {
                    if !reflected.contains(&atom) {
                        reflected.push(atom);
                    }
                }
            }
        } else {
            for &atom in &[
                ManaAtom::WHITE,
                ManaAtom::BLUE,
                ManaAtom::BLACK,
                ManaAtom::RED,
                ManaAtom::GREEN,
            ] {
                if (other.color.mask() as u16) & atom != 0 && !reflected.contains(&atom) {
                    reflected.push(atom);
                }
            }
        }
    }

    if include_colorless && !reflected.contains(&ManaAtom::COLORLESS) {
        reflected.push(ManaAtom::COLORLESS);
    }

    reflected
}

fn atom_short(atom: u16) -> &'static str {
    match atom {
        ManaAtom::WHITE => "W",
        ManaAtom::BLUE => "U",
        ManaAtom::BLACK => "B",
        ManaAtom::RED => "R",
        ManaAtom::GREEN => "G",
        ManaAtom::COLORLESS => "C",
        _ => "C",
    }
}

fn pay_cost_from_pool(unpaid: &mut ManaCostBeingPaid, pool: &ManaPool) {
    let colors = [
        (ManaAtom::WHITE, pool.white()),
        (ManaAtom::BLUE, pool.blue()),
        (ManaAtom::BLACK, pool.black()),
        (ManaAtom::RED, pool.red()),
        (ManaAtom::GREEN, pool.green()),
        (ManaAtom::COLORLESS, pool.colorless()),
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

fn required_phyrexian_life(unpaid: &ManaCostBeingPaid) -> i32 {
    unpaid
        .get_distinct_shards()
        .into_iter()
        .filter(|shard| shard.is_phyrexian())
        .map(|shard| unpaid.get_unpaid_shards(shard) * 2)
        .sum()
}
