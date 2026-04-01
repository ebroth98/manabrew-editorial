use super::*;

use crate::cost::cost_adjustment::apply_cost_reductions;

impl GameLoop {
    pub(super) fn mana_from_cost(cost: &crate::cost::Cost) -> forge_foundation::ManaCost {
        let mut out = forge_foundation::ManaCost::generic(0);
        for part in &cost.parts {
            if let CostPart::Mana { cost: mc, .. } = part {
                out = out.add(mc);
            }
        }
        out
    }

    /// Get cards the active player can play.
    pub(crate) fn get_playable_cards(
        &self,
        game: &GameState,
        player: PlayerId,
        must_be_instant: bool,
    ) -> Vec<crate::agent::PlayOption> {
        let mut playable = Vec::new();
        let hand = game.cards_in_zone(ZoneType::Hand, player);
        let has_flash_permission = |card_id: CardId| {
            let card = game.card(card_id);
            card.type_line.is_instant()
                || card.has_keyword("Flash")
                || card.get_offering_type().is_some()
                || crate::staticability::static_ability_cast_with_flash::any_with_flash(
                    &game.cards,
                    card,
                    player,
                    &card.abilities,
                )
        };

        for &card_id in hand {
            let card = game.card(card_id);
            if card.is_land() {
                if crate::staticability::static_ability_cant_be_cast::cant_play_land_ability(
                    &game.cards,
                    card,
                    player,
                ) {
                    continue;
                }
                // Delegate to land_ability module for timing/land-play checks
                let land_sa = SpellAbility::new_land(Some(card_id), player);
                if !must_be_instant && crate::spellability::land_ability::can_play(&land_sa, game) {
                    playable.push(crate::agent::PlayOption {
                        card_id,
                        mode: crate::agent::PlayCardMode::Normal,
                    });
                }
            } else {
                let cast_sa =
                    crate::spellability::build_spell_ability_for_card_cast(game, card_id, player);
                if crate::staticability::static_ability_cant_be_cast::cant_be_cast_ability_in_context(
                    &game.cards,
                    &cast_sa,
                    card,
                    player,
                    Some(game),
                ) {
                    continue;
                }

                if must_be_instant && !has_flash_permission(card_id) {
                    continue;
                }

                // Spell-level checks: not on battlefield, no split second
                if !crate::spellability::spell::can_play(&cast_sa, game) {
                    continue;
                }

                // Aura targeting check: don't show auras as playable if no valid target exists.
                if card.type_line.has_subtype("Aura") {
                    if let Some(ref tr) = cast_sa.target_restrictions {
                        if !tr.has_candidates(game, player, Some(card_id)) {
                            continue;
                        }
                    }
                }

                // Check if we can pay the mana cost (normal or alternative)
                let available_mana =
                    mana::calculate_available_mana(self.pool(player), game, player);

                // Apply cost reduction/increase from static abilities
                let cost_adj = crate::cost::cost_adjustment::compute_cost_adjustment(
                    game,
                    card,
                    player,
                    ZoneType::Hand,
                );
                let raise_cost = crate::cost::cost_adjustment::compute_raise_cost_parts(
                    game,
                    card,
                    player,
                    ZoneType::Hand,
                );
                let raise_mana = raise_cost
                    .as_ref()
                    .map(Self::mana_from_cost)
                    .unwrap_or_else(|| forge_foundation::ManaCost::generic(0));

                // Check mana conversion for playability
                let any_color =
                    crate::staticability::static_ability_mana_convert::can_spend_mana_as_any_color(
                        &game.cards,
                        player,
                        card,
                    );

                // Check normal cost OR any alternative costs
                // For X-cost spells, check only the non-X portion (X=0 is valid)
                // Delve: reduce generic cost by number of graveyard cards
                // Convoke: reduce total cost by number of untapped creatures
                let normal_ok = {
                    let base = if card.mana_cost.count_x() > 0 {
                        cost_adj.apply(&card.mana_cost.without_x())
                    } else {
                        cost_adj.apply(&card.mana_cost)
                    };
                    let base = base.add(&raise_mana);
                    // Phyrexian mana: check AIPhyrexianPayment to determine
                    // if life payment is allowed for this card. Uses greedy
                    // simulation matching Java's ComputerUtilMana behavior.
                    let has_phyrexian = base.shards().iter().any(|s| s.is_phyrexian());
                    if has_phyrexian {
                        let ai_phy_param = card.abilities.iter().find_map(|ab| {
                            let params = Params::from_raw(ab);
                            params.get_cloned(keys::AI_PHYREXIAN_PAYMENT)
                        });
                        let phyrexian_life_allowed = match ai_phy_param.as_deref() {
                            Some("Never") => false,
                            Some(s) if s.starts_with("OnFatalDamage.") => {
                                let dmg: i32 = s[14..].parse().unwrap_or(0);
                                let opp = game.opponent_of(player);
                                game.player(opp).life <= dmg
                            }
                            _ => true,
                        };
                        if phyrexian_life_allowed {
                            available_mana
                                .can_pay_with_phyrexian_life(&base, game.player(player).life)
                        } else {
                            let colored = base.phyrexian_to_colored();
                            let reduced =
                                apply_cost_reductions(game, player, card_id, card, &colored);
                            if any_color {
                                available_mana.can_pay_any_color(&reduced)
                            } else {
                                available_mana.can_pay(&reduced)
                            }
                        }
                    } else {
                        let reduced = apply_cost_reductions(game, player, card_id, card, &base);
                        if any_color {
                            available_mana.can_pay_any_color(&reduced)
                        } else {
                            available_mana.can_pay(&reduced)
                        }
                    }
                };

                // Spectacle: alt cost if opponent lost life this turn
                let spectacle_ok = if let Some(spec_cost_str) = card.get_spectacle_cost() {
                    let adjusted = cost_adj
                        .apply(&forge_foundation::ManaCost::parse(&spec_cost_str))
                        .add(&raise_mana);
                    game.player_opponents_lost_life_this_turn(player)
                        && available_mana.can_pay(&adjusted)
                } else {
                    false
                };

                // Evoke: alt cost for creatures
                let evoke_ok = if let Some(evoke_cost_str) = card.get_evoke_cost() {
                    let adjusted = cost_adj
                        .apply(&forge_foundation::ManaCost::parse(&evoke_cost_str))
                        .add(&raise_mana);
                    available_mana.can_pay(&adjusted)
                } else {
                    false
                };

                // Dash: alt cost
                let dash_ok = if let Some(dash_cost_str) = card.get_dash_cost() {
                    let adjusted = cost_adj
                        .apply(&forge_foundation::ManaCost::parse(&dash_cost_str))
                        .add(&raise_mana);
                    available_mana.can_pay(&adjusted)
                } else {
                    false
                };

                // Blitz: alt cost
                let blitz_ok = if let Some(blitz_cost_str) = card.get_blitz_cost() {
                    let adjusted = cost_adj
                        .apply(&forge_foundation::ManaCost::parse(&blitz_cost_str))
                        .add(&raise_mana);
                    available_mana.can_pay(&adjusted)
                } else {
                    false
                };

                // Overload: alt cost
                let overload_ok = if let Some(ovl_cost_str) = card.get_overload_cost() {
                    let adjusted = cost_adj
                        .apply(&forge_foundation::ManaCost::parse(&ovl_cost_str))
                        .add(&raise_mana);
                    available_mana.can_pay(&adjusted)
                } else {
                    false
                };

                // StaticAbilityAlternativeCost (Mode$ AlternativeCost)
                let static_alt_ok =
                    crate::staticability::static_ability_alternative_cost::alternative_costs(
                        game,
                        &game.cards,
                        &cast_sa,
                        card,
                        player,
                    )
                    .iter()
                    .any(|entry| {
                        let base = Self::mana_from_cost(&entry.cost);
                        let adjusted = cost_adj.apply(&base).add(&raise_mana);
                        available_mana.can_pay(&adjusted)
                            && crate::cost::can_pay_ignoring_mana_for_spell(
                                &entry.cost,
                                game,
                                card_id,
                                player,
                            )
                    });

                // Suspend: special action, pay suspend cost to exile with time counters
                // (Suspend is not a spell cast — cost reduction doesn't apply)
                let suspend_ok = if let Some((suspend_cost_str, _counters)) =
                    card.get_suspend_cost()
                {
                    available_mana.can_pay(&forge_foundation::ManaCost::parse(&suspend_cost_str))
                } else {
                    false
                };

                // Foretell: pay {2} to exile face-down from hand
                // (This is a special action, not a cast — always costs {2})
                let foretell_exile_ok = if card.get_foretell_cost().is_some() {
                    available_mana.can_pay(&forge_foundation::ManaCost::generic(2))
                } else {
                    false
                };

                // Emerge: alt cost minus sacrificed creature's mana value
                let emerge_ok = if let Some(emerge_cost_str) = card.get_emerge_cost() {
                    // Simplified: check if emerge base cost is affordable
                    // (actual cost reduction from sac'd creature computed at cast time)
                    let adjusted =
                        cost_adj.apply(&forge_foundation::ManaCost::parse(&emerge_cost_str));
                    available_mana.can_pay(&adjusted) || {
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

                // Spree: base cost + cheapest ModeCost must be affordable
                let normal_ok = if card.has_keyword("Spree") && normal_ok {
                    let ability_text = card.abilities.first().cloned().unwrap_or_default();
                    let ability_params = Params::from_raw(&ability_text);
                    if let Some(choices_str) = ability_params.get(keys::CHOICES) {
                        let min_mode_cost = choices_str
                            .split(',')
                            .filter_map(|name| {
                                card.svars.get(name).and_then(|svar_val| {
                                    let params = Params::from_raw(svar_val);
                                    params
                                        .get(keys::MODE_COST)
                                        .map(|c| forge_foundation::ManaCost::parse(c).cmc())
                                })
                            })
                            .min()
                            .unwrap_or(0);
                        let base = cost_adj.apply(&card.mana_cost);
                        let spree_min =
                            base.add(&forge_foundation::ManaCost::generic(min_mode_cost));
                        if any_color {
                            available_mana.can_pay_any_color(&spree_min)
                        } else {
                            available_mana.can_pay(&spree_min)
                        }
                    } else {
                        normal_ok
                    }
                } else {
                    normal_ok
                };

                // Offering: sacrifice a permanent of a type to reduce cost
                let offering_ok = if let Some(offering_type) = card.get_offering_type() {
                    let offering_type_lower = offering_type.to_lowercase();
                    // Check if we have a permanent of the right type to sacrifice
                    game.cards_in_zone(ZoneType::Battlefield, player)
                        .iter()
                        .any(|&cid| {
                            cid != card_id && {
                                let c = game.card(cid);
                                match offering_type_lower.as_str() {
                                    "creature" => c.is_creature(),
                                    "artifact" => c.type_line.is_artifact(),
                                    "enchantment" => c.type_line.is_enchantment(),
                                    "land" => c.type_line.is_land(),
                                    _ => c.type_line.has_subtype(&offering_type),
                                }
                            }
                        })
                } else {
                    false
                };

                // Morph: can cast any Morph card face-down for the morph generic cost
                let morph_ok = card.has_morph
                    && available_mana.can_pay(&forge_foundation::ManaCost::generic(
                        crate::spellability::MORPH_GENERIC_COST,
                    ));

                // Bestow: cast as an Aura for bestow cost
                let bestow_ok = if let Some(bestow_cost_str) = card.get_bestow_cost() {
                    let adjusted =
                        cost_adj.apply(&forge_foundation::ManaCost::parse(&bestow_cost_str));
                    available_mana.can_pay(&adjusted)
                } else {
                    false
                };

                // Warp: alt cost for creatures
                let warp_ok = if let Some(warp_cost_str) = card.get_warp_cost() {
                    let adjusted = cost_adj
                        .apply(&forge_foundation::ManaCost::parse(&warp_cost_str))
                        .add(&raise_mana);
                    available_mana.can_pay(&adjusted)
                } else {
                    false
                };

                if !normal_ok
                    && !spectacle_ok
                    && !evoke_ok
                    && !dash_ok
                    && !blitz_ok
                    && !overload_ok
                    && !static_alt_ok
                    && !suspend_ok
                    && !foretell_exile_ok
                    && !emerge_ok
                    && !offering_ok
                    && !morph_ok
                    && !bestow_ok
                    && !warp_ok
                {
                    continue;
                }

                // Check additional non-mana costs from SP$ line (e.g. Sac<1/Creature>,
                // BeholdExile<...>) through shared cost payability logic.
                // Use the _for_spell variant so CantSacrifice statics (e.g. Yasharn)
                // can properly evaluate ValidCause$ Spell restrictions.
                let spell_cost = Self::parse_spell_cost(&card.abilities);
                let sp_additional_ok = if let Some(ref sc) = spell_cost {
                    crate::cost::can_pay_ignoring_mana_for_spell(sc, game, card_id, player)
                } else {
                    true
                };
                let raised_additional_ok = if let Some(ref rc) = raise_cost {
                    crate::cost::can_pay_ignoring_mana_for_spell(rc, game, card_id, player)
                } else {
                    true
                };
                let additional_costs_ok = sp_additional_ok && raised_additional_ok;

                if additional_costs_ok {
                    // Only validate cast-time targets from SP$ abilities.
                    // Non-spell abilities (AB$/DB$/...) must not gate whether the card
                    // can be cast from hand. Otherwise cards with target-dependent
                    // activated abilities (e.g. Walking Bulwark) become incorrectly
                    // uncastable when no valid AB$ target exists.
                    let all_valid = card
                        .abilities
                        .iter()
                        .filter(|ab| Params::from_raw(ab).has(keys::SP))
                        .all(|ab| {
                            target_restrictions::has_candidates_in_chain(
                                game,
                                player,
                                ab,
                                Some(card_id),
                            )
                        });
                    if all_valid {
                        if normal_ok {
                            playable.push(crate::agent::PlayOption {
                                card_id,
                                mode: crate::agent::PlayCardMode::Normal,
                            });
                        }
                        if spectacle_ok {
                            playable.push(crate::agent::PlayOption {
                                card_id,
                                mode: crate::agent::PlayCardMode::Alternative(
                                    crate::spellability::AlternativeCost::Spectacle,
                                ),
                            });
                        }
                        if evoke_ok {
                            playable.push(crate::agent::PlayOption {
                                card_id,
                                mode: crate::agent::PlayCardMode::Alternative(
                                    crate::spellability::AlternativeCost::Evoke,
                                ),
                            });
                        }
                        if dash_ok {
                            playable.push(crate::agent::PlayOption {
                                card_id,
                                mode: crate::agent::PlayCardMode::Alternative(
                                    crate::spellability::AlternativeCost::Dash,
                                ),
                            });
                        }
                        if blitz_ok {
                            playable.push(crate::agent::PlayOption {
                                card_id,
                                mode: crate::agent::PlayCardMode::Alternative(
                                    crate::spellability::AlternativeCost::Blitz,
                                ),
                            });
                        }
                        if overload_ok {
                            playable.push(crate::agent::PlayOption {
                                card_id,
                                mode: crate::agent::PlayCardMode::Alternative(
                                    crate::spellability::AlternativeCost::Overload,
                                ),
                            });
                        }
                        if static_alt_ok {
                            playable.push(crate::agent::PlayOption {
                                card_id,
                                mode: crate::agent::PlayCardMode::StaticAlternative,
                            });
                        }
                        if emerge_ok {
                            playable.push(crate::agent::PlayOption {
                                card_id,
                                mode: crate::agent::PlayCardMode::Alternative(
                                    crate::spellability::AlternativeCost::Emerge,
                                ),
                            });
                        }
                        if suspend_ok {
                            playable.push(crate::agent::PlayOption {
                                card_id,
                                mode: crate::agent::PlayCardMode::Alternative(
                                    crate::spellability::AlternativeCost::Suspend,
                                ),
                            });
                        }
                        if foretell_exile_ok {
                            playable.push(crate::agent::PlayOption {
                                card_id,
                                mode: crate::agent::PlayCardMode::ForetellExile,
                            });
                        }
                        if morph_ok {
                            playable.push(crate::agent::PlayOption {
                                card_id,
                                mode: crate::agent::PlayCardMode::Alternative(
                                    crate::spellability::AlternativeCost::Morph,
                                ),
                            });
                        }
                        if bestow_ok {
                            playable.push(crate::agent::PlayOption {
                                card_id,
                                mode: crate::agent::PlayCardMode::Alternative(
                                    crate::spellability::AlternativeCost::Bestow,
                                ),
                            });
                        }
                        if warp_ok {
                            playable.push(crate::agent::PlayOption {
                                card_id,
                                mode: crate::agent::PlayCardMode::Alternative(
                                    crate::spellability::AlternativeCost::Warp,
                                ),
                            });
                        }
                    }
                }
            }
        }

        // Check graveyard for cast permissions such as Flashback and Escape.
        let graveyard: Vec<CardId> = game.cards_in_zone(ZoneType::Graveyard, player).to_vec();
        for card_id in graveyard {
            let card = game.card(card_id);
            if must_be_instant && !has_flash_permission(card_id) {
                continue;
            }
            let available_mana = mana::calculate_available_mana(self.pool(player), game, player);
            let flashback_ok = if let Some(fb_cost_str) = card.get_flashback_cost() {
                let fb_cost = crate::cost::parse_cost(&fb_cost_str);
                let fb_mana = Self::mana_from_cost(&fb_cost);
                available_mana.can_pay(&fb_mana)
                    && crate::cost::can_pay_ignoring_mana_for_spell(&fb_cost, game, card_id, player)
            } else {
                false
            };
            let escape_ok = if let Some((escape_mana_str, exile_count)) = card.get_escape_cost() {
                let escape_mc = forge_foundation::ManaCost::parse(&escape_mana_str);
                let other_gy_count = game
                    .cards_in_zone(ZoneType::Graveyard, player)
                    .iter()
                    .filter(|&&cid| cid != card_id)
                    .count() as i32;
                available_mana.can_pay(&escape_mc) && other_gy_count >= exile_count
            } else {
                false
            };
            if flashback_ok {
                playable.push(crate::agent::PlayOption {
                    card_id,
                    mode: crate::agent::PlayCardMode::Alternative(
                        crate::spellability::AlternativeCost::Flashback,
                    ),
                });
            }
            if escape_ok {
                playable.push(crate::agent::PlayOption {
                    card_id,
                    mode: crate::agent::PlayCardMode::Alternative(
                        crate::spellability::AlternativeCost::Escape,
                    ),
                });
            }
        }

        // Check exile for Foretold cards (face-down in exile with foretell cost)
        // and Madness cards (exiled via discard with MadnessExiled marker)
        let exile: Vec<CardId> = game.cards_in_zone(ZoneType::Exile, player).to_vec();
        for card_id in exile {
            let card = game.card(card_id);
            if card.face_down {
                if let Some(foretell_cost_str) = card.get_foretell_cost() {
                    if must_be_instant && !has_flash_permission(card_id) {
                        continue;
                    }
                    let available_mana =
                        mana::calculate_available_mana(self.pool(player), game, player);
                    let foretell_mc = forge_foundation::ManaCost::parse(&foretell_cost_str);
                    let cost_adj = crate::cost::cost_adjustment::compute_cost_adjustment(
                        game,
                        card,
                        player,
                        ZoneType::Exile,
                    );
                    let adjusted = cost_adj.apply(&foretell_mc);
                    if available_mana.can_pay(&adjusted) {
                        playable.push(crate::agent::PlayOption {
                            card_id,
                            mode: crate::agent::PlayCardMode::Alternative(
                                crate::spellability::AlternativeCost::Foretell,
                            ),
                        });
                    }
                }
            } else if card.has_keyword(crate::card::KEYWORD_MADNESS_EXILED) {
                // Madness: exiled card that can be cast for madness cost
                if let Some(madness_cost_str) = card.get_madness_cost() {
                    if must_be_instant && !has_flash_permission(card_id) {
                        continue;
                    }
                    let available_mana =
                        mana::calculate_available_mana(self.pool(player), game, player);
                    if available_mana.can_pay(&forge_foundation::ManaCost::parse(&madness_cost_str))
                    {
                        playable.push(crate::agent::PlayOption {
                            card_id,
                            mode: crate::agent::PlayCardMode::Alternative(
                                crate::spellability::AlternativeCost::Madness,
                            ),
                        });
                    }
                }
            } else if let Some(plotted_turn) = card
                .keywords
                .iter_strings()
                .chain(card.granted_keywords.iter_strings())
                .find_map(|kw| crate::card::parse_plotted_turn(kw))
            {
                // Plot: plotted card in exile can be cast for free on a LATER turn
                if game.turn.turn_number <= plotted_turn {
                    continue;
                }
                if must_be_instant && !has_flash_permission(card_id) {
                    continue;
                }
                playable.push(crate::agent::PlayOption {
                    card_id,
                    mode: crate::agent::PlayCardMode::Alternative(
                        crate::spellability::AlternativeCost::Plot,
                    ),
                });
            } else if card.has_keyword(crate::card::KEYWORD_WARP_EXILED) {
                // Warp: exiled card can be cast for its normal mana cost
                if must_be_instant && !has_flash_permission(card_id) {
                    continue;
                }
                let available_mana =
                    mana::calculate_available_mana(self.pool(player), game, player);
                let cost_adj = crate::cost::cost_adjustment::compute_cost_adjustment(
                    game,
                    card,
                    player,
                    ZoneType::Exile,
                );
                let adjusted = cost_adj.apply(&card.mana_cost);
                if available_mana.can_pay(&adjusted) {
                    playable.push(crate::agent::PlayOption {
                        card_id,
                        mode: crate::agent::PlayCardMode::Normal,
                    });
                }
            }
        }

        // Check Command zone for commanders (with commander tax)
        let command_zone: Vec<CardId> = game.cards_in_zone(ZoneType::Command, player).to_vec();
        for card_id in command_zone {
            let card = game.card(card_id);
            if game.player_is_commander(player, card_id) {
                if must_be_instant && !has_flash_permission(card_id) {
                    continue;
                }
                let tax = game.player_commander_tax(player, card_id);
                let cost_adj = crate::cost::cost_adjustment::compute_cost_adjustment(
                    game,
                    card,
                    player,
                    ZoneType::Command,
                );
                let adjusted_cost = cost_adj.apply(&card.mana_cost);
                let available_mana =
                    mana::calculate_available_mana(self.pool(player), game, player);
                if available_mana.can_pay_with_extra_generic(&adjusted_cost, tax) {
                    playable.push(crate::agent::PlayOption {
                        card_id,
                        mode: crate::agent::PlayCardMode::Normal,
                    });
                }
            }
        }

        playable
    }
}
