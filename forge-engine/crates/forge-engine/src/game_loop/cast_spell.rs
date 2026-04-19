use super::mana_payment::ManaPaymentSession;
use super::*;
use crate::mana::mana_cost_being_paid::ManaCostBeingPaid;

impl GameLoop {
    pub(crate) fn parse_spell_cost(abilities: &[String]) -> Option<crate::cost::Cost> {
        for ability in abilities {
            let params = Params::from_raw(ability);
            // Only process SP$ lines (spell abilities)
            if params.has(keys::SP) {
                if let Some(cost_str) = params.get(keys::COST) {
                    return Some(parse_cost(cost_str));
                }
            }
        }
        None
    }

    pub(crate) fn play_land(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        card_id: CardId,
        card_name: &str,
        play_mode: crate::agent::PlayCardMode,
    ) -> Option<(CardId, String)> {
        let is_back_face_land_play = matches!(play_mode, crate::agent::PlayCardMode::BackFaceLand);
        let can_play_back_face_land = is_back_face_land_play
            && game
                .card(card_id)
                .other_part
                .as_ref()
                .is_some_and(|other| other.type_line.is_land());
        if !matches!(
            play_mode,
            crate::agent::PlayCardMode::Normal | crate::agent::PlayCardMode::BackFaceLand
        ) || (is_back_face_land_play && !can_play_back_face_land)
        {
            return None;
        }
        let origin_zone = game.card(card_id).zone;
        let play_name = if can_play_back_face_land {
            game.card(card_id)
                .other_part
                .as_ref()
                .map(|other| other.name.clone())
                .unwrap_or_else(|| card_name.to_string())
        } else {
            card_name.to_string()
        };

        if can_play_back_face_land && !game.card(card_id).is_transformed {
            game.card_mut(card_id).transform();
        }

        self.move_card_with_runtime(game, card_id, ZoneType::Battlefield, player, agents);

        game.player_record_land_play(player);
        crate::agent::notify_all_agents(
            agents,
            crate::agent::GameLogEvent::action(format!("Played land: {}", play_name))
                .with_player(player)
                .with_card(card_id),
        );

        self.trigger_handler.register_active_trigger(game, card_id);
        crate::ability::effects::emit_zone_trigger(
            &mut self.trigger_handler,
            card_id,
            origin_zone,
            ZoneType::Battlefield,
        );
        self.trigger_handler.run_trigger(
            TriggerType::LandPlayed,
            RunParams {
                card: Some(card_id),
                player: Some(player),
                ..Default::default()
            },
            false,
        );
        self.process_triggers(game, agents);

        Some((card_id, play_name))
    }

    pub(crate) fn emit_becomes_target_triggers(
        &mut self,
        game: &mut GameState,
        cause_player: PlayerId,
        cause_card: CardId,
        source_sa: Option<&SpellAbility>,
        target_card: Option<CardId>,
        target_player: Option<PlayerId>,
    ) {
        if let Some(target_id) = target_card {
            let first_time = !game.card(target_id).has_become_target_this_turn();
            game.card_mut(target_id).add_target_from_this_turn();
            self.trigger_handler.run_trigger(
                TriggerType::BecomesTarget,
                RunParams {
                    card: Some(target_id),
                    target_card: Some(target_id),
                    cards: Some(vec![target_id]),
                    cause_player: Some(cause_player),
                    cause_card: Some(cause_card),
                    source_sa: source_sa.cloned(),
                    first_time: Some(first_time),
                    ..Default::default()
                },
                false,
            );
            self.trigger_handler.run_trigger(
                TriggerType::BecomesTargetOnce,
                RunParams {
                    card: Some(target_id),
                    target_card: Some(target_id),
                    cards: Some(vec![target_id]),
                    cause_player: Some(cause_player),
                    cause_card: Some(cause_card),
                    source_sa: source_sa.cloned(),
                    first_time: Some(first_time),
                    ..Default::default()
                },
                false,
            );
        }

        if let Some(target_id) = target_player {
            self.trigger_handler.run_trigger(
                TriggerType::BecomesTarget,
                RunParams {
                    player: Some(target_id),
                    target_player: Some(target_id),
                    cause_player: Some(cause_player),
                    cause_card: Some(cause_card),
                    source_sa: source_sa.cloned(),
                    ..Default::default()
                },
                false,
            );
            self.trigger_handler.run_trigger(
                TriggerType::BecomesTargetOnce,
                RunParams {
                    player: Some(target_id),
                    target_player: Some(target_id),
                    cause_player: Some(cause_player),
                    cause_card: Some(cause_card),
                    source_sa: source_sa.cloned(),
                    ..Default::default()
                },
                false,
            );
        }
    }

    pub(crate) fn push_spell_ability_to_stack(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        stack_push: StackPushContext,
    ) -> SpellAbility {
        if std::env::var("FORGE_STACK_TRACE").is_ok() {
            eprintln!(
                "[stack-trace] PUSH player={:?} source={} depth_before={} msg={}",
                player,
                game.card(stack_push.source_card).card_name,
                game.stack.len(),
                stack_push.stack_message
            );
        }
        game.stack.push(stack_push.entry.clone());
        self.log_stack_push(&stack_push.stack_log_name, &game.player(player).name);
        let mut event = if stack_push.event_kind == SpellAbilityLogEventKind::Stack {
            crate::agent::GameLogEvent::stack(stack_push.stack_message)
        } else {
            crate::agent::GameLogEvent::action(stack_push.stack_message)
        }
        .with_player(player)
        .with_source_card(stack_push.source_card);
        if let Some(target_id) = stack_push.target_card {
            event = event.with_target_card(target_id);
        }
        crate::agent::notify_all_agents(agents, event);

        if stack_push.move_source_to_stack {
            self.move_card_with_runtime(
                game,
                stack_push.source_card,
                ZoneType::Stack,
                player,
                agents,
            );
        }
        if stack_push.register_source_trigger {
            self.trigger_handler
                .register_active_trigger(game, stack_push.source_card);
        }

        if std::env::var("FORGE_STACK_TRACE").is_ok() {
            let names: Vec<String> = game
                .stack
                .iter()
                .map(|entry| {
                    entry
                        .spell_ability
                        .source
                        .map(|cid| game.card(cid).card_name.clone())
                        .unwrap_or_else(|| "<effect>".to_string())
                })
                .collect();
            eprintln!(
                "[stack-trace] STACK after push depth={} {:?}",
                game.stack.len(),
                names
            );
        }
        if let Some(top) = game.stack.iter_mut().last() {
            top.spell_ability.apply_paying_mana_effects();
            top.spell_ability.clone()
        } else {
            stack_push.entry.spell_ability
        }
    }

    pub(crate) fn emit_post_stack_spell_ability_triggers(
        &mut self,
        game: &mut GameState,
        player: PlayerId,
        sa_for_trigger: &SpellAbility,
        trigger_ctx: PostStackTriggerContext,
    ) {
        if trigger_ctx.emit_ability_activated {
            self.trigger_handler.run_trigger(
                TriggerType::AbilityActivated,
                RunParams {
                    card: Some(trigger_ctx.source_card),
                    player: Some(player),
                    ..Default::default()
                },
                false,
            );
        }

        let run_params = match trigger_ctx.cast_trigger {
            TriggerType::SpellCast => RunParams {
                spell_card: Some(trigger_ctx.source_card),
                spell_controller: Some(player),
                spell_ability: Some(sa_for_trigger.clone()),
                source_sa: Some(sa_for_trigger.clone()),
                cause: Some(sa_for_trigger.clone()),
                cause_card: Some(trigger_ctx.source_card),
                ..Default::default()
            },
            TriggerType::AbilityCast => RunParams {
                card: Some(trigger_ctx.source_card),
                spell_card: Some(trigger_ctx.source_card),
                player: Some(player),
                activator: Some(player),
                spell_controller: Some(player),
                spell_ability: Some(sa_for_trigger.clone()),
                source_sa: Some(sa_for_trigger.clone()),
                cause: Some(sa_for_trigger.clone()),
                cause_card: Some(trigger_ctx.source_card),
                ..Default::default()
            },
            _ => return,
        };
        self.trigger_handler
            .run_trigger(trigger_ctx.cast_trigger, run_params, true);

        if trigger_ctx.emit_waterbend {
            let mut bend_params = RunParams {
                player: Some(player),
                card: Some(trigger_ctx.source_card),
                spell_card: Some(trigger_ctx.source_card),
                spell_controller: Some(player),
                spell_ability: Some(sa_for_trigger.clone()),
                source_sa: Some(sa_for_trigger.clone()),
                cause: Some(sa_for_trigger.clone()),
                cause_card: Some(trigger_ctx.source_card),
                ..Default::default()
            };
            if !trigger_ctx.waterbend_cards.is_empty() {
                bend_params.cards = Some(trigger_ctx.waterbend_cards);
            }
            self.trigger_handler
                .run_trigger(TriggerType::Elementalbend, bend_params, false);
        }

        self.emit_becomes_target_triggers(
            game,
            player,
            trigger_ctx.source_card,
            Some(sa_for_trigger),
            sa_for_trigger.target_chosen.target_card,
            sa_for_trigger.target_chosen.target_player,
        );
    }

    /// Orchestrates the full non-land SpellAbility entrypoint after the action
    /// has already been chosen. Card spells and activated abilities both route
    /// through here; only lands stay outside this boundary.
    pub(crate) fn play_spell_ability(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        prepared: PreparedSpellAbility,
    ) -> Option<PlaySpellAbilityResult> {
        if let Some(ability_idx) = prepared.activated_ability_index {
            let card_id = prepared.spell_ability.source?;
            {
                let ab = {
                    let card = game.card(card_id);
                    card.activated_abilities
                        .iter()
                        .find(|a| a.ability_index == ability_idx)
                        .cloned()
                }?;

                let played = if ab.is_mana_ability {
                    self.resolve_mana_ability(game, agents, player, card_id, &ab, None);
                    true
                } else if ab
                    .params
                    .get(keys::AB)
                    .and_then(crate::ability::api_type::ApiType::smart_value_of)
                    == Some(crate::ability::api_type::ApiType::Plot)
                {
                    // Java models Plot as AbilityStatic, which resolves immediately.
                    self.resolve_immediate_ability(game, agents, player, card_id, &ab)
                } else if ab.ability_text.contains("Mode$ TurnFaceUp") {
                    // Morph face-up is a special action: pay the cost and resolve immediately.
                    self.resolve_immediate_ability(game, agents, player, card_id, &ab)
                } else {
                    self.play_activated_ability_on_stack(game, agents, player, card_id, &ab)
                };

                played.then_some(PlaySpellAbilityResult::AbilityActivated)
            }
        } else {
            self.cast_card_spell_ability(
                game,
                agents,
                player,
                prepared.spell_ability,
                prepared.static_alternative_cost_prepared,
            )
            .map(|(card_id, card_name)| PlaySpellAbilityResult::CardPlayed { card_id, card_name })
        }
    }

    pub(crate) fn prepare_card_spell_ability(
        &mut self,
        game: &GameState,
        player: PlayerId,
        card_id: CardId,
        play: crate::agent::PlayOption,
    ) -> Option<PreparedSpellAbility> {
        let play_mode = play.mode;
        let mut sa = crate::spellability::build_spell_ability_for_card_cast(game, card_id, player);
        sa.alt_cost_index = play.alt_cost_index;
        let mut static_alternative_cost_prepared = false;
        match play_mode {
            crate::agent::PlayCardMode::Normal => {}
            crate::agent::PlayCardMode::UnlockDoor
            | crate::agent::PlayCardMode::ForetellExile
            | crate::agent::PlayCardMode::BackFaceLand => return None,
            crate::agent::PlayCardMode::StaticAlternative => {
                let entry =
                    crate::staticability::static_ability_alternative_cost::alternative_costs(
                        game,
                        &game.cards,
                        &sa,
                        game.card(card_id),
                        player,
                    )
                    .into_iter()
                    .find(|entry| {
                        crate::cost::can_pay_ignoring_mana_for_spell(
                            &entry.cost,
                            game,
                            card_id,
                            player,
                        )
                    })?;
                crate::staticability::static_ability_alternative_cost::apply_alternative_cost_to_sa(
                    &mut sa, &entry,
                );
                static_alternative_cost_prepared = true;
            }
            crate::agent::PlayCardMode::Alternative(alt_cost) => {
                if alt_cost == crate::spellability::AlternativeCost::Suspend {
                    return None;
                }
                sa.alt_cost = Some(alt_cost);
            }
        }
        Some(PreparedSpellAbility {
            spell_ability: sa,
            activated_ability_index: None,
            static_alternative_cost_prepared,
        })
    }

    pub(crate) fn play_special_card_action(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        card_id: CardId,
        play_mode: crate::agent::PlayCardMode,
    ) -> Option<Option<(CardId, String)>> {
        let card_name = game.card(card_id).card_name.clone();
        match play_mode {
            crate::agent::PlayCardMode::ForetellExile => {
                if game.card(card_id).get_foretell_cost().is_some()
                    && game.card(card_id).zone == ZoneType::Hand
                {
                    let available_mana =
                        mana::calculate_available_mana(self.pool(player), game, player);
                    let foretell_exile_cost = forge_foundation::ManaCost::generic(2);
                    if !available_mana.can_pay(&foretell_exile_cost) {
                        return Some(None);
                    }
                    let tapped = mana::auto_tap_lands(
                        game,
                        self.pool_mut(player),
                        player,
                        &foretell_exile_cost,
                        Some(card_id),
                    );
                    self.emit_tap_for_mana_triggers(player, &tapped);
                    self.pool_mut(player).try_pay(&foretell_exile_cost);
                    game.card_mut(card_id).set_face_down(true);
                    self.move_card_with_runtime(game, card_id, ZoneType::Exile, player, agents);
                    self.trigger_handler.run_trigger(
                        TriggerType::Foretell,
                        RunParams {
                            card: Some(card_id),
                            player: Some(player),
                            ..Default::default()
                        },
                        false,
                    );
                    crate::agent::notify_all_agents(
                        agents,
                        crate::agent::GameLogEvent::rule(format!("Foretold: {}", card_name))
                            .with_player(player)
                            .with_card(card_id),
                    );
                    return Some(Some((card_id, card_name)));
                }
                Some(None)
            }
            crate::agent::PlayCardMode::Alternative(
                crate::spellability::AlternativeCost::Suspend,
            ) => {
                if let Some((suspend_cost, counters)) = game.card(card_id).get_suspend_cost() {
                    if game.card(card_id).zone != ZoneType::Hand {
                        return Some(None);
                    }
                    let available_mana =
                        mana::calculate_available_mana(self.pool(player), game, player);
                    let suspend_mc = forge_foundation::ManaCost::parse(&suspend_cost);
                    if !available_mana.can_pay(&suspend_mc) {
                        return Some(None);
                    }
                    let tapped = mana::auto_tap_lands(
                        game,
                        self.pool_mut(player),
                        player,
                        &suspend_mc,
                        Some(card_id),
                    );
                    self.emit_tap_for_mana_triggers(player, &tapped);
                    self.pool_mut(player).try_pay(&suspend_mc);
                    self.move_card_with_runtime(game, card_id, ZoneType::Exile, player, agents);
                    game.card_mut(card_id)
                        .add_counter(&crate::card::CounterType::Time, counters);
                    crate::agent::notify_all_agents(
                        agents,
                        crate::agent::GameLogEvent::rule(format!(
                            "Suspended: {} with {} time counters",
                            card_name, counters
                        ))
                        .with_player(player)
                        .with_card(card_id),
                    );
                    return Some(Some((card_id, card_name)));
                }
                Some(None)
            }
            _ => None,
        }
    }

    fn cast_card_spell_ability(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        mut sa: SpellAbility,
        static_alternative_cost_prepared: bool,
    ) -> Option<(CardId, String)> {
        let card_id = sa.source?;
        let card = game.card(card_id);
        let card_name = card.card_name.clone();
        let original_zone = card.zone;
        let original_owner = card.owner;

        // Cast spell — tap lands for mana, put on stack, resolve
        let is_creature = game.card(card_id).is_creature();
        let is_permanent = game.card(card_id).is_permanent();
        // ── Alternative cost mode selection (from action-space choice) ──────────
        let is_foretell = sa.alt_cost == Some(crate::spellability::AlternativeCost::Foretell);
        let is_flashback = sa.alt_cost == Some(crate::spellability::AlternativeCost::Flashback);
        let is_spectacle = sa.alt_cost == Some(crate::spellability::AlternativeCost::Spectacle);
        let is_evoke = sa.alt_cost == Some(crate::spellability::AlternativeCost::Evoke);
        let is_escape = sa.alt_cost == Some(crate::spellability::AlternativeCost::Escape);
        let is_overload = sa.alt_cost == Some(crate::spellability::AlternativeCost::Overload);
        let is_dash = sa.alt_cost == Some(crate::spellability::AlternativeCost::Dash);
        let is_blitz = sa.alt_cost == Some(crate::spellability::AlternativeCost::Blitz);
        let is_emerge = sa.alt_cost == Some(crate::spellability::AlternativeCost::Emerge);
        let is_plot_cast = sa.alt_cost == Some(crate::spellability::AlternativeCost::Plot);
        let is_bestow = sa.alt_cost == Some(crate::spellability::AlternativeCost::Bestow);
        let is_warp = sa.alt_cost == Some(crate::spellability::AlternativeCost::Warp);
        let is_morph_facedown = sa.alt_cost.map(|alt| alt.is_morph()).unwrap_or(false);
        let is_static_alternative = static_alternative_cost_prepared;

        // Select the card's spell ability line (SP$ ...) for cast-time logic.
        // Mirrors Java where casting operates on a concrete SpellAbility, not
        // arbitrary non-activated lines like `S:Mode$ OptionalCost`.
        let abilities_for_spell = game.card(card_id).abilities.clone();
        let spell_ability_text = abilities_for_spell
            .iter()
            .find(|a| Params::from_raw(a).has(keys::SP))
            .cloned()
            .unwrap_or_default();

        let static_alt_cost = if is_static_alternative {
            sa.pay_costs.clone()
        } else {
            None
        };

        // Mirror Java cast-time Charm gating (CharmEffect.makeChoices):
        // if not enough legal modes exist, casting fails before any payment.
        if spell_ability_text.contains("SP$ Charm")
            && !crate::ability::effects::charm_effect::can_make_choices_precast(
                game,
                player,
                card_id,
                &spell_ability_text,
            )
        {
            return None;
        }

        // Parse flashback total cost once (can include non-mana parts like Sac<...>).
        let flashback_total_cost = if is_flashback {
            // Safe: is_flashback is only true if get_flashback_cost() returned Some
            let fb_cost_str = game.card(card_id).get_flashback_cost().unwrap_or_default();
            Some(parse_cost(&fb_cost_str))
        } else {
            None
        };

        let flashback_mana_cost = flashback_total_cost.as_ref().map(|fb_cost| {
            fb_cost
                .parts
                .iter()
                .filter_map(|part| {
                    if let CostPart::Mana { cost: mc, .. } = part {
                        Some(mc.clone())
                    } else {
                        None
                    }
                })
                .fold(forge_foundation::ManaCost::zero(), |acc, mc| acc.add(&mc))
        });

        // Determine the mana cost to use
        // Note: All unwrap_or_default() below are safe because each is_* flag
        // is only true if the corresponding get_*_cost() returned Some earlier.
        let mana_cost = if is_foretell {
            let foretell_cost_str = game.card(card_id).get_foretell_cost().unwrap_or_default();
            game.card_mut(card_id).set_face_down(false); // reveal it
            forge_foundation::ManaCost::parse(&foretell_cost_str)
        } else if is_flashback {
            flashback_mana_cost.unwrap_or_else(forge_foundation::ManaCost::zero)
        } else if is_spectacle {
            let spec_cost_str = game.card(card_id).get_spectacle_cost().unwrap_or_default();
            forge_foundation::ManaCost::parse(&spec_cost_str)
        } else if is_evoke {
            // Select the specific Evoke cost chosen at action-space enumeration
            // (intrinsic vs granted by Ashling-style AddKeyword static). Without
            // this, cast_spell would always pay the first payable cost.
            let evoke_costs = game.card(card_id).get_all_evoke_costs();
            let evoke_cost_str = evoke_costs
                .get(sa.alt_cost_index as usize)
                .cloned()
                .or_else(|| evoke_costs.first().cloned())
                .unwrap_or_default();
            // Evoke cost may include non-mana parts (e.g. Fury:
            // ExileFromHand<1/Card.Red+Other/red card>). Parse as a full
            // Cost and extract only the mana portion for mana payment;
            // the non-mana parts are paid below via pay_additional_costs.
            Self::mana_from_cost(&crate::cost::parse_cost(&evoke_cost_str))
        } else if is_escape {
            let (escape_mana_str, _) = game
                .card(card_id)
                .get_escape_cost()
                .unwrap_or(("0".to_string(), 0));
            forge_foundation::ManaCost::parse(&escape_mana_str)
        } else if is_overload {
            let overload_cost_str = game.card(card_id).get_overload_cost().unwrap_or_default();
            forge_foundation::ManaCost::parse(&overload_cost_str)
        } else if is_dash {
            let dash_cost_str = game.card(card_id).get_dash_cost().unwrap_or_default();
            forge_foundation::ManaCost::parse(&dash_cost_str)
        } else if is_blitz {
            let blitz_cost_str = game.card(card_id).get_blitz_cost().unwrap_or_default();
            forge_foundation::ManaCost::parse(&blitz_cost_str)
        } else if is_emerge {
            let emerge_cost_str = game.card(card_id).get_emerge_cost().unwrap_or_default();
            forge_foundation::ManaCost::parse(&emerge_cost_str)
        } else if is_plot_cast {
            // Plot: cast from exile for free (already paid plot cost).
            forge_foundation::ManaCost::generic(0)
        } else if is_static_alternative {
            Self::mana_from_cost(static_alt_cost.as_ref()?)
        } else if is_bestow {
            let bestow_cost_str = game.card(card_id).get_bestow_cost().unwrap_or_default();
            forge_foundation::ManaCost::parse(&bestow_cost_str)
        } else if is_warp {
            let warp_cost_str = game.card(card_id).get_warp_cost().unwrap_or_default();
            forge_foundation::ManaCost::parse(&warp_cost_str)
        } else if is_morph_facedown {
            forge_foundation::ManaCost::generic(crate::spellability::MORPH_GENERIC_COST)
        } else {
            game.card(card_id).mana_cost.clone()
        };

        // NOTE: Static mana-cost reduction/increase is now handled centrally
        // by cost_adjustment::adjust() called later in this function.
        // Do not apply compute_cost_adjustment here.
        // We still need raise_cost for its non-mana cost parts (e.g. Waterbend).
        let cast_zone = game.card(card_id).zone;
        let raise_cost = crate::cost::cost_adjustment::compute_raise_cost_parts(
            game,
            game.card(card_id),
            player,
            cast_zone,
        );

        // ── Additional cost checks (Kicker, Buyback, Multikicker, Replicate) ──
        // Check Kicker: offer to pay additional kicker cost
        let kicked = if let Some(kicker_cost_str) = game.card(card_id).get_kicker_cost() {
            let kicker_mc = forge_foundation::ManaCost::parse(&kicker_cost_str);
            let combined = mana_cost.add(&kicker_mc);
            let available_mana = mana::calculate_available_mana(self.pool(player), game, player);
            if available_mana.can_pay(&combined) {
                let name = game.card(card_id).card_name.clone();
                agents[player.index()].snapshot_state(game, &self.mana_pools);
                agents[player.index()].choose_kicker(player, &kicker_cost_str, Some(&name))
            } else {
                false
            }
        } else {
            false
        };

        // Combine kicker cost with base cost if kicked
        let mana_cost = if kicked {
            // Safe: kicked is only true if get_kicker_cost() returned Some
            let kicker_cost_str = game.card(card_id).get_kicker_cost().unwrap_or_default();
            let kicker_mc = forge_foundation::ManaCost::parse(&kicker_cost_str);
            mana_cost.add(&kicker_mc)
        } else {
            mana_cost
        };

        // Check Buyback: offer to pay additional buyback cost
        let buyback_paid = if let Some(buyback_cost_str) = game.card(card_id).get_buyback_cost() {
            let buyback_mc = forge_foundation::ManaCost::parse(&buyback_cost_str);
            let combined = mana_cost.add(&buyback_mc);
            let available_mana = mana::calculate_available_mana(self.pool(player), game, player);
            if available_mana.can_pay(&combined) {
                let name = game.card(card_id).card_name.clone();
                agents[player.index()].snapshot_state(game, &self.mana_pools);
                agents[player.index()].choose_buyback(player, &buyback_cost_str, Some(&name))
            } else {
                false
            }
        } else {
            false
        };

        // Combine buyback cost
        let mana_cost = if buyback_paid {
            // Safe: buyback_paid is only true if get_buyback_cost() returned Some
            let buyback_cost_str = game.card(card_id).get_buyback_cost().unwrap_or_default();
            let buyback_mc = forge_foundation::ManaCost::parse(&buyback_cost_str);
            mana_cost.add(&buyback_mc)
        } else {
            mana_cost
        };

        // Check Multikicker: pay kicker cost any number of times
        let kick_count = if let Some(mk_cost_str) = game.card(card_id).get_multikicker_cost() {
            let mk_mc = forge_foundation::ManaCost::parse(&mk_cost_str);
            let available_mana = mana::calculate_available_mana(self.pool(player), game, player);
            // Calculate max kicks: how many times we can add mk_mc to mana_cost
            let mut max_kicks = 0u32;
            let mut test_cost = mana_cost.clone();
            loop {
                test_cost = test_cost.add(&mk_mc);
                if available_mana.can_pay(&test_cost) {
                    max_kicks += 1;
                } else {
                    break;
                }
                if max_kicks >= 20 {
                    break; // Safety cap
                }
            }
            if max_kicks > 0 {
                let name = game.card(card_id).card_name.clone();
                agents[player.index()].snapshot_state(game, &self.mana_pools);
                agents[player.index()].choose_multikicker(
                    player,
                    &mk_cost_str,
                    max_kicks,
                    Some(&name),
                )
            } else {
                0
            }
        } else {
            0
        };

        // Combine multikicker cost
        let mana_cost = if kick_count > 0 {
            // Safe: kick_count is only >0 if get_multikicker_cost() returned Some
            let mk_cost_str = game
                .card(card_id)
                .get_multikicker_cost()
                .unwrap_or_default();
            let mk_mc = forge_foundation::ManaCost::parse(&mk_cost_str);
            let mut total = mana_cost;
            for _ in 0..kick_count {
                total = total.add(&mk_mc);
            }
            total
        } else {
            mana_cost
        };

        // Check Replicate: copy spell for each time replicate cost is paid
        let replicate_count = if let Some(rep_cost_str) = game.card(card_id).get_replicate_cost() {
            let rep_mc = forge_foundation::ManaCost::parse(&rep_cost_str);
            let available_mana = mana::calculate_available_mana(self.pool(player), game, player);
            let mut max_reps = 0u32;
            let mut test_cost = mana_cost.clone();
            loop {
                test_cost = test_cost.add(&rep_mc);
                if available_mana.can_pay(&test_cost) {
                    max_reps += 1;
                } else {
                    break;
                }
                if max_reps >= 20 {
                    break;
                }
            }
            if max_reps > 0 {
                let name = game.card(card_id).card_name.clone();
                agents[player.index()].snapshot_state(game, &self.mana_pools);
                agents[player.index()].choose_replicate(
                    player,
                    &rep_cost_str,
                    max_reps,
                    Some(&name),
                )
            } else {
                0
            }
        } else {
            0
        };

        // Combine replicate cost
        let mana_cost = if replicate_count > 0 {
            // Safe: replicate_count is only >0 if get_replicate_cost() returned Some
            let rep_cost_str = game.card(card_id).get_replicate_cost().unwrap_or_default();
            let rep_mc = forge_foundation::ManaCost::parse(&rep_cost_str);
            let mut total = mana_cost;
            for _ in 0..replicate_count {
                total = total.add(&rep_mc);
            }
            total
        } else {
            mana_cost
        };

        // Check Escalate: additional cost per mode beyond the first.
        let mana_cost = if let Some(escalate_cost_str) = game.card(card_id).get_escalate_cost() {
            let abilities = game.card(card_id).abilities.clone();
            let ability_text = abilities.first().cloned().unwrap_or_default();
            let ability_params = Params::from_raw(&ability_text);
            let num_modes = ability_params
                .get(keys::CHOICES)
                .map(|c| c.split(',').count())
                .unwrap_or(1);
            if num_modes > 1 {
                let esc_mc = forge_foundation::ManaCost::parse(&escalate_cost_str);
                let available_mana =
                    mana::calculate_available_mana(self.pool(player), game, player);
                let mut extra_modes = 0u32;
                let mut test_cost = mana_cost.clone();
                for _ in 1..num_modes {
                    test_cost = test_cost.add(&esc_mc);
                    if available_mana.can_pay(&test_cost) {
                        extra_modes += 1;
                    } else {
                        break;
                    }
                }
                if extra_modes > 0 {
                    let mut total = mana_cost.clone();
                    for _ in 0..extra_modes {
                        total = total.add(&esc_mc);
                    }
                    total
                } else {
                    mana_cost
                }
            } else {
                mana_cost
            }
        } else {
            mana_cost
        };

        // Check Spree: each chosen mode has its own ModeCost$
        let mana_cost = if game.card(card_id).has_keyword("Spree") {
            let abilities = game.card(card_id).abilities.clone();
            let ability_text = abilities.first().cloned().unwrap_or_default();
            let ability_params = Params::from_raw(&ability_text);
            if let Some(choices_str) = ability_params.get(keys::CHOICES) {
                let choice_names: Vec<&str> = choices_str.split(',').collect();
                let svars = game.card(card_id).svars.clone();
                // Extract ModeCost and description for each mode
                let mut mode_costs: Vec<forge_foundation::ManaCost> = Vec::new();
                let mut mode_descriptions: Vec<String> = Vec::new();
                for name in &choice_names {
                    if let Some(svar_val) = svars.get(*name) {
                        let params = Params::from_raw(svar_val);
                        let cost = params
                            .get(keys::MODE_COST)
                            .map(|c| forge_foundation::ManaCost::parse(c))
                            .unwrap_or_else(|| forge_foundation::ManaCost::generic(0));
                        let desc = params
                            .get_cloned(keys::SPELL_DESCRIPTION)
                            .unwrap_or_else(|| name.to_string());
                        mode_descriptions.push(format!("+ {} — {}", cost, desc));
                        mode_costs.push(cost);
                    }
                }
                // Ask player to choose modes
                let card_name = game.card(card_id).card_name.clone();
                let min_modes = ability_params
                    .get("MinCharmNum")
                    .and_then(|s| s.parse::<usize>().ok())
                    .unwrap_or(1);
                let max_modes = mode_descriptions.len();
                let chosen = agents[player.index()].choose_mode(
                    player,
                    &mode_descriptions,
                    min_modes,
                    max_modes,
                    Some(&card_name),
                );
                // Add selected ModeCosts to base cost
                let mut total = mana_cost.clone();
                for &idx in &chosen {
                    if idx < mode_costs.len() {
                        total = total.add(&mode_costs[idx]);
                    }
                }
                // Store chosen modes on card for charm_effect to reuse
                game.card_mut(card_id).set_chosen_modes(chosen);
                total
            } else {
                mana_cost
            }
        } else {
            mana_cost
        };

        // Check Strive: additional cost per target beyond the first.
        let mana_cost = if let Some(strive_cost_str) = game.card(card_id).get_strive_cost() {
            let strive_mc = forge_foundation::ManaCost::parse(&strive_cost_str);
            let available_mana = mana::calculate_available_mana(self.pool(player), game, player);
            // Calculate max affordable extra targets
            let mut extra_targets = 0u32;
            let mut test_cost = mana_cost.clone();
            for _ in 0..20 {
                // cap at 20 to avoid infinite loop
                test_cost = test_cost.add(&strive_mc);
                if available_mana.can_pay(&test_cost) {
                    extra_targets += 1;
                } else {
                    break;
                }
            }
            if extra_targets > 0 {
                let mut total = mana_cost.clone();
                for _ in 0..extra_targets {
                    total = total.add(&strive_mc);
                }
                // Store max targets (1 base + extras) for resolution targeting
                game.card_mut(card_id)
                    .set_strive_extra_targets(extra_targets);
                total
            } else {
                mana_cost
            }
        } else {
            mana_cost
        };

        // Check Entwine: pay extra to choose all modes of a modal spell
        let entwine_paid = if let Some(entwine_cost_str) = game.card(card_id).get_entwine_cost() {
            let entwine_mc = forge_foundation::ManaCost::parse(&entwine_cost_str);
            let combined = mana_cost.add(&entwine_mc);
            let available_mana = mana::calculate_available_mana(self.pool(player), game, player);
            if available_mana.can_pay(&combined) {
                let name = game.card(card_id).card_name.clone();
                agents[player.index()].snapshot_state(game, &self.mana_pools);
                agents[player.index()].choose_kicker(player, &entwine_cost_str, Some(&name))
            } else {
                false
            }
        } else {
            false
        };

        // Combine entwine cost
        let mana_cost = if entwine_paid {
            // Safe: entwine_paid is only true if get_entwine_cost() returned Some
            let entwine_cost_str = game.card(card_id).get_entwine_cost().unwrap_or_default();
            let entwine_mc = forge_foundation::ManaCost::parse(&entwine_cost_str);
            mana_cost.add(&entwine_mc)
        } else {
            mana_cost
        };

        // Assist: another player can pay generic mana portion (multiplayer mechanic).
        // In 1v1, the opponent is asked but AI will decline. Mirrors Java CostAdjustment.adjustCostByAssist().
        let mana_cost = if game.card(card_id).has_keyword("Assist") {
            let generic = mana_cost.generic_cost();
            if generic > 0 && game.players.len() > 1 {
                // Find the opponent
                let opponent = game
                    .players
                    .iter()
                    .enumerate()
                    .find(|(i, _)| PlayerId(*i as u32) != player)
                    .map(|(i, _)| PlayerId(i as u32));
                if let Some(opp) = opponent {
                    agents[opp.index()].snapshot_state(game, &self.mana_pools);
                    let assisted = agents[opp.index()].help_pay_assist(
                        opp,
                        &game.card(card_id).card_name,
                        generic as u32,
                    );
                    if assisted > 0 {
                        // Reduce generic cost by assisted amount
                        mana_cost.reduce_generic(assisted as i32)
                    } else {
                        mana_cost
                    }
                } else {
                    mana_cost
                }
            } else {
                mana_cost
            }
        } else {
            mana_cost
        };

        // Detect commander cast from Command zone (for commander tax)
        let is_commander_cast = game.player_is_commander(player, card_id)
            && game.card(card_id).zone == ZoneType::Command;
        let mut commander_tax = if is_commander_cast {
            game.player_commander_tax(player, card_id)
        } else {
            0
        };

        // ── X mana cost handling ──────────────────────────────────
        let original_mana_cost = mana_cost.clone();
        let spell_cost = Self::parse_spell_cost(&abilities_for_spell);
        let x_count = mana_cost.count_x();
        let mut x_value = 0u32;
        let mana_cost = if x_count > 0 {
            // Compute max X iteratively, mirroring Java's
            // ComputerUtilMana.determineLeftoverMana(): try X=1,2,...
            // until canPayManaCost fails, then return the last payable X.
            // This correctly handles multi-color sources that inflate
            // pool.total_mana() but can only produce one mana per activation.
            let available_mana = mana::calculate_available_mana(self.pool(player), game, player);
            let non_x_cost = mana_cost.without_x();
            let max_x = {
                let mut x: u32 = 0;
                loop {
                    let extra_generic = ((x + 1) * x_count as u32) as i32 + commander_tax;
                    let full_cost =
                        non_x_cost.add(&forge_foundation::ManaCost::generic(extra_generic));
                    if !available_mana.can_pay(&full_cost) {
                        break;
                    }
                    x += 1;
                    if x >= 99 {
                        break;
                    }
                }
                x
            };
            let name = game.card(card_id).card_name.clone();
            agents[player.index()].snapshot_state(game, &self.mana_pools);
            let chosen_x = agents[player.index()].choose_x_value(player, max_x, Some(&name));
            x_value = chosen_x.min(max_x);
            non_x_cost.add(&forge_foundation::ManaCost::generic(
                (x_value * x_count as u32) as i32,
            ))
        } else {
            mana_cost
        };

        let mana_cost = mana_cost;

        // Build SpellAbility chain and choose modes/targets from the pre-payment
        // game state. This matches Java/MTG casting order: announce modes and
        // targets before paying costs, so mana payments can invalidate a chosen
        // target later (for example, sacrificing a Food token used as a target).
        if is_flashback {
            game.card_mut(card_id).cast_with_flashback = true;
        } else if is_overload {
            sa.overloaded = true;
        } else if is_bestow && sa.target_restrictions.is_none() {
            sa.target_restrictions =
                crate::spellability::target_restrictions::TargetRestrictions::new(
                    &Params::from_raw("ValidTgts$ Creature"),
                );
        }

        if kicked || kick_count > 0 || entwine_paid {
            sa.kicked = true;
        }

        sa.buyback_paid = buyback_paid;
        sa.kick_count = kick_count;
        sa.replicate_count = replicate_count;
        sa.x_mana_cost_paid = x_value;
        game.card_mut(card_id)
            .svars
            .insert("XPaid".to_string(), x_value.to_string());

        if !sa.overloaded {
            let mut targeting_game = game.clone();
            if sa.is_spell && targeting_game.card(card_id).zone != ZoneType::Stack {
                // Java deterministic spell casting moves the spell to stack
                // before setupTargets(), so counterspells can see themselves
                // as legal stack targets during target selection.
                targeting_game.move_card(card_id, ZoneType::Stack, player);
            }
            if sa.api == Some(crate::ability::api_type::ApiType::Charm)
                && !crate::ability::effects::charm_effect::make_choices_precast(
                    &mut targeting_game,
                    agents,
                    &mut sa,
                )
            {
                return None;
            }
            if !sa.setup_targets(&targeting_game, agents, &self.mana_pools) {
                return None;
            }
            // Post-targeting validation: reject cast if MustTarget (Flagbearer)
            // restriction is not satisfied. Mirrors Java's isLegalAfterTargeting()
            // → meetsMustTargetRestriction() which prevents casting if the chosen
            // target doesn't include the required Flagbearer.
            let meets =
                crate::staticability::static_ability_must_target::meets_must_target_restriction(
                    &targeting_game,
                    &sa,
                );
            if !meets {
                eprintln!(
                    "[RUST-MUST-TARGET] Cast rejected for {} — MustTarget restriction not met",
                    card_name
                );
                if self.java_parity_failed_spell_setup_to_stack
                    && game.card(card_id).zone != ZoneType::Stack
                {
                    game.move_card(card_id, ZoneType::Stack, player);
                }
                return None;
            }
        }

        let display_total_cost = if commander_tax > 0 {
            original_mana_cost.add(&forge_foundation::ManaCost::generic(commander_tax))
        } else {
            original_mana_cost.clone()
        };
        let payable_base_cost = if commander_tax > 0 {
            mana_cost.add(&forge_foundation::ManaCost::generic(commander_tax))
        } else {
            mana_cost.clone()
        };
        let mut total_unpaid = ManaCostBeingPaid::from_mana_cost(&payable_base_cost);
        if !crate::cost::cost_adjustment::adjust(
            game,
            agents,
            &mut self.trigger_handler,
            &self.mana_pools,
            &mut total_unpaid,
            &mut sa,
            player,
            None,
            false,
            false,
        ) {
            return None;
        }
        let total_cost = crate::mana::apply_player_life_payment_keywords(
            game,
            player,
            &total_unpaid.to_mana_cost(),
        );

        // Java CostPayment decides sacrifice/discard targets in the accept
        // phase after cost adjustment but before payment.
        let prechosen_spell_sacrifices = if let Some(ref sc) = spell_cost {
            match self.prechoose_additional_cost_sacrifices(game, agents, player, sc, Some(&sa)) {
                Some(picks) => Some(picks),
                None => return None,
            }
        } else {
            None
        };
        let prechosen_spell_discards = if let Some(ref sc) = spell_cost {
            match self.prechoose_additional_cost_discards(game, agents, player, card_id, sc) {
                Some(picks) => Some(picks),
                None => return None,
            }
        } else {
            None
        };
        let prechosen_static_alt_sacrifices = if let Some(ref cost) = static_alt_cost {
            match self.prechoose_additional_cost_sacrifices(game, agents, player, cost, Some(&sa)) {
                Some(picks) => Some(picks),
                None => return None,
            }
        } else {
            None
        };
        let prechosen_static_alt_discards = if let Some(ref cost) = static_alt_cost {
            match self.prechoose_additional_cost_discards(game, agents, player, card_id, cost) {
                Some(picks) => Some(picks),
                None => return None,
            }
        } else {
            None
        };

        // Build mana payment context for restriction checking
        let payment_ctx = {
            let card = game.card(card_id);
            let chosen_types_by_source = game
                .cards
                .iter()
                .filter_map(|c| c.chosen_type.clone().map(|chosen| (c.id, chosen)))
                .collect();
            mana::ManaPaymentContext {
                is_spell: true,
                type_line: Some(card.type_line.clone()),
                card_name: Some(card.card_name.clone()),
                chosen_types_by_source,
            }
        };

        // Check if mana conversion allows spending mana as any color
        let any_color_conversion = {
            let card = game.card(card_id);
            crate::staticability::static_ability_mana_convert::can_spend_mana_as_any_color(
                &game.cards,
                player,
                card,
            )
        };

        // Track mana metadata before payment for post-payment effects
        let uncounterable_before = self.pool(player).count_uncounterable();
        let keywords_before = self.pool(player).collect_keyword_mana();
        let counters_before = self.pool(player).collect_counter_mana();
        let triggers_before = self.pool(player).collect_trigger_mana();
        // Track pool size before payment for ManaExpend
        let pool_size_before = self.pool(player).total_mana();
        let colors_spent_to_cast = std::cell::Cell::new(0u16);
        let paying_mana_to_cast = std::cell::RefCell::new(Vec::new());

        // Unified mana payment loop. Agents decide whether to pay manually or
        // auto-pay through their `pay_mana_cost()` implementation; the engine
        // should not branch on `is_human()` here.
        let card_name = game.card(card_id).card_name.clone();
        if total_cost.is_zero() {
            // Zero-cost spell payments have no mana-payment interaction.
            // Skip the callback loop entirely so parity/UI only see real payments.
        } else {
            let cost_str = total_cost.to_string();
            let cost_display_str = display_total_cost.to_string();
            let cost_checkpoint_str = if x_count > 0 && x_value == 0 {
                cost_display_str.clone()
            } else {
                cost_str.clone()
            };
            let session = ManaPaymentSession {
                player,
                card_id,
                card_name: &card_name,
                mana_cost: &total_cost,
                cost_str: &cost_str,
                cost_display_str: &cost_display_str,
                cost_checkpoint_str: &cost_checkpoint_str,
                is_activated_ability: false,
                reserved_sacrifices: &[],
            };
            let mana_paid = self.pay_mana_cost_session(
                game,
                agents,
                session,
                |game, player, land_id, ab, _reserved_sacrifices| {
                    crate::cost::can_pay_ignoring_mana(&ab.cost, game, land_id, player)
                },
                |slf, game, agents, session| {
                    let auto_result = {
                        let game_ptr: *mut GameState = game;
                        let trigger_handler_ptr = std::ptr::from_mut(&mut slf.trigger_handler);
                        let mut callback = Self::make_mana_payment_callback(
                            trigger_handler_ptr,
                            game_ptr,
                            agents,
                            session.player,
                        );
                        mana::pay_mana_cost_auto_with_callback(
                            game,
                            slf.pool_mut(session.player),
                            session.player,
                            session.mana_cost,
                            Some(session.card_id),
                            0,
                            &payment_ctx,
                            any_color_conversion,
                            &mut callback,
                        )
                    };
                    if let Some(result) = auto_result {
                        colors_spent_to_cast.set(colors_spent_to_cast.get() | result.colors_spent);
                        paying_mana_to_cast
                            .borrow_mut()
                            .extend(result.paying_mana.iter().copied());
                        let trace: Vec<ManaCostAction> = result
                            .choices
                            .iter()
                            .map(|choice| ManaCostAction::TapLand {
                                card_id: choice.card_id,
                                mana_ability_index: Some(choice.mana_ability_index.unwrap_or(0)),
                                express_choice: if choice.needs_express_choice {
                                    Some(choice.chosen_atom)
                                } else {
                                    None
                                },
                            })
                            .collect();
                        for &tapped_id in &result.tapped {
                            slf.trigger_handler.run_trigger(
                                TriggerType::Taps,
                                RunParams {
                                    card: Some(tapped_id),
                                    player: Some(session.player),
                                    ..Default::default()
                                },
                                false,
                            );
                            slf.trigger_handler.run_trigger(
                                TriggerType::TapsForMana,
                                RunParams {
                                    card: Some(tapped_id),
                                    player: Some(session.player),
                                    activator: Some(session.player),
                                    ..Default::default()
                                },
                                false,
                            );
                        }
                        if result.life_paid > 0 {
                            slf.pay_life_cost(
                                game,
                                session.player,
                                session.card_id,
                                result.life_paid,
                            );
                        }
                        Some(trace)
                    } else {
                        None
                    }
                },
                |slf, game, player| {
                    let mut test_pool = slf.pool(player).clone();
                    if let Some(test_payment) = test_pool
                        .try_pay_for_spell_converted_with_phyrexian_life_result(
                            &total_cost,
                            &payment_ctx,
                            any_color_conversion,
                            game.player(player).life,
                        )
                    {
                        let payment = slf
                            .pool_mut(player)
                            .try_pay_for_spell_converted_with_phyrexian_life_result(
                                &total_cost,
                                &payment_ctx,
                                any_color_conversion,
                                game.player(player).life,
                            )
                            .expect("tested phyrexian payment should still be legal");
                        if payment.life_paid != test_payment.life_paid {
                            return false;
                        }
                        colors_spent_to_cast.set(colors_spent_to_cast.get() | payment.colors_spent);
                        paying_mana_to_cast
                            .borrow_mut()
                            .extend(payment.paying_mana.iter().copied());
                        if payment.life_paid > 0 {
                            slf.pay_life_cost(game, player, card_id, payment.life_paid);
                        }
                        true
                    } else {
                        false
                    }
                },
            );
            if !mana_paid {
                if game.card(card_id).zone != original_zone {
                    self.move_card_with_runtime(
                        game,
                        card_id,
                        original_zone,
                        original_owner,
                        agents,
                    );
                }
                return None;
            }
        }

        // If uncounterable mana was consumed during payment (Cavern of Souls),
        // add a "can't be countered" replacement effect to the spell's card.
        let uncounterable_after = self.pool(player).count_uncounterable();
        if uncounterable_after < uncounterable_before {
            use crate::replacement::replacement_effect::{
                ReplacementEffect, ReplacementLayer, ReplacementType,
            };
            let params = crate::parsing::Params::from_raw("ValidCard$ Card.Self");
            game.card_mut(card_id)
                .replacement_effects
                .push(ReplacementEffect::new(
                    ReplacementType::Counter,
                    ReplacementLayer::CantHappen,
                    params,
                    vec![], // active everywhere (including stack)
                ));
        }

        game.card_mut(card_id)
            .set_colors_spent_to_cast(colors_spent_to_cast.get());
        game.card_mut(card_id)
            .set_paying_mana_to_cast(paying_mana_to_cast.into_inner());

        // Fire ManaExpend triggers (Expend mechanic — cumulative per-turn tracking)
        {
            let pool_size_after = self.pool(player).total_mana();
            let mana_spent = (pool_size_before - pool_size_after) as i32;
            if mana_spent > 0 {
                let starting = game.player(player).mana_expended_this_turn;
                let total = starting + mana_spent;
                game.player_set_mana_expended(player, total);
                // Fire trigger for each cumulative amount from starting+1 to total
                for i in (starting + 1)..=total {
                    self.trigger_handler.run_trigger(
                        TriggerType::ManaExpend,
                        RunParams {
                            player: Some(player),
                            mana_expend_amount: Some(i),
                            ..Default::default()
                        },
                        true,
                    );
                }
            }
        }

        // If keyword mana was consumed (Generator Servant, Hall of the Bandit Lord),
        // add those keywords to the spell's card.
        let keywords_after = self.pool(player).collect_keyword_mana();
        {
            let mut applied = std::collections::HashSet::new();
            for (kw, valid) in &keywords_before {
                if applied.contains(kw) {
                    continue;
                }
                let before_count = keywords_before.iter().filter(|(k, _)| k == kw).count();
                let after_count = keywords_after.iter().filter(|(k, _)| k == kw).count();
                if after_count < before_count {
                    let valid_ok = match valid {
                        None => true,
                        Some(v) => crate::mana::mana_meets_restriction(v, &payment_ctx),
                    };
                    if valid_ok {
                        for keyword in kw.split('&').map(str::trim) {
                            if !keyword.is_empty() {
                                game.card_mut(card_id).add_intrinsic_keyword(keyword);
                            }
                        }
                    }
                    applied.insert(kw.clone());
                }
            }
        }

        // If counter mana was consumed (Guildmages' Forum, Opal Palace),
        // mark the card to receive counters on ETB.
        let counters_after = self.pool(player).collect_counter_mana();
        {
            let mut applied = std::collections::HashSet::new();
            for (counter_spec, valid) in &counters_before {
                if applied.contains(counter_spec) {
                    continue;
                }
                let before_count = counters_before
                    .iter()
                    .filter(|(c, _)| c == counter_spec)
                    .count();
                let after_count = counters_after
                    .iter()
                    .filter(|(c, _)| c == counter_spec)
                    .count();
                if after_count < before_count {
                    let valid_ok = match valid {
                        None => true,
                        Some(v) => crate::mana::mana_meets_restriction(v, &payment_ctx),
                    };
                    if valid_ok && counter_spec.contains("P1P1") {
                        let count = counter_spec
                            .rsplit('_')
                            .next()
                            .and_then(|s| s.parse::<i32>().ok())
                            .unwrap_or(1);
                        game.card_mut(card_id).add_etb_counters_p1p1(count);
                    }
                    applied.insert(counter_spec.clone());
                }
            }
        }

        // ── TriggersWhenSpent: fire triggers from consumed mana ──
        {
            let triggers_after = self.pool(player).collect_trigger_mana();
            let mut fired = std::collections::HashSet::new();
            for (svar_name, source_id) in &triggers_before {
                if fired.contains(&(svar_name.clone(), *source_id)) {
                    continue;
                }
                // Check if this trigger mana was consumed (present before but not after)
                let before_count = triggers_before
                    .iter()
                    .filter(|(s, src)| s == svar_name && src == source_id)
                    .count();
                let after_count = triggers_after
                    .iter()
                    .filter(|(s, src)| s == svar_name && src == source_id)
                    .count();
                if after_count < before_count {
                    // Look up the SVar on the source card and fire the trigger
                    if let Some(trigger_svar) = game.card(*source_id).svars.get(svar_name).cloned()
                    {
                        let params = Params::from_raw(&trigger_svar);
                        // Check ValidCard$ filter against the spell being cast
                        let valid = params.get(keys::VALID_CARD).unwrap_or("Card");
                        let card = game.card(card_id);
                        let valid_ok = valid == "Card"
                            || (valid.contains("Creature") && card.is_creature())
                            || (valid.contains("Dragon") && card.type_line.has_subtype("Dragon"))
                            || (valid.contains("cmcGE6") && card.mana_cost.cmc() >= 6)
                            || (valid.contains("cmcGE5") && card.mana_cost.cmc() >= 5)
                            || (valid.contains("IsCommander") && card.is_commander);
                        if valid_ok {
                            if let Some(execute) = params.get(keys::EXECUTE) {
                                if let Some(exec_svar) =
                                    game.card(*source_id).svars.get(execute).cloned()
                                {
                                    let exec_sa = crate::spellability::build_spell_ability(
                                        game, *source_id, &exec_svar, player,
                                    );
                                    crate::ability::effects::resolve_effect(
                                        &mut crate::ability::effects::EffectContext {
                                            game,
                                            combat: Some(&mut self.combat),
                                            agents,
                                            trigger_handler: &mut self.trigger_handler,
                                            token_templates: &self.token_templates,
                                            token_art_variants: &self.token_art_variants,
                                            token_fallback: &self.token_fallback,
                                            edition_dates: &self.edition_dates,
                                            mana_pools: &mut self.mana_pools,
                                            parent_target_card: None,
                                            rng: &mut *self.game_rng,
                                        },
                                        &exec_sa,
                                    );
                                }
                            }
                        }
                    }
                    fired.insert((svar_name.clone(), *source_id));
                }
            }
        }

        // Pay additional costs from SP$ line (e.g. sacrifice a creature).
        let mut waterbend_tapped: Vec<CardId> = Vec::new();
        if let Some(ref sc) = spell_cost {
            let has_waterbend = sc
                .parts
                .iter()
                .any(|p| matches!(p, crate::cost::CostPart::Waterbend { .. }));
            let untapped_before = if has_waterbend {
                game.cards_in_zone(ZoneType::Battlefield, player)
                    .iter()
                    .copied()
                    .filter(|&cid| {
                        let c = game.card(cid);
                        cid != card_id
                            && !c.tapped
                            && (c.is_creature() || c.type_line.is_artifact())
                    })
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            if !self.pay_additional_costs(
                game,
                agents,
                player,
                card_id,
                sc,
                None,
                sc.mandatory,
                Some(&sa),
                prechosen_spell_sacrifices.as_deref(),
                prechosen_spell_discards.as_deref(),
            ) {
                return None;
            }
            if has_waterbend {
                for cid in untapped_before
                    .into_iter()
                    .filter(|&cid| game.card(cid).tapped)
                {
                    if !waterbend_tapped.contains(&cid) {
                        waterbend_tapped.push(cid);
                    }
                }
            }
        }
        if let Some(ref cost) = static_alt_cost {
            let has_waterbend = cost
                .parts
                .iter()
                .any(|p| matches!(p, crate::cost::CostPart::Waterbend { .. }));
            let untapped_before = if has_waterbend {
                game.cards_in_zone(ZoneType::Battlefield, player)
                    .iter()
                    .copied()
                    .filter(|&cid| {
                        let c = game.card(cid);
                        cid != card_id
                            && !c.tapped
                            && (c.is_creature() || c.type_line.is_artifact())
                    })
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            if !self.pay_additional_costs(
                game,
                agents,
                player,
                card_id,
                cost,
                None,
                cost.mandatory,
                Some(&sa),
                prechosen_static_alt_sacrifices.as_deref(),
                prechosen_static_alt_discards.as_deref(),
            ) {
                return None;
            }
            if has_waterbend {
                for cid in untapped_before
                    .into_iter()
                    .filter(|&cid| game.card(cid).tapped)
                {
                    if !waterbend_tapped.contains(&cid) {
                        waterbend_tapped.push(cid);
                    }
                }
            }
        }
        if let Some(ref rc) = raise_cost {
            let has_waterbend = rc
                .parts
                .iter()
                .any(|p| matches!(p, crate::cost::CostPart::Waterbend { .. }));
            let untapped_before = if has_waterbend {
                game.cards_in_zone(ZoneType::Battlefield, player)
                    .iter()
                    .copied()
                    .filter(|&cid| {
                        let c = game.card(cid);
                        cid != card_id
                            && !c.tapped
                            && (c.is_creature() || c.type_line.is_artifact())
                    })
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            if !self.pay_additional_costs(
                game,
                agents,
                player,
                card_id,
                rc,
                None,
                rc.mandatory,
                Some(&sa),
                None,
                None,
            ) {
                return None;
            }
            if has_waterbend {
                for cid in untapped_before
                    .into_iter()
                    .filter(|&cid| game.card(cid).tapped)
                {
                    if !waterbend_tapped.contains(&cid) {
                        waterbend_tapped.push(cid);
                    }
                }
            }
        }

        // Pay additional non-mana costs from Flashback keyword cost
        // (e.g. Lava Dart: Flashback—Sacrifice a Mountain).
        if let Some(ref fb_cost) = flashback_total_cost {
            let has_waterbend = fb_cost
                .parts
                .iter()
                .any(|p| matches!(p, crate::cost::CostPart::Waterbend { .. }));
            let untapped_before = if has_waterbend {
                game.cards_in_zone(ZoneType::Battlefield, player)
                    .iter()
                    .copied()
                    .filter(|&cid| {
                        let c = game.card(cid);
                        cid != card_id
                            && !c.tapped
                            && (c.is_creature() || c.type_line.is_artifact())
                    })
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            if !self.pay_additional_costs(
                game,
                agents,
                player,
                card_id,
                fb_cost,
                None,
                fb_cost.mandatory,
                Some(&sa),
                None,
                None,
            ) {
                return None;
            }
            if has_waterbend {
                for cid in untapped_before
                    .into_iter()
                    .filter(|&cid| game.card(cid).tapped)
                {
                    if !waterbend_tapped.contains(&cid) {
                        waterbend_tapped.push(cid);
                    }
                }
            }
        }

        // Pay Evoke additional (non-mana) costs, e.g. Fury's
        // ExileFromHand<1/Card.Red+Other/red card>. Use the indexed cost so the
        // non-mana part matches the same Evoke chosen for mana payment.
        if is_evoke {
            let evoke_costs = game.card(card_id).get_all_evoke_costs();
            let evoke_cost_str = evoke_costs
                .get(sa.alt_cost_index as usize)
                .cloned()
                .or_else(|| evoke_costs.first().cloned())
                .unwrap_or_default();
            let evoke_cost = crate::cost::parse_cost(&evoke_cost_str);
            if !self.pay_additional_costs(
                game,
                agents,
                player,
                card_id,
                &evoke_cost,
                None,
                evoke_cost.mandatory,
                Some(&sa),
                None,
                None,
            ) {
                return None;
            }
        }

        // Pay Escape exile cost: exile N other graveyard cards
        if is_escape {
            if let Some((_, exile_count)) = game.card(card_id).get_escape_cost() {
                let gy_cards: Vec<CardId> = game
                    .cards_in_zone(ZoneType::Graveyard, player)
                    .iter()
                    .filter(|&&cid| cid != card_id)
                    .copied()
                    .take(exile_count as usize)
                    .collect();
                for cid in gy_cards {
                    self.move_card_with_runtime(game, cid, ZoneType::Exile, player, agents);
                }
            }
        }

        crate::cost::cost_adjustment::commit_offerings_and_emerge(
            game,
            agents,
            &mut self.trigger_handler,
            &mut sa,
        );

        // Increment commander cast count (before moving card to stack)
        if is_commander_cast {
            game.player_increment_commander_cast(player, card_id);
        }

        {
            game.player_record_spell_cast(player, card_id);
        }
        // Track spell cast on the stack (storm count, etc.)
        game.stack.record_spell_cast(card_id);

        let cast_zone = if is_foretell {
            Some(ZoneType::Exile)
        } else if is_flashback || is_escape {
            Some(ZoneType::Graveyard)
        } else if is_plot_cast {
            Some(ZoneType::Exile)
        } else if is_commander_cast {
            Some(ZoneType::Command)
        } else {
            // Use the card's actual zone (usually Hand, but could be
            // Exile for Warp-from-exile casts with Normal mode).
            Some(game.card(card_id).zone)
        };

        let entry = StackEntry {
            id: 0,
            spell_ability: sa,
            is_creature_spell: is_creature,
            is_permanent_spell: is_permanent,
            cast_from_zone: cast_zone,
            optional_trigger_decider: None,
            optional_trigger_description: None,
            optional_trigger_source_name: None,
        };
        let chosen_target = entry.spell_ability.target_chosen.target_card;
        let stack_message = if is_flashback {
            format!("Cast: {} [Flashback from Graveyard]", card_name)
        } else {
            format!("Cast: {}", card_name)
        };
        let sa_for_trigger = self.push_spell_ability_to_stack(
            game,
            agents,
            player,
            StackPushContext {
                source_card: card_id,
                entry: entry.clone(),
                stack_log_name: card_name.clone(),
                stack_message,
                target_card: chosen_target,
                event_kind: SpellAbilityLogEventKind::Stack,
                move_source_to_stack: true,
                register_source_trigger: true,
            },
        );
        self.emit_post_stack_spell_ability_triggers(
            game,
            player,
            &sa_for_trigger,
            PostStackTriggerContext {
                source_card: card_id,
                cast_trigger: TriggerType::SpellCast,
                emit_ability_activated: false,
                emit_waterbend: !waterbend_tapped.is_empty(),
                waterbend_cards: waterbend_tapped.clone(),
            },
        );

        // Storm: create N copies where N = spells_cast_this_turn - 1.
        if game.card(card_id).has_storm() {
            let storm_count = game.player_storm_count(player);
            if storm_count > 0 {
                crate::agent::notify_all_agents(
                    agents,
                    crate::agent::GameLogEvent::stack(format!(
                        "Storm count: {} copies",
                        storm_count
                    ))
                    .with_player(player)
                    .with_card(card_id),
                );
                for i in 0..storm_count {
                    if crate::card::card_factory::spell_ability_cant_be_copied(
                        &game.cards,
                        &entry.spell_ability,
                    ) {
                        continue;
                    }
                    let mut copy = entry.clone();
                    copy.spell_ability =
                        crate::card::card_factory::copy_spell_ability(&entry.spell_ability, player);
                    if copy.spell_ability.uses_targeting() {
                        agents[player.index()].snapshot_state(game, &self.mana_pools);
                        agents[player.index()].notify(
                            crate::agent::notification::GameNotification::Event(
                                crate::agent::GameLogEvent::stack(format!(
                                    "Choose target for Storm copy {}/{}",
                                    i + 1,
                                    storm_count
                                ))
                                .with_player(player)
                                .with_card(card_id),
                            ),
                        );
                        copy.spell_ability
                            .setup_targets(game, agents, &self.mana_pools);
                        crate::ability::effects::emit_targeting_triggers_for_sa(
                            &mut self.trigger_handler,
                            game,
                            card_id,
                            &copy.spell_ability,
                        );
                    }
                    game.stack.push(copy);
                    self.log_stack_push(
                        &format!("{} (Storm copy)", card_name),
                        &game.player(player).name,
                    );

                    // Emit SpellCopied trigger for Magecraft
                    self.trigger_handler.run_trigger(
                        TriggerType::SpellCopied,
                        RunParams {
                            spell_card: Some(card_id),
                            spell_controller: Some(player),
                            ..Default::default()
                        },
                        false,
                    );
                }
            }
        }

        // Replicate: create N copies where N = replicate_count
        if replicate_count > 0 {
            crate::agent::notify_all_agents(
                agents,
                crate::agent::GameLogEvent::stack(format!("Replicate: {} copies", replicate_count))
                    .with_player(player)
                    .with_card(card_id),
            );
            for i in 0..replicate_count {
                if crate::card::card_factory::spell_ability_cant_be_copied(
                    &game.cards,
                    &entry.spell_ability,
                ) {
                    continue;
                }
                let mut copy = entry.clone();
                copy.spell_ability =
                    crate::card::card_factory::copy_spell_ability(&entry.spell_ability, player);
                if copy.spell_ability.uses_targeting() {
                    agents[player.index()].snapshot_state(game, &self.mana_pools);
                    agents[player.index()].notify(
                        crate::agent::notification::GameNotification::Event(
                            crate::agent::GameLogEvent::stack(format!(
                                "Choose target for Replicate copy {}/{}",
                                i + 1,
                                replicate_count
                            ))
                            .with_player(player)
                            .with_card(card_id),
                        ),
                    );
                    copy.spell_ability
                        .setup_targets(game, agents, &self.mana_pools);
                    crate::ability::effects::emit_targeting_triggers_for_sa(
                        &mut self.trigger_handler,
                        game,
                        card_id,
                        &copy.spell_ability,
                    );
                }
                game.stack.push(copy);
                self.log_stack_push(
                    &format!("{} (Replicate copy)", card_name),
                    &game.player(player).name,
                );

                // Emit SpellCopied trigger for Magecraft
                self.trigger_handler.run_trigger(
                    TriggerType::SpellCopied,
                    RunParams {
                        spell_card: Some(card_id),
                        spell_controller: Some(player),
                        ..Default::default()
                    },
                    false,
                );
            }
        }

        // Cascade: exile from library until finding a cheaper nonland card
        let cascade_count = game
            .card(card_id)
            .keywords
            .iter_strings()
            .filter(|k| k.eq_ignore_ascii_case("Cascade"))
            .count();
        if cascade_count > 0 {
            // Run Cascade replacement effects before cascading.
            let skip_cascade = {
                use crate::replacement::replacement_handler::{
                    apply_replacements, ReplacementEvent,
                };
                use crate::replacement::ReplacementResult;
                let mut event = ReplacementEvent::Cascade { player };
                let result = apply_replacements(game, &mut event);
                result == ReplacementResult::Skipped || result == ReplacementResult::Replaced
            };
            if !skip_cascade {
                let caster_mv = game.card(card_id).mana_value();
                for _ in 0..cascade_count {
                    self.resolve_cascade(game, agents, player, caster_mv);
                }
            }
        }
        Some((card_id, card_name))
    }
    pub(crate) fn resolve_cascade(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        caster_mv: i32,
    ) {
        let mut exiled_ids: Vec<CardId> = Vec::new();
        let mut found_card: Option<CardId> = None;

        // Exile cards one at a time from the top of the library
        loop {
            let lib = game.cards_in_zone(ZoneType::Library, player);
            if lib.is_empty() {
                break;
            }
            // Safe: we just checked is_empty(), so last() will return Some
            let top_id = lib.last().copied().unwrap_or(CardId(0));
            if top_id == CardId(0) {
                break; // Safety: should never happen
            }
            self.move_card_with_runtime(game, top_id, ZoneType::Exile, player, agents);
            let card = game.card(top_id);
            let is_land = card.is_land();
            let mv = card.mana_value();

            if !is_land && mv < caster_mv {
                found_card = Some(top_id);
                crate::agent::notify_all_agents(
                    agents,
                    crate::agent::GameLogEvent::stack(format!("Cascade found: {}", card.card_name))
                        .with_player(player)
                        .with_card(top_id),
                );
                break;
            }
            exiled_ids.push(top_id);
        }

        // Snapshot so the player can see the exiled cards
        agents[player.index()].snapshot_state(game, &self.mana_pools);

        // Optionally cast the found card for free (no mana payment)
        if let Some(cascade_card_id) = found_card {
            let card = game.card(cascade_card_id);
            let card_name = card.card_name.clone();

            // Ask the player whether they want to cast the found card
            let wants_to_cast = agents[player.index()].choose_optional_trigger(
                player,
                &format!("Cascade: Cast {} without paying its mana cost?", card_name),
                Some(&card_name),
                None,
            );

            if wants_to_cast {
                let card = game.card(cascade_card_id);
                let is_creature = card.is_creature();
                let is_permanent = card.is_permanent();
                let mut sa = crate::spellability::build_spell_ability_for_card_cast(
                    game,
                    cascade_card_id,
                    player,
                );

                // Snapshot before targeting so the UI shows current game state
                agents[player.index()].snapshot_state(game, &self.mana_pools);
                sa.setup_targets(game, agents, &self.mana_pools);

                let entry = StackEntry {
                    id: 0,
                    spell_ability: sa,
                    is_creature_spell: is_creature,
                    is_permanent_spell: is_permanent,
                    cast_from_zone: Some(ZoneType::Exile),
                    optional_trigger_decider: None,
                    optional_trigger_description: None,
                    optional_trigger_source_name: None,
                };
                game.stack.push(entry);
                self.log_stack_push(&card_name, &game.player(player).name);
                crate::agent::notify_all_agents(
                    agents,
                    crate::agent::GameLogEvent::stack(format!("Cascade cast: {}", card_name))
                        .with_player(player)
                        .with_card(cascade_card_id),
                );
                self.move_card_with_runtime(game, cascade_card_id, ZoneType::Stack, player, agents);

                // Cascade spell counts as being cast
                {
                    game.player_record_spell_cast(player, cascade_card_id);
                }
                game.stack.record_spell_cast(cascade_card_id);
                self.trigger_handler.run_trigger(
                    TriggerType::SpellCast,
                    RunParams {
                        spell_card: Some(cascade_card_id),
                        spell_controller: Some(player),
                        ..Default::default()
                    },
                    false,
                );
                if let Some(sa_for_target) = game.stack.peek().map(|top| top.spell_ability.clone())
                {
                    self.emit_becomes_target_triggers(
                        game,
                        player,
                        cascade_card_id,
                        Some(&sa_for_target),
                        sa_for_target.target_chosen.target_card,
                        sa_for_target.target_chosen.target_player,
                    );
                }
            } else {
                // Player declined — found card goes to bottom with the rest
                exiled_ids.push(cascade_card_id);
            }
        }

        // Put exiled cards on bottom of library in random order
        self.game_rng.shuffle_cards(&mut exiled_ids);
        for card_id in exiled_ids {
            // Move from exile to library, but at the bottom
            let card = &mut game.cards[card_id.index()];
            let src_zone = card.zone;
            let src_owner = card.controller;
            card.zone = ZoneType::Library;
            if src_zone != ZoneType::None {
                game.zone_mut(src_zone, src_owner).remove(card_id);
            }
            game.zone_mut(ZoneType::Library, player)
                .add_to_bottom(card_id);
        }
    }
}
