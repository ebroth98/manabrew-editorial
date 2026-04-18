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

use crate::game::GameState;
use crate::ids::{CardId, PlayerId};
use crate::parsing::keys;
use crate::replacement::replacement_effect::ReplacementType;
use crate::staticability::{CardFilter, Layer, StaticAbility, StaticMode};

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

    // ── 2. Collect (source_id, static_ability) for every battlefield permanent ──
    // We clone the data we need so the borrow checker lets us mutate later.
    let sources: Vec<(CardId, StaticAbility)> = game
        .cards
        .iter()
        .filter(|c| c.zone == ZoneType::Battlefield)
        .flat_map(|c| c.static_abilities.iter().map(move |sa| (c.id, sa.clone())))
        .collect();

    // ── 3. Build list of effects-to-apply (deferred to allow sorting) ────
    let mut pending: Vec<PendingEffect> = Vec::new();

    for (source_id, sa) in &sources {
        let source_card = &game.cards[source_id.index()];

        // Full static-ability condition gate (IsPresent$, CheckSVar$, Condition$, etc.).
        // Mirrors Java static ability checks before applying continuous effects.
        if !sa.check_conditions(source_card, game) {
            continue;
        }

        // CharacteristicDefining statics always affect only the host card.
        // Mirrors Java StaticAbilityContinuous.getAffectedCards() line 1036.
        let is_cda = sa
            .params
            .get(keys::CHARACTERISTIC_DEFINING)
            .map(|v| v.eq_ignore_ascii_case("True"))
            .unwrap_or(false);

        // Determine which cards are affected by this static ability.
        let affected_str = sa
            .params
            .get(keys::AFFECTED)
            .or_else(|| sa.params.get(keys::VALID_CARDS))
            .or_else(|| sa.params.get(keys::VALID_CARD))
            .unwrap_or("Creature.YouControl");
        let targets: Vec<CardId> = if is_cda {
            // CDAs always affect only the source card itself.
            if source_card.zone == ZoneType::Battlefield {
                vec![*source_id]
            } else {
                vec![]
            }
        } else if affected_str.eq_ignore_ascii_case("Card.Self")
            || affected_str.starts_with("Card.Self+")
        {
            // Self-referencing static: only affects the source card itself,
            // but qualifiers after "+" must still be checked (e.g.
            // "Card.Self+counters_GE2_CHARGE" only matches when the card
            // has ≥2 charge counters).  Mirrors Java's
            // StaticAbilityContinuous.getAffectedCards() which validates
            // all qualifiers even for self-referencing statics.
            if source_card.zone == ZoneType::Battlefield
                && crate::card::valid_filter::matches_valid_card(
                    affected_str,
                    source_card,
                    source_card,
                )
            {
                vec![*source_id]
            } else {
                vec![]
            }
        } else if affected_str.eq_ignore_ascii_case("Card.EnchantedBy")
            || affected_str.contains(".EquippedBy")
            || affected_str.contains(".EnchantedBy")
        {
            // Aura / Equipment static effects: affect what this source is
            // attached to.  Java treats EquippedBy and EnchantedBy
            // identically — both resolve to the entity the source is
            // attached to.  (e.g. Short Sword: "Creature.EquippedBy",
            // Control Magic: "Card.EnchantedBy")
            source_card
                .attached_to
                .filter(|&cid| game.card(cid).zone == ZoneType::Battlefield)
                .into_iter()
                .collect()
        } else {
            let filter = CardFilter::parse(affected_str);
            // AffectedZone$ overrides the default Battlefield filter (e.g.
            // Ashling, the Limitless grants Evoke:4 to Elementals in Hand).
            let affected_zones: Vec<ZoneType> = sa
                .params
                .get(keys::AFFECTED_ZONE)
                .map(|s| {
                    s.split(',')
                        .filter_map(|z| ZoneType::from_str_compat(z.trim()))
                        .collect()
                })
                .unwrap_or_else(|| vec![ZoneType::Battlefield]);
            game.cards
                .iter()
                .filter(|c| {
                    affected_zones.contains(&c.zone)
                        && filter.matches_with_game(c, source_card, game)
                })
                .map(|c| c.id)
                .collect()
        };

        match sa.mode {
            // ── Continuous: queue effect for later sorted application ────
            StaticMode::Continuous => {
                for target in targets {
                    // Java parity: one continuous static can contribute effects in
                    // multiple layers (e.g. Brothers Yamazaki adds both +2/+2 and Haste).
                    if sa.params.has(keys::GAIN_CONTROL) {
                        let Some(gain_control) = sa.params.get(keys::GAIN_CONTROL) else {
                            continue;
                        };
                        let new_controller = match gain_control {
                            "You" | "YouCtrl" => source_card.controller,
                            "Opponent" => game.opponent_of(source_card.controller),
                            _ => continue,
                        };
                        pending.push(PendingEffect {
                            layer: Layer::Control,
                            target,
                            kind: EffectKind::SetController {
                                controller: new_controller,
                            },
                        });
                    }

                    if sa.params.has(keys::ADD_POWER) || sa.params.has(keys::ADD_TOUGHNESS)
                    {
                        let p = resolve_add_pt_param(game, sa, *source_id, keys::ADD_POWER);
                        let t = resolve_add_pt_param(game, sa, *source_id, keys::ADD_TOUGHNESS);
                        pending.push(PendingEffect {
                            layer: Layer::ModifyPT,
                            target,
                            kind: EffectKind::AddPT {
                                power: p,
                                toughness: t,
                            },
                        });
                    }

                    let source = game.card(*source_id);
                    for added_type in resolve_added_types(source, sa) {
                        pending.push(PendingEffect {
                            layer: Layer::Type,
                            target,
                            kind: EffectKind::AddType(added_type),
                        });
                    }

                    if sa.params.has(keys::SET_POWER) || sa.params.has(keys::SET_TOUGHNESS)
                    {
                        let sp = resolve_set_pt_param(game, &sa, *source_id, keys::SET_POWER);
                        let st = resolve_set_pt_param(game, &sa, *source_id, keys::SET_TOUGHNESS);
                        pending.push(PendingEffect {
                            layer: Layer::SetPT,
                            target,
                            kind: EffectKind::SetPT {
                                power: sp,
                                toughness: st,
                            },
                        });
                    }

                    if sa.params.has(keys::ADD_KEYWORD) {
                        // AddKeyword$ supports multiple keywords separated by " & ".
                        let kws = sa.params.get_cloned(keys::ADD_KEYWORD).unwrap_or_default();
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
                    if let Some(raw) = sa.params.get(keys::ADD_TYPE) {
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
                    if let Some(svar_name) = sa.params.get(keys::ADD_ABILITY) {
                        let source = game.card(*source_id);
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

                    if let Some(add_trigger) = sa.params.get(keys::ADD_TRIGGER) {
                        let source = game.card(*source_id);
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

                    let source = game.card(*source_id);
                    for subtype in resolve_added_basic_land_types(source, sa) {
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
            }

            // ── Restriction statics: apply immediately (not layer-ordered) ──
            StaticMode::CantAttack => {
                for target in targets {
                    game.cards[target.index()].cant_attack_static = true;
                }
            }
            StaticMode::CantBlock => {
                for target in targets {
                    game.cards[target.index()].cant_block_static = true;
                }
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
        }
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
                    game.cards[effect.target.index()].triggers.push(trig);
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
                        .params
                        .get_cloned(keys::VALID_CARDS)
                        .or_else(|| sa.params.get_cloned(keys::AFFECTED))
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
    let repl_sources: Vec<(CardId, String)> = game
        .cards
        .iter()
        .filter(|c| c.zone == ZoneType::Battlefield)
        .flat_map(|c| {
            c.replacement_effects.iter().filter_map(move |re| {
                if re.event == ReplacementType::Moved
                    && re.params.get(keys::REPLACE_WITH) == Some("ETBTapped")
                    && re.params.get(keys::DESTINATION) == Some("Battlefield")
                    && re.active_in_zone(ZoneType::Battlefield)
                {
                    let filter = re
                        .params
                        .get("ValidCard")
                        .unwrap_or("Card.Self")
                        .to_string();
                    Some((c.id, filter))
                } else {
                    None
                }
            })
        })
        .collect();

    for (source_id, filter_str) in repl_sources {
        let tapped = if filter_str == "Card.Self" || filter_str.is_empty() {
            source_id == entering_card
        } else {
            let source = &game.cards[source_id.index()];
            let filter = CardFilter::parse(&filter_str);
            filter.matches_with_game(&game.cards[entering_card.index()], source, game)
        };

        if tapped {
            game.cards[entering_card.index()].tapped = true;
            return;
        }
    }
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
        if re.params.get(keys::DESTINATION) != Some("Battlefield") {
            continue;
        }
        if let Some(svar_name) = re.params.get(keys::REPLACE_WITH) {
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
        if re.params.get(keys::DESTINATION) != Some("Battlefield") {
            continue;
        }
        if let Some(svar_name) = re.params.get(keys::REPLACE_WITH) {
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

/// Resolve a SetPower/SetToughness parameter that may be an integer literal or
/// Resolve an AddPower$/AddToughness$ parameter that may be a literal integer
/// or an SVar reference (e.g. "X" → Count$Valid Enchantment.YouCtrl).
fn resolve_add_pt_param(
    game: &GameState,
    sa: &StaticAbility,
    source_id: CardId,
    param_name: &str,
) -> i32 {
    let val_str = match sa.params.get(param_name) {
        Some(s) => s,
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
fn resolve_set_pt_param(
    game: &GameState,
    sa: &StaticAbility,
    source_id: CardId,
    param_name: &str,
) -> Option<i32> {
    let val_str = sa.params.get(param_name)?;

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

fn resolve_added_basic_land_types(source: &crate::card::Card, sa: &StaticAbility) -> Vec<String> {
    resolve_added_types(source, sa)
        .into_iter()
        .filter(|added| basic_land_mana_ability_text(added).is_some())
        .collect()
}

fn resolve_added_types(source: &crate::card::Card, sa: &StaticAbility) -> Vec<String> {
    let Some(add_type) = sa.params.get(keys::ADD_TYPE) else {
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
                    .any(|ab| { ab.is_mana_ability && ab.params.get(keys::PRODUCED) == Some("B") }),
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
