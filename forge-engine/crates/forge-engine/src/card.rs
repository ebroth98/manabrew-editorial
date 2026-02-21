use std::collections::{BTreeMap, HashMap};

use forge_foundation::{CardTypeLine, ColorSet, ManaCost, ZoneType};
use serde::{Deserialize, Serialize};

use crate::activated_ability::{parse_activated_ability, ActivatedAbility};
use crate::ids::{CardId, PlayerId};
use crate::static_ability::{parse_static_ability, StaticAbility};
use crate::trigger::Trigger;

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
        // Parse activated abilities from AB$ lines.
        let activated_abilities: Vec<ActivatedAbility> = abilities
            .iter()
            .enumerate()
            .filter_map(|(i, raw)| parse_activated_ability(raw, i))
            .collect();

        // Parse static abilities from S$ lines.
        // Mirrors Java Forge Card constructor calling StaticAbility.create().
        let static_abilities: Vec<StaticAbility> = abilities
            .iter()
            .filter_map(|raw| parse_static_ability(raw))
            .collect();

        CardInstance {
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
        let base = self.static_set_power.unwrap_or(self.base_power.unwrap_or(0));
        base + self.static_power_modifier
            + self.power_modifier
            + self.counter_count(CounterType::P1P1)
            - self.counter_count(CounterType::M1M1)
    }

    /// Effective toughness, accounting for all layer effects and counters.
    pub fn toughness(&self) -> i32 {
        let base = self.static_set_toughness.unwrap_or(self.base_toughness.unwrap_or(0));
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

    /// Check whether this card has a keyword — either intrinsically or granted
    /// by a continuous static effect (Layer 6).
    pub fn has_keyword(&self, kw: &str) -> bool {
        self.keywords.iter().any(|k| k.eq_ignore_ascii_case(kw))
            || self.granted_keywords.iter().any(|k| k.eq_ignore_ascii_case(kw))
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

    pub fn can_attack(&self) -> bool {
        self.is_creature()
            && !self.tapped
            && !self.has_defender()
            && !self.cant_attack_static
            && (self.has_haste() || !self.summoning_sick)
            && self.zone == ZoneType::Battlefield
    }

    pub fn can_block(&self) -> bool {
        self.is_creature()
            && !self.tapped
            && !self.cant_block_static
            && self.zone == ZoneType::Battlefield
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
}

/// Counter types commonly used in MTG.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CounterType {
    P1P1,
    M1M1,
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
}
