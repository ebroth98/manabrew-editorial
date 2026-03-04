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
    ) -> Vec<(CardId, usize)> {
        let mut result = Vec::new();
        let available_mana = mana::calculate_available_mana(self.pool(player), game, player);
        let deterministic_mana_probe =
            |exclude_source: Option<CardId>| -> crate::mana::ManaPool {
                let mut probe = crate::mana::ManaPool::new();
                let mut source_count: i32 = 0;

                for &cid in game.cards_in_zone(ZoneType::Battlefield, player) {
                    if Some(cid) == exclude_source {
                        continue;
                    }
                    let c = game.card(cid);
                    if c.tapped {
                        continue;
                    }

                    let mut can_w = false;
                    let mut can_u = false;
                    let mut can_b = false;
                    let mut can_r = false;
                    let mut can_g = false;
                    let mut can_c = false;

                    for mab in &c.activated_abilities {
                        if !mab.is_mana_ability {
                            continue;
                        }
                        if let Some(produced) = mab.params.get("Produced") {
                            let upper = produced.to_ascii_uppercase();
                            if upper.contains('W') {
                                can_w = true;
                            }
                            if upper.contains('U') {
                                can_u = true;
                            }
                            if upper.contains('B') {
                                can_b = true;
                            }
                            if upper.contains('R') {
                                can_r = true;
                            }
                            if upper.contains('G') {
                                can_g = true;
                            }
                            if upper.contains('C') {
                                can_c = true;
                            }
                            if upper.contains("ANY") {
                                can_w = true;
                                can_u = true;
                                can_b = true;
                                can_r = true;
                                can_g = true;
                                can_c = true;
                            }
                            for atom in mana::produced_to_atoms(produced, &c.chosen_colors) {
                                match atom {
                                    ManaAtom::WHITE => can_w = true,
                                    ManaAtom::BLUE => can_u = true,
                                    ManaAtom::BLACK => can_b = true,
                                    ManaAtom::RED => can_r = true,
                                    ManaAtom::GREEN => can_g = true,
                                    ManaAtom::COLORLESS => can_c = true,
                                    _ => {}
                                }
                            }
                        }
                    }

                    if !can_w && !can_u && !can_b && !can_r && !can_g && !can_c && c.is_land() {
                        for atom in mana::land_mana_atoms(c) {
                            match atom {
                                ManaAtom::WHITE => can_w = true,
                                ManaAtom::BLUE => can_u = true,
                                ManaAtom::BLACK => can_b = true,
                                ManaAtom::RED => can_r = true,
                                ManaAtom::GREEN => can_g = true,
                                ManaAtom::COLORLESS => can_c = true,
                                _ => {}
                            }
                        }
                        if let Some(atom) = mana::basic_land_mana_atom(c) {
                            match atom {
                                ManaAtom::WHITE => can_w = true,
                                ManaAtom::BLUE => can_u = true,
                                ManaAtom::BLACK => can_b = true,
                                ManaAtom::RED => can_r = true,
                                ManaAtom::GREEN => can_g = true,
                                ManaAtom::COLORLESS => can_c = true,
                                _ => {}
                            }
                        }
                    }

                    if can_w || can_u || can_b || can_r || can_g || can_c {
                        source_count += 1;
                        if can_w {
                            probe.white += 1;
                        }
                        if can_u {
                            probe.blue += 1;
                        }
                        if can_b {
                            probe.black += 1;
                        }
                        if can_r {
                            probe.red += 1;
                        }
                        if can_g {
                            probe.green += 1;
                        }
                        if can_c {
                            probe.colorless += 1;
                        }
                    }
                }

                probe.total_sources = Some(source_count);
                probe
            };
        let battlefield = game.cards_in_zone(ZoneType::Battlefield, player).to_vec();
        let can_activate = |card_id: CardId, ab: &crate::ability::ActivatedAbility| {
            // Mirror Java deterministic controller action-space:
            // do not expose pure mana abilities as standalone random actions.
            if ab.is_mana_ability {
                return false;
            }
            if !can_pay(&ab.cost, game, &available_mana, card_id, player) {
                return false;
            }
            // Java DeterministicController hasDeterministicMana parity:
            // for non-mana activated abilities in play, do not assume the source can
            // pay its own mana cost during deterministic probing (even without Tap cost).
            // This keeps the action space aligned with Java chooseSpellAbilityToPlay().
            let has_mana_cost = ab
                .cost
                .parts
                .iter()
                .any(|p| matches!(p, CostPart::Mana(_)));
            if has_mana_cost {
                let available_without_source = deterministic_mana_probe(Some(card_id));
                if !can_pay(&ab.cost, game, &available_without_source, card_id, player) {
                    return false;
                }
            }
            true
        };

        for card_id in battlefield {
            let card = game.card(card_id);
            for ab in &card.activated_abilities {
                // Skip abilities with ActivationZone$ Hand — they're for hand, not battlefield
                if ab.params.get("ActivationZone").map_or(false, |z| z == "Hand") {
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
                if ab.params.get("ActivationZone").map_or(false, |z| z == "Hand") {
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
                if ab.params.get("ActivationZone").map_or(false, |z| z == "Graveyard") {
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
                if ab.params.get("ActivationZone").map_or(false, |z| z == "Exile") {
                    if can_activate(card_id, ab) {
                        result.push((card_id, ab.ability_index));
                    }
                }
            }
        }

        result
    }

    /// Activate an ability on a permanent.
    pub(crate) fn activate_ability(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        card_id: CardId,
        ability_idx: usize,
    ) {
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
            None => return,
        };

        if ab.is_mana_ability {
            self.resolve_mana_ability(game, agents, player, card_id, &ab);
        } else {
            self.activate_ability_on_stack(game, agents, player, card_id, &ab);
        }
    }

    /// Pay life and fire the LifeLost trigger.
    fn pay_life_cost(&mut self, game: &mut GameState, player: PlayerId, amount: i32) {
        if crate::staticability::static_ability_cant_gain_lose_pay_life::cant_pay_life(
            game,
            player,
            true,
            None,
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
        // Let the agent choose which cards to discard.
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

    /// Pay the cost parts of an activated ability (tap, mana, life, sacrifice, etc.).
    /// Mirrors Java's `CostPayment.payCost()` iterating over `CostPart`s.
    pub(crate) fn pay_ability_cost(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        card_id: CardId,
        cost: &crate::cost::Cost,
    ) {
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
                    let tapped = mana::auto_tap_lands(
                        game,
                        &mut self.mana_pools[player.index()],
                        player,
                        mana_cost,
                        Some(card_id),
                    );
                    self.emit_tap_for_mana_triggers(player, &tapped);
                    self.mana_pools[player.index()].try_pay(mana_cost);
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
                        self.pay_discard_cost(game, agents, player, type_filter, *amount);
                    }
                }
                CostPart::ExileFromAnyGrave { amount, type_filter } => {
                    self.pay_exile_from_any_grave_cost(game, agents, player, type_filter, *amount);
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
                CostPart::Exile { amount, type_filter, from } => {
                    if type_filter == "CARDNAME" {
                        game.move_card(card_id, ZoneType::Exile, game.card(card_id).owner);
                    } else {
                        self.pay_exile_cost(game, agents, player, type_filter, *amount, *from);
                    }
                }
                CostPart::Return { amount, type_filter } => {
                    if type_filter == "CARDNAME" {
                        let owner = game.card(card_id).owner;
                        game.move_card(card_id, ZoneType::Hand, owner);
                    } else {
                        self.pay_return_cost(game, agents, player, type_filter, *amount);
                    }
                }
                CostPart::TapType { amount, type_filter } => {
                    self.pay_tap_type_cost(game, agents, player, card_id, type_filter, *amount);
                }
                CostPart::UntapType { amount, type_filter } => {
                    self.pay_untap_type_cost(game, agents, player, card_id, type_filter, *amount);
                }
                CostPart::PayEnergy(amount) => {
                    game.player_mut(player).energy_counters -= amount;
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
                        let lib: Vec<CardId> =
                            game.cards_in_zone(ZoneType::Library, player).to_vec();
                        if let Some(&top) = lib.first() {
                            let owner = game.card(top).owner;
                            game.move_card(top, ZoneType::Graveyard, owner);
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
                CostPart::Reveal { .. } => {
                    // Reveal is a visible-information cost — no hidden state change.
                    // In a full implementation we'd notify all players. No-op here.
                }
                CostPart::Exert => {
                    // Exert: the creature doesn't untap during your next untap step.
                    game.card_mut(card_id).exerted = true;
                    self.trigger_handler.run_trigger(
                        TriggerType::Exerted,
                        RunParams {
                            card: Some(card_id),
                            player: Some(player),
                            ..Default::default()
                        },
                        false,
                    );
                }
                CostPart::GainLife(amount) => {
                    // Opponent gains life
                    let opponent = game.opponent_of(player);
                    game.player_mut(opponent).gain_life(*amount);
                }
                CostPart::GainControl { amount, type_filter } => {
                    self.pay_gain_control_cost(game, agents, player, type_filter, *amount);
                }
                CostPart::RemoveAnyCounter { amount, type_filter, counter_type } => {
                    self.pay_remove_any_counter_cost(game, agents, player, type_filter, *amount, counter_type.as_ref());
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
                CostPart::ExiledMoveToGrave { amount, type_filter } => {
                    self.pay_exiled_move_to_grave_cost(game, agents, player, type_filter, *amount);
                }
            }
        }
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
    ) {
        for part in spell_cost.parts.clone() {
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
                        self.pay_discard_cost(game, agents, player, type_filter, *amount);
                    }
                }
                CostPart::ExileFromAnyGrave { amount, type_filter } => {
                    self.pay_exile_from_any_grave_cost(game, agents, player, type_filter, *amount);
                }
                CostPart::SubCounter {
                    amount,
                    counter_type,
                } => {
                    if game.card(card_id).zone == ZoneType::Battlefield {
                        game.card_mut(card_id).remove_counter(counter_type, *amount);
                    }
                }
                CostPart::AddCounter { amount, counter_type } => {
                    if game.card(card_id).zone == ZoneType::Battlefield {
                        game.card_mut(card_id).add_counter(counter_type, *amount);
                    }
                }
                CostPart::Exile { amount, type_filter, from } => {
                    if type_filter != "CARDNAME" {
                        self.pay_exile_cost(game, agents, player, type_filter, *amount, *from);
                    }
                }
                CostPart::Return { amount, type_filter } => {
                    if type_filter != "CARDNAME" {
                        self.pay_return_cost(game, agents, player, type_filter, *amount);
                    }
                }
                CostPart::TapType { amount, type_filter } => {
                    self.pay_tap_type_cost(game, agents, player, card_id, type_filter, *amount);
                }
                CostPart::UntapType { amount, type_filter } => {
                    self.pay_untap_type_cost(game, agents, player, card_id, type_filter, *amount);
                }
                CostPart::PayEnergy(amount) => {
                    game.player_mut(player).energy_counters -= amount;
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
                        let lib: Vec<CardId> =
                            game.cards_in_zone(ZoneType::Library, player).to_vec();
                        if let Some(&top) = lib.first() {
                            let owner = game.card(top).owner;
                            game.move_card(top, ZoneType::Graveyard, owner);
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
                CostPart::Reveal { .. } => {}
                CostPart::Exert => {
                    game.card_mut(card_id).exerted = true;
                    self.trigger_handler.run_trigger(
                        TriggerType::Exerted,
                        RunParams {
                            card: Some(card_id),
                            player: Some(player),
                            ..Default::default()
                        },
                        false,
                    );
                }
                CostPart::GainLife(amount) => {
                    let opponent = game.opponent_of(player);
                    game.player_mut(opponent).gain_life(*amount);
                }
                CostPart::GainControl { amount, type_filter } => {
                    self.pay_gain_control_cost(game, agents, player, type_filter, *amount);
                }
                CostPart::RemoveAnyCounter { amount, type_filter, counter_type } => {
                    self.pay_remove_any_counter_cost(game, agents, player, type_filter, *amount, counter_type.as_ref());
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
                CostPart::ExiledMoveToGrave { amount, type_filter } => {
                    self.pay_exiled_move_to_grave_cost(game, agents, player, type_filter, *amount);
                }
            }
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
        for _ in 0..amount {
            let valid: Vec<CardId> = game
                .players
                .iter()
                .map(|p| p.id)
                .flat_map(|pid| game.cards_in_zone(ZoneType::Graveyard, pid).to_vec())
                .filter(|&cid| {
                    type_filter == "Card"
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

    /// Exile `amount` cards from `zone` matching `type_filter` for `player`.
    /// Mirrors Java's `CostExile.doListPayment()`.
    pub(crate) fn pay_exile_cost(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        type_filter: &str,
        amount: i32,
        from: ZoneType,
    ) {
        for _ in 0..amount {
            let valid = cost::get_zone_targets(game, player, from, type_filter);
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
    /// Mirrors Java's `CostTapType.doListPayment()`.
    pub(crate) fn pay_tap_type_cost(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        source: CardId,
        type_filter: &str,
        amount: i32,
    ) {
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
                crate::ability::effects::emit_zone_trigger(
                    &mut self.trigger_handler,
                    chosen,
                    ZoneType::Battlefield,
                    ZoneType::Graveyard,
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
        self.pay_ability_cost(game, agents, player, card_id, &ab.cost);

        // Produce mana
        if let Some(produced) = ab.params.get("Produced") {
            if produced == "Combo ColorIdentity" {
                // Commander mana: look up the commander's color identity.
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

                // Java parity: in non-commander games ColorIdentity may be empty;
                // in that case no mana is produced.
                if !colors.is_empty() {
                    if let Some(chosen) = agents[player.index()].choose_color(player, &colors) {
                        if let Some(atom) = color_name_to_mana_atom(&chosen) {
                            self.pool_mut(player).add(atom, 1);
                        }
                    }
                }
            } else {
                let chosen_colors = game.card(card_id).chosen_colors.clone();
                let colors = produced_to_color_names(produced, &chosen_colors);
                if colors.len() > 1 {
                    // Variable-color production (Any / Combo / multi-choice Chosen)
                    if let Some(chosen) = agents[player.index()].choose_color(player, &colors) {
                        if let Some(atom) = color_name_to_mana_atom(&chosen) {
                            self.pool_mut(player).add(atom, 1);
                        }
                    }
                } else if let Some(single) = colors.first() {
                    // Deterministic selected color (e.g. Produced$ Chosen with one chosen color)
                    if let Some(atom) = color_name_to_mana_atom(single) {
                        self.pool_mut(player).add(atom, 1);
                    }
                } else if let Some(atom) = mana_atom_from_produced(produced) {
                    // Backward-compatible single-token fallback
                    self.pool_mut(player).add(atom, 1);
                } else {
                    // Handle raw multi-token fixed outputs like "C C"
                    for tok in produced.split_whitespace() {
                        if let Some(atom) = mana_atom_from_produced(tok) {
                            self.pool_mut(player).add(atom, 1);
                        }
                    }
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
    ) {
        let ability_text = ab.ability_text.clone();

        // Build full SpellAbility chain (including SubAbility$ links) and choose targets.
        // This mirrors Java activated ability resolution for abilities like Walking Bulwark,
        // where the root AB$ Pump resolves a DB$ Effect sub-ability.
        let mut sa = crate::spellability::build_spell_ability(game, card_id, &ability_text, player);
        sa.is_activated = true;
        sa.setup_targets(game, agents, &self.mana_pools);

        // Fire BecomesTarget trigger if a card was targeted
        if let Some(target_card) = sa.target_chosen.target_card {
            self.trigger_handler.run_trigger(
                TriggerType::BecomesTarget,
                RunParams {
                    card: Some(target_card),
                    cause_player: Some(player),
                    ..Default::default()
                },
                false,
            );
        }

        // Pay costs
        self.pay_ability_cost(game, agents, player, card_id, &ab.cost);

        // Push to stack
        let card_name = game.card(card_id).card_name.clone();
        let entry = StackEntry {
            id: 0,
            spell_ability: sa,
            is_creature_spell: false,
            is_permanent_spell: false,
            cast_from_zone: None,
        };
        game.stack.push(entry);
        self.log_stack_push(&format!("{} ability", card_name), &game.player(player).name);
        let ability_kind = ab
            .params
            .get("AB")
            .cloned()
            .unwrap_or_else(|| "Unknown".to_string());
        agents[player.index()].notify(&format!(
            "Activated ability: {} | source={}",
            ability_kind, card_name
        ));

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
    }
}
