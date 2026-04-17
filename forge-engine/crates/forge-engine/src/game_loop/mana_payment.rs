use super::*;

use forge_foundation::{ManaCost, ZoneType};

use crate::ability::activated::ActivatedAbility;

#[derive(Clone, Copy)]
pub(crate) struct ManaPaymentSession<'a> {
    pub player: PlayerId,
    pub card_id: CardId,
    pub card_name: &'a str,
    pub mana_cost: &'a ManaCost,
    pub cost_str: &'a str,
    pub cost_display_str: &'a str,
    pub cost_checkpoint_str: &'a str,
    pub is_activated_ability: bool,
    pub reserved_sacrifices: &'a [CardId],
}

impl GameLoop {
    fn notify_mana_payment_resolved(
        agents: &mut [Box<dyn PlayerAgent>],
        player: PlayerId,
        actions: &[ManaCostAction],
    ) {
        let notification = crate::agent::notification::GameNotification::ManaPaymentResolved {
            player,
            actions: actions.to_vec(),
        };
        for agent in agents.iter_mut() {
            agent.notify(notification.clone());
        }
    }

    pub(crate) fn make_mana_payment_callback<'a>(
        trigger_handler: *mut TriggerHandler,
        game: *mut GameState,
        agents: &'a mut [Box<dyn PlayerAgent>],
        player: PlayerId,
    ) -> impl FnMut(mana::ManaPayCallback<'_>) -> Option<CardId> + 'a {
        move |kind: mana::ManaPayCallback<'_>| -> Option<CardId> {
            match kind {
                mana::ManaPayCallback::ChooseSacrifice(valid) => {
                    agents[player.index()].choose_sacrifice(player, valid, None)
                }
                mana::ManaPayCallback::ChooseColor(valid_colors) => {
                    if !agents[player.index()].is_human() {
                        let _ = agents[player.index()].choose_color(player, valid_colors);
                    }
                    None
                }
                mana::ManaPayCallback::ConfirmSelfSacrifice(source_id) => {
                    if agents[player.index()].confirm_payment(
                        player,
                        "Sacrifice",
                        "Sacrifice for mana",
                        None,
                        Some(crate::ability::api_type::ApiType::Mana),
                    ) {
                        Some(source_id)
                    } else {
                        None
                    }
                }
                mana::ManaPayCallback::ConfirmSubCounter(source_id) => {
                    if agents[player.index()].confirm_payment(
                        player,
                        "SubCounter",
                        "Remove counter for mana",
                        None,
                        Some(crate::ability::api_type::ApiType::Mana),
                    ) {
                        Some(source_id)
                    } else {
                        None
                    }
                }
                mana::ManaPayCallback::ConfirmSourceExile(source_id) => {
                    if agents[player.index()].confirm_payment(
                        player,
                        "Exile",
                        "Exile for mana",
                        None,
                        Some(crate::ability::api_type::ApiType::Mana),
                    ) {
                        Some(source_id)
                    } else {
                        None
                    }
                }
                mana::ManaPayCallback::NotifySacrificeForMana(sacrificed_id) => unsafe {
                    let game = &mut *game;
                    let trigger_handler = &mut *trigger_handler;
                    let owner = game.card(sacrificed_id).owner;
                    let lki_p1p1 = *game
                        .card(sacrificed_id)
                        .counters
                        .get(&crate::card::CounterType::P1P1)
                        .unwrap_or(&0);
                    let lki_power = game.card(sacrificed_id).power();
                    let lki_toughness = game.card(sacrificed_id).toughness();
                    trigger_handler.run_trigger(
                        TriggerType::Sacrificed,
                        RunParams {
                            card: Some(sacrificed_id),
                            player: Some(player),
                            ..Default::default()
                        },
                        false,
                    );
                    crate::ability::effects::emit_zone_trigger_with_lki_counters(
                        trigger_handler,
                        sacrificed_id,
                        ZoneType::Battlefield,
                        ZoneType::Graveyard,
                        lki_p1p1,
                        lki_power,
                        lki_toughness,
                    );
                    trigger_handler.flush_waiting_triggers(game);
                    game.move_card(sacrificed_id, ZoneType::Graveyard, owner);
                    Some(sacrificed_id)
                },
            }
        }
    }

    pub(crate) fn pay_mana_cost_session<FAvail, FAuto, FTryPay>(
        &mut self,
        game: &mut GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        session: ManaPaymentSession<'_>,
        mana_ability_available: FAvail,
        mut auto_pay: FAuto,
        mut try_pay_from_pool: FTryPay,
    ) -> bool
    where
        FAvail: Fn(&GameState, PlayerId, CardId, &ActivatedAbility, &[CardId]) -> bool,
        FAuto: FnMut(
            &mut GameLoop,
            &mut GameState,
            &mut [Box<dyn PlayerAgent>],
            ManaPaymentSession<'_>,
        ) -> Option<Vec<ManaCostAction>>,
        FTryPay: FnMut(&mut GameLoop, &mut GameState, PlayerId) -> bool,
    {
        let saved_pool = self.mana_pools[session.player.index()].clone();
        let mut mana_loop_invalid_count = 0u32;
        let mut executed_actions: Vec<ManaCostAction> = Vec::new();

        loop {
            let mana_sources = mana::collect_mana_payment_sources(
                game,
                session.player,
                session.reserved_sacrifices,
            );
            let tappable_lands = mana_sources.source_cards.clone();
            let mana_ability_options = mana_sources.mana_ability_options;
            let pool_snapshot = self.mana_pools[session.player.index()].clone();
            let untappable_lands: Vec<CardId> = game
                .cards_in_zone(ZoneType::Battlefield, session.player)
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
            let pool_ref = self.mana_pools[session.player.index()].clone();

            agents[session.player.index()].snapshot_state(game, &self.mana_pools);
            let action = agents[session.player.index()].pay_mana_cost(
                session.player,
                session.card_id,
                session.card_name,
                session.cost_str,
                session.cost_display_str,
                session.cost_checkpoint_str,
                session.is_activated_ability,
                session.reserved_sacrifices,
                &mana_ability_options,
                &tappable_lands,
                &untappable_lands,
                &pool_ref,
            );

            match action {
                ManaCostAction::TapLand {
                    card_id: land_id,
                    mana_ability_index,
                    express_choice,
                } => {
                    if !tappable_lands.contains(&land_id) {
                        mana_loop_invalid_count += 1;
                        if mana_loop_invalid_count > 3 {
                            self.mana_pools[session.player.index()] = saved_pool.clone();
                            return false;
                        }
                        continue;
                    }
                    mana_loop_invalid_count = 0;
                    let mana_ab = {
                        let c = game.card(land_id);
                        mana_ability_index
                            .and_then(|idx| c.activated_abilities.get(idx))
                            .filter(|ab| {
                                ab.is_mana_ability
                                    && mana_ability_available(
                                        game,
                                        session.player,
                                        land_id,
                                        ab,
                                        session.reserved_sacrifices,
                                    )
                            })
                            .cloned()
                            .or_else(|| {
                                c.activated_abilities
                                    .iter()
                                    .find(|ab| {
                                        ab.is_mana_ability
                                            && mana_ability_available(
                                                game,
                                                session.player,
                                                land_id,
                                                ab,
                                                session.reserved_sacrifices,
                                            )
                                    })
                                    .cloned()
                            })
                    };
                    if let Some(ab) = mana_ab {
                        executed_actions.push(ManaCostAction::TapLand {
                            card_id: land_id,
                            mana_ability_index: Some(mana_ability_index.unwrap_or(0)),
                            express_choice,
                        });
                        self.resolve_mana_ability(
                            game,
                            agents,
                            session.player,
                            land_id,
                            &ab,
                            express_choice,
                        );
                    } else if let Some(atom) = basic_land_mana_atom(game.card(land_id)) {
                        let _ = atom;
                        executed_actions.push(ManaCostAction::TapLand {
                            card_id: land_id,
                            mana_ability_index: Some(0),
                            express_choice: None,
                        });
                        game.tap(land_id);
                        self.mana_pools[session.player.index()].add(atom, 1);
                        self.trigger_handler.run_trigger(
                            TriggerType::TapsForMana,
                            RunParams {
                                card: Some(land_id),
                                player: Some(session.player),
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
                        executed_actions.push(ManaCostAction::UntapLand(land_id));
                        game.untap(land_id);
                        for atom in atoms {
                            self.mana_pools[session.player.index()].remove(atom, 1);
                        }
                    }
                }
                ManaCostAction::Pay { auto } => {
                    if auto {
                        if let Some(mut auto_trace) = auto_pay(self, game, agents, session) {
                            executed_actions.append(&mut auto_trace);
                            executed_actions.push(ManaCostAction::Pay { auto: false });
                            Self::notify_mana_payment_resolved(
                                agents,
                                session.player,
                                &executed_actions,
                            );
                            return true;
                        }
                        executed_actions.push(ManaCostAction::Cancel);
                        Self::notify_mana_payment_resolved(
                            agents,
                            session.player,
                            &executed_actions,
                        );
                        self.mana_pools[session.player.index()] = saved_pool.clone();
                        return false;
                    }

                    if try_pay_from_pool(self, game, session.player) {
                        executed_actions.push(ManaCostAction::Pay { auto: false });
                        Self::notify_mana_payment_resolved(
                            agents,
                            session.player,
                            &executed_actions,
                        );
                        return true;
                    }

                    mana_loop_invalid_count += 1;
                    if mana_loop_invalid_count > 3 {
                        executed_actions.push(ManaCostAction::Cancel);
                        Self::notify_mana_payment_resolved(
                            agents,
                            session.player,
                            &executed_actions,
                        );
                        self.mana_pools[session.player.index()] = saved_pool.clone();
                        return false;
                    }
                }
                ManaCostAction::Cancel => {
                    executed_actions.push(ManaCostAction::Cancel);
                    Self::notify_mana_payment_resolved(agents, session.player, &executed_actions);
                    self.mana_pools[session.player.index()] = saved_pool.clone();
                    return false;
                }
            }
        }
    }
}
