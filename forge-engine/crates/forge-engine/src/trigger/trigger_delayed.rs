use crate::ability::effects::{evaluate_svar, resolve_defined_player, EffectContext};
use crate::event::TriggerType;
use crate::parsing::keys;
use crate::spellability::SpellAbility;
use crate::trigger::trigger::{parse_trigger, TriggerMode};

/// Trigger-module owned implementation of delayed trigger registration.
/// Mirrors Java's `TriggerDelayed`.
pub fn resolve_delayed_trigger(ctx: &mut EffectContext, sa: &SpellAbility) {
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

    let execute_svar = if let Some(exec) = sa.params.get(keys::EXECUTE) {
        if let Some(svar_text) = ctx.game.card(source_id).get_s_var(exec) {
            svar_text.to_string()
        } else {
            exec.to_string()
        }
    } else {
        return;
    };

    let mut remembered_amount = 0;
    if sa
        .params
        .get(keys::REMEMBER_NUMBER)
        .is_some_and(|v| v.eq_ignore_ascii_case("True"))
    {
        remembered_amount += ctx
            .game
            .card(source_id)
            .remembered_cmc
            .iter()
            .copied()
            .sum::<i32>();
    }
    if let Some(svar_name) = sa.params.get(keys::REMEMBER_SVAR_AMOUNT) {
        if let Some(expr) = ctx.game.card(source_id).get_s_var(svar_name) {
            remembered_amount += evaluate_svar(expr, sa);
        }
    }

    let controller = if let Some(def_player) = sa.params.get(keys::DELAYED_TRIGGER_DEFINED_PLAYER) {
        resolve_defined_player(def_player, sa.activating_player, ctx.game)
            .unwrap_or(sa.activating_player)
    } else {
        sa.activating_player
    };
    let mut remembered_lki_cards = Vec::new();
    if sa.params.get(keys::REMEMBER_OBJECTS).is_some_and(|value| {
        value
            .split(',')
            .any(|token| token.trim() == "RememberedLKI")
    }) {
        remembered_lki_cards = ctx.game.card(source_id).remembered_cards.clone();
    }

    // `RememberObjects$ Remembered` — snapshot the source card's current
    // remembered_cards into the delayed trigger so the executed ability sees
    // them later via `SpellAbility::trigger_remembered`. Ashling uses this to
    // track the token copy it created for its end-step sacrifice clause.
    let remembered_cards: Vec<crate::ids::CardId> = match sa.params.get("RememberObjects") {
        Some("Remembered") => ctx.game.card(source_id).remembered_cards.clone(),
        _ => Vec::new(),
    };

    let delayed = crate::trigger::handler::DelayedTrigger {
        mode,
        trigger_mode: parsed.mode,
        execute_svar,
        controller,
        source_card: source_id,
        created_turn: ctx.game.turn.turn_number,
        created_phase: ctx.game.turn.phase,
        target_card: None,
        remembered_amount,
        remembered_cards,
        remembered_lki_cards,
    };
    if sa.params.has("ThisTurn") {
        ctx.trigger_handler
            .register_this_turn_delayed_trigger(delayed);
    } else if sa.params.has(keys::DELAYED_TRIGGER_DEFINED_PLAYER) {
        ctx.trigger_handler
            .register_player_defined_delayed_trigger(controller, delayed);
    } else {
        ctx.trigger_handler.register_delayed_trigger(delayed);
    }
}

fn trigger_mode_to_type(mode: &TriggerMode) -> Option<TriggerType> {
    Some(match mode {
        TriggerMode::ChangesZone { .. } => TriggerType::ChangesZone,
        TriggerMode::Phase { .. } => TriggerType::Phase,
        TriggerMode::SpellCast { .. } => TriggerType::SpellCast,
        TriggerMode::AbilityCast { .. } => TriggerType::AbilityCast,
        TriggerMode::SpellAbilityCast { .. } => TriggerType::SpellAbilityCast,
        TriggerMode::Attacks { .. } => TriggerType::Attacks,
        TriggerMode::Fight { .. } => TriggerType::Fight,
        TriggerMode::FightOnce { .. } => TriggerType::FightOnce,
        TriggerMode::DamageDone { .. } => TriggerType::DamageDone,
        TriggerMode::Countered { .. } => TriggerType::Countered,
        TriggerMode::Blocks { .. } => TriggerType::Blocks,
        TriggerMode::AttackerBlocked { .. } => TriggerType::AttackerBlocked,
        TriggerMode::AttackerUnblocked { .. } => TriggerType::AttackerUnblocked,
        TriggerMode::LifeGained { .. } => TriggerType::LifeGained,
        TriggerMode::LifeLost { .. } => TriggerType::LifeLost,
        TriggerMode::PayLife { .. } => TriggerType::PayLife,
        TriggerMode::LosesGame { .. } => TriggerType::LosesGame,
        TriggerMode::Discover { .. } => TriggerType::Discover,
        TriggerMode::Elementalbend { .. } => TriggerType::Elementalbend,
        TriggerMode::Clashed { .. } => TriggerType::Clashed,
        TriggerMode::ManifestDread { .. } => TriggerType::ManifestDread,
        TriggerMode::ConjureAll { .. } => TriggerType::ConjureAll,
        TriggerMode::SeekAll { .. } => TriggerType::SeekAll,
        TriggerMode::CounterAdded { .. } => TriggerType::CounterAdded,
        TriggerMode::CounterRemoved { .. } => TriggerType::CounterRemoved,
        TriggerMode::Sacrificed { .. } => TriggerType::Sacrificed,
        TriggerMode::Discarded { .. } => TriggerType::Discarded,
        TriggerMode::Abandoned { .. } => TriggerType::Abandoned,
        TriggerMode::Adapt { .. } => TriggerType::Adapt,
        TriggerMode::BecomeRenowned { .. } => TriggerType::BecomeRenowned,
        TriggerMode::Evolved { .. } => TriggerType::Evolved,
        TriggerMode::Drawn { .. } => TriggerType::Drawn,
        TriggerMode::Milled { .. } => TriggerType::Milled,
        TriggerMode::MilledAll { .. } => TriggerType::MilledAll,
        TriggerMode::MilledOnce { .. } => TriggerType::MilledOnce,
        TriggerMode::PayEcho { .. } => TriggerType::PayEcho,
        TriggerMode::ClassLevelGained { .. } => TriggerType::ClassLevelGained,
        TriggerMode::Taps { .. } => TriggerType::Taps,
        TriggerMode::Untaps { .. } => TriggerType::Untaps,
        TriggerMode::Transformed { .. } => TriggerType::Transformed,
        TriggerMode::TurnFaceUp { .. } => TriggerType::TurnFaceUp,
        TriggerMode::Attached { .. } => TriggerType::Attached,
        TriggerMode::Unattached { .. } => TriggerType::Unattached,
        TriggerMode::LandPlayed { .. } => TriggerType::LandPlayed,
        TriggerMode::BecomesTarget { .. } => TriggerType::BecomesTarget,
        TriggerMode::BecomesCrewed { .. } => TriggerType::BecomesCrewed,
        TriggerMode::Championed { .. } => TriggerType::Championed,
        TriggerMode::Mentored { .. } => TriggerType::Mentored,
        TriggerMode::TapsForMana { .. } => TriggerType::TapsForMana,
        TriggerMode::AbilityActivated { .. } => TriggerType::AbilityActivated,
        TriggerMode::Explored { .. } => TriggerType::Explored,
        TriggerMode::Exploited { .. } => TriggerType::Exploited,
        TriggerMode::BecomeMonstrous { .. } => TriggerType::BecomeMonstrous,
        TriggerMode::BecomeMonarch { .. } => TriggerType::BecomeMonarch,
        TriggerMode::Investigated { .. } => TriggerType::Investigated,
        TriggerMode::Proliferate { .. } => TriggerType::Proliferate,
        TriggerMode::CompletedDungeon { .. } => TriggerType::CompletedDungeon,
        TriggerMode::CommitCrime { .. } => TriggerType::CommitCrime,
        TriggerMode::RingTemptsYou { .. } => TriggerType::RingTemptsYou,
        TriggerMode::PlanarDice { .. } => TriggerType::PlanarDice,
        TriggerMode::NewGame => TriggerType::NewGame,
        TriggerMode::DayTimeChanges => TriggerType::DayTimeChanges,
        TriggerMode::BecomesPlotted { .. } => TriggerType::BecomesPlotted,
        TriggerMode::Specializes { .. } => TriggerType::Specializes,
        TriggerMode::Trains { .. } => TriggerType::Trains,
        TriggerMode::Devoured { .. } => TriggerType::Devoured,
        TriggerMode::FullyUnlock { .. } => TriggerType::FullyUnlock,
        TriggerMode::AbilityResolves { .. } => TriggerType::AbilityResolves,
        TriggerMode::AbilityTriggered { .. } => TriggerType::AbilityTriggered,
        TriggerMode::UnlockDoor { .. } => TriggerType::UnlockDoor,
        TriggerMode::CounterAddedAll { .. } => TriggerType::CounterAddedAll,
        TriggerMode::CounterPlayerAddedAll { .. } => TriggerType::CounterPlayerAddedAll,
        TriggerMode::CounterTypeAddedAll { .. } => TriggerType::CounterTypeAddedAll,
        TriggerMode::CrewedSaddled { .. } => TriggerType::Crewed,
        TriggerMode::DamageDoneOnceByController { .. } => TriggerType::DamageDoneOnceByController,
        TriggerMode::ExcessDamageAll { .. } => TriggerType::ExcessDamageAll,
        TriggerMode::PhaseOutAll { .. } => TriggerType::PhaseOutAll,
        TriggerMode::Vote => TriggerType::Vote,
        TriggerMode::GiveGift { .. } => TriggerType::GiveGift,
        TriggerMode::VisitAttraction { .. } => TriggerType::VisitAttraction,
        TriggerMode::EnteredRoom { .. } => TriggerType::EnteredRoom,
        TriggerMode::PayCumulativeUpkeep { .. } => TriggerType::PayCumulativeUpkeep,
        TriggerMode::DamageDealtOnce { .. } => TriggerType::DamageDealtOnce,
        TriggerMode::Destroyed { .. } => TriggerType::Destroyed,
        TriggerMode::Exiled { .. } => TriggerType::Exiled,
        TriggerMode::TokenCreated { .. } => TriggerType::TokenCreated,
        TriggerMode::SpellCopied { .. } => TriggerType::SpellCopied,
        TriggerMode::SpellCopy { .. } => TriggerType::SpellCopy,
        TriggerMode::SpellAbilityCopy { .. } => TriggerType::SpellAbilityCopy,
        TriggerMode::SpellCastOrCopy { .. } => TriggerType::SpellCastOrCopy,
        TriggerMode::AttackersDeclared { .. } => TriggerType::AttackersDeclared,
        TriggerMode::BlockersDeclared => TriggerType::BlockersDeclared,
        TriggerMode::ChangesZoneAll { .. } => TriggerType::ChangesZoneAll,
        TriggerMode::ChangesController { .. } => TriggerType::ChangesController,
        TriggerMode::TurnBegin { .. } => TriggerType::TurnBegin,
        TriggerMode::DamageDoneOnce { .. } => TriggerType::DamageDoneOnce,
        TriggerMode::SpellCastAll { .. } => TriggerType::SpellCastAll,
        TriggerMode::LifeLostAll { .. } => TriggerType::LifeLostAll,
        TriggerMode::CounterAddedOnce { .. } => TriggerType::CounterAddedOnce,
        TriggerMode::DiscardedAll { .. } => TriggerType::DiscardedAll,
        TriggerMode::SacrificedOnce { .. } => TriggerType::SacrificedOnce,
        TriggerMode::Cycled { .. } => TriggerType::Cycled,
        TriggerMode::PhasedIn { .. } => TriggerType::PhasedIn,
        TriggerMode::PhasedOut { .. } => TriggerType::PhasedOut,
        TriggerMode::Always => TriggerType::Always,
        TriggerMode::Immediate => TriggerType::Immediate,
        TriggerMode::Surveil { .. } => TriggerType::Surveil,
        TriggerMode::Scry { .. } => TriggerType::Scry,
        TriggerMode::Foretell { .. } => TriggerType::Foretell,
        TriggerMode::SearchedLibrary { .. } => TriggerType::SearchedLibrary,
        TriggerMode::Shuffled { .. } => TriggerType::Shuffled,
        TriggerMode::ManaAdded { .. } => TriggerType::ManaAdded,
        TriggerMode::TokenCreatedOnce { .. } => TriggerType::TokenCreatedOnce,
        TriggerMode::TapAll { .. } => TriggerType::TapAll,
        TriggerMode::UntapAll { .. } => TriggerType::UntapAll,
        TriggerMode::BecomesTargetOnce { .. } => TriggerType::BecomesTargetOnce,
        TriggerMode::AttackerBlockedByCreature { .. } => TriggerType::AttackerBlockedByCreature,
        TriggerMode::AttackerBlockedOnce { .. } => TriggerType::AttackerBlockedOnce,
        TriggerMode::AttackerUnblockedOnce { .. } => TriggerType::AttackerUnblockedOnce,
        TriggerMode::SpellCastOnce { .. } => TriggerType::SpellCastOnce,
        TriggerMode::SpellCastOfType { .. } => TriggerType::SpellCastOfType,
        TriggerMode::DamageAll { .. } => TriggerType::DamageAll,
        TriggerMode::DamagePreventedOnce { .. } => TriggerType::DamagePreventedOnce,
        TriggerMode::ExcessDamage { .. } => TriggerType::ExcessDamage,
        TriggerMode::CounterRemovedOnce { .. } => TriggerType::CounterRemovedOnce,
        TriggerMode::Exerted { .. } => TriggerType::Exerted,
        TriggerMode::CollectEvidence { .. } => TriggerType::CollectEvidence,
        TriggerMode::Forage { .. } => TriggerType::Forage,
        TriggerMode::Enlisted { .. } => TriggerType::Enlisted,
        TriggerMode::FlippedCoin { .. } => TriggerType::FlippedCoin,
        TriggerMode::RolledDie { .. } => TriggerType::RolledDie,
        TriggerMode::RolledDieOnce { .. } => TriggerType::RolledDieOnce,
        TriggerMode::ManaExpend { .. } => TriggerType::ManaExpend,
        TriggerMode::Mutates { .. } => TriggerType::Mutates,
        TriggerMode::SetInMotion { .. } => TriggerType::SetInMotion,
        TriggerMode::CaseSolved { .. } => TriggerType::CaseSolved,
        TriggerMode::ClaimPrize { .. } => TriggerType::ClaimPrize,
        TriggerMode::TakesInitiative { .. } => TriggerType::TakeInitiative,
        TriggerMode::PlaneswalkedFrom { .. } => TriggerType::Planeswalk,
        TriggerMode::PlaneswalkedTo { .. } => TriggerType::Planeswalk,
        TriggerMode::CrankContraption { .. } => TriggerType::CrankAdvanced,
        TriggerMode::ChaosEnsues { .. } => TriggerType::ChaosEnsues,
        TriggerMode::BecomesSaddled { .. } => TriggerType::BecomesSaddled,
    })
}
