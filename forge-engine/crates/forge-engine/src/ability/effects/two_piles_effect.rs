use forge_foundation::ZoneType;

use super::{emit_zone_trigger, EffectContext};
use crate::ids::CardId;
use crate::spellability::SpellAbility;

/// `SP$ TwoPiles` — divide cards into two piles and an opponent chooses one.
///
/// Mirrors Java's `TwoPilesEffect.java` (simplified — auto-divides).
/// - `NumCards$` — number of cards to reveal from library (default 5).
/// - `Zone1$` — destination for the chosen pile (default Hand).
/// - `Zone2$` — destination for the unchosen pile (default Graveyard).
///
/// # Card script examples
/// ```text
/// A:SP$ TwoPiles | NumCards$ 5 | Zone1$ Hand | Zone2$ Graveyard
/// A:SP$ TwoPiles | NumCards$ 3 | Zone1$ Hand | Zone2$ Library
/// ```
/// Struct form of this effect so it can participate in the
/// `SpellAbilityEffect` trait hierarchy — mirrors Java's
/// `TwoPilesEffect` class extending `SpellAbilityEffect`.
pub struct TwoPilesEffect;

impl crate::ability::spell_ability_effect::SpellAbilityEffect for TwoPilesEffect {
    fn resolve(ctx: &mut EffectContext, sa: &crate::spellability::SpellAbility) {
    let controller = sa.activating_player;
    let num_cards = sa
        .params
        .get("NumCards")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(5);

    let zone1 = sa
        .params
        .get("Zone1")
        .and_then(|z| super::parse_zone_type(z))
        .unwrap_or(ZoneType::Hand);
    let zone2 = sa
        .params
        .get("Zone2")
        .and_then(|z| super::parse_zone_type(z))
        .unwrap_or(ZoneType::Graveyard);

    // Get top N cards from library
    let lib = ctx
        .game
        .cards_in_zone(ZoneType::Library, controller)
        .to_vec();
    if lib.is_empty() {
        return;
    }

    let count = num_cards.min(lib.len());
    // Library is bottom-to-top, so take from the end
    let revealed: Vec<CardId> = lib[lib.len() - count..].to_vec();

    if revealed.is_empty() {
        return;
    }

    // Let the controller peek at the revealed cards
    ctx.agents[controller.index()].on_library_peek(ctx.game, &revealed);

    // Controller divides into two piles — simplified: they choose cards for pile 1
    // via choose_cards_for_effect (min 0, max count-1 to ensure both piles have at least 1 if count>1)
    let min_pile1 = if count > 1 { 1 } else { 0 };
    let max_pile1 = if count > 1 { count - 1 } else { count };

    let pile1: Vec<CardId> = ctx.agents[controller.index()]
        .choose_cards_for_effect(controller, &revealed, min_pile1, max_pile1);
    let pile2: Vec<CardId> = revealed
        .iter()
        .filter(|c| !pile1.contains(c))
        .copied()
        .collect();

    // Opponent chooses which pile goes to zone1
    let opponent = ctx.game.opponent_of(controller);
    let pile1_names: Vec<String> = pile1
        .iter()
        .map(|&cid| ctx.game.card(cid).card_name.clone())
        .collect();
    let pile2_names: Vec<String> = pile2
        .iter()
        .map(|&cid| ctx.game.card(cid).card_name.clone())
        .collect();

    let prompt = format!(
        "Choose a pile: Pile 1 ({}) or Pile 2 ({})?",
        pile1_names.join(", "),
        pile2_names.join(", "),
    );
    let choose_pile1 =
        ctx.agents[opponent.index()].choose_optional_trigger(opponent, &prompt, None, None);

    let (chosen_pile, unchosen_pile) = if choose_pile1 {
        (pile1, pile2)
    } else {
        (pile2, pile1)
    };

    // Move chosen pile to zone1
    for cid in chosen_pile {
        if ctx.game.card(cid).zone == ZoneType::Library {
            ctx.move_card(cid, zone1, controller);
            emit_zone_trigger(ctx.trigger_handler, cid, ZoneType::Library, zone1);
        }
    }

    // Move unchosen pile to zone2
    for cid in unchosen_pile {
        if ctx.game.card(cid).zone == ZoneType::Library {
            ctx.move_card(cid, zone2, controller);
            emit_zone_trigger(ctx.trigger_handler, cid, ZoneType::Library, zone2);
        }
    }
    }
}
