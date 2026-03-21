use super::{resolve_defined_players, EffectContext};
use crate::parsing::keys;
use crate::spellability::SpellAbility;

/// Mirrors Java's `ActivateAbilityEffect` for the common `ManaAbility$ True`
/// case used by cards like Pygmy Hippo.
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let controller = sa.activating_player;
    let defined = sa
        .params
        .get(keys::DEFINED)
        .unwrap_or("You");
    let only_mana = sa
        .params
        .is_true(keys::MANA_ABILITY);
    let type_filter = sa.params.get(keys::TYPE).unwrap_or("Card");

    let players = resolve_defined_players(defined, controller, ctx.game);
    for pid in players {
        if !ctx.game.player(pid).is_alive() {
            continue;
        }
        let battlefield = ctx
            .game
            .cards_in_zone(forge_foundation::ZoneType::Battlefield, pid);
        let card_ids: Vec<crate::ids::CardId> = battlefield.to_vec();
        for cid in card_ids {
            let (is_land, is_tapped, chosen_colors, produced, has_tap_cost) = {
                let card = ctx.game.card(cid);
                if type_filter.eq_ignore_ascii_case("Land") && !card.is_land() {
                    continue;
                }
                if !type_filter.eq_ignore_ascii_case("Land")
                    && !type_filter.eq_ignore_ascii_case("Card")
                {
                    continue;
                }
                if card.tapped {
                    continue;
                }

                // Java lets controller choose one ability per card. For parity with
                // current engine scope, resolve the first legal mana ability.
                let maybe_mana_ab = card.activated_abilities.iter().find(|ab| {
                    (!only_mana || ab.is_mana_ability)
                        && ab.params.get(keys::AB) == Some("Mana")
                });
                let Some(mana_ab) = maybe_mana_ab else {
                    continue;
                };

                (
                    card.is_land(),
                    card.tapped,
                    card.chosen_colors.clone(),
                    mana_ab.params.get_cloned(keys::PRODUCED),
                    mana_ab.cost.has_tap,
                )
            };

            if type_filter.eq_ignore_ascii_case("Land") && !is_land {
                continue;
            }
            if is_tapped {
                continue;
            }
            if has_tap_cost {
                ctx.game.tap(cid);
            }

            if let Some(produced) = produced.as_deref() {
                let atoms = crate::mana::produced_to_atoms(produced, &chosen_colors);
                if let Some(atom) = atoms.first().copied() {
                    ctx.mana_pools[pid.index()].add(atom, 1);
                }
            }
        }
    }
}
