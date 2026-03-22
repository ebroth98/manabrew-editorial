use super::cost_payment::CostPaymentContext;
use super::*;
use forge_foundation::mana::ManaAtom;

impl GameLoop {
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
        let battlefield = game.cards_in_zone(ZoneType::Battlefield, player).to_vec();
        let can_activate = |card_id: CardId, ab: &crate::ability::ActivatedAbility| {
            // Per-game activation cap (e.g. "GameActivationLimit$ 1").
            if let Some(limit) = ab
                .params
                .get(keys::GAME_ACTIVATION_LIMIT)
                .and_then(|v| v.parse::<u32>().ok())
            {
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
            if ab.params.is_true(keys::POWER_UP) {
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
            if ab.params.is_true(keys::SORCERY_SPEED) && !can_play_sorcery {
                return false;
            }
            // Activated abilities that require targets should only be offered
            // when at least one legal target candidate exists.
            let sa_for_target_check =
                crate::spellability::build_spell_ability(game, card_id, &ab.ability_text, player);
            if crate::staticability::static_ability_cant_be_cast::cant_be_activated_ability(
                &game.cards,
                &sa_for_target_check,
                game.card(card_id),
                player,
            ) {
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
                .any(|p| matches!(p, crate::cost::CostPart::Mana(_)));
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
            if !crate::cost::can_pay_with_ability(
                &ab.cost,
                game,
                &mana_for_check,
                card_id,
                player,
                Some(&sa_for_target_check),
            ) {
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
                // Skip abilities with ActivationZone$ Hand — they're for hand, not battlefield
                if ab.params.get(keys::ACTIVATION_ZONE) == Some("Hand")
                {
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
                if ab.params.get(keys::ACTIVATION_ZONE) == Some("Hand")
                {
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
                if ab.params.get(keys::ACTIVATION_ZONE) == Some("Graveyard")
                {
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
                if ab.params.get(keys::ACTIVATION_ZONE) == Some("Exile")
                {
                    if can_activate(card_id, ab) {
                        result.push((card_id, ab.ability_index));
                    }
                }
            }
        }

        result
    }

    /// Activate an ability on a permanent.
    /// Activate an ability. Returns `true` if the ability was successfully
    /// activated (costs paid, placed on stack / resolved). Returns `false` if
    /// the activation failed (e.g. payment declined, targets invalid).
    pub(crate) fn activate_ability(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        card_id: CardId,
        ability_idx: usize,
    ) -> bool {
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
            None => return false,
        };

        if ab.is_mana_ability {
            self.resolve_mana_ability(game, agents, player, card_id, &ab);
            true
        } else if ab.ability_text.contains("Mode$ TurnFaceUp") {
            // Morph face-up is a special action (CR 702.36e): doesn't use the stack,
            // can't be responded to. Pay the cost and resolve immediately.
            self.resolve_morph_face_up(game, agents, player, card_id, &ab)
        } else {
            self.activate_ability_on_stack(game, agents, player, card_id, &ab)
        }
    }

    /// Morph turn face up: pay the morph cost and resolve immediately
    /// (special action per CR 702.36e — doesn't use the stack).
    fn resolve_morph_face_up(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        card_id: CardId,
        ab: &crate::ability::activated::ActivatedAbility,
    ) -> bool {
        // Pay costs
        let api = ab.params.get(keys::AB).and_then(crate::ability::api_type::ApiType::smart_value_of);
        if !self.pay_ability_cost(
            game,
            agents,
            player,
            card_id,
            &ab.cost,
            api,
            ab.cost.mandatory,
            CostPaymentContext::ActivatedAbility,
        ) {
            return false;
        }

        // Build the spell ability and resolve effect immediately (no stack)
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

        let card_name = game.card(card_id).card_name.clone();
        crate::agent::notify_all_agents(
            agents,
            crate::agent::GameLogEvent::action(format!("Morph face-up: {}", card_name))
                .with_player(player)
                .with_source_card(card_id),
        );

        // Apply continuous effects and SBA after the face-up
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
    ) {
        let api = ab.params.get(keys::AB).and_then(crate::ability::api_type::ApiType::smart_value_of);
        if !self.pay_ability_cost(
            game,
            agents,
            player,
            card_id,
            &ab.cost,
            api,
            ab.cost.mandatory,
            CostPaymentContext::ManaAbility,
        ) {
            return;
        }

        // If this is a ManaReflected ability, delegate to the effect resolver
        if ab.params.get(keys::AB) == Some("ManaReflected") {
            let sa = SpellAbility::new_simple(Some(card_id), player, &ab.ability_text);
            self.resolve_single_effect(game, agents, &sa, None);
            // Fire triggers
            self.trigger_handler.run_trigger(
                TriggerType::TapsForMana,
                RunParams {
                    card: Some(card_id),
                    player: Some(player),
                    ..Default::default()
                },
                false,
            );
            self.trigger_handler.run_trigger(
                TriggerType::ManaAdded,
                RunParams {
                    card: Some(card_id),
                    player: Some(player),
                    ..Default::default()
                },
                false,
            );
            return;
        }

        // Check if source permanent is snow (snow mana tracking)
        let source_is_snow = game.card(card_id).type_line.is_snow();
        // Check for mana restrictions (RestrictValid$) and uncounterability (AddsNoCounter$)
        let mana_restriction = ab.params.get_cloned(keys::RESTRICT_VALID);
        let adds_no_counter = ab.params.is_true(keys::ADDS_NO_COUNTER);
        let adds_keywords = ab.params.get_cloned(keys::ADDS_KEYWORDS);
        let adds_keywords_valid = ab.params.get_cloned(keys::ADDS_KEYWORDS_VALID);
        let adds_counters = ab.params.get_cloned(keys::ADDS_COUNTERS);
        let adds_counters_valid = ab.params.get_cloned(keys::ADDS_COUNTERS_VALID);
        let triggers_when_spent = ab.params.get_cloned(keys::TRIGGERS_WHEN_SPENT);

        // Helper: convert a ManaAtom to its short letter for mana strings.
        fn atom_to_letter(atom: u16) -> &'static str {
            match atom {
                ManaAtom::WHITE => "W",
                ManaAtom::BLUE => "U",
                ManaAtom::BLACK => "B",
                ManaAtom::RED => "R",
                ManaAtom::GREEN => "G",
                ManaAtom::COLORLESS => "C",
                _ => "C",
            }
        }

        // Determine the final mana string to produce
        let mut mana_string: Option<String> = None;

        if let Some(produced) = ab.params.get(keys::PRODUCED) {
            if produced.starts_with("Special") {
                // Delegate to the special mana handler in mana_effect
                let special = produced.strip_prefix("Special ").unwrap_or("");
                let sa = SpellAbility::new_simple(Some(card_id), player, &ab.ability_text);
                let mut effect_ctx = crate::ability::effects::EffectContext {
                    game,
                    agents,
                    trigger_handler: &mut self.trigger_handler,
                    token_templates: &self.token_templates,
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
                for tok in &tokens {
                    if let Some(atom) = mana_atom_from_produced(tok) {
                        let mut m = crate::mana::Mana::simple(atom);
                        m.is_snow = source_is_snow;
                        m.restriction = mana_restriction.clone();
                        m.adds_no_counter = adds_no_counter;
                        m.adds_keywords = adds_keywords.clone();
                        m.adds_keywords_valid = adds_keywords_valid.clone();
                        m.adds_counters = adds_counters.clone();
                        m.adds_counters_valid = adds_counters_valid.clone();
                        m.triggers_when_spent = triggers_when_spent.clone();
                        m.source_card = Some(card_id);
                        self.pool_mut(player).add_mana(m);
                    }
                }
                // Fire triggers and return
                self.trigger_handler.run_trigger(
                    TriggerType::TapsForMana,
                    RunParams {
                        card: Some(card_id),
                        player: Some(player),
                        ..Default::default()
                    },
                    false,
                );
                self.trigger_handler.run_trigger(
                    TriggerType::ManaAdded,
                    RunParams {
                        card: Some(card_id),
                        player: Some(player),
                        ..Default::default()
                    },
                    false,
                );
                return;
            } else if produced == "Combo ColorIdentity" {
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
                    .unwrap_or_default();

                if !colors.is_empty() {
                    if let Some(chosen) = agents[player.index()].choose_color(player, &colors) {
                        if let Some(atom) = color_name_to_mana_atom(&chosen) {
                            mana_string = Some(atom_to_letter(atom).to_string());
                        }
                    }
                }
            } else {
                let chosen_colors = game.card(card_id).chosen_colors.clone();
                let colors = produced_to_color_names(produced, &chosen_colors);
                if colors.len() > 1 {
                    if let Some(chosen) = agents[player.index()].choose_color(player, &colors) {
                        if let Some(atom) = color_name_to_mana_atom(&chosen) {
                            mana_string = Some(atom_to_letter(atom).to_string());
                        }
                    }
                } else if let Some(single) = colors.first() {
                    if let Some(atom) = color_name_to_mana_atom(single) {
                        mana_string = Some(atom_to_letter(atom).to_string());
                    }
                } else {
                    // Raw produced string (single or multi-token like "C C")
                    mana_string = Some(produced.to_string());
                }
            }
        }

        // Apply Amount$ multiplier (e.g. Rofellos produces mana equal to Forests)
        if let Some(ref mut ms) = mana_string {
            if let Some(amount_str) = ab.params.get(keys::AMOUNT) {
                let amount = if let Ok(n) = amount_str.parse::<i32>() {
                    n
                } else {
                    // Try to resolve as SVar on the source card
                    if let Some(svar_expr) =
                        game.card(card_id).svars.get(amount_str).cloned()
                    {
                        crate::ability::effects::resolve_count_svar(
                            &svar_expr, game, card_id, player,
                        )
                    } else {
                        1
                    }
                };
                if amount > 1 {
                    // Check if this is combo/any mana (multiple color choices)
                    let produced = ab.params.get(keys::PRODUCED).unwrap_or("");
                    let is_combo = produced.contains("Any")
                        || produced.starts_with("Combo")
                        || produced.contains(',');
                    if is_combo {
                        // Multi-amount combo: let agent choose color distribution
                        let available: Vec<String> = if produced.contains("Any") {
                            vec!["W", "U", "B", "R", "G"]
                                .into_iter()
                                .map(String::from)
                                .collect()
                        } else {
                            let chosen_colors = game.card(card_id).chosen_colors.clone();
                            let names = produced_to_color_names(produced, &chosen_colors);
                            names
                                .iter()
                                .filter_map(|name| {
                                    color_name_to_mana_atom(name)
                                        .map(|a| atom_to_letter(a).to_string())
                                })
                                .collect()
                        };
                        let card_name = game.card(card_id).card_name.clone();
                        let chosen = agents[player.index()].specify_mana_combo(
                            player,
                            &available,
                            amount as usize,
                            Some(&card_name),
                        );
                        *ms = chosen.join(" ");
                    } else {
                        let base = ms.clone();
                        for _ in 1..amount {
                            ms.push(' ');
                            ms.push_str(&base);
                        }
                    }
                } else if amount <= 0 {
                    mana_string = None;
                }
            }
        }

        // Apply ProduceMana replacement effects (mana doublers like Mirari's Wake)
        if let Some(ref mut ms) = mana_string {
            use crate::replacement::replacement_handler::{apply_replacements, ReplacementEvent};
            let mut event = ReplacementEvent::ProduceMana {
                source: card_id,
                activator: player,
                mana: ms.clone(),
            };
            let result = apply_replacements(game, &mut event);
            if result == crate::replacement::ReplacementResult::Updated {
                if let ReplacementEvent::ProduceMana { mana: new_mana, .. } = event {
                    *ms = new_mana;
                }
            }
        }

        // Add the (possibly multiplied) mana to the pool
        if let Some(ref ms) = mana_string {
            for tok in ms.split_whitespace() {
                if let Some(atom) = mana_atom_from_produced(tok) {
                    let mut m = crate::mana::Mana::simple(atom);
                    m.is_snow = source_is_snow;
                    m.restriction = mana_restriction.clone();
                    m.adds_no_counter = adds_no_counter;
                    m.adds_keywords = adds_keywords.clone();
                    m.adds_keywords_valid = adds_keywords_valid.clone();
                    m.adds_counters = adds_counters.clone();
                    m.adds_counters_valid = adds_counters_valid.clone();
                    m.triggers_when_spent = triggers_when_spent.clone();
                    m.source_card = Some(card_id);
                    self.pool_mut(player).add_mana(m);
                }
            }
        }

        // Resolve SubAbility chain (e.g. DealDamage on pain lands)
        if let Some(sub_svar_name) = ab.params.get(keys::SUB_ABILITY) {
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

        // Fire ManaAdded trigger (mirrors Java AbilityManaPart.produceMana)
        self.trigger_handler.run_trigger(
            TriggerType::ManaAdded,
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

        // Fire BecomesTarget trigger if a card was targeted
        if let Some(target_card) = sa.target_chosen.target_card {
            self.trigger_handler.run_trigger(
                TriggerType::BecomesTarget,
                RunParams {
                    card: Some(target_card),
                    cause_player: Some(player),
                    cause_card: Some(card_id),
                    ..Default::default()
                },
                false,
            );
        }

        // PowerUp: reduce cost by card's mana cost if it entered the battlefield this turn
        let adjusted_cost = if ab.params.is_true(keys::POWER_UP)
            && game.card(card_id).entered_battlefield_this_turn
        {
            let mut cost = ab.cost.clone();
            // Subtract the card's mana cost from the ability's mana cost
            let card_mc = game.card(card_id).mana_cost.clone();
            for part in &mut cost.parts {
                if let crate::cost::CostPart::Mana(ref mut mc) = part {
                    *mc = mc.reduce_generic(card_mc.cmc() as i32);
                    break;
                }
            }
            cost
        } else {
            ab.cost.clone()
        };

        // Pay costs
        let api = ab.params.get(keys::AB).and_then(crate::ability::api_type::ApiType::smart_value_of);
        if !self.pay_ability_cost(
            game,
            agents,
            player,
            card_id,
            &adjusted_cost,
            api,
            adjusted_cost.mandatory,
            CostPaymentContext::ActivatedAbility,
        ) {
            return false;
        }

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
        game.stack.push(entry);
        self.log_stack_push(&format!("{} ability", card_name), &game.player(player).name);
        let ability_kind = ab
            .params
            .get(keys::AB)
            .unwrap_or("Unknown")
            .to_string();
        let mut event = crate::agent::GameLogEvent::action(format!(
            "Activated ability: {} | source={}",
            ability_kind, card_name
        ))
        .with_player(player)
        .with_source_card(card_id);
        if let Some(target_id) = target_card {
            event = event.with_target_card(target_id);
        }
        crate::agent::notify_all_agents(agents, event);

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
        true
    }
}
