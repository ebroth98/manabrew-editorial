use super::*;

pub(crate) struct ActionSpace {
    pub playable: Vec<crate::agent::PlayOption>,
    pub tappable_lands: Vec<CardId>,
    pub untappable_lands: Vec<CardId>,
    pub activatable: Vec<(CardId, usize)>,
}

impl GameLoop {
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
            .filter(|&cid| {
                let c = game.card(cid);
                c.is_land() && !c.tapped
            })
            .collect();

        let pool_snapshot = self.pool(player).clone();
        let untappable_lands: Vec<CardId> = game
            .cards_in_zone(ZoneType::Battlefield, player)
            .iter()
            .copied()
            .filter(|&cid| {
                let c = game.card(cid);
                if !c.is_land() || !c.tapped {
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
