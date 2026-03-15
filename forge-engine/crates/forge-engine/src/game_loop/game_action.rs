use super::*;
use forge_foundation::mana::ManaAtom;

pub(crate) enum CostPaymentContext {
    TriggerResolve,
    ActivatedAbility,
    ManaAbility,
}

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
                .get("GameActivationLimit")
                .and_then(|v| v.trim().parse::<u32>().ok())
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
            if ab.params.get("PowerUp").map_or(false, |v| v == "True") {
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
            if ab.params.get("SorcerySpeed").map_or(false, |v| v == "True") && !can_play_sorcery {
                return false;
            }
            // Activated abilities that require targets should only be offered
            // when at least one legal target candidate exists.
            let sa_for_target_check =
                crate::spellability::build_spell_ability(game, card_id, &ab.ability_text, player);
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
                if ab
                    .params
                    .get("ActivationZone")
                    .map_or(false, |z| z == "Hand")
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
                if ab
                    .params
                    .get("ActivationZone")
                    .map_or(false, |z| z == "Hand")
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
                if ab
                    .params
                    .get("ActivationZone")
                    .map_or(false, |z| z == "Graveyard")
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
                if ab
                    .params
                    .get("ActivationZone")
                    .map_or(false, |z| z == "Exile")
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
        let api = ab.params.get("AB").map(String::as_str);
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

    /// Pay life and fire the LifeLost trigger.
    fn pay_life_cost(&mut self, game: &mut GameState, player: PlayerId, amount: i32) {
        if crate::staticability::static_ability_cant_gain_lose_pay_life::cant_pay_life(
            game, player, true, None,
        ) {
            return;
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
    ) {
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
        for cid in chosen {
            let owner = game.card(cid).owner;
            self.trigger_handler.run_trigger(
                TriggerType::Discarded,
                RunParams {
                    card: Some(cid),
                    player: Some(player),
                    ..Default::default()
                },
                false,
            );
            game.move_card(cid, ZoneType::Graveyard, owner);
            crate::ability::effects::emit_zone_trigger(
                &mut self.trigger_handler,
                cid,
                ZoneType::Hand,
                ZoneType::Graveyard,
            );
        }
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
        // Mirror Java cost-payment flow by resolving optional confirmations before
        // mutating any game state. This avoids partially paid costs when a later
        // cost part is declined.
        for part in cost.parts.iter() {
            if !self.confirm_cost_part_payment(game, agents, player, card_id, part, api, mandatory)
            {
                return false;
            }
        }
        // Java CostPayment is transactional: if any later cost part fails,
        // previously applied parts are undone. Mirror that via full snapshot.
        let payment_snapshot = self.make_snapshot(game, true);
        let mut payment_ok = true;
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
                    let reuses_reserved_source = cost.parts.iter().any(|part| {
                        matches!(
                            part,
                            CostPart::Sacrifice { type_filter, .. } if type_filter == "CARDNAME"
                        )
                    });
                    // Use sacrifice chooser callback to match Java's
                    // choosePermanentsToSacrifice RNG path during auto-tap.
                    let mut chooser = |valid: &[CardId]| -> Option<CardId> {
                        agents[player.index()].choose_sacrifice(player, valid)
                    };
                    let tapped = if reuses_reserved_source {
                        mana::auto_tap_lands_allow_reserved_source_reuse_with_chooser(
                            game,
                            &mut self.mana_pools[player.index()],
                            player,
                            mana_cost,
                            Some(card_id),
                            &mut chooser,
                        )
                    } else {
                        mana::auto_tap_lands_with_chooser(
                            game,
                            &mut self.mana_pools[player.index()],
                            player,
                            mana_cost,
                            Some(card_id),
                            &mut chooser,
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
            crate::ability::effects::emit_zone_trigger_with_lki_counters(
                &mut self.trigger_handler,
                chosen,
                ZoneType::Battlefield,
                ZoneType::Graveyard,
                lki_p1p1,
            );
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
                    game.move_card(chosen, ZoneType::Graveyard, owner);
                    crate::ability::effects::emit_zone_trigger_with_lki_counters(
                        &mut self.trigger_handler,
                        chosen,
                        ZoneType::Battlefield,
                        ZoneType::Graveyard,
                        lki_p1p1,
                    );
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
                game.move_card(chosen, ZoneType::Graveyard, owner);
                crate::ability::effects::emit_zone_trigger_with_lki_counters(
                    &mut self.trigger_handler,
                    chosen,
                    ZoneType::Battlefield,
                    ZoneType::Graveyard,
                    lki_p1p1,
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
        let api = ab.params.get("AB").map(String::as_str);
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
        if ab.params.get("AB").map_or(false, |v| v == "ManaReflected") {
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
        let mana_restriction = ab.params.get("RestrictValid").cloned();
        let adds_no_counter = ab
            .params
            .get("AddsNoCounter")
            .map_or(false, |v| v == "True");
        let adds_keywords = ab.params.get("AddsKeywords").cloned();
        let adds_keywords_valid = ab.params.get("AddsKeywordsValid").cloned();
        let adds_counters = ab.params.get("AddsCounters").cloned();
        let adds_counters_valid = ab.params.get("AddsCountersValid").cloned();
        let triggers_when_spent = ab.params.get("TriggersWhenSpent").cloned();

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

        if let Some(produced) = ab.params.get("Produced") {
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
            if let Some(amount_str) = ab.params.get("Amount") {
                let amount = if let Ok(n) = amount_str.parse::<i32>() {
                    n
                } else {
                    // Try to resolve as SVar on the source card
                    if let Some(svar_expr) =
                        game.card(card_id).svars.get(amount_str.as_str()).cloned()
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
                    let produced = ab.params.get("Produced").map(String::as_str).unwrap_or("");
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
            use crate::replacement::handler::{apply_replacements, ReplacementEvent};
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
        if sa.api.as_deref() == Some("Charm")
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
        let adjusted_cost = if ab.params.get("PowerUp").map_or(false, |v| v == "True")
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
        let api = ab.params.get("AB").map(String::as_str);
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
            .get("AB")
            .cloned()
            .unwrap_or_else(|| "Unknown".to_string());
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
