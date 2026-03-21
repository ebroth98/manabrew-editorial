use super::*;

pub(crate) enum CostPaymentContext {
    TriggerResolve,
    ActivatedAbility,
    ManaAbility,
}

impl GameLoop {
    /// Pay life and fire the LifeLost trigger.
    fn pay_life_cost(&mut self, game: &mut GameState, player: PlayerId, amount: i32) {
        if crate::staticability::static_ability_cant_gain_lose_pay_life::cant_pay_life(
            game, player, true, None,
        ) {
            return;
        }
        // Run PayLife replacement effects before paying life.
        {
            use crate::replacement::replacement_handler::{apply_replacements, ReplacementEvent};
            use crate::replacement::ReplacementResult;
            let mut event = ReplacementEvent::PayLife {
                player,
                amount,
            };
            let result = apply_replacements(game, &mut event);
            if result == ReplacementResult::Skipped || result == ReplacementResult::Replaced {
                return;
            }
        }
        game.player_mut(player).lose_life(amount);
        self.trigger_handler.run_trigger(
            TriggerType::LifeLost,
            RunParams {
                player: Some(player),
                life_amount: Some(amount),
                ..Default::default()
            },
            false,
        );
    }

    /// Discard N cards from hand via agent choice and fire Discarded triggers.
    /// Mirrors Java's `CostDiscard.doListPayment()`.
    fn pay_discard_cost(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        source: CardId,
        type_filter: &str,
        amount: i32,
    ) -> Vec<CardId> {
        // Build eligible hand cards — filtered by type if the cost specifies it.
        let eligible: Vec<CardId> = if type_filter == "Card" || type_filter.is_empty() {
            game.cards_in_zone(ZoneType::Hand, player).to_vec()
        } else {
            game.cards_in_zone(ZoneType::Hand, player)
                .iter()
                .copied()
                .filter(|&cid| {
                    crate::ability::effects::matches_change_type(game.card(cid), type_filter, &[])
                })
                .collect()
        };
        let eligible: Vec<CardId> = eligible
            .into_iter()
            .filter(|&cid| cid != source || game.card(source).owner != player)
            .collect();
        let chosen = agents[player.index()].choose_discard(player, &eligible, amount as usize);
        for &cid in &chosen {
            crate::ability::effects::helpers::discard_with_madness_replacement(
                game,
                &mut self.trigger_handler,
                cid,
                player,
            );
        }
        chosen
    }

    fn cost_part_kind(part: &CostPart) -> &'static str {
        match part {
            CostPart::Tap => "Tap",
            CostPart::Untap => "Untap",
            CostPart::Mana(_) => "Mana",
            CostPart::PayLife(_) => "PayLife",
            CostPart::Sacrifice { .. } => "Sacrifice",
            CostPart::Discard { .. } => "Discard",
            CostPart::ExileFromAnyGrave { .. } => "ExileFromAnyGrave",
            CostPart::ExileFromSameGrave { .. } => "ExileFromSameGrave",
            CostPart::SubCounter { .. } => "SubCounter",
            CostPart::AddCounter { .. } => "AddCounter",
            CostPart::Exile { .. } => "Exile",
            CostPart::Return { .. } => "Return",
            CostPart::TapType { .. } => "TapType",
            CostPart::UntapType { .. } => "UntapType",
            CostPart::PayEnergy(_) => "PayEnergy",
            CostPart::PayShards(_) => "PayShards",
            CostPart::DamageYou(_) => "DamageYou",
            CostPart::Draw(_) => "Draw",
            CostPart::Mill(_) => "Mill",
            CostPart::Reveal { .. } => "Reveal",
            CostPart::Exert { .. } => "Exert",
            CostPart::GainLife(_) => "GainLife",
            CostPart::GainControl { .. } => "GainControl",
            CostPart::RemoveAnyCounter { .. } => "RemoveAnyCounter",
            CostPart::Unattach => "Unattach",
            CostPart::ExiledMoveToGrave { .. } => "ExiledMoveToGrave",
            CostPart::AddMana { .. } => "AddMana",
            CostPart::Waterbend { .. } => "Waterbend",
            CostPart::ChooseColor(_) => "ChooseColor",
            CostPart::ChooseCreatureType(_) => "ChooseCreatureType",
            CostPart::FlipCoin(_) => "FlipCoin",
            CostPart::RollDice { .. } => "RollDice",
            CostPart::ExileFromStack { .. } => "ExileFromStack",
            CostPart::CollectEvidence(_) => "CollectEvidence",
            CostPart::Forage => "Forage",
            CostPart::PutCardToLib { .. } => "PutCardToLib",
            CostPart::Enlist { .. } => "Enlist",
            CostPart::PromiseGift => "PromiseGift",
            CostPart::RevealChosen { .. } => "RevealChosen",
            CostPart::Behold { .. } => "Behold",
            CostPart::Blight(_) => "Blight",
            CostPart::ExileCtrlOrGrave { .. } => "ExileCtrlOrGrave",
        }
    }

    fn should_confirm_payment(
        part: &CostPart,
        source_is_planeswalker: bool,
        mandatory: bool,
    ) -> bool {
        match part {
            // HumanCostDecision.confirmAction(...) branches
            CostPart::AddMana { .. } => true,
            CostPart::DamageYou(_) => true,
            CostPart::Draw(_) => true,
            CostPart::Exile {
                type_filter, from, ..
            } => {
                type_filter == "All"
                    || type_filter == "CARDNAME"
                    || type_filter == "OriginalHost"
                    || *from == ZoneType::Library
            }
            CostPart::Exert { type_filter, .. } => {
                type_filter == "CARDNAME" || type_filter == "OriginalHost"
            }
            CostPart::FlipCoin(_) => true,
            CostPart::Forage => true,
            CostPart::Mill(_) => true,
            CostPart::PayLife(_) => !mandatory,
            CostPart::PayEnergy(_) => true,
            CostPart::PayShards(_) => true,
            CostPart::PutCardToLib { type_filter, .. } => {
                type_filter == "CARDNAME" || type_filter == "OriginalHost"
            }
            CostPart::Return { type_filter, .. } => {
                type_filter == "CARDNAME" || type_filter == "OriginalHost"
            }
            CostPart::RollDice { .. } => true,
            CostPart::Sacrifice { type_filter, .. } => {
                (type_filter == "CARDNAME" && !mandatory) || type_filter == "OriginalHost"
            }
            CostPart::SubCounter { .. } => !source_is_planeswalker,
            CostPart::Unattach => true,

            // HumanPlay.payCostDuringAbilityResolve(...) explicit branches
            CostPart::Discard { type_filter, .. } => {
                type_filter == "Hand" || type_filter == "Random"
            }
            CostPart::Mana(mana_cost) => mana_cost.is_zero(),
            _ => false,
        }
    }

    fn confirm_cost_part_payment(
        &mut self,
        game: &GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        source: CardId,
        part: &CostPart,
        api: Option<&str>,
        mandatory: bool,
    ) -> bool {
        let source_is_planeswalker = game.card(source).type_line.is_planeswalker();
        if !Self::should_confirm_payment(part, source_is_planeswalker, mandatory) {
            return true;
        }
        let card_name = game.card(source).card_name.clone();
        let kind = Self::cost_part_kind(part);
        let message = format!("Pay {} cost for {}?", kind, card_name);
        agents[player.index()].confirm_payment(player, kind, &message, Some(&card_name), api)
    }

    /// Pay the cost parts of an activated ability (tap, mana, life, sacrifice, etc.).
    /// Mirrors Java's `CostPayment.payCost()` iterating over `CostPart`s.
    pub(crate) fn pay_ability_cost(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        card_id: CardId,
        cost: &crate::cost::Cost,
        api: Option<&str>,
        mandatory: bool,
        context: CostPaymentContext,
    ) -> bool {
        let _ = context;
        // Java CostPayment is transactional: if any later cost part fails,
        // previously applied parts are undone. Mirror that via full snapshot.
        let payment_snapshot = self.make_snapshot(game, true);
        let mut payment_ok = true;

        // Java's CostPayment.payComputerCosts uses a two-phase approach:
        // Phase 1 (accept/visit): iterate all cost parts, gather decisions
        //   - CostDiscard.visit() → pick cards to discard (consumes RNG)
        //   - CostSacrifice(CARDNAME).visit() → confirmPayment (consumes RNG)
        //   - Other parts → no-op
        // Phase 2 (payAsDecided): iterate all cost parts, execute payments
        //   - CostPartMana → auto-tap (may trigger mana abilities, consuming RNG)
        //   - CostDiscard → discard the pre-picked cards
        //   - CostSacrifice → sacrifice
        //
        // We match this by pre-picking discards and pre-confirming sacrifices
        // before the main payment loop.

        // Phase 1: visit/decide (matching Java's accept loop order).
        let mut pre_picked_discards: Vec<CardId> = Vec::new();
        for part in cost.parts.clone() {
            match &part {
                CostPart::Discard {
                    type_filter,
                    amount,
                } => {
                    if type_filter != "CARDNAME" {
                        // Pre-pick discard cards (mirrors Java CostDiscard.visit → pickCards)
                        let eligible: Vec<CardId> =
                            if type_filter == "Card" || type_filter.is_empty() {
                                game.cards_in_zone(ZoneType::Hand, player).to_vec()
                            } else {
                                game.cards_in_zone(ZoneType::Hand, player)
                                    .iter()
                                    .copied()
                                    .filter(|&cid| {
                                        crate::ability::effects::matches_change_type(
                                            game.card(cid),
                                            type_filter,
                                            &[],
                                        )
                                    })
                                    .collect()
                            };
                        let eligible: Vec<CardId> = eligible
                            .into_iter()
                            .filter(|&cid| cid != card_id || game.card(card_id).owner != player)
                            .collect();
                        let chosen = agents[player.index()].choose_discard(
                            player,
                            &eligible,
                            *amount as usize,
                        );
                        pre_picked_discards.extend(chosen);
                    }
                }
                _ => {
                    // Confirm decisions for parts that need them
                    if !self.confirm_cost_part_payment(
                        game, agents, player, card_id, &part, api, mandatory,
                    ) {
                        payment_ok = false;
                        break;
                    }
                }
            }
        }
        if !payment_ok {
            self.restore_snapshot(game, &payment_snapshot);
            return false;
        }

        // Phase 2: execute payments.
        for part in cost.parts.clone() {
            match &part {
                CostPart::Tap => {
                    game.tap(card_id);
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
                CostPart::Untap => {
                    game.untap(card_id);
                }
                CostPart::Mana(mana_cost) => {
                    // Check if this ability also sacrifices itself (e.g. Food Token
                    // "{2}, {T}, Sacrifice this: Gain 3 life"). If so, allow the
                    // auto-tapper to reuse the reserved source for mana.
                    let reuses_reserved_source = cost.parts.iter().any(|part| {
                        matches!(
                            part,
                            CostPart::Sacrifice { type_filter, .. } if type_filter == "CARDNAME"
                        )
                    });
                    // Use full callback with confirm_payment for sacrifice and
                    // sub-counter mana abilities, matching the spell casting path
                    // in cast_spell.rs. This ensures Rasputin Dreamweaver's
                    // counter-removal mana ability triggers confirm_payment.
                    let mut callback = |kind: mana::ManaPayCallback<'_>| -> Option<CardId> {
                        match kind {
                            mana::ManaPayCallback::ChooseSacrifice(valid) => {
                                agents[player.index()].choose_sacrifice(player, valid)
                            }
                            mana::ManaPayCallback::ConfirmSelfSacrifice(sacrifice_id) => {
                                let confirmed = agents[player.index()].confirm_payment(
                                    player,
                                    "Sacrifice",
                                    "Sacrifice for mana",
                                    None,
                                    Some("Mana"),
                                );
                                if confirmed {
                                    Some(sacrifice_id)
                                } else {
                                    None
                                }
                            }
                            mana::ManaPayCallback::ConfirmSubCounter(source_id) => {
                                let confirmed = agents[player.index()].confirm_payment(
                                    player,
                                    "SubCounter",
                                    "Remove counter for mana",
                                    None,
                                    Some("Mana"),
                                );
                                if confirmed {
                                    Some(source_id)
                                } else {
                                    None
                                }
                            }
                        }
                    };
                    let tapped = if reuses_reserved_source {
                        mana::auto_tap_lands_allow_reserved_source_reuse_with_callbacks(
                            game,
                            &mut self.mana_pools[player.index()],
                            player,
                            mana_cost,
                            Some(card_id),
                            &mut callback,
                        )
                    } else {
                        mana::auto_tap_lands_with_callbacks(
                            game,
                            &mut self.mana_pools[player.index()],
                            player,
                            mana_cost,
                            Some(card_id),
                            &mut callback,
                        )
                    };
                    self.emit_tap_for_mana_triggers(player, &tapped);
                    if !self.mana_pools[player.index()].try_pay(mana_cost) {
                        payment_ok = false;
                        break;
                    }
                }
                CostPart::PayLife(amount) => {
                    self.pay_life_cost(game, player, *amount);
                }
                CostPart::Sacrifice {
                    type_filter,
                    amount,
                } => {
                    if type_filter == "CARDNAME" {
                        let owner = game.card(card_id).owner;
                        let lki_p1p1 = *game
                            .card(card_id)
                            .counters
                            .get(&crate::card::CounterType::P1P1)
                            .unwrap_or(&0);
                        self.trigger_handler.run_trigger(
                            TriggerType::Sacrificed,
                            RunParams {
                                card: Some(card_id),
                                player: Some(player),
                                ..Default::default()
                            },
                            false,
                        );
                        // Clear temporary Animate triggers BEFORE emitting
                        // ChangesZone so they are not pre-matched by flush.
                        // Per CR 400.7 the dying card becomes a new object.
                        {
                            let card = game.card_mut(card_id);
                            let pt = card.pump_trigger_count;
                            if pt > 0 {
                                let new_len = card.triggers.len().saturating_sub(pt);
                                card.triggers.truncate(new_len);
                                card.pump_trigger_count = 0;
                            }
                        }
                        crate::ability::effects::emit_zone_trigger_with_lki_counters(
                            &mut self.trigger_handler,
                            card_id,
                            ZoneType::Battlefield,
                            ZoneType::Graveyard,
                            lki_p1p1,
                        );
                        self.trigger_handler.flush_waiting_triggers(game);
                        game.move_card(card_id, ZoneType::Graveyard, owner);
                    } else {
                        self.pay_sacrifice_cost(game, agents, player, type_filter, *amount);
                    }
                }
                CostPart::Discard {
                    type_filter,
                    amount,
                } => {
                    if type_filter == "CARDNAME" {
                        let owner = game.card(card_id).owner;
                        self.trigger_handler.run_trigger(
                            TriggerType::Discarded,
                            RunParams {
                                card: Some(card_id),
                                player: Some(player),
                                ..Default::default()
                            },
                            false,
                        );
                        game.move_card(card_id, ZoneType::Graveyard, owner);
                    } else if !pre_picked_discards.is_empty() {
                        // Use pre-picked cards from visit phase
                        let to_discard: Vec<CardId> = pre_picked_discards
                            .drain(..(*amount as usize).min(pre_picked_discards.len()))
                            .collect();
                        for cid in to_discard {
                            crate::ability::effects::helpers::discard_with_madness_replacement(
                                game,
                                &mut self.trigger_handler,
                                cid,
                                player,
                            );
                            // Store discarded card for SVar evaluation
                            game.card_mut(card_id).remembered_cards.push(cid);
                        }
                    } else {
                        self.pay_discard_cost(game, agents, player, card_id, type_filter, *amount);
                    }
                }
                CostPart::ExileFromAnyGrave {
                    amount,
                    type_filter,
                } => {
                    self.pay_exile_from_any_grave_cost(game, agents, player, type_filter, *amount);
                }
                CostPart::ExileFromSameGrave {
                    amount,
                    type_filter,
                } => {
                    self.pay_exile_from_same_grave_cost(game, agents, player, type_filter, *amount);
                }
                CostPart::SubCounter {
                    amount,
                    counter_type,
                } => {
                    game.card_mut(card_id).remove_counter(counter_type, *amount);
                }
                CostPart::AddCounter {
                    amount,
                    counter_type,
                } => {
                    game.card_mut(card_id).add_counter(counter_type, *amount);
                }
                CostPart::Exile {
                    amount,
                    type_filter,
                    from,
                } => {
                    if type_filter == "CARDNAME" || type_filter == "OriginalHost" {
                        game.move_card(card_id, ZoneType::Exile, game.card(card_id).owner);
                    } else {
                        self.pay_exile_cost(
                            game,
                            agents,
                            player,
                            card_id,
                            type_filter,
                            *amount,
                            *from,
                        );
                    }
                }
                CostPart::Return {
                    amount,
                    type_filter,
                } => {
                    if type_filter == "CARDNAME" {
                        let owner = game.card(card_id).owner;
                        game.move_card(card_id, ZoneType::Hand, owner);
                    } else {
                        self.pay_return_cost(game, agents, player, type_filter, *amount);
                    }
                }
                CostPart::TapType {
                    amount,
                    type_filter,
                    min_total_power,
                } => {
                    self.pay_tap_type_cost(
                        game,
                        agents,
                        player,
                        card_id,
                        type_filter,
                        *amount,
                        *min_total_power,
                    );
                }
                CostPart::UntapType {
                    amount,
                    type_filter,
                } => {
                    self.pay_untap_type_cost(game, agents, player, card_id, type_filter, *amount);
                }
                CostPart::PayEnergy(amount) => {
                    let resolved_amount =
                        crate::cost::resolve_dynamic_amount(game, card_id, player, *amount);
                    game.player_mut(player).energy_counters -= resolved_amount;
                }
                CostPart::PayShards(amount) => {
                    let resolved_amount =
                        crate::cost::resolve_dynamic_amount(game, card_id, player, *amount);
                    game.player_mut(player).mana_shards -= resolved_amount;
                }
                CostPart::DamageYou(amount) => {
                    // Java CostDamage calls game.getAction().dealDamage() — use the
                    // same path so damage prevention, replacement effects, and
                    // DamageDone triggers all fire correctly.
                    game.deal_damage_to_player(player, *amount);
                    self.trigger_handler.run_trigger(
                        TriggerType::DamageDone,
                        RunParams {
                            damage_target_player: Some(player),
                            damage_amount: Some(*amount),
                            is_combat_damage: Some(false),
                            ..Default::default()
                        },
                        false,
                    );
                }
                CostPart::Draw(amount) => {
                    for _ in 0..*amount {
                        game.draw_card(player);
                    }
                }
                CostPart::Mill(amount) => {
                    for _ in 0..*amount {
                        if let Some(top) = game.zone_mut(ZoneType::Library, player).take_top() {
                            game.move_card(top, ZoneType::Graveyard, player);
                            self.trigger_handler.run_trigger(
                                TriggerType::Milled,
                                RunParams {
                                    card: Some(top),
                                    player: Some(player),
                                    ..Default::default()
                                },
                                false,
                            );
                            crate::ability::effects::emit_zone_trigger(
                                &mut self.trigger_handler,
                                top,
                                ZoneType::Library,
                                ZoneType::Graveyard,
                            );
                        }
                    }
                }
                CostPart::Reveal {
                    amount,
                    type_filter,
                    from,
                } => {
                    let resolved_amount =
                        crate::cost::resolve_dynamic_amount(game, card_id, player, *amount);
                    self.pay_reveal_cost(
                        game,
                        agents,
                        player,
                        card_id,
                        type_filter,
                        resolved_amount,
                        from,
                    );
                }
                CostPart::Exert {
                    amount,
                    type_filter,
                } => {
                    let resolved_amount =
                        crate::cost::resolve_dynamic_amount(game, card_id, player, *amount);
                    self.pay_exert_cost(
                        game,
                        agents,
                        player,
                        card_id,
                        type_filter,
                        resolved_amount,
                    );
                }
                CostPart::GainLife(amount) => {
                    // Opponent gains life
                    let opponent = game.opponent_of(player);
                    game.player_mut(opponent).gain_life(*amount);
                }
                CostPart::GainControl {
                    amount,
                    type_filter,
                } => {
                    self.pay_gain_control_cost(game, agents, player, type_filter, *amount);
                }
                CostPart::RemoveAnyCounter {
                    amount,
                    type_filter,
                    counter_type,
                } => {
                    self.pay_remove_any_counter_cost(
                        game,
                        agents,
                        player,
                        type_filter,
                        *amount,
                        counter_type.as_ref(),
                    );
                }
                CostPart::Unattach => {
                    // Detach the source equipment from whatever it is equipping
                    game.detach(card_id);
                    self.trigger_handler.run_trigger(
                        TriggerType::Unattached,
                        RunParams {
                            card: Some(card_id),
                            player: Some(player),
                            ..Default::default()
                        },
                        false,
                    );
                }
                CostPart::ExiledMoveToGrave {
                    amount,
                    type_filter,
                } => {
                    self.pay_exiled_move_to_grave_cost(game, agents, player, type_filter, *amount);
                }
                CostPart::AddMana { amount, mana_type } => {
                    use forge_foundation::mana::ManaAtom;
                    let atom = match mana_type.to_uppercase().as_str() {
                        "W" | "WHITE" => ManaAtom::WHITE,
                        "U" | "BLUE" => ManaAtom::BLUE,
                        "B" | "BLACK" => ManaAtom::BLACK,
                        "R" | "RED" => ManaAtom::RED,
                        "G" | "GREEN" => ManaAtom::GREEN,
                        "C" | "COLORLESS" => ManaAtom::COLORLESS,
                        _ => ManaAtom::COLORLESS,
                    };
                    for _ in 0..*amount {
                        let mut m = crate::mana::Mana::simple(atom);
                        m.source_card = Some(card_id);
                        self.mana_pools[player.index()].add_mana(m);
                    }
                }
                CostPart::Waterbend { amount } => {
                    self.pay_waterbend_cost(game, agents, player, card_id, *amount);
                }
                CostPart::ChooseColor(amount) => {
                    let resolved_amount =
                        crate::cost::resolve_dynamic_amount(game, card_id, player, *amount);
                    let valid_colors = vec![
                        "White".to_string(),
                        "Blue".to_string(),
                        "Black".to_string(),
                        "Red".to_string(),
                        "Green".to_string(),
                    ];
                    game.card_mut(card_id).chosen_colors.clear();
                    for _ in 0..resolved_amount {
                        if let Some(color) =
                            agents[player.index()].choose_color(player, &valid_colors)
                        {
                            game.card_mut(card_id).chosen_colors.push(color);
                        }
                    }
                }
                CostPart::ChooseCreatureType(_) => {
                    let valid_types = crate::game::TypeRegistry::creature_types().to_vec();
                    if let Some(chosen) =
                        agents[player.index()].choose_type(player, "Creature", &valid_types)
                    {
                        let source = game.card_mut(card_id);
                        source.chosen_type = Some(chosen);
                        source.chosen_type_controller = Some(player);
                        source.chosen_type_revealed = false;
                    }
                }
                CostPart::FlipCoin(amount) => {
                    let resolved_amount =
                        crate::cost::resolve_dynamic_amount(game, card_id, player, *amount);
                    for _ in 0..resolved_amount {
                        let source_name = game.card(card_id).card_name.clone();
                        let called_heads = agents[player.index()].choose_binary(
                            player,
                            "Call the coin flip",
                            crate::agent::BinaryChoiceKind::HeadsOrTails,
                            None,
                            Some(&source_name),
                            None,
                        );
                        let is_heads = self.game_rng.next_int(2) == 0;
                        let won = called_heads == is_heads;
                        self.trigger_handler.run_trigger(
                            TriggerType::FlippedCoin,
                            RunParams {
                                player: Some(player),
                                coin_flip_won: Some(won),
                                ..Default::default()
                            },
                            false,
                        );
                    }
                }
                CostPart::RollDice {
                    amount,
                    sides,
                    result_svar,
                } => {
                    let resolved_amount =
                        crate::cost::resolve_dynamic_amount(game, card_id, player, *amount);
                    let mut last_result = 0;
                    for _ in 0..resolved_amount {
                        last_result = self.game_rng.next_int(*sides) + 1;
                        self.trigger_handler.run_trigger(
                            TriggerType::RolledDie,
                            RunParams {
                                player: Some(player),
                                die_result: Some(last_result),
                                die_sides: Some(*sides),
                                ..Default::default()
                            },
                            false,
                        );
                    }
                    game.card_mut(card_id)
                        .svars
                        .insert(result_svar.clone(), last_result.to_string());
                    self.trigger_handler.run_trigger(
                        TriggerType::RolledDieOnce,
                        RunParams {
                            player: Some(player),
                            die_result: Some(last_result),
                            die_sides: Some(*sides),
                            ..Default::default()
                        },
                        false,
                    );
                }
                CostPart::ExileFromStack {
                    amount,
                    type_filter,
                } => {
                    let resolved_amount =
                        crate::cost::resolve_dynamic_amount(game, card_id, player, *amount);
                    self.pay_exile_from_stack_cost(
                        game,
                        agents,
                        player,
                        type_filter,
                        resolved_amount,
                    );
                }
                CostPart::CollectEvidence(amount) => {
                    let resolved_amount =
                        crate::cost::resolve_dynamic_amount(game, card_id, player, *amount);
                    if !self.pay_collect_evidence_cost(game, agents, player, resolved_amount) {
                        payment_ok = false;
                        break;
                    }
                }
                CostPart::Forage => {
                    self.pay_forage_cost(game, agents, player);
                }
                CostPart::PutCardToLib {
                    amount,
                    lib_pos,
                    type_filter,
                    from,
                    same_zone,
                } => {
                    let resolved_amount =
                        crate::cost::resolve_dynamic_amount(game, card_id, player, *amount);
                    if !self.pay_put_card_to_lib_cost(
                        game,
                        agents,
                        player,
                        card_id,
                        type_filter,
                        resolved_amount,
                        *lib_pos,
                        *from,
                        *same_zone,
                    ) {
                        payment_ok = false;
                        break;
                    }
                }
                CostPart::Enlist {
                    amount,
                    type_filter,
                } => {
                    let resolved_amount =
                        crate::cost::resolve_dynamic_amount(game, card_id, player, *amount);
                    self.pay_enlist_cost(
                        game,
                        agents,
                        player,
                        card_id,
                        type_filter,
                        resolved_amount,
                    );
                }
                CostPart::PromiseGift => {
                    let opps: Vec<_> = game
                        .alive_players()
                        .into_iter()
                        .filter(|&pid| pid != player)
                        .collect();
                    let chosen = agents[player.index()].choose_target_player(player, &opps);
                    game.card_mut(card_id).promised_gift = chosen;
                }
                CostPart::RevealChosen { reveal_type } => {
                    let source = game.card_mut(card_id);
                    if reveal_type.eq_ignore_ascii_case("Player") {
                        source.chosen_player_revealed = true;
                    } else if reveal_type.eq_ignore_ascii_case("Type") {
                        source.chosen_type_revealed = true;
                    }
                }
                CostPart::Behold {
                    amount,
                    type_filter,
                    exile,
                } => {
                    let resolved_amount =
                        crate::cost::resolve_dynamic_amount(game, card_id, player, *amount);
                    self.pay_behold_cost(
                        game,
                        agents,
                        player,
                        card_id,
                        type_filter,
                        resolved_amount,
                        *exile,
                    );
                }
                CostPart::Blight(amount) => {
                    let resolved_amount =
                        crate::cost::resolve_dynamic_amount(game, card_id, player, *amount);
                    self.pay_blight_cost(game, agents, player, resolved_amount);
                }
                CostPart::ExileCtrlOrGrave {
                    amount,
                    type_filter,
                } => {
                    let resolved_amount =
                        crate::cost::resolve_dynamic_amount(game, card_id, player, *amount);
                    self.pay_exile_ctrl_or_grave_cost(
                        game,
                        agents,
                        player,
                        type_filter,
                        resolved_amount,
                    );
                }
            }
        }
        if !payment_ok {
            self.restore_snapshot(game, &payment_snapshot);
            return false;
        }
        true
    }

    /// Pay additional costs from an SP$ ability line (non-mana cost parts only).
    /// Used during spell casting for costs like `Sac<1/Creature>`.
    /// Mirrors Java's `CostPayment.payCost()` for spell abilities.
    pub(crate) fn pay_additional_costs(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        card_id: CardId,
        spell_cost: &crate::cost::Cost,
        _api: Option<&str>,
        _mandatory: bool,
    ) -> bool {
        let payment_snapshot = self.make_snapshot(game, true);
        let mut payment_ok = true;
        for part in spell_cost.parts.clone() {
            // Java deterministic parity does not route confirm-payment prompts
            // through RNG while paying spell costs; confirmPayment() returns true
            // for spell payment context. Keep Rust aligned to avoid decision-RNG
            // drift when spell-cost prompt timing differs between engines.
            match &part {
                // Mana is already paid by play_card's main mana payment flow.
                // Tap/Untap are not applicable to spell additional costs.
                CostPart::Mana(_) | CostPart::Tap | CostPart::Untap => {}
                CostPart::PayLife(amount) => {
                    self.pay_life_cost(game, player, *amount);
                }
                CostPart::Sacrifice {
                    type_filter,
                    amount,
                } => {
                    if type_filter != "CARDNAME" {
                        self.pay_sacrifice_cost(game, agents, player, type_filter, *amount);
                    }
                }
                CostPart::Discard {
                    type_filter,
                    amount,
                } => {
                    if type_filter != "CARDNAME" {
                        let discarded = self.pay_discard_cost(
                            game,
                            agents,
                            player,
                            card_id,
                            type_filter,
                            *amount,
                        );
                        // Store discarded cards on the source card for SVar evaluation
                        // (e.g. Grab the Prize: X = Discarded$Valid Card.nonLand/Times.2)
                        game.card_mut(card_id).remembered_cards.extend(discarded);
                    }
                }
                CostPart::ExileFromAnyGrave {
                    amount,
                    type_filter,
                } => {
                    self.pay_exile_from_any_grave_cost(game, agents, player, type_filter, *amount);
                }
                CostPart::ExileFromSameGrave {
                    amount,
                    type_filter,
                } => {
                    self.pay_exile_from_same_grave_cost(game, agents, player, type_filter, *amount);
                }
                CostPart::SubCounter {
                    amount,
                    counter_type,
                } => {
                    if game.card(card_id).zone == ZoneType::Battlefield {
                        game.card_mut(card_id).remove_counter(counter_type, *amount);
                    }
                }
                CostPart::AddCounter {
                    amount,
                    counter_type,
                } => {
                    if game.card(card_id).zone == ZoneType::Battlefield {
                        game.card_mut(card_id).add_counter(counter_type, *amount);
                    }
                }
                CostPart::Exile {
                    amount,
                    type_filter,
                    from,
                } => {
                    if type_filter == "CARDNAME" || type_filter == "OriginalHost" {
                        if game.card(card_id).zone == *from {
                            let owner = game.card(card_id).owner;
                            game.move_card(card_id, ZoneType::Exile, owner);
                        }
                    } else {
                        self.pay_exile_cost(
                            game,
                            agents,
                            player,
                            card_id,
                            type_filter,
                            *amount,
                            *from,
                        );
                    }
                }
                CostPart::Return {
                    amount,
                    type_filter,
                } => {
                    if type_filter != "CARDNAME" {
                        self.pay_return_cost(game, agents, player, type_filter, *amount);
                    }
                }
                CostPart::TapType {
                    amount,
                    type_filter,
                    min_total_power,
                } => {
                    self.pay_tap_type_cost(
                        game,
                        agents,
                        player,
                        card_id,
                        type_filter,
                        *amount,
                        *min_total_power,
                    );
                }
                CostPart::UntapType {
                    amount,
                    type_filter,
                } => {
                    self.pay_untap_type_cost(game, agents, player, card_id, type_filter, *amount);
                }
                CostPart::PayEnergy(amount) => {
                    let resolved_amount =
                        crate::cost::resolve_dynamic_amount(game, card_id, player, *amount);
                    game.player_mut(player).energy_counters -= resolved_amount;
                }
                CostPart::PayShards(amount) => {
                    let resolved_amount =
                        crate::cost::resolve_dynamic_amount(game, card_id, player, *amount);
                    game.player_mut(player).mana_shards -= resolved_amount;
                }
                CostPart::DamageYou(amount) => {
                    // Java CostDamage calls game.getAction().dealDamage() — use the
                    // same path so damage prevention, replacement effects, and
                    // DamageDone triggers all fire correctly.
                    game.deal_damage_to_player(player, *amount);
                    self.trigger_handler.run_trigger(
                        TriggerType::DamageDone,
                        RunParams {
                            damage_target_player: Some(player),
                            damage_amount: Some(*amount),
                            is_combat_damage: Some(false),
                            ..Default::default()
                        },
                        false,
                    );
                }
                CostPart::Draw(amount) => {
                    for _ in 0..*amount {
                        game.draw_card(player);
                    }
                }
                CostPart::Mill(amount) => {
                    for _ in 0..*amount {
                        if let Some(top) = game.zone_mut(ZoneType::Library, player).take_top() {
                            game.move_card(top, ZoneType::Graveyard, player);
                            self.trigger_handler.run_trigger(
                                TriggerType::Milled,
                                RunParams {
                                    card: Some(top),
                                    player: Some(player),
                                    ..Default::default()
                                },
                                false,
                            );
                            crate::ability::effects::emit_zone_trigger(
                                &mut self.trigger_handler,
                                top,
                                ZoneType::Library,
                                ZoneType::Graveyard,
                            );
                        }
                    }
                }
                CostPart::Reveal {
                    amount,
                    type_filter,
                    from,
                } => {
                    let resolved_amount =
                        crate::cost::resolve_dynamic_amount(game, card_id, player, *amount);
                    self.pay_reveal_cost(
                        game,
                        agents,
                        player,
                        card_id,
                        type_filter,
                        resolved_amount,
                        from,
                    );
                }
                CostPart::Exert {
                    amount,
                    type_filter,
                } => {
                    let resolved_amount =
                        crate::cost::resolve_dynamic_amount(game, card_id, player, *amount);
                    self.pay_exert_cost(
                        game,
                        agents,
                        player,
                        card_id,
                        type_filter,
                        resolved_amount,
                    );
                }
                CostPart::GainLife(amount) => {
                    let opponent = game.opponent_of(player);
                    game.player_mut(opponent).gain_life(*amount);
                }
                CostPart::GainControl {
                    amount,
                    type_filter,
                } => {
                    self.pay_gain_control_cost(game, agents, player, type_filter, *amount);
                }
                CostPart::RemoveAnyCounter {
                    amount,
                    type_filter,
                    counter_type,
                } => {
                    self.pay_remove_any_counter_cost(
                        game,
                        agents,
                        player,
                        type_filter,
                        *amount,
                        counter_type.as_ref(),
                    );
                }
                CostPart::Unattach => {
                    game.detach(card_id);
                    self.trigger_handler.run_trigger(
                        TriggerType::Unattached,
                        RunParams {
                            card: Some(card_id),
                            player: Some(player),
                            ..Default::default()
                        },
                        false,
                    );
                }
                CostPart::ExiledMoveToGrave {
                    amount,
                    type_filter,
                } => {
                    self.pay_exiled_move_to_grave_cost(game, agents, player, type_filter, *amount);
                }
                CostPart::AddMana { amount, mana_type } => {
                    use forge_foundation::mana::ManaAtom;
                    let atom = match mana_type.to_uppercase().as_str() {
                        "W" | "WHITE" => ManaAtom::WHITE,
                        "U" | "BLUE" => ManaAtom::BLUE,
                        "B" | "BLACK" => ManaAtom::BLACK,
                        "R" | "RED" => ManaAtom::RED,
                        "G" | "GREEN" => ManaAtom::GREEN,
                        "C" | "COLORLESS" => ManaAtom::COLORLESS,
                        _ => ManaAtom::COLORLESS,
                    };
                    for _ in 0..*amount {
                        let mut m = crate::mana::Mana::simple(atom);
                        m.source_card = Some(card_id);
                        self.mana_pools[player.index()].add_mana(m);
                    }
                }
                CostPart::Waterbend { amount } => {
                    self.pay_waterbend_cost(game, agents, player, card_id, *amount);
                }
                CostPart::ChooseColor(amount) => {
                    let resolved_amount =
                        crate::cost::resolve_dynamic_amount(game, card_id, player, *amount);
                    let valid_colors = vec![
                        "White".to_string(),
                        "Blue".to_string(),
                        "Black".to_string(),
                        "Red".to_string(),
                        "Green".to_string(),
                    ];
                    game.card_mut(card_id).chosen_colors.clear();
                    for _ in 0..resolved_amount {
                        if let Some(color) =
                            agents[player.index()].choose_color(player, &valid_colors)
                        {
                            game.card_mut(card_id).chosen_colors.push(color);
                        }
                    }
                }
                CostPart::ChooseCreatureType(_) => {
                    let valid_types = crate::game::TypeRegistry::creature_types().to_vec();
                    if let Some(chosen) =
                        agents[player.index()].choose_type(player, "Creature", &valid_types)
                    {
                        let source = game.card_mut(card_id);
                        source.chosen_type = Some(chosen);
                        source.chosen_type_controller = Some(player);
                        source.chosen_type_revealed = false;
                    }
                }
                CostPart::FlipCoin(amount) => {
                    let resolved_amount =
                        crate::cost::resolve_dynamic_amount(game, card_id, player, *amount);
                    for _ in 0..resolved_amount {
                        let source_name = game.card(card_id).card_name.clone();
                        let called_heads = agents[player.index()].choose_binary(
                            player,
                            "Call the coin flip",
                            crate::agent::BinaryChoiceKind::HeadsOrTails,
                            None,
                            Some(&source_name),
                            None,
                        );
                        let is_heads = self.game_rng.next_int(2) == 0;
                        let won = called_heads == is_heads;
                        self.trigger_handler.run_trigger(
                            TriggerType::FlippedCoin,
                            RunParams {
                                player: Some(player),
                                coin_flip_won: Some(won),
                                ..Default::default()
                            },
                            false,
                        );
                    }
                }
                CostPart::RollDice {
                    amount,
                    sides,
                    result_svar,
                } => {
                    let resolved_amount =
                        crate::cost::resolve_dynamic_amount(game, card_id, player, *amount);
                    let mut last_result = 0;
                    for _ in 0..resolved_amount {
                        last_result = self.game_rng.next_int(*sides) + 1;
                        self.trigger_handler.run_trigger(
                            TriggerType::RolledDie,
                            RunParams {
                                player: Some(player),
                                die_result: Some(last_result),
                                die_sides: Some(*sides),
                                ..Default::default()
                            },
                            false,
                        );
                    }
                    game.card_mut(card_id)
                        .svars
                        .insert(result_svar.clone(), last_result.to_string());
                    self.trigger_handler.run_trigger(
                        TriggerType::RolledDieOnce,
                        RunParams {
                            player: Some(player),
                            die_result: Some(last_result),
                            die_sides: Some(*sides),
                            ..Default::default()
                        },
                        false,
                    );
                }
                CostPart::ExileFromStack {
                    amount,
                    type_filter,
                } => {
                    let resolved_amount =
                        crate::cost::resolve_dynamic_amount(game, card_id, player, *amount);
                    self.pay_exile_from_stack_cost(
                        game,
                        agents,
                        player,
                        type_filter,
                        resolved_amount,
                    );
                }
                CostPart::CollectEvidence(amount) => {
                    let resolved_amount =
                        crate::cost::resolve_dynamic_amount(game, card_id, player, *amount);
                    if !self.pay_collect_evidence_cost(game, agents, player, resolved_amount) {
                        payment_ok = false;
                        break;
                    }
                }
                CostPart::Forage => {
                    self.pay_forage_cost(game, agents, player);
                }
                CostPart::PutCardToLib {
                    amount,
                    lib_pos,
                    type_filter,
                    from,
                    same_zone,
                } => {
                    let resolved_amount =
                        crate::cost::resolve_dynamic_amount(game, card_id, player, *amount);
                    if !self.pay_put_card_to_lib_cost(
                        game,
                        agents,
                        player,
                        card_id,
                        type_filter,
                        resolved_amount,
                        *lib_pos,
                        *from,
                        *same_zone,
                    ) {
                        payment_ok = false;
                        break;
                    }
                }
                CostPart::Enlist {
                    amount,
                    type_filter,
                } => {
                    let resolved_amount =
                        crate::cost::resolve_dynamic_amount(game, card_id, player, *amount);
                    self.pay_enlist_cost(
                        game,
                        agents,
                        player,
                        card_id,
                        type_filter,
                        resolved_amount,
                    );
                }
                CostPart::PromiseGift => {
                    let opps: Vec<_> = game
                        .alive_players()
                        .into_iter()
                        .filter(|&pid| pid != player)
                        .collect();
                    let chosen = agents[player.index()].choose_target_player(player, &opps);
                    game.card_mut(card_id).promised_gift = chosen;
                }
                CostPart::RevealChosen { reveal_type } => {
                    let source = game.card_mut(card_id);
                    if reveal_type.eq_ignore_ascii_case("Player") {
                        source.chosen_player_revealed = true;
                    } else if reveal_type.eq_ignore_ascii_case("Type") {
                        source.chosen_type_revealed = true;
                    }
                }
                CostPart::Behold {
                    amount,
                    type_filter,
                    exile,
                } => {
                    let resolved_amount =
                        crate::cost::resolve_dynamic_amount(game, card_id, player, *amount);
                    self.pay_behold_cost(
                        game,
                        agents,
                        player,
                        card_id,
                        type_filter,
                        resolved_amount,
                        *exile,
                    );
                }
                CostPart::Blight(amount) => {
                    let resolved_amount =
                        crate::cost::resolve_dynamic_amount(game, card_id, player, *amount);
                    self.pay_blight_cost(game, agents, player, resolved_amount);
                }
                CostPart::ExileCtrlOrGrave {
                    amount,
                    type_filter,
                } => {
                    let resolved_amount =
                        crate::cost::resolve_dynamic_amount(game, card_id, player, *amount);
                    self.pay_exile_ctrl_or_grave_cost(
                        game,
                        agents,
                        player,
                        type_filter,
                        resolved_amount,
                    );
                }
            }
        }
        if !payment_ok {
            self.restore_snapshot(game, &payment_snapshot);
            return false;
        }
        true
    }

    /// Pay a Waterbend cost: pay `amount` generic mana, but the player can tap
    /// artifacts and/or creatures to help (each tapped pays {1}).
    /// Mirrors Java's CostWaterbend + adjustCostByWaterbend.
    pub(crate) fn pay_waterbend_cost(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        card_id: CardId,
        amount: i32,
    ) {
        if amount <= 0 {
            return;
        }
        // Gather tappable artifacts/creatures (excluding the source card)
        let untapped: Vec<CardId> = game
            .cards_in_zone(ZoneType::Battlefield, player)
            .iter()
            .filter(|&&cid| {
                let c = game.card(cid);
                !c.tapped && cid != card_id && (c.is_creature() || c.type_line.is_artifact())
            })
            .copied()
            .collect();

        let mut remaining = amount;

        if !untapped.is_empty() && remaining > 0 {
            // Reuse the convoke agent method — waterbend is convoke+improvise combined
            let card_name = game.card(card_id).card_name.clone();
            let generic_cost = forge_foundation::ManaCost::generic(remaining);
            agents[player.index()].snapshot_state(game, &self.mana_pools);
            let to_tap = agents[player.index()].choose_convoke(
                player,
                &untapped,
                &generic_cost,
                Some(&card_name),
            );
            let max_tap = remaining as usize;
            let mut count = 0usize;
            for &cid in &to_tap {
                if count >= max_tap {
                    break;
                }
                if !untapped.contains(&cid) {
                    continue;
                }
                game.tap(cid);
                remaining -= 1;
                count += 1;
            }
        }

        // Pay remaining from mana pool as generic
        if remaining > 0 {
            self.mana_pools[player.index()].pay_generic(remaining);
        }
    }

    /// Exile `amount` cards from ANY player's graveyard matching `type_filter`.
    /// Mirrors Java's CostExile with zoneMode=-1 (ExileAnyGrave).
    pub(crate) fn pay_exile_from_any_grave_cost(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        type_filter: &str,
        amount: i32,
    ) {
        let base_filter = crate::cost::normalize_exile_base_filter(type_filter);
        for _ in 0..amount {
            let valid: Vec<CardId> = game
                .players
                .iter()
                .map(|p| p.id)
                .flat_map(|pid| game.cards_in_zone(ZoneType::Graveyard, pid).to_vec())
                .filter(|&cid| {
                    (base_filter == "Card"
                        || base_filter.is_empty()
                        || crate::ability::effects::matches_change_type(
                            game.card(cid),
                            &base_filter,
                            &[],
                        ))
                        && can_exile_for_cost(game, cid)
                })
                .collect();
            if valid.is_empty() {
                break;
            }
            if let Some(chosen) = agents[player.index()].choose_sacrifice(player, &valid) {
                let owner = game.card(chosen).owner;
                game.move_card(chosen, ZoneType::Exile, owner);
                crate::ability::effects::emit_zone_trigger(
                    &mut self.trigger_handler,
                    chosen,
                    ZoneType::Graveyard,
                    ZoneType::Exile,
                );
            }
        }
    }

    /// Exile `amount` cards from the same graveyard matching `type_filter`.
    /// Mirrors Java's CostExile zoneMode=0 (ExileSameGrave).
    pub(crate) fn pay_exile_from_same_grave_cost(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        type_filter: &str,
        amount: i32,
    ) {
        let base_filter = crate::cost::normalize_exile_base_filter(type_filter);
        let mut chosen_owner: Option<PlayerId> = None;
        for _ in 0..amount {
            let valid: Vec<CardId> = game
                .players
                .iter()
                .map(|p| p.id)
                .flat_map(|pid| game.cards_in_zone(ZoneType::Graveyard, pid).to_vec())
                .filter(|&cid| {
                    if let Some(owner) = chosen_owner {
                        if game.card(cid).owner != owner {
                            return false;
                        }
                    }
                    (base_filter == "Card"
                        || base_filter.is_empty()
                        || crate::ability::effects::matches_change_type(
                            game.card(cid),
                            &base_filter,
                            &[],
                        ))
                        && can_exile_for_cost(game, cid)
                })
                .collect();
            if valid.is_empty() {
                break;
            }
            if let Some(chosen) = agents[player.index()].choose_sacrifice(player, &valid) {
                chosen_owner = Some(game.card(chosen).owner);
                let owner = game.card(chosen).owner;
                game.move_card(chosen, ZoneType::Exile, owner);
                crate::ability::effects::emit_zone_trigger(
                    &mut self.trigger_handler,
                    chosen,
                    ZoneType::Graveyard,
                    ZoneType::Exile,
                );
            }
        }
    }

    /// Exile `amount` cards from `zone` matching `type_filter` for `player`.
    /// Mirrors Java's `CostExile.doListPayment()`.
    pub(crate) fn pay_exile_cost(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        source: CardId,
        type_filter: &str,
        amount: i32,
        from: ZoneType,
    ) {
        let base_filter = crate::cost::normalize_exile_base_filter(type_filter);
        for _ in 0..amount {
            let mut valid = cost::get_zone_targets(game, player, from, &base_filter);
            valid.retain(|&cid| can_exile_for_cost(game, cid));
            if from == ZoneType::Hand
                && game.card(source).zone == ZoneType::Hand
                && game.card(source).owner == player
            {
                valid.retain(|&cid| cid != source);
            }
            if valid.is_empty() {
                break;
            }
            // Use choose_sacrifice to pick target (reuse the choose interface)
            if let Some(chosen) = agents[player.index()].choose_sacrifice(player, &valid) {
                let owner = game.card(chosen).owner;
                game.move_card(chosen, ZoneType::Exile, owner);
                crate::ability::effects::emit_zone_trigger(
                    &mut self.trigger_handler,
                    chosen,
                    from,
                    ZoneType::Exile,
                );
            }
        }
    }

    /// Exile spell(s) from stack as a cost.
    /// Mirrors Java CostExileFromStack.
    pub(crate) fn pay_exile_from_stack_cost(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        type_filter: &str,
        amount: i32,
    ) {
        for _ in 0..amount {
            let valid_entries: Vec<u32> = game
                .stack
                .iter()
                .filter(|e| e.spell_ability.is_spell)
                .filter(|e| {
                    e.spell_ability.source.is_some_and(|cid| {
                        crate::cost::matches_exile_from_stack_filter(game, cid, player, type_filter)
                    })
                })
                .map(|e| e.id)
                .collect();
            if valid_entries.is_empty() {
                break;
            }
            let Some(chosen_entry) =
                agents[player.index()].choose_target_spell(player, &valid_entries)
            else {
                break;
            };
            if let Some(entry) = game.stack.remove_by_id(chosen_entry) {
                if let Some(chosen_card) = entry.spell_ability.source {
                    let owner = game.card(chosen_card).owner;
                    game.move_card(chosen_card, ZoneType::Exile, owner);
                    crate::ability::effects::emit_zone_trigger(
                        &mut self.trigger_handler,
                        chosen_card,
                        ZoneType::Stack,
                        ZoneType::Exile,
                    );
                }
            }
        }
    }

    /// Collect evidence N as a cost: exile cards from your graveyard with total MV >= N.
    pub(crate) fn pay_collect_evidence_cost(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        amount: i32,
    ) -> bool {
        if amount <= 0 {
            return true;
        }
        // Mirror Java human-style collect-evidence selection:
        // choose any number from graveyard, then require total CMC >= amount.
        let valid: Vec<CardId> = game
            .cards_in_zone(ZoneType::Graveyard, player)
            .iter()
            .copied()
            .filter(|&cid| can_exile_for_cost(game, cid))
            .collect();
        if valid.is_empty() {
            return false;
        }

        let selected =
            agents[player.index()].choose_cards_for_effect(player, &valid, 0, valid.len());
        let chosen: Vec<CardId> = selected
            .into_iter()
            .filter(|cid| valid.contains(cid))
            .collect();

        let total_mv: i32 = chosen
            .iter()
            .map(|&cid| game.card(cid).mana_cost.cmc() as i32)
            .sum();
        if total_mv < amount {
            return false;
        }

        for cid in chosen {
            let owner = game.card(cid).owner;
            game.move_card(cid, ZoneType::Exile, owner);
            crate::ability::effects::emit_zone_trigger(
                &mut self.trigger_handler,
                cid,
                ZoneType::Graveyard,
                ZoneType::Exile,
            );
        }
        self.trigger_handler.run_trigger(
            TriggerType::CollectEvidence,
            RunParams {
                player: Some(player),
                ..Default::default()
            },
            false,
        );
        true
    }

    /// Forage as a cost: exile 3 from graveyard or sacrifice a Food.
    pub(crate) fn pay_forage_cost(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
    ) {
        let battlefield_cards: Vec<_> = game
            .players
            .iter()
            .flat_map(|p| game.cards_in_zone(ZoneType::Battlefield, p.id))
            .map(|&cid| game.card(cid).clone())
            .collect();
        let foods: Vec<CardId> = game
            .cards_in_zone(ZoneType::Battlefield, player)
            .iter()
            .copied()
            .filter(|&cid| game.card(cid).type_line.has_subtype("Food"))
            .filter(|&cid| {
                !crate::staticability::static_ability_cant_sacrifice::cant_sacrifice(
                    &battlefield_cards,
                    game.card(cid),
                    None,
                    true,
                )
            })
            .collect();
        let gy: Vec<CardId> = game
            .cards_in_zone(ZoneType::Graveyard, player)
            .iter()
            .copied()
            .filter(|&cid| can_exile_for_cost(game, cid))
            .collect();

        if !foods.is_empty() && gy.len() < 3 {
            let chosen = agents[player.index()]
                .choose_sacrifice(player, &foods)
                .unwrap_or(foods[0]);
            let owner = game.card(chosen).owner;
            let lki_p1p1 = *game
                .card(chosen)
                .counters
                .get(&crate::card::CounterType::P1P1)
                .unwrap_or(&0);
            {
                let card = game.card_mut(chosen);
                let pt = card.pump_trigger_count;
                if pt > 0 {
                    let new_len = card.triggers.len().saturating_sub(pt);
                    card.triggers.truncate(new_len);
                    card.pump_trigger_count = 0;
                }
            }
            self.trigger_handler.run_trigger(
                TriggerType::Sacrificed,
                RunParams {
                    card: Some(chosen),
                    player: Some(player),
                    ..Default::default()
                },
                false,
            );
            crate::ability::effects::emit_zone_trigger_with_lki_counters(
                &mut self.trigger_handler,
                chosen,
                ZoneType::Battlefield,
                ZoneType::Graveyard,
                lki_p1p1,
            );
            self.trigger_handler.flush_waiting_triggers(game);
            game.move_card(chosen, ZoneType::Graveyard, owner);
        } else if !foods.is_empty() {
            // Let the chooser pick between food + graveyard cards. Food means sacrifice path.
            let mut combined = foods.clone();
            combined.extend(gy.iter().copied());
            if let Some(chosen) = agents[player.index()].choose_sacrifice(player, &combined) {
                if foods.contains(&chosen) {
                    let owner = game.card(chosen).owner;
                    let lki_p1p1 = *game
                        .card(chosen)
                        .counters
                        .get(&crate::card::CounterType::P1P1)
                        .unwrap_or(&0);
                    self.trigger_handler.run_trigger(
                        TriggerType::Sacrificed,
                        RunParams {
                            card: Some(chosen),
                            player: Some(player),
                            ..Default::default()
                        },
                        false,
                    );
                    {
                        let card = game.card_mut(chosen);
                        let pt = card.pump_trigger_count;
                        if pt > 0 {
                            let new_len = card.triggers.len().saturating_sub(pt);
                            card.triggers.truncate(new_len);
                            card.pump_trigger_count = 0;
                        }
                    }
                    crate::ability::effects::emit_zone_trigger_with_lki_counters(
                        &mut self.trigger_handler,
                        chosen,
                        ZoneType::Battlefield,
                        ZoneType::Graveyard,
                        lki_p1p1,
                    );
                    self.trigger_handler.flush_waiting_triggers(game);
                    game.move_card(chosen, ZoneType::Graveyard, owner);
                } else {
                    // Graveyard path: exile chosen + two more.
                    let mut chosen_gy = vec![chosen];
                    while chosen_gy.len() < 3 {
                        let remaining: Vec<CardId> = game
                            .cards_in_zone(ZoneType::Graveyard, player)
                            .iter()
                            .copied()
                            .filter(|cid| !chosen_gy.contains(cid))
                            .filter(|&cid| can_exile_for_cost(game, cid))
                            .collect();
                        if remaining.is_empty() {
                            return;
                        }
                        let next = agents[player.index()]
                            .choose_sacrifice(player, &remaining)
                            .unwrap_or(remaining[0]);
                        chosen_gy.push(next);
                    }
                    for cid in chosen_gy.into_iter().take(3) {
                        let owner = game.card(cid).owner;
                        game.move_card(cid, ZoneType::Exile, owner);
                        crate::ability::effects::emit_zone_trigger(
                            &mut self.trigger_handler,
                            cid,
                            ZoneType::Graveyard,
                            ZoneType::Exile,
                        );
                    }
                }
            }
        } else {
            for _ in 0..3 {
                let remaining: Vec<CardId> = game
                    .cards_in_zone(ZoneType::Graveyard, player)
                    .iter()
                    .copied()
                    .filter(|&cid| can_exile_for_cost(game, cid))
                    .collect();
                if remaining.is_empty() {
                    return;
                }
                let chosen = agents[player.index()]
                    .choose_sacrifice(player, &remaining)
                    .unwrap_or(remaining[0]);
                let owner = game.card(chosen).owner;
                game.move_card(chosen, ZoneType::Exile, owner);
                crate::ability::effects::emit_zone_trigger(
                    &mut self.trigger_handler,
                    chosen,
                    ZoneType::Graveyard,
                    ZoneType::Exile,
                );
            }
        }

        self.trigger_handler.run_trigger(
            TriggerType::Forage,
            RunParams {
                player: Some(player),
                ..Default::default()
            },
            false,
        );
    }

    pub(crate) fn pay_reveal_cost(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        source: CardId,
        type_filter: &str,
        amount: i32,
        from: &crate::cost::RevealFrom,
    ) {
        if amount <= 0 {
            return;
        }

        let mut candidates: Vec<CardId> = match from {
            crate::cost::RevealFrom::Hand => game.cards_in_zone(ZoneType::Hand, player).to_vec(),
            crate::cost::RevealFrom::Exile => game.cards_in_zone(ZoneType::Exile, player).to_vec(),
            crate::cost::RevealFrom::HandOrBattlefield => {
                let mut v = game.cards_in_zone(ZoneType::Hand, player).to_vec();
                v.extend(
                    game.cards_in_zone(ZoneType::Battlefield, player)
                        .iter()
                        .copied(),
                );
                v
            }
            crate::cost::RevealFrom::All => {
                let mut v = game.cards_in_zone(ZoneType::Hand, player).to_vec();
                v.extend(
                    game.cards_in_zone(ZoneType::Battlefield, player)
                        .iter()
                        .copied(),
                );
                v.extend(
                    game.cards_in_zone(ZoneType::Graveyard, player)
                        .iter()
                        .copied(),
                );
                v.extend(
                    game.cards_in_zone(ZoneType::Library, player)
                        .iter()
                        .copied(),
                );
                v.extend(game.cards_in_zone(ZoneType::Exile, player).iter().copied());
                v
            }
        };

        if matches!(
            from,
            crate::cost::RevealFrom::Hand
                | crate::cost::RevealFrom::HandOrBattlefield
                | crate::cost::RevealFrom::All
        ) && game.card(source).zone == ZoneType::Hand
        {
            candidates.retain(|&cid| cid != source);
        }

        let mut revealed: Vec<CardId> = Vec::new();
        if type_filter == "Hand" {
            revealed = candidates;
        } else if type_filter == "CARDNAME" || type_filter == "NICKNAME" {
            revealed.push(source);
        } else if type_filter == "SameColor" {
            if let Some(first) = agents[player.index()].choose_sacrifice(player, &candidates) {
                let color = game.card(first).color;
                revealed.push(first);
                while (revealed.len() as i32) < amount {
                    let valid: Vec<CardId> = candidates
                        .iter()
                        .copied()
                        .filter(|cid| !revealed.contains(cid))
                        .filter(|&cid| game.card(cid).color.shares_color_with(color))
                        .collect();
                    if valid.is_empty() {
                        break;
                    }
                    let next = agents[player.index()]
                        .choose_sacrifice(player, &valid)
                        .unwrap_or(valid[0]);
                    revealed.push(next);
                }
            }
        } else {
            candidates.retain(|&cid| {
                type_filter == "Card"
                    || type_filter.is_empty()
                    || crate::ability::effects::matches_change_type(
                        game.card(cid),
                        type_filter,
                        &[],
                    )
            });
            while (revealed.len() as i32) < amount && !candidates.is_empty() {
                let next = agents[player.index()]
                    .choose_sacrifice(player, &candidates)
                    .unwrap_or(candidates[0]);
                revealed.push(next);
                candidates.retain(|&cid| cid != next);
            }
        }

        if revealed.is_empty() {
            return;
        }

        let names = revealed
            .iter()
            .map(|&cid| game.card(cid).card_name.clone())
            .collect::<Vec<_>>()
            .join(", ");
        crate::agent::notify_all_agents(
            agents,
            crate::agent::GameLogEvent::action(format!(
                "{} reveals {}",
                game.player(player).name,
                names
            ))
            .with_player(player),
        );
    }

    /// Put selected cards to top/bottom of library as a cost.
    pub(crate) fn pay_put_card_to_lib_cost(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        source: CardId,
        type_filter: &str,
        amount: i32,
        lib_pos: i32,
        from: ZoneType,
        same_zone: bool,
    ) -> bool {
        if type_filter == "CARDNAME" || type_filter == "NICKNAME" {
            // Self payment path: move the source card itself if it's in the expected zone.
            if game.card(source).zone == from {
                let owner = game.card(source).owner;
                if lib_pos == 0 {
                    game.move_card(source, ZoneType::Library, owner);
                } else {
                    game.put_on_bottom_of_library(source, owner);
                }
                return true;
            }
            return false;
        }
        let mut chosen_controller: Option<PlayerId> = None;
        let mut paid = 0i32;
        for _ in 0..amount {
            let valid: Vec<CardId> = if same_zone {
                game.players
                    .iter()
                    .flat_map(|p| game.cards_in_zone(from, p.id).to_vec())
                    .filter(|&cid| {
                        if let Some(ctrl) = chosen_controller {
                            if game.card(cid).controller != ctrl {
                                return false;
                            }
                        }
                        type_filter == "Card"
                            || type_filter.is_empty()
                            || crate::ability::effects::matches_change_type(
                                game.card(cid),
                                type_filter,
                                &[],
                            )
                    })
                    .collect()
            } else {
                crate::cost::get_zone_targets(game, player, from, type_filter)
            };
            if valid.is_empty() {
                return false;
            }
            let Some(chosen) = agents[player.index()].choose_sacrifice(player, &valid) else {
                return false;
            };
            if same_zone {
                chosen_controller = Some(game.card(chosen).controller);
            }
            let origin = game.card(chosen).zone;
            let owner = game.card(chosen).owner;
            if lib_pos == 0 {
                game.move_card(chosen, ZoneType::Library, owner);
            } else {
                game.put_on_bottom_of_library(chosen, owner);
            }
            crate::ability::effects::emit_zone_trigger(
                &mut self.trigger_handler,
                chosen,
                origin,
                ZoneType::Library,
            );
            paid += 1;
        }
        paid == amount
    }

    pub(crate) fn pay_exert_cost(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        source: CardId,
        type_filter: &str,
        amount: i32,
    ) {
        if amount <= 0 {
            return;
        }
        if type_filter == "CARDNAME" || type_filter == "NICKNAME" {
            if game.card(source).zone == ZoneType::Battlefield {
                game.card_mut(source).exerted = true;
                self.trigger_handler.run_trigger(
                    TriggerType::Exerted,
                    RunParams {
                        card: Some(source),
                        player: Some(player),
                        ..Default::default()
                    },
                    false,
                );
            }
            return;
        }
        for _ in 0..amount {
            let valid: Vec<CardId> = game
                .cards_in_zone(ZoneType::Battlefield, player)
                .iter()
                .copied()
                .filter(|&cid| {
                    crate::ability::effects::matches_change_type(game.card(cid), type_filter, &[])
                })
                .collect();
            if valid.is_empty() {
                break;
            }
            let Some(chosen) = agents[player.index()].choose_sacrifice(player, &valid) else {
                break;
            };
            game.card_mut(chosen).exerted = true;
            self.trigger_handler.run_trigger(
                TriggerType::Exerted,
                RunParams {
                    card: Some(chosen),
                    player: Some(player),
                    ..Default::default()
                },
                false,
            );
        }
    }

    /// Enlist as a cost.
    pub(crate) fn pay_enlist_cost(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        source: CardId,
        type_filter: &str,
        amount: i32,
    ) {
        for _ in 0..amount {
            let valid: Vec<CardId> = crate::cost::get_enlist_targets(game, player)
                .into_iter()
                .filter(|&cid| {
                    type_filter.eq_ignore_ascii_case("Creature")
                        || type_filter.is_empty()
                        || crate::ability::effects::matches_change_type(
                            game.card(cid),
                            type_filter,
                            &[],
                        )
                })
                .collect();
            if valid.is_empty() {
                break;
            }
            let Some(chosen) = agents[player.index()].choose_sacrifice(player, &valid) else {
                break;
            };
            let enlisted_power = game.card(chosen).power();
            game.tap(chosen);
            game.card_mut(source).enlisted_this_combat = true;
            // Enlist rule text: add enlisted creature's power to attacker until end of turn.
            // Temporary power modifiers are cleared in cleanup.
            game.card_mut(source).power_modifier += enlisted_power;
            self.trigger_handler.run_trigger(
                TriggerType::TapAll,
                RunParams {
                    card: Some(chosen),
                    player: Some(player),
                    ..Default::default()
                },
                false,
            );
            self.trigger_handler.run_trigger(
                TriggerType::Enlisted,
                RunParams {
                    card: Some(source),
                    enlisted: Some(chosen),
                    player: Some(player),
                    ..Default::default()
                },
                false,
            );
        }
    }

    /// Behold as a cost (optionally exile revealed cards).
    ///
    /// Mirrors Java's `DeterministicCostPlumbing.visit(CostBehold)` which uses
    /// `chooseCardsForEffect` (not `choose_sacrifice`). For ChosenType filters,
    /// Java does two separate calls: pick 1 first, filter by shared creature
    /// type, then pick `amount` from the filtered set.
    pub(crate) fn pay_behold_cost(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        source: CardId,
        type_filter: &str,
        amount: i32,
        exile: bool,
    ) {
        let build_pool = |game: &GameState| -> Vec<CardId> {
            let mut valid: Vec<CardId> = game
                .cards_in_zone(ZoneType::Hand, player)
                .iter()
                .chain(game.cards_in_zone(ZoneType::Battlefield, player).iter())
                .copied()
                .collect();
            valid.retain(|&cid| {
                if cid == source {
                    return false;
                }
                type_filter == "Card"
                    || type_filter.is_empty()
                    || crate::ability::effects::matches_change_type(
                        game.card(cid),
                        type_filter,
                        &[],
                    )
            });
            valid
        };

        let chosen_cards = if type_filter.ends_with("ChosenType") {
            // Java two-phase approach: pick 1 first, then pick `amount` from
            // cards sharing a creature type with the first pick.
            let pool = build_pool(game);
            if pool.is_empty() {
                return;
            }
            let first_pick = agents[player.index()].choose_cards_for_effect(player, &pool, 1, 1);
            if first_pick.is_empty() {
                return;
            }
            let first = first_pick[0];
            let same_type: Vec<CardId> = pool
                .into_iter()
                .filter(|&cid| shares_creature_type(game, first, cid))
                .collect();
            if (same_type.len() as i32) < amount {
                return;
            }
            agents[player.index()].choose_cards_for_effect(
                player,
                &same_type,
                amount as usize,
                amount as usize,
            )
        } else {
            // Non-ChosenType: pick `amount` cards at once.
            let pool = build_pool(game);
            if pool.is_empty() {
                return;
            }
            agents[player.index()].choose_cards_for_effect(
                player,
                &pool,
                amount as usize,
                amount as usize,
            )
        };

        for chosen in chosen_cards {
            if exile {
                let origin = game.card(chosen).zone;
                let owner = game.card(chosen).owner;
                game.move_card(chosen, ZoneType::Exile, owner);
                // Track the exile-with relationship so "Defined$ ExiledWith"
                // can find this card later (e.g. Champions of the Shoal's
                // leave-battlefield trigger returns the exiled card to hand).
                // We do NOT use `exiled_by` here because that field is reserved
                // for ChangeZoneAll Duration$ UntilHostLeavesPlay effects which
                // auto-return to battlefield in move_card.  BeholdExile cards
                // have their own dedicated leave-trigger to handle the return.
                game.card_mut(source).add_remembered_card(chosen);
                crate::ability::effects::emit_zone_trigger(
                    &mut self.trigger_handler,
                    chosen,
                    origin,
                    ZoneType::Exile,
                );
            }
        }
    }

    /// Blight as a cost: put -1/-1 counters on creatures you control.
    pub(crate) fn pay_blight_cost(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        amount: i32,
    ) {
        for _ in 0..amount {
            let valid: Vec<CardId> = game
                .cards_in_zone(ZoneType::Battlefield, player)
                .iter()
                .copied()
                .filter(|&cid| game.card(cid).is_creature())
                .collect();
            if valid.is_empty() {
                break;
            }
            if let Some(chosen) = agents[player.index()].choose_sacrifice(player, &valid) {
                game.card_mut(chosen)
                    .add_counter(&crate::card::CounterType::M1M1, 1);
            }
        }
    }

    /// Exile cards from controller battlefield or graveyard (craft helper).
    pub(crate) fn pay_exile_ctrl_or_grave_cost(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        type_filter: &str,
        amount: i32,
    ) {
        let base_filter = crate::cost::normalize_exile_base_filter(type_filter);
        for _ in 0..amount {
            let mut valid: Vec<CardId> =
                crate::cost::get_zone_targets(game, player, ZoneType::Battlefield, &base_filter);
            valid.extend(crate::cost::get_zone_targets(
                game,
                player,
                ZoneType::Graveyard,
                &base_filter,
            ));
            valid.retain(|&cid| can_exile_for_cost(game, cid));
            if valid.is_empty() {
                break;
            }
            if let Some(chosen) = agents[player.index()].choose_sacrifice(player, &valid) {
                let origin = game.card(chosen).zone;
                let owner = game.card(chosen).owner;
                game.move_card(chosen, ZoneType::Exile, owner);
                crate::ability::effects::emit_zone_trigger(
                    &mut self.trigger_handler,
                    chosen,
                    origin,
                    ZoneType::Exile,
                );
            }
        }
    }

    /// Return `amount` permanents matching `type_filter` for `player` to hand.
    /// Mirrors Java's `CostReturn.doListPayment()`.
    pub(crate) fn pay_return_cost(
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
                let from_zone = game.card(chosen).zone;
                game.move_card(chosen, ZoneType::Hand, owner);
                crate::ability::effects::emit_zone_trigger(
                    &mut self.trigger_handler,
                    chosen,
                    from_zone,
                    ZoneType::Hand,
                );
            }
        }
    }

    /// Tap `amount` other permanents matching `type_filter` as cost.
    /// When `min_total_power` is Some(N), tap creatures greedily until total power >= N (Crew).
    /// Otherwise tap exactly `amount` matching permanents.
    /// Mirrors Java's `CostTapType.doListPayment()`.
    pub(crate) fn pay_tap_type_cost(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        source: CardId,
        type_filter: &str,
        amount: i32,
        min_total_power: Option<i32>,
    ) {
        if let Some(power_threshold) = min_total_power {
            // Crew: greedily select creatures by descending power until threshold met.
            let mut valid = cost::get_tap_type_targets(game, player, type_filter, source);
            valid.sort_by(|&a, &b| game.card(b).power().cmp(&game.card(a).power()));
            let mut accum = 0;
            for &cid in &valid {
                if accum >= power_threshold {
                    break;
                }
                game.tap(cid);
                accum += game.card(cid).power();
                self.trigger_handler.run_trigger(
                    TriggerType::Taps,
                    RunParams {
                        card: Some(cid),
                        player: Some(player),
                        ..Default::default()
                    },
                    false,
                );
            }
        } else {
            for _ in 0..amount {
                let valid = cost::get_tap_type_targets(game, player, type_filter, source);
                if valid.is_empty() {
                    break;
                }
                if let Some(chosen) = agents[player.index()].choose_sacrifice(player, &valid) {
                    game.tap(chosen);
                    self.trigger_handler.run_trigger(
                        TriggerType::Taps,
                        RunParams {
                            card: Some(chosen),
                            player: Some(player),
                            ..Default::default()
                        },
                        false,
                    );
                }
            }
        }
    }

    /// Untap `amount` tapped permanents matching `type_filter` as cost.
    /// Mirrors Java's `CostUntapType.doListPayment()`.
    pub(crate) fn pay_untap_type_cost(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        source: CardId,
        type_filter: &str,
        amount: i32,
    ) {
        for _ in 0..amount {
            let valid: Vec<CardId> = game
                .cards_in_zone(ZoneType::Battlefield, player)
                .to_vec()
                .into_iter()
                .filter(|&cid| {
                    cid != source
                        && game.card(cid).tapped
                        && (type_filter == "Card"
                            || type_filter.is_empty()
                            || crate::ability::effects::matches_change_type(
                                game.card(cid),
                                type_filter,
                                &[],
                            ))
                })
                .collect();
            if valid.is_empty() {
                break;
            }
            if let Some(chosen) = agents[player.index()].choose_sacrifice(player, &valid) {
                game.untap(chosen);
            }
        }
    }

    /// Gain control of `amount` permanents matching `type_filter` as cost.
    /// Mirrors Java's `CostGainControl.doPayment()` which calls `addTempController`.
    pub(crate) fn pay_gain_control_cost(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        type_filter: &str,
        amount: i32,
    ) {
        for _ in 0..amount {
            // Rebuild valid list each iteration so already-controlled permanents are excluded.
            let valid: Vec<CardId> = game
                .players
                .iter()
                .map(|p| p.id)
                .flat_map(|pid| game.cards_in_zone(ZoneType::Battlefield, pid).to_vec())
                .filter(|&cid| {
                    game.card(cid).controller != player
                        && crate::ability::effects::matches_change_type(
                            game.card(cid),
                            type_filter,
                            &[],
                        )
                })
                .collect();
            if valid.is_empty() {
                break;
            }
            if let Some(chosen) = agents[player.index()].choose_sacrifice(player, &valid) {
                game.change_controller(chosen, player);
            }
        }
    }

    /// Remove `amount` counters (of `counter_type` or any type if None) from permanents
    /// matching `type_filter` as cost. Mirrors Java's `CostRemoveAnyCounter.payAsDecided()`.
    pub(crate) fn pay_remove_any_counter_cost(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        type_filter: &str,
        amount: i32,
        counter_type: Option<&crate::card::CounterType>,
    ) {
        for _ in 0..amount {
            // Build candidates that have at least one counter of the required type.
            let candidates: Vec<CardId> = game
                .cards_in_zone(ZoneType::Battlefield, player)
                .to_vec()
                .into_iter()
                .filter(|&cid| {
                    let matches_type = type_filter == "Permanent"
                        || type_filter.is_empty()
                        || crate::ability::effects::matches_change_type(
                            game.card(cid),
                            type_filter,
                            &[],
                        );
                    if !matches_type {
                        return false;
                    }
                    if let Some(ct) = counter_type {
                        game.card(cid).counter_count(ct) > 0
                    } else {
                        !game.card(cid).counters.is_empty()
                    }
                })
                .collect();
            if candidates.is_empty() {
                break;
            }
            // Let the agent choose which permanent to remove a counter from.
            let chosen = agents[player.index()]
                .choose_sacrifice(player, &candidates)
                .unwrap_or(candidates[0]);
            let ct_to_remove = if let Some(ct) = counter_type {
                ct.clone()
            } else {
                // Pick first available counter type on the chosen card.
                game.card(chosen).counters.keys().next().unwrap().clone()
            };
            game.card_mut(chosen).remove_counter(&ct_to_remove, 1);
            self.trigger_handler.run_trigger(
                TriggerType::CounterRemoved,
                RunParams {
                    card: Some(chosen),
                    player: Some(player),
                    counter_type: Some(format!("{:?}", ct_to_remove)),
                    counter_amount: Some(1),
                    ..Default::default()
                },
                false,
            );
        }
    }

    /// Move `amount` cards from exile to graveyard as cost.
    /// Mirrors Java's `CostExiledMoveToGrave.doPayment()`.
    pub(crate) fn pay_exiled_move_to_grave_cost(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        type_filter: &str,
        amount: i32,
    ) {
        for _ in 0..amount {
            let valid = cost::get_exiled_targets(game, type_filter);
            if valid.is_empty() {
                break;
            }
            if let Some(chosen) = agents[player.index()].choose_sacrifice(player, &valid) {
                let owner = game.card(chosen).owner;
                game.move_card(chosen, ZoneType::Graveyard, owner);
                crate::ability::effects::emit_zone_trigger(
                    &mut self.trigger_handler,
                    chosen,
                    ZoneType::Exile,
                    ZoneType::Graveyard,
                );
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
                if crate::staticability::static_ability_cant_sacrifice::cant_sacrifice(
                    &game.cards,
                    game.card(chosen),
                    None,
                    true,
                ) {
                    continue;
                }
                let owner = game.card(chosen).owner;
                // Capture sacrificed creature's power/toughness for Sacrificed$CardPower
                // SVar (used by Rite of Consumption, Altar's Reap, etc.).
                // Must be before move_card clears counters and resets stats.
                // Store LKI on the sacrificed card, and remember the card on the
                // spell's source so the SVar resolver can find it.
                {
                    let sac_power = game.card(chosen).power();
                    let sac_toughness = game.card(chosen).toughness();
                    game.card_mut(chosen).lki_power = Some(sac_power);
                    game.card_mut(chosen).lki_toughness = Some(sac_toughness);
                    // Store as last sacrificed card for SVar lookup.
                    // The SVar resolver reads this from the spell source's svars.
                    // Since we don't have the source card_id here, store it
                    // as a game-level transient for the SVar resolver to find.
                    game.last_sacrificed_card = Some(chosen);
                }
                // Capture +1/+1 counter count BEFORE move_card clears counters.
                // Needed for Modular death triggers (CR 702.43b).
                let lki_p1p1 = *game
                    .card(chosen)
                    .counters
                    .get(&crate::card::CounterType::P1P1)
                    .unwrap_or(&0);
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
                // NOTE: pump triggers (e.g. Supernatural Stamina's death-return
                // trigger) must NOT be cleared here. They need to fire during the
                // ChangesZone event below. Cleanup happens in move_card when the
                // card actually changes zones — after the trigger has been matched
                // and queued by emit_zone_trigger + flush_waiting_triggers.
                crate::ability::effects::emit_zone_trigger_with_lki_counters(
                    &mut self.trigger_handler,
                    chosen,
                    ZoneType::Battlefield,
                    ZoneType::Graveyard,
                    lki_p1p1,
                );
                self.trigger_handler.flush_waiting_triggers(game);
                game.move_card(chosen, ZoneType::Graveyard, owner);
            }
        }
    }
}

fn shares_creature_type(game: &GameState, a: CardId, b: CardId) -> bool {
    let ca = game.card(a);
    let cb = game.card(b);
    if !ca.is_creature() || !cb.is_creature() {
        return false;
    }
    ca.type_line
        .subtypes
        .iter()
        .any(|st| cb.type_line.has_subtype(st))
}

fn can_exile_for_cost(game: &GameState, card_id: CardId) -> bool {
    let static_sources = crate::cost::static_ability_source_cards(game);
    !crate::staticability::static_ability_cant_exile::cant_exile(
        &static_sources,
        game.card(card_id),
        None,
        true,
    )
}
