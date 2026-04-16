//! Keyword-based ability and trigger generation for Card.
//!
//! These functions translate keywords like "Cycling", "Prowess", "Bushido", etc. into
//! concrete activated abilities and triggered abilities. They're called during card
//! initialization in `Card::from_rules()`.

use crate::ability::activated::parse_activated_ability;
use crate::parsing::keys;
use crate::staticability::parse_static_ability;
use crate::trigger::parse_trigger;

use super::Card;

impl Card {
    /// Generate intrinsic mana abilities for basic land subtypes (Plains → {W}, etc.).
    /// Mirrors Java's `CardFactoryUtil.addIntrinsicAbilities()`.
    pub(super) fn generate_basic_land_mana_abilities(&mut self) {
        const SUBTYPE_MANA: &[(&str, &str, &str)] = &[
            ("Plains", "W", "Add {W}."),
            ("Island", "U", "Add {U}."),
            ("Swamp", "B", "Add {B}."),
            ("Mountain", "R", "Add {R}."),
            ("Forest", "G", "Add {G}."),
        ];
        for &(subtype, letter, desc) in SUBTYPE_MANA {
            if self.type_line.has_subtype(subtype) {
                let already_produces = self
                    .activated_abilities
                    .iter()
                    .any(|ab| ab.is_mana_ability && ab.params.get(keys::PRODUCED) == Some(letter));
                if !already_produces {
                    let raw = format!(
                        "AB$ Mana | Cost$ T | Produced$ {} | SpellDescription$ {}",
                        letter, desc
                    );
                    let idx = self.abilities.len();
                    self.abilities.push(raw.clone());
                    if let Some(ab) = parse_activated_ability(&raw, idx) {
                        self.activated_abilities.push(ab);
                    }
                }
            }
        }
    }

    /// Generate activated abilities from keywords (e.g. Cycling → AB$ Draw).
    /// Mirrors Java's `CardFactoryUtil.setupKeywordedAbilities()`.
    pub(super) fn generate_keyword_abilities(&mut self) {
        // Cycling: K:Cycling:{cost} → AB$ Draw | Cost$ {cost} Discard<1/CARDNAME> | ActivationZone$ Hand
        if let Some(cycling_cost) = self.get_keyword_cost("Cycling") {
            let ab_text = format!(
                "AB$ Draw | Cost$ {} Discard<1/CARDNAME> | ActivationZone$ Hand | NumCards$ 1 | Defined$ You",
                cycling_cost
            );
            let next_idx = self.activated_abilities.len();
            if let Some(ab) = parse_activated_ability(&ab_text, next_idx) {
                self.activated_abilities.push(ab);
            }
        }

        // TypeCycling: K:TypeCycling:{type}:{cost} → AB$ ChangeZone | Cost$ {cost} Discard<1/CARDNAME> | ActivationZone$ Hand
        // Mirrors Java CardFactoryUtil lines 3852-3864.
        for kw in self
            .keywords
            .iter_strings()
            .chain(self.granted_keywords.iter_strings())
        {
            if let Some(rest) = kw.strip_prefix("TypeCycling:") {
                let parts: Vec<&str> = rest.splitn(2, ':').collect();
                if parts.len() == 2 {
                    let cycle_type = parts[0].trim(); // e.g., "Swamp"
                    let mana_cost = parts[1].trim(); // e.g., "1"
                                                     // getTitleWithoutCost() = capitalize(descType) + "cycling"
                    let precost_desc = format!(
                        "{}cycling",
                        cycle_type
                            .chars()
                            .next()
                            .map(|c| c.to_uppercase().to_string())
                            .unwrap_or_default()
                            + &cycle_type[1..]
                    );
                    let ab_text = format!(
                        "AB$ ChangeZone | Cost$ {} Discard<1/CARDNAME> | ActivationZone$ Hand | PrecostDesc$ {} | Origin$ Library | Destination$ Hand | ChangeType$ {}",
                        mana_cost, precost_desc, cycle_type
                    );
                    let next_idx = self.activated_abilities.len();
                    if let Some(ab) = parse_activated_ability(&ab_text, next_idx) {
                        self.activated_abilities.push(ab);
                    }
                }
            }
        }

        // Equip: K:Equip:{cost}[...]
        // Forge keyword payload can include optional suffix data; we only need
        // the activation cost + default target filter to mirror Java baseline.
        if let Some(equip_raw) = self.get_keyword_cost("Equip") {
            let payload = equip_raw
                .split(":::")
                .next()
                .unwrap_or(equip_raw.as_str())
                .trim();
            let mut parts = payload.split(':');
            let equip_cost = parts.next().unwrap_or(payload).trim();
            let target_filter = parts
                .next()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or("Creature.YouCtrl");
            if !equip_cost.is_empty() {
                let ab_text = format!(
                    "AB$ Attach | Cost$ {} | ValidTgts$ {} | SorcerySpeed$ True | SpellDescription$ Equip {}",
                    equip_cost, target_filter, equip_cost
                );
                let next_idx = self.activated_abilities.len();
                if let Some(ab) = parse_activated_ability(&ab_text, next_idx) {
                    self.activated_abilities.push(ab);
                }
            }
        }

        // Adapt: K:Adapt:N:cost → AB$ PutCounter with Adapt$ True gate.
        // Mirrors Java CardFactoryUtil lines 2665-2684.
        for kw in self
            .keywords
            .iter_strings()
            .chain(self.granted_keywords.iter_strings())
        {
            if let Some(rest) = crate::keyword::extract_keyword_cost_str(&kw, "Adapt") {
                let parts: Vec<&str> = rest.splitn(2, ':').collect();
                if parts.len() == 2 {
                    let magnitude = parts[0].trim();
                    let mana_cost = parts[1].trim();
                    let ab_text = format!(
                        "AB$ PutCounter | Cost$ {} | Adapt$ True | CounterNum$ {} | CounterType$ P1P1 | StackDescription$ SpellDescription | SpellDescription$ Adapt {}",
                        mana_cost, magnitude, magnitude
                    );
                    let next_idx = self.activated_abilities.len();
                    if let Some(ab) = parse_activated_ability(&ab_text, next_idx) {
                        self.activated_abilities.push(ab);
                    }
                }
            }
        }

        // Crew: K:Crew:N → AB$ Animate (tap creatures with total power ≥N).
        // Mirrors Java CardFactoryUtil lines 3820-3835.
        // Uses tapXType<Any/Creature.Other+withTotalPowerGE{N}> matching Java's format.
        for kw in self
            .keywords
            .iter_strings()
            .chain(self.granted_keywords.iter_strings())
        {
            if let Some(n_str) = crate::keyword::extract_keyword_cost_str(&kw, "Crew") {
                let n = n_str.trim();
                let ab_text = format!(
                    "AB$ Animate | Cost$ tapXType<Any/Creature.Other+withTotalPowerGE{{{}}}> | Defined$ Self | Types$ Artifact,Creature | Secondary$ True | SpellDescription$ Crew {}",
                    n, n
                );
                let next_idx = self.activated_abilities.len();
                if let Some(ab) = parse_activated_ability(&ab_text, next_idx) {
                    self.activated_abilities.push(ab);
                }
            }
        }

        // Station: K:Station:N → AB$ PutCounter (tap another creature to add charge counters).
        // Mirrors Java CardFactoryUtil lines 3587-3595.
        // The ability is sorcery-speed and puts charge counters equal to the tapped
        // creature's power onto this Spacecraft/Planet.
        for kw in self
            .keywords
            .iter_strings()
            .chain(self.granted_keywords.iter_strings())
        {
            if let Some(_n_str) = crate::keyword::extract_keyword_cost_str(&kw, "Station") {
                let ab_text = "AB$ PutCounter | Cost$ tapXType<1/Creature.Other> | Defined$ Self | CounterType$ CHARGE | CounterNum$ StationX | SorcerySpeed$ True | CostDesc$ | SpellDescription$ Station";
                let next_idx = self.activated_abilities.len();
                if let Some(ab) = parse_activated_ability(ab_text, next_idx) {
                    self.activated_abilities.push(ab);
                }
                self.svars
                    .entry("StationX".to_string())
                    .or_insert_with(|| "TappedCards$TapPowerValue".to_string());
            }
        }

        // Embalm: K:Embalm:cost → AB$ CopyPermanent from graveyard.
        // Mirrors Java CardFactoryUtil lines 2879-2891.
        for kw in self
            .keywords
            .iter_strings()
            .chain(self.granted_keywords.iter_strings())
        {
            if let Some(cost_str) = crate::keyword::extract_keyword_cost_str(&kw, "Embalm") {
                let cost = cost_str.trim();
                let ab_text = format!(
                    "AB$ CopyPermanent | Cost$ {} ExileFromGrave<1/CARDNAME> | ActivationZone$ Graveyard | SorcerySpeed$ True | Defined$ Self | SetColor$ White | AddTypes$ Zombie | SpellDescription$ Embalm",
                    cost
                );
                let next_idx = self.activated_abilities.len();
                if let Some(ab) = parse_activated_ability(&ab_text, next_idx) {
                    self.activated_abilities.push(ab);
                }
            }
        }

        // Eternalize: K:Eternalize:cost → AB$ CopyPermanent from graveyard as 4/4.
        // Mirrors Java CardFactoryUtil lines 3023-3052.
        for kw in self
            .keywords
            .iter_strings()
            .chain(self.granted_keywords.iter_strings())
        {
            if let Some(cost_str) = crate::keyword::extract_keyword_cost_str(&kw, "Eternalize") {
                let cost = cost_str.trim();
                let ab_text = format!(
                    "AB$ CopyPermanent | Cost$ {} ExileFromGrave<1/CARDNAME> | ActivationZone$ Graveyard | SorcerySpeed$ True | Defined$ Self | SetColor$ Black | SetPower$ 4 | SetToughness$ 4 | AddTypes$ Zombie | SpellDescription$ Eternalize",
                    cost
                );
                let next_idx = self.activated_abilities.len();
                if let Some(ab) = parse_activated_ability(&ab_text, next_idx) {
                    self.activated_abilities.push(ab);
                }
            }
        }

        // Enlist: K:Enlist -> intrinsic optional attack cost static ability.
        // Java builds: Mode$ OptionalAttackCost | Cost$ Enlist<1/CARDNAME/creature> ...
        // Rust cost parser normalizes this and the combat loop applies the enlist payment.
        if self
            .keywords
            .iter_strings()
            .chain(self.granted_keywords.iter_strings())
            .any(|k| k.eq_ignore_ascii_case("Enlist"))
        {
            let raw = "S:Mode$ OptionalAttackCost | ValidCard$ Card.Self | Cost$ Enlist<1/CARDNAME/creature> | Secondary$ True";
            if let Some(sa) = parse_static_ability(raw) {
                self.add_static_ability(sa);
            }
        }

        // Morph / Megamorph: mark card as castable face-down for {3}.
        // The actual casting logic is in game_action_util (playable check + cost handling).
        if self
            .keywords
            .iter_strings()
            .chain(self.granted_keywords.iter_strings())
            .any(|k| k.starts_with("Morph:") || k.starts_with("Megamorph:"))
        {
            self.has_morph = true;
        }

        // Plot: K:Plot:{cost} → AB$ Plot | Cost$ {cost} | ActivationZone$ Hand | SorcerySpeed$ True
        // Mirrors Java CardFactoryUtil lines 3398-3449.
        // Exiles the card from hand; plotted cards can later be cast for free.
        if let Some(plot_cost) = self.get_keyword_cost("Plot") {
            let ab_text = format!(
                "AB$ Plot | Cost$ {} | ActivationZone$ Hand | SorcerySpeed$ True | Secondary$ True | SpellDescription$ Plot",
                plot_cost
            );
            let next_idx = self.activated_abilities.len();
            if let Some(ab) = parse_activated_ability(&ab_text, next_idx) {
                self.activated_abilities.push(ab);
            }
        }
    }

    /// Generate triggered abilities from keywords (e.g. Prowess, Bushido, Annihilator, etc.).
    /// Mirrors Java's `CardFactoryUtil.setupKeywordedTriggers()`.
    pub fn generate_keyword_triggers(&mut self) {
        let mut next_id = self.triggers.len() as u32;

        for kw in self.keywords.as_string_list() {
            // Prowess: +1/+1 when you cast a noncreature spell
            if kw == "Prowess" {
                let raw = "Mode$ SpellCast | ValidCard$ Card.nonCreature | ValidActivatingPlayer$ You | Execute$ TrigProwess | TriggerZones$ Battlefield | TriggerDescription$ Prowess";
                if let Some(mut trig) = parse_trigger(raw, &mut next_id) {
                    trig.execute = "TrigProwess".to_string();
                    self.add_trigger(trig);
                }
                self.svars
                    .entry("TrigProwess".to_string())
                    .or_insert_with(|| {
                        "DB$ Pump | Defined$ Self | NumAtt$ 1 | NumDef$ 1".to_string()
                    });
            }

            // Bushido N: +N/+N when blocking or becoming blocked
            if let Some(n_str) = crate::keyword::extract_keyword_cost_str(&kw, "Bushido") {
                if n_str.parse::<i32>().is_ok() {
                    let raw1 = format!("Mode$ Blocks | ValidCard$ Card.Self | Execute$ TrigBushido | TriggerZones$ Battlefield | TriggerDescription$ Bushido {n_str}");
                    if let Some(mut trig) = parse_trigger(&raw1, &mut next_id) {
                        trig.execute = "TrigBushido".to_string();
                        self.add_trigger(trig);
                    }
                    let raw2 = format!("Mode$ AttackerBlocked | ValidCard$ Card.Self | Execute$ TrigBushido | TriggerZones$ Battlefield | TriggerDescription$ Bushido {n_str}");
                    if let Some(mut trig) = parse_trigger(&raw2, &mut next_id) {
                        trig.execute = "TrigBushido".to_string();
                        self.add_trigger(trig);
                    }
                    self.svars
                        .entry("TrigBushido".to_string())
                        .or_insert_with(|| {
                            format!("DB$ Pump | Defined$ Self | NumAtt$ {n_str} | NumDef$ {n_str}")
                        });
                }
            }

            // Annihilator N: when this creature attacks, defending player sacrifices N permanents.
            // Mirrors Java CardFactoryUtil lines 723-736.
            if let Some(n_str) = crate::keyword::extract_keyword_cost_str(&kw, "Annihilator") {
                if n_str.parse::<i32>().is_ok() {
                    let raw = format!(
                        "Mode$ Attacks | ValidCard$ Card.Self | Execute$ TrigAnnihilator | TriggerZones$ Battlefield | TriggerDescription$ Annihilator {n_str}"
                    );
                    if let Some(mut trig) = parse_trigger(&raw, &mut next_id) {
                        trig.execute = "TrigAnnihilator".to_string();
                        self.add_trigger(trig);
                    }
                    self.svars
                        .entry("TrigAnnihilator".to_string())
                        .or_insert_with(|| {
                            format!("DB$ Sacrifice | Defined$ TriggeredDefendingPlayer | SacValid$ Permanent | Amount$ {n_str}")
                        });
                }
            }

            // Afflict N: when this creature becomes blocked, defending player loses N life.
            // Mirrors Java CardFactoryUtil lines 695-708.
            if let Some(n_str) = crate::keyword::extract_keyword_cost_str(&kw, "Afflict") {
                if n_str.parse::<i32>().is_ok() {
                    let raw = format!(
                        "Mode$ AttackerBlocked | ValidCard$ Card.Self | TriggerZones$ Battlefield | Secondary$ True | Execute$ TrigAfflict | TriggerDescription$ Afflict {n_str}"
                    );
                    if let Some(mut trig) = parse_trigger(&raw, &mut next_id) {
                        trig.execute = "TrigAfflict".to_string();
                        self.add_trigger(trig);
                    }
                    self.svars
                        .entry("TrigAfflict".to_string())
                        .or_insert_with(|| {
                            format!("DB$ LoseLife | Defined$ TriggeredDefendingPlayer | LifeAmount$ {n_str}")
                        });
                }
            }

            // Undying: when this creature dies, if it had no +1/+1 counters, return it
            // to the battlefield with a +1/+1 counter.
            // Mirrors Java CardFactoryUtil lines 1965-1974.
            if kw == "Undying" {
                let raw = "Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.Self+counters_EQ0_P1P1 | TriggerZones$ Battlefield | Execute$ TrigUndying | TriggerDescription$ Undying";
                if let Some(mut trig) = parse_trigger(raw, &mut next_id) {
                    trig.execute = "TrigUndying".to_string();
                    self.add_trigger(trig);
                }
                self.svars
                    .entry("TrigUndying".to_string())
                    .or_insert_with(|| {
                        "DB$ ChangeZone | Defined$ TriggeredNewCardLKICopy | Origin$ Graveyard | Destination$ Battlefield | WithCountersType$ P1P1".to_string()
                    });
            }

            // Persist: when this creature dies, if it had no -1/-1 counters, return it
            // to the battlefield with a -1/-1 counter.
            // Mirrors Java CardFactoryUtil lines 1663-1672.
            if kw == "Persist" {
                let raw = "Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.Self+counters_EQ0_M1M1 | TriggerZones$ Battlefield | Execute$ TrigPersist | TriggerDescription$ Persist";
                if let Some(mut trig) = parse_trigger(raw, &mut next_id) {
                    trig.execute = "TrigPersist".to_string();
                    self.add_trigger(trig);
                }
                self.svars
                    .entry("TrigPersist".to_string())
                    .or_insert_with(|| {
                        "DB$ ChangeZone | Defined$ TriggeredNewCardLKICopy | Origin$ Graveyard | Destination$ Battlefield | WithCountersType$ M1M1".to_string()
                    });
            }

            // Afterlife N: when this creature dies, create N 1/1 white and black Spirit
            // creature tokens with flying.
            // Mirrors Java CardFactoryUtil lines 709-722.
            if let Some(n_str) = crate::keyword::extract_keyword_cost_str(&kw, "Afterlife") {
                if n_str.parse::<i32>().is_ok() {
                    let raw = format!(
                        "Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.Self | TriggerZones$ Battlefield | Execute$ TrigAfterlife | TriggerDescription$ Afterlife {n_str}"
                    );
                    if let Some(mut trig) = parse_trigger(&raw, &mut next_id) {
                        trig.execute = "TrigAfterlife".to_string();
                        self.add_trigger(trig);
                    }
                    self.svars
                        .entry("TrigAfterlife".to_string())
                        .or_insert_with(|| {
                            format!("DB$ Token | TokenAmount$ {n_str} | TokenScript$ wb_1_1_spirit_flying")
                        });
                }
            }

            // Exploit: when this creature enters the battlefield, you may sacrifice a creature.
            // Mirrors Java CardFactoryUtil lines 1104-1113.
            if kw == "Exploit" {
                let raw = "Mode$ ChangesZone | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigExploit | TriggerDescription$ Exploit";
                if let Some(mut trig) = parse_trigger(raw, &mut next_id) {
                    trig.execute = "TrigExploit".to_string();
                    self.add_trigger(trig);
                }
                self.svars
                    .entry("TrigExploit".to_string())
                    .or_insert_with(|| {
                        "DB$ Sacrifice | SacValid$ Creature | Optional$ True | Exploit$ True"
                            .to_string()
                    });
            }

            // Fabricate N: when this creature enters the battlefield, choose either
            // N +1/+1 counters on it or create N 1/1 Servo tokens.
            // Mirrors Java CardFactoryUtil lines 1132-1151.
            // Java uses DB$ Token with UnlessCost$ AddCounter<N/P1P1> | UnlessPayer$ You:
            // default is tokens, unless the controller "pays" by putting counters instead.
            if let Some(n_str) = crate::keyword::extract_keyword_cost_str(&kw, "Fabricate") {
                if n_str.parse::<i32>().is_ok() {
                    let raw = format!(
                        "Mode$ ChangesZone | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigFabricate | Secondary$ True | TriggerDescription$ Fabricate {n_str}"
                    );
                    if let Some(mut trig) = parse_trigger(&raw, &mut next_id) {
                        trig.execute = "TrigFabricate".to_string();
                        self.add_trigger(trig);
                    }
                    self.svars
                        .entry("TrigFabricate".to_string())
                        .or_insert_with(|| {
                            format!(
                                "DB$ Token | TokenAmount$ {n_str} | TokenScript$ c_1_1_a_servo \
                                 | UnlessCost$ AddCounter<{n_str}/P1P1> | UnlessPayer$ You \
                                 | SpellDescription$ Fabricate {n_str}"
                            )
                        });
                }
            }

            // Modular N: enters with N +1/+1 counters; when it dies, move its +1/+1
            // counters to target artifact creature.
            // Mirrors Java CardFactoryUtil lines 1579-1596 & 2425-2436.
            if let Some(n_str) = crate::keyword::extract_keyword_cost_str(&kw, "Modular") {
                if let Ok(n) = n_str.parse::<i32>() {
                    // ETB counters: set on the card instance so they're added as a
                    // replacement effect when entering the battlefield (not a trigger,
                    // because 0/0 creatures would die to SBA before triggers resolve).
                    self.etb_counters_p1p1 += n;

                    // Death trigger: move counters to target artifact creature
                    let raw = format!(
                        "Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.Self | TriggerZones$ Battlefield | Execute$ TrigModular | TriggerDescription$ Modular {n_str}"
                    );
                    if let Some(mut trig) = parse_trigger(&raw, &mut next_id) {
                        trig.execute = "TrigModular".to_string();
                        trig.optional = true;
                        self.add_trigger(trig);
                    }
                    // Put +1/+1 counters on target artifact creature.
                    // Uses SP$ Charm with a single mode so the charm system handles
                    // target selection via choose_target_card.
                    // CounterNum$ uses the static Modular N value as default.
                    // At resolution time, if trigger_remembered_amount > 0
                    // (set by LKI counter capture in the death path), that
                    // value overrides the static N — mirroring Java's
                    // `TriggeredCard$CardCounters.P1P1` (CR 702.43b).
                    self.svars
                        .entry("TrigModular".to_string())
                        .or_insert_with(|| "SP$ Charm | Choices$ ModularMove".to_string());
                    self.svars
                        .entry("ModularMove".to_string())
                        .or_insert_with(|| {
                            format!("DB$ PutCounter | Defined$ Targeted | CounterType$ P1P1 | CounterNum$ {n_str} | Modular$ true | ValidTgts$ Creature.Artifact | SpellDescription$ Put +1/+1 counter(s) on target artifact creature")
                        });
                }
            }

            // Ward:{cost} — when this permanent becomes the target of a spell or ability
            // an opponent controls, counter that spell/ability unless its controller pays {cost}.
            // Mirrors Java CardFactoryUtil lines 2055-2069.
            // The opponent is prompted via confirm_action to pay the Ward cost;
            // if they decline, the spell is countered.
            if let Some(cost_str) = crate::keyword::extract_keyword_cost_str(&kw, "Ward") {
                let raw = "Mode$ BecomesTarget | ValidCard$ Card.Self | Execute$ TrigWard | TriggerZones$ Battlefield | TriggerDescription$ Ward";
                if let Some(mut trig) = parse_trigger(raw, &mut next_id) {
                    trig.execute = "TrigWard".to_string();
                    self.add_trigger(trig);
                }
                self.svars
                    .entry("TrigWard".to_string())
                    .or_insert_with(|| format!("DB$ Counter | UnlessCost$ {cost_str}"));
            }

            // Exalted — whenever a creature you control attacks alone, it gets +1/+1 until EOT.
            // Mirrors Java CardFactoryUtil lines 1094-1103.
            if kw == "Exalted" {
                let raw = "Mode$ Attacks | ValidCard$ Creature.YouCtrl | Alone$ True | Execute$ TrigExalted | TriggerZones$ Battlefield | TriggerDescription$ Exalted";
                if let Some(mut trig) = parse_trigger(raw, &mut next_id) {
                    trig.execute = "TrigExalted".to_string();
                    self.add_trigger(trig);
                }
                self.svars
                    .entry("TrigExalted".to_string())
                    .or_insert_with(|| {
                        "DB$ Pump | Defined$ TriggeredAttacker | NumAtt$ +1 | NumDef$ +1"
                            .to_string()
                    });
            }

            // Renown N — when this creature deals combat damage to a player, if it's not
            // renowned, put N +1/+1 counters on it and it becomes renowned.
            // Mirrors Java CardFactoryUtil lines 1744-1756.
            if let Some(n_str) = crate::keyword::extract_keyword_cost_str(&kw, "Renown") {
                if n_str.parse::<i32>().is_ok() {
                    let raw = format!(
                        "Mode$ DamageDone | ValidSource$ Card.Self | ValidTarget$ Player | CombatDamage$ True | Execute$ TrigRenown | TriggerZones$ Battlefield | TriggerDescription$ Renown {n_str}"
                    );
                    if let Some(mut trig) = parse_trigger(&raw, &mut next_id) {
                        trig.execute = "TrigRenown".to_string();
                        self.add_trigger(trig);
                    }
                    self.svars
                        .entry("TrigRenown".to_string())
                        .or_insert_with(|| {
                            format!("DB$ PutCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ {n_str} | Renown$ True")
                        });
                }
            }

            // Flanking — when this creature becomes blocked by a creature without flanking,
            // the blocking creature gets -1/-1 until end of turn.
            // Mirrors Java CardFactoryUtil lines 1194-1205.
            if kw == "Flanking" {
                let raw = "Mode$ AttackerBlockedByCreature | ValidBlocked$ Card.Self | ValidCard$ Creature.withoutFlanking | Execute$ TrigFlanking | TriggerZones$ Battlefield | TriggerDescription$ Flanking";
                if let Some(mut trig) = parse_trigger(raw, &mut next_id) {
                    trig.execute = "TrigFlanking".to_string();
                    self.add_trigger(trig);
                }
                self.svars
                    .entry("TrigFlanking".to_string())
                    .or_insert_with(|| {
                        "DB$ Pump | Defined$ TriggeredBlocker | NumAtt$ -1 | NumDef$ -1".to_string()
                    });
            }

            // Extort — whenever you cast a spell, you may drain 1 life from each opponent.
            // Mirrors Java CardFactoryUtil lines 1114-1131.
            if kw == "Extort" {
                let raw = "Mode$ SpellCast | ValidActivatingPlayer$ You | Execute$ TrigExtort | TriggerZones$ Battlefield | TriggerDescription$ Extort";
                if let Some(mut trig) = parse_trigger(raw, &mut next_id) {
                    trig.execute = "TrigExtort".to_string();
                    trig.optional = true;
                    self.add_trigger(trig);
                }
                self.svars
                    .entry("TrigExtort".to_string())
                    .or_insert_with(|| {
                        "DB$ LoseLife | Defined$ Player.Opponent | LifeAmount$ 1 | SubAbility$ ExtortGain".to_string()
                    });
                self.svars
                    .entry("ExtortGain".to_string())
                    .or_insert_with(|| "DB$ GainLife | Defined$ You | LifeAmount$ 1".to_string());
            }

            // Bloodthirst N — if an opponent was dealt damage this turn, this creature
            // enters the battlefield with N additional +1/+1 counters.
            // Mirrors Java CardFactoryUtil lines 2164-2182.
            if let Some(n_str) = crate::keyword::extract_keyword_cost_str(&kw, "Bloodthirst") {
                if n_str.parse::<i32>().is_ok() {
                    let raw = format!(
                        "Mode$ ChangesZone | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigBloodthirst | TriggerDescription$ Bloodthirst {n_str}"
                    );
                    if let Some(mut trig) = parse_trigger(&raw, &mut next_id) {
                        trig.execute = "TrigBloodthirst".to_string();
                        self.add_trigger(trig);
                    }
                    self.svars
                        .entry("TrigBloodthirst".to_string())
                        .or_insert_with(|| {
                            format!("DB$ PutCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ {n_str} | Bloodthirst$ True")
                        });
                }
            }

            // Riot — when this creature enters the battlefield, choose: +1/+1 counter or haste.
            // Mirrors Java CardFactoryUtil lines 2518-2524.
            if kw == "Riot" {
                let raw = "Mode$ ChangesZone | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigRiot | TriggerDescription$ Riot";
                if let Some(mut trig) = parse_trigger(raw, &mut next_id) {
                    trig.execute = "TrigRiot".to_string();
                    self.add_trigger(trig);
                }
                self.svars
                    .entry("TrigRiot".to_string())
                    .or_insert_with(|| "SP$ Charm | Choices$ RiotCounter,RiotHaste".to_string());
                self.svars
                    .entry("RiotCounter".to_string())
                    .or_insert_with(|| {
                        "DB$ PutCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ 1 | SpellDescription$ Put a +1/+1 counter on this creature".to_string()
                    });
                self.svars
                    .entry("RiotHaste".to_string())
                    .or_insert_with(|| {
                        "DB$ Pump | Defined$ Self | KW$ Haste | SpellDescription$ This creature gains haste".to_string()
                    });
            }

            // Unleash — this creature enters the battlefield with a +1/+1 counter on it.
            // It can't block as long as it has a +1/+1 counter on it.
            // Mirrors Java CardFactoryUtil lines 2571-2576.
            if kw == "Unleash" {
                let raw = "Mode$ ChangesZone | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigUnleash | TriggerDescription$ Unleash";
                if let Some(mut trig) = parse_trigger(raw, &mut next_id) {
                    trig.execute = "TrigUnleash".to_string();
                    self.add_trigger(trig);
                }
                self.svars
                    .entry("TrigUnleash".to_string())
                    .or_insert_with(|| {
                        "DB$ PutCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ 1"
                            .to_string()
                    });
            }

            // Cumulative upkeep — at the beginning of your upkeep, put an age counter
            // on this permanent, then sacrifice it unless you pay its upkeep cost for
            // each age counter on it.
            // Mirrors Java CardFactoryUtil lines 960-976: generates a Phase trigger
            // with a Sacrifice effect that has CumulativeUpkeep$ param.
            if let Some(rest) = kw.strip_prefix("Cumulative upkeep:") {
                let cost_spec = rest.split(':').next().unwrap_or(rest);
                let raw = "Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | TriggerDescription$ Cumulative upkeep";
                if let Some(mut trig) = parse_trigger(raw, &mut next_id) {
                    trig.execute = "TrigCumulativeUpkeep".to_string();
                    self.add_trigger(trig);
                }
                self.svars
                    .entry("TrigCumulativeUpkeep".to_string())
                    .or_insert_with(|| {
                        format!("DB$ Sacrifice | SacValid$ Self | CumulativeUpkeep$ {cost_spec}")
                    });
            }

            // Madness: K:Madness:{cost} → trigger when this card is exiled
            // (via the discard replacement). Mirrors Java's Madness trigger created in
            // CardFactoryUtil.java:1474-1508.
            //
            // Flow: player discards → replacement asks "exile instead?" (optional) →
            // if yes, card goes to exile → ChangesZone trigger fires →
            // Play effect with Optional$ True (player chooses to cast or not) →
            // if not played, card moves to graveyard.
            //
            // NOTE: No OptionalDecider$ on the trigger — the first optionality is
            // in the replacement effect (choose_single_replacement_effect), and
            // the second is in the Play effect's Optional$ True.
            if let Some(madness_cost) = crate::keyword::extract_keyword_cost_str(&kw, "Madness") {
                let raw = "Mode$ ChangesZone | Origin$ Hand | Destination$ Exile | ValidCard$ Card.Self | Secondary$ True | TriggerZones$ Exile | TriggerDescription$ You may cast this card for its madness cost.";
                if let Some(mut trig) = parse_trigger(raw, &mut next_id) {
                    trig.execute = "TrigMadnessPlay".to_string();
                    self.add_trigger(trig);
                }
                self.svars
                    .entry("TrigMadnessPlay".to_string())
                    .or_insert_with(|| {
                        format!(
                            "DB$ Play | Defined$ Self | ValidSA$ Spell | PlayCost$ {} | Optional$ True | RememberPlayed$ True | SubAbility$ MadnessMoveToYard",
                            madness_cost
                        )
                    });
                self.svars
                    .entry("MadnessMoveToYard".to_string())
                    .or_insert_with(|| {
                        "DB$ ChangeZone | Defined$ Self | Origin$ Exile | Destination$ Graveyard | TrackDiscarded$ True | ConditionDefined$ Remembered | ConditionPresent$ Card | ConditionCompare$ EQ0 | SubAbility$ MadnessCleanup".to_string()
                    });
                self.svars
                    .entry("MadnessCleanup".to_string())
                    .or_insert_with(|| "DB$ Cleanup | ClearRemembered$ True".to_string());
            }
        }
    }
}
