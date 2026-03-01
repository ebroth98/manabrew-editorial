pub mod card_property;

use std::collections::{BTreeMap, HashMap};

use forge_foundation::{CardTypeLine, ColorSet, ManaCost, ZoneType};
use serde::{Deserialize, Serialize};

use crate::ability::activated::{parse_activated_ability, ActivatedAbility};
use crate::ids::{CardId, PlayerId};
use crate::replacement::{parse_replacement_effect, ReplacementEffect};
use crate::staticability::{parse_static_ability, StaticAbility};
use crate::trigger::Trigger;

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
    pub summoning_sick: bool,
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
    /// True if detained — can't attack, block, or activate abilities. Clears at controller's next turn.
    pub detained: bool,
    /// Player who goaded this creature. Goaded creature must attack but can't attack goader.
    pub goaded_by: Option<PlayerId>,
    /// Damage prevention shields (decremented when damage would be dealt). Resets at EOT.
    pub damage_prevention: i32,
    /// True if this creature must block if able.
    pub must_block: bool,
    /// Spell cards encoded/ciphered onto this creature.
    pub encoded_cards: Vec<CardId>,
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
            summoning_sick: true,
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
            is_transformed: false,
            other_part: None,
            set_code: None,
            phased_out: false,
            regeneration_shields: 0,
            kicked: false,
            chosen_colors: Vec::new(),
            chosen_cards: Vec::new(),
            animate_state: None,
            chosen_type: None,
            named_cards: Vec::new(),
            chosen_number: None,
            chosen_player: None,
            detained: false,
            goaded_by: None,
            damage_prevention: 0,
            must_block: false,
            encoded_cards: Vec::new(),
        };

        // Generate keyword-derived activated abilities (mirrors Java CardFactoryUtil.setupKeywordedAbilities)
        card.generate_keyword_abilities();
        card
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
            + self.counter_count(CounterType::P1P1)
            - self.counter_count(CounterType::M1M1)
    }

    /// Effective toughness, accounting for all layer effects and counters.
    pub fn toughness(&self) -> i32 {
        let base = self
            .static_set_toughness
            .unwrap_or(self.base_toughness.unwrap_or(0));
        base + self.static_toughness_modifier
            + self.toughness_modifier
            + self.counter_count(CounterType::P1P1)
            - self.counter_count(CounterType::M1M1)
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

    /// Get evoke cost (e.g. "Evoke:2 B" → Some("2 B")).
    pub fn get_evoke_cost(&self) -> Option<String> {
        self.get_keyword_cost("Evoke")
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

    pub fn counter_count(&self, ct: CounterType) -> i32 {
        *self.counters.get(&ct).unwrap_or(&0)
    }

    pub fn add_counter(&mut self, ct: CounterType, count: i32) {
        let entry = self.counters.entry(ct).or_insert(0);
        *entry += count;
    }

    pub fn remove_counter(&mut self, ct: CounterType, count: i32) {
        let entry = self.counters.entry(ct).or_insert(0);
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
    }

    /// Reset per-turn state at start of turn.
    pub fn new_turn(&mut self) {
        self.entered_battlefield_this_turn = false;
        self.attacked_this_turn = false;
        self.has_deathtouch_damage = false;
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    // Add more as needed
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

        card.add_counter(CounterType::P1P1, 1);
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
