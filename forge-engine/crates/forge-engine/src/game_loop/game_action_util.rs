use super::*;

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

    /// Get cards the active player can play.
    pub(crate) fn get_playable_cards(
        &self,
        game: &GameState,
        player: PlayerId,
        must_be_instant: bool,
    ) -> Vec<CardId> {
        let mut playable = Vec::new();
        let hand = game.cards_in_zone(ZoneType::Hand, player);

        // Check Command zone for commanders (with commander tax)
        let command_zone: Vec<CardId> = game.cards_in_zone(ZoneType::Command, player).to_vec();

        for card_id in command_zone {
            let card = game.card(card_id);
            if card.is_commander {
                if must_be_instant && !card.has_keyword("Flash") && !card.type_line.is_instant() {
                    continue;
                }
                let tax = card.commander_cast_count as i32 * 2;
                let available_mana =
                    mana::calculate_available_mana(self.pool(player), game, player);
                if available_mana.can_pay_with_extra_generic(&card.mana_cost, tax) {
                    playable.push(card_id);
                }
            }
        }

        // Check Graveyard for Flashback and Escape cards
        let graveyard: Vec<CardId> = game.cards_in_zone(ZoneType::Graveyard, player).to_vec();
        for card_id in graveyard.iter().copied() {
            let card = game.card(card_id);
            let is_instant = card.type_line.is_instant() || card.has_keyword("Flash");
            if must_be_instant && !is_instant {
                continue;
            }

            // Flashback
            if let Some(fb_cost_str) = card.get_flashback_cost() {
                let fb_cost = forge_foundation::ManaCost::parse(&fb_cost_str);
                let available_mana =
                    mana::calculate_available_mana(self.pool(player), game, player);
                if available_mana.can_pay(&fb_cost) {
                    playable.push(card_id);
                    continue;
                }
            }

            // Escape: cast from graveyard with escape cost + exiling other graveyard cards
            if let Some((escape_mana_str, exile_count)) = card.get_escape_cost() {
                let escape_mc = forge_foundation::ManaCost::parse(&escape_mana_str);
                let available_mana =
                    mana::calculate_available_mana(self.pool(player), game, player);
                let other_gy_count = graveyard.iter().filter(|&&cid| cid != card_id).count() as i32;
                if available_mana.can_pay(&escape_mc) && other_gy_count >= exile_count {
                    playable.push(card_id);
                }
            }
        }

        // Check Exile for Madness cards (discarded with madness go to exile)
        let exile: Vec<CardId> = game.cards_in_zone(ZoneType::Exile, player).to_vec();
        for card_id in exile {
            let card = game.card(card_id);
            if let Some(madness_cost_str) = card.get_madness_cost() {
                let is_instant = card.type_line.is_instant() || card.has_keyword("Flash");
                if must_be_instant && !is_instant {
                    continue;
                }
                let madness_mc = forge_foundation::ManaCost::parse(&madness_cost_str);
                let available_mana =
                    mana::calculate_available_mana(self.pool(player), game, player);
                if available_mana.can_pay(&madness_mc) {
                    playable.push(card_id);
                }
            }
        }

        for &card_id in hand {
            let card = game.card(card_id);
            if card.is_land() {
                if !must_be_instant && game.player(player).can_play_land() {
                    playable.push(card_id);
                }
            } else {
                let is_instant = card.type_line.is_instant() || card.has_keyword("Flash");
                if must_be_instant && !is_instant {
                    continue;
                }

                // Check if we can pay the mana cost (normal or alternative)
                let available_mana =
                    mana::calculate_available_mana(self.pool(player), game, player);

                // Check normal cost OR any alternative costs
                let normal_ok = available_mana.can_pay(&card.mana_cost);

                // Spectacle: alt cost if opponent lost life this turn
                let spectacle_ok = if let Some(spec_cost_str) = card.get_spectacle_cost() {
                    let opp = game.opponent_of(player);
                    game.player(opp).life_lost_this_turn > 0
                        && available_mana
                            .can_pay(&forge_foundation::ManaCost::parse(&spec_cost_str))
                } else {
                    false
                };

                // Evoke: alt cost for creatures
                let evoke_ok = if let Some(evoke_cost_str) = card.get_evoke_cost() {
                    available_mana.can_pay(&forge_foundation::ManaCost::parse(&evoke_cost_str))
                } else {
                    false
                };

                // Dash: alt cost
                let dash_ok = if let Some(dash_cost_str) = card.get_dash_cost() {
                    available_mana.can_pay(&forge_foundation::ManaCost::parse(&dash_cost_str))
                } else {
                    false
                };

                // Blitz: alt cost
                let blitz_ok = if let Some(blitz_cost_str) = card.get_blitz_cost() {
                    available_mana.can_pay(&forge_foundation::ManaCost::parse(&blitz_cost_str))
                } else {
                    false
                };

                // Overload: alt cost
                let overload_ok = if let Some(ovl_cost_str) = card.get_overload_cost() {
                    available_mana.can_pay(&forge_foundation::ManaCost::parse(&ovl_cost_str))
                } else {
                    false
                };

                // Suspend: special action, pay suspend cost to exile with time counters
                let suspend_ok = if let Some((suspend_cost_str, _counters)) =
                    card.get_suspend_cost()
                {
                    available_mana.can_pay(&forge_foundation::ManaCost::parse(&suspend_cost_str))
                } else {
                    false
                };

                // Foretell: pay {2} to exile face-down from hand
                let foretell_exile_ok = if card.get_foretell_cost().is_some() {
                    available_mana.can_pay(&forge_foundation::ManaCost::generic(2))
                } else {
                    false
                };

                // Emerge: alt cost minus sacrificed creature's mana value
                let emerge_ok = if let Some(emerge_cost_str) = card.get_emerge_cost() {
                    // Simplified: check if emerge base cost is affordable
                    // (actual cost reduction from sac'd creature computed at cast time)
                    available_mana.can_pay(&forge_foundation::ManaCost::parse(&emerge_cost_str))
                        || {
                            // Even if base emerge cost isn't payable, if we have creatures to sac
                            // the reduction might make it payable — approximate check
                            !game
                                .cards_in_zone(ZoneType::Battlefield, player)
                                .iter()
                                .filter(|&&cid| game.card(cid).is_creature())
                                .collect::<Vec<_>>()
                                .is_empty()
                        }
                } else {
                    false
                };

                if !normal_ok
                    && !spectacle_ok
                    && !evoke_ok
                    && !dash_ok
                    && !blitz_ok
                    && !overload_ok
                    && !suspend_ok
                    && !foretell_exile_ok
                    && !emerge_ok
                {
                    continue;
                }

                // Check additional costs from SP$ line (e.g. Sac<1/Creature>).
                let spell_cost = Self::parse_spell_cost(&card.abilities);
                let additional_costs_ok = if let Some(ref sc) = spell_cost {
                    sc.parts.iter().all(|part| match part {
                        CostPart::Sacrifice {
                            type_filter,
                            amount,
                        } => {
                            if type_filter == "CARDNAME" {
                                true
                            } else {
                                let targets =
                                    cost::get_sacrifice_targets(game, player, type_filter);
                                (targets.len() as i32) >= *amount
                            }
                        }
                        CostPart::PayLife(life) => game.player(player).life >= *life,
                        _ => true,
                    })
                } else {
                    true
                };

                if additional_costs_ok {
                    let all_valid = card.abilities.iter().all(|ab| {
                        target_restrictions::has_candidates_in_chain(
                            game,
                            player,
                            ab,
                            Some(card_id),
                        )
                    });
                    if all_valid {
                        playable.push(card_id);
                    }
                }
            }
        }

        // Check graveyard for Escape cards
        let graveyard: Vec<CardId> = game.cards_in_zone(ZoneType::Graveyard, player).to_vec();
        for card_id in graveyard {
            let card = game.card(card_id);
            if let Some((escape_mana, exile_count)) = card.get_escape_cost() {
                let is_instant = card.type_line.is_instant() || card.has_keyword("Flash");
                if must_be_instant && !is_instant {
                    continue;
                }
                let available_mana =
                    mana::calculate_available_mana(self.pool(player), game, player);
                if available_mana.can_pay(&forge_foundation::ManaCost::parse(&escape_mana)) {
                    let other_gy_count = game
                        .cards_in_zone(ZoneType::Graveyard, player)
                        .iter()
                        .filter(|&&cid| cid != card_id)
                        .count() as i32;
                    if other_gy_count >= exile_count {
                        playable.push(card_id);
                    }
                }
            }
        }

        // Check exile for Foretold cards (face-down in exile with foretell cost)
        // and Madness cards (exiled via discard with MadnessExiled marker)
        let exile: Vec<CardId> = game.cards_in_zone(ZoneType::Exile, player).to_vec();
        for card_id in exile {
            let card = game.card(card_id);
            if card.face_down {
                if let Some(foretell_cost_str) = card.get_foretell_cost() {
                    let is_instant = card.type_line.is_instant() || card.has_keyword("Flash");
                    if must_be_instant && !is_instant {
                        continue;
                    }
                    let available_mana =
                        mana::calculate_available_mana(self.pool(player), game, player);
                    if available_mana
                        .can_pay(&forge_foundation::ManaCost::parse(&foretell_cost_str))
                    {
                        playable.push(card_id);
                    }
                }
            } else if card.has_keyword("MadnessExiled") {
                // Madness: exiled card that can be cast for madness cost
                if let Some(madness_cost_str) = card.get_madness_cost() {
                    let is_instant = card.type_line.is_instant() || card.has_keyword("Flash");
                    if must_be_instant && !is_instant {
                        continue;
                    }
                    let available_mana =
                        mana::calculate_available_mana(self.pool(player), game, player);
                    if available_mana.can_pay(&forge_foundation::ManaCost::parse(&madness_cost_str))
                    {
                        playable.push(card_id);
                    }
                }
            }
        }

        playable
    }

    /// Play a card from hand (or graveyard for Escape). Returns the (card_id, card_name)
    /// if the card was successfully played, so the caller can emit the notification.
    pub(crate) fn play_card(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        card_id: CardId,
    ) -> Option<(CardId, String)> {
        let card = game.card(card_id);
        let card_name = card.card_name.clone();

        if card.is_land() {
            // Check for shock-land-style "pay life or enter tapped" before entering
            let etb_life_cost =
                crate::staticability::layer::get_etb_unless_life_cost(game.card(card_id));

            // Play land — goes directly to battlefield
            game.move_card(card_id, ZoneType::Battlefield, player);

            // Prompt for shock land life payment (after ETB so the card is on battlefield)
            if let Some(life_cost) = etb_life_cost {
                let desc = format!("Pay {} life so {} enters untapped?", life_cost, card_name);
                agents[player.index()].snapshot_state(game, &self.mana_pools);
                let pay =
                    agents[player.index()].choose_optional_trigger(player, &desc, Some(&card_name));
                if pay {
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
                } else {
                    // Player declines — enter tapped
                    game.card_mut(card_id).tapped = true;
                }
            }

            game.player_mut(player).lands_played_this_turn += 1;
            agents[player.index()].notify(&format!("Played land: {}", card_name));

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

            // ── Alternative cost detection ──────────────────────────────

            // Detect Foretell: casting from exile (face-down) with foretell cost
            let is_foretell = game.card(card_id).zone == ZoneType::Exile
                && game.card(card_id).face_down
                && game.card(card_id).get_foretell_cost().is_some();

            // Detect Flashback: casting from graveyard with flashback cost
            let is_flashback = !is_foretell
                && game.card(card_id).zone == ZoneType::Graveyard
                && game.card(card_id).get_flashback_cost().is_some();

            // Detect Spectacle: alternative cost if opponent lost life this turn
            let is_spectacle = if !is_flashback && !is_foretell {
                if let Some(_spec_cost_str) = game.card(card_id).get_spectacle_cost() {
                    let opponent_lost_life = game
                        .player_order
                        .iter()
                        .filter(|&&pid| pid != player)
                        .any(|&pid| game.player(pid).life_lost_this_turn > 0);
                    if opponent_lost_life {
                        let spec_cost_str = game.card(card_id).get_spectacle_cost().unwrap();
                        let spec_mc = forge_foundation::ManaCost::parse(&spec_cost_str);
                        let available_mana =
                            mana::calculate_available_mana(self.pool(player), game, player);
                        if available_mana.can_pay(&spec_mc) {
                            // Offer choice: spectacle is cheaper, auto-pick it
                            true
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            };

            // Detect Evoke: alternative cost for creatures
            let is_evoke = if !is_flashback && !is_spectacle && !is_foretell {
                if let Some(evoke_cost_str) = game.card(card_id).get_evoke_cost() {
                    let evoke_mc = forge_foundation::ManaCost::parse(&evoke_cost_str);
                    let normal_mc = &game.card(card_id).mana_cost;
                    let available_mana =
                        mana::calculate_available_mana(self.pool(player), game, player);
                    // Offer evoke if: can afford evoke but NOT normal cost, or player chooses it
                    if available_mana.can_pay(&evoke_mc) && !available_mana.can_pay(normal_mc) {
                        true // auto-evoke when can't afford normal cost
                    } else if available_mana.can_pay(&evoke_mc) && available_mana.can_pay(normal_mc)
                    {
                        // Both affordable — offer choice via alternative cost prompt
                        let name = game.card(card_id).card_name.clone();
                        let options = vec![
                            format!("Normal cost: {}", normal_mc),
                            format!("Evoke: {} (sacrifice on ETB)", evoke_cost_str),
                        ];
                        agents[player.index()].snapshot_state(game, &self.mana_pools);
                        let choice = agents[player.index()].choose_alternative_cost(
                            player,
                            &options,
                            Some(&name),
                        );
                        choice == 1
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            };

            // Detect Escape: casting from graveyard with escape cost + exiling graveyard cards
            let is_escape = if !is_flashback && !is_foretell {
                if game.card(card_id).zone == ZoneType::Graveyard {
                    if let Some((escape_mana_str, exile_count)) =
                        game.card(card_id).get_escape_cost()
                    {
                        let escape_mc = forge_foundation::ManaCost::parse(&escape_mana_str);
                        let available_mana =
                            mana::calculate_available_mana(self.pool(player), game, player);
                        // Count other cards in graveyard that can be exiled
                        let other_gy_count = game
                            .cards_in_zone(ZoneType::Graveyard, player)
                            .iter()
                            .filter(|&&cid| cid != card_id)
                            .count() as i32;
                        available_mana.can_pay(&escape_mc) && other_gy_count >= exile_count
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            };

            // Detect Overload: alternative cost that changes "target" to "each"
            let is_overload =
                if !is_flashback && !is_spectacle && !is_evoke && !is_escape && !is_foretell {
                    if let Some(overload_cost_str) = game.card(card_id).get_overload_cost() {
                        let overload_mc = forge_foundation::ManaCost::parse(&overload_cost_str);
                        let normal_mc = &game.card(card_id).mana_cost;
                        let available_mana =
                            mana::calculate_available_mana(self.pool(player), game, player);
                        if available_mana.can_pay(&overload_mc) {
                            if available_mana.can_pay(normal_mc) {
                                // Both affordable — offer choice
                                let name = game.card(card_id).card_name.clone();
                                let options = vec![
                                    format!("Normal cost: {}", normal_mc),
                                    format!("Overload: {} (affects all)", overload_cost_str),
                                ];
                                agents[player.index()].snapshot_state(game, &self.mana_pools);
                                let choice = agents[player.index()].choose_alternative_cost(
                                    player,
                                    &options,
                                    Some(&name),
                                );
                                choice == 1
                            } else {
                                true // Can only afford overload cost — auto-select it
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                };

            // Detect Dash: alternative cost, creature gains haste, returns at EOT
            let is_dash = if !is_flashback
                && !is_spectacle
                && !is_evoke
                && !is_escape
                && !is_overload
                && !is_foretell
            {
                if let Some(dash_cost_str) = game.card(card_id).get_dash_cost() {
                    let dash_mc = forge_foundation::ManaCost::parse(&dash_cost_str);
                    let normal_mc = &game.card(card_id).mana_cost;
                    let available_mana =
                        mana::calculate_available_mana(self.pool(player), game, player);
                    if available_mana.can_pay(&dash_mc) {
                        if !available_mana.can_pay(normal_mc) {
                            true // can only afford dash
                        } else {
                            let name = game.card(card_id).card_name.clone();
                            let options = vec![
                                format!("Normal cost: {}", normal_mc),
                                format!("Dash: {} (haste, return at EOT)", dash_cost_str),
                            ];
                            agents[player.index()].snapshot_state(game, &self.mana_pools);
                            let choice = agents[player.index()].choose_alternative_cost(
                                player,
                                &options,
                                Some(&name),
                            );
                            choice == 1
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            };

            // Detect Blitz: alternative cost, haste + "dies: draw" + sacrifice at EOT
            let is_blitz = if !is_flashback
                && !is_spectacle
                && !is_evoke
                && !is_escape
                && !is_overload
                && !is_dash
                && !is_foretell
            {
                if let Some(blitz_cost_str) = game.card(card_id).get_blitz_cost() {
                    let blitz_mc = forge_foundation::ManaCost::parse(&blitz_cost_str);
                    let normal_mc = &game.card(card_id).mana_cost;
                    let available_mana =
                        mana::calculate_available_mana(self.pool(player), game, player);
                    if available_mana.can_pay(&blitz_mc) {
                        if !available_mana.can_pay(normal_mc) {
                            true
                        } else {
                            let name = game.card(card_id).card_name.clone();
                            let options = vec![
                                format!("Normal cost: {}", normal_mc),
                                format!(
                                    "Blitz: {} (haste, draw on death, sac at EOT)",
                                    blitz_cost_str
                                ),
                            ];
                            agents[player.index()].snapshot_state(game, &self.mana_pools);
                            let choice = agents[player.index()].choose_alternative_cost(
                                player,
                                &options,
                                Some(&name),
                            );
                            choice == 1
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            };

            // Detect Madness: casting from exile with madness cost
            let is_madness = !is_foretell
                && game.card(card_id).zone == ZoneType::Exile
                && game.card(card_id).get_madness_cost().is_some();

            // Detect Emerge: alternative cost (sacrifice creature to reduce cost)
            let is_emerge = if !is_flashback
                && !is_foretell
                && !is_spectacle
                && !is_evoke
                && !is_escape
                && !is_overload
                && !is_dash
                && !is_blitz
                && !is_madness
            {
                game.card(card_id).get_emerge_cost().is_some()
            } else {
                false
            };

            // ── Foretell exile: special action, not a cast ────────────
            // If foretell card is in hand (not being cast from exile), offer to exile face-down for {2}.
            if !is_foretell
                && game.card(card_id).get_foretell_cost().is_some()
                && game.card(card_id).zone == ZoneType::Hand
            {
                let available_mana =
                    mana::calculate_available_mana(self.pool(player), game, player);
                let foretell_exile_cost = forge_foundation::ManaCost::generic(2);
                if available_mana.can_pay(&foretell_exile_cost) {
                    let name = game.card(card_id).card_name.clone();
                    let options = vec![
                        "Cast normally".to_string(),
                        "Foretell (exile face-down for {2})".to_string(),
                    ];
                    agents[player.index()].snapshot_state(game, &self.mana_pools);
                    let choice = agents[player.index()].choose_alternative_cost(
                        player,
                        &options,
                        Some(&name),
                    );
                    if choice == 1 {
                        // Pay {2}
                        let tapped = mana::auto_tap_lands(
                            game,
                            self.pool_mut(player),
                            player,
                            &foretell_exile_cost,
                        );
                        self.emit_tap_for_mana_triggers(player, &tapped);
                        self.pool_mut(player).try_pay(&foretell_exile_cost);
                        // Exile face-down
                        game.card_mut(card_id).face_down = true;
                        game.move_card(card_id, ZoneType::Exile, player);
                        agents[player.index()].notify(&format!("Foretold: {}", card_name));
                        return Some((card_id, card_name));
                    }
                }
            }

            // ── Suspend: special action, exile with time counters ────────
            if let Some((suspend_cost, counters)) = game.card(card_id).get_suspend_cost() {
                if game.card(card_id).zone == ZoneType::Hand {
                    let available_mana =
                        mana::calculate_available_mana(self.pool(player), game, player);
                    let suspend_mc = forge_foundation::ManaCost::parse(&suspend_cost);
                    if available_mana.can_pay(&suspend_mc) {
                        let name = game.card(card_id).card_name.clone();
                        let options = vec![
                            "Cast normally".to_string(),
                            format!("Suspend ({}, {} time counters)", suspend_cost, counters),
                        ];
                        agents[player.index()].snapshot_state(game, &self.mana_pools);
                        let choice = agents[player.index()].choose_alternative_cost(
                            player,
                            &options,
                            Some(&name),
                        );
                        if choice == 1 {
                            let tapped = mana::auto_tap_lands(
                                game,
                                self.pool_mut(player),
                                player,
                                &suspend_mc,
                            );
                            self.emit_tap_for_mana_triggers(player, &tapped);
                            self.pool_mut(player).try_pay(&suspend_mc);
                            game.move_card(card_id, ZoneType::Exile, player);
                            game.card_mut(card_id)
                                .add_counter(crate::card::CounterType::Time, counters);
                            agents[player.index()].notify(&format!(
                                "Suspended: {} with {} time counters",
                                card_name, counters
                            ));
                            return Some((card_id, card_name));
                        }
                    }
                }
            }

            // Determine the mana cost to use
            let mana_cost = if is_foretell {
                let foretell_cost_str = game.card(card_id).get_foretell_cost().unwrap();
                game.card_mut(card_id).face_down = false; // reveal it
                forge_foundation::ManaCost::parse(&foretell_cost_str)
            } else if is_flashback {
                let fb_cost_str = game.card(card_id).get_flashback_cost().unwrap();
                forge_foundation::ManaCost::parse(&fb_cost_str)
            } else if is_madness {
                let madness_cost_str = game.card(card_id).get_madness_cost().unwrap();
                // Remove the MadnessExiled marker
                game.card_mut(card_id)
                    .granted_keywords
                    .retain(|k| k != "MadnessExiled");
                forge_foundation::ManaCost::parse(&madness_cost_str)
            } else if is_spectacle {
                let spec_cost_str = game.card(card_id).get_spectacle_cost().unwrap();
                forge_foundation::ManaCost::parse(&spec_cost_str)
            } else if is_evoke {
                let evoke_cost_str = game.card(card_id).get_evoke_cost().unwrap();
                forge_foundation::ManaCost::parse(&evoke_cost_str)
            } else if is_escape {
                let (escape_mana_str, _) = game.card(card_id).get_escape_cost().unwrap();
                forge_foundation::ManaCost::parse(&escape_mana_str)
            } else if is_overload {
                let overload_cost_str = game.card(card_id).get_overload_cost().unwrap();
                forge_foundation::ManaCost::parse(&overload_cost_str)
            } else if is_dash {
                let dash_cost_str = game.card(card_id).get_dash_cost().unwrap();
                forge_foundation::ManaCost::parse(&dash_cost_str)
            } else if is_blitz {
                let blitz_cost_str = game.card(card_id).get_blitz_cost().unwrap();
                forge_foundation::ManaCost::parse(&blitz_cost_str)
            } else if is_emerge {
                let emerge_cost_str = game.card(card_id).get_emerge_cost().unwrap();
                forge_foundation::ManaCost::parse(&emerge_cost_str)
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
                        game.move_card(sac_id, ZoneType::Graveyard, sac_owner);
                        crate::ability::effects::emit_zone_trigger(
                            &mut self.trigger_handler,
                            sac_id,
                            ZoneType::Battlefield,
                            ZoneType::Graveyard,
                        );
                    }
                }
                cost
            } else {
                mana_cost
            };

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
                let kicker_cost_str = game.card(card_id).get_kicker_cost().unwrap();
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
                let buyback_cost_str = game.card(card_id).get_buyback_cost().unwrap();
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
                let mk_cost_str = game.card(card_id).get_multikicker_cost().unwrap();
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
                let rep_cost_str = game.card(card_id).get_replicate_cost().unwrap();
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
                let entwine_cost_str = game.card(card_id).get_entwine_cost().unwrap();
                let entwine_mc = forge_foundation::ManaCost::parse(&entwine_cost_str);
                mana_cost.add(&entwine_mc)
            } else {
                mana_cost
            };

            // Detect commander cast from Command zone (for commander tax)
            let is_commander_cast =
                game.card(card_id).is_commander && game.card(card_id).zone == ZoneType::Command;
            let commander_tax = if is_commander_cast {
                game.card(card_id).commander_cast_count as i32 * 2
            } else {
                0
            };

            // Auto-tap lands to pay the effective cost
            let tapped = mana::auto_tap_lands(game, self.pool_mut(player), player, &mana_cost);
            self.emit_tap_for_mana_triggers(player, &tapped);

            // Auto-tap extra lands for commander tax
            if commander_tax > 0 {
                let tapped_tax = mana::auto_tap_lands_generic(
                    game,
                    self.pool_mut(player),
                    player,
                    commander_tax,
                );
                self.emit_tap_for_mana_triggers(player, &tapped_tax);
            }

            let abilities = game.card(card_id).abilities.clone();

            // Pay the mana cost from pool
            let paid = self.pool_mut(player).try_pay(&mana_cost);
            if !paid {
                return None;
            }

            // Pay commander tax
            if commander_tax > 0 && !self.pool_mut(player).try_pay_extra_generic(commander_tax) {
                return None;
            }

            // Pay additional costs from SP$ line (e.g. sacrifice a creature).
            let spell_cost = Self::parse_spell_cost(&abilities);
            if let Some(ref sc) = spell_cost {
                self.pay_additional_costs(game, agents, player, card_id, sc);
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

            // Increment commander cast count (before moving card to stack)
            if is_commander_cast {
                game.card_mut(card_id).commander_cast_count += 1;
            }

            game.player_mut(player).spells_cast_this_turn += 1;

            // Emit SpellCast trigger
            self.trigger_handler.run_trigger(
                TriggerType::SpellCast,
                RunParams {
                    spell_card: Some(card_id),
                    spell_controller: Some(player),
                    ..Default::default()
                },
                false,
            );

            // Build SpellAbility chain and choose targets.
            let ability_text = abilities.first().cloned().unwrap_or_default();
            let mut sa = build_spell_ability(game, card_id, &ability_text, player);
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
            }

            // Set kicked flag on the SpellAbility (also set for entwine -- charm_effect checks sa.kicked)
            if kicked || kick_count > 0 || entwine_paid {
                sa.kicked = true;
            }

            // Set additional cost flags
            sa.buyback_paid = buyback_paid;
            sa.kick_count = kick_count;
            sa.replicate_count = replicate_count;

            // Overloaded spells replace "target" with "each" -- skip targeting.
            if !sa.overloaded {
                sa.setup_targets(game, agents, &self.mana_pools);
            }

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

            let cast_zone = if is_foretell {
                Some(ZoneType::Exile)
            } else if is_flashback || is_escape {
                Some(ZoneType::Graveyard)
            } else if is_madness {
                Some(ZoneType::Exile)
            } else if is_commander_cast {
                Some(ZoneType::Command)
            } else {
                Some(ZoneType::Hand)
            };

            let entry = StackEntry {
                id: 0,
                spell_ability: sa,
                is_creature_spell: is_creature,
                is_permanent_spell: is_permanent,
                cast_from_zone: cast_zone,
            };

            game.stack.push(entry.clone());
            self.log_stack_push(&card_name, &game.player(player).name);
            agents[player.index()].notify(&format!("Cast: {}", card_name));

            // Move spell to stack zone
            game.move_card(card_id, ZoneType::Stack, player);

            // Storm: create N copies where N = spells_cast_this_turn - 1.
            if game.card(card_id).has_storm() {
                let storm_count = game.player(player).spells_cast_this_turn - 1;
                if storm_count > 0 {
                    agents[player.index()].notify(&format!("Storm count: {} copies", storm_count));
                    for i in 0..storm_count {
                        let mut copy = entry.clone();
                        copy.spell_ability.is_copy = true;
                        if copy.spell_ability.uses_targeting() {
                            agents[player.index()].snapshot_state(game, &self.mana_pools);
                            agents[player.index()].notify(&format!(
                                "Choose target for Storm copy {}/{}",
                                i + 1,
                                storm_count
                            ));
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
                agents[player.index()].notify(&format!("Replicate: {} copies", replicate_count));
                for i in 0..replicate_count {
                    let mut copy = entry.clone();
                    copy.spell_ability.is_copy = true;
                    if copy.spell_ability.uses_targeting() {
                        agents[player.index()].snapshot_state(game, &self.mana_pools);
                        agents[player.index()].notify(&format!(
                            "Choose target for Replicate copy {}/{}",
                            i + 1,
                            replicate_count
                        ));
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
            let top_id = *lib.last().unwrap(); // last = top of library
            game.move_card(top_id, ZoneType::Exile, player);
            let card = game.card(top_id);
            let is_land = card.is_land();
            let mv = card.mana_value();

            if !is_land && mv < caster_mv {
                found_card = Some(top_id);
                agents[player.index()].notify(&format!("Cascade found: {}", card.card_name));
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
                };
                game.stack.push(entry);
                self.log_stack_push(&card_name, &game.player(player).name);
                agents[player.index()].notify(&format!("Cascade cast: {}", card_name));
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
        use rand::seq::SliceRandom;
        let mut rng = rand::thread_rng();
        exiled_ids.shuffle(&mut rng);
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
