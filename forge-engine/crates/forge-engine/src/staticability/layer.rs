//! CR 613 layer system — continuous effect application.
//!
//! Mirrors Java Forge's `GameAction.checkStaticAbilities()` and
//! `StaticAbilityContinuous.applyContinuousAbility()`.
//!
//! # How to use
//!
//! Call [`apply_continuous_effects`] after any event that could change which
//! static abilities are active (card entering/leaving the battlefield, spell
//! resolution, etc.):
//!
//! ```ignore
//! apply_continuous_effects(&mut game);
//! ```
//!
//! The function resets all derived fields (`static_power_modifier`,
//! `static_toughness_modifier`, `static_set_power`, `static_set_toughness`,
//! `granted_keywords`, `cant_attack_static`, `cant_block_static`) and
//! recomputes them from scratch.
//!
//! # Layer ordering (CR 613)
//!
//! 1. Copy effects (not yet implemented)
//! 2. Control-changing
//! 3. Text-changing (not yet implemented)
//! 4. Type-changing  → [`Layer::Type`]
//! 5. Color-changing → [`Layer::Color`]
//! 6. Ability-adding/removing → [`Layer::Ability`]
//! 7a. CDA P/T (not yet implemented)
//! 7b. Set P/T → [`Layer::SetPT`]
//! 7c. Modify P/T → [`Layer::ModifyPT`]
//! 7d. Counters (handled intrinsically by `Card::power()`)

use std::collections::BTreeMap;

use forge_foundation::ZoneType;

use crate::agent::PlayerAgent;
use crate::game::GameState;
use crate::ids::{CardId, PlayerId};
use crate::parsing::{keys, Params};
use crate::replacement::replacement_effect::ReplacementType;
use crate::staticability::{CardFilter, Layer, StaticMode};

// ── Effect collection ────────────────────────────────────────────────────────

/// An effect ready to be applied to a specific target card.
struct PendingEffect {
    /// CR 613 layer (used for sort ordering).
    layer: Layer,
    /// Target card index.
    target: CardId,
    /// Payload.
    kind: EffectKind,
}

enum EffectKind {
    SetController {
        controller: PlayerId,
    },
    AddPT {
        power: i32,
        toughness: i32,
    },
    SetPT {
        power: Option<i32>,
        toughness: Option<i32>,
    },
    GrantKeyword(String),
    /// Grant an activated ability (from AddAbility$). The string is the ability text.
    GrantAbility {
        text: String,
        svars: BTreeMap<String, String>,
    },
    /// Add a type/subtype to the card (`AddType$`). Mirrors Java layer 4.
    AddType(String),
    /// Grant a triggered ability (from AddTrigger$). The string is the raw trigger text.
    GrantTrigger {
        text: String,
        svars: BTreeMap<String, String>,
    },
}

struct ContinuousParamRefs<'a> {
    characteristic_defining: Option<&'a str>,
    affected: Option<&'a str>,
    valid_cards: Option<&'a str>,
    valid_card: Option<&'a str>,
    affected_zone: Option<&'a str>,
    gain_control: Option<&'a str>,
    add_power: Option<&'a str>,
    add_toughness: Option<&'a str>,
    add_type: Option<&'a str>,
    set_power: Option<&'a str>,
    set_toughness: Option<&'a str>,
    add_keyword: Option<&'a str>,
    add_ability: Option<&'a str>,
    add_trigger: Option<&'a str>,
}

impl<'a> ContinuousParamRefs<'a> {
    fn from_params(params: &'a Params) -> Self {
        let mut refs = Self {
            characteristic_defining: None,
            affected: None,
            valid_cards: None,
            valid_card: None,
            affected_zone: None,
            gain_control: None,
            add_power: None,
            add_toughness: None,
            add_type: None,
            set_power: None,
            set_toughness: None,
            add_keyword: None,
            add_ability: None,
            add_trigger: None,
        };
        for (key, value) in params.iter() {
            match key {
                keys::CHARACTERISTIC_DEFINING => refs.characteristic_defining = Some(value),
                keys::AFFECTED => refs.affected = Some(value),
                keys::VALID_CARDS => refs.valid_cards = Some(value),
                keys::VALID_CARD => refs.valid_card = Some(value),
                keys::AFFECTED_ZONE => refs.affected_zone = Some(value),
                keys::GAIN_CONTROL => refs.gain_control = Some(value),
                keys::ADD_POWER => refs.add_power = Some(value),
                keys::ADD_TOUGHNESS => refs.add_toughness = Some(value),
                keys::ADD_TYPE => refs.add_type = Some(value),
                keys::SET_POWER => refs.set_power = Some(value),
                keys::SET_TOUGHNESS => refs.set_toughness = Some(value),
                keys::ADD_KEYWORD => refs.add_keyword = Some(value),
                keys::ADD_ABILITY => refs.add_ability = Some(value),
                keys::ADD_TRIGGER => refs.add_trigger = Some(value),
                _ => {}
            }
        }
        refs
    }
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Recompute all continuously-applied static-ability effects for the current
/// game state.
///
/// This is the Rust equivalent of Java Forge's
/// `GameAction.checkStaticAbilities()` + `StaticAbilityContinuous.applyContinuousAbility()`.
///
/// **Call this** after:
/// - Any permanent enters or leaves the battlefield.
/// - Any spell or ability resolves.
/// - Any triggered ability fires.
/// - Before querying `can_attack()` / `can_block()` for combat legality.
pub fn apply_continuous_effects(game: &mut GameState) {
    let _perf_timer = crate::perf::ScopeTimer::start(
        crate::perf::Metric::ContinuousEffectsCalls,
        crate::perf::Metric::ContinuousEffectsNs,
    );
    let _params_lookup_scope =
        crate::perf::ParamsLookupScopeGuard::enter(crate::perf::ParamsLookupScope::Continuous);
    // ── 1. Reset all derived fields ──────────────────────────────────────
    for card in game.cards.iter_mut() {
        // Remove abilities granted by continuous effects (AddAbility$).
        // The base_ability_count tracks how many abilities the card originally had.
        if card.activated_abilities.len() > card.base_ability_count {
            card.activated_abilities.truncate(card.base_ability_count);
        }
        let intrinsic_trigger_count = card.base_trigger_count + card.pump_trigger_count;
        if card.triggers.len() > intrinsic_trigger_count {
            card.triggers.truncate(intrinsic_trigger_count);
        }
        card.static_power_modifier = 0;
        card.static_toughness_modifier = 0;
        // Preserve face-down morph P/T override (2/2); only reset for face-up cards.
        if !card.face_down {
            card.static_set_power = None;
            card.static_set_toughness = None;
        }
        card.granted_keywords.clear();
        card.granted_svars.clear();
        // Remove any subtypes previously added by AddType$ statics so this
        // cycle starts from the intrinsic type line.
        if !card.static_added_subtypes.is_empty() {
            let added = std::mem::take(&mut card.static_added_subtypes);
            card.type_line.subtypes.retain(|s| !added.contains(s));
        }
        card.cant_attack_static = false;
        card.cant_block_static = false;
    }

    // ── 1b. Keyword-derived restrictions ────────────────────────────────
    // Unleash: creatures with Unleash keyword and a +1/+1 counter can't block.
    for card in game.cards.iter_mut() {
        if card.zone == ZoneType::Battlefield
            && card.has_keyword("Unleash")
            && card.counter_count(&crate::card::CounterType::P1P1) > 0
        {
            card.cant_block_static = true;
        }
    }

    // ── 2. Build list of effects-to-apply (deferred to allow sorting) ────
    let mut pending: Vec<PendingEffect> = Vec::new();
    let mut cant_attack_targets: Vec<CardId> = Vec::new();
    let mut cant_block_targets: Vec<CardId> = Vec::new();

    for player_idx in 0..game.player_order.len() {
        let player = game.player_order[player_idx];
        for &source_id in game.cards_in_zone(ZoneType::Battlefield, player) {
            let source_card = game.card(source_id);
            let static_ability_count = source_card.static_abilities.len();

            for sa_idx in 0..static_ability_count {
                let source_card = game.card(source_id);
                let sa = &source_card.static_abilities[sa_idx];

                // Full static-ability condition gate (IsPresent$, CheckSVar$, Condition$, etc.).
                // Mirrors Java static ability checks before applying continuous effects.
                if !sa.check_conditions(source_card, game) {
                    continue;
                }

                let params = ContinuousParamRefs::from_params(&sa.params);

                // CharacteristicDefining statics always affect only the host card.
                // Mirrors Java StaticAbilityContinuous.getAffectedCards() line 1036.
                let is_cda = params
                    .characteristic_defining
                    .map(|v| v.eq_ignore_ascii_case("True"))
                    .unwrap_or(false);

                // Determine which cards are affected by this static ability.
                let affected_str = params
                    .affected
                    .or(params.valid_cards)
                    .or(params.valid_card)
                    .unwrap_or("Creature.YouControl");

                let mut apply_to_target = |target: CardId| match sa.mode {
                // ── Continuous: queue effect for later sorted application ────
                StaticMode::Continuous => {
                    // Java parity: one continuous static can contribute effects in
                    // multiple layers (e.g. Brothers Yamazaki adds both +2/+2 and Haste).
                    if let Some(gain_control) = params.gain_control {
                        let new_controller = match gain_control {
                            "You" | "YouCtrl" => Some(source_card.controller),
                            "Opponent" => Some(game.opponent_of(source_card.controller)),
                            _ => None,
                        };
                        if let Some(controller) = new_controller {
                            pending.push(PendingEffect {
                                layer: Layer::Control,
                                target,
                                kind: EffectKind::SetController { controller },
                            });
                        }
                    }

                    let add_power = params.add_power;
                    let add_toughness = params.add_toughness;
                    if add_power.is_some() || add_toughness.is_some() {
                        let p = resolve_add_pt_value(game, source_id, add_power);
                        let t = resolve_add_pt_value(game, source_id, add_toughness);
                        pending.push(PendingEffect {
                            layer: Layer::ModifyPT,
                            target,
                            kind: EffectKind::AddPT {
                                power: p,
                                toughness: t,
                            },
                        });
                    }

                    let add_type = params.add_type;
                    let source = game.card(source_id);
                    for added_type in resolve_added_types(source, add_type) {
                        pending.push(PendingEffect {
                            layer: Layer::Type,
                            target,
                            kind: EffectKind::AddType(added_type),
                        });
                    }

                    let set_power = params.set_power;
                    let set_toughness = params.set_toughness;
                    if set_power.is_some() || set_toughness.is_some() {
                        let sp = resolve_set_pt_value(game, source_id, set_power);
                        let st = resolve_set_pt_value(game, source_id, set_toughness);
                        pending.push(PendingEffect {
                            layer: Layer::SetPT,
                            target,
                            kind: EffectKind::SetPT {
                                power: sp,
                                toughness: st,
                            },
                        });
                    }

                    if let Some(kws) = params.add_keyword {
                        // AddKeyword$ supports multiple keywords separated by " & ".
                        for kw in kws.split('&').map(str::trim).filter(|s| !s.is_empty()) {
                            pending.push(PendingEffect {
                                layer: Layer::Ability,
                                target,
                                kind: EffectKind::GrantKeyword(kw.to_string()),
                            });
                        }
                    }

                    // AddType$ — grant a type/subtype (layer 4). Supports
                    // comma or " & " separated lists, e.g. Yavimaya, Cradle of
                    // Growth: AddType$ Forest.
                    if let Some(raw) = add_type {
                        for t in raw.split(|c| c == ',' || c == '&').map(str::trim) {
                            if t.is_empty() {
                                continue;
                            }
                            pending.push(PendingEffect {
                                layer: Layer::Type,
                                target,
                                kind: EffectKind::AddType(t.to_string()),
                            });
                        }
                    }

                    // AddAbility$ — grant an activated ability to the affected card.
                    // The value is an SVar name on the source card containing the ability text.
                    // E.g. Abundant Growth: AddAbility$ AbundantGrowthTap
                    //   SVar:AbundantGrowthTap:AB$ Mana | Cost$ T | Produced$ Any
                    if let Some(svar_name) = params.add_ability {
                        let source = game.card(source_id);
                        if let Some(ab_text) = source.svars.get(svar_name).cloned() {
                            pending.push(PendingEffect {
                                layer: Layer::Ability,
                                target,
                                kind: EffectKind::GrantAbility {
                                    text: ab_text,
                                    svars: source.svars.clone(),
                                },
                            });
                        }
                    }

                    if let Some(add_trigger) = params.add_trigger {
                        let source = game.card(source_id);
                        for svar_name in add_trigger
                            .split(" & ")
                            .map(str::trim)
                            .filter(|s| !s.is_empty())
                        {
                            if let Some(trig_text) = source.svars.get(svar_name).cloned() {
                                pending.push(PendingEffect {
                                    layer: Layer::Ability,
                                    target,
                                    kind: EffectKind::GrantTrigger {
                                        text: trig_text,
                                        svars: source.svars.clone(),
                                    },
                                });
                            }
                        }
                    }

                    let source = game.card(source_id);
                    for subtype in resolve_added_basic_land_types(source, add_type) {
                        if let Some(ab_text) = basic_land_mana_ability_text(&subtype) {
                            pending.push(PendingEffect {
                                layer: Layer::Ability,
                                target,
                                kind: EffectKind::GrantAbility {
                                    text: ab_text.to_string(),
                                    svars: BTreeMap::new(),
                                },
                            });
                        }
                    }
                }

                // ── Restriction statics: apply immediately (not layer-ordered) ──
                StaticMode::CantAttack => {
                    cant_attack_targets.push(target);
                }
                StaticMode::CantBlock => {
                    cant_block_targets.push(target);
                }

                // Attack-cost statics are checked at combat time, not continuously.
                StaticMode::CantAttackUnless
                | StaticMode::CantBlockUnless
                | StaticMode::CantBlockBy
                | StaticMode::OptionalAttackCost
                // Non-layer static modes are enforced by dedicated rule checks
                // in their own modules / gameplay paths (cast checks, targeting
                // checks, combat checks, trigger suppression, etc.), so they are
                // intentionally not applied in the continuous layer collector.
                | StaticMode::ETBTapped
                | StaticMode::CantBeCast
                | StaticMode::CantBeActivated
                | StaticMode::CantPlayLand
                | StaticMode::ReduceCost
                | StaticMode::IncreaseCost
                | StaticMode::SetCost
                | StaticMode::CantTarget
                | StaticMode::CantAttach
                | StaticMode::MustAttack
                | StaticMode::MustBlock
                | StaticMode::Panharmonicon
                | StaticMode::CantGainLosePayLife
                | StaticMode::CantDraw
                | StaticMode::CantExile
                | StaticMode::CantSacrifice
                | StaticMode::CantRegenerate
                | StaticMode::DisableTriggers
                | StaticMode::CantPutCounter
                | StaticMode::CastWithFlash
                | StaticMode::BlockRestrict
                | StaticMode::AttackRestrict
                | StaticMode::CanAttackDefender
                | StaticMode::IgnoreHexproof
                | StaticMode::IgnoreShroud
                | StaticMode::IgnoreLegendRule
                | StaticMode::MustTarget
                | StaticMode::AssignCombatDamageAsUnblocked
                | StaticMode::AssignNoCombatDamage
                | StaticMode::CombatDamageToughness
                | StaticMode::NoCleanupDamage
                | StaticMode::InfectDamage
                | StaticMode::WitherDamage
                | StaticMode::ColorlessDamageSource
                | StaticMode::CountersRemain
                | StaticMode::MaxCounter
                | StaticMode::ManaConvert
                | StaticMode::UnspentMana
                | StaticMode::ManaBurn
                | StaticMode::ActivateAbilityAsIfHaste
                | StaticMode::CanAdapt
                | StaticMode::AlternativeCost
                | StaticMode::CantAttackBlock
                | StaticMode::CantBeCopied
                | StaticMode::CantBeSuspected
                | StaticMode::CantBecomeMonarch
                | StaticMode::CantChangeDayTime
                | StaticMode::CantCrew
                | StaticMode::CantDiscard
                | StaticMode::CantPhaseIn
                | StaticMode::CantPhaseOut
                | StaticMode::CantTransform
                | StaticMode::CantVenture
                | StaticMode::Devotion
                | StaticMode::CanExhaust
                | StaticMode::FlipCoinMod
                | StaticMode::GainLifeRadiation
                | StaticMode::IgnoreLandwalk
                | StaticMode::NumLoyaltyAct
                | StaticMode::PlotZone
                | StaticMode::SurveilNum
                | StaticMode::TapPowerValue
                | StaticMode::TurnReversed
                | StaticMode::PhaseReversed
                | StaticMode::UntapOtherPlayer
                | StaticMode::CanBlockIfReach
                | StaticMode::BlockTapped
                | StaticMode::CanAttackIfHaste
                | StaticMode::MinMaxBlocker
                | StaticMode::AttackVigilance
                | StaticMode::CantPreventDamage
                | StaticMode::CantGainLife
                | StaticMode::CantLoseLife
                | StaticMode::CantPayLife
                | StaticMode::CantChangeLife
                | StaticMode::Other(_) => {}
            };

                if is_cda {
                    // CDAs always affect only the source card itself.
                    if source_card.zone == ZoneType::Battlefield {
                        apply_to_target(source_id);
                    }
                } else if affected_str.eq_ignore_ascii_case("Card.Self")
                    || affected_str.starts_with("Card.Self+")
                {
                    // Self-referencing static: only affects the source card itself,
                    // but qualifiers after "+" must still be checked (e.g.
                    // "Card.Self+counters_GE2_CHARGE" only matches when the card
                    // has >=2 charge counters). Mirrors Java's
                    // StaticAbilityContinuous.getAffectedCards() which validates
                    // all qualifiers even for self-referencing statics.
                    if source_card.zone == ZoneType::Battlefield
                        && crate::card::valid_filter::matches_valid_card(
                            affected_str,
                            source_card,
                            source_card,
                        )
                    {
                        apply_to_target(source_id);
                    }
                } else if affected_str.eq_ignore_ascii_case("Card.EnchantedBy")
                    || affected_str.contains(".EquippedBy")
                    || affected_str.contains(".EnchantedBy")
                {
                    // Aura / Equipment static effects: affect what this source is
                    // attached to. Java treats EquippedBy and EnchantedBy
                    // identically: both resolve to the entity the source is
                    // attached to. (e.g. Short Sword: "Creature.EquippedBy",
                    // Control Magic: "Card.EnchantedBy")
                    if let Some(cid) = source_card.attached_to {
                        if game.card(cid).zone == ZoneType::Battlefield {
                            apply_to_target(cid);
                        }
                    }
                } else {
                    let filter = CardFilter::parse(affected_str);
                    // AffectedZone$ overrides the default Battlefield filter (e.g.
                    // Ashling, the Limitless grants Evoke:4 to Elementals in Hand).
                    let affected_zones: Option<Vec<ZoneType>> = params.affected_zone.map(|s| {
                        s.split(',')
                            .filter_map(|z| ZoneType::from_str_compat(z.trim()))
                            .collect()
                    });
                    for card in &game.cards {
                        let zone_matches = match &affected_zones {
                            Some(zones) => zones.contains(&card.zone),
                            None => card.zone == ZoneType::Battlefield,
                        };
                        if zone_matches && filter.matches_with_game(card, source_card, game) {
                            apply_to_target(card.id);
                        }
                    }
                }
            }
        }
    }

    for target in cant_attack_targets {
        game.cards[target.index()].cant_attack_static = true;
    }
    for target in cant_block_targets {
        game.cards[target.index()].cant_block_static = true;
    }

    // ── 4. Sort by layer then apply ──────────────────────────────────────
    // CR 613.1: apply layers 1→7c in order. Within the same layer, timestamp
    // ordering is preserved by the stable sort (sources were collected in
    // card-declaration order, which approximates timestamp order).
    pending.sort_by_key(|e| e.layer);

    for effect in pending {
        match effect.kind {
            EffectKind::SetController { controller } => {
                game.change_controller(effect.target, controller);
            }
            EffectKind::AddPT { power, toughness } => {
                let card = &mut game.cards[effect.target.index()];
                card.static_power_modifier += power;
                card.static_toughness_modifier += toughness;
            }
            EffectKind::SetPT { power, toughness } => {
                let card = &mut game.cards[effect.target.index()];
                // Layer 7b: override the base P/T for this calculation cycle.
                // We use `static_set_power` rather than mutating `base_power`
                // so the original base value is preserved for the next reset.
                if let Some(p) = power {
                    card.static_set_power = Some(p);
                }
                if let Some(t) = toughness {
                    card.static_set_toughness = Some(t);
                }
            }
            EffectKind::GrantKeyword(kw) => {
                let card = &mut game.cards[effect.target.index()];
                card.granted_keywords.add(&kw);
                if let Some(cost_str) = crate::keyword::extract_keyword_cost_str(&kw, "Ward") {
                    let next_id = card
                        .triggers
                        .iter()
                        .map(|t| t.id)
                        .max()
                        .unwrap_or(0)
                        .saturating_add(1);
                    let mut next_id_mut = next_id;
                    let execute = format!("TrigWardGranted{}", next_id);
                    let raw = format!(
                        "Mode$ BecomesTarget | ValidSource$ SpellAbility.OppCtrl | ValidTarget$ Card.Self | Secondary$ True | Execute$ {} | TriggerZones$ Battlefield | TriggerDescription$ Ward",
                        execute
                    );
                    if let Some(mut trig) = crate::trigger::parse_trigger(&raw, &mut next_id_mut) {
                        trig.execute = execute.clone();
                        card.add_trigger(trig);
                    }
                    card.granted_svars.insert(
                        execute,
                        format!(
                            "DB$ Counter | Defined$ TriggeredSourceSA | UnlessCost$ {cost_str}"
                        ),
                    );
                }
            }
            EffectKind::AddType(t) => {
                let card = &mut game.cards[effect.target.index()];
                if !card.type_line.subtypes.iter().any(|s| s == &t) {
                    card.type_line.subtypes.push(t.clone());
                    card.static_added_subtypes.push(t);
                }
            }
            EffectKind::GrantAbility { text, svars } => {
                // Parse the ability text and add it to the target's activated abilities.
                // This grants abilities like "{T}: Add one mana of any color."
                game.cards[effect.target.index()]
                    .granted_svars
                    .extend(svars);
                let next_idx = game.cards[effect.target.index()].activated_abilities.len();
                if let Some(ab) =
                    crate::ability::activated::parse_activated_ability(&text, next_idx)
                {
                    game.cards[effect.target.index()]
                        .activated_abilities
                        .push(ab);
                }
            }
            EffectKind::GrantTrigger { text, svars } => {
                game.cards[effect.target.index()]
                    .granted_svars
                    .extend(svars);
                let next_id = game.cards[effect.target.index()]
                    .triggers
                    .iter()
                    .map(|t| t.id)
                    .max()
                    .unwrap_or(0)
                    .saturating_add(1);
                let mut next_id_mut = next_id;
                if let Some(trig) = crate::trigger::parse_trigger(&text, &mut next_id_mut) {
                    game.cards[effect.target.index()].add_trigger(trig);
                }
            }
        }
    }

    // Rebuild intrinsic basic-land mana abilities after type-changing continuous
    // effects have been applied (e.g. Urborg making lands into Swamps).
    for card in game.cards.iter_mut() {
        if card.zone == ZoneType::Battlefield {
            card.generate_basic_land_mana_abilities();
        }
    }
}

/// Apply ETB-tapped effects to `entering_card` as it enters the battlefield.
///
/// Checks:
/// 1. The card's own static abilities for `Mode$ ETBTapped` (intrinsic).
/// 2. Any other battlefield permanent with `Mode$ ETBTapped` whose filter
///    matches the entering card (extrinsic, e.g. Imposing Sovereign).
///
/// Call this immediately after [`GameState::move_card`] resolves a
/// `Battlefield` destination and before triggers are fired.
pub fn apply_etb_tapped(game: &mut GameState, entering_card: CardId) {
    apply_etb_tapped_with_agents(game, entering_card, None);
}

fn applicable_etb_tapped_replacement_sources(
    game: &GameState,
    entering_card: CardId,
) -> Vec<(CardId, String)> {
    let mut repl_sources: Vec<(CardId, String, String)> = Vec::new();
    for c in &game.cards {
        if c.zone != ZoneType::Battlefield {
            continue;
        }
        for re in &c.replacement_effects {
            if re.event == ReplacementType::Moved
                && re.replace_with() == Some("ETBTapped")
                && re.ir.destination_zone == Some(ZoneType::Battlefield)
                && re.active_in_zone(ZoneType::Battlefield)
            {
                let filter = re
                    .ir
                    .valid_card_text
                    .as_deref()
                    .unwrap_or("Card.Self")
                    .to_string();
                let desc = re.description(c, game);
                repl_sources.push((c.id, filter, desc));
            }
        }
    }

    repl_sources
        .into_iter()
        .filter_map(|(source_id, filter_str, desc)| {
            let tapped = if filter_str == "Card.Self" || filter_str.is_empty() {
                source_id == entering_card
            } else {
                let source = &game.cards[source_id.index()];
                let filter = CardFilter::parse(&filter_str);
                filter.matches_with_game(&game.cards[entering_card.index()], source, game)
            };
            tapped.then_some((source_id, desc))
        })
        .collect()
}

pub fn prompt_etb_tapped_replacement_with_agents(
    game: &mut GameState,
    entering_card: CardId,
    agents: &mut [Box<dyn PlayerAgent>],
) {
    let applicable = applicable_etb_tapped_replacement_sources(game, entering_card);
    if applicable.is_empty() {
        return;
    }

    let affected_player = game.cards[entering_card.index()].controller;
    let descriptions: Vec<String> = applicable
        .iter()
        .map(|(source_id, desc)| format!("{}: {}", game.card(*source_id).card_name, desc))
        .collect();
    let _chosen = agents[affected_player.index()]
        .choose_single_replacement_effect(affected_player, &descriptions)
        .min(applicable.len().saturating_sub(1));
}

pub fn apply_etb_tapped_with_agents(
    game: &mut GameState,
    entering_card: CardId,
    mut agents: Option<&mut [Box<dyn PlayerAgent>]>,
) {
    // Collect all ETBTapped sources: (source_id, filter_str).
    // We need owned data to avoid aliasing the cards slice while mutating.
    let etb_sources: Vec<(CardId, String)> = game
        .cards
        .iter()
        .filter(|c| c.zone == ZoneType::Battlefield)
        .flat_map(|c| {
            c.static_abilities.iter().filter_map(move |sa| {
                if sa.mode == StaticMode::ETBTapped {
                    let filter_str = sa
                        .ir
                        .valid_cards_text
                        .clone()
                        .or_else(|| sa.ir.affected_text.clone())
                        // Default: the card itself (intrinsic self-ETBTapped).
                        .unwrap_or_else(|| "Card.Self".to_string());
                    Some((c.id, filter_str))
                } else {
                    None
                }
            })
        })
        .collect();

    for (source_id, filter_str) in etb_sources {
        // "Card.Self" means only the card that owns the ability.
        let tapped = if filter_str == "Card.Self" || filter_str.is_empty() {
            source_id == entering_card
        } else {
            let source = &game.cards[source_id.index()];
            let filter = CardFilter::parse(&filter_str);
            filter.matches_with_game(&game.cards[entering_card.index()], source, game)
        };

        if tapped {
            game.cards[entering_card.index()].tapped = true;
            return; // once tapped, no need to check further sources
        }
    }

    // ── Second pass: check replacement effects for ReplaceWith$ ETBTapped ──
    // Many cards (e.g. Path of Ancestry, Temple of Mystery) use:
    //   R:Event$ Moved | Destination$ Battlefield | ValidCard$ Card.Self | ReplaceWith$ ETBTapped
    // Extrinsic sources (e.g. Kismet) may use broader ValidCard filters.
    let applicable = applicable_etb_tapped_replacement_sources(game, entering_card);
    if applicable.is_empty() {
        return;
    }

    if let Some(agents) = agents.as_deref_mut() {
        prompt_etb_tapped_replacement_with_agents(game, entering_card, agents);
    }

    game.cards[entering_card.index()].tapped = true;
}

/// Check if a card has a shock-land-style "enters tapped unless you pay life" effect.
///
/// Looks for `R:Event$ Moved | Destination$ Battlefield | ReplaceWith$ <SVar>`
/// where the SVar is `DB$ Tap | ETB$ True | UnlessCost$ PayLife<N>`.
///
/// Returns `Some(life_cost)` if found (e.g. `Some(2)` for shock lands), `None` otherwise.
/// Called from `play_card` / `resolve_stack` where agents are available for prompting.
pub fn get_etb_unless_life_cost(card: &crate::card::Card) -> Option<i32> {
    for re in &card.replacement_effects {
        if re.event != ReplacementType::Moved {
            continue;
        }
        if re.ir.destination_zone != Some(ZoneType::Battlefield) {
            continue;
        }
        if let Some(svar_name) = re.replace_with() {
            if svar_name == "ETBTapped" {
                continue;
            }
            if let Some(svar_val) = card.svars.get(svar_name) {
                if svar_val.contains("DB$ Tap") && svar_val.contains("ETB$ True") {
                    // Parse life cost from "UnlessCost$ PayLife<N>"
                    if let Some(pos) = svar_val.find("PayLife<") {
                        let after = &svar_val[pos + 8..]; // skip "PayLife<"
                        if let Some(end) = after.find('>') {
                            if let Ok(n) = after[..end].parse::<i32>() {
                                return Some(n);
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

/// Check if a card has a "enters tapped unless you reveal a <type> from hand" effect.
///
/// Looks for `R:Event$ Moved | Destination$ Battlefield | ReplaceWith$ <SVar>`
/// where the SVar is `DB$ Tap | ETB$ True | UnlessCost$ Reveal<N/Filter>`.
///
/// Returns `Some((n, filter))` if found (e.g. `Some((1, "Merfolk"))` for Wanderwine Hub).
pub fn get_etb_unless_reveal_cost(card: &crate::card::Card) -> Option<(i32, String)> {
    for re in &card.replacement_effects {
        if re.event != ReplacementType::Moved {
            continue;
        }
        if re.ir.destination_zone != Some(ZoneType::Battlefield) {
            continue;
        }
        if let Some(svar_name) = re.replace_with() {
            if svar_name == "ETBTapped" {
                continue;
            }
            if let Some(svar_val) = card.svars.get(svar_name) {
                if svar_val.contains("DB$ Tap") && svar_val.contains("ETB$ True") {
                    // Parse reveal cost from "UnlessCost$ Reveal<N/Filter>"
                    if let Some(pos) = svar_val.find("Reveal<") {
                        let after = &svar_val[pos + 7..]; // skip "Reveal<"
                        if let Some(end) = after.find('>') {
                            let inner = &after[..end]; // "1/Merfolk" or "1/Filter"
                            let mut parts = inner.splitn(2, '/');
                            let n = parts
                                .next()
                                .and_then(|s| s.trim().parse::<i32>().ok())
                                .unwrap_or(1);
                            let filter = parts.next().unwrap_or("").trim().to_string();
                            return Some((n, filter));
                        }
                    }
                }
            }
        }
    }
    None
}

/// Resolve an AddPower$/AddToughness$ parameter that may be a literal integer
/// or an SVar reference (e.g. "X" → Count$Valid Enchantment.YouCtrl).
fn resolve_add_pt_value(game: &GameState, source_id: CardId, val_str: Option<&str>) -> i32 {
    let val_str = match val_str {
        Some(val_str) => val_str,
        None => return 0,
    };

    // Try direct integer parse first
    if let Ok(n) = val_str.trim().parse::<i32>() {
        return n;
    }

    // It's an SVar reference — look it up on the source card
    let source = game.card(source_id);
    if let Some(svar_expr) = source.svars.get(val_str.trim()) {
        if svar_expr.starts_with("Count$") {
            return crate::ability::effects::resolve_count_svar(
                svar_expr,
                game,
                source_id,
                source.controller,
            );
        }
        return crate::ability::effects::evaluate_svar(
            svar_expr,
            &crate::spellability::SpellAbility::new_empty(Some(source_id), source.controller),
        );
    }

    0
}

/// Resolve a SetPower$/SetToughness$ parameter that may be a literal integer or
/// an SVar reference (e.g. "X" → SVar:X:Count$Valid Creature.ChosenType).
/// Mirrors Java `AbilityUtils.calculateAmount(hostCard, param, stAb)`.
fn resolve_set_pt_value(game: &GameState, source_id: CardId, val_str: Option<&str>) -> Option<i32> {
    let val_str = val_str?;
    // Try direct integer parse first
    if let Ok(n) = val_str.trim().parse::<i32>() {
        return Some(n);
    }

    // It's an SVar reference — look it up on the source card
    let source = game.card(source_id);
    if let Some(svar_expr) = source.svars.get(val_str.trim()) {
        if svar_expr.starts_with("Count$") {
            return Some(crate::ability::effects::resolve_count_svar(
                svar_expr,
                game,
                source_id,
                source.controller,
            ));
        }
        // Simple SVar evaluation (e.g. Number$2)
        return Some(crate::ability::effects::evaluate_svar(
            svar_expr,
            &crate::spellability::SpellAbility::new_empty(Some(source_id), source.controller),
        ));
    }

    None
}

fn basic_land_mana_ability_text(subtype: &str) -> Option<&'static str> {
    match subtype {
        "Plains" => Some("AB$ Mana | Cost$ T | Produced$ W | SpellDescription$ Add {W}."),
        "Island" => Some("AB$ Mana | Cost$ T | Produced$ U | SpellDescription$ Add {U}."),
        "Swamp" => Some("AB$ Mana | Cost$ T | Produced$ B | SpellDescription$ Add {B}."),
        "Mountain" => Some("AB$ Mana | Cost$ T | Produced$ R | SpellDescription$ Add {R}."),
        "Forest" => Some("AB$ Mana | Cost$ T | Produced$ G | SpellDescription$ Add {G}."),
        _ => None,
    }
}

fn resolve_added_basic_land_types(
    source: &crate::card::Card,
    add_type: Option<&str>,
) -> Vec<String> {
    resolve_added_types(source, add_type)
        .into_iter()
        .filter(|added| basic_land_mana_ability_text(added).is_some())
        .collect()
}

fn resolve_added_types(source: &crate::card::Card, add_type: Option<&str>) -> Vec<String> {
    let Some(add_type) = add_type else {
        return Vec::new();
    };
    let mut resolved = Vec::new();
    for raw in add_type.split('&').map(str::trim).filter(|s| !s.is_empty()) {
        match raw {
            "ChosenType" => {
                if let Some(chosen) = source.chosen_type.as_ref() {
                    resolved.push(chosen.clone());
                }
            }
            "ChosenType2" => {
                if let Some(chosen) = source.chosen_type2.as_ref() {
                    resolved.push(chosen.clone());
                }
            }
            "AllBasicLandType" => {
                resolved.extend(
                    ["Plains", "Island", "Swamp", "Mountain", "Forest"]
                        .into_iter()
                        .map(str::to_string),
                );
            }
            other => resolved.push(other.to_string()),
        }
    }
    resolved
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use forge_foundation::{CardTypeLine, ColorSet, ManaCost, ZoneType};

    use crate::card::Card;
    use crate::ids::{CardId, PlayerId};

    // Build a minimal two-player game with empty zones.
    fn new_game() -> GameState {
        GameState::new(&["Alice", "Bob"], 20)
    }

    fn add_creature(
        game: &mut GameState,
        owner: PlayerId,
        power: i32,
        toughness: i32,
        keywords: Vec<String>,
        abilities: Vec<String>,
    ) -> CardId {
        let card = Card::new(
            CardId(0), // reassigned by create_card
            "Creature".to_string(),
            owner,
            CardTypeLine::parse("Creature"),
            ManaCost::parse("1 G"),
            ColorSet::GREEN,
            Some(power),
            Some(toughness),
            keywords,
            abilities,
        );
        let id = game.create_card(card);
        game.move_card(id, ZoneType::Battlefield, owner);
        id
    }

    fn add_enchantment(game: &mut GameState, owner: PlayerId, abilities: Vec<String>) -> CardId {
        let card = Card::new(
            CardId(0),
            "Enchantment".to_string(),
            owner,
            CardTypeLine::parse("Enchantment"),
            ManaCost::parse("2 W"),
            ColorSet::WHITE,
            None,
            None,
            vec![],
            abilities,
        );
        let id = game.create_card(card);
        game.move_card(id, ZoneType::Battlefield, owner);
        id
    }

    fn add_land(
        game: &mut GameState,
        owner: PlayerId,
        name: &str,
        type_line: &str,
        abilities: Vec<String>,
    ) -> CardId {
        let card = Card::new(
            CardId(0),
            name.to_string(),
            owner,
            CardTypeLine::parse(type_line),
            ManaCost::no_cost(),
            ColorSet::COLORLESS,
            None,
            None,
            vec![],
            abilities,
        );
        let id = game.create_card(card);
        game.move_card(id, ZoneType::Battlefield, owner);
        id
    }

    // ── Anthem (+1/+1) ────────────────────────────────────────────────────

    #[test]
    fn anthem_boosts_your_creatures() {
        let mut game = new_game();
        let alice = PlayerId(0);
        let bob = PlayerId(1);

        // Add two creatures for Alice and one for Bob.
        let a1 = add_creature(&mut game, alice, 2, 2, vec![], vec![]);
        let a2 = add_creature(&mut game, alice, 1, 1, vec![], vec![]);
        let b1 = add_creature(&mut game, bob, 2, 2, vec![], vec![]);

        // Add Glorious Anthem-style enchantment controlled by Alice.
        let _anthem = add_enchantment(
            &mut game,
            alice,
            vec!["S$ Mode$ Continuous | Affected$ Creature.YouControl | AddPower$ 1 | AddToughness$ 1 | Description$ Creatures you control get +1/+1.".to_string()],
        );

        apply_continuous_effects(&mut game);

        // Alice's creatures get +1/+1.
        assert_eq!(game.card(a1).power(), 3, "Alice's 2/2 should be 3/3");
        assert_eq!(game.card(a1).toughness(), 3);
        assert_eq!(game.card(a2).power(), 2, "Alice's 1/1 should be 2/2");
        assert_eq!(game.card(a2).toughness(), 2);

        // Bob's creature is unaffected.
        assert_eq!(
            game.card(b1).power(),
            2,
            "Bob's creature should be unchanged"
        );
        assert_eq!(game.card(b1).toughness(), 2);
    }

    #[test]
    fn anthem_resets_when_removed() {
        let mut game = new_game();
        let alice = PlayerId(0);

        let creature = add_creature(&mut game, alice, 2, 2, vec![], vec![]);
        let anthem = add_enchantment(
            &mut game,
            alice,
            vec!["S$ Mode$ Continuous | Affected$ Creature.YouControl | AddPower$ 1 | AddToughness$ 1".to_string()],
        );

        apply_continuous_effects(&mut game);
        assert_eq!(game.card(creature).power(), 3);

        // Remove the anthem from the battlefield.
        game.move_card(anthem, ZoneType::Graveyard, alice);
        apply_continuous_effects(&mut game);

        assert_eq!(
            game.card(creature).power(),
            2,
            "Bonus should be gone after anthem leaves"
        );
    }

    #[test]
    fn stacking_anthems() {
        let mut game = new_game();
        let alice = PlayerId(0);

        let creature = add_creature(&mut game, alice, 1, 1, vec![], vec![]);
        // Two separate +1/+1 anthems.
        add_enchantment(
            &mut game,
            alice,
            vec!["S$ Mode$ Continuous | Affected$ Creature.YouControl | AddPower$ 1 | AddToughness$ 1".to_string()],
        );
        add_enchantment(
            &mut game,
            alice,
            vec!["S$ Mode$ Continuous | Affected$ Creature.YouControl | AddPower$ 1 | AddToughness$ 1".to_string()],
        );

        apply_continuous_effects(&mut game);
        assert_eq!(game.card(creature).power(), 3, "Two anthems should give +2");
        assert_eq!(game.card(creature).toughness(), 3);
    }

    // ── Keyword granting ──────────────────────────────────────────────────

    #[test]
    fn grant_flying_to_your_creatures() {
        let mut game = new_game();
        let alice = PlayerId(0);
        let bob = PlayerId(1);

        let a1 = add_creature(&mut game, alice, 2, 2, vec![], vec![]);
        let b1 = add_creature(&mut game, bob, 2, 2, vec![], vec![]);

        add_enchantment(
            &mut game,
            alice,
            vec!["S$ Mode$ Continuous | Affected$ Creature.YouControl | AddKeyword$ Flying | Description$ Creatures you control have flying.".to_string()],
        );

        apply_continuous_effects(&mut game);

        assert!(
            game.card(a1).has_flying(),
            "Alice's creature should have flying"
        );
        assert!(
            !game.card(b1).has_flying(),
            "Bob's creature should not have flying"
        );
    }

    #[test]
    fn grant_multiple_keywords() {
        let mut game = new_game();
        let alice = PlayerId(0);

        let creature = add_creature(&mut game, alice, 2, 2, vec![], vec![]);
        add_enchantment(
            &mut game,
            alice,
            vec!["S$ Mode$ Continuous | Affected$ Creature.YouControl | AddKeyword$ Flying & First Strike".to_string()],
        );

        apply_continuous_effects(&mut game);

        assert!(game.card(creature).has_flying());
        assert!(game.card(creature).has_first_strike());
    }

    // ── SetPT (Layer 7b) ──────────────────────────────────────────────────

    #[test]
    fn set_pt_overrides_base() {
        let mut game = new_game();
        let alice = PlayerId(0);

        let creature = add_creature(&mut game, alice, 5, 5, vec![], vec![]);
        // Effect: set all your creatures to 0/1 (e.g. Humility).
        add_enchantment(
            &mut game,
            alice,
            vec!["S$ Mode$ Continuous | Affected$ Creature.YouControl | SetPower$ 0 | SetToughness$ 1".to_string()],
        );

        apply_continuous_effects(&mut game);
        assert_eq!(game.card(creature).power(), 0);
        assert_eq!(game.card(creature).toughness(), 1);
    }

    #[test]
    fn modify_pt_adds_on_top_of_set_pt() {
        // CR 613.7c: ModifyPT applies after SetPT within the same turn.
        let mut game = new_game();
        let alice = PlayerId(0);

        let creature = add_creature(&mut game, alice, 5, 5, vec![], vec![]);
        // Layer 7b: set to 0/1.
        add_enchantment(
            &mut game,
            alice,
            vec!["S$ Mode$ Continuous | Affected$ Creature.YouControl | SetPower$ 0 | SetToughness$ 1".to_string()],
        );
        // Layer 7c: +1/+1 anthem on top.
        add_enchantment(
            &mut game,
            alice,
            vec!["S$ Mode$ Continuous | Affected$ Creature.YouControl | AddPower$ 1 | AddToughness$ 1".to_string()],
        );

        apply_continuous_effects(&mut game);
        // 0 + 1 = 1 power, 1 + 1 = 2 toughness.
        assert_eq!(game.card(creature).power(), 1);
        assert_eq!(game.card(creature).toughness(), 2);
    }

    // ── CantAttack / CantBlock ────────────────────────────────────────────

    #[test]
    fn cant_attack_flag_set() {
        let mut game = new_game();
        let alice = PlayerId(0);

        let creature = add_creature(&mut game, alice, 2, 2, vec![], vec![]);
        // Pacifism-like effect.
        add_enchantment(
            &mut game,
            alice,
            vec!["S$ Mode$ CantAttack | Affected$ Creature.YouControl | Description$ Creatures you control can't attack.".to_string()],
        );

        apply_continuous_effects(&mut game);
        assert!(game.card(creature).cant_attack_static);
    }

    #[test]
    fn cant_block_flag_set() {
        let mut game = new_game();
        let alice = PlayerId(0);

        let creature = add_creature(&mut game, alice, 2, 2, vec![], vec![]);
        add_enchantment(
            &mut game,
            alice,
            vec!["S$ Mode$ CantBlock | Affected$ Creature.YouControl".to_string()],
        );

        apply_continuous_effects(&mut game);
        assert!(game.card(creature).cant_block_static);
    }

    #[test]
    fn flags_reset_on_reapplication() {
        let mut game = new_game();
        let alice = PlayerId(0);

        let creature = add_creature(&mut game, alice, 2, 2, vec![], vec![]);
        let restrictor = add_enchantment(
            &mut game,
            alice,
            vec!["S$ Mode$ CantAttack | Affected$ Creature.YouControl".to_string()],
        );

        apply_continuous_effects(&mut game);
        assert!(game.card(creature).cant_attack_static);

        game.move_card(restrictor, ZoneType::Graveyard, alice);
        apply_continuous_effects(&mut game);
        assert!(
            !game.card(creature).cant_attack_static,
            "Flag should clear after enchantment leaves"
        );
    }

    #[test]
    fn lands_gain_swamp_mana_ability_from_urborg_style_effect() {
        let mut game = new_game();
        let alice = PlayerId(0);

        let urborg = add_land(
            &mut game,
            alice,
            "Urborg, Tomb of Yawgmoth",
            "Legendary Land",
            vec!["S$ Mode$ Continuous | Affected$ Land | AddType$ Swamp | Description$ Each land is a Swamp in addition to its other land types.".to_string()],
        );
        let black_gate = add_land(
            &mut game,
            alice,
            "The Black Gate",
            "Legendary Land Gate",
            vec![],
        );

        apply_continuous_effects(&mut game);

        for land_id in [urborg, black_gate] {
            let land = game.card(land_id);
            assert!(
                land.type_line.has_subtype("Swamp"),
                "{} should gain the Swamp subtype",
                land.card_name
            );
            assert!(
                land.activated_abilities
                    .iter()
                    .any(|ab| ab.is_mana_ability && ab.produced.as_deref() == Some("B")),
                "{} should gain an intrinsic black mana ability from Swamp",
                land.card_name
            );
        }
    }

    // ── ETB Tapped ────────────────────────────────────────────────────────

    #[test]
    fn self_etb_tapped() {
        let mut game = new_game();
        let alice = PlayerId(0);

        // A permanent with ETBTapped on itself.
        let card = Card::new(
            CardId(0),
            "TappedLand".to_string(),
            alice,
            CardTypeLine::parse("Land"),
            ManaCost::parse(""),
            ColorSet::from_mask(0),
            None,
            None,
            vec![],
            vec!["S$ Mode$ ETBTapped | Description$ Enters tapped.".to_string()],
        );
        let id = game.create_card(card);
        game.move_card(id, ZoneType::Battlefield, alice);
        apply_etb_tapped(&mut game, id);

        assert!(
            game.card(id).tapped,
            "Card with ETBTapped should enter tapped"
        );
    }

    #[test]
    fn no_etb_tapped_without_ability() {
        let mut game = new_game();
        let alice = PlayerId(0);

        let id = add_creature(&mut game, alice, 2, 2, vec![], vec![]);
        // Fresh ETB, no static — should not be tapped.
        assert!(
            !game.card(id).tapped,
            "Normal creature should not enter tapped"
        );
    }

    #[test]
    fn etb_tapped_via_replacement_effect() {
        let mut game = new_game();
        let alice = PlayerId(0);

        // A land with R:Event$ Moved replacement effect (like Path of Ancestry).
        let card = Card::new(
            CardId(0),
            "PathOfAncestry".to_string(),
            alice,
            CardTypeLine::parse("Land"),
            ManaCost::parse(""),
            ColorSet::from_mask(0),
            None,
            None,
            vec![],
            vec!["R:Event$ Moved | Destination$ Battlefield | ValidCard$ Card.Self | ReplaceWith$ ETBTapped | Description$ ~ enters tapped.".to_string()],
        );
        let id = game.create_card(card);
        game.move_card(id, ZoneType::Battlefield, alice);
        apply_etb_tapped(&mut game, id);

        assert!(
            game.card(id).tapped,
            "Card with ReplaceWith$ ETBTapped replacement should enter tapped"
        );
    }
}
