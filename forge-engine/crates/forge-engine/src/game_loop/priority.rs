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
            MainPhaseAction::Play(play) => {
                format!("Play {}", card_name_or_id(play.card_id))
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

            // Mirrors Java's checkStateBasedEffects():
            //   do { checkStateEffects(); } while (addAllTriggeredAbilitiesToStack());
            // SBA check may cause triggers (e.g. creature dies → death triggers),
            // and processing triggers may cause more SBA (e.g. token creation
            // causing legend rule). Loop until stable.
            loop {
                let sba_changed = super::check_sba(game, &mut self.trigger_handler, agents);
                if game.game_over {
                    return;
                }
                // Process pending triggers and put them on the stack.
                // Mirrors Java's addAllTriggeredAbilitiesToStack() inside
                // checkStateBasedEffects(). This must happen BEFORE the player
                // gets to choose an action, so triggers are already on the stack
                // when the player sees their options.
                let stack_before = game.stack.len();
                self.with_shared_state_mutation(game, agents, |this, game, agents| {
                    this.process_triggers(game, agents);
                });
                let triggers_added = game.stack.len() > stack_before;
                // Keep looping while either SBA changed state or new triggers were added
                if !sba_changed && !triggers_added {
                    break;
                }
            }
            if game.game_over {
                return;
            }

            let action_space = self.action_space(game, priority_player, is_main_phase);

            agents[priority_player.index()].snapshot_state(game, &self.mana_pools);
            self.log_waiting_for_priority(game, priority_player);
            let action = agents[priority_player.index()].choose_action(
                priority_player,
                &action_space.playable,
                &action_space.tappable_lands,
                &action_space.untappable_lands,
                &action_space.activatable,
            );

            if self.apply_pending_snapshot_restore(game, agents) {
                passed_count = 0;
                priority_player = game.turn.priority_player;
                continue;
            }

            match action {
                MainPhaseAction::Pass => {
                    self.log_priority_pass(game, priority_player);
                    passed_count += 1;
                    priority_player = game.next_player(priority_player);
                    self.with_shared_state_mutation(game, agents, |_this, game, _agents| {
                        game.turn.priority_player = priority_player;
                    });
                }
                MainPhaseAction::Play(play) => {
                    self.log_priority_response(
                        game,
                        priority_player,
                        &self.describe_priority_action(game, action, None),
                    );
                    if !action_space.playable.contains(&play) {
                        crate::agent::notify_all_agents(
                            agents,
                            crate::agent::GameLogEvent::warning(
                                "Illegal action ignored: unplayable card",
                            )
                            .with_player(priority_player),
                        );
                        passed_count += 1;
                        priority_player = game.next_player(priority_player);
                        self.with_shared_state_mutation(game, agents, |_this, game, _agents| {
                            game.turn.priority_player = priority_player;
                        });
                        continue;
                    }
                    let played =
                        self.with_shared_state_mutation(game, agents, |this, game, agents| {
                            this.play_card(game, agents, priority_player, play.card_id, play.mode)
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
                        // Cast/setup failed (e.g. no legal targets, auto-tap heuristic
                        // failure, mana restrictions).  Match Java's behaviour: the
                        // player retains priority and gets to choose again (Java's
                        // do-while loops back to chooseSpellAbilityToPlay for the same
                        // player).  Do NOT change passed_count or priority_player.
                        crate::agent::notify_all_agents(
                            agents,
                            crate::agent::GameLogEvent::warning("Card play failed")
                                .with_player(priority_player),
                        );
                    }
                }
                MainPhaseAction::ActivateMana(land_id) => {
                    self.log_priority_response(
                        game,
                        priority_player,
                        &self.describe_priority_action(game, action, None),
                    );
                    if !action_space.tappable_lands.contains(&land_id) {
                        crate::agent::notify_all_agents(
                            agents,
                            crate::agent::GameLogEvent::warning(
                                "Illegal action ignored: permanent can't tap for mana",
                            )
                            .with_player(priority_player),
                        );
                        passed_count += 1;
                        priority_player = game.next_player(priority_player);
                        self.with_shared_state_mutation(game, agents, |_this, game, _agents| {
                            game.turn.priority_player = priority_player;
                        });
                        continue;
                    }
                    // Snapshot pool BEFORE any mana production — single source of truth
                    // for rollback. Covers base ability + granted abilities + aura triggers.
                    let pool_snapshot = self.pool(priority_player).begin_tap_tracking();

                    // Collect all mana abilities on this permanent (native + granted by auras).
                    let mana_abs: Vec<_> = {
                        let c = game.card(land_id);
                        c.activated_abilities
                            .iter()
                            .filter(|ab| ab.is_mana_ability)
                            .cloned()
                            .collect()
                    };
                    if !mana_abs.is_empty() {
                        // Resolve the primary (tap-cost) mana ability
                        let primary = &mana_abs[0];
                        self.with_shared_state_mutation(game, agents, |this, game, agents| {
                            this.resolve_mana_ability(game, agents, priority_player, land_id, primary);
                        });
                        // Resolve additional mana abilities whose tap cost was already paid
                        for ab in mana_abs.iter().skip(1) {
                            let is_tap_only = ab.cost.parts.len() == 1
                                && ab.cost.parts.iter().all(|p| matches!(p, crate::cost::CostPart::Tap));
                            if is_tap_only {
                                self.with_shared_state_mutation(game, agents, |this, game, agents| {
                                    let produced = ab.params.get(crate::parsing::keys::PRODUCED).unwrap_or("");
                                    let mana_string = crate::mana::determine_mana_production(
                                        game, agents, priority_player, land_id, produced, ab.params.get(crate::parsing::keys::AMOUNT),
                                    );
                                    if let Some(ms) = mana_string {
                                        let source_is_snow = game.card(land_id).type_line.is_snow();
                                        let mana_params = crate::mana::ManaProductionParams {
                                            source_card: land_id,
                                            is_snow: source_is_snow,
                                            restriction: ab.params.get_cloned(crate::parsing::keys::RESTRICT_VALID),
                                            adds_no_counter: ab.params.is_true(crate::parsing::keys::ADDS_NO_COUNTER),
                                            adds_keywords: ab.params.get_cloned(crate::parsing::keys::ADDS_KEYWORDS),
                                            adds_keywords_valid: ab.params.get_cloned(crate::parsing::keys::ADDS_KEYWORDS_VALID),
                                            adds_counters: ab.params.get_cloned(crate::parsing::keys::ADDS_COUNTERS),
                                            adds_counters_valid: ab.params.get_cloned(crate::parsing::keys::ADDS_COUNTERS_VALID),
                                            triggers_when_spent: ab.params.get_cloned(crate::parsing::keys::TRIGGERS_WHEN_SPENT),
                                        };
                                        crate::mana::add_produced_mana_to_pool(
                                            this.pool_mut(priority_player), &ms, &mana_params,
                                        );
                                    }
                                });
                            }
                        }
                    } else {
                        // Legacy fallback for lands with no parsed mana abilities
                        self.with_shared_state_mutation(game, agents, |this, game, agents| {
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
                                // Resolve mana triggers inline (e.g. Utopia Sprawl).
                                let pending = this.trigger_handler.run_waiting_triggers(game);
                                for pt in pending {
                                    this.resolve_single_effect(game, agents, &pt.entry.spell_ability, None);
                                }
                            }
                        });
                    }

                    // Record ALL mana produced by this tap for rollback — single snapshot
                    // covers base ability + granted abilities + aura triggers.
                    let produced = self.pool(priority_player).end_tap_tracking(&pool_snapshot);
                    if !produced.is_empty() {
                        self.with_shared_state_mutation(game, agents, |_this, game, _agents| {
                            game.card_mut(land_id).last_mana_produced = Some(produced);
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
                    if !action_space.untappable_lands.contains(&land_id) {
                        crate::agent::notify_all_agents(
                            agents,
                            crate::agent::GameLogEvent::warning(
                                "Illegal action ignored: land can't be untapped for mana rollback",
                            )
                            .with_player(priority_player),
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
                            // Remove all mana produced by the last tap — covers base,
                            // aura triggers, static doublers, and any other source.
                            if let Some(produced) = game.card_mut(land_id).last_mana_produced.take() {
                                this.pool_mut(priority_player).rollback_tap(&produced);
                            } else {
                                // Fallback: remove the first matching base atom
                                let pool = this.pool_mut(priority_player);
                                for &atom in &atoms {
                                    if pool.has_atom(atom, 1) {
                                        pool.remove(atom, 1);
                                        break;
                                    }
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
                    if !action_space.activatable.contains(&(card_id, ability_idx)) {
                        crate::agent::notify_all_agents(
                            agents,
                            crate::agent::GameLogEvent::warning(
                                "Illegal action ignored: ability not activatable",
                            )
                            .with_player(priority_player),
                        );
                        passed_count += 1;
                        priority_player = game.next_player(priority_player);
                        self.with_shared_state_mutation(game, agents, |_this, game, _agents| {
                            game.turn.priority_player = priority_player;
                        });
                        continue;
                    }
                    let activated =
                        self.with_shared_state_mutation(game, agents, |this, game, agents| {
                            this.activate_ability(
                                game,
                                agents,
                                priority_player,
                                card_id,
                                ability_idx,
                            )
                        });
                    if activated {
                        // Process triggers immediately after ability activation so
                        // they go on the stack above the ability (mirroring the
                        // Play arm and Java's addAndUnfreeze behaviour).
                        self.with_shared_state_mutation(game, agents, |this, game, agents| {
                            this.process_triggers(game, agents);
                        });
                        passed_count = 0;
                    } else {
                        // Activation failed (e.g. payment declined, targets invalid).
                        // Match Java's behaviour: the player retains priority and
                        // gets to choose again (Java's do-while loops back to
                        // chooseSpellAbilityToPlay for the same player).  Do NOT
                        // change passed_count or priority_player.
                    }
                }
            }
        }
        self.with_shared_state_mutation(game, agents, |_this, game, _agents| {
            game.turn.priority_player = game.active_player();
        });
    }
}
