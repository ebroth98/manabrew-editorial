use super::*;
use crate::cost::CostPart;

pub(crate) struct ActionSpace {
    pub playable: Vec<crate::agent::PlayOption>,
    pub tappable_lands: Vec<CardId>,
    pub untappable_lands: Vec<CardId>,
    pub activatable: Vec<(CardId, usize)>,
}

impl GameLoop {
    pub(crate) fn mana_source_available_for_payment(
        game: &GameState,
        player: PlayerId,
        card_id: CardId,
    ) -> bool {
        let card = game.card(card_id);
        let summoning_sick = card.is_creature() && card.summoning_sick && !card.has_haste();

        let has_usable_mana_ability = card.activated_abilities.iter().any(|ab| {
            let needs_tap = ab
                .cost
                .parts
                .iter()
                .any(|part| matches!(part, CostPart::Tap));
            ab.is_mana_ability
                && (!card.tapped || !needs_tap)
                && (!summoning_sick || !needs_tap)
                && crate::cost::can_pay_ignoring_mana(&ab.cost, game, card_id, player)
        });

        (card.is_land() && !card.tapped) || has_usable_mana_ability
    }

    pub(crate) fn action_space(
        &self,
        game: &GameState,
        player: PlayerId,
        is_main_phase: bool,
    ) -> ActionSpace {
        let can_play_sorcery =
            is_main_phase && player == game.active_player() && game.stack.is_empty();
        let must_be_instant = !can_play_sorcery;

        let playable = self.get_playable_cards(game, player, must_be_instant);
        let activatable: Vec<(CardId, usize)> = self
            .get_activatable_abilities(game, player, can_play_sorcery)
            .into_iter()
            .filter(|&(card_id, ability_idx)| {
                game.card(card_id)
                    .activated_abilities
                    .iter()
                    .find(|ab| ab.ability_index == ability_idx)
                    .map(|ab| !ab.is_mana_ability)
                    .unwrap_or(false)
            })
            .collect();

        let tappable_lands: Vec<CardId> = game
            .cards_in_zone(ZoneType::Battlefield, player)
            .iter()
            .copied()
            .filter(|&cid| Self::mana_source_available_for_payment(game, player, cid))
            .collect();

        let pool_snapshot = self.pool(player).clone();
        let untappable_lands: Vec<CardId> = game
            .cards_in_zone(ZoneType::Battlefield, player)
            .iter()
            .copied()
            .filter(|&cid| {
                let c = game.card(cid);
                if !c.tapped {
                    return false;
                }
                // Must be a land or a permanent with a mana ability.
                let has_mana_ability =
                    c.is_land() || c.activated_abilities.iter().any(|ab| ab.is_mana_ability);
                if !has_mana_ability {
                    return false;
                }
                let atoms = mana::land_mana_atoms(c);
                if !atoms.is_empty() {
                    atoms.iter().any(|&a| pool_snapshot.has_atom(a, 1))
                } else if let Some(atom) = basic_land_mana_atom(c) {
                    pool_snapshot.has_atom(atom, 1)
                } else {
                    false
                }
            })
            .collect();

        ActionSpace {
            playable,
            tappable_lands,
            untappable_lands,
            activatable,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::activated::parse_activated_ability;
    use crate::card::Card;
    use forge_foundation::{CardTypeLine, ColorSet, ManaCost};

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
            ManaCost::zero(),
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
    fn tapped_non_tap_mana_source_is_available_for_payment() {
        let player = PlayerId(0);
        let mut game = GameState::new(&["P1", "P2"], 20);
        let spawn = game.create_card(make_card(
            1,
            player,
            "Eldrazi Spawn Token",
            "Creature Eldrazi Spawn",
            vec!["AB$ Mana | Cost$ Sac<1/CARDNAME> | Produced$ C | Amount$ 1"],
        ));

        game.zone_mut(ZoneType::Battlefield, player).add(spawn);
        game.card_mut(spawn).zone = ZoneType::Battlefield;
        game.card_mut(spawn).tapped = true;

        assert!(GameLoop::mana_source_available_for_payment(
            &game, player, spawn
        ));
    }
}
