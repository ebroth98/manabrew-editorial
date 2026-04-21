pub mod ability;
pub mod ability_activated;
pub mod ability_mana_part;
pub mod ability_static;
pub mod ability_sub;
pub mod alternative_cost;
pub mod land_ability;
pub mod optional_cost;
pub mod optional_cost_value;
pub mod params;
pub mod spell;
pub mod spell_ability_condition;
pub mod spell_ability_predicates;
pub mod spell_ability_restriction;
pub mod spell_ability_stack_instance;
pub mod spell_ability_variables;
pub mod spell_permanent;
pub mod target_choices;
pub mod target_restrictions;
pub mod trait_spell_ability;

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};

use serde::{Deserialize, Serialize};

use crate::ability::api_type::ApiType;
use crate::ability::AbilityKey;
use crate::agent::PlayerAgent;
use crate::card::card_damage_map::CardDamageMap;
use crate::card::card_zone_table::CardZoneTable;
use crate::cost::{parse_cost, Cost};
use crate::event::AbilityValue;
use crate::game::GameState;
use crate::ids::{CardId, PlayerId};
use crate::mana::ManaPool;
use crate::parsing::{keys, Params};

pub use ability_mana_part::AbilityManaPart;
pub use alternative_cost::{AlternativeCost, MORPH_GENERIC_COST, MORPH_PT};
pub use optional_cost::OptionalCost;
pub use optional_cost_value::OptionalCostValue;
pub use spell_ability_condition::SpellAbilityCondition;
pub use spell_ability_predicates::{has_sub_ability_api, is_api, is_valid};
pub use spell_ability_restriction::SpellAbilityRestriction;
pub use spell_ability_variables::SpellAbilityVariables;
pub use target_choices::TargetChoices;
pub use target_restrictions::{TargetKind, TargetRestrictions};

static NEXT_SPELL_ABILITY_ID: AtomicU32 = AtomicU32::new(1);

fn next_spell_ability_id() -> u32 {
    NEXT_SPELL_ABILITY_ID.fetch_add(1, Ordering::Relaxed)
}

pub trait TriggerKeyInput {
    fn into_ability_key(self) -> Option<AbilityKey>;
}

impl TriggerKeyInput for AbilityKey {
    fn into_ability_key(self) -> Option<AbilityKey> {
        Some(self)
    }
}

impl TriggerKeyInput for &str {
    fn into_ability_key(self) -> Option<AbilityKey> {
        crate::ability::ability_key::from_string(self)
    }
}

impl TriggerKeyInput for String {
    fn into_ability_key(self) -> Option<AbilityKey> {
        crate::ability::ability_key::from_string(&self)
    }
}

impl TriggerKeyInput for &String {
    fn into_ability_key(self) -> Option<AbilityKey> {
        crate::ability::ability_key::from_string(self)
    }
}

// ── SpellAbility (mirrors Java's SpellAbility.java) ──────────────────

/// A spell or ability with its own targeting, costs, and sub-ability chain.
/// Mirrors Java's `SpellAbility` class — each node in the chain has its own
/// `target_restrictions`, `target_chosen`, `sub_ability`, `api`, etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpellAbility {
    #[serde(default)]
    pub id: u32,
    /// Effect API type (e.g. DealDamage, Destroy, Draw).
    /// Mirrors Java's `ApiType api` field.
    pub api: Option<ApiType>,
    /// The card that hosts this ability. Mirrors Java's `hostCard`.
    pub source: Option<CardId>,
    /// Java parity: original host card for granted/copied abilities.
    /// Used by costs like `Unattach<OriginalHost>`.
    #[serde(default)]
    pub original_host: Option<CardId>,
    /// The player who activated/cast this. Mirrors Java's `activatingPlayer`.
    pub activating_player: PlayerId,
    /// The player who chooses this ability's targets. Mirrors Java's
    /// `targetingPlayer` field.
    pub targeting_player: Option<PlayerId>,
    /// The raw ability text (pipe-delimited params).
    pub ability_text: String,
    /// Parsed pipe-delimited parameters.
    pub params: Params,
    /// Targeting restrictions parsed from `ValidTgts$`.
    /// `None` means this ability doesn't use targeting.
    /// Mirrors Java's `targetRestrictions` field.
    pub target_restrictions: Option<TargetRestrictions>,
    /// The chosen targets for this ability.
    /// Mirrors Java's `targetChosen` field.
    pub target_chosen: TargetChoices,
    /// Parsed costs from `Cost$` parameter.
    /// Mirrors Java's `payCosts` field.
    pub pay_costs: Option<Cost>,
    /// Linked sub-ability chain. Mirrors Java's `subAbility` field
    /// (AbilitySub extends SpellAbility).
    pub sub_ability: Option<Box<SpellAbility>>,
    /// Java parity: payload carried by `WrappedAbility`.
    #[serde(default)]
    pub wrapped_ability: Option<Box<SpellAbility>>,
    /// Whether this is a spell (not an ability).
    pub is_spell: bool,
    /// Whether this is a triggered ability.
    pub is_trigger: bool,
    /// Whether this is an activated ability.
    pub is_activated: bool,
    /// Java parity: whether this ability is intrinsic to its host.
    #[serde(default)]
    pub intrinsic: bool,
    /// Card that owns the trigger (for intervening-if recheck).
    pub trigger_source: Option<CardId>,
    /// Zone timestamp of the trigger source when this triggered ability was created.
    /// Used to preserve object identity across zone changes (CR 400.7).
    #[serde(default)]
    pub trigger_source_zone_timestamp: Option<u64>,
    /// Zone timestamp of `source` when this SpellAbility instance was created.
    /// Used for non-target references like `Defined$ Self` to preserve object identity.
    #[serde(default)]
    pub source_zone_timestamp: Option<u64>,
    /// Source trigger id (Java `sourceTrigger`), used for state-trigger dedupe.
    pub source_trigger_id: Option<u32>,
    /// Index into card.triggers for intervening-if recheck.
    pub trigger_index: Option<usize>,
    /// Alternative cost used to cast this spell (Flashback, Spectacle, Evoke, Dash, etc.).
    pub alt_cost: Option<AlternativeCost>,
    /// Index within the card's list of same-kind alternative costs. Zero for
    /// all cases except multi-cost Evoke (intrinsic + granted by Ashling-style
    /// static AddKeyword): 0 = first payable Evoke, 1 = second, …
    #[serde(default)]
    pub alt_cost_index: u8,
    /// Whether the kicker cost was paid.
    pub kicked: bool,
    /// Whether buyback was paid (spell returns to hand on resolve).
    pub buyback_paid: bool,
    /// Whether this spell is overloaded (targets all valid instead of one).
    pub overloaded: bool,
    /// Whether this spell is a copy (created by Storm, Replicate, etc.).
    pub is_copy: bool,
    /// Java parity: life paid while activating or casting this ability.
    #[serde(default)]
    pub paid_life_amount: i32,
    /// Number of times the kicker/multikicker cost was paid.
    pub kick_count: u32,
    /// Number of times the replicate cost was paid.
    pub replicate_count: u32,
    /// Whether a generic optional additional cost was paid.
    pub optional_generic_cost_paid: bool,
    /// Sum of integer values remembered on the trigger that spawned this
    /// ability (Java: TriggerRememberAmount / sa.getTriggerRemembered()).
    pub trigger_remembered_amount: i32,
    /// The value chosen for X in the mana cost (e.g. Fireball X=5 means 5 damage).
    /// Mirrors Java's `SpellAbility.getXManaCostPaid()`.
    pub x_mana_cost_paid: u32,
    /// Cards discarded as part of the cost payment.
    /// Mirrors Java's `CostPayment.getPaidList("Discarded")`.
    pub discarded_cost_cards: Vec<crate::ids::CardId>,
    /// Optional costs that have been paid for this spell.
    /// Mirrors Java's `SpellAbility.optionalCosts`.
    #[serde(default)]
    pub optional_costs: Vec<OptionalCost>,
    /// Hash of costs paid, keyed by cost type with list of values.
    /// Mirrors Java's `SpellAbility.paidHash`.
    #[serde(default)]
    pub paid_hash: HashMap<String, Vec<String>>,
    /// Java parity: mana atoms used to pay this spell or ability.
    #[serde(default)]
    pub paying_mana: Vec<u16>,
    /// Java parity: paid abilities list.
    #[serde(default)]
    pub paid_abilities: Vec<SpellAbility>,
    /// Mana-producing part of this ability (for mana abilities).
    /// Mirrors Java's `SpellAbility.manaPart`.
    pub mana_part: Option<AbilityManaPart>,
    /// Express mana choice forced by callback/autopay for flexible mana abilities.
    #[serde(default)]
    pub express_mana_choice: Option<u16>,
    /// Cards tapped for convoke cost reduction.
    /// Mirrors Java's `SpellAbility.tappedForConvoke`.
    #[serde(default)]
    pub convoke_tapped: Vec<CardId>,
    /// Cards spliced onto this spell.
    /// Mirrors Java's `SpellAbility.splicedCards`.
    #[serde(default)]
    pub spliced_cards: Vec<CardId>,
    /// Announced variable values (e.g. X, number of targets).
    /// Mirrors Java's `SpellAbility.announceVars`.
    #[serde(default)]
    pub announce_vars: HashMap<String, i32>,
    /// Card sacrificed as part of emerge cost.
    /// Mirrors Java's `SpellAbility.sacrificedAsEmerge`.
    pub sacrificed_as_emerge: Option<CardId>,
    /// Card sacrificed as part of offering cost.
    /// Mirrors Java's `SpellAbility.sacrificedAsOffering`.
    pub sacrificed_as_offering: Option<CardId>,
    /// Human-readable description of this ability.
    /// Mirrors Java's `SpellAbility.description`.
    #[serde(default)]
    pub description: String,
    /// Description used when this ability is on the stack.
    /// Mirrors Java's `SpellAbility.stackDescription`.
    #[serde(default)]
    pub stack_description: String,
    /// Whether this is a mana ability (doesn't use the stack).
    /// Mirrors Java's `SpellAbility.isManaAbility`.
    #[serde(default)]
    pub is_mana_ability: bool,
    /// Whether this is a land ability (play land action).
    /// Mirrors Java's `LandAbility` subclass flag.
    #[serde(default)]
    pub is_land_ability: bool,
    /// Trigger objects map for tracking trigger context.
    #[serde(default)]
    pub trigger_objects: HashMap<AbilityKey, AbilityValue>,
    /// Java parity: non-scalar trigger objects that carry spell/ability context.
    #[serde(default)]
    pub trigger_spell_abilities: HashMap<AbilityKey, SpellAbility>,
    /// Java parity: additional ability lists used by mode/charm-style abilities.
    #[serde(default)]
    pub additional_ability_lists: HashMap<String, Vec<SpellAbility>>,
    /// Java parity: replacing-objects payload.
    #[serde(default)]
    pub replacing_objects: HashMap<AbilityKey, AbilityValue>,
    /// Java parity: trigger remembered objects copied from the originating trigger.
    #[serde(default)]
    pub trigger_remembered: Vec<AbilityValue>,
    /// Activation restriction for this ability.
    #[serde(default)]
    pub restriction: SpellAbilityRestriction,
    /// Condition that must be met for the effect to apply.
    #[serde(default)]
    pub condition: SpellAbilityCondition,
    /// Rollback effects tracked for undo support.
    #[serde(default)]
    pub rollback_effects: Vec<String>,
    /// Keyword amounts for optional keyword costs.
    #[serde(default)]
    pub optional_keyword_amounts: HashMap<String, i32>,
    /// Pips to reduce from cost.
    #[serde(default)]
    pub pips_to_reduce: Vec<String>,
    /// Java parity: whether copied effects may choose new targets.
    #[serde(default)]
    pub may_choose_new_targets: bool,
    /// Last known state for LKI tracking.
    #[serde(default)]
    pub last_state: HashMap<String, String>,
    /// Java parity: batched zone-change table accumulated for `ChangeZoneResolve`.
    #[serde(skip)]
    pub change_zone_table: Option<CardZoneTable>,
    /// Java parity: accumulated damage map for `DamageResolve`.
    #[serde(skip)]
    pub damage_map: Option<CardDamageMap>,
    /// Java parity: accumulated prevented-damage map for `DamageResolve`.
    #[serde(skip)]
    pub prevent_map: Option<CardDamageMap>,
}

/// Mirrors Java's `SpellAbility.toString()`.
/// Walks the sub-ability chain, concatenating descriptions.
impl std::fmt::Display for SpellAbility {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut node = Some(self);
        let mut first = true;
        while let Some(current) = node {
            if !first {
                write!(f, " ")?;
            }
            first = false;
            write!(f, "{}", current.description)?;
            node = current.sub_ability.as_deref();
        }
        Ok(())
    }
}

impl SpellAbility {
    /// Whether this ability uses targeting.
    /// Mirrors Java's `usesTargeting()`: `return targetRestrictions != null`.
    pub fn uses_targeting(&self) -> bool {
        self.target_restrictions.is_some()
    }

    /// Check if a parameter is set to "True" (case-insensitive).
    /// Common pattern for boolean params like `Ninjutsu$ True`, `Mega$ True`, etc.
    pub fn param_is_true(&self, key: &str) -> bool {
        self.params.is_true(key)
    }

    /// Get the chosen targets. Mirrors Java's `getTargets()`.
    pub fn get_targets(&self) -> &TargetChoices {
        &self.target_chosen
    }

    /// Get the chosen targets mutably. Mirrors Java's `getTargets()` for mutation.
    pub fn get_targets_mut(&mut self) -> &mut TargetChoices {
        &mut self.target_chosen
    }

    /// Get the sub-ability. Mirrors Java's `getSubAbility()`.
    pub fn get_sub_ability(&self) -> Option<&SpellAbility> {
        self.sub_ability.as_deref()
    }

    /// Get the sub-ability mutably.
    pub fn get_sub_ability_mut(&mut self) -> Option<&mut SpellAbility> {
        self.sub_ability.as_deref_mut()
    }

    /// Mirrors Java's `SpellAbility.isWrapper()`.
    pub fn is_wrapper(&self) -> bool {
        self.wrapped_ability.is_some()
    }

    /// Mirrors Java's `WrappedAbility.getWrappedAbility()`.
    pub fn get_wrapped_ability(&self) -> &SpellAbility {
        self.wrapped_ability
            .as_deref()
            .expect("SpellAbility.get_wrapped_ability called on non-wrapper")
    }

    pub fn get_wrapped_ability_mut(&mut self) -> &mut SpellAbility {
        self.wrapped_ability
            .as_deref_mut()
            .expect("SpellAbility.get_wrapped_ability_mut called on non-wrapper")
    }

    pub fn set_wrapped_ability(&mut self, wrapped: SpellAbility) {
        self.wrapped_ability = Some(Box::new(wrapped));
    }

    /// Clear the chosen targets. Mirrors Java's `clearTargets()`.
    pub fn clear_targets(&mut self) {
        self.target_chosen = TargetChoices::default();
    }

    /// Walk the entire ability chain and choose targets for each node that
    /// uses targeting. Mirrors Java's `SpellAbility.setupTargets()` do/while loop.
    ///
    /// Returns `true` if all targeting succeeded, `false` if any node couldn't
    /// find valid targets.
    pub fn setup_targets(
        &mut self,
        game: &GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        mana_pools: &[ManaPool],
    ) -> bool {
        // Walk self, then sub_ability chain — mirrors Java's do/while
        if self.uses_targeting() {
            self.clear_targets();
            self.targeting_player = choose_targeting_player(self, game, agents);
            let player = self.targeting_player.unwrap_or(self.activating_player);
            if !agents[player.index()].choose_targets_for(self, game, mana_pools) {
                return false;
            }
        }

        // Walk sub-ability chain
        let mut current = self.sub_ability.as_deref_mut();
        while let Some(sa) = current {
            if sa.uses_targeting() {
                sa.clear_targets();
                sa.targeting_player = choose_targeting_player(sa, game, agents);
                let player = sa.targeting_player.unwrap_or(sa.activating_player);
                if !agents[player.index()].choose_targets_for(sa, game, mana_pools) {
                    return false;
                }
            }
            current = sa.sub_ability.as_deref_mut();
        }

        if !crate::staticability::static_ability_must_target::meets_must_target_restriction(
            game, self,
        ) {
            return false;
        }

        true
    }

    /// Create a simple SpellAbility for tests and triggers.
    pub fn new_simple(source: Option<CardId>, player: PlayerId, ability_text: &str) -> Self {
        let _perf_scope = crate::perf::ParamsLookupScopeGuard::enter(
            crate::perf::ParamsLookupScope::AbilityBuild,
        );
        let params = Params::from_raw(ability_text);
        let api = crate::parsing::raw_get(ability_text, keys::SP)
            .or_else(|| crate::parsing::raw_get(ability_text, keys::DB))
            .or_else(|| crate::parsing::raw_get(ability_text, keys::AB))
            .and_then(|s| ApiType::smart_value_of(s));
        let target_restrictions = if crate::parsing::raw_has_key(ability_text, keys::VALID_TGTS) {
            TargetRestrictions::new(&params)
        } else {
            None
        };
        let cost = crate::parsing::raw_get(ability_text, keys::COST).map(parse_cost);

        SpellAbility {
            id: next_spell_ability_id(),
            api,
            source,
            original_host: None,
            activating_player: player,
            targeting_player: None,
            ability_text: ability_text.to_string(),
            params,
            target_restrictions,
            target_chosen: TargetChoices::default(),
            pay_costs: cost,
            sub_ability: None,
            wrapped_ability: None,
            is_spell: false,
            is_trigger: false,
            is_activated: false,
            intrinsic: false,
            trigger_source: None,
            trigger_source_zone_timestamp: None,
            source_zone_timestamp: None,
            source_trigger_id: None,
            trigger_index: None,
            alt_cost: None,
            alt_cost_index: 0,
            kicked: false,
            buyback_paid: false,
            overloaded: false,
            is_copy: false,
            paid_life_amount: 0,
            kick_count: 0,
            replicate_count: 0,
            optional_generic_cost_paid: false,
            trigger_remembered_amount: 0,
            x_mana_cost_paid: 0,
            discarded_cost_cards: Vec::new(),
            optional_costs: Vec::new(),
            paid_hash: HashMap::new(),
            paying_mana: Vec::new(),
            paid_abilities: Vec::new(),
            mana_part: None,
            express_mana_choice: None,
            convoke_tapped: Vec::new(),
            spliced_cards: Vec::new(),
            announce_vars: HashMap::new(),
            sacrificed_as_emerge: None,
            sacrificed_as_offering: None,
            description: String::new(),
            stack_description: String::new(),
            is_mana_ability: false,
            is_land_ability: false,
            trigger_objects: HashMap::new(),
            trigger_spell_abilities: HashMap::new(),
            additional_ability_lists: HashMap::new(),
            replacing_objects: HashMap::new(),
            trigger_remembered: Vec::new(),
            restriction: SpellAbilityRestriction::default(),
            condition: SpellAbilityCondition::default(),
            rollback_effects: Vec::new(),
            optional_keyword_amounts: HashMap::new(),
            pips_to_reduce: Vec::new(),
            may_choose_new_targets: false,
            last_state: HashMap::new(),
            change_zone_table: None,
            damage_map: None,
            prevent_map: None,
        }
    }

    /// Create a minimal empty SpellAbility stub.
    /// Mirrors Java's common `SpellAbility.EmptySa` usage.
    pub fn new_empty(source: Option<CardId>, player: PlayerId) -> Self {
        Self::new_simple(source, player, "")
    }

    /// Create a minimal land-play SpellAbility stub.
    pub fn new_land(source: Option<CardId>, player: PlayerId) -> Self {
        let mut sa = Self::new_empty(source, player);
        sa.is_land_ability = true;
        sa
    }

    // ── Sub-ability chain walking ─────────────────────────────────────────

    /// Walk the sub-ability chain looking for a specific API type.
    /// Mirrors Java's `SpellAbility.findSubAbilityByType(ApiType)`.
    pub fn find_sub_ability_by_type(&self, api: ApiType) -> Option<&SpellAbility> {
        let mut current = self.sub_ability.as_deref();
        while let Some(sub) = current {
            if sub.api == Some(api) {
                return Some(sub);
            }
            current = sub.sub_ability.as_deref();
        }
        None
    }

    // ── Mana part delegation ──────────────────────────────────────────────

    /// Whether this ability can produce mana.
    /// Mirrors Java's `SpellAbility.canThisProduce()`.
    pub fn can_this_produce(&self) -> bool {
        match &self.mana_part {
            Some(mp) => mp.can_this_produce(),
            None => false,
        }
    }

    /// Whether this ability can produce a specific color.
    /// Mirrors Java's `SpellAbility.canProduce(String)`.
    pub fn can_produce(&self, color: &str) -> bool {
        match &self.mana_part {
            Some(mp) => mp.can_produce(color),
            None => false,
        }
    }

    /// Amount of mana generated by this ability.
    /// Mirrors Java's `SpellAbility.amountOfManaGenerated()`.
    pub fn amount_of_mana_generated(&self) -> i32 {
        match &self.mana_part {
            Some(mp) => mp.amount_of_mana_generated(),
            None => 0,
        }
    }

    /// Total amount of mana generated, counting Any/All as 1.
    /// Mirrors Java's `SpellAbility.totalAmountOfManaGenerated()`.
    pub fn total_amount_of_mana_generated(&self) -> i32 {
        match &self.mana_part {
            Some(mp) => mp.total_amount_of_mana_generated(),
            None => 0,
        }
    }

    // ── Cost and payment ──────────────────────────────────────────────────

    /// Whether paying with shard mana is allowed.
    /// Mirrors Java's `SpellAbility.allowsPayingWithShard()`.
    pub fn allows_paying_with_shard(&self) -> bool {
        self.params.is_true("AllowsPayingWithShard")
    }

    /// Whether this ability cannot be copied.
    /// Mirrors Java's `SpellAbility.cantBeCopied()`.
    pub fn cant_be_copied(&self) -> bool {
        self.params.is_true("CantBeCopied")
    }

    /// Whether this ability can be played (checks restrictions).
    /// Mirrors Java's `SpellAbility.canPlay()`.
    pub fn can_play(&self, game: &GameState) -> bool {
        if let Some(card_id) = self.source {
            self.restriction
                .can_play(game, card_id, self.activating_player)
        } else {
            true
        }
    }

    /// Whether this ability can be played with optional costs.
    /// Mirrors Java's `SpellAbility.canPlayWithOptionalCost()`.
    pub fn can_play_with_optional_cost(&self) -> bool {
        !self.optional_costs.is_empty()
    }

    /// Whether to prompt even if this is the only possible ability.
    /// Mirrors Java's `SpellAbility.promptIfOnlyPossibleAbility()`.
    pub fn prompt_if_only_possible_ability(&self) -> bool {
        self.params.is_true("PromptIfOnlyPossible")
    }

    /// Add an optional cost to this ability.
    /// Mirrors Java's `SpellAbility.addOptionalCost(OptionalCost)`.
    pub fn add_optional_cost(&mut self, cost: OptionalCost) {
        if !self.optional_costs.contains(&cost) {
            self.optional_costs.push(cost);
        }
    }

    /// Whether the mana cost contains X.
    /// Mirrors Java's `SpellAbility.costHasX()`.
    pub fn cost_has_x(&self) -> bool {
        self.ability_text.contains("X")
            || self.params.get("Cost").map_or(false, |c| c.contains('X'))
    }

    /// Whether the mana cost contains X (mana-specific check).
    /// Mirrors Java's `SpellAbility.costHasManaX()`.
    pub fn cost_has_mana_x(&self) -> bool {
        self.params.get("Cost").map_or(false, |c| c.contains('X'))
    }

    /// Whether conditions are met for this ability.
    /// Mirrors Java's `SpellAbility.metConditions()`.
    pub fn met_conditions(&self, game: &GameState) -> bool {
        self.condition.are_met(game, self)
    }

    /// Clear mana paid tracking.
    /// Mirrors Java's `SpellAbility.clearManaPaid()`.
    pub fn clear_mana_paid(&mut self) {
        self.x_mana_cost_paid = 0;
    }

    /// Apply effects from paying mana (e.g. Sunburst).
    /// Mirrors Java's `SpellAbility.applyPayingManaEffects()`.
    pub fn apply_paying_mana_effects(&mut self) {
        // Mana payment effects are applied during resolution based on
        // the colors of mana spent, tracked in the card's colors_spent_to_cast.
    }

    /// Run this ability (no-op in Rust; Java resolves via resolveStack).
    /// Mirrors Java's `SpellAbility.run()`.
    pub fn run(&self) {
        // Resolution is handled by the stack resolution system in Rust.
        // This method exists for API parity with Java.
    }

    // ── Paid cost tracking ────────────────────────────────────────────────

    /// Add a value to the paid cost hash.
    /// Mirrors Java's `SpellAbility.addCostToHashList(String, String)`.
    pub fn add_cost_to_hash_list(&mut self, key: &str, value: &str) {
        self.paid_hash
            .entry(key.to_string())
            .or_default()
            .push(value.to_string());
    }

    /// Reset the paid cost hash.
    /// Mirrors Java's `SpellAbility.resetPaidHash()`.
    pub fn reset_paid_hash(&mut self) {
        self.paid_hash.clear();
    }

    // ── Trigger objects ───────────────────────────────────────────────────

    /// Check if a triggering object is set.
    /// Mirrors Java's `SpellAbility.hasTriggeringObject(String)`.
    pub fn has_triggering_object<K: TriggerKeyInput>(&self, key: K) -> bool {
        key.into_ability_key()
            .map(|parsed| self.trigger_objects.contains_key(&parsed))
            .unwrap_or(false)
    }

    /// Get a triggering object value by key.
    pub fn get_triggering_value(&self, key: AbilityKey) -> Option<&AbilityValue> {
        self.trigger_objects.get(&key)
    }

    /// Get a triggering card by key.
    pub fn get_triggering_card(&self, key: AbilityKey) -> Option<CardId> {
        match self.get_triggering_value(key) {
            Some(AbilityValue::Card(card)) => Some(*card),
            Some(AbilityValue::Cards(cards)) => cards.first().copied(),
            _ => None,
        }
    }

    /// Get a triggering player by key.
    pub fn get_triggering_player(&self, key: AbilityKey) -> Option<PlayerId> {
        match self.get_triggering_value(key) {
            Some(AbilityValue::Player(player)) => Some(*player),
            Some(AbilityValue::Players(players)) => players.first().copied(),
            _ => None,
        }
    }

    /// Get triggering cards by key.
    pub fn get_triggering_cards(&self, key: AbilityKey) -> Vec<CardId> {
        match self.get_triggering_value(key) {
            Some(AbilityValue::Card(card)) => vec![*card],
            Some(AbilityValue::Cards(cards)) => cards.clone(),
            _ => Vec::new(),
        }
    }

    /// Get triggering players by key.
    pub fn get_triggering_players(&self, key: AbilityKey) -> Vec<PlayerId> {
        match self.get_triggering_value(key) {
            Some(AbilityValue::Player(player)) => vec![*player],
            Some(AbilityValue::Players(players)) => players.clone(),
            _ => Vec::new(),
        }
    }

    /// Get a triggering object by key.
    /// Mirrors Java's `SpellAbility.getTriggeringObject(String)`.
    pub fn get_triggering_object<K: TriggerKeyInput>(&self, key: K) -> Option<&str> {
        key.into_ability_key()
            .and_then(|parsed| self.get_triggering_value(parsed))
            .and_then(|value| match value {
                AbilityValue::String(raw) => Some(raw.as_str()),
                _ => None,
            })
    }

    /// Clear all triggering objects.
    /// Mirrors Java's `SpellAbility.resetTriggeringObjects()`.
    pub fn reset_triggering_objects(&mut self) {
        self.trigger_objects.clear();
    }

    /// Cleanup after resolution — reset targets, trigger objects, paid hash.
    /// Mirrors Java's `SpellAbility.resetOnceResolved()`.
    pub fn reset_once_resolved(&mut self) {
        self.clear_targets();
        self.reset_triggering_objects();
        self.reset_paid_hash();
        self.x_mana_cost_paid = 0;
        self.kick_count = 0;
        self.replicate_count = 0;
        self.optional_generic_cost_paid = false;
        self.discarded_cost_cards.clear();
        self.optional_costs.clear();
        self.convoke_tapped.clear();
        self.spliced_cards.clear();
        self.announce_vars.clear();
        self.sacrificed_as_emerge = None;
        self.sacrificed_as_offering = None;
    }

    // ── Description and text ──────────────────────────────────────────────

    /// Generate a unique key for this ability.
    /// Mirrors Java's `SpellAbility.yieldKey()`.
    pub fn yield_key(&self) -> String {
        let api_str = self.api.map(|a| format!("{:?}", a)).unwrap_or_default();
        let source_str = self.source.map(|s| format!("{}", s.0)).unwrap_or_default();
        format!("{}_{}", api_str, source_str)
    }

    /// Build a description from params.
    /// Mirrors Java's `SpellAbility.rebuiltDescription()`.
    pub fn rebuilt_description(&self) -> String {
        if !self.description.is_empty() {
            return self.description.clone();
        }
        if let Some(desc) = self.params.get("SpDesc") {
            return desc.to_string();
        }
        self.ability_text.clone()
    }

    /// Full text without suppression.
    /// Mirrors Java's `SpellAbility.toUnsuppressedString()`.
    pub fn to_unsuppressed_string(&self) -> String {
        self.rebuilt_description()
    }

    // ── Sub-abilities ─────────────────────────────────────────────────────

    /// Check if an additional ability with the given key exists.
    /// Mirrors Java's `SpellAbility.hasAdditionalAbility(String)`.
    pub fn has_additional_ability<K: TriggerKeyInput>(&self, key: K) -> bool {
        key.into_ability_key()
            .map(|parsed| self.trigger_spell_abilities.contains_key(&parsed))
            .unwrap_or(false)
    }

    /// Get an additional ability by key.
    /// Mirrors Java's `SpellAbility.getAdditionalAbility(String)`.
    pub fn get_additional_ability<K: TriggerKeyInput>(&self, key: K) -> Option<&SpellAbility> {
        key.into_ability_key()
            .and_then(|parsed| self.trigger_spell_abilities.get(&parsed))
    }

    /// Set an additional ability by key.
    /// Mirrors Java's `SpellAbility.setAdditionalAbility(String, SpellAbility)`.
    pub fn set_additional_ability<K: TriggerKeyInput>(&mut self, key: K, ability: SpellAbility) {
        if let Some(parsed) = key.into_ability_key() {
            self.trigger_spell_abilities.insert(parsed, ability);
        }
    }

    /// Append a sub-ability to the end of the chain.
    /// Mirrors Java's `SpellAbility.appendSubAbility(SpellAbility)`.
    pub fn append_sub_ability(&mut self, sub: SpellAbility) {
        if self.sub_ability.is_none() {
            self.sub_ability = Some(Box::new(sub));
        } else {
            // Walk to end of chain
            let mut current = self.sub_ability.as_deref_mut();
            while let Some(sa) = current {
                if sa.sub_ability.is_none() {
                    sa.sub_ability = Some(Box::new(sub));
                    return;
                }
                current = sa.sub_ability.as_deref_mut();
            }
        }
    }

    // ── Copying ───────────────────────────────────────────────────────────

    /// Clone this spell ability.
    /// Mirrors Java's `SpellAbility.copy()`.
    pub fn copy(&self) -> Self {
        crate::perf::increment(crate::perf::Metric::SpellAbilityClones, 1);
        self.clone()
    }

    pub fn copy_for_player(&self, activ: PlayerId) -> Self {
        crate::perf::increment(crate::perf::Metric::SpellAbilityClones, 1);
        let mut clone = self.clone();
        clone.activating_player = activ;
        clone
    }

    pub fn copy_with_host_lki(&self, host: crate::card::Card, lki: bool) -> Self {
        self.copy_with_host_activating_lki_keep_text_changes(
            host,
            self.activating_player,
            lki,
            false,
        )
    }

    pub fn copy_with_host_lki_keep_text_changes(
        &self,
        host: crate::card::Card,
        lki: bool,
        keep_text_changes: bool,
    ) -> Self {
        self.copy_with_host_activating_lki_keep_text_changes(
            host,
            self.activating_player,
            lki,
            keep_text_changes,
        )
    }

    pub fn copy_with_host_activating_lki(
        &self,
        host: crate::card::Card,
        activ: PlayerId,
        lki: bool,
    ) -> Self {
        self.copy_with_host_activating_lki_keep_text_changes(host, activ, lki, false)
    }

    pub fn copy_with_host_activating_lki_keep_text_changes(
        &self,
        host: crate::card::Card,
        activ: PlayerId,
        lki: bool,
        keep_text_changes: bool,
    ) -> Self {
        crate::perf::increment(crate::perf::Metric::SpellAbilityClones, 1);
        let mut clone = self.clone();
        clone.id = if lki {
            self.id
        } else {
            next_spell_ability_id()
        };

        clone.source = Some(host.id);
        clone.may_choose_new_targets = false;
        clone.trigger_objects = self.trigger_objects.clone();
        if !lki {
            clone.replacing_objects = HashMap::new();
        }

        clone.pay_costs = self.pay_costs.clone();
        if self.mana_part.is_some() {
            clone.mana_part = self.mana_part.clone();
        }

        clone.optional_keyword_amounts = self.optional_keyword_amounts.clone();
        clone.damage_map = self.damage_map.clone();
        clone.prevent_map = self.prevent_map.clone();
        clone.change_zone_table = self.change_zone_table.clone();
        clone.paying_mana = self.paying_mana.clone();
        clone.paid_abilities = Vec::new();
        clone.paid_hash = self.paid_hash.clone();

        if self.uses_targeting() {
            clone.target_chosen = self.target_chosen.clone();
        }

        clone.trigger_spell_abilities = HashMap::new();
        clone.additional_ability_lists = HashMap::new();

        if let Some(sub_ability) = &self.sub_ability {
            clone.sub_ability = Some(Box::new(
                sub_ability.copy_with_host_activating_lki_keep_text_changes(
                    host.clone(),
                    activ,
                    lki,
                    keep_text_changes,
                ),
            ));
        }

        for (name, ability) in &self.trigger_spell_abilities {
            clone.trigger_spell_abilities.insert(
                name.clone(),
                ability.copy_with_host_activating_lki_keep_text_changes(
                    host.clone(),
                    activ,
                    lki,
                    keep_text_changes,
                ),
            );
        }

        for (name, abilities) in &self.additional_ability_lists {
            clone.additional_ability_lists.insert(
                name.clone(),
                abilities
                    .iter()
                    .map(|ability| {
                        ability.copy_with_host_activating_lki_keep_text_changes(
                            host.clone(),
                            activ,
                            lki,
                            keep_text_changes,
                        )
                    })
                    .collect(),
            );
        }

        clone.restriction = self.restriction.clone();
        clone.condition = self.condition.clone();
        clone.activating_player = activ;

        let _ = keep_text_changes;
        clone
    }

    /// Clone with no mana cost.
    /// Mirrors Java's `SpellAbility.copyWithNoManaCost()`.
    pub fn copy_with_no_mana_cost(&self) -> Self {
        crate::perf::increment(crate::perf::Metric::SpellAbilityClones, 1);
        let mut copied = self.clone();
        copied.pay_costs = None;
        copied
    }

    /// Clone with a specific cost.
    /// Mirrors Java's `SpellAbility.copyWithDefinedCost(String)`.
    pub fn copy_with_defined_cost(&self, cost: &str) -> Self {
        crate::perf::increment(crate::perf::Metric::SpellAbilityClones, 1);
        let mut copied = self.clone();
        copied.pay_costs = Some(parse_cost(cost));
        copied
    }

    /// Clone with mana cost replacement.
    /// Mirrors Java's `SpellAbility.copyWithManaCostReplaced(String, String)`.
    pub fn copy_with_mana_cost_replaced(&self, old: &str, new: &str) -> Self {
        crate::perf::increment(crate::perf::Metric::SpellAbilityClones, 1);
        let mut copied = self.clone();
        if let Some(ref cost) = self.pay_costs {
            let cost_str = format!("{:?}", cost);
            let replaced = cost_str.replace(old, new);
            copied.pay_costs = Some(parse_cost(&replaced));
        }
        copied
    }

    // ── Targeting ─────────────────────────────────────────────────────────

    /// Check if this ability can target a specific card.
    /// Mirrors Java's `SpellAbility.canTarget(Card)`.
    pub fn can_target(&self, card: CardId, game: &GameState) -> bool {
        if let Some(ref tr) = self.target_restrictions {
            tr.has_candidates(game, self.activating_player, self.source)
                && self
                    .params
                    .get("TargetsWithDefinedController")
                    .map(|defined| {
                        crate::ability::ability_utils::resolve_defined_players_with_sa(
                            defined,
                            self,
                            self.activating_player,
                            game,
                        )
                    })
                    .map(|players| {
                        players.is_empty() || players.contains(&game.card(card).controller)
                    })
                    .unwrap_or(true)
                && target_restrictions::can_be_targeted_by_sa(
                    game,
                    card,
                    self.activating_player,
                    self,
                )
        } else {
            false
        }
    }

    /// Reset targets (alias for clear_targets).
    /// Mirrors Java's `SpellAbility.resetTargets()`.
    pub fn reset_targets(&mut self) {
        self.clear_targets();
    }

    /// Add divided allocation for a target.
    /// Mirrors Java's `SpellAbility.addDividedAllocation(Card, int)`.
    pub fn add_divided_allocation(&mut self, card: CardId, amount: i32) {
        self.target_chosen.add_divided_allocation(card, amount);
    }

    /// Reset only the first target in the chain.
    /// Mirrors Java's `SpellAbility.resetFirstTarget()`.
    pub fn reset_first_target(&mut self) {
        self.target_chosen = TargetChoices::default();
    }

    /// Check if more targets can be added.
    /// Mirrors Java's `SpellAbility.canAddMoreTarget()`.
    pub fn can_add_more_target(&self, game: &GameState) -> bool {
        if let Some(ref tr) = self.target_restrictions {
            let max = tr.get_max_targets(game, self);
            let current = self.target_chosen.all_target_cards().len() as i32
                + self.target_chosen.all_target_players().len() as i32;
            current < max
        } else {
            false
        }
    }

    /// Collect all targeted cards from the entire chain.
    /// Mirrors Java's `SpellAbility.findTargetedCards()`.
    pub fn find_targeted_cards(&self) -> Vec<CardId> {
        let mut cards = Vec::new();
        cards.extend(self.target_chosen.all_target_cards());
        let mut current = self.sub_ability.as_deref();
        while let Some(sub) = current {
            cards.extend(sub.target_chosen.all_target_cards());
            current = sub.sub_ability.as_deref();
        }
        cards
    }

    /// Whether this ability targets spells/abilities on the stack.
    /// Mirrors Java's `SpellAbility.canTargetSpellAbility()`.
    pub fn can_target_spell_ability(&self) -> bool {
        matches!(
            self.target_restrictions.as_ref().map(|tr| &tr.target_kind),
            Some(TargetKind::Spell)
        )
    }

    /// Setup new targets for a retargeting scenario.
    /// Mirrors Java's `SpellAbility.setupNewTargets()`.
    pub fn setup_new_targets(
        &mut self,
        game: &GameState,
        agents: &mut [Box<dyn PlayerAgent>],
        mana_pools: &[ManaPool],
    ) -> bool {
        self.clear_targets();
        self.setup_targets(game, agents, mana_pools)
    }

    // ── Convoke / Emerge / Offering ───────────────────────────────────────

    /// Clear pip reduction tracking.
    /// Mirrors Java's `SpellAbility.clearPipsToReduce()`.
    pub fn clear_pips_to_reduce(&mut self) {
        self.pips_to_reduce.clear();
    }

    /// Add a card tapped for convoke.
    /// Mirrors Java's `SpellAbility.addTappedForConvoke(Card)`.
    pub fn add_tapped_for_convoke(&mut self, card: CardId) {
        self.convoke_tapped.push(card);
    }

    /// Clear convoke tracking.
    /// Mirrors Java's `SpellAbility.clearTappedForConvoke()`.
    pub fn clear_tapped_for_convoke(&mut self) {
        self.convoke_tapped.clear();
    }

    /// Reset the sacrificed-as-emerge card.
    /// Mirrors Java's `SpellAbility.resetSacrificedAsEmerge()`.
    pub fn reset_sacrificed_as_emerge(&mut self) {
        self.sacrificed_as_emerge = None;
    }

    /// Reset the sacrificed-as-offering card.
    /// Mirrors Java's `SpellAbility.resetSacrificedAsOffering()`.
    pub fn reset_sacrificed_as_offering(&mut self) {
        self.sacrificed_as_offering = None;
    }

    // ── Splice ────────────────────────────────────────────────────────────

    /// Add spliced cards to this spell.
    /// Mirrors Java's `SpellAbility.addSplicedCards(List<Card>)`.
    pub fn add_spliced_cards(&mut self, cards: Vec<CardId>) {
        self.spliced_cards.extend(cards);
    }

    // ── Deterministic checks ──────────────────────────────────────────────

    /// Whether `Defined$` resolves to a deterministic set of objects.
    /// Mirrors Java's `SpellAbility.knownDetermineDefined()`.
    pub fn known_determine_defined(&self) -> bool {
        match self.params.get("Defined") {
            Some(defined) => matches!(
                defined,
                "Self"
                    | "You"
                    | "Targeted"
                    | "TargetedPlayer"
                    | "Remembered"
                    | "ParentTarget"
                    | "SourceController"
                    | "Imprinted"
            ),
            None => true,
        }
    }

    // ── Undo ──────────────────────────────────────────────────────────────

    /// Undo this ability.
    /// Mirrors Java's `SpellAbility.undo()`.
    pub fn undo(&mut self) -> bool {
        self.clear_tapped_for_convoke();
        self.reset_sacrificed_as_emerge();
        self.reset_sacrificed_as_offering();
        self.reset_paid_hash();
        self.clear_mana_paid();
        true
    }

    // ── Announce vars ─────────────────────────────────────────────────────

    /// Add an announced variable value.
    /// Mirrors Java's `SpellAbility.addAnnounceVar(String, int)`.
    pub fn add_announce_var(&mut self, key: &str, value: i32) {
        self.announce_vars.insert(key.to_string(), value);
    }

    // ── Targeting by SA ───────────────────────────────────────────────────

    /// Check if this spell ability can be targeted by another SA.
    /// Mirrors Java's `SpellAbility.canBeTargetedBy(SpellAbility)`.
    pub fn can_be_targeted_by(&self, _sa: &SpellAbility) -> bool {
        // Spells on the stack can generally be targeted unless they have
        // "can't be countered" or similar protection. The basic check is
        // whether this is a spell (on the stack).
        if self.is_spell {
            return !self.cant_be_copied();
        }
        true
    }

    // ── Property checks ───────────────────────────────────────────────────

    /// Check if this ability has a specific property.
    /// Mirrors Java's `SpellAbility.hasProperty(String)`.
    pub fn has_property(&self, property: &str) -> bool {
        // Check if the property matches a param key or ability characteristic
        if self.params.is_true(property) {
            return true;
        }
        if property.eq_ignore_ascii_case("Spell") && self.is_spell {
            return true;
        }
        if property.eq_ignore_ascii_case("Trigger") && self.is_trigger {
            return true;
        }
        if property.eq_ignore_ascii_case("Activated") && self.is_activated {
            return true;
        }
        if property.eq_ignore_ascii_case("ManaAbility") && self.is_mana_ability {
            return true;
        }
        false
    }

    /// Whether this ability tracks mana spent.
    /// Mirrors Java's `SpellAbility.tracksManaSpent()`.
    pub fn tracks_mana_spent(&self) -> bool {
        self.params.is_true("TrackManaSpent")
    }

    // ── Text changes ──────────────────────────────────────────────────────

    /// Apply text replacement.
    /// Mirrors Java's `SpellAbility.changeText(String, String)`.
    pub fn apply_text_change(&mut self, original: &str, replacement: &str) {
        if original == replacement {
            return;
        }
        self.description = self.description.replace(original, replacement);
        self.stack_description = self.stack_description.replace(original, replacement);
        if let Some(ref mut tr) = self.target_restrictions {
            tr.apply_target_text_changes(&[(original, replacement)]);
        }

        if let Some(sub_ability) = self.sub_ability.as_deref_mut() {
            sub_ability.apply_text_change(original, replacement);
        }

        for ability in self.trigger_spell_abilities.values_mut() {
            ability.apply_text_change(original, replacement);
        }
    }

    /// Apply intrinsic text replacement.
    /// Mirrors Java's `SpellAbility.changeTextIntrinsic(String, String)`.
    pub fn apply_text_change_intrinsic(&mut self, original: &str, replacement: &str) {
        self.apply_text_change(original, replacement);
    }

    /// Apply a batch of text replacements to this ability and linked abilities.
    pub fn apply_text_changes(&mut self, pairs: &[(String, String)]) {
        for (original, replacement) in pairs {
            self.apply_text_change(original, replacement);
        }
    }

    /// Apply intrinsic text changes to this ability and linked abilities.
    pub fn apply_text_changes_intrinsic(
        &mut self,
        color_map: &HashMap<String, String>,
        type_map: &HashMap<String, String>,
    ) {
        for (original, replacement) in color_map.iter().chain(type_map.iter()) {
            self.apply_text_change_intrinsic(original, replacement);
        }
    }

    /// Java parity hook for `SpellAbility.setHostCard(Card)`.
    pub fn set_host_card(&mut self, card: crate::card::Card) {
        self.source = Some(card.id);
        if self.original_host.is_none() {
            self.original_host = Some(card.id);
        }

        if let Some(sub_ability) = self.sub_ability.as_deref_mut() {
            sub_ability.set_host_card(card.clone());
        }

        for ability in self.trigger_spell_abilities.values_mut() {
            ability.set_host_card(card.clone());
        }
    }

    /// Java parity hook for `SpellAbility.setKeyword(KeywordInterface)`.
    pub fn set_keyword(&mut self, keyword: crate::keyword::keyword_interface::KeywordInterface) {
        if let Some(sub_ability) = self.sub_ability.as_deref_mut() {
            sub_ability.set_keyword(keyword.clone());
        }

        for ability in self.trigger_spell_abilities.values_mut() {
            ability.set_keyword(keyword.clone());
        }
    }

    /// Java parity hook for `SpellAbility.setCardState(CardState)`.
    pub fn set_card_state(&mut self, state: crate::card::card_state::CardState) {
        if let Some(sub_ability) = self.sub_ability.as_deref_mut() {
            sub_ability.set_card_state(state.clone());
        }

        for ability in self.trigger_spell_abilities.values_mut() {
            ability.set_card_state(state.clone());
        }
    }

    /// Java parity hook for `SpellAbility.setIntrinsic(boolean)`.
    pub fn set_intrinsic(&mut self, intrinsic: bool) {
        self.intrinsic = intrinsic;

        if let Some(sub_ability) = self.sub_ability.as_deref_mut() {
            if sub_ability.is_intrinsic() != intrinsic {
                sub_ability.set_intrinsic(intrinsic);
            }
        }

        for ability in self.trigger_spell_abilities.values_mut() {
            if ability.is_intrinsic() != intrinsic {
                ability.set_intrinsic(intrinsic);
            }
        }
    }

    pub fn is_intrinsic(&self) -> bool {
        self.intrinsic
    }

    /// Mirrors Java's `SpellAbility.getAmountLifePaid()`.
    pub fn get_amount_life_paid(&self) -> i32 {
        self.paid_life_amount
    }

    /// Mirrors Java's `SpellAbility.setAmountLifePaid(int)`.
    pub fn set_amount_life_paid(&mut self, value: i32) {
        self.paid_life_amount = value;
    }

    // ── AI scoring ────────────────────────────────────────────────────────

    /// Calculate an AI score for this mana ability.
    /// Mirrors Java's `SpellAbility.calculateScoreForManaAbility()`.
    pub fn calculate_score_for_mana_ability(&self) -> i32 {
        if !self.is_mana_ability {
            return 0;
        }
        let base = self.total_amount_of_mana_generated();
        // Prefer abilities that produce more mana and have fewer restrictions
        let restriction_penalty = if self.restriction.variables.sorcery_speed() {
            -1
        } else {
            0
        };
        base + restriction_penalty
    }

    // ── Timing checks ─────────────────────────────────────────────────────

    /// Check if this ability can be cast at the current timing.
    /// Mirrors Java's `SpellAbility.canCastTiming(Game)`.
    pub fn can_cast_timing(&self, game: &GameState) -> bool {
        let can_cast_sorcery = game.turn.phase.is_main()
            && game.stack.is_empty()
            && game.turn.active_player == self.activating_player;

        // Non-spell, non-activated abilities do not have default timing checks here.
        if !self.is_spell && !self.is_activated {
            return true;
        }

        if can_cast_sorcery || self.with_flash(game) {
            return true;
        }

        // Spells are sorcery-speed by default unless an explicit timing permission applies.
        if self.is_spell {
            return false;
        }

        // Activated abilities are instant-speed by default except for explicit
        // sorcery-speed restrictions and planeswalker abilities.
        if self.is_activated {
            return !self.params.is_true("PwAbility")
                && !self.restriction.variables.sorcery_speed();
        }

        true
    }

    /// Check if this spell has flash.
    /// Mirrors Java's `SpellAbility.withFlash(Game)`.
    pub fn with_flash(&self, game: &GameState) -> bool {
        if self.restriction.variables.instant_speed() {
            return true;
        }
        if self.params.is_true("Flash") {
            return true;
        }
        if let Some(card_id) = self.source {
            let card = game.card(card_id);
            if ((self.is_spell || self.is_land_ability) && card.type_line.is_instant())
                || card.has_keyword("Flash")
            {
                return true;
            }
            return crate::staticability::static_ability_cast_with_flash::any_with_flash(
                &game.cards,
                card,
                self.activating_player,
                &card.abilities,
            );
        }
        false
    }

    /// Check restrictions for this ability.
    /// Mirrors Java's `SpellAbility.checkRestrictions(Game)`.
    pub fn check_restrictions(&self, game: &GameState) -> bool {
        self.can_play(game)
    }

    // ── Rollback ──────────────────────────────────────────────────────────

    /// Add a rollback effect.
    /// Mirrors Java's `SpellAbility.addRollbackEffect(String)`.
    pub fn add_rollback_effect(&mut self, effect: String) {
        self.rollback_effects.push(effect);
    }

    /// Rollback all tracked effects.
    /// Mirrors Java's `SpellAbility.rollback()`.
    pub fn rollback(&mut self) -> bool {
        let had_effects = !self.rollback_effects.is_empty();
        self.rollback_effects.clear();
        had_effects
    }

    // ── Optional keyword amounts ──────────────────────────────────────────

    /// Check if this ability has an optional keyword with a specific amount.
    /// Mirrors Java's `SpellAbility.hasOptionalKeywordAmount(String)`.
    pub fn has_optional_keyword_amount(&self, keyword: &str) -> bool {
        self.optional_keyword_amounts.contains_key(keyword)
    }

    /// Clear all optional keyword amounts.
    /// Mirrors Java's `SpellAbility.clearOptionalKeywordAmount()`.
    pub fn clear_optional_keyword_amount(&mut self) {
        self.optional_keyword_amounts.clear();
    }

    /// Clear last known state tracking.
    /// Mirrors Java's `SpellAbility.clearLastState()`.
    pub fn clear_last_state(&mut self) {
        self.last_state.clear();
    }

    // ── Trigger object management ─────────────────────────────────────────

    /// Set a triggering object in the map.
    /// Mirrors Java's `SpellAbility.setTriggeringObject(AbilityKey, Object)`.
    pub fn set_triggering_object<K: TriggerKeyInput, V: Into<AbilityValue>>(
        &mut self,
        key: K,
        value: V,
    ) {
        if let Some(parsed) = key.into_ability_key() {
            self.trigger_objects.insert(parsed, value.into());
        }
    }

    /// Typed trigger value setter.
    pub fn set_triggering_value<V: Into<AbilityValue>>(&mut self, key: AbilityKey, value: V) {
        self.trigger_objects.insert(key, value.into());
    }

    /// Set a triggering spell ability in the map.
    /// Mirrors Java's `SpellAbility.setTriggeringObject(AbilityKey, Object)` for SpellAbility values.
    pub fn set_triggering_spell_ability<K: TriggerKeyInput>(
        &mut self,
        key: K,
        value: SpellAbility,
    ) {
        if let Some(parsed) = key.into_ability_key() {
            self.trigger_spell_abilities.insert(parsed, value);
        }
    }

    /// Get a triggering spell ability from the map.
    pub fn get_triggering_spell_ability<K: TriggerKeyInput>(
        &self,
        key: K,
    ) -> Option<&SpellAbility> {
        key.into_ability_key()
            .and_then(|parsed| self.trigger_spell_abilities.get(&parsed))
    }

    /// Update an existing triggering object.
    /// Mirrors Java's `SpellAbility.updateTriggeringObject(String, Object)`.
    pub fn update_triggering_object<K: TriggerKeyInput, V: Into<AbilityValue>>(
        &mut self,
        key: K,
        value: V,
    ) {
        self.set_triggering_object(key, value);
    }

    // ── Target management ─────────────────────────────────────────────────

    /// Update a target in the chosen targets.
    /// Mirrors Java's `SpellAbility.updateTarget(Card, Card)`.
    pub fn update_target(&mut self, old: CardId, new: CardId) {
        self.target_chosen.replace_target_card(old, new);
    }

    /// Whether this targets a single target only.
    /// Mirrors Java's `SpellAbility.targetsSingleTarget()`.
    pub fn targets_single_target(&self) -> bool {
        if let Some(ref tr) = self.target_restrictions {
            tr.max_targets == "1"
        } else {
            false
        }
    }

    // ── Variable operand getters/setters ──────────────────────────────────
    // These mirror Java's SpellAbilityVariables Operand/ToCheck/Operator accessors.
    // In Rust, they are stored in the SpellAbilityVariables but accessed via SA.

    /// Get variable operand 1.
    /// Mirrors Java's `SpellAbility.getSVar("Operand")`.
    pub fn gets_var_operand(&self) -> Option<&str> {
        self.params.get("Operand")
    }

    /// Get variable operand 2.
    /// Mirrors Java's `SpellAbility.getSVar("Operand2")`.
    pub fn gets_var_operand2(&self) -> Option<&str> {
        self.params.get("Operand2")
    }

    /// Set variable operand 1.
    /// Mirrors Java's `SpellAbility.setSVar("Operand", val)`.
    pub fn sets_var_operand(&mut self, value: &str) {
        self.params.put("Operand".to_string(), value.to_string());
    }

    /// Set variable operand 2.
    /// Mirrors Java's `SpellAbility.setSVar("Operand2", val)`.
    pub fn sets_var_operand2(&mut self, value: &str) {
        self.params.put("Operand2".to_string(), value.to_string());
    }

    /// Get variable to check 1.
    /// Mirrors Java's `SpellAbility.getSVar("VarToCheck")`.
    pub fn gets_var_to_check(&self) -> Option<&str> {
        self.params.get("VarToCheck")
    }

    /// Get variable to check 2.
    /// Mirrors Java's `SpellAbility.getSVar("VarToCheck2")`.
    pub fn gets_var_to_check2(&self) -> Option<&str> {
        self.params.get("VarToCheck2")
    }

    /// Set variable to check 1.
    /// Mirrors Java's `SpellAbility.setSVar("VarToCheck", val)`.
    pub fn sets_var_to_check(&mut self, value: &str) {
        self.params.put("VarToCheck".to_string(), value.to_string());
    }

    /// Set variable to check 2.
    /// Mirrors Java's `SpellAbility.setSVar("VarToCheck2", val)`.
    pub fn sets_var_to_check2(&mut self, value: &str) {
        self.params
            .put("VarToCheck2".to_string(), value.to_string());
    }

    /// Get variable operator 1.
    /// Mirrors Java's `SpellAbility.getSVar("Operator")`.
    pub fn gets_var_operator(&self) -> Option<&str> {
        self.params.get("Operator")
    }

    /// Get variable operator 2.
    /// Mirrors Java's `SpellAbility.getSVar("Operator2")`.
    pub fn gets_var_operator2(&self) -> Option<&str> {
        self.params.get("Operator2")
    }

    /// Set variable operator 1.
    /// Mirrors Java's `SpellAbility.setSVar("Operator", val)`.
    pub fn sets_var_operator(&mut self, value: &str) {
        self.params.put("Operator".to_string(), value.to_string());
    }

    /// Set variable operator 2.
    /// Mirrors Java's `SpellAbility.setSVar("Operator2", val)`.
    pub fn sets_var_operator2(&mut self, value: &str) {
        self.params.put("Operator2".to_string(), value.to_string());
    }
}

// build_spell_ability now lives in ability::ability_factory.
// Re-export here for backward compatibility.
pub use crate::ability::ability_factory::build_spell_ability;
pub use crate::ability::ability_factory::build_spell_ability_for_card_cast;
pub use crate::ability::ability_factory::build_spell_ability_from_host_card;

/// Check whether any spell on the stack has split second.
/// Split second prevents players from casting spells or activating abilities
/// (except mana abilities) while it's on the stack.
/// Single source of truth — used by spell, ability, and ability_activated modules.
pub fn has_split_second_on_stack(game: &GameState) -> bool {
    for entry in game.stack.iter() {
        if entry.spell_ability.params.is_true("SplitSecond") {
            return true;
        }
        if let Some(card_id) = entry.spell_ability.source {
            let card = game.card(card_id);
            if card.has_keyword("Split second") {
                return true;
            }
        }
    }
    false
}

pub fn choose_targets_by_kind(
    agent: &mut dyn PlayerAgent,
    sa: &mut SpellAbility,
    game: &GameState,
    mana_pools: &[ManaPool],
) -> bool {
    use crate::card::card_util;

    let tr = match &sa.target_restrictions {
        Some(tr) => tr,
        None => return true,
    };

    let player = sa.targeting_player.unwrap_or(sa.activating_player);

    let min_targets = tr.get_min_targets(game, sa);
    let max_targets = tr.get_max_targets(game, sa);
    if max_targets <= 0 {
        return true;
    }

    if !tr.has_candidates(game, player, sa.source) {
        return min_targets <= 0;
    }

    sa.target_chosen.target_card = None;
    sa.target_chosen.target_card_zone_timestamp = None;
    sa.target_chosen.divided_map.clear();

    match &tr.target_kind {
        TargetKind::None => {}
        TargetKind::Player => {
            agent.snapshot_state(game, mana_pools);
            let is_opponent_only = tr
                .valid_tgts
                .iter()
                .any(|v| v.eq_ignore_ascii_case("Opponent"));
            let valid_players: Vec<PlayerId> = game
                .alive_players()
                .into_iter()
                .filter(|&pid| !is_opponent_only || pid != player)
                .collect();
            if max_targets > 1 {
                let mut chosen = Vec::new();
                while (chosen.len() as i32) < max_targets {
                    let Some(pid) = agent.choose_target_player(player, &valid_players, Some(&*sa))
                    else {
                        break;
                    };
                    if !chosen.contains(&pid) {
                        chosen.push(pid);
                    }
                    if chosen.len() == valid_players.len() {
                        break;
                    }
                }
                sa.target_chosen.target_player = chosen.first().copied();
                sa.target_chosen.additional_target_players = chosen.into_iter().skip(1).collect();
            } else {
                sa.target_chosen.target_player =
                    agent.choose_target_player(player, &valid_players, Some(&*sa));
            }
        }
        TargetKind::Any => {
            let valid_players: Vec<PlayerId> =
                if target_restrictions::any_target_allows_players(&tr.valid_tgts) {
                    game.alive_players().into_iter().collect()
                } else {
                    Vec::new()
                };
            let valid_cards: Vec<CardId> = card_util::get_valid_cards_to_target(game, sa);
            agent.snapshot_state(game, mana_pools);
            match agent.choose_target_any(player, &valid_players, &valid_cards, Some(&*sa)) {
                crate::agent::TargetChoice::Player(pid) => {
                    sa.target_chosen.target_player = Some(pid)
                }
                crate::agent::TargetChoice::Card(cid) => {
                    sa.target_chosen.target_card = Some(cid);
                    sa.target_chosen.target_card_zone_timestamp =
                        Some(game.card(cid).zone_timestamp);
                }
                crate::agent::TargetChoice::None => {}
            }
        }
        TargetKind::Creature(_) => {
            let valid: Vec<CardId> = card_util::get_valid_cards_to_target(game, sa)
                .into_iter()
                .filter(|&cid| target_allowed_by_defined_controller(game, sa, cid))
                .collect();
            agent.snapshot_state(game, mana_pools);
            if max_targets > 1 {
                let chosen = agent.choose_cards_for_effect(
                    player,
                    &valid,
                    min_targets.max(0) as usize,
                    max_targets as usize,
                );
                if let Some(&first) = chosen.first() {
                    sa.target_chosen.target_card = Some(first);
                    sa.target_chosen.target_card_zone_timestamp =
                        Some(game.card(first).zone_timestamp);
                    for &extra in chosen.iter().skip(1) {
                        sa.target_chosen.divided_map.insert(extra, 0);
                    }
                }
            } else {
                sa.target_chosen.target_card = agent.choose_target_card(player, &valid, Some(&*sa));
                if let Some(cid) = sa.target_chosen.target_card {
                    sa.target_chosen.target_card_zone_timestamp =
                        Some(game.card(cid).zone_timestamp);
                }
            }
        }
        TargetKind::Permanent(_) => {
            let valid: Vec<CardId> = card_util::get_valid_cards_to_target(game, sa)
                .into_iter()
                .filter(|&cid| target_allowed_by_defined_controller(game, sa, cid))
                .collect();
            agent.snapshot_state(game, mana_pools);
            if max_targets > 1 {
                let chosen = agent.choose_cards_for_effect(
                    player,
                    &valid,
                    min_targets.max(0) as usize,
                    max_targets as usize,
                );
                if let Some(&first) = chosen.first() {
                    sa.target_chosen.target_card = Some(first);
                    sa.target_chosen.target_card_zone_timestamp =
                        Some(game.card(first).zone_timestamp);
                    for &extra in chosen.iter().skip(1) {
                        sa.target_chosen.divided_map.insert(extra, 0);
                    }
                }
            } else {
                sa.target_chosen.target_card = agent.choose_target_card(player, &valid, Some(&*sa));
                if let Some(cid) = sa.target_chosen.target_card {
                    sa.target_chosen.target_card_zone_timestamp =
                        Some(game.card(cid).zone_timestamp);
                }
            }
        }
        TargetKind::CardInZone { zone, .. } => {
            let valid: Vec<CardId> = card_util::get_valid_cards_to_target(game, sa)
                .into_iter()
                .filter(|&cid| target_allowed_by_defined_controller(game, sa, cid))
                .collect();
            agent.snapshot_state(game, mana_pools);
            if max_targets > 1 {
                let chosen = agent.choose_cards_for_effect(
                    player,
                    &valid,
                    min_targets.max(0) as usize,
                    max_targets as usize,
                );
                if let Some(&first) = chosen.first() {
                    sa.target_chosen.target_card = Some(first);
                    sa.target_chosen.target_card_zone_timestamp =
                        Some(game.card(first).zone_timestamp);
                    for &extra in chosen.iter().skip(1) {
                        sa.target_chosen.divided_map.insert(extra, 0);
                    }
                }
            } else {
                sa.target_chosen.target_card =
                    agent.choose_target_card_from_zone(player, *zone, &valid, Some(&*sa));
                if let Some(cid) = sa.target_chosen.target_card {
                    sa.target_chosen.target_card_zone_timestamp =
                        Some(game.card(cid).zone_timestamp);
                }
            }
        }
        TargetKind::Spell => {
            let valid = target_restrictions::get_all_candidates_spells(game);
            let valid = if let Some(ref restrictions) = sa.target_restrictions {
                target_restrictions::filter_spells_for_target_restrictions(
                    game,
                    &valid,
                    restrictions,
                )
            } else {
                valid
            };
            agent.snapshot_state(game, mana_pools);
            sa.target_chosen.target_stack_entry = agent.choose_target_spell(player, &valid);
        }
    }

    true
}

fn target_allowed_by_defined_controller(
    game: &GameState,
    sa: &SpellAbility,
    card_id: CardId,
) -> bool {
    let Some(defined) = sa.params.get("TargetsWithDefinedController") else {
        return true;
    };
    let players = crate::ability::ability_utils::resolve_defined_players_with_sa(
        defined,
        sa,
        sa.activating_player,
        game,
    );
    players.is_empty() || players.contains(&game.card(card_id).controller)
}

fn choose_targeting_player(
    sa: &SpellAbility,
    game: &GameState,
    agents: &mut [Box<dyn PlayerAgent>],
) -> Option<PlayerId> {
    if let Some(defined) = sa.params.get(keys::TARGETING_PLAYER) {
        let candidates = crate::ability::ability_utils::resolve_defined_players_with_sa(
            defined,
            sa,
            sa.activating_player,
            game,
        );
        if candidates.is_empty() {
            return None;
        }
        return agents[sa.activating_player.index()].choose_target_player(
            sa.activating_player,
            &candidates,
            None,
        );
    }
    Some(sa.activating_player)
}

// Re-export MagicStack and StackEntry from zone module (their canonical home,
// matching Java's `forge.game.zone.MagicStack`).
pub use crate::zone::magic_stack::{MagicStack, StackEntry};
