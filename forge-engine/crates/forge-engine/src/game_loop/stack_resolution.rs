use super::cost_payment::CostPaymentContext;
use super::*;
use crate::spellability::TargetKind;

impl GameLoop {
    fn effect_kind_for_sa(sa: &SpellAbility) -> String {
        if let Some(api) = sa.api {
            return api.name().to_string();
        }
        if let Some(kind) = sa
            .params
            .get(keys::SP)
            .or_else(|| sa.params.get(keys::DB))
            .or_else(|| sa.params.get(keys::AB))
        {
            return kind.to_string();
        }
        if sa.is_trigger {
            if let Some(mode) = sa.params.get(keys::MODE) {
                return format!("Trigger({mode})");
            }
            return "Trigger".to_string();
        }
        if sa.is_activated {
            return "ActivatedAbility".to_string();
        }
        if sa.is_spell {
            return "Spell".to_string();
        }
        "Effect".to_string()
    }

    pub fn resolve_stack(&mut self, game: &mut GameState, agents: &mut [Box<dyn PlayerAgent>]) {
        if game.stack.is_empty() {
            return;
        }

        // LKI: Snapshot battlefield state before resolution.
        // Mirrors Java MagicStack line 623: game.copyLastState() before resolving.
        game.copy_last_state();

        let entry = game.stack.resolve_stack().unwrap();
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
            super::check_sba(game, &mut self.trigger_handler, agents);
            self.process_triggers(game, agents);
            return;
        }

        // Fizzle check — mirrors Java's MagicStack.hasFizzled() (CR 608.2b).
        // A spell or ability is countered by game rules if ALL of its targets
        // are illegal on resolution. Walk the SA chain; if every targeting node
        // has only invalid targets, the whole thing fizzles.
        if std::env::var("FORGE_TRIGGER_TRACE").is_ok() && entry.optional_trigger_decider.is_some() {
            eprintln!("[trigger-trace] RESOLVING optional trigger from stack: {} api={:?}", stack_item_name, entry.spell_ability.api);
        }
        if Self::has_fizzled(&entry.spell_ability, game) {
            crate::agent::notify_all_agents(
                agents,
                crate::agent::GameLogEvent::warning(format!(
                    "{} fizzles (all targets invalid)",
                    stack_item_name
                ))
                .with_player(entry.spell_ability.activating_player),
            );
            // CR 608.2b: A countered spell is still put into its owner's
            // graveyard (or exile for flashback/escape). Only triggers and
            // activated abilities have no physical card to move.
            if !entry.spell_ability.is_trigger
                && !entry.spell_ability.is_activated
                && !entry.spell_ability.is_copy
            {
                if let Some(card_id) = entry.spell_ability.source {
                    let owner = game.card(card_id).owner;
                    let dest = if entry.spell_ability.alt_cost
                        == Some(crate::spellability::AlternativeCost::Flashback)
                        || entry.spell_ability.alt_cost
                            == Some(crate::spellability::AlternativeCost::Escape)
                    {
                        ZoneType::Exile
                    } else {
                        ZoneType::Graveyard
                    };
                    game.move_card(card_id, dest, owner);
                }
            }
            apply_continuous_effects(game);
            super::check_sba(game, &mut self.trigger_handler, agents);
            self.process_triggers(game, agents);
            return;
        }

        if entry.spell_ability.is_trigger || entry.spell_ability.is_activated {
            // Optional trigger confirmation — mirrors Java's WrappedAbility.resolve()
            // calling confirmTrigger() FIRST, before cost payment or effect resolution.
            // This happens at resolution time, AFTER the trigger has been on the stack
            // and priority has passed.
            if let Some(decider) = entry.optional_trigger_decider {
                let description = entry.optional_trigger_description.as_deref().unwrap_or("");
                let source_name = entry.optional_trigger_source_name.as_deref();
                let api = entry.spell_ability.api;
                let accepted = agents[decider.index()].choose_optional_trigger(
                    decider,
                    description,
                    source_name,
                    api,
                );
                if !accepted {
                    // Player declined — trigger does nothing.
                    // For Madness triggers: move the source card from exile to graveyard.
                    // Mirrors Java's Madness cleanup when the trigger is declined.
                    if entry
                        .spell_ability
                        .param_is_true(crate::card::PARAM_MADNESS_PLAY)
                    {
                        if let Some(source_id) = entry.spell_ability.source {
                            if game.card(source_id).zone == ZoneType::Exile {
                                let owner = game.card(source_id).owner;
                                game.move_card(source_id, ZoneType::Graveyard, owner);
                                crate::ability::effects::helpers::remove_madness_exiled_marker(
                                    game.card_mut(source_id),
                                );
                            }
                        }
                    }
                    apply_continuous_effects(game);
                    super::check_sba(game, &mut self.trigger_handler, agents);
                    self.process_triggers(game, agents);
                    return;
                }
            }

            // Check if the triggered/activated ability has a mana cost that must be paid.
            // Mirrors Java's trigger resolution: if Cost$ is present, the player pays
            // when the ability resolves. If they can't pay, the ability does nothing.
            //
            // For triggers with costs (e.g. Roar of Resistance's "you may pay {1}{R}"),
            // optionality lives in cost payment decisions. Mirror Java by routing
            // through per-cost-part confirmations (`confirmPayment` parity hook).
            if entry.spell_ability.is_trigger {
                if let Some(cost) = &entry.spell_ability.pay_costs {
                    let player = entry.spell_ability.activating_player;
                    let source = entry.spell_ability.source.unwrap_or(CardId(0));
                    let api = entry.spell_ability.api;
                    let available = crate::mana::calculate_available_mana(
                        &self.mana_pools[player.index()],
                        game,
                        player,
                    );
                    if !crate::cost::can_pay_with_ability(
                        cost,
                        game,
                        &available,
                        source,
                        player,
                        Some(&entry.spell_ability),
                    ) {
                        // Can't pay the cost — ability fizzles
                        apply_continuous_effects(game);
                        super::check_sba(game, &mut self.trigger_handler, agents);
                        self.process_triggers(game, agents);
                        return;
                    }
                    if !self.pay_ability_cost(
                        game,
                        agents,
                        player,
                        source,
                        cost,
                        api,
                        cost.mandatory,
                        CostPaymentContext::TriggerResolve,
                    ) {
                        apply_continuous_effects(game);
                        super::check_sba(game, &mut self.trigger_handler, agents);
                        self.process_triggers(game, agents);
                        return;
                    }
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
                    .get(keys::PRECOST_DESC)
                    .map_or(false, |d| d.to_lowercase().contains("cycling"));
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

                // Ninjutsu: add the ninja to combat as an attacker.
                // The change_zone_effect already set attacking_player; here we
                // register it in the CombatState so it participates in damage.
                if entry.spell_ability.param_is_true(keys::NINJUTSU) {
                    if let Some(source_card) = entry.spell_ability.source {
                        if game.card(source_card).zone == ZoneType::Battlefield {
                            if let Some(def_pid) = game.card(source_card).attacking_player {
                                let defender = crate::combat::DefenderId::Player(def_pid);
                                self.combat.declare_attacker(source_card, defender);
                            }
                        }
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
                    let keywords = game.card(card_id).keywords.as_string_list();
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
                                let params = Params::from_raw(&svar_text);
                                params
                                    .get(keys::SPELL_DESCRIPTION)
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
                        let mut parent_target_player = None;
                        let mut current_sa = Some(&etb_sa);
                        while let Some(sa) = current_sa {
                            let mut sa_with_ctx;
                            let sa_ref =
                                if parent_target_player.is_some()
                                    && sa.target_chosen.target_player.is_none()
                                {
                                    sa_with_ctx = sa.clone();
                                    sa_with_ctx.target_chosen.target_player = parent_target_player;
                                    &sa_with_ctx
                                } else {
                                    sa
                                };
                            self.resolve_single_effect(game, agents, sa_ref, parent_target_card);
                            parent_target_card = sa_ref.target_chosen.target_card;
                            parent_target_player = sa_ref.target_chosen.target_player;
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
                    let type_name = filter_str
                        .split('/')
                        .next()
                        .unwrap_or(&filter_str)
                        .to_string();
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
                    let pay = agents[player.index()].choose_optional_trigger(
                        player,
                        &desc,
                        Some(&cname),
                        None,
                    );
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
                        .add("Haste");
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

                // Warp: mark card so it can be cast from exile on a later turn,
                // and register delayed trigger to exile at EOT.
                // Mirrors Java PermanentEffect + StaticAbilityCastWithFlash for Warp.
                if alt_cost == Some(crate::spellability::AlternativeCost::Warp) {
                    game.card_mut(card_id)
                        .keywords
                        .add(crate::card::KEYWORD_WARP_EXILED);
                    self.trigger_handler.register_delayed_trigger(
                        crate::trigger::handler::DelayedTrigger {
                            mode: TriggerType::Phase,
                            trigger_mode: crate::trigger::TriggerMode::Phase {
                                phase: Some(forge_foundation::PhaseType::EndOfTurn),
                                valid_player: None,
                            },
                            execute_svar: format!(
                                "DB$ ChangeZone | Origin$ Battlefield | Destination$ Exile | Defined$ CardUID_{}", card_id.0
                            ),
                            controller: player,
                            source_card: card_id,
                            target_card: Some(card_id),
                            remembered_amount: 0,
                        },
                    );
                }

                // Bestow: attach to a creature as an Aura
                if alt_cost == Some(crate::spellability::AlternativeCost::Bestow) {
                    // Find a creature to attach to (agent chooses)
                    let creatures: Vec<crate::ids::CardId> = game
                        .cards_in_zone(ZoneType::Battlefield, player)
                        .iter()
                        .chain(
                            game.cards_in_zone(ZoneType::Battlefield, game.opponent_of(player))
                                .iter(),
                        )
                        .copied()
                        .filter(|&cid| cid != card_id && game.card(cid).is_creature())
                        .collect();
                    if !creatures.is_empty() {
                        agents[player.index()].snapshot_state(game, &self.mana_pools);
                        if let Some(target) =
                            agents[player.index()].choose_sacrifice(player, &creatures)
                        {
                            game.card_mut(card_id).is_bestowed = true;
                            game.attach_to(card_id, target);
                        }
                    }
                }

                // Morph/Megamorph: enter face-down as a 2/2 creature
                if alt_cost.map_or(false, |ac| ac.is_morph()) {
                    let is_mega = alt_cost == Some(crate::spellability::AlternativeCost::Megamorph);
                    let c = game.card_mut(card_id);
                    c.face_down = true;
                    c.static_set_power = Some(crate::spellability::MORPH_PT);
                    c.static_set_toughness = Some(crate::spellability::MORPH_PT);

                    // Add "turn face up" activated ability (morph cost → SetState TurnFaceUp).
                    // This is a game rule, not a card ability — face-down morph creatures
                    // can always be turned face up by paying the morph cost.
                    let morph_cost = c
                        .get_keyword_cost(if is_mega { "Megamorph" } else { "Morph" })
                        .unwrap_or_else(|| "3".to_string());
                    let mega_param = if is_mega { " | Mega$ True" } else { "" };
                    let ab_text = format!(
                        "AB$ SetState | Cost$ {} | Mode$ TurnFaceUp{}",
                        morph_cost, mega_param
                    );
                    let ab_index = c.activated_abilities.len();
                    if let Some(parsed) =
                        crate::ability::activated::parse_activated_ability(&ab_text, ab_index)
                    {
                        c.activated_abilities.push(parsed);
                    }
                }

                // Blitz: grant haste + "dies: draw a card" + sacrifice at EOT
                if alt_cost == Some(crate::spellability::AlternativeCost::Blitz) {
                    game.card_mut(card_id)
                        .pump_keywords
                        .add("Haste");
                    let trig_id = game.card(card_id).triggers.len() as u32;
                    let dies_trigger = crate::trigger::Trigger {
                        id: trig_id,
                        mode: crate::trigger::TriggerMode::ChangesZone {
                            origin: Some(ZoneType::Battlefield),
                            destination: Some(ZoneType::Graveyard),
                            valid_card: Some("Card.Self".to_string()),
                        },
                        params: crate::parsing::Params::default(),
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

        // Mark resolution complete on the stack.
        game.stack.finish_resolving();

        // Continuous effects might change after resolution
        apply_continuous_effects(game);
        super::check_sba(game, &mut self.trigger_handler, agents);

        // LKI: Second snapshot after resolution and SBAs, before processing triggers.
        // Mirrors Java MagicStack line 676: game.copyLastState() in finishResolving().
        // This captures cards that entered the battlefield during this resolution
        // (e.g., creatures reanimated by Exhume) so their LKI is available
        // when they later leave the battlefield.
        game.copy_last_state();

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
        let mut parent_target_player = None;
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
            let mut sa_with_ctx;
            let needs_ctx_clone = (root_kicked && !sa.kicked)
                || (parent_target_player.is_some() && sa.target_chosen.target_player.is_none());
            let sa_ref = if needs_ctx_clone {
                sa_with_ctx = sa.clone();
                if root_kicked && !sa_with_ctx.kicked {
                    sa_with_ctx.kicked = true;
                }
                if sa_with_ctx.target_chosen.target_player.is_none() {
                    sa_with_ctx.target_chosen.target_player = parent_target_player;
                }
                &sa_with_ctx
            } else {
                sa
            };
            self.resolve_single_effect(game, agents, sa_ref, parent_target_card);
            // This SA's target card becomes the parent context for the next sub-ability.
            parent_target_card = sa_ref.target_chosen.target_card;
            parent_target_player = sa_ref.target_chosen.target_player;
            current = sa.get_sub_ability();
        }
    }

    /// Check whether a spell/ability should fizzle (CR 608.2b).
    /// Mirrors Java's `MagicStack.hasFizzled()`.
    ///
    /// Walks the SpellAbility chain. If every targeting node has only invalid
    /// targets, the whole spell/ability is countered by game rules.
    /// Returns `false` if no node uses targeting at all.
    fn has_fizzled(sa: &SpellAbility, game: &GameState) -> bool {
        let result = Self::has_fizzled_inner(sa, game, None);
        // Java: `return fizzle != null && fizzle;`
        result.unwrap_or(false)
    }

    /// Recursive helper mirroring Java's `hasFizzled(sa, source, fizzle)`.
    /// Returns `Option<bool>`:
    ///   `None`        = no targeting node seen in chain yet
    ///   `Some(true)`  = all targeting nodes have only invalid targets
    ///   `Some(false)` = at least one valid target found somewhere
    fn has_fizzled_inner(
        sa: &SpellAbility,
        game: &GameState,
        mut fizzle: Option<bool>,
    ) -> Option<bool> {
        if sa.uses_targeting() {
            // Check if we actually have any chosen targets (mirrors Java's
            // `!sa.isZeroTargets()` — Rust stores at most one target per slot
            // so having any slot filled means non-zero targets)
            let has_any_chosen = sa.target_chosen.target_card.is_some()
                || sa.target_chosen.target_player.is_some()
                || sa.target_chosen.target_stack_entry.is_some();

            if has_any_chosen {
                // This node uses targeting and has chosen targets — fizzling
                // is now possible.
                if fizzle.is_none() {
                    fizzle = Some(true);
                }

                // Check each chosen target. If ANY is still valid, fizzle = false.
                // Mirrors Java's for loop over `sa.getTargets()`.

                if let Some(target_card_id) = sa.target_chosen.target_card {
                    if Self::is_card_target_valid(sa, target_card_id, game) {
                        fizzle = Some(false);
                    }
                }

                if let Some(target_player_id) = sa.target_chosen.target_player {
                    if Self::is_player_target_valid(target_player_id, game) {
                        fizzle = Some(false);
                    }
                }

                if let Some(target_stack_id) = sa.target_chosen.target_stack_entry {
                    if game.stack.find_by_id(target_stack_id).is_some() {
                        fizzle = Some(false);
                    }
                }

                // CantFizzle param (e.g. Gilded Drake) overrides fizzle
                if sa.params.has(keys::CANT_FIZZLE) {
                    fizzle = Some(false);
                }
            }
        }

        // Recurse into sub-abilities — mirrors Java's:
        //   if (sa.getSubAbility() != null)
        //       fizzle = hasFizzled(sa.getSubAbility(), source, fizzle);
        if let Some(sub) = sa.get_sub_ability() {
            fizzle = Self::has_fizzled_inner(sub, game, fizzle);
        }

        fizzle
    }

    /// Check if a card target is still valid at resolution time.
    /// The card must still be in a zone that makes it a legal target:
    /// - For Battlefield targets (Creature/Permanent/Any): must be on battlefield
    /// - For CardInZone targets: must be in the specified zone
    /// - The card must also still be targetable (hexproof etc.)
    fn is_card_target_valid(sa: &SpellAbility, target_card_id: CardId, game: &GameState) -> bool {
        // Check if card index is valid
        if target_card_id.index() >= game.cards.len() {
            return false;
        }

        let card = game.card(target_card_id);

        // Determine expected zone from target restrictions
        let expected_zone = if let Some(ref tr) = sa.target_restrictions {
            match &tr.target_kind {
                TargetKind::Creature(_) | TargetKind::Permanent(_) | TargetKind::Any => {
                    Some(ZoneType::Battlefield)
                }
                TargetKind::CardInZone { zone, .. } => Some(*zone),
                _ => Some(ZoneType::Battlefield), // default
            }
        } else {
            Some(ZoneType::Battlefield) // default
        };

        // Card must be in the expected zone
        if let Some(zone) = expected_zone {
            if card.zone != zone {
                return false;
            }
        }

        // Card must still be targetable (hexproof, shroud, protection, etc.)
        // Use the activating player as the source controller
        crate::spellability::target_restrictions::can_be_targeted_by_sa(
            game,
            target_card_id,
            sa.activating_player,
            sa,
        )
    }

    /// Check if a player target is still valid (player must still be alive).
    fn is_player_target_valid(target_player_id: PlayerId, game: &GameState) -> bool {
        if target_player_id.index() >= game.players.len() {
            return false;
        }
        !game.player(target_player_id).has_lost
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
        let effect_kind = Self::effect_kind_for_sa(sa);
        let mut event = crate::agent::GameLogEvent::stack(format!(
            "Effect resolved: {} | source={}",
            effect_kind, source_name
        ))
        .with_player(sa.activating_player);
        if let Some(source_id) = sa.source {
            event = event.with_source_card(source_id);
        }
        if let Some(target_id) = sa.target_chosen.target_card {
            event = event.with_target_card(target_id);
        }
        crate::agent::notify_all_agents(agents, event);

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
