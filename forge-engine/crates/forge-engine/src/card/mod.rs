pub mod card_property;
pub mod damage_history;

use std::collections::{BTreeMap, HashMap};

use forge_carddb::CardRules;
use forge_foundation::{CardTypeLine, ColorSet, ManaCost, ZoneType};
use serde::{Deserialize, Serialize};

use crate::ability::activated::{parse_activated_ability, ActivatedAbility};
use crate::ids::{CardId, PlayerId};
use crate::replacement::{parse_replacement_effect, ReplacementEffect};
use crate::staticability::{parse_static_ability, StaticAbility};
use crate::trigger::{parse_trigger, Trigger};

/// Parse `Mode$ AlternativeCost | Cost$ GainLife<N/...> | IsPresent$ ...` from a
/// static ability raw string and return `Some("AltCostGainLife:N:condition")` keyword.
fn parse_gainlife_alt_cost_keyword(raw: &str) -> Option<String> {
    if !raw.contains("AlternativeCost") {
        return None;
    }
    let life_amount = raw.split('|').find_map(|part| {
        let p = part.trim();
        if let Some(rest) = p.strip_prefix("Cost$") {
            let cost = rest.trim();
            if let Some(inner) = cost
                .strip_prefix("GainLife<")
                .and_then(|s| s.split('>').next())
            {
                let n = inner
                    .split('/')
                    .next()
                    .and_then(|s| s.trim().parse::<i32>().ok())?;
                return Some(n);
            }
        }
        None
    })?;
    let condition = raw
        .split('|')
        .find_map(|part| {
            let p = part.trim();
            p.strip_prefix("IsPresent$").map(|s| s.trim().to_string())
        })
        .unwrap_or_default();
    Some(format!("AltCostGainLife:{}:{}", life_amount, condition))
}

/// Stores alternate-face characteristics for double-faced cards (DFCs).
/// The `transform()` method swaps `CardInstance` fields with these values.
/// Mirrors Java's `CardState` stored as the "backside" state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardOtherPart {
    pub name: String,
    pub type_line: CardTypeLine,
    pub mana_cost: ManaCost,
    pub color: ColorSet,
    pub base_power: Option<i32>,
    pub base_toughness: Option<i32>,
    pub keywords: Vec<String>,
    pub abilities: Vec<String>,
    pub triggers: Vec<Trigger>,
    pub svars: BTreeMap<String, String>,
}

/// Saved pre-animate state for AnimateEffect, restored at cleanup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimateState {
    pub original_type_line: CardTypeLine,
    pub original_base_power: Option<i32>,
    pub original_base_toughness: Option<i32>,
    pub original_color: ColorSet,
}

/// A card instance in a game. This is the mutable game-state representation,
/// as opposed to CardRules which is the immutable definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardInstance {
    pub id: CardId,
    /// Index into the CardDatabase (or name) identifying the card definition.
    pub card_name: String,

    // Ownership and control
    pub owner: PlayerId,
    pub controller: PlayerId,

    // Current zone
    pub zone: ZoneType,

    // Type line (can be modified by effects)
    pub type_line: CardTypeLine,

    // Mana cost (can be modified)
    pub mana_cost: ManaCost,

    // Color (can be modified)
    pub color: ColorSet,

    // Power/Toughness (base values, can be modified)
    pub base_power: Option<i32>,
    pub base_toughness: Option<i32>,
    /// Temporary P/T modifications from spells/abilities resolving this turn
    /// (e.g. Giant Growth).  Reset when leaving the battlefield.
    pub power_modifier: i32,
    pub toughness_modifier: i32,
    /// Layer 7b override: set by `SetPower$` / `SetToughness$` continuous effects.
    /// `None` means use `base_power` / `base_toughness` as normal.
    /// Reset to `None` each time [`layer::apply_continuous_effects`] runs.
    pub static_set_power: Option<i32>,
    pub static_set_toughness: Option<i32>,
    /// Layer 7c bonus: accumulated from `AddPower$` / `AddToughness$` anthems.
    /// Reset to 0 each time [`layer::apply_continuous_effects`] runs.
    pub static_power_modifier: i32,
    pub static_toughness_modifier: i32,

    // Combat/state
    pub tapped: bool,
    pub flipped: bool,
    pub face_down: bool,
    /// True if this card has Morph or Megamorph and can be cast face-down for {3}.
    pub has_morph: bool,
    /// True if this card is currently bestowed (attached as an Aura via Bestow).
    pub is_bestowed: bool,
    pub summoning_sick: bool,
    pub exerted: bool,
    pub damage: i32,

    // Counters
    pub counters: HashMap<CounterType, i32>,

    // Keywords intrinsic to this card (from its card definition).
    pub keywords: Vec<String>,
    /// Keywords granted by continuous static effects (Layer 6).
    /// Reset and recomputed each time [`layer::apply_continuous_effects`] runs.
    pub granted_keywords: Vec<String>,
    /// Keywords granted temporarily by pump effects (`KW$` parameter) until end of turn.
    /// Cleared during step_cleanup alongside power_modifier / toughness_modifier.
    pub pump_keywords: Vec<String>,

    // Abilities (raw strings from card definition)
    pub abilities: Vec<String>,

    // Parsed activated abilities (from AB$ lines in abilities)
    pub activated_abilities: Vec<ActivatedAbility>,

    /// Parsed static abilities (from S$ lines in abilities).
    /// Mirrors Java Forge `Card.getStaticAbilities()`.
    pub static_abilities: Vec<StaticAbility>,

    // Combat tracking
    pub has_deathtouch_damage: bool,
    /// Set by `Mode$ CantAttack` static effects. Reset each time
    /// [`layer::apply_continuous_effects`] runs.
    pub cant_attack_static: bool,
    /// Set by `Mode$ CantBlock` static effects. Reset each time
    /// [`layer::apply_continuous_effects`] runs.
    pub cant_block_static: bool,

    // Turn tracking
    pub entered_battlefield_this_turn: bool,
    pub attacked_this_turn: bool,

    // Triggers — mirrors Java Card.getTriggers()
    pub triggers: Vec<Trigger>,
    // SVars — mirrors Java Card.getSVars()
    pub svars: BTreeMap<String, String>,

    // Commander tracking
    /// True if this card is designated as a commander.
    pub is_commander: bool,
    /// How many times this commander has been cast from the command zone (for tax).
    pub commander_cast_count: u32,

    /// True if this permanent is a token or a copy-token (ceases to exist on zone change).
    pub is_token: bool,

    // Replacement effects — parsed from R$ lines in card abilities.
    // Mirrors Java `Card.getReplacementEffects()`.
    pub replacement_effects: Vec<ReplacementEffect>,

    // Attachment tracking (Auras / Equipment).
    // Mirrors Java `Card.getAttachedTo()` / `Card.getAttachedCards()`.
    /// The permanent this card is currently attached to (for Auras/Equipment).
    pub attached_to: Option<CardId>,
    /// Cards currently attached to this permanent (inverse of `attached_to`).
    pub attachments: Vec<CardId>,

    // Memory for "Remember" parameters
    /// Cards remembered by this card (for RememberCountered, etc.)
    pub remembered_cards: Vec<CardId>,
    /// Players remembered by this card (for Player.IsRemembered checks).
    pub remembered_players: Vec<PlayerId>,
    /// CMC values remembered by this card
    pub remembered_cmc: Vec<i32>,
    /// Source card that created this effect card (for Card.EffectSource checks).
    pub effect_source: Option<CardId>,
    /// True if this temporary effect expires at end of turn cleanup.
    pub temp_effect_until_eot: bool,
    /// Host card this temporary effect is linked to; when host leaves the
    /// battlefield, this effect expires.
    pub temp_effect_host: Option<CardId>,
    /// Forget remembered cards when they move from this origin zone.
    pub forget_on_moved_origin: Option<ZoneType>,
    /// Exile this effect when remembered cards become empty after forget logic.
    pub exile_when_no_remembered: bool,

    /// Original controller to restore at end of turn (for `LoseControl$ EOT`).
    pub original_controller_eot: Option<PlayerId>,

    // Double-faced card (DFC) state
    /// True if this card is currently showing its back face.
    pub is_transformed: bool,
    /// Back-face characteristics for DFC cards. `None` for single-faced cards.
    pub other_part: Option<CardOtherPart>,

    /// Optional set code (e.g., "M21") for specific printings.
    pub set_code: Option<String>,

    // Phase-out state (issue #22, Phases effect).
    pub phased_out: bool,

    // Regeneration shields (issue #22, Regenerate effect).
    // Decremented instead of destroying; resets at end of turn.
    pub regeneration_shields: i32,

    /// Whether this permanent was kicked when cast.
    /// Mirrors Java `Card.isKicked()`. Stored on the card so triggers
    /// with `ValidCard$ Card.Self+kicked` can check it after resolution.
    pub kicked: bool,
    /// Whether this permanent has become monstrous.
    /// Mirrors Java `Card.isMonstrous()`. Resets when the permanent changes zones.
    pub monstrous: bool,

    /// Colors chosen by ChooseColorEffect (stored for later reference by other effects).
    pub chosen_colors: Vec<String>,
    /// Cards chosen by ChooseCardEffect (stored for later reference by other effects).
    pub chosen_cards: Vec<CardId>,

    /// Saved state for AnimateEffect — restored during step_cleanup.
    pub animate_state: Option<AnimateState>,

    // ── Issue #53: High-priority effect fields ──────────────────────────
    /// Type chosen by ChooseType effect (e.g. "Goblin", "Artifact").
    pub chosen_type: Option<String>,
    /// Card names chosen by NameCard effect.
    pub named_cards: Vec<String>,
    /// Number chosen by ChooseNumber effect.
    pub chosen_number: Option<i32>,
    /// Player chosen by ChoosePlayer effect.
    pub chosen_player: Option<PlayerId>,
    /// Controller who made the chosen-player choice.
    pub chosen_player_controller: Option<PlayerId>,
    /// Controller who made the chosen-type choice.
    pub chosen_type_controller: Option<PlayerId>,
    /// Whether the chosen player has been revealed.
    pub chosen_player_revealed: bool,
    /// Whether the chosen type has been revealed.
    pub chosen_type_revealed: bool,
    /// Opponent chosen for PromiseGift cost.
    pub promised_gift: Option<PlayerId>,
    /// True if detained — can't attack, block, or activate abilities. Clears at controller's next turn.
    pub detained: bool,
    /// Set during combat to the player this creature is attacking; None if not attacking.
    pub attacking_player: Option<PlayerId>,
    /// Player who goaded this creature. Goaded creature must attack but can't attack goader.
    pub goaded_by: Option<PlayerId>,
    /// Damage prevention shields (decremented when damage would be dealt). Resets at EOT.
    pub damage_prevention: i32,
    /// True if this creature must block if able.
    pub must_block: bool,
    /// Spell cards encoded/ciphered onto this creature.
    pub encoded_cards: Vec<CardId>,
    /// Cards that dealt damage to this creature this turn (for DamagedBy trigger filters).
    /// Mirrors Java `CardDamageHistory.getDamageReceivedThisTurn()`.
    pub damage_sources_this_turn: Vec<CardId>,
    /// Damage history tracking (attacks, blocks, damage dealt).
    /// Mirrors Java `CardDamageHistory`.
    #[serde(skip)]
    pub damage_history: damage_history::DamageHistory,
    /// Specific cards this creature must block (set by effects like Lure variants).
    pub must_block_cards: Vec<CardId>,
    /// +1/+1 counters to add on ETB (from mana that adds counters, e.g. Guildmages' Forum).
    pub etb_counters_p1p1: i32,
    /// Bitmask of colors of mana spent to cast this spell (for Sunburst/Converge).
    /// Uses ManaAtom bit flags (W=1, U=2, B=4, R=8, G=16).
    pub colors_spent_to_cast: u16,
    /// Pre-selected charm/mode indices (for Spree — modes chosen before payment).
    /// If `Some`, charm_effect should use these instead of asking the player again.
    pub chosen_modes: Option<Vec<usize>>,
    /// Number of extra targets paid for via Strive (0 = no extra targets).
    pub strive_extra_targets: u32,
    /// Set when this creature enlisted another creature in the current combat.
    pub enlisted_this_combat: bool,
    /// Per-ability activation count this game (for PowerUp once-per-game restriction).
    pub activations_this_game: std::collections::BTreeMap<usize, u32>,
    /// True once Renown has triggered (creature dealt combat damage to a player).
    /// Mirrors Java `Card.isRenowned()`.
    pub is_renowned: bool,
}

impl CardInstance {
    pub fn new(
        id: CardId,
        card_name: String,
        owner: PlayerId,
        type_line: CardTypeLine,
        mana_cost: ManaCost,
        color: ColorSet,
        base_power: Option<i32>,
        base_toughness: Option<i32>,
        keywords: Vec<String>,
        abilities: Vec<String>,
    ) -> Self {
        // Parse activated abilities from raw ability strings.
        let activated_abilities: Vec<ActivatedAbility> = abilities
            .iter()
            .enumerate()
            .filter_map(|(i, raw)| parse_activated_ability(raw, i))
            .collect();

        // Parse replacement effects from R$ lines in card abilities.
        // Mirrors Java Card constructor calling ReplacementHandler registration.
        let replacement_effects: Vec<ReplacementEffect> = abilities
            .iter()
            .filter_map(|raw| parse_replacement_effect(raw))
            .collect();

        // Parse static abilities from S$ lines.
        // Mirrors Java Forge Card constructor calling StaticAbility.create().
        let static_abilities: Vec<StaticAbility> = abilities
            .iter()
            .filter_map(|raw| parse_static_ability(raw))
            .collect();

        let mut card = CardInstance {
            id,
            card_name,
            owner,
            controller: owner,
            zone: ZoneType::None,
            type_line,
            mana_cost,
            color,
            base_power,
            base_toughness,
            power_modifier: 0,
            toughness_modifier: 0,
            static_set_power: None,
            static_set_toughness: None,
            static_power_modifier: 0,
            static_toughness_modifier: 0,
            tapped: false,
            flipped: false,
            face_down: false,
            has_morph: false,
            is_bestowed: false,
            summoning_sick: true,
            exerted: false,
            damage: 0,
            counters: HashMap::new(),
            keywords,
            granted_keywords: Vec::new(),
            pump_keywords: Vec::new(),
            abilities,
            activated_abilities,
            static_abilities,
            has_deathtouch_damage: false,
            cant_attack_static: false,
            cant_block_static: false,
            entered_battlefield_this_turn: false,
            attacked_this_turn: false,
            triggers: Vec::new(),
            svars: BTreeMap::new(),
            is_commander: false,
            commander_cast_count: 0,
            is_token: false,
            replacement_effects,
            attached_to: None,
            attachments: Vec::new(),
            remembered_cards: Vec::new(),
            remembered_players: Vec::new(),
            remembered_cmc: Vec::new(),
            effect_source: None,
            temp_effect_until_eot: false,
            temp_effect_host: None,
            forget_on_moved_origin: None,
            exile_when_no_remembered: false,
            original_controller_eot: None,
            is_transformed: false,
            other_part: None,
            set_code: None,
            phased_out: false,
            regeneration_shields: 0,
            kicked: false,
            monstrous: false,
            chosen_colors: Vec::new(),
            chosen_cards: Vec::new(),
            animate_state: None,
            chosen_type: None,
            named_cards: Vec::new(),
            chosen_number: None,
            chosen_player: None,
            chosen_player_controller: None,
            chosen_type_controller: None,
            chosen_player_revealed: false,
            chosen_type_revealed: false,
            promised_gift: None,
            detained: false,
            attacking_player: None,
            goaded_by: None,
            damage_prevention: 0,
            must_block: false,
            encoded_cards: Vec::new(),
            damage_sources_this_turn: Vec::new(),
            damage_history: damage_history::DamageHistory::default(),
            must_block_cards: Vec::new(),
            etb_counters_p1p1: 0,
            colors_spent_to_cast: 0,
            chosen_modes: None,
            strive_extra_targets: 0,
            enlisted_this_combat: false,
            activations_this_game: std::collections::BTreeMap::new(),
            is_renowned: false,
        };

        // Generate intrinsic abilities from card properties (mirrors Java CardFactoryUtil)
        card.generate_basic_land_mana_abilities();
        card.generate_keyword_abilities();
        card.generate_keyword_triggers();
        card
    }

    /// Generate intrinsic mana abilities for basic land subtypes (Plains → {W}, etc.).
    /// Mirrors Java's `CardFactoryUtil.addIntrinsicAbilities()`.
    fn generate_basic_land_mana_abilities(&mut self) {
        const SUBTYPE_MANA: &[(&str, &str, &str)] = &[
            ("Plains", "W", "Add {W}."),
            ("Island", "U", "Add {U}."),
            ("Swamp", "B", "Add {B}."),
            ("Mountain", "R", "Add {R}."),
            ("Forest", "G", "Add {G}."),
        ];
        for &(subtype, letter, desc) in SUBTYPE_MANA {
            if self.type_line.has_subtype(subtype) {
                let already_produces = self.activated_abilities.iter().any(|ab| {
                    ab.is_mana_ability && ab.params.get("Produced").map_or(false, |p| p == letter)
                });
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
    fn generate_keyword_abilities(&mut self) {
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
        for kw in self.keywords.iter().chain(self.granted_keywords.iter()) {
            if let Some(rest) = kw.strip_prefix("Adapt:") {
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
        for kw in self.keywords.iter().chain(self.granted_keywords.iter()) {
            if let Some(n_str) = kw.strip_prefix("Crew:") {
                let n = n_str.trim();
                let ab_text = format!(
                    "AB$ Animate | Cost$ tapXType<{}/Creature.Other> | Defined$ Self | Types$ Artifact,Creature | Secondary$ True | SpellDescription$ Crew {}",
                    n, n
                );
                let next_idx = self.activated_abilities.len();
                if let Some(ab) = parse_activated_ability(&ab_text, next_idx) {
                    self.activated_abilities.push(ab);
                }
            }
        }

        // Embalm: K:Embalm:cost → AB$ CopyPermanent from graveyard.
        // Mirrors Java CardFactoryUtil lines 2879-2891.
        for kw in self.keywords.iter().chain(self.granted_keywords.iter()) {
            if let Some(cost_str) = kw.strip_prefix("Embalm:") {
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
        for kw in self.keywords.iter().chain(self.granted_keywords.iter()) {
            if let Some(cost_str) = kw.strip_prefix("Eternalize:") {
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
            .iter()
            .chain(self.granted_keywords.iter())
            .any(|k| k.eq_ignore_ascii_case("Enlist"))
        {
            let raw = "S:Mode$ OptionalAttackCost | ValidCard$ Card.Self | Cost$ Enlist<1/CARDNAME/creature> | Secondary$ True";
            if let Some(sa) = parse_static_ability(raw) {
                self.static_abilities.push(sa);
            }
        }
    }

    /// Generate triggered abilities from keywords (e.g. Prowess, Bushido, Annihilator, etc.).
    /// Mirrors Java's `CardFactoryUtil.setupKeywordedTriggers()`.
    pub fn generate_keyword_triggers(&mut self) {
        let mut next_id = self.triggers.len() as u32;

        for kw in self.keywords.clone() {
            // Prowess: +1/+1 when you cast a noncreature spell
            if kw == "Prowess" {
                let raw = "Mode$ SpellCast | ValidCard$ Card.nonCreature | ValidActivatingPlayer$ You | Execute$ TrigProwess | TriggerZones$ Battlefield | TriggerDescription$ Prowess";
                if let Some(mut trig) = parse_trigger(raw, &mut next_id) {
                    trig.execute = "TrigProwess".to_string();
                    self.triggers.push(trig);
                }
                self.svars
                    .entry("TrigProwess".to_string())
                    .or_insert_with(|| {
                        "DB$ Pump | Defined$ Self | NumAtt$ 1 | NumDef$ 1".to_string()
                    });
            }

            // Bushido N: +N/+N when blocking or becoming blocked
            if let Some(n_str) = kw.strip_prefix("Bushido:") {
                if n_str.parse::<i32>().is_ok() {
                    let raw1 = format!("Mode$ Blocks | ValidCard$ Card.Self | Execute$ TrigBushido | TriggerZones$ Battlefield | TriggerDescription$ Bushido {n_str}");
                    if let Some(mut trig) = parse_trigger(&raw1, &mut next_id) {
                        trig.execute = "TrigBushido".to_string();
                        self.triggers.push(trig);
                    }
                    let raw2 = format!("Mode$ AttackerBlocked | ValidCard$ Card.Self | Execute$ TrigBushido | TriggerZones$ Battlefield | TriggerDescription$ Bushido {n_str}");
                    if let Some(mut trig) = parse_trigger(&raw2, &mut next_id) {
                        trig.execute = "TrigBushido".to_string();
                        self.triggers.push(trig);
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
            if let Some(n_str) = kw.strip_prefix("Annihilator:") {
                if n_str.parse::<i32>().is_ok() {
                    let raw = format!(
                        "Mode$ Attacks | ValidCard$ Card.Self | Execute$ TrigAnnihilator | TriggerZones$ Battlefield | TriggerDescription$ Annihilator {n_str}"
                    );
                    if let Some(mut trig) = parse_trigger(&raw, &mut next_id) {
                        trig.execute = "TrigAnnihilator".to_string();
                        self.triggers.push(trig);
                    }
                    self.svars
                        .entry("TrigAnnihilator".to_string())
                        .or_insert_with(|| {
                            format!("DB$ Sacrifice | Defined$ TriggeredDefendingPlayer | SacValid$ Permanent | Amount$ {n_str}")
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
                    self.triggers.push(trig);
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
                    self.triggers.push(trig);
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
            if let Some(n_str) = kw.strip_prefix("Afterlife:") {
                if n_str.parse::<i32>().is_ok() {
                    let raw = format!(
                        "Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.Self | TriggerZones$ Battlefield | Execute$ TrigAfterlife | TriggerDescription$ Afterlife {n_str}"
                    );
                    if let Some(mut trig) = parse_trigger(&raw, &mut next_id) {
                        trig.execute = "TrigAfterlife".to_string();
                        self.triggers.push(trig);
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
                    self.triggers.push(trig);
                }
                self.svars
                    .entry("TrigExploit".to_string())
                    .or_insert_with(|| {
                        "DB$ Sacrifice | SacValid$ Creature | Optional$ True | Exploit$ True".to_string()
                    });
            }

            // Fabricate N: when this creature enters the battlefield, choose either
            // N +1/+1 counters on it or create N 1/1 Servo tokens.
            // Mirrors Java CardFactoryUtil lines 1132-1151.
            // Java uses DB$ Token with UnlessCost$ AddCounter<N/P1P1> | UnlessPayer$ You:
            // default is tokens, unless the controller "pays" by putting counters instead.
            if let Some(n_str) = kw.strip_prefix("Fabricate:") {
                if n_str.parse::<i32>().is_ok() {
                    let raw = format!(
                        "Mode$ ChangesZone | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigFabricate | Secondary$ True | TriggerDescription$ Fabricate {n_str}"
                    );
                    if let Some(mut trig) = parse_trigger(&raw, &mut next_id) {
                        trig.execute = "TrigFabricate".to_string();
                        self.triggers.push(trig);
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
            // Mirrors Java CardFactoryUtil lines 1579-1596.
            if let Some(n_str) = kw.strip_prefix("Modular:") {
                if n_str.parse::<i32>().is_ok() {
                    // Death trigger: move counters to target artifact creature
                    let raw = format!(
                        "Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.Self | TriggerZones$ Battlefield | Execute$ TrigModular | TriggerDescription$ Modular {n_str}"
                    );
                    if let Some(mut trig) = parse_trigger(&raw, &mut next_id) {
                        trig.execute = "TrigModular".to_string();
                        trig.optional = true;
                        self.triggers.push(trig);
                    }
                    // Put N +1/+1 counters on target artifact creature.
                    // Uses SP$ Charm with a single mode so the charm system handles
                    // target selection via choose_target_card.
                    self.svars
                        .entry("TrigModular".to_string())
                        .or_insert_with(|| {
                            "SP$ Charm | Choices$ ModularMove".to_string()
                        });
                    self.svars
                        .entry("ModularMove".to_string())
                        .or_insert_with(|| {
                            format!("DB$ PutCounter | Defined$ Targeted | CounterType$ P1P1 | CounterNum$ {n_str} | ValidTgts$ Creature.Artifact | SpellDescription$ Put {n_str} +1/+1 counter(s) on target artifact creature")
                        });
                }
            }

            // Ward:{cost} — when this permanent becomes the target of a spell or ability
            // an opponent controls, counter that spell/ability unless its controller pays {cost}.
            // Mirrors Java CardFactoryUtil lines 2055-2069.
            // The opponent is prompted via confirm_action to pay the Ward cost;
            // if they decline, the spell is countered.
            if let Some(cost_str) = kw.strip_prefix("Ward:") {
                let raw = "Mode$ BecomesTarget | ValidCard$ Card.Self | Execute$ TrigWard | TriggerZones$ Battlefield | TriggerDescription$ Ward";
                if let Some(mut trig) = parse_trigger(raw, &mut next_id) {
                    trig.execute = "TrigWard".to_string();
                    self.triggers.push(trig);
                }
                self.svars
                    .entry("TrigWard".to_string())
                    .or_insert_with(|| {
                        format!("DB$ Counter | UnlessCost$ {cost_str}")
                    });
            }

            // Exalted — whenever a creature you control attacks alone, it gets +1/+1 until EOT.
            // Mirrors Java CardFactoryUtil lines 1094-1103.
            if kw == "Exalted" {
                let raw = "Mode$ Attacks | ValidCard$ Creature.YouCtrl | Alone$ True | Execute$ TrigExalted | TriggerZones$ Battlefield | TriggerDescription$ Exalted";
                if let Some(mut trig) = parse_trigger(raw, &mut next_id) {
                    trig.execute = "TrigExalted".to_string();
                    self.triggers.push(trig);
                }
                self.svars
                    .entry("TrigExalted".to_string())
                    .or_insert_with(|| {
                        "DB$ Pump | Defined$ TriggeredAttacker | NumAtt$ +1 | NumDef$ +1".to_string()
                    });
            }

            // Renown N — when this creature deals combat damage to a player, if it's not
            // renowned, put N +1/+1 counters on it and it becomes renowned.
            // Mirrors Java CardFactoryUtil lines 1744-1756.
            if let Some(n_str) = kw.strip_prefix("Renown:") {
                if n_str.parse::<i32>().is_ok() {
                    let raw = format!(
                        "Mode$ DamageDone | ValidSource$ Card.Self | ValidTarget$ Player | CombatDamage$ True | Execute$ TrigRenown | TriggerZones$ Battlefield | TriggerDescription$ Renown {n_str}"
                    );
                    if let Some(mut trig) = parse_trigger(&raw, &mut next_id) {
                        trig.execute = "TrigRenown".to_string();
                        self.triggers.push(trig);
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
                    self.triggers.push(trig);
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
                    self.triggers.push(trig);
                }
                self.svars
                    .entry("TrigExtort".to_string())
                    .or_insert_with(|| {
                        "DB$ LoseLife | Defined$ Player.Opponent | LifeAmount$ 1 | SubAbility$ ExtortGain".to_string()
                    });
                self.svars
                    .entry("ExtortGain".to_string())
                    .or_insert_with(|| {
                        "DB$ GainLife | Defined$ You | LifeAmount$ 1".to_string()
                    });
            }

            // Bloodthirst N — if an opponent was dealt damage this turn, this creature
            // enters the battlefield with N additional +1/+1 counters.
            // Mirrors Java CardFactoryUtil lines 2164-2182.
            if let Some(n_str) = kw.strip_prefix("Bloodthirst:") {
                if n_str.parse::<i32>().is_ok() {
                    let raw = format!(
                        "Mode$ ChangesZone | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigBloodthirst | TriggerDescription$ Bloodthirst {n_str}"
                    );
                    if let Some(mut trig) = parse_trigger(&raw, &mut next_id) {
                        trig.execute = "TrigBloodthirst".to_string();
                        self.triggers.push(trig);
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
                    self.triggers.push(trig);
                }
                self.svars
                    .entry("TrigRiot".to_string())
                    .or_insert_with(|| {
                        "SP$ Charm | Choices$ RiotCounter,RiotHaste".to_string()
                    });
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
                    self.triggers.push(trig);
                }
                self.svars
                    .entry("TrigUnleash".to_string())
                    .or_insert_with(|| {
                        "DB$ PutCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ 1".to_string()
                    });
            }
        }
    }

    /// Construct a `CardInstance` from a `CardRules` definition.
    /// This is the single entry point for creating game-ready cards from the
    /// card database. Mirrors Java's `CardFactory.readCard()` + `CardFactoryUtil`.
    ///
    /// Handles:
    /// - Base stats (name, mana cost, type line, color, P/T, keywords, abilities)
    /// - Trigger parsing (T: lines) including SpellCastOrCopy → SpellCopied duplication
    /// - Static ability parsing (S: lines) with alternative cost keyword conversion
    /// - Replacement effect parsing (R: lines)
    /// - SVars
    /// - Double-faced card back face setup
    /// - Intrinsic mana abilities (basic land subtypes)
    /// - Keyword-generated abilities and triggers (Cycling, Prowess, Bushido)
    pub fn from_rules(rules: &CardRules, owner: PlayerId) -> Self {
        let face = &rules.main_part;
        let mut next_trigger_id = 0u32;

        // Parse triggers from T: lines
        let mut triggers: Vec<Trigger> = Vec::new();
        let mut spell_cast_or_copy_raw: Vec<String> = Vec::new();
        for raw in &face.triggers {
            if let Some(trig) = parse_trigger(raw, &mut next_trigger_id) {
                triggers.push(trig);
                if raw.contains("Mode$ SpellCastOrCopy") {
                    spell_cast_or_copy_raw.push(raw.clone());
                }
            }
        }
        // Duplicate SpellCastOrCopy triggers as SpellCopied (for Magecraft)
        for raw in &spell_cast_or_copy_raw {
            let converted = raw.replace("Mode$ SpellCastOrCopy", "Mode$ SpellCopied");
            if let Some(trig) = parse_trigger(&converted, &mut next_trigger_id) {
                triggers.push(trig);
            }
        }

        let mut card = CardInstance::new(
            CardId(0),
            face.name.clone(),
            owner,
            face.type_line.clone(),
            face.mana_cost.clone(),
            face.resolved_color(),
            face.int_power,
            face.int_toughness,
            face.keywords.clone(),
            face.abilities.clone(),
        );

        // Append card-text triggers to keyword-generated ones (from generate_keyword_triggers)
        card.triggers.extend(triggers);
        // Merge card-text SVars (keyword-generated SVars already set by constructor)
        for (k, v) in &face.svars {
            card.svars.entry(k.clone()).or_insert_with(|| v.clone());
        }

        // Parse static abilities from S: lines
        for raw in &face.static_abilities {
            // Convert Mode$ AlternativeCost | Cost$ GainLife<N> to keyword for runtime detection
            if let Some(kw) = parse_gainlife_alt_cost_keyword(raw) {
                card.keywords.push(kw);
            }
            let prefixed = format!("S$ {}", raw);
            if let Some(sa) = parse_static_ability(&prefixed) {
                card.static_abilities.push(sa);
            }
        }

        // Parse replacement effects from R: lines
        for raw in &face.replacements {
            let prefixed = format!("R$ {}", raw);
            if let Some(re) = parse_replacement_effect(&prefixed) {
                card.replacement_effects.push(re);
            }
        }

        // Double-faced cards
        if rules.split_type.is_dual_faced() {
            if let Some(ref back_face) = rules.other_part {
                let mut back_trigger_id = 0u32;
                let back_triggers: Vec<_> = back_face
                    .triggers
                    .iter()
                    .filter_map(|raw| parse_trigger(raw, &mut back_trigger_id))
                    .collect();

                card.other_part = Some(CardOtherPart {
                    name: back_face.name.clone(),
                    type_line: back_face.type_line.clone(),
                    mana_cost: back_face.mana_cost.clone(),
                    color: back_face.resolved_color(),
                    base_power: back_face.int_power,
                    base_toughness: back_face.int_toughness,
                    keywords: back_face.keywords.clone(),
                    abilities: back_face.abilities.clone(),
                    triggers: back_triggers,
                    svars: back_face.svars.clone(),
                });
            }
        }

        card
    }

    /// Effective power, accounting for all layer effects and counters.
    ///
    /// Calculation order (CR 613):
    /// - Layer 7b: `static_set_power` overrides `base_power` if set.
    /// - Layer 7c: `static_power_modifier` (anthem bonuses) is added.
    /// - Temporary: `power_modifier` (from spells like Giant Growth) is added.
    /// - Layer 7d: +1/+1 and -1/-1 counters are factored in.
    pub fn power(&self) -> i32 {
        let base = self
            .static_set_power
            .unwrap_or(self.base_power.unwrap_or(0));
        base + self.static_power_modifier
            + self.power_modifier
            + self.counter_count(&CounterType::P1P1)
            - self.counter_count(&CounterType::M1M1)
    }

    /// Effective toughness, accounting for all layer effects and counters.
    pub fn toughness(&self) -> i32 {
        let base = self
            .static_set_toughness
            .unwrap_or(self.base_toughness.unwrap_or(0));
        base + self.static_toughness_modifier
            + self.toughness_modifier
            + self.counter_count(&CounterType::P1P1)
            - self.counter_count(&CounterType::M1M1)
    }

    pub fn lethal_damage(&self) -> bool {
        self.damage >= self.toughness()
    }

    pub fn is_creature(&self) -> bool {
        self.type_line.is_creature()
    }

    pub fn is_land(&self) -> bool {
        self.type_line.is_land()
    }

    pub fn is_permanent(&self) -> bool {
        self.type_line.is_permanent()
    }

    /// Check whether this card has a keyword — intrinsically, granted by a
    /// continuous static effect (Layer 6), or temporarily from a pump effect.
    /// Count distinct colors of mana spent to cast this spell (for Sunburst/Converge).
    pub fn sunburst_count(&self) -> i32 {
        use forge_foundation::mana::ManaAtom;
        let mut count = 0;
        for &bit in &[
            ManaAtom::WHITE,
            ManaAtom::BLUE,
            ManaAtom::BLACK,
            ManaAtom::RED,
            ManaAtom::GREEN,
        ] {
            if (self.colors_spent_to_cast & bit) != 0 {
                count += 1;
            }
        }
        count
    }

    pub fn has_keyword(&self, kw: &str) -> bool {
        self.keywords.iter().any(|k| k.eq_ignore_ascii_case(kw))
            || self
                .granted_keywords
                .iter()
                .any(|k| k.eq_ignore_ascii_case(kw))
            || self
                .pump_keywords
                .iter()
                .any(|k| k.eq_ignore_ascii_case(kw))
    }

    pub fn has_haste(&self) -> bool {
        self.has_keyword("Haste")
    }

    pub fn has_flying(&self) -> bool {
        self.has_keyword("Flying")
    }

    pub fn has_reach(&self) -> bool {
        self.has_keyword("Reach")
    }

    pub fn has_first_strike(&self) -> bool {
        self.has_keyword("First Strike")
    }

    pub fn has_double_strike(&self) -> bool {
        self.has_keyword("Double Strike")
    }

    pub fn has_trample(&self) -> bool {
        self.has_keyword("Trample")
    }

    pub fn has_deathtouch(&self) -> bool {
        self.has_keyword("Deathtouch")
    }

    pub fn has_lifelink(&self) -> bool {
        self.has_keyword("Lifelink")
    }

    pub fn has_vigilance(&self) -> bool {
        self.has_keyword("Vigilance")
    }

    pub fn has_defender(&self) -> bool {
        self.has_keyword("Defender")
    }

    pub fn has_hexproof(&self) -> bool {
        self.has_keyword("Hexproof")
    }

    pub fn has_shroud(&self) -> bool {
        self.has_keyword("Shroud")
    }

    pub fn has_menace(&self) -> bool {
        self.has_keyword("Menace")
    }

    pub fn has_fear(&self) -> bool {
        self.has_keyword("Fear")
    }

    pub fn has_intimidate(&self) -> bool {
        self.has_keyword("Intimidate")
    }

    pub fn has_shadow(&self) -> bool {
        self.has_keyword("Shadow")
    }

    pub fn has_skulk(&self) -> bool {
        self.has_keyword("Skulk")
    }

    pub fn has_horsemanship(&self) -> bool {
        self.has_keyword("Horsemanship")
    }

    pub fn has_indestructible(&self) -> bool {
        self.has_keyword("Indestructible")
    }

    pub fn has_infect(&self) -> bool {
        self.has_keyword("Infect")
    }

    pub fn has_wither(&self) -> bool {
        self.has_keyword("Wither")
    }

    pub fn has_prowess(&self) -> bool {
        self.has_keyword("Prowess")
    }

    // ── Keyword cost helpers (pattern: "Keyword:cost_string") ────────

    /// Get buyback cost (e.g. "Buyback:2" → Some("2")).
    pub fn get_buyback_cost(&self) -> Option<String> {
        self.get_keyword_cost("Buyback")
    }

    /// Get spectacle cost (e.g. "Spectacle:B R" → Some("B R")).
    pub fn get_spectacle_cost(&self) -> Option<String> {
        self.get_keyword_cost("Spectacle")
    }

    /// Get GainLife alternative cost info.
    ///
    /// Stored as keyword `AltCostGainLife:N:IsPresent` where N is the life amount
    /// and IsPresent is the condition string (e.g. `Forest.YouCtrl`).
    /// Returns `Some((life_amount, condition))` if present.
    pub fn get_gainlife_alt_cost(&self) -> Option<(i32, String)> {
        for kw in self.keywords.iter().chain(self.granted_keywords.iter()) {
            if let Some(rest) = kw.strip_prefix("AltCostGainLife:") {
                let mut parts = rest.splitn(2, ':');
                let amount = parts
                    .next()
                    .and_then(|s| s.parse::<i32>().ok())
                    .unwrap_or(0);
                let condition = parts.next().unwrap_or("").to_string();
                return Some((amount, condition));
            }
        }
        None
    }

    /// Get evoke cost (e.g. "Evoke:2 B" → Some("2 B")).
    pub fn get_evoke_cost(&self) -> Option<String> {
        self.get_keyword_cost("Evoke")
    }

    /// Get bestow cost (e.g. "Bestow:3 G G" → Some("3 G G")).
    pub fn get_bestow_cost(&self) -> Option<String> {
        self.get_keyword_cost("Bestow")
    }

    /// Get dash cost (e.g. "Dash:1 R" → Some("1 R")).
    pub fn get_dash_cost(&self) -> Option<String> {
        self.get_keyword_cost("Dash")
    }

    /// Get blitz cost (e.g. "Blitz:1 R" → Some("1 R")).
    pub fn get_blitz_cost(&self) -> Option<String> {
        self.get_keyword_cost("Blitz")
    }

    /// Get multikicker cost (e.g. "Multikicker:1 G" → Some("1 G")).
    pub fn get_multikicker_cost(&self) -> Option<String> {
        self.get_keyword_cost("Multikicker")
    }

    /// Get replicate cost (e.g. "Replicate:U" → Some("U")).
    pub fn get_replicate_cost(&self) -> Option<String> {
        self.get_keyword_cost("Replicate")
    }

    /// Get entwine cost (e.g. "Entwine:2" → Some("2")).
    pub fn get_entwine_cost(&self) -> Option<String> {
        self.get_keyword_cost("Entwine")
    }

    /// Get escalate cost (e.g. "Escalate:1" → Some("1")).
    pub fn get_escalate_cost(&self) -> Option<String> {
        self.get_keyword_cost("Escalate")
    }

    /// Get escape cost and exile count (e.g. "Escape:1 B B:4" → Some(("1 B B", 4))).
    pub fn get_escape_cost(&self) -> Option<(String, i32)> {
        for kw in self.keywords.iter().chain(self.granted_keywords.iter()) {
            if let Some(rest) = kw.strip_prefix("Escape:") {
                // Format: "mana_cost:exile_count"
                if let Some(colon_pos) = rest.rfind(':') {
                    let mana = rest[..colon_pos].trim().to_string();
                    let exile = rest[colon_pos + 1..].trim().parse().unwrap_or(0);
                    return Some((mana, exile));
                }
            }
        }
        None
    }

    /// Get overload cost (e.g. "Overload:3 R" → Some("3 R")).
    pub fn get_overload_cost(&self) -> Option<String> {
        self.get_keyword_cost("Overload")
    }

    /// Get madness cost (e.g. "Madness:1 R" → Some("1 R")).
    pub fn get_madness_cost(&self) -> Option<String> {
        self.get_keyword_cost("Madness")
    }

    /// Get strive cost (e.g. "Strive:1 W" → Some("1 W")).
    pub fn get_strive_cost(&self) -> Option<String> {
        self.get_keyword_cost("Strive")
    }

    /// Check if card has Rebound keyword.
    pub fn has_rebound(&self) -> bool {
        self.has_keyword("Rebound")
    }

    /// Get suspend cost and time counters (e.g. "Suspend:1 U:3" → Some(("1 U", 3))).
    pub fn get_suspend_cost(&self) -> Option<(String, i32)> {
        for kw in self.keywords.iter().chain(self.granted_keywords.iter()) {
            if let Some(rest) = kw.strip_prefix("Suspend:") {
                if let Some(colon_pos) = rest.rfind(':') {
                    let mana = rest[..colon_pos].trim().to_string();
                    let counters = rest[colon_pos + 1..].trim().parse().unwrap_or(0);
                    return Some((mana, counters));
                }
            }
        }
        None
    }

    /// Get foretell cost (e.g. "Foretell:W W" → Some("W W")).
    pub fn get_foretell_cost(&self) -> Option<String> {
        self.get_keyword_cost("Foretell")
    }

    /// Get emerge cost (e.g. "Emerge:5 U U" → Some("5 U U")).
    pub fn get_emerge_cost(&self) -> Option<String> {
        self.get_keyword_cost("Emerge")
    }

    /// Get offering type (e.g. "Offering:Snake" → Some("Snake")).
    pub fn get_offering_type(&self) -> Option<String> {
        self.get_keyword_cost("Offering")
    }

    /// Generic keyword cost parser — looks for "Keyword:cost" in keywords vec.
    fn get_keyword_cost(&self, keyword: &str) -> Option<String> {
        let prefix = format!("{}:", keyword);
        for kw in self.keywords.iter().chain(self.granted_keywords.iter()) {
            if let Some(cost) = kw.strip_prefix(&prefix) {
                return Some(cost.to_string());
            }
        }
        None
    }

    /// Check "Hexproof from <color>" variants (e.g. "Hexproof from blue").
    pub fn has_hexproof_from(&self, color: &str) -> bool {
        let target = format!("Hexproof from {}", color);
        self.keywords
            .iter()
            .any(|k| k.eq_ignore_ascii_case(&target))
            || self
                .granted_keywords
                .iter()
                .any(|k| k.eq_ignore_ascii_case(&target))
    }

    /// Get Ward cost (e.g. "Ward:2" → Some("2"), "Ward:{U}" → Some("{U}")).
    pub fn get_ward_cost(&self) -> Option<String> {
        for kw in self.keywords.iter().chain(self.granted_keywords.iter()) {
            if let Some(cost) = kw.strip_prefix("Ward:") {
                return Some(cost.to_string());
            }
        }
        None
    }

    /// Get Toxic count (e.g. "Toxic:1" → Some(1)).
    pub fn get_toxic_count(&self) -> Option<i32> {
        for kw in self.keywords.iter().chain(self.granted_keywords.iter()) {
            if let Some(n) = kw.strip_prefix("Toxic:") {
                return n.parse().ok();
            }
        }
        None
    }

    /// Get Flashback cost (e.g. "Flashback:2 R" → Some("2 R")).
    pub fn get_flashback_cost(&self) -> Option<String> {
        self.get_keyword_cost("Flashback")
    }

    /// Get Kicker cost (e.g. "Kicker:W" → Some("W")).
    pub fn get_kicker_cost(&self) -> Option<String> {
        self.get_keyword_cost("Kicker")
    }

    /// Whether this card has the Storm keyword.
    pub fn has_storm(&self) -> bool {
        self.has_keyword("Storm")
    }

    /// Whether this card has the Cascade keyword.
    pub fn has_cascade(&self) -> bool {
        self.has_keyword("Cascade")
    }

    /// Converted mana cost (mana value).
    pub fn mana_value(&self) -> i32 {
        self.mana_cost.cmc()
    }

    /// Check "Protection from <quality>" (e.g. "Protection from red").
    pub fn has_protection_from(&self, quality: &str) -> bool {
        let target = format!("Protection from {}", quality);
        self.keywords
            .iter()
            .any(|k| k.eq_ignore_ascii_case(&target))
            || self
                .granted_keywords
                .iter()
                .any(|k| k.eq_ignore_ascii_case(&target))
    }

    /// Get all "Protection from X" values this card has.
    pub fn get_protections(&self) -> Vec<String> {
        let mut prots = Vec::new();
        for kw in self.keywords.iter().chain(self.granted_keywords.iter()) {
            if let Some(from) = kw.strip_prefix("Protection from ") {
                prots.push(from.to_lowercase());
            }
        }
        prots
    }

    /// Check if this card is protected from a source card.
    /// Protection from <color> checks source's color.
    /// Protection from <type> checks source's type (e.g. "artifacts", "creatures").
    pub fn is_protected_from(&self, source: &CardInstance) -> bool {
        for prot in self.get_protections() {
            match prot.as_str() {
                "white" => {
                    if source.color.has_white() {
                        return true;
                    }
                }
                "blue" => {
                    if source.color.has_blue() {
                        return true;
                    }
                }
                "black" => {
                    if source.color.has_black() {
                        return true;
                    }
                }
                "red" => {
                    if source.color.has_red() {
                        return true;
                    }
                }
                "green" => {
                    if source.color.has_green() {
                        return true;
                    }
                }
                "colorless" => {
                    if source.color.is_colorless() {
                        return true;
                    }
                }
                "artifacts" => {
                    if source.type_line.is_artifact() {
                        return true;
                    }
                }
                "creatures" => {
                    if source.type_line.is_creature() {
                        return true;
                    }
                }
                "enchantments" => {
                    if source.type_line.is_enchantment() {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    pub fn can_attack(&self) -> bool {
        self.is_creature()
            && !self.tapped
            && !self.has_defender()
            && !self.cant_attack_static
            && !self.detained
            && (self.has_haste() || !self.summoning_sick)
            && self.zone == ZoneType::Battlefield
    }

    pub fn can_block(&self) -> bool {
        self.is_creature()
            && !self.tapped
            && !self.cant_block_static
            && !self.detained
            && self.zone == ZoneType::Battlefield
    }

    /// Check if this card can be controlled by the given player
    /// (e.g., checks for "Other players can't gain control of CARDNAME.")
    pub fn can_be_controlled_by(&self, _player: PlayerId) -> bool {
        // TODO: Check for "Other players can't gain control of CARDNAME." keyword
        // For now, return true - all cards can be controlled
        true
    }

    pub fn counter_count(&self, ct: &CounterType) -> i32 {
        *self.counters.get(ct).unwrap_or(&0)
    }

    pub fn add_counter(&mut self, ct: &CounterType, count: i32) {
        let entry = self.counters.entry(ct.clone()).or_insert(0);
        *entry += count;
    }

    pub fn remove_counter(&mut self, ct: &CounterType, count: i32) {
        let entry = self.counters.entry(ct.clone()).or_insert(0);
        *entry = (*entry - count).max(0);
    }

    /// Reset state when entering the battlefield.
    pub fn enter_battlefield(&mut self) {
        self.tapped = false;
        self.damage = 0;
        self.summoning_sick = true;
        self.has_deathtouch_damage = false;
        self.entered_battlefield_this_turn = true;
        self.attacked_this_turn = false;
        self.damage_sources_this_turn.clear();
    }

    /// Reset per-turn state at start of turn.
    pub fn new_turn(&mut self) {
        self.entered_battlefield_this_turn = false;
        self.attacked_this_turn = false;
        self.has_deathtouch_damage = false;
        self.damage_sources_this_turn.clear();
        if self.zone == ZoneType::Battlefield {
            self.summoning_sick = false;
        }
    }

    /// Add a remembered card (for RememberCountered, etc.)
    pub fn add_remembered_card(&mut self, card_id: CardId) {
        if !self.remembered_cards.contains(&card_id) {
            self.remembered_cards.push(card_id);
        }
    }

    /// Add a remembered CMC value
    pub fn add_remembered_cmc(&mut self, cmc: i32) {
        self.remembered_cmc.push(cmc);
    }

    /// Transform this double-faced card to its other face.
    /// Swaps all face-dependent characteristics with `other_part`.
    /// No-op if `other_part` is `None`.
    /// Mirrors Java's `CardUtil.applyState(card, CardStateName.Backside)`.
    pub fn transform(&mut self) {
        if let Some(other) = self.other_part.as_mut() {
            std::mem::swap(&mut self.card_name, &mut other.name);
            std::mem::swap(&mut self.type_line, &mut other.type_line);
            std::mem::swap(&mut self.mana_cost, &mut other.mana_cost);
            std::mem::swap(&mut self.color, &mut other.color);
            std::mem::swap(&mut self.base_power, &mut other.base_power);
            std::mem::swap(&mut self.base_toughness, &mut other.base_toughness);
            std::mem::swap(&mut self.keywords, &mut other.keywords);
            std::mem::swap(&mut self.abilities, &mut other.abilities);
            std::mem::swap(&mut self.triggers, &mut other.triggers);
            std::mem::swap(&mut self.svars, &mut other.svars);

            // Reset per-face transient state
            self.power_modifier = 0;
            self.toughness_modifier = 0;
            self.damage = 0;
            self.granted_keywords.clear();

            // Re-parse activated abilities from new face's abilities
            self.activated_abilities = self
                .abilities
                .iter()
                .enumerate()
                .filter_map(|(i, raw)| parse_activated_ability(raw, i))
                .collect();

            self.is_transformed = !self.is_transformed;
        }
    }
}

/// Counter types commonly used in MTG.
/// Note: `Copy` is intentionally absent because the `Named(String)` variant
/// holds heap-allocated data. Use `.clone()` when an owned copy is needed.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CounterType {
    P1P1,
    M1M1,
    Poison,
    Loyalty,
    Charge,
    Quest,
    Study,
    Age,
    Fade,
    Time,
    Depletion,
    Storage,
    Mining,
    Brick,
    Level,
    Lore,
    Page,
    Dream,
    /// Catch-all for counter types not in the enum (e.g. SUPPLY, VERSE, LUCK).
    /// Stored as uppercase name for consistent comparison.
    Named(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_foundation::ManaCost;

    #[test]
    fn card_power_toughness() {
        let mut card = CardInstance::new(
            CardId(0),
            "Test".to_string(),
            PlayerId(0),
            CardTypeLine::parse("Creature Bear"),
            ManaCost::parse("1 G"),
            ColorSet::GREEN,
            Some(2),
            Some(2),
            vec![],
            vec![],
        );
        assert_eq!(card.power(), 2);
        assert_eq!(card.toughness(), 2);

        card.add_counter(&CounterType::P1P1, 1);
        assert_eq!(card.power(), 3);
        assert_eq!(card.toughness(), 3);
    }

    #[test]
    fn can_attack() {
        let mut card = CardInstance::new(
            CardId(0),
            "Test".to_string(),
            PlayerId(0),
            CardTypeLine::parse("Creature Bear"),
            ManaCost::parse("1 G"),
            ColorSet::GREEN,
            Some(2),
            Some(2),
            vec![],
            vec![],
        );
        card.zone = ZoneType::Battlefield;
        assert!(!card.can_attack()); // summoning sick

        card.summoning_sick = false;
        assert!(card.can_attack());

        card.tapped = true;
        assert!(!card.can_attack()); // tapped
    }

    #[test]
    fn haste_bypasses_summoning_sickness() {
        let mut card = CardInstance::new(
            CardId(0),
            "Test".to_string(),
            PlayerId(0),
            CardTypeLine::parse("Creature Bear"),
            ManaCost::parse("1 G"),
            ColorSet::GREEN,
            Some(2),
            Some(2),
            vec!["Haste".to_string()],
            vec![],
        );
        card.zone = ZoneType::Battlefield;
        assert!(card.can_attack()); // haste means no summoning sickness check
    }

    #[test]
    fn keyword_helpers() {
        let card = CardInstance::new(
            CardId(0),
            "Test".to_string(),
            PlayerId(0),
            CardTypeLine::parse("Creature Bear"),
            ManaCost::parse("1 G"),
            ColorSet::GREEN,
            Some(2),
            Some(2),
            vec![
                "Hexproof".to_string(),
                "Menace".to_string(),
                "Indestructible".to_string(),
            ],
            vec![],
        );
        assert!(card.has_hexproof());
        assert!(card.has_menace());
        assert!(card.has_indestructible());
        assert!(!card.has_shroud());
        assert!(!card.has_fear());
        assert!(!card.has_shadow());
    }

    #[test]
    fn protection_from_color() {
        let knight = CardInstance::new(
            CardId(0),
            "White Knight".to_string(),
            PlayerId(0),
            CardTypeLine::parse("Creature Knight"),
            ManaCost::parse("W W"),
            ColorSet::WHITE,
            Some(2),
            Some(2),
            vec!["Protection from black".to_string()],
            vec![],
        );
        let black_source = CardInstance::new(
            CardId(1),
            "Doom Blade".to_string(),
            PlayerId(1),
            CardTypeLine::parse("Instant"),
            ManaCost::parse("1 B"),
            ColorSet::BLACK,
            None,
            None,
            vec![],
            vec![],
        );
        let green_source = CardInstance::new(
            CardId(2),
            "Giant Growth".to_string(),
            PlayerId(1),
            CardTypeLine::parse("Instant"),
            ManaCost::parse("G"),
            ColorSet::GREEN,
            None,
            None,
            vec![],
            vec![],
        );
        assert!(knight.is_protected_from(&black_source));
        assert!(!knight.is_protected_from(&green_source));
        assert!(knight.has_protection_from("black"));
        assert!(!knight.has_protection_from("red"));
    }

    #[test]
    fn ward_and_toxic_parsing() {
        let ward_card = CardInstance::new(
            CardId(0),
            "Ward Bear".to_string(),
            PlayerId(0),
            CardTypeLine::parse("Creature Bear"),
            ManaCost::parse("1 U"),
            ColorSet::BLUE,
            Some(2),
            Some(2),
            vec!["Ward:2".to_string()],
            vec![],
        );
        assert_eq!(ward_card.get_ward_cost(), Some("2".to_string()));

        let toxic_card = CardInstance::new(
            CardId(1),
            "Toxic Elf".to_string(),
            PlayerId(0),
            CardTypeLine::parse("Creature Elf"),
            ManaCost::parse("G"),
            ColorSet::GREEN,
            Some(1),
            Some(1),
            vec!["Toxic:1".to_string()],
            vec![],
        );
        assert_eq!(toxic_card.get_toxic_count(), Some(1));

        // No ward/toxic
        let plain = CardInstance::new(
            CardId(2),
            "Bear".to_string(),
            PlayerId(0),
            CardTypeLine::parse("Creature Bear"),
            ManaCost::parse("1 G"),
            ColorSet::GREEN,
            Some(2),
            Some(2),
            vec![],
            vec![],
        );
        assert_eq!(plain.get_ward_cost(), None);
        assert_eq!(plain.get_toxic_count(), None);
    }

    #[test]
    fn hexproof_from_color() {
        let card = CardInstance::new(
            CardId(0),
            "Test".to_string(),
            PlayerId(0),
            CardTypeLine::parse("Creature Bear"),
            ManaCost::parse("1 G"),
            ColorSet::GREEN,
            Some(2),
            Some(2),
            vec!["Hexproof from blue".to_string()],
            vec![],
        );
        assert!(card.has_hexproof_from("blue"));
        assert!(!card.has_hexproof_from("red"));
        assert!(!card.has_hexproof()); // partial hexproof is not full hexproof
    }
}
