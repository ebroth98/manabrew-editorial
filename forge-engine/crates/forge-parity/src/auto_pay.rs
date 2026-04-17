use forge_engine_core::agent::{ManaAbilityOption, ManaCostAction};
use forge_engine_core::cost::CostPart;
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
    allow_reserved_source_reuse: bool,
    reserved_sacrifices: &[CardId],
    mana_ability_options: &[ManaAbilityOption],
    tappable_lands: &[CardId],
) -> NextManaCostAction {
    let cost = parse_callback_mana_cost(mana_cost);
    let mut unpaid = ManaCostBeingPaid::from_mana_cost(&cost);
    pay_cost_from_pool(&mut unpaid, mana_pool);

    if unpaid.is_paid() {
        return NextManaCostAction {
            action: ManaCostAction::Pay { auto: false },
        };
    }

    let candidates = collect_playable_mana_abilities(
        game,
        player,
        source,
        mana_ability_options,
        tappable_lands,
        reserved_sacrifices,
        allow_reserved_source_reuse,
    );
    if let Some((candidate, chosen_atom)) = choose_candidate(&unpaid, &candidates) {
        // Java's AutoPay only sets expressChoice when the ability is
        // isAnyMana / isComboMana / ManaReflected — i.e. when a single
        // mana ability can produce more than one color. Single-color
        // abilities leave expressChoice null.
        let express_choice = if candidate.atoms.len() > 1 {
            Some(chosen_atom)
        } else {
            None
        };
        return NextManaCostAction {
            action: ManaCostAction::TapLand {
                card_id: candidate.card_id,
                mana_ability_index: candidate.ability_index,
                express_choice,
            },
        };
    }

    let mut solver_pool = mana_pool.clone();
    if solver_pool
        .try_pay_cost_with_phyrexian_life(&cost, false, game.player(player).life)
        .is_some()
    {
        return NextManaCostAction {
            action: ManaCostAction::Pay { auto: false },
        };
    }

    if unpaid.contains_only_phyrexian_mana()
        && game.player(player).life >= required_phyrexian_life(&unpaid)
    {
        return NextManaCostAction {
            action: ManaCostAction::Pay { auto: false },
        };
    }

    NextManaCostAction {
        action: ManaCostAction::Cancel,
    }
}

/// Mirror the Java harness action-space mana feasibility for phyrexian costs.
///
/// This is intentionally greedier than the Rust engine solver: Java's
/// ComputerUtilMana may commit a matching colored source to a phyrexian shard
/// before considering that the same source is needed for generic mana. Keep this
/// confined to parity action enumeration so real gameplay can still use the
/// rules-correct payment path.
pub fn can_pay_action_space_phyrexian_cost(
    game: &GameState,
    mana_pool: &ManaPool,
    player: PlayerId,
    source: CardId,
    cost: &ManaCost,
) -> bool {
    if !cost.has_phyrexian() {
        return true;
    }

    let mut unpaid = ManaCostBeingPaid::from_mana_cost(cost);
    pay_cost_from_pool(&mut unpaid, mana_pool);
    if unpaid.is_paid() {
        return true;
    }

    let sources = forge_engine_core::mana::collect_mana_payment_sources(game, player, &[]);
    let mut candidates = collect_playable_mana_abilities(
        game,
        player,
        source,
        &sources.mana_ability_options,
        &sources.source_cards,
        &[],
        false,
    );
    loop {
        if unpaid.is_paid() {
            return true;
        }

        let Some((candidate, chosen_atom)) = choose_candidate(&unpaid, &candidates) else {
            return unpaid.contains_only_phyrexian_mana()
                && game.player(player).life > required_phyrexian_life(&unpaid);
        };
        if unpaid
            .try_pay_mana(chosen_atom, chosen_atom as u8)
            .is_none()
        {
            return false;
        }
        if let Some(pos) = candidates.iter().position(|existing| {
            existing.card_id == candidate.card_id
                && existing.ability_index == candidate.ability_index
                && existing.source_order == candidate.source_order
        }) {
            candidates.remove(pos);
        } else {
            return false;
        }
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
    mana_ability_options: &[ManaAbilityOption],
    tappable_lands: &[CardId],
    reserved_sacrifices: &[CardId],
    allow_reserved_source_reuse: bool,
) -> Vec<ManaAbilityCandidate> {
    let mut candidates = Vec::new();
    let mut source_order = 0usize;
    let mut explicit_sources = Vec::new();

    for option in mana_ability_options {
        if option.card_id == source_being_paid {
            continue;
        }
        let card = game.card(option.card_id);
        let Some(ab) = card
            .activated_abilities
            .iter()
            .find(|ab| ab.ability_index == option.ability_index)
        else {
            continue;
        };

        if !can_pay_mana_ability_costs_with_reserved(
            game,
            player,
            option.card_id,
            &ab.cost.parts,
            reserved_sacrifices,
            allow_reserved_source_reuse,
        ) {
            continue;
        }

        if ab.params.get(keys::AB) == Some("ManaReflected") {
            let atoms = compute_reflected_atoms(game, player, option.card_id, ab);
            if atoms.is_empty() {
                continue;
            }
            explicit_sources.push(option.card_id);
            candidates.push(ManaAbilityCandidate {
                card_id: option.card_id,
                ability_index: Some(option.ability_index),
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

        explicit_sources.push(option.card_id);
        candidates.push(ManaAbilityCandidate {
            card_id: option.card_id,
            ability_index: Some(option.ability_index),
            atoms,
            mana_text: ability_mana_text_for_score(produced, &card.chosen_colors),
            source_order,
        });
        source_order += 1;
    }

    for &card_id in tappable_lands {
        if card_id == source_being_paid || explicit_sources.contains(&card_id) {
            continue;
        }
        let card = game.card(card_id);
        if !card.is_land() || card.tapped {
            continue;
        }
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

    candidates.sort_by_key(|candidate| {
        autopay_source_score(game, candidate) * 1000 + candidate.source_order as i32
    });
    candidates
}

fn can_pay_mana_ability_costs_with_reserved(
    game: &GameState,
    player: PlayerId,
    source_id: CardId,
    cost_parts: &[CostPart],
    reserved_sacrifices: &[CardId],
    allow_reserved_source_reuse: bool,
) -> bool {
    for part in cost_parts {
        match part {
            CostPart::Tap | CostPart::Mana { .. } => {}
            CostPart::PayLife(amount) => {
                if game.player(player).life < *amount {
                    return false;
                }
            }
            CostPart::SubCounter {
                amount,
                counter_type,
            } => {
                if game.card(source_id).counter_count(counter_type) < *amount {
                    return false;
                }
            }
            CostPart::Sacrifice {
                type_filter,
                amount,
            } => {
                if type_filter == "CARDNAME" {
                    if *amount > 1
                        || game.card(source_id).zone != ZoneType::Battlefield
                        || (reserved_sacrifices.contains(&source_id)
                            && !allow_reserved_source_reuse)
                    {
                        return false;
                    }
                } else {
                    let mut targets = forge_engine_core::cost::get_sacrifice_targets_for_cost(
                        game,
                        player,
                        type_filter,
                        None,
                    );
                    if !allow_reserved_source_reuse {
                        targets.retain(|cid| !reserved_sacrifices.contains(cid));
                    }
                    if (targets.len() as i32) < *amount {
                        return false;
                    }
                }
            }
            _ => return false,
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_engine_core::ability::activated::parse_activated_ability;
    use forge_engine_core::card::Card;
    use forge_foundation::{CardTypeLine, ColorSet};

    fn make_card(
        id: u32,
        owner: PlayerId,
        name: &str,
        type_line: &str,
        abilities: Vec<&str>,
    ) -> Card {
        let mut card = Card::new(
            CardId(id),
            name.to_string(),
            owner,
            CardTypeLine::parse(type_line),
            ManaCost::no_cost(),
            ColorSet::COLORLESS,
            None,
            None,
            vec![],
            vec![],
        );
        card.abilities = abilities.iter().map(|s| s.to_string()).collect();
        card.activated_abilities = abilities
            .iter()
            .enumerate()
            .filter_map(|(i, raw)| parse_activated_ability(raw, i))
            .collect();
        card
    }

    #[test]
    fn activated_cost_payment_can_reuse_reserved_food_for_goose_mana() {
        let player = PlayerId(0);
        let mut game = GameState::new(&["P1", "P2"], 20);
        let mut pool = ManaPool::new();
        pool.add(ManaAtom::GREEN, 1);

        let reserved_food = game.create_card(make_card(
            1,
            player,
            "Food Token",
            "Artifact Food",
            vec!["AB$ GainLife | Cost$ 2 T Sac<1/CARDNAME> | LifeAmount$ 3"],
        ));
        let goose = game.create_card(make_card(
            2,
            player,
            "Gilded Goose",
            "Creature Bird",
            vec!["AB$ Mana | Cost$ T Sac<1/Food> | Produced$ Any"],
        ));
        let forest = game.create_card(make_card(
            3,
            player,
            "Forest",
            "Land Forest",
            vec!["AB$ Mana | Cost$ T | Produced$ G"],
        ));

        for cid in [reserved_food, goose, forest] {
            game.zone_mut(ZoneType::Battlefield, player).add(cid);
            game.card_mut(cid).zone = ZoneType::Battlefield;
            game.card_mut(cid).summoning_sick = false;
        }
        game.card_mut(forest).tapped = true;
        let mana_ability_options = vec![ManaAbilityOption {
            card_id: goose,
            ability_index: 0,
            description: "AB$ Mana | Cost$ T Sac<1/Food> | Produced$ Any".to_string(),
        }];
        let tappable_lands = vec![goose];

        let next = next_mana_cost_action(
            &game,
            &pool,
            player,
            reserved_food,
            "{2}",
            true,
            &[reserved_food],
            &mana_ability_options,
            &tappable_lands,
        );

        match next.action {
            ManaCostAction::TapLand {
                card_id,
                mana_ability_index,
                ..
            } => {
                assert_eq!(game.card(card_id).card_name, "Gilded Goose");
                assert_eq!(mana_ability_index, Some(0));
            }
            other => panic!("expected Goose mana ability, got {:?}", other),
        }
    }

    #[test]
    fn activated_cost_payment_can_reuse_reserved_spawn_for_its_own_mana() {
        let player = PlayerId(0);
        let mut game = GameState::new(&["P1", "P2"], 20);
        let munitions = game.create_card(make_card(
            1,
            player,
            "Makeshift Munitions",
            "Enchantment",
            vec!["AB$ DealDamage | Cost$ 1 Sac<1/Artifact.Creature> | ValidTgts$ Any | NumDmg$ 1"],
        ));
        let spawn = game.create_card(make_card(
            2,
            player,
            "Eldrazi Spawn Token",
            "Creature Eldrazi Spawn",
            vec!["AB$ Mana | Cost$ Sac<1/CARDNAME> | Produced$ C | Amount$ 1"],
        ));

        for cid in [munitions, spawn] {
            game.zone_mut(ZoneType::Battlefield, player).add(cid);
            game.card_mut(cid).zone = ZoneType::Battlefield;
            game.card_mut(cid).summoning_sick = false;
        }
        let mana_ability_options = vec![ManaAbilityOption {
            card_id: spawn,
            ability_index: 0,
            description: "AB$ Mana | Cost$ Sac<1/CARDNAME> | Produced$ C | Amount$ 1".to_string(),
        }];
        let tappable_lands = vec![spawn];

        let next = next_mana_cost_action(
            &game,
            &ManaPool::new(),
            player,
            munitions,
            "{1}",
            true,
            &[spawn],
            &mana_ability_options,
            &tappable_lands,
        );

        match next.action {
            ManaCostAction::TapLand {
                card_id,
                mana_ability_index,
                ..
            } => {
                assert_eq!(card_id, spawn);
                assert_eq!(mana_ability_index, Some(0));
            }
            other => panic!("expected reserved Spawn mana ability, got {:?}", other),
        }
    }

    #[test]
    fn phyrexian_spell_payment_prefers_finishing_with_life_over_cancel() {
        let player = PlayerId(0);
        let game = GameState::new(&["P1", "P2"], 20);
        let mut pool = ManaPool::new();
        pool.add(ManaAtom::BLACK, 1);

        let next = next_mana_cost_action(
            &game,
            &pool,
            player,
            CardId(0),
            "{1}{B/P}{B/P}",
            false,
            &[],
            &[],
            &[],
        );

        match next.action {
            ManaCostAction::Pay { .. } => {}
            other => panic!(
                "expected Pay for Dismember-style phyrexian finish, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn phyrexian_action_space_mirrors_java_greedy_source_choice() {
        let player = PlayerId(0);
        let mut game = GameState::new(&["P1", "P2"], 20);
        let swamp = game.create_card(make_card(
            1,
            player,
            "Swamp",
            "Basic Land Swamp",
            vec!["AB$ Mana | Cost$ T | Produced$ B"],
        ));
        game.zone_mut(ZoneType::Battlefield, player).add(swamp);
        game.card_mut(swamp).zone = ZoneType::Battlefield;

        assert!(!can_pay_action_space_phyrexian_cost(
            &game,
            &ManaPool::new(),
            player,
            CardId(99),
            &ManaCost::parse("1 BP BP"),
        ));
    }

    #[test]
    fn phyrexian_action_space_refuses_life_payment_to_zero() {
        let player = PlayerId(0);
        let mut game = GameState::new(&["P1", "P2"], 2);

        assert!(!can_pay_action_space_phyrexian_cost(
            &game,
            &ManaPool::new(),
            player,
            CardId(99),
            &ManaCost::parse("GP"),
        ));

        game.player_mut(player).life = 3;
        assert!(can_pay_action_space_phyrexian_cost(
            &game,
            &ManaPool::new(),
            player,
            CardId(99),
            &ManaCost::parse("GP"),
        ));
    }

    #[test]
    fn phyrexian_action_space_uses_matching_mana_before_life_floor() {
        let player = PlayerId(0);
        let mut game = GameState::new(&["P1", "P2"], 2);
        let mountain = game.create_card(make_card(
            1,
            player,
            "Mountain",
            "Basic Land Mountain",
            vec!["AB$ Mana | Cost$ T | Produced$ R"],
        ));
        game.zone_mut(ZoneType::Battlefield, player).add(mountain);
        game.card_mut(mountain).zone = ZoneType::Battlefield;

        assert!(can_pay_action_space_phyrexian_cost(
            &game,
            &ManaPool::new(),
            player,
            CardId(99),
            &ManaCost::parse("RP"),
        ));
    }
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
        score += 26;
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
