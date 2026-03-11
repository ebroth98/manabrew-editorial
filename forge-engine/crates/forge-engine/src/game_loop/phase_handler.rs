use super::*;
use super::game_action::CostPaymentContext;
use crate::card::CounterType;

impl GameLoop {
    pub(crate) fn run_turn_state_machine(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
    ) {
        // Fire TurnBegin trigger at the start of each turn.
        let active = game.active_player();
        self.trigger_handler.run_trigger(
            TriggerType::TurnBegin,
            RunParams {
                player: Some(active),
                ..Default::default()
            },
            false,
        );

        let mut state = TurnMachineState::Untap;
        while !game.game_over && state != TurnMachineState::Done {
            // EndTurn (issue #22): if end_turn_requested, skip directly to cleanup.
            if game.end_turn_requested
                && state != TurnMachineState::Cleanup
                && state != TurnMachineState::Done
            {
                game.end_turn_requested = false;
                self.combat.clear_with_cards(&mut game.cards);
                state = TurnMachineState::Cleanup;
                continue;
            }
            state = match state {
                TurnMachineState::Untap => {
                    // SkipPhase: skip untap if flag set
                    let active = game.active_player();
                    if game.player(active).skip_next_untap {
                        game.player_mut(active).skip_next_untap = false;
                        TurnMachineState::Upkeep
                    } else {
                        self.apply_turn_event(
                            game,
                            agents,
                            TurnEvent::EnterPhase {
                                phase: PhaseType::Untap,
                                emit_phase_trigger: false,
                            },
                        );
                        self.apply_turn_event(game, agents, TurnEvent::UntapStep);
                        TurnMachineState::Upkeep
                    }
                }
                TurnMachineState::Upkeep => {
                    self.apply_turn_event(
                        game,
                        agents,
                        TurnEvent::EnterPhase {
                            phase: PhaseType::Upkeep,
                            emit_phase_trigger: true,
                        },
                    );
                    self.apply_turn_event(
                        game,
                        agents,
                        TurnEvent::PriorityWindow {
                            is_main_phase: false,
                        },
                    );
                    let mut had_cumulative_upkeep = false;
                    self.with_shared_state_mutation(game, agents, |this, game, agents| {
                        had_cumulative_upkeep = this.process_cumulative_upkeep(game, agents);
                        // Cumulative upkeep costs can fire triggers (e.g. FlipCoin).
                        // Resolve those triggers in Upkeep, then return priority in
                        // Upkeep before advancing to Draw. This mirrors Java's
                        // phase flow around beginning-of-upkeep trigger resolution.
                        if had_cumulative_upkeep {
                            this.process_triggers(game, agents);
                            while !game.stack.is_empty() {
                                this.resolve_stack(game, agents);
                            }
                        }
                    });
                    if had_cumulative_upkeep {
                        self.apply_turn_event(
                            game,
                            agents,
                            TurnEvent::PriorityWindow {
                                is_main_phase: false,
                            },
                        );
                    }
                    TurnMachineState::Draw
                }
                TurnMachineState::Draw => {
                    // SkipPhase: skip draw if flag set
                    let active = game.active_player();
                    if game.player(active).skip_next_draw {
                        game.player_mut(active).skip_next_draw = false;
                        TurnMachineState::Main1
                    } else {
                        self.apply_turn_event(
                            game,
                            agents,
                            TurnEvent::EnterPhase {
                                phase: PhaseType::Draw,
                                emit_phase_trigger: true,
                            },
                        );
                        self.apply_turn_event(game, agents, TurnEvent::DrawStep);
                        self.apply_turn_event(
                            game,
                            agents,
                            TurnEvent::PriorityWindow {
                                is_main_phase: false,
                            },
                        );
                        TurnMachineState::Main1
                    }
                }
                TurnMachineState::Main1 => {
                    self.apply_turn_event(
                        game,
                        agents,
                        TurnEvent::EnterPhase {
                            phase: PhaseType::Main1,
                            emit_phase_trigger: true,
                        },
                    );
                    self.apply_turn_event(
                        game,
                        agents,
                        TurnEvent::PriorityWindow {
                            is_main_phase: true,
                        },
                    );
                    TurnMachineState::Combat
                }
                TurnMachineState::Combat => {
                    // SkipPhase: skip combat if flag set
                    let active = game.active_player();
                    if game.player(active).skip_next_combat {
                        game.player_mut(active).skip_next_combat = false;
                        TurnMachineState::Main2
                    } else {
                        self.apply_turn_event(game, agents, TurnEvent::CombatStep);
                        // Extra combat phases (issue #22, AddPhase effect)
                        if game.extra_combat_phases > 0 {
                            game.extra_combat_phases -= 1;
                            TurnMachineState::Combat // loop back for another combat
                        } else {
                            TurnMachineState::Main2
                        }
                    }
                }
                TurnMachineState::Main2 => {
                    self.apply_turn_event(
                        game,
                        agents,
                        TurnEvent::EnterPhase {
                            phase: PhaseType::Main2,
                            emit_phase_trigger: true,
                        },
                    );
                    self.apply_turn_event(
                        game,
                        agents,
                        TurnEvent::PriorityWindow {
                            is_main_phase: true,
                        },
                    );
                    TurnMachineState::EndOfTurn
                }
                TurnMachineState::EndOfTurn => {
                    self.apply_turn_event(
                        game,
                        agents,
                        TurnEvent::EnterPhase {
                            phase: PhaseType::EndOfTurn,
                            emit_phase_trigger: true,
                        },
                    );
                    self.apply_turn_event(
                        game,
                        agents,
                        TurnEvent::PriorityWindow {
                            is_main_phase: false,
                        },
                    );
                    TurnMachineState::Cleanup
                }
                TurnMachineState::Cleanup => {
                    self.apply_turn_event(
                        game,
                        agents,
                        TurnEvent::EnterPhase {
                            phase: PhaseType::Cleanup,
                            emit_phase_trigger: false,
                        },
                    );
                    self.apply_turn_event(game, agents, TurnEvent::CleanupStep);
                    self.apply_turn_event(game, agents, TurnEvent::AdvanceTurn);
                    TurnMachineState::Done
                }
                TurnMachineState::Done => TurnMachineState::Done,
            };
        }
    }

    fn apply_turn_event(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        event: TurnEvent,
    ) {
        if game.game_over {
            return;
        }
        match event {
            TurnEvent::EnterPhase {
                phase,
                emit_phase_trigger,
            } => {
                self.set_phase(game, agents, phase);
                if emit_phase_trigger {
                    self.emit_phase_trigger(game, phase);
                }
                // Suspend: at the beginning of each upkeep, remove a time counter
                // from each suspended card in exile. If last counter removed, cast for free.
                if phase == PhaseType::Upkeep {
                    self.process_suspend_upkeep(game, agents);
                }
            }
            TurnEvent::PriorityWindow { is_main_phase } => {
                self.step_with_priority(game, agents, is_main_phase);
            }
            TurnEvent::UntapStep => {
                self.with_shared_state_mutation(game, agents, |this, game, agents| {
                    this.step_untap(game, agents);
                });
            }
            TurnEvent::DrawStep => {
                self.with_shared_state_mutation(game, agents, |this, game, _agents| {
                    this.step_draw(game);
                });
            }
            TurnEvent::CombatStep => {
                self.step_combat(game, agents);
            }
            TurnEvent::CleanupStep => {
                self.with_shared_state_mutation(game, agents, |this, game, agents| {
                    this.step_cleanup(game, agents);
                });
            }
            TurnEvent::AdvanceTurn => {
                self.with_shared_state_mutation(game, agents, |_this, game, _agents| {
                    // Extra turns (issue #22): if the queue is non-empty, the
                    // front player gets the next turn instead of the normal
                    // rotation.  Mirrors Java's PhaseHandler.handleNextTurn().
                    if let Some(extra_turn) = game.extra_turns.pop_front() {
                        game.turn.active_player = extra_turn.player;
                        game.turn.priority_player = extra_turn.player;
                        game.turn.turn_number += 1;
                        game.turn.combat_attackers_declared = false;
                        game.turn.combat_blockers_declared = false;
                        game.turn.drawn_for_turn = false;
                        // SkipUntap on extra turn (issue #22, AddTurn parity fix)
                        if extra_turn.skip_untap {
                            game.player_mut(extra_turn.player).skip_next_untap = true;
                        }
                    } else {
                        game.turn.next_player_turn(&game.player_order.clone());
                    }
                });
            }
        }
    }

    pub fn step_untap(&mut self, game: &mut GameState, agents: &mut [Box<dyn PlayerAgent>]) {
        let active = game.active_player();

        // Phase-in (issue #22): phase in all phased-out permanents controlled by active player.
        for i in 0..game.cards.len() {
            if game.cards[i].phased_out
                && game.cards[i].controller == active
                && game.cards[i].zone == ZoneType::Battlefield
            {
                game.cards[i].phased_out = false;
            }
        }

        // Untap permanents, skipping exerted ones and resetting their flag.
        let cards: Vec<crate::ids::CardId> =
            game.cards_in_zone(ZoneType::Battlefield, active).to_vec();
        for cid in cards {
            if game.card(cid).exerted {
                // Exerted creatures don't untap this turn; reset flag so they untap next turn.
                game.card_mut(cid).exerted = false;
            } else {
                let optional_keyword =
                    "You may choose not to untap CARDNAME during your untap step.";
                let should_untap = if game.card(cid).has_keyword(optional_keyword) {
                    let question = format!("Untap {}?", game.card(cid).card_name);
                    let source_name = game.card(cid).card_name.clone();
                    agents[active.index()].choose_binary(
                        active,
                        &question,
                        crate::agent::BinaryChoiceKind::UntapOrLeaveTapped,
                        Some(true),
                        Some(&source_name),
                        None,
                    )
                } else {
                    true
                };
                if should_untap {
                    game.untap(cid);
                }
            }
        }
        self.pool_mut(active).empty();
    }

    fn process_cumulative_upkeep(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
    ) -> bool {
        let active = game.active_player();
        let permanents = game.cards_in_zone(ZoneType::Battlefield, active).to_vec();
        let mut processed_any = false;

        for card_id in permanents {
            if game.card(card_id).zone != ZoneType::Battlefield || game.card(card_id).controller != active {
                continue;
            }

            let cumulative_kw = game
                .card(card_id)
                .keywords
                .iter()
                .find(|kw| kw.starts_with("Cumulative upkeep:"))
                .cloned();
            let Some(keyword) = cumulative_kw else {
                continue;
            };
            processed_any = true;

            // Card script format: "Cumulative upkeep:<CostSpec>:<Reminder>"
            let rest = &keyword["Cumulative upkeep:".len()..];
            let Some((cost_spec, _)) = rest.split_once(':') else {
                continue;
            };
            let cost = crate::cost::parse_cost(cost_spec);
            if cost.parts.is_empty() {
                continue;
            }

            game.card_mut(card_id).add_counter(&CounterType::Age, 1);
            let age_counters = game.card(card_id).counter_count(&CounterType::Age).max(0);

            let mut paid_all = true;
            for _ in 0..age_counters {
                let paid = self.pay_ability_cost(
                    game,
                    agents,
                    active,
                    card_id,
                    &cost,
                    None,
                    false,
                    CostPaymentContext::TriggerResolve,
                );
                if !paid {
                    paid_all = false;
                    break;
                }
            }

            if !paid_all && game.card(card_id).zone == ZoneType::Battlefield {
                let owner = game.card(card_id).owner;
                self.trigger_handler.run_trigger(
                    TriggerType::Sacrificed,
                    RunParams {
                        card: Some(card_id),
                        player: Some(active),
                        ..Default::default()
                    },
                    false,
                );
                game.move_card(card_id, ZoneType::Graveyard, owner);
                self.trigger_handler.run_trigger(
                    TriggerType::ChangesZone,
                    RunParams {
                        card: Some(card_id),
                        origin: Some(ZoneType::Battlefield),
                        destination: Some(ZoneType::Graveyard),
                        ..Default::default()
                    },
                    false,
                );
            }
        }
        processed_any
    }

    pub fn step_draw(&mut self, game: &mut GameState) {
        let active = game.active_player();
        // Skip draw on turn 1
        if game.turn.turn_number > 1 {
            if let Some(card_id) = game.draw_card(active) {
                // Fire Drawn trigger for turn draw
                self.trigger_handler.run_trigger(
                    TriggerType::Drawn,
                    RunParams {
                        card: Some(card_id),
                        player: Some(active),
                        ..Default::default()
                    },
                    false,
                );
            }
        }
    }
    pub fn step_with_priority(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        is_main_phase: bool,
    ) {
        self.game_log.log(
            GameLogEntryType::Info,
            2,
            format!(
                "Priority window opened ({}, stack depth: {})",
                if is_main_phase {
                    "main-phase speed"
                } else {
                    "instant speed only"
                },
                game.stack.len()
            ),
        );
        self.with_shared_state_mutation(game, agents, |_this, game, _agents| {
            game.turn.priority_player = game.active_player();
        });
        loop {
            if game.game_over {
                return;
            }

            // 1. Process any pending triggers and put them on the stack
            self.with_shared_state_mutation(game, agents, |this, game, agents| {
                this.process_triggers(game, agents);
            });

            // 2. Give players priority
            self.priority_round(game, agents, is_main_phase);

            if game.game_over {
                return;
            }

            // 3. If stack is empty after everyone passed, the phase ends
            if game.stack.is_empty() {
                break;
            }

            // 4. Resolve top of stack (resolve_stack resolves one and gives priority)
            self.with_shared_state_mutation(game, agents, |this, game, agents| {
                this.resolve_stack(game, agents);
            });
        }
    }
    pub fn step_main_phase(&mut self, game: &mut GameState, agents: &mut [Box<dyn PlayerAgent>]) {
        self.step_with_priority(game, agents, true);
    }

    pub fn step_combat(&mut self, game: &mut GameState, agents: &mut [Box<dyn PlayerAgent>]) {
        let active = game.active_player();
        let defending = game.opponent_of(active);
        self.combat.clear_with_cards(&mut game.cards);
        game.turn.combat_block_assignments.clear();
        self.combat.attacking_player = Some(active);
        self.combat.defending_player = Some(defending);

        // Begin Combat
        self.set_phase(game, agents, PhaseType::CombatBegin);
        self.emit_phase_trigger(game, PhaseType::CombatBegin);
        self.step_with_priority(game, agents, false);
        if game.game_over {
            self.combat.clear_with_cards(&mut game.cards);
            return;
        }

        // EndCombatPhase (issue #22): if requested, exit combat early
        if game.end_combat_requested {
            game.end_combat_requested = false;
            self.combat.clear_with_cards(&mut game.cards);
            return;
        }

        // Recompute continuous effects before evaluating attack/block legality.
        // CantAttack / CantBlock flags are set here.
        apply_continuous_effects(game);

        // Declare Attackers
        self.set_phase(game, agents, PhaseType::CombatDeclareAttackers);
        let available_attackers = combat::get_available_attackers(game, active);
        let possible_defenders = combat::get_possible_defenders(game, active);

        // Compute attack requirements (must-attack from statics + goad)
        let must_attackers = if available_attackers.is_empty() {
            Vec::new()
        } else {
            let requirements = combat::attack_requirement::compute_attack_requirements(
                &game.cards,
                &available_attackers,
                defending,
            );
            combat::attack_requirement::must_attack_ids(&requirements)
        };

        // Java's PhaseHandler uses a do-while loop: declare attackers, validate,
        // and re-prompt if invalid.  We mirror this so RNG consumption matches.
        let mut chosen_attackers: Vec<(CardId, combat::DefenderId)> = Vec::new();
        if !available_attackers.is_empty() {
            let max_attempts = 50; // safety valve to avoid infinite loops
            for _attempt in 0..max_attempts {
                agents[active.index()].snapshot_state(game, &self.mana_pools);
                self.game_log.log(
                    GameLogEntryType::PriorityWaiting,
                    2,
                    format!(
                        "Waiting for {} attacker declaration",
                        game.player(active).name
                    ),
                );
                let agent = &mut agents[active.index()];
                let mut picked =
                    agent.choose_attackers(active, &available_attackers, &possible_defenders);
                if self.apply_pending_snapshot_restore(game, agents) {
                    return;
                }
                self.game_log.log(
                    GameLogEntryType::PriorityResponse,
                    2,
                    format!(
                        "{} declared {} attacker(s)",
                        game.player(active).name,
                        picked.len()
                    ),
                );

                // Mirror Java's validateAttackers + getLegalAttackers fallback:
                // if must-attack creatures were not included by the agent, add them
                // directly without re-calling the agent (no extra RNG consumption).
                let default_defender = possible_defenders
                    .first()
                    .copied()
                    .unwrap_or(combat::DefenderId::Player(defending));
                for &must in &must_attackers {
                    if !picked.iter().any(|(a, _)| *a == must) {
                        picked.push((must, default_defender));
                    }
                }

                // Validate attack restrictions (OnlyAlone, NotAlone, NeedGreaterPower, etc.)
                let attacker_ids: Vec<CardId> = picked.iter().map(|(a, _)| *a).collect();
                let illegal = combat::attack_restriction::validate_attack_restrictions(
                    &attacker_ids,
                    &game.cards,
                );
                if !illegal.is_empty() {
                    picked.retain(|(id, _)| !illegal.contains(id));
                }

                // Check AttackRestrict limits (global + per-defender).
                let global_max =
                    crate::staticability::static_ability_attack_restrict::global_attack_restrict(
                        &game.cards,
                    );

                // Global limit applies to ALL attackers regardless of defender.
                let mut invalid = false;
                if let Some(max) = global_max {
                    if picked.len() > max as usize {
                        invalid = true;
                    }
                }

                // Per-defender limit: only count attackers going to that defender.
                // Crawlspace's "ValidDefender$ You" only restricts attacks against
                // the Crawlspace controller, not attacks against planeswalkers.
                if !invalid {
                    let defender_max =
                        crate::staticability::static_ability_attack_restrict::attack_restrict_num_for_defender(
                            &game.cards,
                            defending,
                        );
                    if let Some(max) = defender_max {
                        let attackers_to_defender = picked
                            .iter()
                            .filter(|(_, def)| def.controlling_player(game) == defending)
                            .count();
                        if attackers_to_defender > max as usize {
                            invalid = true;
                        }
                    }
                }

                if invalid {
                    // Declaration invalid — re-prompt like Java's PhaseHandler.
                    agents[active.index()].notify("Attack declaration invalid");
                    continue;
                }

                chosen_attackers = picked;
                break;
            }
        }

        // Java parity: pre-mark declared attackers before optional attack-cost
        // resolution so they are not valid enlist targets.
        // Java does this by temporarily tapping non-vigilance attackers and
        // treating them as attacking before OptionalAttackCost is paid.
        let premarked_attackers: Vec<(CardId, combat::DefenderId)> = chosen_attackers.clone();
        for &(attacker_id, def) in &premarked_attackers {
            let defending_player = def.controlling_player(game);
            game.card_mut(attacker_id).attacking_player = Some(defending_player);
            if !game.card(attacker_id).has_vigilance() {
                game.card_mut(attacker_id).tapped = true;
            }
        }

        // Java parity: optional attack costs (Exert/Enlist) are chosen immediately
        // after attackers are declared and before CantAttackUnless payments.
        {
            let declared_attackers: Vec<CardId> =
                chosen_attackers.iter().map(|(attacker, _)| *attacker).collect();
            let mut optional_exert_by_attacker: std::collections::HashMap<CardId, Vec<(i32, String)>> =
                std::collections::HashMap::new();
            let mut optional_enlist_by_attacker: std::collections::HashMap<CardId, Vec<(i32, String)>> =
                std::collections::HashMap::new();

            for &attacker in &declared_attackers {
                let static_abilities = game.card(attacker).static_abilities.clone();
                for st in &static_abilities {
                    if st.mode != crate::staticability::StaticMode::OptionalAttackCost {
                        continue;
                    }
                    let Some(cost_raw) = st.params.get("Cost") else {
                        continue;
                    };
                    let parsed = crate::cost::parse_cost(cost_raw);
                    for part in parsed.parts {
                        match part {
                            crate::cost::CostPart::Exert { amount, type_filter } => {
                                optional_exert_by_attacker
                                    .entry(attacker)
                                    .or_default()
                                    .push((amount, type_filter));
                            }
                            crate::cost::CostPart::Enlist { amount, type_filter } => {
                                optional_enlist_by_attacker
                                    .entry(attacker)
                                    .or_default()
                                    .push((amount, type_filter));
                            }
                            _ => {}
                        }
                    }
                }
            }

            let possible_exerters: Vec<CardId> = declared_attackers
                .iter()
                .copied()
                .filter(|cid| optional_exert_by_attacker.contains_key(cid))
                .collect();
            if !possible_exerters.is_empty() {
                let chosen = agents[active.index()].exert_attackers(active, &possible_exerters);
                for attacker in chosen {
                    if let Some(parts) = optional_exert_by_attacker.get(&attacker).cloned() {
                        for (amount, type_filter) in parts {
                            let resolved =
                                crate::cost::resolve_dynamic_amount(game, attacker, active, amount);
                            if resolved > 0 {
                                self.pay_exert_cost(
                                    game,
                                    agents,
                                    active,
                                    attacker,
                                    &type_filter,
                                    resolved,
                                );
                            }
                        }
                    }
                }
            }

            // Re-check enlist targets AFTER exert loop — exerting taps creatures,
            // which can invalidate enlist candidates.
            let enlist_can_pay = !crate::cost::get_enlist_targets(game, active).is_empty();
            let possible_enlisters: Vec<CardId> = if enlist_can_pay {
                declared_attackers
                    .iter()
                    .copied()
                    .filter(|cid| optional_enlist_by_attacker.contains_key(cid))
                    .collect()
            } else {
                Vec::new()
            };

            if !possible_enlisters.is_empty() {
                let chosen = agents[active.index()].enlist_attackers(active, &possible_enlisters);
                for attacker in chosen {
                    if let Some(parts) = optional_enlist_by_attacker.get(&attacker).cloned() {
                        for (amount, type_filter) in parts {
                            let resolved =
                                crate::cost::resolve_dynamic_amount(game, attacker, active, amount);
                            if resolved > 0 {
                                self.pay_enlist_cost(
                                    game,
                                    agents,
                                    active,
                                    attacker,
                                    &type_filter,
                                    resolved,
                                );
                            }
                        }
                    }
                }
            }
        }

        // Check attack costs (Propaganda, Ghostly Prison effects)
        {
            let mut cost_failures = Vec::new();
            for &(attacker_id, defender) in &chosen_attackers {
                let cost = combat::attack_cost::get_attack_cost(
                    &game.cards,
                    game.card(attacker_id),
                    defender,
                );
                if cost > 0 {
                    let controller = game.card(attacker_id).controller;
                    let attacker_name = game.card(attacker_id).card_name.clone();
                    let description = format!("Pay {{{}}} to attack with {}", cost, attacker_name);

                    // Loop: let the agent tap lands / pay / decline
                    loop {
                        let tappable_lands = self.get_tappable_lands(game, controller);
                        let pool_snapshot = self.pool(controller).clone();
                        let untappable_lands =
                            self.get_untappable_lands(game, controller, &pool_snapshot);
                        let pool_total = self.pool(controller).total();

                        agents[controller.index()].snapshot_state(game, &self.mana_pools);
                        let action = agents[controller.index()].pay_combat_cost(
                            controller,
                            attacker_id,
                            cost,
                            &description,
                            &tappable_lands,
                            &untappable_lands,
                            pool_total,
                        );

                        match action {
                            CombatCostAction::TapLand(land_id) => {
                                if !tappable_lands.contains(&land_id) {
                                    continue;
                                }
                                // Use actual mana ability when available
                                let mana_ab = {
                                    let c = game.card(land_id);
                                    c.activated_abilities
                                        .iter()
                                        .find(|ab| ab.is_mana_ability)
                                        .cloned()
                                };
                                if let Some(ab) = mana_ab {
                                    self.with_shared_state_mutation(
                                        game,
                                        agents,
                                        |this, game, agents| {
                                            this.resolve_mana_ability(
                                                game, agents, controller, land_id, &ab,
                                            );
                                        },
                                    );
                                } else {
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
                                        self.pool_mut(controller).add(atom, 1);
                                        self.trigger_handler.run_trigger(
                                            TriggerType::Taps,
                                            RunParams {
                                                card: Some(land_id),
                                                player: Some(controller),
                                                ..Default::default()
                                            },
                                            false,
                                        );
                                        self.trigger_handler.run_trigger(
                                            TriggerType::TapsForMana,
                                            RunParams {
                                                card: Some(land_id),
                                                player: Some(controller),
                                                ..Default::default()
                                            },
                                            false,
                                        );
                                    }
                                }
                            }
                            CombatCostAction::UntapLand(land_id) => {
                                if !untappable_lands.contains(&land_id) {
                                    continue;
                                }
                                let atoms = {
                                    let c = game.card(land_id);
                                    if c.is_land() && c.tapped {
                                        let a = mana::land_mana_atoms(c);
                                        if a.is_empty() {
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
                                    let pool = self.pool_mut(controller);
                                    for &atom in &atoms {
                                        if pool.has_atom(atom, 1) {
                                            pool.remove(atom, 1);
                                            break;
                                        }
                                    }
                                    self.trigger_handler.run_trigger(
                                        TriggerType::Untaps,
                                        RunParams {
                                            card: Some(land_id),
                                            player: Some(controller),
                                            ..Default::default()
                                        },
                                        false,
                                    );
                                }
                            }
                            CombatCostAction::Pay => {
                                let pool = &mut self.mana_pools[controller.index()];
                                if pool.total() >= cost {
                                    pool.spend_generic(cost);
                                    // Successfully paid
                                } else {
                                    // Not enough mana — treat as decline
                                    cost_failures.push(attacker_id);
                                }
                                break;
                            }
                            CombatCostAction::Decline => {
                                cost_failures.push(attacker_id);
                                break;
                            }
                        }
                    }
                }
            }
            chosen_attackers.retain(|(id, _)| !cost_failures.contains(id));
        }

        // Undo temporary attack markers for attackers removed by cost payment.
        for &(attacker_id, _) in &premarked_attackers {
            if !chosen_attackers.iter().any(|(id, _)| *id == attacker_id) {
                game.card_mut(attacker_id).attacking_player = None;
                if !game.card(attacker_id).has_vigilance() {
                    game.card_mut(attacker_id).tapped = false;
                }
            }
        }

        if !chosen_attackers.is_empty() {
            crate::agent::notify_all_agents(
                agents,
                crate::agent::GameLogEvent::action("Combat phase begins").with_player(active),
            );
            let attackers_msg = chosen_attackers
                .iter()
                .map(|(attacker_id, defender)| {
                    let attacker_name = game.card(*attacker_id).card_name.clone();
                    let defender_name = match defender {
                        combat::DefenderId::Player(pid) => game.player(*pid).name.clone(),
                        combat::DefenderId::Permanent(cid) => game.card(*cid).card_name.clone(),
                    };
                    format!("{attacker_name} -> {defender_name}")
                })
                .collect::<Vec<_>>()
                .join(", ");
            crate::agent::notify_all_agents(
                agents,
                crate::agent::GameLogEvent::action(format!("Attackers: {attackers_msg}"))
                    .with_player(active),
            );
        }

        // Tap attackers (Vigilance skips tapping)
        let num_attackers = chosen_attackers.len() as i32;
        for &(attacker_id, defender) in &chosen_attackers {
            if !game.card(attacker_id).has_vigilance() {
                // We pre-tapped attackers before OptionalAttackCost resolution to
                // mirror Java legality checks; untap first so this tap emits the
                // declaration-time Taps trigger once.
                if game.card(attacker_id).tapped {
                    game.untap(attacker_id);
                }
                game.tap(attacker_id);
                // Java attacker.tap(...) emits Taps triggers when a creature becomes tapped
                // as part of attacker declaration.
                self.trigger_handler.run_trigger(
                    TriggerType::Taps,
                    RunParams {
                        card: Some(attacker_id),
                        player: Some(active),
                        ..Default::default()
                    },
                    false,
                );
            }
            game.card_mut(attacker_id).attacked_this_turn = true;
            // Set attacking_player to the controlling player of the defender
            let def_player = defender.controlling_player(game);
            game.card_mut(attacker_id).attacking_player = Some(def_player);
            self.combat.declare_attacker(attacker_id, defender);

            // Record attack in damage history
            game.card_mut(attacker_id)
                .damage_history
                .record_attack(num_attackers - 1);

            // Fire Attacks trigger for each attacker
            self.trigger_handler.run_trigger(
                TriggerType::Attacks,
                RunParams {
                    attacker: Some(attacker_id),
                    card: Some(attacker_id),
                    defending_player: Some(def_player),
                    num_attackers: Some(num_attackers as usize),
                    ..Default::default()
                },
                false,
            );
        }
        // Fire AttackersDeclared batch trigger
        if !chosen_attackers.is_empty() {
            let attacker_ids: Vec<CardId> = chosen_attackers.iter().map(|(a, _)| *a).collect();
            self.trigger_handler.run_trigger(
                TriggerType::AttackersDeclared,
                RunParams {
                    player: Some(game.active_player()),
                    attacker_ids: Some(attacker_ids),
                    ..Default::default()
                },
                false,
            );
        }
        // Recompute continuous effects now that `attacking_player` is set on
        // declared attackers.  This allows effects like Watchdog's
        // "Affected$ Creature.attackingYou | AddPower$ -1" to apply correctly.
        apply_continuous_effects(game);
        self.step_with_priority(game, agents, false);
        if game.game_over {
            self.combat.clear_with_cards(&mut game.cards);
            return;
        }

        // Java parity: COMBAT_DECLARE_BLOCKERS is skipped when no attackers remain.
        self.combat.remove_absent_combatants(&game.cards);
        if self.combat.has_attackers() {
            // Declare Blockers
            self.set_phase(game, agents, PhaseType::CombatDeclareBlockers);
            let attacker_card_ids: Vec<CardId> =
                self.combat.attackers.iter().map(|(a, _)| *a).collect();
            let available_blockers = combat::get_available_blockers(game, defending);
            let legal_blockers = combat::filter_legal_blockers(game, &attacker_card_ids, &available_blockers);
            let has_any_legal_blocker = !legal_blockers.is_empty();

            if has_any_legal_blocker {
                agents[defending.index()].snapshot_state(game, &self.mana_pools);
                self.game_log.log(
                    GameLogEntryType::PriorityWaiting,
                    2,
                    format!(
                        "Waiting for {} blocker declaration",
                        game.player(defending).name
                    ),
                );
                let mut chosen_blockers = {
                    let def_agent = &mut agents[defending.index()];
                    def_agent.choose_blockers(defending, &attacker_card_ids, &available_blockers)
                };
                if self.apply_pending_snapshot_restore(game, agents) {
                    return;
                }
                // Ignore duplicate blocker assignments; first assignment wins.
                let mut seen_blockers = std::collections::HashSet::new();
                chosen_blockers.retain(|(blocker, _)| seen_blockers.insert(*blocker));
                self.game_log.log(
                    GameLogEntryType::PriorityResponse,
                    2,
                    format!(
                        "{} declared {} blocker assignment(s)",
                        game.player(defending).name,
                        chosen_blockers.len()
                    ),
                );

                for (blocker, attacker) in chosen_blockers.into_iter() {
                    // Validate: use comprehensive evasion check
                    if !combat::can_creature_block(game, blocker, attacker) {
                        continue; // illegal block
                    }
                    self.combat.declare_blocker(blocker, attacker);

                    // Fire Blocks trigger for each (blocker, attacker) pair
                    self.trigger_handler.run_trigger(
                        TriggerType::Blocks,
                        RunParams {
                            blocker: Some(blocker),
                            blocked_attacker: Some(attacker),
                            card: Some(blocker),
                            ..Default::default()
                        },
                        false,
                    );
                }

                // Block cost checking (War Cadence effects)
                {
                    let mut block_cost_failures = Vec::new();
                    for &(blocker_id, attacker_id) in &self.combat.blockers {
                        let cost = combat::block_cost::get_block_cost(
                            &game.cards,
                            game.card(blocker_id),
                            game.card(attacker_id),
                        );
                        if cost > 0 {
                            let controller = game.card(blocker_id).controller;
                            let pool = &mut self.mana_pools[controller.index()];
                            if pool.total() >= cost {
                                pool.spend_generic(cost);
                            } else {
                                block_cost_failures.push(blocker_id);
                            }
                        }
                    }
                    self.combat
                        .blockers
                        .retain(|(b, _)| !block_cost_failures.contains(b));
                }

                // Block validation (Menace, can't block alone)
                let invalid_blocks = combat::validate_blocks(game, &self.combat);
                for (blocker_id, attacker_id) in &invalid_blocks {
                    self.combat
                        .blockers
                        .retain(|(b, a)| !(b == blocker_id && a == attacker_id));
                }

                // Must-block enforcement: auto-assign blockers to required targets
                let all_legal_blockers: Vec<CardId> = available_blockers.clone();
                for &blocker_id in &all_legal_blockers {
                    let must_targets =
                        combat::compute_must_block_targets(game, &self.combat, blocker_id);
                    if must_targets.is_empty() {
                        continue;
                    }
                    let currently_blocking: Vec<CardId> = self
                        .combat
                        .blockers
                        .iter()
                        .filter(|(b, _)| *b == blocker_id)
                        .map(|(_, a)| *a)
                        .collect();
                    if !must_targets.iter().any(|t| currently_blocking.contains(t)) {
                        // Not blocking any required target — force-assign first
                        if combat::can_creature_block(game, blocker_id, must_targets[0]) {
                            self.combat.declare_blocker(blocker_id, must_targets[0]);
                        }
                    }
                }

                // Record damage history for blockers
                for &(blocker_id, attacker_id) in &self.combat.blockers {
                    game.card_mut(blocker_id).damage_history.record_block();
                    game.card_mut(attacker_id)
                        .damage_history
                        .record_got_blocked();
                }

                // Publish finalized blocker assignments for UI snapshots in this combat.
                game.turn.combat_block_assignments = self.combat.blockers.clone();

                if !self.combat.blockers.is_empty() {
                    let blockers_msg = self
                        .combat
                        .blockers
                        .iter()
                        .map(|(blocker_id, attacker_id)| {
                            let blocker_name = game.card(*blocker_id).card_name.clone();
                            let attacker_name = game.card(*attacker_id).card_name.clone();
                            format!("{blocker_name} -> {attacker_name}")
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    crate::agent::notify_all_agents(
                        agents,
                        crate::agent::GameLogEvent::action(format!("Blockers: {blockers_msg}"))
                            .with_player(defending),
                    );
                }
            }

            // Prompt for damage assignment order on multi-blocked attackers
            for &(attacker_id, _) in &self.combat.attackers.clone() {
                let blockers_for = self.combat.get_blockers_for(attacker_id);
                if blockers_for.len() > 1 {
                    let controller = game.card(attacker_id).controller;
                    agents[controller.index()].snapshot_state(game, &self.mana_pools);
                    let ordered = agents[controller.index()].choose_damage_assignment_order(
                        controller,
                        attacker_id,
                        &blockers_for,
                    );
                    // Validate: must be a permutation of the blockers
                    if ordered.len() == blockers_for.len()
                        && blockers_for.iter().all(|b| ordered.contains(b))
                    {
                        self.combat.damage_order.insert(attacker_id, ordered);
                    } else {
                        // Invalid order — use default
                        self.combat.damage_order.insert(attacker_id, blockers_for);
                    }
                }
            }

            self.step_with_priority(game, agents, false);
            if game.game_over {
                self.combat.clear_with_cards(&mut game.cards);
                game.turn.combat_block_assignments.clear();
                return;
            }

            // Fire BlockersDeclared batch trigger
            self.trigger_handler.run_trigger(
                TriggerType::BlockersDeclared,
                RunParams::default(),
                false,
            );

            // Fire AttackerBlocked / AttackerUnblocked triggers
            for &(attacker_id, _) in &self.combat.attackers.clone() {
                if self.combat.is_blocked(attacker_id) {
                    self.trigger_handler.run_trigger(
                        TriggerType::AttackerBlocked,
                        RunParams {
                            attacker: Some(attacker_id),
                            card: Some(attacker_id),
                            ..Default::default()
                        },
                        false,
                    );
                } else {
                    self.trigger_handler.run_trigger(
                        TriggerType::AttackerUnblocked,
                        RunParams {
                            attacker: Some(attacker_id),
                            card: Some(attacker_id),
                            ..Default::default()
                        },
                        false,
                    );
                }
            }
        }

        // Pre-populate LKI cache for all combat participants so that if a
        // creature dies during damage, its combat role is already recorded.
        for &(attacker_id, _) in &self.combat.attackers.clone() {
            self.combat.save_lki(attacker_id);
        }
        for &(blocker_id, _) in &self.combat.blockers.clone() {
            self.combat.save_lki(blocker_id);
        }

        // Determine if we need first strike damage step
        let has_first_strikers = self.combat.has_first_strikers(game);

        if has_first_strikers && self.combat.has_attackers() {
            // First Strike Damage step
            self.set_phase(game, agents, PhaseType::CombatFirstStrikeDamage);
            let fs_unblocked_choices = self.choose_assign_as_unblocked(game, agents, true);
            let fs_events = self
                .combat
                .resolve_damage_step(game, true, &fs_unblocked_choices);
            // Record damage in source damage history for player-targeted combat damage
            for event in &fs_events {
                if event.target_player.is_some() && event.amount > 0 {
                    game.card_mut(event.source)
                        .damage_history
                        .record_damage(event.amount, true);
                }
            }
            self.fire_combat_damage_triggers(&fs_events);
            // Flush triggers before SBA so that triggers from creatures about
            // to die (e.g. enrage) are matched while still on the battlefield.
            self.trigger_handler.flush_waiting_triggers(game);

            // SBA between damage steps
            loop {
                if !game.check_state_based_actions_with_triggers(Some(&mut self.trigger_handler)) {
                    break;
                }
            }
            self.combat.remove_absent_combatants(&game.cards);
            if game.game_over {
                self.set_phase(game, agents, PhaseType::CombatEnd);
                self.combat.clear_with_cards(&mut game.cards);
                game.turn.combat_block_assignments.clear();
                return;
            }
            self.step_with_priority(game, agents, false);
            if game.game_over {
                self.combat.clear_with_cards(&mut game.cards);
                game.turn.combat_block_assignments.clear();
                return;
            }
        }

        // Java parity: skip regular combat damage step when no attackers remain.
        if self.combat.has_attackers() {
            self.set_phase(game, agents, PhaseType::CombatDamage);
            let unblocked_choices = self.choose_assign_as_unblocked(game, agents, false);
            let dmg_events = self
                .combat
                .resolve_damage_step(game, false, &unblocked_choices);
            // Record damage in source damage history for player-targeted combat damage
            for event in &dmg_events {
                if event.target_player.is_some() && event.amount > 0 {
                    game.card_mut(event.source)
                        .damage_history
                        .record_damage(event.amount, true);
                }
            }
            self.fire_combat_damage_triggers(&dmg_events);
            // Flush triggers before SBA so that triggers from creatures about
            // to die (e.g. enrage) are matched while still on the battlefield.
            self.trigger_handler.flush_waiting_triggers(game);

            // SBA after combat
            loop {
                if !game.check_state_based_actions_with_triggers(Some(&mut self.trigger_handler)) {
                    break;
                }
            }
            self.combat.remove_absent_combatants(&game.cards);
            self.step_with_priority(game, agents, false);
            if game.game_over {
                self.combat.clear_with_cards(&mut game.cards);
                game.turn.combat_block_assignments.clear();
                return;
            }
        }

        // End combat
        self.set_phase(game, agents, PhaseType::CombatEnd);
        self.emit_phase_trigger(game, PhaseType::CombatEnd);
        self.step_with_priority(game, agents, false);

        // End-of-combat damage history reset and must_block cleanup
        for card in game.cards.iter_mut() {
            if card.zone == ZoneType::Battlefield && card.is_creature() {
                card.damage_history.end_combat();
                card.must_block = false;
                card.must_block_cards.clear();
            }
        }

        self.combat.clear_with_cards(&mut game.cards);
        game.turn.combat_block_assignments.clear();
        // Recompute continuous effects after combat ends so that stale
        // combat-dependent modifiers (e.g. Watchdog's "creatures attacking you
        // get -1/-0") are cleared.  Without this, static_power_modifier lingers
        // until the next apply_continuous_effects call, causing snapshot drift.
        apply_continuous_effects(game);
    }

    fn choose_assign_as_unblocked(
        &mut self,
        game: &GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        first_strike_only: bool,
    ) -> std::collections::HashSet<CardId> {
        let mut choices = std::collections::HashSet::new();
        for &(attacker_id, _) in &self.combat.attackers {
            if !self.combat.is_blocked(attacker_id) {
                continue;
            }
            let attacker = game.card(attacker_id);
            let has_fs = attacker.has_first_strike();
            let has_ds = attacker.has_double_strike();
            let deals_in_step = if first_strike_only {
                has_fs || has_ds
            } else {
                !has_fs || has_ds
            };
            if !deals_in_step {
                continue;
            }
            if !crate::staticability::static_ability_assign_combat_damage_as_unblocked::has_optional_assign_as_unblocked(
                &game.cards,
                attacker,
            ) {
                continue;
            }

            let controller = attacker.controller;
            let desc = format!(
                "Have {} assign combat damage as though unblocked?",
                attacker.card_name
            );
            agents[controller.index()].snapshot_state(game, &self.mana_pools);
            if agents[controller.index()].choose_optional_trigger(
                controller,
                &desc,
                Some(&attacker.card_name),
                None,
            ) {
                choices.insert(attacker_id);
            }
        }
        choices
    }

    pub fn step_cleanup(&mut self, game: &mut GameState, agents: &mut [Box<dyn PlayerAgent>]) {
        let active = game.active_player();

        // Discard down to max hand size — player chooses which cards to discard.
        // Mirrors Java's Player.discard() during cleanup step.
        let hand_size = game.zone(ZoneType::Hand, active).len() as i32;
        let max = game.player(active).max_hand_size;
        if hand_size > max {
            let to_discard = (hand_size - max) as usize;
            let hand: Vec<CardId> = game.cards_in_zone(ZoneType::Hand, active).to_vec();
            agents[active.index()].snapshot_state(game, &self.mana_pools);
            self.game_log.log(
                GameLogEntryType::PriorityWaiting,
                2,
                format!(
                    "Waiting for {} discard decision (choose {})",
                    game.player(active).name,
                    to_discard
                ),
            );
            let chosen = agents[active.index()].choose_discard(active, &hand, to_discard);
            self.game_log.log(
                GameLogEntryType::PriorityResponse,
                2,
                format!(
                    "{} selected {} card(s) to discard",
                    game.player(active).name,
                    chosen.len().min(to_discard)
                ),
            );
            for card_id in chosen.iter().take(to_discard) {
                if game.card(*card_id).zone == ZoneType::Hand {
                    let has_madness = game.card(*card_id).get_madness_cost().is_some();
                    if has_madness {
                        // Madness: exile instead of graveyard (can cast for madness cost)
                        game.move_card(*card_id, ZoneType::Exile, active);
                        effects::emit_zone_trigger(
                            &mut self.trigger_handler,
                            *card_id,
                            ZoneType::Hand,
                            ZoneType::Exile,
                        );
                    } else {
                        game.move_card(*card_id, ZoneType::Graveyard, active);
                        effects::emit_zone_trigger(
                            &mut self.trigger_handler,
                            *card_id,
                            ZoneType::Hand,
                            ZoneType::Graveyard,
                        );
                    }
                    // Fire Discarded trigger regardless of destination
                    self.trigger_handler.run_trigger(
                        TriggerType::Discarded,
                        RunParams {
                            card: Some(*card_id),
                            player: Some(active),
                            ..Default::default()
                        },
                        false,
                    );
                }
            }
        }

        // Reset fog flag (issue #22: Fog effect lasts until end of turn).
        game.prevent_all_combat_damage = false;

        // Reset end-of-turn flags (issue #22).
        game.end_turn_requested = false;
        game.end_combat_requested = false;
        game.extra_combat_phases = 0;

        // Monarch draw (issue #22): at end of turn, the monarch draws a card.
        if let Some(monarch_id) = game.monarch {
            if game.player(monarch_id).is_alive() && monarch_id == active {
                game.draw_card(monarch_id);
            }
        }

        // Empty mana pool at end of turn (cleanup step), per Magic rules.
        self.pool_mut(active).empty();

        // Remove temporary command-zone effect cards created by AB$ Effect
        // that expire at end of turn.
        // These helper effect cards should cease to exist when they expire;
        // keeping them in Exile causes parity drift versus Java snapshots.
        let temp_effect_ids: Vec<CardId> = game
            .cards
            .iter()
            .filter(|c| c.zone == ZoneType::Command && c.temp_effect_until_eot)
            .map(|c| c.id)
            .collect();
        for effect_id in temp_effect_ids {
            if game.card(effect_id).zone == ZoneType::Command {
                let controller = game.card(effect_id).controller;
                game.zone_mut(ZoneType::Command, controller)
                    .remove(effect_id);
                game.cards[effect_id.index()].zone = ZoneType::None;
            }
        }

        // Remove damage and reset until-end-of-turn effects on all battlefield permanents
        for i in 0..game.cards.len() {
            if game.cards[i].zone == ZoneType::Battlefield {
                // Restore animate state before checking creature status (issue #52).
                // Must happen first: if the card was animated into a creature but its base
                // form is not a creature, we still need to reset its type/P/T.
                if let Some(state) = game.cards[i].animate_state.take() {
                    game.cards[i].type_line = state.original_type_line;
                    game.cards[i].base_power = state.original_base_power;
                    game.cards[i].base_toughness = state.original_base_toughness;
                    game.cards[i].color = state.original_color;
                    // Clear damage accumulated while animated as a creature.
                    // Without this, damage leaks into the next turn if the
                    // card is re-animated (the is_creature() check below would
                    // miss it since the card is no longer a creature).
                    game.cards[i].damage = 0;
                }

                if game.cards[i].is_creature() {
                    let keep_damage =
                        crate::staticability::static_ability_no_cleanup_damage::damage_not_removed(
                            &game.cards,
                            &game.cards[i],
                        );
                    if !keep_damage {
                        game.cards[i].damage = 0;
                    }
                    game.cards[i].power_modifier = 0;
                    game.cards[i].toughness_modifier = 0;
                    game.cards[i].pump_keywords.clear();
                    game.cards[i].has_deathtouch_damage = false;
                    // Reset regeneration shields at end of turn (issue #22).
                    game.cards[i].regeneration_shields = 0;
                    // Reset per-turn damage history.
                    game.cards[i].damage_history.new_turn();
                }
            }
        }
    }
    pub(crate) fn emit_phase_trigger(&mut self, game: &GameState, phase: PhaseType) {
        let active = game.active_player();
        self.trigger_handler.run_trigger(
            TriggerType::Phase,
            RunParams {
                phase: Some(phase),
                player: Some(active),
                ..Default::default()
            },
            false,
        );
        // Fire Always trigger alongside every phase trigger.
        self.trigger_handler.run_trigger(
            TriggerType::Always,
            RunParams {
                phase: Some(phase),
                player: Some(active),
                ..Default::default()
            },
            false,
        );
    }

    /// Fire DamageDone and LifeGained triggers from combat damage events.
    pub(crate) fn fire_combat_damage_triggers(&mut self, events: &[combat::CombatDamageEvent]) {
        for event in events {
            self.trigger_handler.run_trigger(
                TriggerType::DamageDone,
                RunParams {
                    damage_source: Some(event.source),
                    damage_target_player: event.target_player,
                    damage_target_card: event.target_card,
                    damage_amount: Some(event.amount),
                    is_combat_damage: Some(event.is_combat),
                    ..Default::default()
                },
                false,
            );
            if let Some(player) = event.lifelink_player {
                if event.lifelink_amount > 0 {
                    self.trigger_handler.run_trigger(
                        TriggerType::LifeGained,
                        RunParams {
                            player: Some(player),
                            life_amount: Some(event.lifelink_amount),
                            ..Default::default()
                        },
                        false,
                    );
                }
            }
        }
    }

    /// Suspend upkeep processing: for each suspended card in exile owned by the
    /// active player, remove a time counter. If the last counter was removed,
    /// cast the card for free (and grant haste if creature).
    /// Mirrors Java's GameAction.handleSuspendTriggers().
    pub(crate) fn process_suspend_upkeep(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
    ) {
        let active = game.active_player();
        let exile: Vec<CardId> = game.cards_in_zone(ZoneType::Exile, active).to_vec();
        for card_id in exile {
            let card = game.card(card_id);
            // Check if the card has suspend keyword and time counters
            if card.get_suspend_cost().is_none() {
                continue;
            }
            let time_counters = *card
                .counters
                .get(&crate::card::CounterType::Time)
                .unwrap_or(&0);
            if time_counters <= 0 {
                continue;
            }
            // Remove one time counter
            game.card_mut(card_id)
                .remove_counter(&crate::card::CounterType::Time, 1);

            // Emit CounterRemoved trigger
            self.trigger_handler.run_trigger(
                TriggerType::CounterRemoved,
                RunParams {
                    card: Some(card_id),
                    player: Some(active),
                    ..Default::default()
                },
                false,
            );

            let remaining = *game
                .card(card_id)
                .counters
                .get(&crate::card::CounterType::Time)
                .unwrap_or(&0);
            if remaining <= 0 {
                // Last counter removed — cast for free
                let card_name = game.card(card_id).card_name.clone();
                let is_creature = game.card(card_id).is_creature();
                let is_permanent = game.card(card_id).is_permanent();
                let abilities = game.card(card_id).abilities.clone();

                // Move from exile to stack
                game.player_mut(active).spells_cast_this_turn += 1;

                // Emit SpellCast trigger
                self.trigger_handler.run_trigger(
                    TriggerType::SpellCast,
                    RunParams {
                        spell_card: Some(card_id),
                        spell_controller: Some(active),
                        ..Default::default()
                    },
                    false,
                );

                // Build SpellAbility
                let ability_text = abilities.first().cloned().unwrap_or_default();
                let mut sa = build_spell_ability(game, card_id, &ability_text, active);
                sa.is_spell = true;
                sa.alt_cost = Some(crate::spellability::AlternativeCost::Suspend);
                sa.setup_targets(game, agents, &self.mana_pools);

                let entry = StackEntry {
                    id: 0,
                    spell_ability: sa,
                    is_creature_spell: is_creature,
                    is_permanent_spell: is_permanent,
                    cast_from_zone: Some(ZoneType::Exile),
                };

                game.stack.push(entry);
                self.log_stack_push(&card_name, &game.player(active).name);
                game.move_card(card_id, ZoneType::Stack, active);
                crate::agent::notify_all_agents(
                    agents,
                    crate::agent::GameLogEvent::stack(format!(
                        "Suspend: casting {} for free!",
                        card_name
                    ))
                    .with_player(active)
                    .with_card(card_id),
                );

                // Grant haste if creature (suspend creatures get haste)
                if is_creature {
                    if !game.card(card_id).has_haste() {
                        game.card_mut(card_id)
                            .granted_keywords
                            .push("Haste".to_string());
                    }
                }
            }
        }
    }
}
