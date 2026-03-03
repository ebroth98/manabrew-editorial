use super::*;

impl GameLoop {
    pub fn resolve_stack(&mut self, game: &mut GameState, agents: &mut [Box<dyn PlayerAgent>]) {
        if game.stack.is_empty() {
            return;
        }

        let entry = game.stack.pop().unwrap();
        let stack_item_name = entry
            .spell_ability
            .source
            .and_then(|cid| game.cards.get(cid.index()).map(|c| c.card_name.clone()))
            .unwrap_or_else(|| "Ability".to_string());
        self.log_stack_resolved_item(&stack_item_name);

        // Storm/copy spells: resolve effect only, no card movement (copies have no physical card)
        if entry.spell_ability.is_copy {
            self.resolve_spell_effect(game, agents, &entry);
            apply_continuous_effects(game);
            game.check_state_based_actions_with_triggers(Some(&mut self.trigger_handler));
            self.process_triggers(game, agents);
            return;
        }

        if entry.spell_ability.is_trigger || entry.spell_ability.is_activated {
            // Check if the triggered/activated ability has a mana cost that must be paid.
            // Mirrors Java's trigger resolution: if Cost$ is present, the player pays
            // when the ability resolves. If they can't pay, the ability does nothing.
            //
            // For triggers with costs (e.g. Roar of Resistance's "you may pay {1}{R}"),
            // the optionality lives in the cost payment, not in OptionalDecider$. Java's
            // AI brain evaluates whether the cost is worth paying (e.g. PumpAllAi declines).
            // We mirror this by asking the agent before paying.
            if entry.spell_ability.is_trigger {
                if let Some(cost) = &entry.spell_ability.pay_costs {
                    let player = entry.spell_ability.activating_player;
                    let source = entry.spell_ability.source.unwrap_or(CardId(0));

                    // Ask agent if they want to pay — mirrors Java's AI brain evaluation
                    // (e.g. PumpAllAi.doTriggerNoCost() declining optional pump costs).
                    let api = entry.spell_ability.api.as_deref();
                    let source_name = entry
                        .spell_ability
                        .source
                        .and_then(|cid| game.cards.get(cid.index()).map(|c| c.card_name.clone()))
                        .unwrap_or_else(|| "Ability".to_string());
                    agents[player.index()].snapshot_state(game, &self.mana_pools);
                    let wants_to_pay = agents[player.index()].choose_optional_trigger(
                        player,
                        &format!("Pay trigger cost for {}?", source_name),
                        Some(&source_name),
                        api,
                    );
                    if !wants_to_pay {
                        // Agent declined to pay — ability does nothing
                        apply_continuous_effects(game);
                        game.check_state_based_actions_with_triggers(Some(&mut self.trigger_handler));
                        self.process_triggers(game, agents);
                        return;
                    }

                    let available = crate::mana::calculate_available_mana(
                        &self.mana_pools[player.index()],
                        game,
                        player,
                    );
                    if !crate::cost::can_pay(cost, game, &available, source, player) {
                        // Can't pay the cost — ability fizzles
                        apply_continuous_effects(game);
                        game.check_state_based_actions_with_triggers(Some(&mut self.trigger_handler));
                        self.process_triggers(game, agents);
                        return;
                    }
                    // Pay the cost
                    self.pay_ability_cost(game, agents, player, source, cost);
                }
            }
            // Triggered/activated ability: resolve the effect
            self.resolve_spell_effect(game, agents, &entry);

            // Fire Cycled trigger if this was a cycling ability
            // (mirrors Java MagicStack resolve → Player.addCycled)
            if entry.spell_ability.is_activated {
                let is_cycling = entry
                    .spell_ability
                    .params
                    .get("PrecostDesc")
                    .map_or(false, |d| d.contains("Cycling"));
                if is_cycling {
                    if let Some(source_card) = entry.spell_ability.source {
                        self.trigger_handler.run_trigger(
                            TriggerType::Cycled,
                            RunParams {
                                card: Some(source_card),
                                player: Some(entry.spell_ability.activating_player),
                                ..Default::default()
                            },
                            false,
                        );
                    }
                }
            }
        } else if entry.spell_ability.is_copy {
            // Copy of a spell (from Replicate/Storm): resolve effect only, no card movement
            self.resolve_spell_effect(game, agents, &entry);
        } else if let Some(card_id) = entry.spell_ability.source {
            let alt_cost = entry.spell_ability.alt_cost;
            let player = entry.spell_ability.activating_player;

            if entry.is_creature_spell || entry.is_permanent_spell {
                // Permanent spell: move to battlefield
                let origin = game.card(card_id).zone;

                // Propagate kicked flag to the card so triggers with
                // ValidCard$ Card.Self+kicked can check it after resolution.
                if entry.spell_ability.kicked {
                    game.card_mut(card_id).kicked = true;
                }

                // Resolve any ETB effects defined on the card
                self.resolve_spell_effect(game, agents, &entry);

                // Process ETBReplacement keywords (e.g., Clone entering as copy).
                // Mirrors Java's CardFactoryUtil.createETBReplacement — the keyword
                // format is ETBReplacement:Layer:SVarName[:Optional[:ValidCard[:Zone]]].
                {
                    let keywords = game.card(card_id).keywords.clone();
                    for kw in &keywords {
                        if !kw.starts_with("ETBReplacement") {
                            continue;
                        }
                        let parts: Vec<&str> = kw.split(':').collect();
                        // Need at least ETBReplacement:Layer:SVarName
                        if parts.len() < 3 {
                            continue;
                        }
                        let svar_name = parts[2];
                        let is_optional = parts
                            .get(3)
                            .map(|s| s.contains("Optional"))
                            .unwrap_or(false);

                        // Look up the SVar on the card
                        let svar_text = match game.card(card_id).svars.get(svar_name).cloned() {
                            Some(text) => text,
                            None => continue,
                        };

                        // For Optional replacements, ask the player
                        if is_optional {
                            let card_name = game.card(card_id).card_name.clone();
                            // Extract SpellDescription from the SVar for a better prompt
                            let desc = {
                                let params = parse_pipe_params(&svar_text);
                                params
                                    .get("SpellDescription")
                                    .map(|d| d.replace("CARDNAME", &card_name))
                                    .unwrap_or_else(|| {
                                        format!("Use {} replacement ability?", card_name)
                                    })
                            };
                            agents[player.index()].snapshot_state(game, &self.mana_pools);
                            let accept = agents[player.index()].choose_optional_trigger(
                                player,
                                &desc,
                                Some(&card_name),
                                None,
                            );
                            if !accept {
                                continue;
                            }
                        }

                        // Build the replacement ability from the SVar
                        let mut etb_sa = build_spell_ability(game, card_id, &svar_text, player);

                        // Set up targeting if the ability uses it
                        if etb_sa.uses_targeting() {
                            agents[player.index()].snapshot_state(game, &self.mana_pools);
                            if !etb_sa.setup_targets(game, agents, &self.mana_pools) {
                                continue; // Targeting failed
                            }
                        }

                        // Resolve the effect chain (walk sub-abilities)
                        let mut parent_target_card: Option<CardId> = None;
                        let mut current_sa = Some(&etb_sa);
                        while let Some(sa) = current_sa {
                            self.resolve_single_effect(game, agents, sa, parent_target_card);
                            parent_target_card = sa.target_chosen.target_card;
                            current_sa = sa.get_sub_ability();
                        }
                    }
                }

                // Check for shock-land-style "pay life or enter tapped" before entering
                let etb_life_cost =
                    crate::staticability::layer::get_etb_unless_life_cost(game.card(card_id));
                // Check for "reveal <type> from hand or enter tapped"
                let etb_reveal_cost =
                    crate::staticability::layer::get_etb_unless_reveal_cost(game.card(card_id));

                game.move_card(card_id, ZoneType::Battlefield, player);

                // Handle reveal-or-enter-tapped
                if let Some((_n, filter_str)) = etb_reveal_cost {
                    let type_name = filter_str.split('/').next().unwrap_or(&filter_str).to_string();
                    let has_matching = game
                        .cards_in_zone(ZoneType::Hand, player)
                        .iter()
                        .any(|&cid| game.card(cid).type_line.has_subtype(&type_name));
                    if !has_matching {
                        game.card_mut(card_id).tapped = true;
                    } else {
                        // DeterministicAgent always passes optional reveals (enter tapped)
                        game.card_mut(card_id).tapped = true;
                    }
                }

                // Prompt for shock land life payment
                if let Some(life_cost) = etb_life_cost {
                    let cname = game.card(card_id).card_name.clone();
                    let desc = format!("Pay {} life so {} enters untapped?", life_cost, cname);
                    agents[player.index()].snapshot_state(game, &self.mana_pools);
                    let pay =
                        agents[player.index()].choose_optional_trigger(player, &desc, Some(&cname), None);
                    if pay {
                        game.card_mut(card_id).tapped = false;
                        game.player_mut(player).lose_life(life_cost);
                        self.trigger_handler.run_trigger(
                            TriggerType::LifeLost,
                            RunParams {
                                player: Some(player),
                                life_amount: Some(life_cost),
                                ..Default::default()
                            },
                            false,
                        );
                    } else {
                        game.card_mut(card_id).tapped = true;
                    }
                }

                // Register triggers for the new permanent
                self.trigger_handler.register_active_trigger(game, card_id);

                // Emit ChangesZone trigger (ETB)
                crate::ability::effects::emit_zone_trigger(
                    &mut self.trigger_handler,
                    card_id,
                    origin,
                    ZoneType::Battlefield,
                );

                // -- Post-ETB effects for alternative costs --

                // Evoke: register a one-shot ETB trigger that sacrifices this creature.
                // This mirrors Forge Java semantics where Evoke uses a ChangesZone trigger
                // and allows normal ETB abilities to trigger before the sacrifice resolves.
                if alt_cost == Some(crate::spellability::AlternativeCost::Evoke) {
                    self.trigger_handler.register_delayed_trigger(
                        crate::trigger::handler::DelayedTrigger {
                            mode: TriggerType::ChangesZone,
                            trigger_mode: crate::trigger::TriggerMode::ChangesZone {
                                origin: None,
                                destination: Some(ZoneType::Battlefield),
                                valid_card: Some("Card.Self".to_string()),
                            },
                            execute_svar: "DB$ Sacrifice".to_string(),
                            controller: player,
                            source_card: card_id,
                            target_card: Some(card_id),
                            remembered_amount: 0,
                        },
                    );
                }

                // Dash: grant haste, register delayed trigger to return to hand at EOT
                if alt_cost == Some(crate::spellability::AlternativeCost::Dash) {
                    game.card_mut(card_id)
                        .pump_keywords
                        .push("Haste".to_string());
                    self.trigger_handler.register_delayed_trigger(
                        crate::trigger::handler::DelayedTrigger {
                            mode: TriggerType::Phase,
                            trigger_mode: crate::trigger::TriggerMode::Phase {
                                phase: Some(forge_foundation::PhaseType::EndOfTurn),
                                valid_player: None,
                            },
                            execute_svar: format!(
                                "DB$ ChangeZone | Origin$ Battlefield | Destination$ Hand | Defined$ CardUID_{}", card_id.0
                            ),
                            controller: player,
                            source_card: card_id,
                            target_card: Some(card_id),
                            remembered_amount: 0,
                        },
                    );
                }

                // Blitz: grant haste + "dies: draw a card" + sacrifice at EOT
                if alt_cost == Some(crate::spellability::AlternativeCost::Blitz) {
                    game.card_mut(card_id)
                        .pump_keywords
                        .push("Haste".to_string());
                    let trig_id = game.card(card_id).triggers.len() as u32;
                    let dies_trigger = crate::trigger::Trigger {
                        id: trig_id,
                        mode: crate::trigger::TriggerMode::ChangesZone {
                            origin: Some(ZoneType::Battlefield),
                            destination: Some(ZoneType::Graveyard),
                            valid_card: Some("Card.Self".to_string()),
                        },
                        params: std::collections::BTreeMap::new(),
                        active_zones: vec![ZoneType::Battlefield],
                        execute: "BlitzDiesDraw".to_string(),
                        optional: false,
                        description: "When this creature dies, draw a card.".to_string(),
                        intrinsic: false,
                    };
                    game.card_mut(card_id).triggers.push(dies_trigger);
                    game.card_mut(card_id).svars.insert(
                        "BlitzDiesDraw".to_string(),
                        "DB$ Draw | NumCards$ 1 | Defined$ You".to_string(),
                    );
                    self.trigger_handler.unregister_active_triggers(card_id);
                    self.trigger_handler.register_active_trigger(game, card_id);

                    self.trigger_handler.register_delayed_trigger(
                        crate::trigger::handler::DelayedTrigger {
                            mode: TriggerType::Phase,
                            trigger_mode: crate::trigger::TriggerMode::Phase {
                                phase: Some(forge_foundation::PhaseType::EndOfTurn),
                                valid_player: None,
                            },
                            execute_svar: format!("DB$ Sacrifice | Defined$ CardUID_{}", card_id.0),
                            controller: player,
                            source_card: card_id,
                            target_card: Some(card_id),
                            remembered_amount: 0,
                        },
                    );
                }
            } else {
                // Non-permanent spell: resolve effect, then route to destination zone
                self.resolve_spell_effect(game, agents, &entry);
                let owner = game.card(card_id).owner;
                // Only move if still in stack zone (some effects move the card themselves)
                if game.card(card_id).zone != ZoneType::Exile
                    && game.card(card_id).zone != ZoneType::Library
                    && game.card(card_id).zone != ZoneType::Hand
                {
                    // Determine destination based on alternative cost / keywords
                    let dest = if alt_cost == Some(crate::spellability::AlternativeCost::Flashback)
                        || alt_cost == Some(crate::spellability::AlternativeCost::Escape)
                    {
                        ZoneType::Exile
                    } else if entry.spell_ability.buyback_paid {
                        // Buyback: return to hand instead of graveyard
                        ZoneType::Hand
                    } else if game.card(card_id).has_rebound()
                        && entry.cast_from_zone == Some(ZoneType::Hand)
                    {
                        // Rebound: exile instead of graveyard (will be cast next upkeep)
                        self.trigger_handler.register_delayed_trigger(
                            crate::trigger::handler::DelayedTrigger {
                                mode: TriggerType::Phase,
                                trigger_mode: crate::trigger::TriggerMode::Phase {
                                    phase: Some(forge_foundation::PhaseType::Upkeep),
                                    valid_player: Some("You".to_string()),
                                },
                                execute_svar: format!(
                                    "DB$ Play | Defined$ CardUID_{} | WithoutManaCost$ True",
                                    card_id.0
                                ),
                                controller: player,
                                source_card: card_id,
                                target_card: Some(card_id),
                                remembered_amount: 0,
                            },
                        );
                        ZoneType::Exile
                    } else {
                        ZoneType::Graveyard
                    };
                    game.move_card(card_id, dest, owner);
                }
            }
        }

        // Continuous effects might change after resolution
        apply_continuous_effects(game);
        game.check_state_based_actions_with_triggers(Some(&mut self.trigger_handler));

        // Process triggers that may have fired during resolution
        self.process_triggers(game, agents);
    }

    pub(crate) fn resolve_spell_effect(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        entry: &StackEntry,
    ) {
        // Walk the SpellAbility chain: resolve each node's effect, propagating
        // the parent SA's chosen target card so sub-abilities can resolve
        // `Defined$ ParentTarget`. Mirrors Java's resolveApiAbility() + resolveSubAbilities().
        let mut parent_target_card: Option<CardId> = None;
        let root_kicked = entry.spell_ability.kicked;
        let mut current = Some(&entry.spell_ability);
        let mut is_first = true;
        while let Some(sa) = current {
            // Refresh agent snapshots between sub-abilities so that prompts
            // (e.g. "choose discard") reflect state changes from earlier
            // sub-abilities (e.g. "draw 2" before "discard 2").
            if !is_first {
                for agent in agents.iter_mut() {
                    agent.snapshot_state(game, &self.mana_pools);
                }
            }
            is_first = false;

            // Propagate kicked flag from root SA to sub-abilities for condition checks
            let mut sa_with_kicked;
            let sa_ref = if root_kicked && !sa.kicked {
                sa_with_kicked = sa.clone();
                sa_with_kicked.kicked = true;
                &sa_with_kicked
            } else {
                sa
            };
            self.resolve_single_effect(game, agents, sa_ref, parent_target_card);
            // This SA's target card becomes the parent context for the next sub-ability.
            parent_target_card = sa.target_chosen.target_card;
            current = sa.get_sub_ability();
        }
    }

    /// Resolve a single effect line by delegating to the effects module.
    pub(crate) fn resolve_single_effect(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        sa: &SpellAbility,
        parent_target_card: Option<CardId>,
    ) {
        let source_name = sa
            .source
            .and_then(|cid| game.cards.get(cid.index()).map(|c| c.card_name.clone()))
            .unwrap_or_else(|| "Unknown source".to_string());
        let effect_kind = sa.api.clone().unwrap_or_else(|| "Unknown".to_string());
        agents[sa.activating_player.index()].notify(&format!(
            "Effect resolved: {} | source={}",
            effect_kind, source_name
        ));

        let mut ctx = EffectContext {
            game,
            agents,
            trigger_handler: &mut self.trigger_handler,
            token_templates: &self.token_templates,
            mana_pools: &mut self.mana_pools,
            parent_target_card,
            rng: &mut *self.game_rng,
        };
        effects::resolve_effect(&mut ctx, sa);
    }
}
