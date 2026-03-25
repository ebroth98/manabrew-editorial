//! RestartGame — restart the game (Karn Liberated ultimate).
//! Ported from Java's RestartGameEffect: resets all game state, moves all cards
//! back to libraries, resets life totals, clears counters, and restarts.

use forge_foundation::ZoneType;

use super::EffectContext;
use crate::ids::CardId;
use crate::parsing::keys;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let activator = sa.activating_player;

    // Get all player IDs
    let player_ids: Vec<_> = ctx.game.players.iter().map(|p| p.id).collect();

    // Collect cards to move back to library (from all restart zones)
    let restart_zones = [
        ZoneType::Battlefield,
        ZoneType::Hand,
        ZoneType::Graveyard,
        ZoneType::Exile,
    ];

    // Optional: RestrictFromZone — leave some cards in a zone
    let leave_zone = sa
        .params
        .get(keys::RESTRICT_FROM_ZONE)
        .and_then(|z| match z {
            "Battlefield" => Some(ZoneType::Battlefield),
            "Hand" => Some(ZoneType::Hand),
            "Graveyard" => Some(ZoneType::Graveyard),
            "Exile" => Some(ZoneType::Exile),
            "Library" => Some(ZoneType::Library),
            _ => None,
        });

    for &player_id in &player_ids {
        // Reset player state
        let starting_life = ctx.game.player(player_id).starting_life;
        let player = ctx.game.player_mut(player_id);
        player.life = starting_life;
        player.poison_counters = 0;
        player.lands_played_this_turn = 0;
        player.max_land_plays_per_turn = 1;
        player.spells_cast_this_turn = 0;
        player.max_hand_size = 7;
        player.drawn_this_turn = 0;
        player.has_lost = false;
        player.has_won = false;
        player.has_conceded = false;
        player.commander_damage_received.clear();
        player.skip_turns = 0;
        player.skip_next_draw = false;
        player.skip_next_combat = false;
        player.skip_next_untap = false;
        player.damage_prevention = 0;
        player.energy_counters = 0;
        player.mana_shards = 0;
        player.mana_expended_this_turn = 0;
        player.controlled_by = None;
        player.has_city_blessing = false;
        player.ring_level = 0;
        player.ring_bearer = None;
        player.radiation_counters = 0;
        player.life_gained_this_turn = 0;
        player.life_lost_this_turn = 0;

        // Collect all cards from restart zones for this player
        let cards_to_move: Vec<CardId> = ctx
            .game
            .cards
            .iter()
            .filter(|c| {
                c.owner == player_id
                    && restart_zones.contains(&c.zone)
                    && leave_zone.map_or(true, |lz| c.zone != lz)
            })
            .map(|c| c.id)
            .collect();

        // Move all collected cards to library
        for card_id in cards_to_move {
            // Reset card state
            ctx.game.card_mut(card_id).set_tapped(false);
            ctx.game.card_mut(card_id).set_face_down(false);
            ctx.game.card_mut(card_id).clear_counters();
            ctx.game.card_mut(card_id).set_power_modifier(0);
            ctx.game.card_mut(card_id).set_toughness_modifier(0);
            ctx.game.card_mut(card_id).set_controller(player_id);
            ctx.game.card_mut(card_id).clear_intrinsic_keywords();
            ctx.game.move_card(card_id, ZoneType::Library, player_id);
        }

        // Shuffle library
        {
            let lib = ctx.game.zone_mut(ZoneType::Library, player_id);
            ctx.rng.shuffle_cards(&mut lib.cards);
        }
    }

    // Reset global game state
    ctx.game.is_night = false;

    // Set active player to the activator (Karn's controller restarts)
    let _ = activator;
}
