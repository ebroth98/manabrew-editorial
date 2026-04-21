use super::cost_payment::CostPaymentContext;
use super::*;

impl GameLoop {
    fn fixed_reserved_sacrifices_for_action(sa: &SpellAbility, source: CardId) -> Vec<CardId> {
        let mut reserved = Vec::new();
        let Some(pay_costs) = sa.pay_costs.as_ref() else {
            return reserved;
        };
        for part in &pay_costs.parts {
            if let CostPart::Sacrifice { type_filter, .. } = part {
                let reserved_card = match type_filter.as_str() {
                    "CARDNAME" | "NICKNAME" => Some(source),
                    "OriginalHost" => sa.original_host,
                    _ => None,
                };
                if let Some(card_id) = reserved_card {
                    if !reserved.contains(&card_id) {
                        reserved.push(card_id);
                    }
                }
            }
        }
        reserved
    }

    pub(crate) fn emit_tap_for_mana_triggers(&mut self, player: PlayerId, tapped_lands: &[CardId]) {
        for &land_id in tapped_lands {
            self.trigger_handler.run_trigger(
                TriggerType::Taps,
                RunParams {
                    card: Some(land_id),
                    player: Some(player),
                    ..Default::default()
                },
                false,
            );
            self.trigger_handler.run_trigger(
                TriggerType::TapsForMana,
                RunParams {
                    card: Some(land_id),
                    player: Some(player),
                    ..Default::default()
                },
                false,
            );
        }
    }

    pub(crate) fn get_activatable_abilities(
        &self,
        game: &GameState,
        player: PlayerId,
        can_play_sorcery: bool,
    ) -> Vec<(CardId, usize)> {
        let mut result = Vec::new();
        let available_mana = mana::calculate_available_mana(self.pool(player), game, player);
        let mut battlefield = game.cards_in_zone(ZoneType::Battlefield, player).to_vec();
        for &other_player in &game.player_order {
            if other_player == player {
                continue;
            }
            battlefield.extend(game.cards_in_zone(ZoneType::Battlefield, other_player));
        }
        let can_activate = |card_id: CardId, ab: &crate::ability::ActivatedAbility| {
            // Per-game activation cap (e.g. "GameActivationLimit$ 1").
            if let Some(limit) = ab.game_activation_limit {
                let used = game
                    .card(card_id)
                    .activations_this_game
                    .get(&ab.ability_index)
                    .copied()
                    .unwrap_or(0);
                if used >= limit {
                    return false;
                }
            }
            // PowerUp: once-per-game restriction
            if ab.power_up {
                let card = game.card(card_id);
                if card
                    .activations_this_game
                    .get(&ab.ability_index)
                    .copied()
                    .unwrap_or(0)
                    > 0
                {
                    return false;
                }
            }
            if ab.sorcery_speed && !can_play_sorcery {
                return false;
            }
            // Activated abilities that require targets should only be offered
            // when at least one legal target candidate exists.
            let sa_for_target_check =
                crate::spellability::build_spell_ability(game, card_id, &ab.ability_text, player);
            if crate::staticability::static_ability_cant_be_cast::cant_be_activated_ability(
                game,
                &game.cards,
                &sa_for_target_check,
                game.card(card_id),
                player,
            ) {
                return false;
            }
            // Activated-ability legality checks (split second, suppression, detention, etc.).
            if !crate::spellability::ability_activated::can_play(&sa_for_target_check, game) {
                return false;
            }
            if let Some(tr) = sa_for_target_check.target_restrictions.as_ref() {
                let min_targets = tr.get_min_targets(game, &sa_for_target_check);
                if min_targets > 0
                    && !crate::spellability::target_restrictions::has_candidates_in_chain(
                        game,
                        player,
                        &ab.ability_text,
                        Some(card_id),
                    )
                {
                    return false;
                }
            }
            // Java parity: Equip/Attach abilities are offered if ValidTgts
            // candidates exist, without checking CantAttach statics.  CantAttach
            // is enforced at resolution time, not during action-space generation.
            // This matches Java's ActionSpace.hasValidTargets() behavior.
            let needs_mana = ab
                .cost
                .parts
                .iter()
                .any(|p| matches!(p, crate::cost::CostPart::Mana { .. }));
            let reserved_sacrifices =
                Self::fixed_reserved_sacrifices_for_action(&sa_for_target_check, card_id);
            let mana_for_check = if needs_mana {
                // Java parity: ComputerUtilMana.canPayManaCost(...) excludes mana
                // abilities on the same host card as the spell/ability being paid for.
                // Without that, cards like Gilded Goose incorrectly appear able to pay
                // for their own non-mana activated abilities.
                mana::calculate_available_mana_excluding(
                    self.pool(player),
                    game,
                    player,
                    Some(card_id),
                )
            } else {
                available_mana.clone()
            };
            let can_pay_cost = if reserved_sacrifices.is_empty() {
                crate::cost::can_pay_with_ability(
                    &ab.cost,
                    game,
                    &mana_for_check,
                    card_id,
                    player,
                    Some(&sa_for_target_check),
                )
            } else {
                crate::cost::can_pay_ignoring_mana_with_ability(
                    &ab.cost,
                    game,
                    card_id,
                    player,
                    &sa_for_target_check,
                ) && mana::can_pay_mana_cost_with_reserved_sacrifices(
                    game,
                    self.pool(player),
                    player,
                    card_id,
                    &ab.cost,
                    &reserved_sacrifices,
                    Some(&crate::mana::payment_context_for_sa(
                        game,
                        &sa_for_target_check,
                    )),
                )
            };
            if !can_pay_cost {
                return false;
            }
            true
        };

        for card_id in battlefield {
            let card = game.card(card_id);
            // Face-down creatures only expose morph turn-face-up ability (game rule).
            // All other abilities are hidden while face-down.
            if card.face_down {
                for ab in &card.activated_abilities {
                    if ab.ability_text.contains("Mode$ TurnFaceUp") {
                        if can_activate(card_id, ab) {
                            result.push((card_id, ab.ability_index));
                        }
                    }
                }
                continue;
            }
            for ab in &card.activated_abilities {
                if ab.is_mana_ability || ab.is_unlock_door {
                    continue;
                }
                // Skip abilities with ActivationZone$ Hand — they're for hand, not battlefield
                if ab.activation_zone == Some(ZoneType::Hand) {
                    continue;
                }
                if can_activate(card_id, ab) {
                    result.push((card_id, ab.ability_index));
                }
            }
        }

        // Check hand for abilities with ActivationZone$ Hand (e.g. Cycling)
        let hand = game.cards_in_zone(ZoneType::Hand, player).to_vec();
        for card_id in hand {
            let card = game.card(card_id);
            for ab in &card.activated_abilities {
                if ab.is_mana_ability || ab.is_unlock_door {
                    continue;
                }
                if ab.activation_zone == Some(ZoneType::Hand) {
                    if can_activate(card_id, ab) {
                        result.push((card_id, ab.ability_index));
                    }
                }
            }
        }

        // Check graveyard for abilities with ActivationZone$ Graveyard (e.g. Scavenge, Unearth)
        let graveyard = game.cards_in_zone(ZoneType::Graveyard, player).to_vec();
        for card_id in graveyard {
            let card = game.card(card_id);
            for ab in &card.activated_abilities {
                if ab.is_mana_ability || ab.is_unlock_door {
                    continue;
                }
                if ab.activation_zone == Some(ZoneType::Graveyard) {
                    if can_activate(card_id, ab) {
                        result.push((card_id, ab.ability_index));
                    }
                }
            }
        }

        // Check exile for abilities with ActivationZone$ Exile
        let exile = game.cards_in_zone(ZoneType::Exile, player).to_vec();
        for card_id in exile {
            let card = game.card(card_id);
            for ab in &card.activated_abilities {
                if ab.is_mana_ability || ab.is_unlock_door {
                    continue;
                }
                if ab.activation_zone == Some(ZoneType::Exile) {
                    if can_activate(card_id, ab) {
                        result.push((card_id, ab.ability_index));
                    }
                }
            }
        }

        result
    }

    /// Resolve an ability immediately without using the stack.
    ///
    /// Used for abilities that Java models as `AbilityStatic` (e.g. Plot) and
    /// for special actions like turning a Morph face up.
    pub(crate) fn resolve_immediate_ability(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        card_id: CardId,
        ab: &crate::ability::activated::ActivatedAbility,
    ) -> bool {
        // Pay costs
        let api = ab
            .params
            .get(keys::AB)
            .and_then(crate::ability::api_type::ApiType::smart_value_of);
        if !self.pay_ability_cost(
            game,
            agents,
            player,
            card_id,
            &ab.cost,
            api,
            ab.cost.mandatory,
            CostPaymentContext::ActivatedAbility,
            None,
        ) {
            return false;
        }

        let card_name = game.card(card_id).card_name.clone();
        let ability_kind = ab.params.get(keys::AB).unwrap_or("Unknown").to_string();
        crate::agent::notify_all_agents(
            agents,
            crate::agent::GameLogEvent::action(format!(
                "Activated ability: {} | source={}",
                ability_kind, card_name
            ))
            .with_player(player)
            .with_source_card(card_id),
        );

        // Build the spell ability and resolve effect immediately (no stack).
        let sa = crate::spellability::build_spell_ability(game, card_id, &ab.ability_text, player);
        let entry = StackEntry {
            id: 0,
            spell_ability: sa,
            is_creature_spell: false,
            is_permanent_spell: false,
            cast_from_zone: None,
            optional_trigger_decider: None,
            optional_trigger_description: None,
            optional_trigger_source_name: None,
        };
        self.resolve_spell_effect(game, agents, &entry);

        // Apply continuous effects and SBA after the immediate resolution.
        crate::staticability::layer::apply_continuous_effects(game);
        super::check_sba(game, &mut self.trigger_handler, agents);
        self.process_triggers(game, agents);
        true
    }

    /// Resolve a mana ability immediately (no stack).
    pub(crate) fn resolve_mana_ability(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        card_id: CardId,
        ab: &crate::ability::activated::ActivatedAbility,
        express_choice: Option<u16>,
    ) {
        let api = ab
            .params
            .get(keys::AB)
            .and_then(crate::ability::api_type::ApiType::smart_value_of);
        if !self.pay_ability_cost(
            game,
            agents,
            player,
            card_id,
            &ab.cost,
            api,
            ab.cost.mandatory,
            CostPaymentContext::ManaAbility,
            None,
        ) {
            return;
        }

        // If this is a ManaReflected ability, delegate to the effect resolver
        if ab.params.get(keys::AB) == Some("ManaReflected") {
            let mut sa =
                crate::spellability::build_spell_ability(game, card_id, &ab.ability_text, player);
            sa.express_mana_choice = express_choice;
            self.resolve_single_effect(game, agents, &sa, None);
            // Fire triggers
            self.trigger_handler.run_trigger(
                TriggerType::TapsForMana,
                RunParams {
                    card: Some(card_id),
                    player: Some(player),
                    activator: Some(player),
                    ..Default::default()
                },
                false,
            );
            self.trigger_handler.run_trigger(
                TriggerType::ManaAdded,
                RunParams {
                    card: Some(card_id),
                    player: Some(player),
                    activator: Some(player),
                    ..Default::default()
                },
                false,
            );
            return;
        }

        // Build metadata params for produced mana
        let source_is_snow = game.card(card_id).type_line.is_snow();
        let mana_params = crate::mana::ManaProductionParams {
            source_card: card_id,
            is_snow: source_is_snow,
            restriction: ab.params.get_cloned(keys::RESTRICT_VALID),
            adds_no_counter: ab.params.is_true(keys::ADDS_NO_COUNTER),
            adds_keywords: ab.params.get_cloned(keys::ADDS_KEYWORDS),
            adds_keywords_valid: ab.params.get_cloned(keys::ADDS_KEYWORDS_VALID),
            adds_counters: ab.params.get_cloned(keys::ADDS_COUNTERS),
            adds_counters_valid: ab.params.get_cloned(keys::ADDS_COUNTERS_VALID),
            triggers_when_spent: ab.params.get_cloned(keys::TRIGGERS_WHEN_SPENT),
        };

        if let Some(produced) = ab.params.get(keys::PRODUCED) {
            if produced.starts_with("Special") {
                // Delegate to the special mana handler in mana_effect
                let special = produced.strip_prefix("Special ").unwrap_or("");
                let sa = crate::spellability::build_spell_ability(
                    game,
                    card_id,
                    &ab.ability_text,
                    player,
                );
                let mut effect_ctx = crate::ability::effects::EffectContext {
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
                };
                let tokens = crate::ability::effects::mana_effect::resolve_special_mana(
                    &mut effect_ctx,
                    &sa,
                    card_id,
                    player,
                    special,
                );
                // Add produced mana with metadata
                let mana_str = tokens.join(" ");
                crate::mana::add_produced_mana_to_pool(
                    &mut effect_ctx.mana_pools[player.index()],
                    &mana_str,
                    &mana_params,
                );
                drop(effect_ctx);
                // Fire triggers and return
                self.trigger_handler.run_trigger(
                    TriggerType::TapsForMana,
                    RunParams {
                        card: Some(card_id),
                        player: Some(player),
                        activator: Some(player),
                        ..Default::default()
                    },
                    false,
                );
                self.trigger_handler.run_trigger(
                    TriggerType::ManaAdded,
                    RunParams {
                        card: Some(card_id),
                        player: Some(player),
                        activator: Some(player),
                        ..Default::default()
                    },
                    false,
                );
                return;
            }

            // Determine mana production (color choice, Amount$, replacement effects)
            let amount_param = ab.params.get(keys::AMOUNT);
            let mana_string = crate::mana::determine_mana_production(
                game,
                agents,
                player,
                card_id,
                produced,
                amount_param,
                express_choice,
            );

            // Add the produced mana to the pool
            if let Some(ref ms) = mana_string {
                crate::mana::add_produced_mana_to_pool(self.pool_mut(player), ms, &mana_params);
            }
        }

        // Resolve SubAbility chain (e.g. DealDamage on pain lands)
        if let Some(sub_svar_name) = ab.params.get(keys::SUB_ABILITY) {
            if let Some(sub_text) = game.card(card_id).svars.get(sub_svar_name).cloned() {
                let sub_sa =
                    crate::spellability::build_spell_ability(game, card_id, &sub_text, player);
                self.resolve_single_effect(game, agents, &sub_sa, None);
            }
        }

        // Fire TapsForMana trigger (mana abilities produce mana)
        self.trigger_handler.run_trigger(
            TriggerType::TapsForMana,
            RunParams {
                card: Some(card_id),
                player: Some(player),
                activator: Some(player),
                ..Default::default()
            },
            false,
        );

        // Fire ManaAdded trigger (mirrors Java AbilityManaPart.produceMana)
        self.trigger_handler.run_trigger(
            TriggerType::ManaAdded,
            RunParams {
                card: Some(card_id),
                player: Some(player),
                activator: Some(player),
                ..Default::default()
            },
            false,
        );

        // Resolve mana-producing triggers inline (Static$ True triggers like Utopia Sprawl).
        // These fire from TapsForMana and produce extra mana without using the stack.
        // Mirrors Java's AbilityStatic resolution path for mana triggers.
        let pending = self.trigger_handler.run_waiting_triggers(game);
        for pt in pending {
            self.resolve_single_effect(game, agents, &pt.entry.spell_ability, None);
        }
    }

    /// Activate a non-mana ability: choose targets, pay costs, put on stack.
    pub(crate) fn play_activated_ability_on_stack(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        card_id: CardId,
        ab: &crate::ability::activated::ActivatedAbility,
    ) -> bool {
        let ability_text = ab.ability_text.clone();

        // Build full SpellAbility chain (including SubAbility$ links) and choose targets.
        // This mirrors Java activated ability resolution for abilities like Walking Bulwark,
        // where the root AB$ Pump resolves a DB$ Effect sub-ability.
        let mut sa = crate::spellability::build_spell_ability(game, card_id, &ability_text, player);
        sa.is_activated = true;
        if sa.api == Some(crate::ability::api_type::ApiType::Charm)
            && !crate::ability::effects::charm_effect::make_choices_precast(game, agents, &mut sa)
        {
            return false;
        }
        if !sa.setup_targets(game, agents, &self.mana_pools) {
            return false;
        }
        crate::ability::effects::emit_targeting_triggers_for_sa(
            &mut self.trigger_handler,
            game,
            card_id,
            &sa,
        );

        // PowerUp: reduce cost by card's mana cost if it entered the battlefield this turn
        let adjusted_cost = if ab.params.is_true(keys::POWER_UP)
            && game.card(card_id).entered_battlefield_this_turn
        {
            let mut cost = ab.cost.clone();
            // Subtract the card's mana cost from the ability's mana cost
            let card_mc = game.card(card_id).mana_cost.clone();
            for part in &mut cost.parts {
                if let crate::cost::CostPart::Mana {
                    cost: ref mut mc, ..
                } = part
                {
                    *mc = mc.reduce_generic(card_mc.cmc() as i32);
                    break;
                }
            }
            cost
        } else {
            ab.cost.clone()
        };
        let host_before_payment = game.card(card_id).clone();
        let spell_desc = ab
            .params
            .get(keys::SPELL_DESCRIPTION)
            .unwrap_or("")
            .to_ascii_lowercase();
        let is_vehicle_crew = host_before_payment.type_line.has_subtype("Vehicle")
            && (host_before_payment
                .has_keyword_enum(crate::keyword::keyword_instance::Keyword::Crew)
                || spell_desc.starts_with("crew"));
        let is_mount_saddle = host_before_payment.type_line.has_subtype("Mount")
            && (host_before_payment
                .has_keyword_enum(crate::keyword::keyword_instance::Keyword::Saddle)
                || spell_desc.starts_with("saddle"));
        let is_station = (host_before_payment.type_line.has_subtype("Spacecraft")
            || host_before_payment.type_line.has_subtype("Planet"))
            && (host_before_payment
                .has_keyword_enum(crate::keyword::keyword_instance::Keyword::Station)
                || spell_desc.starts_with("station"));
        let uses_waterbend = adjusted_cost
            .parts
            .iter()
            .any(|p| matches!(p, crate::cost::CostPart::Waterbend { .. }));
        let untapped_before_payment: Vec<CardId> = game
            .cards_in_zone(ZoneType::Battlefield, player)
            .iter()
            .copied()
            .filter(|&cid| !game.card(cid).tapped)
            .collect();

        // Pay costs
        let api = ab
            .params
            .get(keys::AB)
            .and_then(crate::ability::api_type::ApiType::smart_value_of);
        if !self.pay_ability_cost(
            game,
            agents,
            player,
            card_id,
            &adjusted_cost,
            api,
            adjusted_cost.mandatory,
            CostPaymentContext::ActivatedAbility,
            Some(&sa),
        ) {
            return false;
        }
        let tapped_by_activation: Vec<CardId> = untapped_before_payment
            .into_iter()
            .filter(|&cid| game.card(cid).tapped)
            .collect();
        let tapped_crews: Vec<CardId> = tapped_by_activation
            .iter()
            .copied()
            .filter(|&cid| cid != card_id && game.card(cid).is_creature())
            .collect();

        // Track activation count (for PowerUp once-per-game)
        game.card_mut(card_id)
            .activations_this_game
            .entry(ab.ability_index)
            .and_modify(|c| *c += 1)
            .or_insert(1);

        // Push to stack
        let card_name = game.card(card_id).card_name.clone();
        let target_card = sa.target_chosen.target_card;
        let entry = StackEntry {
            id: 0,
            spell_ability: sa,
            is_creature_spell: false,
            is_permanent_spell: false,
            cast_from_zone: None,
            optional_trigger_decider: None,
            optional_trigger_description: None,
            optional_trigger_source_name: None,
        };
        let ability_kind = ab.params.get(keys::AB).unwrap_or("Unknown");
        let stack_message = format!("Activated ability: {} | source={}", ability_kind, card_name);
        let sa_for_trigger = self.push_spell_ability_to_stack(
            game,
            agents,
            player,
            StackPushContext {
                source_card: card_id,
                entry,
                stack_log_name: format!("{} ability", card_name),
                stack_message,
                target_card,
                event_kind: SpellAbilityLogEventKind::Action,
                move_source_to_stack: false,
                register_source_trigger: false,
            },
        );
        self.emit_post_stack_spell_ability_triggers(
            game,
            player,
            &sa_for_trigger,
            PostStackTriggerContext {
                source_card: card_id,
                cast_trigger: TriggerType::AbilityCast,
                emit_ability_activated: true,
                emit_waterbend: uses_waterbend,
                waterbend_cards: Vec::new(),
            },
        );
        if is_vehicle_crew || is_mount_saddle || is_station {
            for crew_card in &tapped_crews {
                let run_params = RunParams {
                    card: Some(card_id),
                    crew_cards: Some(vec![*crew_card]),
                    source_sa: Some(sa_for_trigger.clone()),
                    spell_ability: Some(sa_for_trigger.clone()),
                    cause: Some(sa_for_trigger.clone()),
                    cause_card: Some(card_id),
                    player: Some(player),
                    ..Default::default()
                };
                if is_vehicle_crew {
                    self.trigger_handler.run_trigger(
                        TriggerType::Crewed,
                        run_params.clone(),
                        false,
                    );
                }
                if is_mount_saddle {
                    self.trigger_handler.run_trigger(
                        TriggerType::Saddled,
                        run_params.clone(),
                        false,
                    );
                }
                if is_station {
                    self.trigger_handler.run_trigger(
                        TriggerType::Stationed,
                        run_params.clone(),
                        false,
                    );
                }
            }
        }
        true
    }
}
