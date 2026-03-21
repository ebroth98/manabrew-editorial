use super::*;

use crate::cost::cost_adjustment::{count_affinity_permanents, get_affinity_type};
use forge_foundation::mana::ManaAtom;

impl GameLoop {
    pub(crate) fn parse_spell_cost(abilities: &[String]) -> Option<crate::cost::Cost> {
        for ability in abilities {
            let params = parse_pipe_params(ability);
            // Only process SP$ lines (spell abilities)
            if params.contains_key("SP") {
                if let Some(cost_str) = params.get("Cost") {
                    return Some(parse_cost(cost_str));
                }
            }
        }
        None
    }

    /// Play a card from hand (or graveyard for Escape). Returns the (card_id, card_name)
    /// if the card was successfully played, so the caller can emit the notification.
    pub(crate) fn play_card(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        card_id: CardId,
        play_mode: crate::agent::PlayCardMode,
    ) -> Option<(CardId, String)> {
        let card = game.card(card_id);
        let card_name = card.card_name.clone();

        if card.is_land() {
            if play_mode != crate::agent::PlayCardMode::Normal {
                return None;
            }
            // Check for shock-land-style "pay life or enter tapped" before entering
            let etb_life_cost =
                crate::staticability::layer::get_etb_unless_life_cost(game.card(card_id));
            // Check for "reveal <type> from hand or enter tapped" (e.g. Wanderwine Hub)
            let etb_reveal_cost =
                crate::staticability::layer::get_etb_unless_reveal_cost(game.card(card_id));

            // Play land — goes directly to battlefield
            game.move_card(card_id, ZoneType::Battlefield, player);

            // Handle "reveal or enter tapped" before the shock-land check
            if let Some((_n, filter_str)) = etb_reveal_cost {
                // Check if player has matching cards in hand to reveal
                let type_name = filter_str.split('/').next().unwrap_or(&filter_str);
                let has_matching = game
                    .cards_in_zone(ZoneType::Hand, player)
                    .iter()
                    .any(|&cid| game.card(cid).type_line.has_subtype(type_name));
                if has_matching {
                    // Player can choose to reveal — DeterministicAgent always passes (no reveal)
                    // to match Java DeterministicController which passes optional pays
                    game.card_mut(card_id).tapped = true;
                } else {
                    // No matching card — must enter tapped
                    game.card_mut(card_id).tapped = true;
                }
            }

            // Prompt for shock land life payment (after ETB so the card is on battlefield)
            if let Some(life_cost) = etb_life_cost {
                let desc = format!("Pay {} life so {} enters untapped?", life_cost, card_name);
                agents[player.index()].snapshot_state(game, &self.mana_pools);
                let pay = agents[player.index()].choose_optional_trigger(
                    player,
                    &desc,
                    Some(&card_name),
                    None,
                );
                if pay {
                    // Run PayLife replacement effects before paying life.
                    let skip_life = {
                        use crate::replacement::replacement_handler::{
                            apply_replacements, ReplacementEvent,
                        };
                        use crate::replacement::ReplacementResult;
                        let mut event = ReplacementEvent::PayLife {
                            player,
                            amount: life_cost,
                        };
                        let result = apply_replacements(game, &mut event);
                        result == ReplacementResult::Skipped
                            || result == ReplacementResult::Replaced
                    };
                    if !skip_life {
                        // Player pays life — untap the card (it wasn't tapped by apply_etb_tapped
                        // since we removed the third pass, but ensure untapped state)
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
                    }
                } else {
                    // Player declines — enter tapped
                    game.card_mut(card_id).tapped = true;
                }
            }

            game.player_mut(player).lands_played_this_turn += 1;
            crate::agent::notify_all_agents(
                agents,
                crate::agent::GameLogEvent::action(format!("Played land: {}", card_name))
                    .with_player(player)
                    .with_card(card_id),
            );

            // Register triggers for the new permanent (must happen before
            // emitting ChangesZone so the land's own ETB triggers are active).
            self.trigger_handler.register_active_trigger(game, card_id);

            // Emit ChangesZone trigger (ETB) — mirrors the stack resolver's
            // emit_zone_trigger for spells entering the battlefield.
            crate::ability::effects::emit_zone_trigger(
                &mut self.trigger_handler,
                card_id,
                ZoneType::Hand,
                ZoneType::Battlefield,
            );

            // Fire LandPlayed trigger
            self.trigger_handler.run_trigger(
                TriggerType::LandPlayed,
                RunParams {
                    card: Some(card_id),
                    player: Some(player),
                    ..Default::default()
                },
                false,
            );
        } else {
            // Cast spell — tap lands for mana, put on stack, resolve
            let is_creature = game.card(card_id).is_creature();
            let is_permanent = game.card(card_id).is_permanent();

            // ── Alternative cost mode selection (from action-space choice) ──────────
            let mut is_foretell = false;
            let mut is_flashback = false;
            let mut is_spectacle = false;
            let mut is_evoke = false;
            let mut is_escape = false;
            let mut is_overload = false;
            let mut is_dash = false;
            let mut is_blitz = false;
            let mut is_madness = false;
            let mut is_emerge = false;
            let mut is_gainlife_alt = false;
            let mut is_sacrifice_alt = false;
            let mut is_plot_cast = false;
            let mut is_bestow = false;
            let mut is_warp = false;
            let mut is_morph_facedown = false;

            match play_mode {
                crate::agent::PlayCardMode::Normal => {}
                crate::agent::PlayCardMode::GainLifeAlt => {
                    is_gainlife_alt = true;
                }
                crate::agent::PlayCardMode::ForetellExile => {
                    if game.card(card_id).get_foretell_cost().is_some()
                        && game.card(card_id).zone == ZoneType::Hand
                    {
                        let available_mana =
                            mana::calculate_available_mana(self.pool(player), game, player);
                        let foretell_exile_cost = forge_foundation::ManaCost::generic(2);
                        if !available_mana.can_pay(&foretell_exile_cost) {
                            return None;
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
                        game.card_mut(card_id).face_down = true;
                        game.move_card(card_id, ZoneType::Exile, player);
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
                        return Some((card_id, card_name));
                    }
                    return None;
                }
                crate::agent::PlayCardMode::Alternative(alt) => match alt {
                    crate::spellability::AlternativeCost::Foretell => is_foretell = true,
                    crate::spellability::AlternativeCost::Flashback => is_flashback = true,
                    crate::spellability::AlternativeCost::Spectacle => is_spectacle = true,
                    crate::spellability::AlternativeCost::Evoke => is_evoke = true,
                    crate::spellability::AlternativeCost::Dash => is_dash = true,
                    crate::spellability::AlternativeCost::Blitz => is_blitz = true,
                    crate::spellability::AlternativeCost::Escape => is_escape = true,
                    crate::spellability::AlternativeCost::Overload => is_overload = true,
                    crate::spellability::AlternativeCost::Madness => is_madness = true,
                    crate::spellability::AlternativeCost::Emerge => is_emerge = true,
                    crate::spellability::AlternativeCost::Bestow => is_bestow = true,
                    crate::spellability::AlternativeCost::Warp => is_warp = true,
                    crate::spellability::AlternativeCost::SacrificeAlt => is_sacrifice_alt = true,
                    crate::spellability::AlternativeCost::Plot => is_plot_cast = true,
                    crate::spellability::AlternativeCost::Morph
                    | crate::spellability::AlternativeCost::Megamorph => {
                        is_morph_facedown = true;
                    }
                    crate::spellability::AlternativeCost::Suspend => {
                        if let Some((suspend_cost, counters)) =
                            game.card(card_id).get_suspend_cost()
                        {
                            if game.card(card_id).zone != ZoneType::Hand {
                                return None;
                            }
                            let available_mana =
                                mana::calculate_available_mana(self.pool(player), game, player);
                            let suspend_mc = forge_foundation::ManaCost::parse(&suspend_cost);
                            if !available_mana.can_pay(&suspend_mc) {
                                return None;
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
                            game.move_card(card_id, ZoneType::Exile, player);
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
                            return Some((card_id, card_name));
                        }
                        return None;
                    }
                },
            }

            // Select the card's spell ability line (SP$ ...) for cast-time logic.
            // Mirrors Java where casting operates on a concrete SpellAbility, not
            // arbitrary non-activated lines like `S:Mode$ OptionalCost`.
            let abilities_for_spell = game.card(card_id).abilities.clone();
            let spell_ability_text = abilities_for_spell
                .iter()
                .find(|a| parse_pipe_params(a).contains_key("SP"))
                .cloned()
                .unwrap_or_default();

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
                        if let CostPart::Mana(mc) = part {
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
                game.card_mut(card_id).face_down = false; // reveal it
                forge_foundation::ManaCost::parse(&foretell_cost_str)
            } else if is_flashback {
                flashback_mana_cost.unwrap_or_else(forge_foundation::ManaCost::zero)
            } else if is_madness {
                let madness_cost_str = game.card(card_id).get_madness_cost().unwrap_or_default();
                crate::ability::effects::helpers::remove_madness_exiled_marker(
                    game.card_mut(card_id),
                );
                forge_foundation::ManaCost::parse(&madness_cost_str)
            } else if is_spectacle {
                let spec_cost_str = game.card(card_id).get_spectacle_cost().unwrap_or_default();
                forge_foundation::ManaCost::parse(&spec_cost_str)
            } else if is_evoke {
                let evoke_cost_str = game.card(card_id).get_evoke_cost().unwrap_or_default();
                forge_foundation::ManaCost::parse(&evoke_cost_str)
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
            } else if is_gainlife_alt {
                // GainLife alternative cost: cast for free (zero mana).
                // The side effect (opponent gains life) is applied below.
                forge_foundation::ManaCost::generic(0)
            } else if is_sacrifice_alt {
                // Sacrifice-based alternative cost: cast for free (zero mana).
                // The sacrifice is performed below.
                forge_foundation::ManaCost::generic(0)
            } else if is_plot_cast {
                // Plot: cast from exile for free (already paid plot cost).
                forge_foundation::ManaCost::generic(0)
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

            // ── Emerge: sacrifice a creature to reduce cost ──────────
            let mana_cost = if is_emerge {
                let mut cost = mana_cost;
                let creatures: Vec<CardId> = game
                    .cards_in_zone(ZoneType::Battlefield, player)
                    .iter()
                    .filter(|&&cid| cid != card_id && game.card(cid).is_creature())
                    .copied()
                    .collect();
                if !creatures.is_empty() {
                    agents[player.index()].snapshot_state(game, &self.mana_pools);
                    if let Some(sac_id) =
                        agents[player.index()].choose_sacrifice(player, &creatures)
                    {
                        // Reduce emerge cost by the sacrificed creature's mana value
                        let sac_cmc = game.card(sac_id).mana_cost.cmc();
                        cost = cost.reduce_generic(sac_cmc);
                        // Sacrifice the creature
                        let sac_owner = game.card(sac_id).owner;
                        self.trigger_handler.run_trigger(
                            TriggerType::Sacrificed,
                            RunParams {
                                card: Some(sac_id),
                                player: Some(player),
                                ..Default::default()
                            },
                            false,
                        );
                        {
                            let card = game.card_mut(sac_id);
                            let pt = card.pump_trigger_count;
                            if pt > 0 {
                                let new_len = card.triggers.len().saturating_sub(pt);
                                card.triggers.truncate(new_len);
                                card.pump_trigger_count = 0;
                            }
                        }
                        crate::ability::effects::emit_zone_trigger(
                            &mut self.trigger_handler,
                            sac_id,
                            ZoneType::Battlefield,
                            ZoneType::Graveyard,
                        );
                        self.trigger_handler.flush_waiting_triggers(game);
                        game.move_card(sac_id, ZoneType::Graveyard, sac_owner);
                    }
                }
                cost
            } else {
                mana_cost
            };

            // ── Offering: sacrifice a permanent of a type to reduce cost ──────────
            let mana_cost = if let Some(offering_type) = game.card(card_id).get_offering_type() {
                let mut cost = mana_cost;
                let offering_type_lower = offering_type.to_lowercase();
                let candidates: Vec<CardId> = game
                    .cards_in_zone(ZoneType::Battlefield, player)
                    .iter()
                    .filter(|&&cid| {
                        cid != card_id && {
                            let c = game.card(cid);
                            match offering_type_lower.as_str() {
                                "creature" => c.is_creature(),
                                "artifact" => c.type_line.is_artifact(),
                                "enchantment" => c.type_line.is_enchantment(),
                                "land" => c.is_land(),
                                _ => c.type_line.has_subtype(&offering_type),
                            }
                        }
                    })
                    .copied()
                    .collect();
                if !candidates.is_empty() {
                    agents[player.index()].snapshot_state(game, &self.mana_pools);
                    if let Some(sac_id) =
                        agents[player.index()].choose_sacrifice(player, &candidates)
                    {
                        // Reduce cost by sacrificed permanent's mana value
                        let sac_cmc = game.card(sac_id).mana_cost.cmc();
                        cost = cost.reduce_generic(sac_cmc);
                        let sac_owner = game.card(sac_id).owner;
                        self.trigger_handler.run_trigger(
                            TriggerType::Sacrificed,
                            RunParams {
                                card: Some(sac_id),
                                player: Some(player),
                                ..Default::default()
                            },
                            false,
                        );
                        {
                            let card = game.card_mut(sac_id);
                            let pt = card.pump_trigger_count;
                            if pt > 0 {
                                let new_len = card.triggers.len().saturating_sub(pt);
                                card.triggers.truncate(new_len);
                                card.pump_trigger_count = 0;
                            }
                        }
                        crate::ability::effects::emit_zone_trigger(
                            &mut self.trigger_handler,
                            sac_id,
                            ZoneType::Battlefield,
                            ZoneType::Graveyard,
                        );
                        self.trigger_handler.flush_waiting_triggers(game);
                        game.move_card(sac_id, ZoneType::Graveyard, sac_owner);
                    }
                }
                cost
            } else {
                mana_cost
            };

            // ── Cost reduction / increase from static abilities ──────────
            let cast_zone = game.card(card_id).zone;
            let cost_adj = crate::cost::cost_adjustment::compute_cost_adjustment(
                game,
                game.card(card_id),
                player,
                cast_zone,
            );
            let raise_cost = crate::cost::cost_adjustment::compute_raise_cost_parts(
                game,
                game.card(card_id),
                player,
                cast_zone,
            );
            let raise_mana = raise_cost
                .as_ref()
                .map(Self::mana_from_cost)
                .unwrap_or_else(|| forge_foundation::ManaCost::generic(0));
            let mana_cost = cost_adj.apply(&mana_cost).add(&raise_mana);

            // ── Additional cost checks (Kicker, Buyback, Multikicker, Replicate) ──
            // Check Kicker: offer to pay additional kicker cost
            let kicked = if let Some(kicker_cost_str) = game.card(card_id).get_kicker_cost() {
                let kicker_mc = forge_foundation::ManaCost::parse(&kicker_cost_str);
                let combined = mana_cost.add(&kicker_mc);
                let available_mana =
                    mana::calculate_available_mana(self.pool(player), game, player);
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
            let buyback_paid = if let Some(buyback_cost_str) = game.card(card_id).get_buyback_cost()
            {
                let buyback_mc = forge_foundation::ManaCost::parse(&buyback_cost_str);
                let combined = mana_cost.add(&buyback_mc);
                let available_mana =
                    mana::calculate_available_mana(self.pool(player), game, player);
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
                let available_mana =
                    mana::calculate_available_mana(self.pool(player), game, player);
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
            let replicate_count =
                if let Some(rep_cost_str) = game.card(card_id).get_replicate_cost() {
                    let rep_mc = forge_foundation::ManaCost::parse(&rep_cost_str);
                    let available_mana =
                        mana::calculate_available_mana(self.pool(player), game, player);
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
            let mana_cost = if let Some(escalate_cost_str) = game.card(card_id).get_escalate_cost()
            {
                let abilities = game.card(card_id).abilities.clone();
                let ability_text = abilities.first().cloned().unwrap_or_default();
                let ability_params = crate::trigger::parse_pipe_params(&ability_text);
                let num_modes = ability_params
                    .get("Choices")
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
                let ability_params = crate::trigger::parse_pipe_params(&ability_text);
                if let Some(choices_str) = ability_params.get("Choices") {
                    let choice_names: Vec<&str> = choices_str.split(',').collect();
                    let svars = game.card(card_id).svars.clone();
                    // Extract ModeCost and description for each mode
                    let mut mode_costs: Vec<forge_foundation::ManaCost> = Vec::new();
                    let mut mode_descriptions: Vec<String> = Vec::new();
                    for name in &choice_names {
                        if let Some(svar_val) = svars.get(*name) {
                            let params = crate::trigger::parse_pipe_params(svar_val);
                            let cost = params
                                .get("ModeCost")
                                .map(|c| forge_foundation::ManaCost::parse(c))
                                .unwrap_or_else(|| forge_foundation::ManaCost::generic(0));
                            let desc = params
                                .get("SpellDescription")
                                .cloned()
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
                    game.card_mut(card_id).chosen_modes = Some(chosen);
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
                let available_mana =
                    mana::calculate_available_mana(self.pool(player), game, player);
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
                    game.card_mut(card_id).strive_extra_targets = extra_targets;
                    total
                } else {
                    mana_cost
                }
            } else {
                mana_cost
            };

            // Check Entwine: pay extra to choose all modes of a modal spell
            let entwine_paid = if let Some(entwine_cost_str) = game.card(card_id).get_entwine_cost()
            {
                let entwine_mc = forge_foundation::ManaCost::parse(&entwine_cost_str);
                let combined = mana_cost.add(&entwine_mc);
                let available_mana =
                    mana::calculate_available_mana(self.pool(player), game, player);
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
            let is_commander_cast =
                game.card(card_id).is_commander && game.card(card_id).zone == ZoneType::Command;
            let mut commander_tax = if is_commander_cast {
                game.card(card_id).commander_cast_count as i32 * 2
            } else {
                0
            };

            // ── X mana cost handling ──────────────────────────────────
            let x_count = mana_cost.count_x();
            let x_value;
            let mana_cost = if x_count > 0 {
                // Compute max X iteratively, mirroring Java's
                // ComputerUtilMana.determineLeftoverMana(): try X=1,2,...
                // until canPayManaCost fails, then return the last payable X.
                // This correctly handles multi-color sources that inflate
                // pool.total() but can only produce one mana per activation.
                let available_mana =
                    mana::calculate_available_mana(self.pool(player), game, player);
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
                // Build effective cost: non-X shards + (X * x_count) generic
                let extra_generic = if agents[player.index()].pay_x_cost_in_mana() {
                    (x_value as i32) * (x_count as i32)
                } else {
                    0
                };
                let mut effective = non_x_cost;
                if extra_generic > 0 {
                    effective = effective.add(&forge_foundation::ManaCost::generic(extra_generic));
                }
                effective
            } else {
                x_value = 0;
                mana_cost
            };

            // ── Phyrexian mana: per-shard greedy payment ──────────────────
            // For each phyrexian shard, try to pay with colored mana first.
            // If no colored mana source is available, pay 2 life instead.
            // This is done shard-by-shard, not all-or-nothing.
            let mana_cost = if mana_cost.has_phyrexian() {
                let available = mana::calculate_available_mana(self.pool(player), game, player);
                let mut remaining_shards: Vec<forge_foundation::ManaCostShard> = Vec::new();
                let mut life_to_pay = 0i32;
                let source_colors = available.source_colors.clone().unwrap_or_default();
                let mut committed = vec![false; source_colors.len()];

                for shard in mana_cost.shards() {
                    if shard.is_phyrexian() {
                        let colored = shard.to_non_phyrexian();
                        let color_mask = (colored.shard()
                            & (forge_foundation::ManaAtom::WHITE
                                | forge_foundation::ManaAtom::BLUE
                                | forge_foundation::ManaAtom::BLACK
                                | forge_foundation::ManaAtom::RED
                                | forge_foundation::ManaAtom::GREEN))
                            as u16;
                        let mut best_idx: Option<usize> = None;
                        let mut best_pop = u32::MAX;
                        for (i, &src) in source_colors.iter().enumerate() {
                            if committed[i] {
                                continue;
                            }
                            if (src & color_mask) != 0 {
                                let pop = src.count_ones();
                                if pop < best_pop {
                                    best_idx = Some(i);
                                    best_pop = pop;
                                }
                            }
                        }
                        if let Some(idx) = best_idx {
                            committed[idx] = true;
                            remaining_shards.push(colored);
                        } else if game.player(player).life >= life_to_pay + 2 {
                            life_to_pay += 2;
                        } else {
                            remaining_shards.push(colored);
                        }
                    } else {
                        remaining_shards.push(*shard);
                    }
                }

                if life_to_pay > 0 {
                    // Run PayLife replacement effects before paying life for shards.
                    let skip_life = {
                        use crate::replacement::replacement_handler::{
                            apply_replacements, ReplacementEvent,
                        };
                        use crate::replacement::ReplacementResult;
                        let mut event = ReplacementEvent::PayLife {
                            player,
                            amount: life_to_pay,
                        };
                        let result = apply_replacements(game, &mut event);
                        result == ReplacementResult::Skipped
                            || result == ReplacementResult::Replaced
                    };
                    if !skip_life {
                        game.player_mut(player).lose_life(life_to_pay);
                        self.trigger_handler.run_trigger(
                            TriggerType::LifeLost,
                            RunParams {
                                player: Some(player),
                                life_amount: Some(life_to_pay),
                                ..Default::default()
                            },
                            false,
                        );
                    }
                }

                forge_foundation::ManaCost::from_parts(remaining_shards, mana_cost.generic_cost())
            } else {
                mana_cost
            };

            // ── Delve: exile graveyard cards to reduce generic cost ──
            let mana_cost = if game.card(card_id).has_keyword("Delve") {
                let generic = mana_cost.generic_cost() + commander_tax;
                if generic > 0 {
                    let gy_cards: Vec<CardId> = game
                        .cards_in_zone(ZoneType::Graveyard, player)
                        .iter()
                        .filter(|&&cid| cid != card_id)
                        .copied()
                        .collect();
                    let max_delve = (generic as usize).min(gy_cards.len());
                    if max_delve > 0 {
                        let card_name = game.card(card_id).card_name.clone();
                        agents[player.index()].snapshot_state(game, &self.mana_pools);
                        let to_exile = agents[player.index()].choose_delve(
                            player,
                            &gy_cards,
                            max_delve,
                            Some(&card_name),
                        );
                        let delve_count = to_exile.len().min(max_delve) as i32;
                        for cid in &to_exile[..delve_count as usize] {
                            game.move_card(*cid, ZoneType::Exile, player);
                        }
                        if delve_count > 0 {
                            // Reduce generic cost (or commander tax first, then generic)
                            let reduce_tax = delve_count.min(commander_tax);
                            commander_tax -= reduce_tax;
                            let reduce_generic = delve_count - reduce_tax;
                            if reduce_generic > 0 {
                                let new_generic =
                                    (mana_cost.generic_cost() - reduce_generic).max(0);
                                mana_cost.with_generic(new_generic)
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
                }
            } else {
                mana_cost
            };

            // ── Convoke: tap creatures to pay colored/generic mana ──
            let mana_cost = if game.card(card_id).has_keyword("Convoke") {
                let untapped_creatures: Vec<CardId> = game
                    .cards_in_zone(ZoneType::Battlefield, player)
                    .iter()
                    .filter(|&&cid| {
                        let c = game.card(cid);
                        c.is_creature() && !c.tapped && cid != card_id
                    })
                    .copied()
                    .collect();
                if !untapped_creatures.is_empty() && mana_cost.cmc() > 0 {
                    let card_name = game.card(card_id).card_name.clone();
                    agents[player.index()].snapshot_state(game, &self.mana_pools);
                    let to_tap = agents[player.index()].choose_convoke(
                        player,
                        &untapped_creatures,
                        &mana_cost,
                        Some(&card_name),
                    );
                    if !to_tap.is_empty() {
                        let mut reduced = mana_cost.clone();
                        for &cid in &to_tap {
                            if !untapped_creatures.contains(&cid) {
                                continue;
                            }
                            // Try to pay a colored shard matching the creature's color first
                            let creature_colors = &game.card(cid).color;
                            let mut paid_colored = false;
                            for color in creature_colors.iter() {
                                let atom = match color {
                                    forge_foundation::Color::White => ManaAtom::WHITE,
                                    forge_foundation::Color::Blue => ManaAtom::BLUE,
                                    forge_foundation::Color::Black => ManaAtom::BLACK,
                                    forge_foundation::Color::Red => ManaAtom::RED,
                                    forge_foundation::Color::Green => ManaAtom::GREEN,
                                };
                                if reduced.has_color_shard(atom) {
                                    reduced = reduced.remove_color_shard(atom);
                                    paid_colored = true;
                                    break;
                                }
                            }
                            if !paid_colored {
                                // Pay generic
                                let g = reduced.generic_cost();
                                if g > 0 {
                                    reduced = reduced.with_generic(g - 1);
                                } else {
                                    continue; // Can't reduce further
                                }
                            }
                            game.tap(cid);
                        }
                        reduced
                    } else {
                        mana_cost
                    }
                } else {
                    mana_cost
                }
            } else {
                mana_cost
            };

            // ── Improvise: tap artifacts to pay generic mana ──
            let mana_cost = if game.card(card_id).has_keyword("Improvise") {
                let untapped_artifacts: Vec<CardId> = game
                    .cards_in_zone(ZoneType::Battlefield, player)
                    .iter()
                    .filter(|&&cid| {
                        let c = game.card(cid);
                        c.type_line.is_artifact() && !c.tapped && cid != card_id
                    })
                    .copied()
                    .collect();
                let generic = mana_cost.generic_cost() + commander_tax;
                if !untapped_artifacts.is_empty() && generic > 0 {
                    let card_name = game.card(card_id).card_name.clone();
                    agents[player.index()].snapshot_state(game, &self.mana_pools);
                    let to_tap = agents[player.index()].choose_improvise(
                        player,
                        &untapped_artifacts,
                        &mana_cost,
                        Some(&card_name),
                    );
                    if !to_tap.is_empty() {
                        let mut reduced = mana_cost.clone();
                        let max_improvise = generic as usize;
                        let mut count = 0usize;
                        for &cid in &to_tap {
                            if count >= max_improvise {
                                break;
                            }
                            if !untapped_artifacts.contains(&cid) {
                                continue;
                            }
                            // Improvise only pays generic
                            let g = reduced.generic_cost() + commander_tax;
                            if g > 0 {
                                if commander_tax > 0 {
                                    commander_tax -= 1;
                                } else {
                                    reduced = reduced.with_generic(reduced.generic_cost() - 1);
                                }
                                game.tap(cid);
                                count += 1;
                            }
                        }
                        reduced
                    } else {
                        mana_cost
                    }
                } else {
                    mana_cost
                }
            } else {
                mana_cost
            };

            // ── Affinity: automatic generic cost reduction based on permanent count ──
            let mana_cost = if let Some(affinity_type) = get_affinity_type(game.card(card_id)) {
                let count = count_affinity_permanents(game, player, &affinity_type, card_id);
                if count > 0 {
                    let generic = mana_cost.generic_cost() + commander_tax;
                    let reduce = count.min(generic);
                    // Reduce commander_tax first, then generic
                    let from_tax = reduce.min(commander_tax);
                    commander_tax -= from_tax;
                    let from_generic = reduce - from_tax;
                    if from_generic > 0 {
                        mana_cost.with_generic(mana_cost.generic_cost() - from_generic)
                    } else {
                        mana_cost
                    }
                } else {
                    mana_cost
                }
            } else {
                mana_cost
            };

            // Build SpellAbility chain and choose modes/targets from the pre-payment
            // game state. This matches Java/MTG casting order: announce modes and
            // targets before paying costs, so mana payments can invalidate a chosen
            // target later (for example, sacrificing a Food token used as a target).
            let mut sa = build_spell_ability(game, card_id, &spell_ability_text, player);
            sa.is_spell = true;

            // Set alternative cost on the SpellAbility
            if is_foretell {
                sa.alt_cost = Some(crate::spellability::AlternativeCost::Foretell);
            } else if is_flashback {
                sa.alt_cost = Some(crate::spellability::AlternativeCost::Flashback);
            } else if is_madness {
                sa.alt_cost = Some(crate::spellability::AlternativeCost::Madness);
            } else if is_spectacle {
                sa.alt_cost = Some(crate::spellability::AlternativeCost::Spectacle);
            } else if is_evoke {
                sa.alt_cost = Some(crate::spellability::AlternativeCost::Evoke);
            } else if is_escape {
                sa.alt_cost = Some(crate::spellability::AlternativeCost::Escape);
            } else if is_overload {
                sa.alt_cost = Some(crate::spellability::AlternativeCost::Overload);
                sa.overloaded = true;
            } else if is_dash {
                sa.alt_cost = Some(crate::spellability::AlternativeCost::Dash);
            } else if is_blitz {
                sa.alt_cost = Some(crate::spellability::AlternativeCost::Blitz);
            } else if is_emerge {
                sa.alt_cost = Some(crate::spellability::AlternativeCost::Emerge);
            } else if is_bestow {
                sa.alt_cost = Some(crate::spellability::AlternativeCost::Bestow);
            } else if is_warp {
                sa.alt_cost = Some(crate::spellability::AlternativeCost::Warp);
            } else if is_morph_facedown {
                let is_mega = game
                    .card(card_id)
                    .keywords
                    .iter()
                    .any(|k| k.starts_with("Megamorph:"));
                sa.alt_cost = Some(if is_mega {
                    crate::spellability::AlternativeCost::Megamorph
                } else {
                    crate::spellability::AlternativeCost::Morph
                });
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
                if sa.api.as_deref() == Some("Charm")
                    && !crate::ability::effects::charm_effect::make_choices_precast(
                        &mut targeting_game,
                        agents,
                        &mut sa,
                    )
                {
                    return None;
                }
                if !sa.setup_targets(&targeting_game, agents, &self.mana_pools) {
                    // Parity with Java harness deterministic cast flow:
                    // handlePlayingSpellAbilityDeterministic() moves spells to stack
                    // before setupTargets(), and a setupTargets() failure is not rolled back.
                    // Net effect is the spell leaves hand without resolving.
                    game.move_card(card_id, ZoneType::Stack, player);
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
                eprintln!(
                    "[RUST-CAST-CHECK] {} meets_must_target={} target={:?}",
                    card_name,
                    meets,
                    sa.target_chosen
                        .target_card
                        .map(|c| (c.0, targeting_game.card(c).card_name.clone()))
                );
                if !meets {
                    eprintln!(
                        "[RUST-MUST-TARGET] Cast rejected for {} — MustTarget restriction not met",
                        card_name
                    );
                    return None;
                }
            }

            // Build mana payment context for restriction checking
            let payment_ctx = {
                let card = game.card(card_id);
                mana::ManaPaymentContext {
                    is_spell: true,
                    type_line: Some(card.type_line.clone()),
                    card_name: Some(card.card_name.clone()),
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
            // Track pool colors before payment for Sunburst/Converge
            let pool_snapshot_for_colors: Vec<u16> = self.pool(player).mana_colors();
            // Track pool size before payment for ManaExpend
            let pool_size_before = self.pool(player).total();

            // ── Mana payment: interactive (human) or auto-tap (AI) ──
            // TODO(cost_payment): Replace this is_human branch with a unified flow via
            // CostPayment + agent.decide_cost_part(). In Java, CostPartMana.payAsDecided()
            // calls player.getController().payManaCost() which is abstract — HumanPlay
            // does the interactive loop, PlayerControllerAi calls ComputerUtilMana.
            // The Rust equivalent should call agent.pay_mana_cost() (interactive) or
            // mana::pay_mana_cost_auto() (auto-tap) based on agent.pays_right_after_decision().
            // See cost/cost_payment.rs for the CostPayment orchestrator skeleton.
            let is_human = agents[player.index()].is_human();
            if is_human {
                // Interactive mana payment loop — mirrors combat cost payment pattern
                let card_name = game.card(card_id).card_name.clone();
                let total_cost = if commander_tax > 0 {
                    mana_cost.add(&forge_foundation::ManaCost::generic(commander_tax))
                } else {
                    mana_cost.clone()
                };
                let cost_str = total_cost.to_string();

                // Save state for refund on cancel (recursive mana refund)
                // Mirrors Java's ManaRefundService: save pool + permanent states
                let saved_pool = self.pool(player).clone();
                let saved_permanent_states: Vec<(
                    CardId,
                    bool,
                    std::collections::BTreeMap<crate::card::CounterType, i32>,
                )> = game
                    .cards_in_zone(ZoneType::Battlefield, player)
                    .iter()
                    .map(|&cid| {
                        let c = game.card(cid);
                        (cid, c.tapped, c.counters.clone())
                    })
                    .collect();

                loop {
                    let tappable_lands: Vec<CardId> = game
                        .cards_in_zone(ZoneType::Battlefield, player)
                        .to_vec()
                        .into_iter()
                        .filter(|&cid| {
                            let c = game.card(cid);
                            !c.tapped
                                && (c.is_land()
                                    || c.activated_abilities.iter().any(|ab| {
                                        ab.is_mana_ability
                                            && crate::cost::can_pay_ignoring_mana(
                                                &ab.cost, game, cid, player,
                                            )
                                    }))
                        })
                        .collect();
                    let pool_snapshot = self.pool(player).clone();
                    let untappable_lands: Vec<CardId> = game
                        .cards_in_zone(ZoneType::Battlefield, player)
                        .to_vec()
                        .into_iter()
                        .filter(|&cid| {
                            let c = game.card(cid);
                            if !c.tapped {
                                return false;
                            }
                            let atoms = mana::land_mana_atoms(c);
                            if !atoms.is_empty() {
                                atoms.iter().any(|&a| pool_snapshot.has_atom(a, 1))
                            } else if let Some(atom) = basic_land_mana_atom(c) {
                                pool_snapshot.has_atom(atom, 1)
                            } else {
                                false
                            }
                        })
                        .collect();
                    let pool_ref = self.pool(player).clone();

                    agents[player.index()].snapshot_state(game, &self.mana_pools);
                    let action = agents[player.index()].pay_mana_cost(
                        player,
                        card_id,
                        &card_name,
                        &cost_str,
                        &tappable_lands,
                        &untappable_lands,
                        &pool_ref,
                    );

                    match action {
                        ManaCostAction::TapLand(land_id) => {
                            if !tappable_lands.contains(&land_id) {
                                continue;
                            }
                            let mana_ab = {
                                let c = game.card(land_id);
                                c.activated_abilities
                                    .iter()
                                    .find(|ab| {
                                        ab.is_mana_ability
                                            && crate::cost::can_pay_ignoring_mana(
                                                &ab.cost, game, land_id, player,
                                            )
                                    })
                                    .cloned()
                            };
                            if let Some(ab) = mana_ab {
                                self.resolve_mana_ability(game, agents, player, land_id, &ab);
                            } else if let Some(atom) = basic_land_mana_atom(game.card(land_id)) {
                                game.tap(land_id);
                                self.pool_mut(player).add(atom, 1);
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
                        ManaCostAction::UntapLand(land_id) => {
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
                                for atom in atoms {
                                    self.pool_mut(player).remove(atom, 1);
                                }
                            }
                        }
                        ManaCostAction::Pay => {
                            // Verify pool can pay the total cost (respecting restrictions)
                            let mut test_pool = self.pool(player).clone();
                            if test_pool.try_pay_for_spell_converted(
                                &mana_cost,
                                &payment_ctx,
                                any_color_conversion,
                            ) && (commander_tax == 0
                                || test_pool.try_pay_extra_generic(commander_tax))
                            {
                                // Actually deduct
                                self.pool_mut(player).try_pay_for_spell_converted(
                                    &mana_cost,
                                    &payment_ctx,
                                    any_color_conversion,
                                );
                                if commander_tax > 0 {
                                    self.pool_mut(player).try_pay_extra_generic(commander_tax);
                                }
                                break;
                            }
                            // Not enough mana yet — stay in loop
                        }
                        ManaCostAction::Cancel => {
                            // Recursive mana refund: restore pool, untap lands, restore counters
                            // Mirrors Java's ManaRefundService two-pass undo
                            *self.pool_mut(player) = saved_pool;
                            for &(cid, was_tapped, ref saved_counters) in &saved_permanent_states {
                                // Restore tapped state
                                if !was_tapped && game.card(cid).tapped {
                                    game.untap(cid);
                                }
                                // Restore counters (undo counter costs from mana abilities)
                                game.card_mut(cid).counters = saved_counters.clone();
                            }
                            return None;
                        }
                    }
                }
            } else {
                // AI deterministic auto-pay: preserve full state so failed payment
                // cannot leave partial taps or partial pool mutations behind.
                let mana_payment_snapshot = self.make_snapshot(game, true);
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
                let tapped = match mana::pay_mana_cost_auto_with_callback(
                    game,
                    self.pool_mut(player),
                    player,
                    &mana_cost,
                    Some(card_id),
                    commander_tax,
                    &payment_ctx,
                    any_color_conversion,
                    &mut callback,
                ) {
                    Some(tapped) => tapped,
                    None => {
                        self.restore_snapshot(game, &mana_payment_snapshot);
                        return None;
                    }
                };

                // Note: Java AutoPay sets express choice on ManaReflected abilities,
                // which restricts chooseColor to 1 option (no RNG consumed).
                // No RNG compensation needed here.

                self.emit_tap_for_mana_triggers(player, &tapped);
            }

            // If uncounterable mana was consumed during payment (Cavern of Souls),
            // add a "can't be countered" replacement effect to the spell's card.
            let uncounterable_after = self.pool(player).count_uncounterable();
            if uncounterable_after < uncounterable_before {
                use crate::replacement::replacement_effect::{
                    ReplacementEffect, ReplacementLayer, ReplacementType,
                };
                let mut params = std::collections::BTreeMap::new();
                params.insert("ValidCard".to_string(), "Card.Self".to_string());
                game.card_mut(card_id)
                    .replacement_effects
                    .push(ReplacementEffect {
                        event: ReplacementType::Counter,
                        layer: ReplacementLayer::CantHappen,
                        params,
                        active_zones: vec![], // active everywhere (including stack)
                    });
            }

            // Track colors of mana spent (Sunburst/Converge)
            {
                let pool_after_colors: Vec<u16> = self.pool(player).mana_colors();
                // Colors consumed = colors in before snapshot but not in after
                let mut colors_spent = 0u16;
                let mut after_clone = pool_after_colors.clone();
                for &color in &pool_snapshot_for_colors {
                    if let Some(pos) = after_clone.iter().position(|&c| c == color) {
                        after_clone.remove(pos); // still in pool, not consumed
                    } else {
                        colors_spent |= color; // consumed
                    }
                }
                game.card_mut(card_id).colors_spent_to_cast = colors_spent;
            }

            // Fire ManaExpend triggers (Expend mechanic — cumulative per-turn tracking)
            {
                let pool_size_after = self.pool(player).total();
                let mana_spent = (pool_size_before - pool_size_after) as i32;
                if mana_spent > 0 {
                    let starting = game.player(player).mana_expended_this_turn;
                    let total = starting + mana_spent;
                    game.player_mut(player).mana_expended_this_turn = total;
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
                                    game.card_mut(card_id).keywords.push(keyword.to_string());
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
                            game.card_mut(card_id).etb_counters_p1p1 += count;
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
                        if let Some(trigger_svar) =
                            game.card(*source_id).svars.get(svar_name).cloned()
                        {
                            let params = crate::trigger::parse_pipe_params(&trigger_svar);
                            // Check ValidCard$ filter against the spell being cast
                            let valid = params
                                .get("ValidCard")
                                .map(String::as_str)
                                .unwrap_or("Card");
                            let card = game.card(card_id);
                            let valid_ok = valid == "Card"
                                || (valid.contains("Creature") && card.is_creature())
                                || (valid.contains("Dragon")
                                    && card.type_line.has_subtype("Dragon"))
                                || (valid.contains("cmcGE6") && card.mana_cost.cmc() >= 6)
                                || (valid.contains("cmcGE5") && card.mana_cost.cmc() >= 5)
                                || (valid.contains("IsCommander") && card.is_commander);
                            if valid_ok {
                                if let Some(execute) = params.get("Execute") {
                                    if let Some(exec_svar) =
                                        game.card(*source_id).svars.get(execute).cloned()
                                    {
                                        let exec_sa = crate::spellability::build_spell_ability(
                                            game, *source_id, &exec_svar, player,
                                        );
                                        crate::ability::effects::resolve_effect(
                                            &mut crate::ability::effects::EffectContext {
                                                game,
                                                agents,
                                                trigger_handler: &mut self.trigger_handler,
                                                token_templates: &self.token_templates,
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
            let spell_cost = Self::parse_spell_cost(&abilities_for_spell);
            if let Some(ref sc) = spell_cost {
                if !self.pay_additional_costs(game, agents, player, card_id, sc, None, sc.mandatory)
                {
                    return None;
                }
            }
            if let Some(ref rc) = raise_cost {
                if !self.pay_additional_costs(game, agents, player, card_id, rc, None, rc.mandatory)
                {
                    return None;
                }
            }

            // Pay additional non-mana costs from Flashback keyword cost
            // (e.g. Lava Dart: Flashback—Sacrifice a Mountain).
            if let Some(ref fb_cost) = flashback_total_cost {
                if !self.pay_additional_costs(
                    game,
                    agents,
                    player,
                    card_id,
                    fb_cost,
                    None,
                    fb_cost.mandatory,
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
                        game.move_card(cid, ZoneType::Exile, player);
                    }
                }
            }

            // Apply GainLife alternative cost side-effect: opponent gains N life.
            if is_gainlife_alt {
                if let Some((life_amount, _)) = game.card(card_id).get_gainlife_alt_cost() {
                    let opp = game.opponent_of(player);
                    game.player_mut(opp).life += life_amount;
                }
            }

            // Apply sacrifice-based alternative cost (e.g. Fireblast: sacrifice two Mountains).
            if is_sacrifice_alt {
                if let Some((amount, type_filter)) = game.card(card_id).get_sacrifice_alt_cost() {
                    self.pay_sacrifice_cost(game, agents, player, &type_filter, amount);
                }
            }

            // Increment commander cast count (before moving card to stack)
            if is_commander_cast {
                game.card_mut(card_id).commander_cast_count += 1;
            }

            game.player_mut(player).spells_cast_this_turn += 1;

            // Emit SpellCast trigger only after successful target setup.
            self.trigger_handler.run_trigger(
                TriggerType::SpellCast,
                RunParams {
                    spell_card: Some(card_id),
                    spell_controller: Some(player),
                    ..Default::default()
                },
                false,
            );

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

            let cast_zone = if is_foretell {
                Some(ZoneType::Exile)
            } else if is_flashback || is_escape {
                Some(ZoneType::Graveyard)
            } else if is_madness || is_plot_cast {
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

            game.stack.push(entry.clone());
            self.log_stack_push(&card_name, &game.player(player).name);
            let chosen_target = entry.spell_ability.target_chosen.target_card;
            if is_flashback {
                let mut event = crate::agent::GameLogEvent::stack(format!(
                    "Cast: {} [Flashback from Graveyard]",
                    card_name
                ))
                .with_player(player)
                .with_source_card(card_id);
                if let Some(target_id) = chosen_target {
                    event = event.with_target_card(target_id);
                }
                crate::agent::notify_all_agents(agents, event);
            } else {
                let mut event = crate::agent::GameLogEvent::stack(format!("Cast: {}", card_name))
                    .with_player(player)
                    .with_source_card(card_id);
                if let Some(target_id) = chosen_target {
                    event = event.with_target_card(target_id);
                }
                crate::agent::notify_all_agents(agents, event);
            }

            // Move spell to stack zone
            game.move_card(card_id, ZoneType::Stack, player);

            // Storm: create N copies where N = spells_cast_this_turn - 1.
            if game.card(card_id).has_storm() {
                let storm_count = game.player(player).spells_cast_this_turn - 1;
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
                        let mut copy = entry.clone();
                        copy.spell_ability.is_copy = true;
                        if copy.spell_ability.uses_targeting() {
                            agents[player.index()].snapshot_state(game, &self.mana_pools);
                            agents[player.index()].notify_event(
                                crate::agent::GameLogEvent::stack(format!(
                                    "Choose target for Storm copy {}/{}",
                                    i + 1,
                                    storm_count
                                ))
                                .with_player(player)
                                .with_card(card_id),
                            );
                            copy.spell_ability
                                .setup_targets(game, agents, &self.mana_pools);
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
                    crate::agent::GameLogEvent::stack(format!(
                        "Replicate: {} copies",
                        replicate_count
                    ))
                    .with_player(player)
                    .with_card(card_id),
                );
                for i in 0..replicate_count {
                    let mut copy = entry.clone();
                    copy.spell_ability.is_copy = true;
                    if copy.spell_ability.uses_targeting() {
                        agents[player.index()].snapshot_state(game, &self.mana_pools);
                        agents[player.index()].notify_event(
                            crate::agent::GameLogEvent::stack(format!(
                                "Choose target for Replicate copy {}/{}",
                                i + 1,
                                replicate_count
                            ))
                            .with_player(player)
                            .with_card(card_id),
                        );
                        copy.spell_ability
                            .setup_targets(game, agents, &self.mana_pools);
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
                .iter()
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
            game.move_card(top_id, ZoneType::Exile, player);
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
                let abilities = card.abilities.clone();

                let ability_text = abilities.first().cloned().unwrap_or_default();
                let mut sa = build_spell_ability(game, cascade_card_id, &ability_text, player);
                sa.is_spell = true;

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
                game.move_card(cascade_card_id, ZoneType::Stack, player);

                // Cascade spell counts as being cast
                game.player_mut(player).spells_cast_this_turn += 1;
                self.trigger_handler.run_trigger(
                    TriggerType::SpellCast,
                    RunParams {
                        spell_card: Some(cascade_card_id),
                        spell_controller: Some(player),
                        ..Default::default()
                    },
                    false,
                );
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
