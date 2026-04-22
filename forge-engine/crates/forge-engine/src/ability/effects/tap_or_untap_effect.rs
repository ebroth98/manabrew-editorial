use forge_foundation::ZoneType;

use super::EffectContext;
use crate::agent::BinaryChoiceKind;
use crate::ids::CardId;
use crate::parsing::keys;
use crate::spellability::SpellAbility;

/// `SP$ TapOrUntap` — choose tap or untap for the targeted/defined permanent.
///
/// Mirrors Java `TapOrUntapEffect.java` binary prompt behavior.
/// Struct form of this effect so it can participate in the
/// `SpellAbilityEffect` trait hierarchy — mirrors Java's
/// `TapOrUntapEffect` class extending `SpellAbilityEffect`.
#[forge_engine_macros::spell_effect(TapOrUntapEffect)]
fn resolve(ctx: &mut EffectContext, sa: &crate::spellability::SpellAbility) {
    let controller = sa.activating_player;
    let source_name = sa
        .source
        .map(|cid| ctx.game.card(cid).card_name.clone())
        .unwrap_or_else(|| "Ability".to_string());

    let mut candidates: Vec<CardId> = Vec::new();
    if let Some(target) = sa.target_chosen.target_card {
        candidates.push(target);
    } else if let Some(source) = sa.source {
        if sa.params.get(crate::parsing::keys::DEFINED) == Some("Self") {
            candidates.push(source);
        }
    }

    for card_id in candidates {
        if ctx.game.card(card_id).zone != ZoneType::Battlefield {
            continue;
        }
        let should_tap = if sa.params.is_true(keys::TOGGLE) {
            !ctx.game.card(card_id).tapped
        } else {
            let prompt = format!("Tap or untap {}?", ctx.game.card(card_id).card_name);
            ctx.agents[controller.index()].choose_binary(
                controller,
                &prompt,
                BinaryChoiceKind::TapOrUntap,
                None,
                Some(&source_name),
                sa.api,
            )
        };

        if should_tap {
            if ctx.game.tap(card_id) {
                ctx.trigger_handler.run_trigger(
                    crate::trigger::TriggerType::Taps,
                    crate::event::RunParams {
                        card: Some(card_id),
                        player: Some(controller),
                        ..Default::default()
                    },
                    false,
                );
            }
        } else if ctx.game.untap(card_id) {
            ctx.trigger_handler.run_trigger(
                crate::trigger::TriggerType::Untaps,
                crate::event::RunParams {
                    card: Some(card_id),
                    player: Some(controller),
                    ..Default::default()
                },
                false,
            );
        }
    }
}
