use forge_foundation::ZoneType;

use super::{emit_zone_trigger, matches_change_type, parse_zone_type, EffectContext};
use crate::ids::{CardId, PlayerId};
use crate::spellability::SpellAbility;

fn matches_change_zone_all_filter(
    cid: CardId,
    sa: &SpellAbility,
    game: &crate::game::GameState,
    filter: &str,
    source_chosen_colors: &[String],
) -> bool {
    if filter.trim().is_empty() {
        return true;
    }

    // Forge uses comma-separated OR clauses in ChangeType$.
    for clause in filter.split(',') {
        let clause = clause.trim();
        if clause.is_empty() {
            continue;
        }

        let mut clause_ok = true;
        let targeted = sa.target_chosen.target_card;

        // "+" means AND within a clause.
        for term in clause.split('+') {
            let term = term.trim();
            if term.is_empty() {
                continue;
            }

            if term.eq_ignore_ascii_case("NotDefinedTargeted") {
                if let Some(t) = targeted {
                    if cid == t {
                        clause_ok = false;
                        break;
                    }
                } else {
                    clause_ok = false;
                    break;
                }
                continue;
            }

            if term.eq_ignore_ascii_case("TargetedCard.Self") {
                if targeted != Some(cid) {
                    clause_ok = false;
                    break;
                }
                continue;
            }

            if term.starts_with("sharesNameWith") {
                let arg = term
                    .strip_prefix("sharesNameWith")
                    .unwrap_or("")
                    .trim_start();
                if arg.eq_ignore_ascii_case("Targeted") {
                    if let Some(t) = targeted {
                        if game.card(cid).card_name != game.card(t).card_name {
                            clause_ok = false;
                            break;
                        }
                    } else {
                        clause_ok = false;
                        break;
                    }
                    continue;
                }
            }

            if term.starts_with("ControlledBy") {
                let arg = term.strip_prefix("ControlledBy").unwrap_or("").trim_start();
                if arg.eq_ignore_ascii_case("TargetedController") {
                    if let Some(t) = targeted {
                        if game.card(cid).controller != game.card(t).controller {
                            clause_ok = false;
                            break;
                        }
                    } else {
                        clause_ok = false;
                        break;
                    }
                    continue;
                }
            }

            if !matches_change_type(game.card(cid), term, source_chosen_colors) {
                clause_ok = false;
                break;
            }
        }

        if clause_ok {
            return true;
        }
    }

    false
}

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let origin_str = sa
        .params
        .get("Origin")
        .map(|s| s.as_str())
        .unwrap_or("Battlefield");
    let destination_str = sa
        .params
        .get("Destination")
        .map(|s| s.as_str())
        .unwrap_or("Graveyard");
    // Forge uses ChangeType$ as the primary filter for ChangeZoneAll; fall back to ValidCards$.
    let valid_cards_filter = sa
        .params
        .get("ChangeType")
        .or_else(|| sa.params.get("ValidCards"))
        .cloned()
        .unwrap_or_else(|| "Card".to_string());
    let tapped = sa
        .params
        .get("Tapped")
        .map(|s| s.eq_ignore_ascii_case("True"))
        .unwrap_or(false);

    // Resolve source card's chosen_colors for ChosenColor qualifier support.
    let source_chosen_colors: Vec<String> = sa
        .source
        .map(|src| ctx.game.card(src).chosen_colors.clone())
        .unwrap_or_default();

    if let (Some(dest_zone), Some(origin_zone)) = (
        parse_zone_type(destination_str),
        parse_zone_type(origin_str),
    ) {
        let player_ids = ctx.game.player_order.clone();
        let mut to_move: Vec<(CardId, PlayerId)> = Vec::new();

        for &pid in &player_ids {
            let zone_cards = ctx.game.cards_in_zone(origin_zone, pid).to_vec();
            for cid in zone_cards {
                if matches_change_zone_all_filter(
                    cid,
                    sa,
                    ctx.game,
                    &valid_cards_filter,
                    &source_chosen_colors,
                ) {
                    let dest_owner = if dest_zone == ZoneType::Battlefield {
                        sa.activating_player
                    } else {
                        ctx.game.card(cid).owner
                    };
                    to_move.push((cid, dest_owner));
                }
            }
        }

        for (card_id, dest_owner) in to_move {
            if ctx.game.card(card_id).zone != origin_zone {
                continue; // already moved
            }
            let old_zone = ctx.game.card(card_id).zone;
            ctx.game.move_card(card_id, dest_zone, dest_owner);
            if dest_zone == ZoneType::Battlefield {
                if tapped {
                    ctx.game.tap(card_id);
                }
                ctx.trigger_handler
                    .register_active_trigger(ctx.game, card_id);
            }
            emit_zone_trigger(ctx.trigger_handler, card_id, old_zone, dest_zone);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::matches_change_zone_all_filter;
    use crate::card::CardInstance;
    use crate::game::GameState;
    use crate::ids::{CardId, PlayerId};
    use crate::spellability::SpellAbility;
    use forge_foundation::{CardTypeLine, ColorSet, ManaCost, ZoneType};

    #[test]
    fn deputy_filter_only_hits_targeted_name_group() {
        let mut game = GameState::new(&["A", "B"], 20);
        let p0 = PlayerId(0);
        let p1 = PlayerId(1);

        let make = |name: &str, owner: PlayerId, type_line: &str| {
            CardInstance::new(
                CardId(0),
                name.to_string(),
                owner,
                CardTypeLine::parse(type_line),
                ManaCost::no_cost(),
                ColorSet::COLORLESS,
                None,
                None,
                vec![],
                vec![],
            )
        };

        let t1 = game.create_card(make("Token Engine", p1, "Artifact"));
        let t2 = game.create_card(make("Token Engine", p1, "Artifact"));
        let opp_land = game.create_card(make("Island", p1, "Land Island"));
        let my_land = game.create_card(make("Forest", p0, "Land Forest"));

        game.move_card(t1, ZoneType::Battlefield, p1);
        game.move_card(t2, ZoneType::Battlefield, p1);
        game.move_card(opp_land, ZoneType::Battlefield, p1);
        game.move_card(my_land, ZoneType::Battlefield, p0);

        let filter = "TargetedCard.Self,Permanent.nonLand+NotDefinedTargeted+sharesNameWith Targeted+ControlledBy TargetedController";
        let mut sa = SpellAbility::new_simple(None, p0, "DB$ ChangeZoneAll | Origin$ Battlefield");
        sa.target_chosen.target_card = Some(t1);

        assert!(matches_change_zone_all_filter(t1, &sa, &game, filter, &[]));
        assert!(matches_change_zone_all_filter(t2, &sa, &game, filter, &[]));
        assert!(!matches_change_zone_all_filter(
            opp_land,
            &sa,
            &game,
            filter,
            &[]
        ));
        assert!(!matches_change_zone_all_filter(
            my_land,
            &sa,
            &game,
            filter,
            &[]
        ));
    }
}
