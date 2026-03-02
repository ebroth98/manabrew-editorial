use super::*;

impl GameLoop {
    fn describe_priority_action(
        &self,
        game: &GameState,
        action: MainPhaseAction,
        ability_idx: Option<usize>,
    ) -> String {
        let card_name_or_id = |card_id: CardId| -> String {
            game.cards
                .get(card_id.index())
                .map(|c| c.card_name.clone())
                .unwrap_or_else(|| format!("CardId({})", card_id.0))
        };
        match action {
            MainPhaseAction::Pass => "Pass".to_string(),
            MainPhaseAction::Play(card_id) => {
                format!("Play {}", card_name_or_id(card_id))
            }
            MainPhaseAction::ActivateMana(card_id) => {
                format!("Activate mana ({})", card_name_or_id(card_id))
            }
            MainPhaseAction::UntapMana(card_id) => {
                format!("Untap mana ({})", card_name_or_id(card_id))
            }
            MainPhaseAction::ActivateAbility(card_id, _) => {
                let idx = ability_idx.unwrap_or_default();
                format!("Activate ability {} ({})", idx, card_name_or_id(card_id))
            }
        }
    }

    pub fn priority_round(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        is_main_phase: bool,
    ) {
        let mut priority_player = game.active_player();
        let mut passed_count = 0;
        let num_players = game.players.len();

        while passed_count < num_players {
            if game.game_over {
                return;
            }
            self.with_shared_state_mutation(game, agents, |_this, game, _agents| {
                game.turn.priority_player = priority_player;
            });

            // Check SBA before any player gets priority
            loop {
                if !game.check_state_based_actions_with_triggers(Some(&mut self.trigger_handler)) {
                    break;
                }
            }
            if game.game_over {
                return;
            }

            // A player can play sorcery-speed cards if:
            // - It's their own turn
            // - It's a main phase
            // - The stack is empty
            let can_play_sorcery =
                is_main_phase && priority_player == game.active_player() && game.stack.is_empty();
            let must_be_instant = !can_play_sorcery;

            let playable = self.get_playable_cards(game, priority_player, must_be_instant);

            let tappable_lands: Vec<CardId> = game
                .cards_in_zone(ZoneType::Battlefield, priority_player)
                .to_vec()
                .into_iter()
                .filter(|&cid| {
                    let c = game.card(cid);
                    c.is_land() && !c.tapped
                })
                .collect();

            let pool_snapshot = self.pool(priority_player).clone();
            let untappable_lands: Vec<CardId> = game
                .cards_in_zone(ZoneType::Battlefield, priority_player)
                .to_vec()
                .into_iter()
                .filter(|&cid| {
                    let c = game.card(cid);
                    if !c.is_land() || !c.tapped {
                        return false;
                    }
                    // A tapped land is untappable if the pool has ANY atom it could produce
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

            let activatable = self.get_activatable_abilities(game, priority_player);

            agents[priority_player.index()].snapshot_state(game, &self.mana_pools);
            self.log_waiting_for_priority(game, priority_player);
            let action = agents[priority_player.index()].choose_action(
                priority_player,
                &playable,
                &tappable_lands,
                &untappable_lands,
                &activatable,
            );

            match action {
                MainPhaseAction::Pass => {
                    self.log_priority_pass(game, priority_player);
                    passed_count += 1;
                    priority_player = game.next_player(priority_player);
                    self.with_shared_state_mutation(game, agents, |_this, game, _agents| {
                        game.turn.priority_player = priority_player;
                    });
                }
                MainPhaseAction::Play(card_id) => {
                    self.log_priority_response(
                        game,
                        priority_player,
                        &self.describe_priority_action(game, action, None),
                    );
                    if !playable.contains(&card_id) {
                        agents[priority_player.index()]
                            .notify("Illegal action ignored: unplayable card");
                        passed_count += 1;
                        priority_player = game.next_player(priority_player);
                        self.with_shared_state_mutation(game, agents, |_this, game, _agents| {
                            game.turn.priority_player = priority_player;
                        });
                        continue;
                    }
                    let played =
                        self.with_shared_state_mutation(game, agents, |this, game, agents| {
                            this.play_card(game, agents, priority_player, card_id)
                        });
                    if let Some((played_id, played_name)) = played {
                        let set_code = game.card(played_id).set_code.clone().unwrap_or_default();
                        for agent in agents.iter_mut() {
                            agent.snapshot_state(game, &self.mana_pools);
                            agent.notify_card_played(
                                priority_player,
                                played_id,
                                &played_name,
                                &set_code,
                            );
                        }
                        // Process SpellCast / BecomesTarget triggers immediately so they
                        // go on the stack ABOVE the spell (resolving before it).
                        // Mirrors Java's MagicStack.addAndUnfreeze() which runs waiting
                        // triggers right after the spell is placed on the stack.
                        self.with_shared_state_mutation(game, agents, |this, game, agents| {
                            this.process_triggers(game, agents);
                        });
                        passed_count = 0;
                    } else {
                        // Payment failed — treat as a pass to avoid infinite retry loop
                        agents[priority_player.index()]
                            .notify("Card play failed (insufficient mana)");
                        passed_count += 1;
                        priority_player = game.next_player(priority_player);
                        self.with_shared_state_mutation(game, agents, |_this, game, _agents| {
                            game.turn.priority_player = priority_player;
                        });
                    }
                }
                MainPhaseAction::ActivateMana(land_id) => {
                    self.log_priority_response(
                        game,
                        priority_player,
                        &self.describe_priority_action(game, action, None),
                    );
                    if !tappable_lands.contains(&land_id) {
                        agents[priority_player.index()]
                            .notify("Illegal action ignored: land can't tap for mana");
                        passed_count += 1;
                        priority_player = game.next_player(priority_player);
                        self.with_shared_state_mutation(game, agents, |_this, game, _agents| {
                            game.turn.priority_player = priority_player;
                        });
                        continue;
                    }
                    // Use the card's actual mana abilities when available;
                    // fall back to basic_land_mana_atom only for lands with no abilities.
                    let mana_ab = {
                        let c = game.card(land_id);
                        c.activated_abilities
                            .iter()
                            .find(|ab| ab.is_mana_ability)
                            .cloned()
                    };
                    if let Some(ab) = mana_ab {
                        self.with_shared_state_mutation(game, agents, |this, game, agents| {
                            this.resolve_mana_ability(game, agents, priority_player, land_id, &ab);
                        });
                    } else {
                        // Legacy fallback for lands with no parsed mana abilities
                        self.with_shared_state_mutation(game, agents, |this, game, _agents| {
                            let atom_opt = {
                                let c = game.card(land_id);
                                if c.is_land() && !c.tapped {
                                    basic_land_mana_atom(c)
                                } else {
                                    None
                                }
                            };
                            if let Some(atom) = atom_opt {
                                game.tap(land_id);
                                this.pool_mut(priority_player).add(atom, 1);
                                this.trigger_handler.run_trigger(
                                    TriggerType::Taps,
                                    RunParams {
                                        card: Some(land_id),
                                        player: Some(priority_player),
                                        ..Default::default()
                                    },
                                    false,
                                );
                                this.trigger_handler.run_trigger(
                                    TriggerType::TapsForMana,
                                    RunParams {
                                        card: Some(land_id),
                                        player: Some(priority_player),
                                        ..Default::default()
                                    },
                                    false,
                                );
                            }
                        });
                    }
                    passed_count = 0;
                }
                MainPhaseAction::UntapMana(land_id) => {
                    self.log_priority_response(
                        game,
                        priority_player,
                        &self.describe_priority_action(game, action, None),
                    );
                    if !untappable_lands.contains(&land_id) {
                        agents[priority_player.index()].notify(
                            "Illegal action ignored: land can't be untapped for mana rollback",
                        );
                        passed_count += 1;
                        priority_player = game.next_player(priority_player);
                        self.with_shared_state_mutation(game, agents, |_this, game, _agents| {
                            game.turn.priority_player = priority_player;
                        });
                        continue;
                    }
                    self.with_shared_state_mutation(game, agents, |this, game, _agents| {
                        let atoms = {
                            let c = game.card(land_id);
                            if c.is_land() && c.tapped {
                                let a = mana::land_mana_atoms(c);
                                if a.is_empty() {
                                    // Fallback for lands without mana abilities
                                    basic_land_mana_atom(c).into_iter().collect::<Vec<_>>()
                                } else {
                                    a
                                }
                            } else {
                                vec![]
                            }
                        };
                        if !atoms.is_empty() {
                            game.untap(land_id);
                            // Remove the first atom we find in the pool
                            let pool = this.pool_mut(priority_player);
                            for &atom in &atoms {
                                if pool.has_atom(atom, 1) {
                                    pool.remove(atom, 1);
                                    break;
                                }
                            }
                            // Fire Untaps trigger
                            this.trigger_handler.run_trigger(
                                TriggerType::Untaps,
                                RunParams {
                                    card: Some(land_id),
                                    player: Some(priority_player),
                                    ..Default::default()
                                },
                                false,
                            );
                        }
                    });
                    passed_count = 0;
                }
                MainPhaseAction::ActivateAbility(card_id, ability_idx) => {
                    self.log_priority_response(
                        game,
                        priority_player,
                        &self.describe_priority_action(game, action, Some(ability_idx)),
                    );
                    if !activatable.contains(&(card_id, ability_idx)) {
                        agents[priority_player.index()]
                            .notify("Illegal action ignored: ability not activatable");
                        passed_count += 1;
                        priority_player = game.next_player(priority_player);
                        self.with_shared_state_mutation(game, agents, |_this, game, _agents| {
                            game.turn.priority_player = priority_player;
                        });
                        continue;
                    }
                    self.with_shared_state_mutation(game, agents, |this, game, agents| {
                        this.activate_ability(game, agents, priority_player, card_id, ability_idx);
                    });
                    passed_count = 0;
                }
            }
        }
        self.with_shared_state_mutation(game, agents, |_this, game, _agents| {
            game.turn.priority_player = game.active_player();
        });
    }
}
