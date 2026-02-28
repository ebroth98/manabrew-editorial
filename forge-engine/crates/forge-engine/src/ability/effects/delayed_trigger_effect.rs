use super::{evaluate_svar, resolve_defined_player, EffectContext};
use crate::event::TriggerType;
use crate::spellability::SpellAbility;
use crate::trigger::{parse_trigger, TriggerMode};

/// Mirrors Java's `DelayedTriggerEffect` (core path).
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let Some(source_id) = sa.source else {
        return;
    };

    let mut next_id = 0;
    let Some(parsed) = parse_trigger(&sa.ability_text, &mut next_id) else {
        return;
    };
    let Some(mode) = trigger_mode_to_type(&parsed.mode) else {
        return;
    };

    let execute_svar = if let Some(exec) = sa.params.get("Execute") {
        if let Some(svar_text) = ctx.game.card(source_id).svars.get(exec) {
            svar_text.clone()
        } else {
            exec.clone()
        }
    } else {
        return;
    };

    let mut remembered_amount = 0;
    if sa
        .params
        .get("RememberNumber")
        .is_some_and(|v| v.eq_ignore_ascii_case("True"))
    {
        remembered_amount += ctx.game.card(source_id).remembered_cmc.iter().copied().sum::<i32>();
    }
    if let Some(svar_name) = sa.params.get("RememberSVarAmount") {
        if let Some(expr) = ctx.game.card(source_id).svars.get(svar_name) {
            remembered_amount += evaluate_svar(expr, sa);
        }
    }

    // Full ThisTurn / NextTurn / UpcomingTurn routing is still broader work;
    // current parity-critical cards use direct registration.
    let controller = if let Some(def_player) = sa.params.get("DelayedTriggerDefinedPlayer") {
        resolve_defined_player(def_player, sa.activating_player, ctx.game)
            .unwrap_or(sa.activating_player)
    } else {
        sa.activating_player
    };

    ctx.trigger_handler
        .register_delayed_trigger(crate::trigger::handler::DelayedTrigger {
            mode,
            trigger_mode: parsed.mode,
            execute_svar,
            controller,
            source_card: source_id,
            target_card: None,
            remembered_amount,
        });
}

fn trigger_mode_to_type(mode: &TriggerMode) -> Option<TriggerType> {
    Some(match mode {
        TriggerMode::ChangesZone { .. } => TriggerType::ChangesZone,
        TriggerMode::Phase { .. } => TriggerType::Phase,
        TriggerMode::SpellCast { .. } => TriggerType::SpellCast,
        TriggerMode::Attacks { .. } => TriggerType::Attacks,
        TriggerMode::DamageDone { .. } => TriggerType::DamageDone,
        TriggerMode::Countered { .. } => TriggerType::Countered,
        TriggerMode::Blocks { .. } => TriggerType::Blocks,
        TriggerMode::AttackerBlocked { .. } => TriggerType::AttackerBlocked,
        TriggerMode::AttackerUnblocked { .. } => TriggerType::AttackerUnblocked,
        TriggerMode::LifeGained { .. } => TriggerType::LifeGained,
        TriggerMode::LifeLost { .. } => TriggerType::LifeLost,
        TriggerMode::CounterAdded { .. } => TriggerType::CounterAdded,
        TriggerMode::CounterRemoved { .. } => TriggerType::CounterRemoved,
        TriggerMode::Sacrificed { .. } => TriggerType::Sacrificed,
        TriggerMode::Drawn { .. } => TriggerType::Drawn,
        TriggerMode::Milled { .. } => TriggerType::Milled,
        TriggerMode::Taps { .. } => TriggerType::Taps,
        TriggerMode::Untaps { .. } => TriggerType::Untaps,
        TriggerMode::Transformed { .. } => TriggerType::Transformed,
        TriggerMode::Attached { .. } => TriggerType::Attached,
        TriggerMode::Unattached { .. } => TriggerType::Unattached,
        TriggerMode::LandPlayed { .. } => TriggerType::LandPlayed,
        TriggerMode::BecomesTarget { .. } => TriggerType::BecomesTarget,
        TriggerMode::TapsForMana { .. } => TriggerType::TapsForMana,
        TriggerMode::AbilityActivated { .. } => TriggerType::AbilityActivated,
        TriggerMode::Explored { .. } => TriggerType::Explored,
        TriggerMode::BecomeMonarch { .. } => TriggerType::BecomeMonarch,
        TriggerMode::DamageDealtOnce { .. } => TriggerType::DamageDealtOnce,
        TriggerMode::Destroyed { .. } => TriggerType::Destroyed,
        TriggerMode::Exiled { .. } => TriggerType::Exiled,
        TriggerMode::TokenCreated { .. } => TriggerType::TokenCreated,
        TriggerMode::SpellCopied { .. } => TriggerType::SpellCopied,
        TriggerMode::AttackersDeclared { .. } => TriggerType::AttackersDeclared,
        TriggerMode::BlockersDeclared => TriggerType::BlockersDeclared,
        TriggerMode::ChangesZoneAll { .. } => TriggerType::ChangesZoneAll,
        TriggerMode::ChangesController { .. } => TriggerType::ChangesController,
        TriggerMode::TurnBegin { .. } => TriggerType::TurnBegin,
        TriggerMode::DamageDoneOnce { .. } => TriggerType::DamageDoneOnce,
        TriggerMode::SpellCastAll { .. } => TriggerType::SpellCastAll,
        TriggerMode::LifeLostAll { .. } => TriggerType::LifeLostAll,
        TriggerMode::CounterAddedOnce { .. } => TriggerType::CounterAddedOnce,
        TriggerMode::DiscardedAll { .. } => TriggerType::Discarded,
        TriggerMode::SacrificedOnce { .. } => TriggerType::Sacrificed,
        TriggerMode::Cycled { .. } => TriggerType::Cycled,
        TriggerMode::PhasedIn { .. } => TriggerType::PhasedIn,
        TriggerMode::PhasedOut { .. } => TriggerType::PhasedOut,
        TriggerMode::Always => TriggerType::Always,
        TriggerMode::Immediate => TriggerType::Always,
        TriggerMode::Surveil { .. } => TriggerType::Surveil,
        TriggerMode::Scry { .. } => TriggerType::Scry,
        TriggerMode::Foretell { .. } => TriggerType::Foretell,
        TriggerMode::SearchedLibrary { .. } => TriggerType::SearchedLibrary,
        TriggerMode::Shuffled { .. } => TriggerType::Shuffled,
        TriggerMode::ManaAdded { .. } => TriggerType::ManaAdded,
        TriggerMode::TokenCreatedOnce { .. } => TriggerType::TokenCreated,
        TriggerMode::TapAll { .. } => TriggerType::Taps,
        TriggerMode::UntapAll { .. } => TriggerType::Untaps,
        TriggerMode::BecomesTargetOnce { .. } => TriggerType::BecomesTarget,
        TriggerMode::AttackerBlockedByCreature { .. } => TriggerType::AttackerBlocked,
        TriggerMode::AttackerBlockedOnce { .. } => TriggerType::AttackerBlocked,
        TriggerMode::AttackerUnblockedOnce { .. } => TriggerType::AttackerUnblocked,
        TriggerMode::SpellCastOnce { .. } => TriggerType::SpellCast,
        TriggerMode::SpellCastOfType { .. } => TriggerType::SpellCast,
        TriggerMode::DamageAll { .. } => TriggerType::DamageDone,
        TriggerMode::DamagePreventedOnce { .. } => TriggerType::DamageDone,
        TriggerMode::ExcessDamage { .. } => TriggerType::DamageDone,
        TriggerMode::LifeGainedAll { .. } => TriggerType::LifeGained,
        TriggerMode::CounterRemovedOnce { .. } => TriggerType::CounterRemoved,
    })
}

