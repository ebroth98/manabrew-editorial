//! UnlockDoor — unlock a door on a Room card.
//! Ported from Java's UnlockDoorEffect: unlocks one side of a Room
//! enchantment, activating its abilities.

use forge_foundation::ZoneType;

use super::EffectContext;
use crate::ids::CardId;
use crate::parsing::keys;
use crate::spellability::SpellAbility;

pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let targets: Vec<CardId> = if let Some(target) = sa.target_chosen.target_card {
        vec![target]
    } else if let Some(source) = sa.source {
        vec![source]
    } else {
        return;
    };

    let mode = sa
        .params
        .get(keys::MODE)
        .unwrap_or("ThisDoor");

    for card_id in targets {
        if ctx.game.card(card_id).zone != ZoneType::Battlefield {
            continue;
        }

        match mode {
            "ThisDoor" => {
                // Unlock the door specified by the spell ability's card state
                ctx.game.card_mut(card_id).svars.insert(
                    "DoorUnlocked".to_string(),
                    "True".to_string(),
                );
            }
            "Unlock" => {
                // Unlock a chosen locked room
                ctx.game.card_mut(card_id).svars.insert(
                    "DoorUnlocked".to_string(),
                    "True".to_string(),
                );
            }
            "LockOrUnlock" => {
                // Toggle lock state
                let is_locked = ctx
                    .game
                    .card(card_id)
                    .svars
                    .get("DoorUnlocked")
                    .map_or(true, |v| v != "True");
                if is_locked {
                    ctx.game.card_mut(card_id).svars.insert(
                        "DoorUnlocked".to_string(),
                        "True".to_string(),
                    );
                } else {
                    ctx.game
                        .card_mut(card_id)
                        .svars
                        .remove("DoorUnlocked");
                }
            }
            _ => {}
        }
    }
}
