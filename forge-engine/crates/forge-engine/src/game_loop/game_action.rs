use super::*;

impl GameLoop {
    pub(crate) fn get_activatable_abilities(
        &self,
        game: &GameState,
        player: PlayerId,
    ) -> Vec<(CardId, usize)> {
        let mut result = Vec::new();
        let available_mana = mana::calculate_available_mana(self.pool(player), game, player);
        let battlefield = game.cards_in_zone(ZoneType::Battlefield, player).to_vec();

        for card_id in battlefield {
            let card = game.card(card_id);
            for ab in &card.activated_abilities {
                if can_pay(&ab.cost, game, &available_mana, card_id, player) {
                    result.push((card_id, ab.ability_index));
                }
            }
        }

        result
    }

    /// Activate an ability on a permanent.
    pub(crate) fn activate_ability(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        card_id: CardId,
        ability_idx: usize,
    ) {
        // Clone the ability data we need before mutating game
        let ab = {
            let card = game.card(card_id);
            card.activated_abilities
                .iter()
                .find(|a| a.ability_index == ability_idx)
                .cloned()
        };

        let ab = match ab {
            Some(ab) => ab,
            None => return,
        };

        if ab.is_mana_ability {
            self.resolve_mana_ability(game, agents, player, card_id, &ab);
        } else {
            self.activate_ability_on_stack(game, agents, player, card_id, &ab);
        }
    }

    /// Pay the cost parts of an activated ability (tap, mana, life, sacrifice).
    /// Mirrors Java's `CostPayment.payCost()` iterating over `CostPart`s.
    pub(crate) fn pay_ability_cost(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        card_id: CardId,
        cost: &crate::cost::Cost,
    ) {
        for part in &cost.parts {
            match part {
                CostPart::Tap => {
                    game.tap(card_id);
                    // Fire Taps trigger
                    self.trigger_handler.run_trigger(
                        TriggerType::Taps,
                        RunParams {
                            card: Some(card_id),
                            player: Some(player),
                            ..Default::default()
                        },
                        false,
                    );
                }
                CostPart::Mana(mana_cost) => {
                    mana::auto_tap_lands(
                        game,
                        &mut self.mana_pools[player.index()],
                        player,
                        mana_cost,
                    );
                    self.mana_pools[player.index()].try_pay(mana_cost);
                }
                CostPart::PayLife(amount) => {
                    game.player_mut(player).lose_life(*amount);
                    // Fire LifeLost trigger
                    self.trigger_handler.run_trigger(
                        TriggerType::LifeLost,
                        RunParams {
                            player: Some(player),
                            life_amount: Some(*amount),
                            ..Default::default()
                        },
                        false,
                    );
                }
                CostPart::Sacrifice {
                    type_filter,
                    amount,
                } => {
                    if type_filter == "CARDNAME" {
                        let owner = game.card(card_id).owner;
                        // Fire Sacrificed trigger before moving
                        self.trigger_handler.run_trigger(
                            TriggerType::Sacrificed,
                            RunParams {
                                card: Some(card_id),
                                player: Some(player),
                                ..Default::default()
                            },
                            false,
                        );
                        game.move_card(card_id, ZoneType::Graveyard, owner);
                    } else {
                        self.pay_sacrifice_cost(game, agents, player, type_filter, *amount);
                    }
                }
            }
        }
    }

    /// Pay additional costs from an SP$ ability line (non-mana cost parts only).
    /// Used during spell casting for costs like `Sac<1/Creature>`.
    /// Mirrors Java's `CostPayment.payCost()` for spell abilities.
    pub(crate) fn pay_additional_costs(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        _card_id: CardId,
        spell_cost: &crate::cost::Cost,
    ) {
        for part in &spell_cost.parts {
            match part {
                // Mana is already paid by play_card's main mana payment flow
                CostPart::Mana(_) | CostPart::Tap => {}
                CostPart::PayLife(amount) => {
                    game.player_mut(player).lose_life(*amount);
                    // Fire LifeLost trigger
                    self.trigger_handler.run_trigger(
                        TriggerType::LifeLost,
                        RunParams {
                            player: Some(player),
                            life_amount: Some(*amount),
                            ..Default::default()
                        },
                        false,
                    );
                }
                CostPart::Sacrifice {
                    type_filter,
                    amount,
                } => {
                    if type_filter != "CARDNAME" {
                        self.pay_sacrifice_cost(game, agents, player, type_filter, *amount);
                    }
                }
            }
        }
    }

    /// Pay a sacrifice cost by prompting the agent to choose targets.
    /// Mirrors Java's `CostSacrifice.doListPayment()` which calls
    /// `GameAction.sacrifice()`.
    pub(crate) fn pay_sacrifice_cost(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        type_filter: &str,
        amount: i32,
    ) {
        for _ in 0..amount {
            let valid = cost::get_sacrifice_targets(game, player, type_filter);
            if valid.is_empty() {
                break;
            }
            if let Some(chosen) = agents[player.index()].choose_sacrifice(player, &valid) {
                let owner = game.card(chosen).owner;
                // Fire Sacrificed trigger before moving
                self.trigger_handler.run_trigger(
                    TriggerType::Sacrificed,
                    RunParams {
                        card: Some(chosen),
                        player: Some(player),
                        ..Default::default()
                    },
                    false,
                );
                game.move_card(chosen, ZoneType::Graveyard, owner);
                crate::ability::effects::emit_zone_trigger(
                    &mut self.trigger_handler,
                    chosen,
                    ZoneType::Battlefield,
                    ZoneType::Graveyard,
                );
            }
        }
    }

    /// Resolve a mana ability immediately (no stack).
    pub(crate) fn resolve_mana_ability(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        card_id: CardId,
        ab: &crate::ability::activated::ActivatedAbility,
    ) {
        self.pay_ability_cost(game, agents, player, card_id, &ab.cost);

        // Produce mana
        if let Some(produced) = ab.params.get("Produced") {
            if produced == "Combo ColorIdentity" {
                // Commander mana: look up the commander's color identity.
                let command_cards = game.cards_in_zone(ZoneType::Command, player).to_vec();
                let colors: Vec<String> = command_cards
                    .iter()
                    .find_map(|&cid| {
                        let c = game.card(cid);
                        if c.is_commander {
                            let cols: Vec<String> = c
                                .color
                                .iter()
                                .map(|col| capitalize_color(col.long_name()))
                                .collect();
                            if cols.is_empty() {
                                None
                            } else {
                                Some(cols)
                            }
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| {
                        // Fallback for non-commander games: all 5 colors
                        vec![
                            "White".to_string(),
                            "Blue".to_string(),
                            "Black".to_string(),
                            "Red".to_string(),
                            "Green".to_string(),
                        ]
                    });

                if let Some(chosen) = agents[player.index()].choose_color(player, &colors) {
                    if let Some(atom) = color_name_to_mana_atom(&chosen) {
                        self.pool_mut(player).add(atom, 1);
                    }
                }
            } else if produced.starts_with("Combo") {
                // Combo mana: player chooses one color from the combo
                let colors = parse_combo_colors(produced);
                if !colors.is_empty() {
                    if let Some(chosen) = agents[player.index()].choose_color(player, &colors) {
                        if let Some(atom) = color_name_to_mana_atom(&chosen) {
                            self.pool_mut(player).add(atom, 1);
                        }
                    }
                }
            } else if let Some(atom) = mana_atom_from_produced(produced) {
                self.pool_mut(player).add(atom, 1);
            }
        }

        // Resolve SubAbility chain (e.g. DealDamage on pain lands)
        if let Some(sub_svar_name) = ab.params.get("SubAbility") {
            if let Some(sub_text) = game.card(card_id).svars.get(sub_svar_name).cloned() {
                let sub_sa = SpellAbility::new_simple(Some(card_id), player, &sub_text);
                self.resolve_single_effect(game, agents, &sub_sa, None);
            }
        }

        // Fire TapsForMana trigger (mana abilities produce mana)
        self.trigger_handler.run_trigger(
            TriggerType::TapsForMana,
            RunParams {
                card: Some(card_id),
                player: Some(player),
                ..Default::default()
            },
            false,
        );
    }

    /// Activate a non-mana ability: choose targets, pay costs, put on stack.
    pub(crate) fn activate_ability_on_stack(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        card_id: CardId,
        ab: &crate::ability::activated::ActivatedAbility,
    ) {
        let ability_text = ab.ability_text.clone();

        // Build SpellAbility and choose targets
        let mut sa = SpellAbility::new_simple(Some(card_id), player, &ability_text);
        sa.is_activated = true;
        sa.setup_targets(game, agents, &self.mana_pools);

        // Fire BecomesTarget trigger if a card was targeted
        if let Some(target_card) = sa.target_chosen.target_card {
            self.trigger_handler.run_trigger(
                TriggerType::BecomesTarget,
                RunParams {
                    card: Some(target_card),
                    cause_player: Some(player),
                    ..Default::default()
                },
                false,
            );
        }

        // Pay costs
        self.pay_ability_cost(game, agents, player, card_id, &ab.cost);

        // Push to stack
        let card_name = game.card(card_id).card_name.clone();
        let entry = StackEntry {
            id: 0,
            spell_ability: sa,
            is_creature_spell: false,
            is_permanent_spell: false,
            cast_from_zone: None,
        };
        game.stack.push(entry);
        self.log_stack_push(&format!("{} ability", card_name), &game.player(player).name);
        agents[player.index()].notify(&format!("Activated ability of {}", card_name));

        // Fire AbilityActivated trigger
        self.trigger_handler.run_trigger(
            TriggerType::AbilityActivated,
            RunParams {
                card: Some(card_id),
                player: Some(player),
                ..Default::default()
            },
            false,
        );
    }
}
