//! Replacement effect dispatcher.
//!
//! Mirrors the Java Forge `ReplacementHandler.java` in
//! `forge/game/replacement/`.
//!
//! The entry point is [`apply_replacements`], which accepts a mutable
//! [`ReplacementEvent`] and iterates through the five CR 616 layers
//! (CantHappen → Control → Copy → Transform → Other), applying the first
//! matching effect in each layer.

use forge_foundation::ZoneType;

use crate::game::GameState;
use crate::ids::{CardId, PlayerId};
use crate::replacement::{ReplacementEffect, ReplacementLayer, ReplacementResult};

// ── ReplacementEvent ──────────────────────────────────────────────────────────

/// A game event that may be subject to one or more replacement effects.
///
/// Each variant carries the mutable parameters of the event so that a
/// replacement effect can modify them (e.g. reduce damage to 0, redirect a
/// zone move, etc.).
///
/// Mirrors the `runParams` map passed to `ReplacementHandler.run()` in Java,
/// but typed for safety.
#[derive(Debug, Clone)]
pub enum ReplacementEvent {
    /// A card is being drawn by a player.
    Draw { player: PlayerId },

    /// Damage is being dealt to a permanent.
    DamageToCard {
        target: CardId,
        amount: i32,
        /// The card dealing the damage, if known.
        source: Option<CardId>,
    },

    /// Damage is being dealt to a player.
    DamageToPlayer {
        target: PlayerId,
        amount: i32,
        /// The card dealing the damage, if known.
        source: Option<CardId>,
    },

    /// A permanent is being destroyed (lethal damage or destroy effect).
    Destroy { target: CardId },

    /// A card is moving between zones.
    Moved {
        card: CardId,
        origin: ZoneType,
        destination: ZoneType,
    },
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Apply all applicable replacement effects to a game event.
///
/// Processes effects in CR 616 layer order:
/// CantHappen → Control → Copy → Transform → Other.
/// Within each layer, effects are applied in battlefield entry order
/// (timestamp order, i.e. the order cards appear in `game.cards`).
///
/// Returns the final [`ReplacementResult`]:
/// - `NotReplaced`  — no effect applied.
/// - `Replaced`     — event was replaced (may have been a no-op replacement).
/// - `Prevented`    — event was prevented (damage set to 0).
/// - `Skipped`      — event skipped (e.g. "skip your draw step").
/// - `Updated`      — event parameters were modified; caller should re-check.
///
/// Mirrors Java `ReplacementHandler.run(ReplacementType, Map<AbilityKey,Object>)`.
pub fn apply_replacements(game: &GameState, event: &mut ReplacementEvent) -> ReplacementResult {
    for layer in [
        ReplacementLayer::CantHappen,
        ReplacementLayer::Control,
        ReplacementLayer::Copy,
        ReplacementLayer::Transform,
        ReplacementLayer::Other,
    ] {
        let result = run_layer(game, event, layer);
        if result != ReplacementResult::NotReplaced {
            return result;
        }
    }
    ReplacementResult::NotReplaced
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/// Apply one CR 616 layer of replacement effects.
///
/// Mirrors the private `run(event, runParams, layer, decider)` in Java's
/// `ReplacementHandler`.
fn run_layer(
    game: &GameState,
    event: &mut ReplacementEvent,
    layer: ReplacementLayer,
) -> ReplacementResult {
    let effects = collect_effects(game, event, layer);

    if effects.is_empty() {
        return ReplacementResult::NotReplaced;
    }

    // In a full implementation, the affected player would choose among multiple
    // effects in the same layer (CR 616.1).  For this framework we auto-select
    // the first effect (deterministic timestamp order).
    let (_source_card_id, ref effect) = effects[0];
    execute_effect(effect, event)
}

/// Collect all `(source_card_id, effect)` pairs that apply to `event` in `layer`.
///
/// Iterates over every card in the game and checks each of its replacement
/// effects.  Only effects whose source card is in an active zone are included.
///
/// Mirrors `ReplacementHandler.getReplacementList()`.
fn collect_effects(
    game: &GameState,
    event: &ReplacementEvent,
    layer: ReplacementLayer,
) -> Vec<(CardId, ReplacementEffect)> {
    let mut result = Vec::new();

    for (i, card) in game.cards.iter().enumerate() {
        let card_id = CardId(i as u32);

        for re in &card.replacement_effects {
            // Layer filter.
            if re.layer != layer {
                continue;
            }
            // Zone filter — effect is only active when host is in a valid zone.
            if !re.active_in_zone(card.zone) {
                continue;
            }
            // Event-specific applicability check.
            let applies = match event {
                ReplacementEvent::Draw { player } => re.can_replace_draw(*player, card),

                ReplacementEvent::DamageToCard { amount, .. } => {
                    *amount > 0 && re.can_replace_damage(false, card)
                }

                ReplacementEvent::DamageToPlayer { amount, .. } => {
                    *amount > 0 && re.can_replace_damage(true, card)
                }

                ReplacementEvent::Destroy { target } => {
                    let target_card = &game.cards[target.index()];
                    re.can_replace_destroy(target_card, card)
                }

                ReplacementEvent::Moved {
                    card: moving_id,
                    origin,
                    destination,
                } => {
                    let moving_card = &game.cards[moving_id.index()];
                    re.can_replace_moved(*origin, *destination, moving_card, card)
                }
            };

            if applies {
                result.push((card_id, re.clone()));
            }
        }
    }

    result
}

/// Execute a single replacement effect, mutating the event parameters.
///
/// Mirrors `ReplacementHandler.executeReplacement()`.
fn execute_effect(effect: &ReplacementEffect, event: &mut ReplacementEvent) -> ReplacementResult {
    match event {
        ReplacementEvent::Draw { .. } => {
            // Prevent$ True or Skip$ True → skip the draw.
            if effect.params.get("Prevent").map(|s| s == "True").unwrap_or(false)
                || effect.params.contains_key("Skip")
            {
                return ReplacementResult::Skipped;
            }
            ReplacementResult::Replaced
        }

        ReplacementEvent::DamageToCard { amount, .. } => {
            if effect.params.get("Prevent").map(|s| s == "True").unwrap_or(false) {
                *amount = 0;
                return ReplacementResult::Prevented;
            }
            ReplacementResult::Replaced
        }

        ReplacementEvent::DamageToPlayer { amount, .. } => {
            if effect.params.get("Prevent").map(|s| s == "True").unwrap_or(false) {
                *amount = 0;
                return ReplacementResult::Prevented;
            }
            ReplacementResult::Replaced
        }

        ReplacementEvent::Destroy { .. } => {
            // Indestructible: destruction is replaced by nothing.
            // The caller (check_state_based_actions) will not move the card.
            ReplacementResult::Replaced
        }

        ReplacementEvent::Moved { destination, .. } => {
            // NewDestination$ redirects where the card ends up.
            if let Some(new_dest) = effect.params.get("NewDestination") {
                let new_zone = match new_dest.trim() {
                    "Exile" => Some(ZoneType::Exile),
                    "Graveyard" => Some(ZoneType::Graveyard),
                    "Hand" => Some(ZoneType::Hand),
                    "Library" => Some(ZoneType::Library),
                    "Battlefield" => Some(ZoneType::Battlefield),
                    _ => None,
                };
                if let Some(z) = new_zone {
                    *destination = z;
                    return ReplacementResult::Updated;
                }
            }
            ReplacementResult::Replaced
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use forge_foundation::{CardTypeLine, ColorSet, ManaCost};

    use crate::card::CardInstance;
    use crate::ids::{CardId, PlayerId};

    // ── Test helpers ──────────────────────────────────────────────────────

    fn make_game() -> GameState {
        GameState::new(&["Alice", "Bob"], 20)
    }

    fn add_creature_with_abilities(
        game: &mut GameState,
        owner: PlayerId,
        name: &str,
        abilities: Vec<String>,
    ) -> CardId {
        let card = CardInstance::new(
            CardId(0), // placeholder; create_card assigns real ID
            name.to_string(),
            owner,
            CardTypeLine::parse("Creature - Bear"),
            ManaCost::parse("1 G"),
            ColorSet::GREEN,
            Some(2),
            Some(2),
            vec![],
            abilities,
        );
        game.create_card(card)
    }

    fn put_on_battlefield(game: &mut GameState, card_id: CardId, owner: PlayerId) {
        game.move_card(card_id, ZoneType::Battlefield, owner);
    }

    // ── Draw replacement tests ────────────────────────────────────────────

    #[test]
    fn draw_skip_for_controller() {
        let mut game = make_game();
        let alice = PlayerId(0);
        // Card with "skip your draw step" effect.
        let cid = add_creature_with_abilities(
            &mut game,
            alice,
            "SkipDraw",
            vec!["R$ Event$ Draw | ValidPlayer$ You | Prevent$ True".to_string()],
        );
        put_on_battlefield(&mut game, cid, alice);

        let mut event = ReplacementEvent::Draw { player: alice };
        let result = apply_replacements(&game, &mut event);
        assert_eq!(result, ReplacementResult::Skipped);
    }

    #[test]
    fn draw_not_skipped_for_opponent() {
        let mut game = make_game();
        let alice = PlayerId(0);
        let bob = PlayerId(1);
        let cid = add_creature_with_abilities(
            &mut game,
            alice,
            "SkipDraw",
            vec!["R$ Event$ Draw | ValidPlayer$ You | Prevent$ True".to_string()],
        );
        put_on_battlefield(&mut game, cid, alice);

        // Bob's draw is not affected by Alice's effect.
        let mut event = ReplacementEvent::Draw { player: bob };
        let result = apply_replacements(&game, &mut event);
        assert_eq!(result, ReplacementResult::NotReplaced);
    }

    #[test]
    fn draw_not_applied_when_card_not_on_battlefield() {
        let mut game = make_game();
        let alice = PlayerId(0);
        let cid = add_creature_with_abilities(
            &mut game,
            alice,
            "SkipDraw",
            vec!["R$ Event$ Draw | ActiveZones$ Battlefield | ValidPlayer$ You | Prevent$ True".to_string()],
        );
        // Card stays in hand, not battlefield.
        game.move_card(cid, ZoneType::Hand, alice);

        let mut event = ReplacementEvent::Draw { player: alice };
        let result = apply_replacements(&game, &mut event);
        assert_eq!(result, ReplacementResult::NotReplaced);
    }

    // ── Damage prevention tests ───────────────────────────────────────────

    #[test]
    fn damage_to_player_prevented() {
        let mut game = make_game();
        let alice = PlayerId(0);
        // "Prevent all damage dealt to you."
        let cid = add_creature_with_abilities(
            &mut game,
            alice,
            "Shield",
            vec!["R$ Event$ DamageDone | ValidTarget$ Player | Prevent$ True".to_string()],
        );
        put_on_battlefield(&mut game, cid, alice);

        let mut event = ReplacementEvent::DamageToPlayer {
            target: alice,
            amount: 5,
            source: None,
        };
        let result = apply_replacements(&game, &mut event);
        assert_eq!(result, ReplacementResult::Prevented);
        // Amount should be zeroed out.
        if let ReplacementEvent::DamageToPlayer { amount, .. } = event {
            assert_eq!(amount, 0);
        } else {
            panic!("unexpected event type");
        }
    }

    #[test]
    fn damage_zero_not_replaced() {
        let mut game = make_game();
        let alice = PlayerId(0);
        let cid = add_creature_with_abilities(
            &mut game,
            alice,
            "Shield",
            vec!["R$ Event$ DamageDone | ValidTarget$ Player | Prevent$ True".to_string()],
        );
        put_on_battlefield(&mut game, cid, alice);

        // Zero damage — effect should not apply.
        let mut event = ReplacementEvent::DamageToPlayer {
            target: alice,
            amount: 0,
            source: None,
        };
        let result = apply_replacements(&game, &mut event);
        assert_eq!(result, ReplacementResult::NotReplaced);
    }

    #[test]
    fn damage_to_card_prevented() {
        let mut game = make_game();
        let alice = PlayerId(0);
        let shield = add_creature_with_abilities(
            &mut game,
            alice,
            "Shield",
            vec!["R$ Event$ DamageDone | ValidTarget$ Card | Prevent$ True".to_string()],
        );
        put_on_battlefield(&mut game, shield, alice);

        let target_creature = add_creature_with_abilities(&mut game, alice, "Target", vec![]);
        put_on_battlefield(&mut game, target_creature, alice);

        let mut event = ReplacementEvent::DamageToCard {
            target: target_creature,
            amount: 3,
            source: None,
        };
        let result = apply_replacements(&game, &mut event);
        assert_eq!(result, ReplacementResult::Prevented);
        if let ReplacementEvent::DamageToCard { amount, .. } = event {
            assert_eq!(amount, 0);
        } else {
            panic!("unexpected event type");
        }
    }

    // ── Destroy replacement tests ─────────────────────────────────────────

    #[test]
    fn indestructible_destroy_replaced() {
        let mut game = make_game();
        let alice = PlayerId(0);
        // Indestructible: "If ~ would be destroyed, instead it isn't."
        let cid = add_creature_with_abilities(
            &mut game,
            alice,
            "Indestructible Bear",
            vec!["R$ Event$ Destroy | ValidCard$ Card.Self".to_string()],
        );
        put_on_battlefield(&mut game, cid, alice);

        let mut event = ReplacementEvent::Destroy { target: cid };
        let result = apply_replacements(&game, &mut event);
        assert_eq!(result, ReplacementResult::Replaced);
    }

    #[test]
    fn destroy_not_replaced_for_other_card() {
        let mut game = make_game();
        let alice = PlayerId(0);
        // Indestructible creature — protects only itself.
        let indestructible = add_creature_with_abilities(
            &mut game,
            alice,
            "Indestructible Bear",
            vec!["R$ Event$ Destroy | ValidCard$ Card.Self".to_string()],
        );
        let other = add_creature_with_abilities(&mut game, alice, "Mortal Bear", vec![]);
        put_on_battlefield(&mut game, indestructible, alice);
        put_on_battlefield(&mut game, other, alice);

        let mut event = ReplacementEvent::Destroy { target: other };
        let result = apply_replacements(&game, &mut event);
        assert_eq!(result, ReplacementResult::NotReplaced);
    }

    // ── Moved replacement tests ───────────────────────────────────────────

    #[test]
    fn moved_destination_updated_to_exile() {
        let mut game = make_game();
        let alice = PlayerId(0);
        // "If ~ would be put into a graveyard from the battlefield, exile it instead."
        let cid = add_creature_with_abilities(
            &mut game,
            alice,
            "Exile Bear",
            vec!["R$ Event$ Moved | Destination$ Graveyard | Origin$ Battlefield | ValidCard$ Card.Self | NewDestination$ Exile".to_string()],
        );
        put_on_battlefield(&mut game, cid, alice);

        let mut event = ReplacementEvent::Moved {
            card: cid,
            origin: ZoneType::Battlefield,
            destination: ZoneType::Graveyard,
        };
        let result = apply_replacements(&game, &mut event);
        assert_eq!(result, ReplacementResult::Updated);
        if let ReplacementEvent::Moved { destination, .. } = event {
            assert_eq!(destination, ZoneType::Exile);
        } else {
            panic!("unexpected event type");
        }
    }

    #[test]
    fn moved_not_replaced_wrong_origin() {
        let mut game = make_game();
        let alice = PlayerId(0);
        let cid = add_creature_with_abilities(
            &mut game,
            alice,
            "Exile Bear",
            vec!["R$ Event$ Moved | Destination$ Graveyard | Origin$ Battlefield | ValidCard$ Card.Self | NewDestination$ Exile".to_string()],
        );
        put_on_battlefield(&mut game, cid, alice);

        // Card moving from Hand → Graveyard: Origin doesn't match Battlefield.
        let mut event = ReplacementEvent::Moved {
            card: cid,
            origin: ZoneType::Hand,
            destination: ZoneType::Graveyard,
        };
        let result = apply_replacements(&game, &mut event);
        assert_eq!(result, ReplacementResult::NotReplaced);
    }

    // ── No effects test ───────────────────────────────────────────────────

    #[test]
    fn no_effects_returns_not_replaced() {
        let game = make_game();
        let mut event = ReplacementEvent::Draw { player: PlayerId(0) };
        let result = apply_replacements(&game, &mut event);
        assert_eq!(result, ReplacementResult::NotReplaced);
    }
}
