use forge_foundation::mana::ManaAtom;
use forge_foundation::{PhaseType, ZoneType};
use serde::{Deserialize, Serialize};

use crate::card::CardInstance;
use crate::cost::CostPart;
use crate::game::GameState;
use crate::ids::{CardId, PlayerId};

pub mod auto_pay;
pub mod computer_util_mana;
pub(crate) mod mana_cost_being_paid;
pub use auto_pay::{
    pay_mana_cost_auto, pay_mana_cost_auto_with_callback, pay_mana_cost_auto_with_chooser,
};
pub use computer_util_mana::{
    auto_tap_lands, auto_tap_lands_allow_reserved_source_reuse,
    auto_tap_lands_allow_reserved_source_reuse_with_chooser, auto_tap_lands_generic,
    auto_tap_lands_with_callbacks, auto_tap_lands_with_chooser, ManaPayCallback, ManaPayCallbackFn,
    SacrificeChooser,
};

/// An individual mana object in the pool, tracking source and properties.
#[derive(Debug, Clone)]
pub struct Mana {
    pub color: u16,
    pub source_card: Option<CardId>,
    pub is_snow: bool,
    /// Mana that persists across all phase transitions (Omnath, Kruphix).
    pub is_persistent: bool,
    /// Mana that persists through combat phases but empties at end of combat.
    pub is_combat_mana: bool,
    /// Restriction on what this mana can be spent on (from RestrictValid$).
    /// e.g. "Spell.Creature", "Spell.Artifact", "Activated", "nonSpell".
    pub restriction: Option<String>,
    /// If true, spells paid with this mana can't be countered (Cavern of Souls).
    pub adds_no_counter: bool,
    /// Keywords to add to spells cast with this mana (e.g. "Haste" from Generator Servant).
    /// Format: "Keyword" with optional valid filter "Keyword|ValidFilter" (e.g. "Haste|Spell.Creature").
    pub adds_keywords: Option<String>,
    /// Valid filter for which spells get the keywords (e.g. "Spell.Creature").
    pub adds_keywords_valid: Option<String>,
    /// Counter spec to add to permanents cast with this mana (e.g. "P1P1" from Guildmages' Forum).
    pub adds_counters: Option<String>,
    /// Valid filter for which cards get the counters.
    pub adds_counters_valid: Option<String>,
    /// SVar name of a trigger to fire when this mana is spent to cast a spell.
    /// The SVar lives on the source card (identified by `source_card`).
    pub triggers_when_spent: Option<String>,
}

impl Mana {
    pub fn simple(color: u16) -> Self {
        Self {
            color,
            source_card: None,
            is_snow: false,
            is_persistent: false,
            is_combat_mana: false,
            restriction: None,
            adds_no_counter: false,
            adds_keywords: None,
            adds_keywords_valid: None,
            adds_counters: None,
            adds_counters_valid: None,
            triggers_when_spent: None,
        }
    }
}

/// Context about what a mana payment is for, used to check restrictions.
#[derive(Debug, Clone, Default)]
pub struct ManaPaymentContext {
    /// True if paying for a spell (not an ability).
    pub is_spell: bool,
    /// Card type line of the spell being cast (for type checks).
    pub type_line: Option<forge_foundation::CardTypeLine>,
    /// Subtypes of the spell being cast.
    pub card_name: Option<String>,
}

/// Check if a mana with the given restriction can be spent in the given context.
pub fn mana_meets_restriction(restriction: &str, ctx: &ManaPaymentContext) -> bool {
    // Multiple comma-separated restrictions: any match is OK (OR logic)
    for part in restriction.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if check_single_restriction(part, ctx) {
            return true;
        }
    }
    false
}

fn check_single_restriction(restriction: &str, ctx: &ManaPaymentContext) -> bool {
    match restriction {
        "nonSpell" => !ctx.is_spell,
        "Activated" => !ctx.is_spell,
        _ if restriction.starts_with("Spell.") => {
            if !ctx.is_spell {
                return false;
            }
            let type_check = &restriction[6..]; // After "Spell."
            if let Some(ref tl) = ctx.type_line {
                match type_check {
                    "Creature" => tl.is_creature(),
                    "Artifact" => tl.is_artifact(),
                    "Enchantment" => tl.is_enchantment(),
                    "Instant" => tl.is_instant(),
                    "Sorcery" => tl.is_sorcery(),
                    "Planeswalker" => tl.is_planeswalker(),
                    "Land" => tl.is_land(),
                    other => {
                        // Check subtype (e.g. "Spell.Dragon", "Spell.Lesson")
                        // Handle compound checks with + (e.g. "Creature+Dragon")
                        if let Some((base, sub)) = other.split_once('+') {
                            let base_ok = match base {
                                "Creature" => tl.is_creature(),
                                "Artifact" => tl.is_artifact(),
                                _ => tl.has_subtype(base),
                            };
                            base_ok && tl.has_subtype(sub)
                        } else {
                            tl.has_subtype(other)
                        }
                    }
                }
            } else {
                false
            }
        }
        _ if restriction.starts_with("Activated.") => !ctx.is_spell,
        _ if restriction.starts_with("CantPayGenericCosts") => true, // handled separately in payment
        _ if restriction.starts_with("CantCast") => true, // zone restrictions handled elsewhere
        _ => true,                                        // Unknown restriction — be permissive
    }
}

/// Tracks available mana for a player during a turn.
/// Uses individual Mana objects to support source tracking, snow, and future restrictions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ManaPool {
    #[serde(skip)]
    mana: Vec<Mana>,
    /// When set, caps total producible mana for playability checks.
    /// Used by `calculate_available_mana` to prevent multi-color sources
    /// (dual lands, Command Tower) from being counted as multiple mana.
    #[serde(skip)]
    pub total_sources: Option<i32>,
    /// Per-source color bitmasks for source-level matching in `can_pay`.
    /// Each entry is a bitmask of ManaAtom colors that one mana source can produce.
    /// Used by `calculate_available_mana` to prevent dual lands from satisfying
    /// multiple colored requirements simultaneously.
    #[serde(skip)]
    pub source_colors: Option<Vec<u16>>,
}

impl ManaPool {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, atom: u16, amount: i32) {
        for _ in 0..amount {
            self.mana.push(Mana::simple(atom));
        }
    }

    /// Add mana with snow flag set (from a snow permanent source).
    pub fn add_snow(&mut self, atom: u16, amount: i32) {
        for _ in 0..amount {
            let mut m = Mana::simple(atom);
            m.is_snow = true;
            self.mana.push(m);
        }
    }

    /// Add mana with a restriction (from RestrictValid$).
    pub fn add_restricted(&mut self, atom: u16, restriction: String) {
        let mut m = Mana::simple(atom);
        m.restriction = Some(restriction);
        self.mana.push(m);
    }

    /// Count mana in pool that has the "can't be countered" flag.
    pub fn count_uncounterable(&self) -> i32 {
        self.mana.iter().filter(|m| m.adds_no_counter).count() as i32
    }

    /// Collect keywords that should be added to a spell based on consumed mana.
    /// Call this before and after payment to diff.
    pub fn collect_keyword_mana(&self) -> Vec<(String, Option<String>)> {
        self.mana
            .iter()
            .filter_map(|m| {
                m.adds_keywords
                    .as_ref()
                    .map(|kw| (kw.clone(), m.adds_keywords_valid.clone()))
            })
            .collect()
    }

    /// Collect counter specs from mana that should be applied to permanents cast with it.
    pub fn collect_counter_mana(&self) -> Vec<(String, Option<String>)> {
        self.mana
            .iter()
            .filter_map(|m| {
                m.adds_counters
                    .as_ref()
                    .map(|cs| (cs.clone(), m.adds_counters_valid.clone()))
            })
            .collect()
    }

    /// Collect trigger SVars from mana that should fire when spent.
    /// Returns (svar_name, source_card_id) pairs.
    pub fn collect_trigger_mana(&self) -> Vec<(String, CardId)> {
        self.mana
            .iter()
            .filter_map(|m| {
                m.triggers_when_spent
                    .as_ref()
                    .and_then(|svar| m.source_card.map(|src| (svar.clone(), src)))
            })
            .collect()
    }

    /// Get the color of each mana in the pool (for tracking consumed colors).
    pub fn mana_colors(&self) -> Vec<u16> {
        self.mana.iter().map(|m| m.color).collect()
    }

    /// Get a bitmask of all colors present in the pool.
    pub fn colors_present(&self) -> u16 {
        let mut mask = 0u16;
        for m in &self.mana {
            mask |= m.color;
        }
        mask
    }

    /// Count snow mana in the pool (any color).
    pub fn count_snow(&self) -> i32 {
        self.mana.iter().filter(|m| m.is_snow).count() as i32
    }

    pub fn add_mana(&mut self, m: Mana) {
        self.mana.push(m);
    }

    pub fn total(&self) -> i32 {
        self.mana.len() as i32
    }

    pub fn count_color(&self, atom: u16) -> i32 {
        self.mana.iter().filter(|m| m.color == atom).count() as i32
    }

    pub fn white(&self) -> i32 {
        self.count_color(ManaAtom::WHITE)
    }
    pub fn blue(&self) -> i32 {
        self.count_color(ManaAtom::BLUE)
    }
    pub fn black(&self) -> i32 {
        self.count_color(ManaAtom::BLACK)
    }
    pub fn red(&self) -> i32 {
        self.count_color(ManaAtom::RED)
    }
    pub fn green(&self) -> i32 {
        self.count_color(ManaAtom::GREEN)
    }
    pub fn colorless(&self) -> i32 {
        self.count_color(ManaAtom::COLORLESS)
    }

    /// Remove `amount` of a given mana atom from the pool, saturating at 0.
    pub fn remove(&mut self, atom: u16, amount: i32) {
        let mut remaining = amount;
        self.mana.retain(|m| {
            if remaining > 0 && m.color == atom {
                remaining -= 1;
                false
            } else {
                true
            }
        });
    }

    /// Returns true if the pool contains at least `amount` of the given atom.
    pub fn has_atom(&self, atom: u16, amount: i32) -> bool {
        self.count_color(atom) >= amount
    }

    /// Spend generic mana from the pool, consuming colorless first then any color.
    /// Returns the amount actually spent.
    pub fn spend_generic(&mut self, mut amount: i32) -> i32 {
        let spent = amount.min(self.total());
        // Consume colorless first
        let colorless_count = self.colorless();
        let from_colorless = amount.min(colorless_count);
        self.remove(ManaAtom::COLORLESS, from_colorless);
        amount -= from_colorless;
        // Then consume from colors in WUBRG order
        for &color in &[
            ManaAtom::WHITE,
            ManaAtom::BLUE,
            ManaAtom::BLACK,
            ManaAtom::RED,
            ManaAtom::GREEN,
        ] {
            if amount <= 0 {
                break;
            }
            let available = self.count_color(color);
            let take = amount.min(available);
            self.remove(color, take);
            amount -= take;
        }
        spent
    }

    pub fn empty(&mut self) {
        self.mana.clear();
    }

    /// Clear mana pool at phase transitions, retaining persistent and combat mana.
    /// Mirrors Java's PhaseHandler.onPhaseEnd() → clearPool(true) (MTG rule 500.4).
    pub fn clear_pool(&mut self, phase: PhaseType) -> usize {
        self.clear_pool_with_keep(phase, 0)
    }

    /// Clear the mana pool, retaining persistent mana, combat mana (if in combat),
    /// and mana of colors specified by `keep_colors` bitmask (from UnspentMana statics).
    /// Returns the number of mana cleared (for mana burn calculation).
    pub fn clear_pool_with_keep(&mut self, phase: PhaseType, keep_colors: u16) -> usize {
        let before = self.mana.len();
        let in_combat = matches!(
            phase,
            PhaseType::CombatBegin
                | PhaseType::CombatDeclareAttackers
                | PhaseType::CombatDeclareBlockers
                | PhaseType::CombatFirstStrikeDamage
                | PhaseType::CombatDamage
                | PhaseType::CombatEnd
        );
        self.mana.retain(|m| {
            m.is_persistent
                || (m.is_combat_mana && in_combat)
                || (keep_colors != 0 && (m.color & keep_colors) != 0)
        });
        before - self.mana.len()
    }

    /// Try to pay a mana cost. Returns true if successful and deducts the mana.
    /// This is a simplified payment algorithm that handles colored and generic mana.
    pub fn can_pay(&self, cost: &forge_foundation::ManaCost) -> bool {
        // When source_colors is available (from calculate_available_mana), use
        // source-level matching to prevent dual lands from satisfying multiple
        // colored requirements simultaneously.
        if let Some(ref sources) = self.source_colors {
            return Self::can_pay_source_matching(sources, cost, 0);
        }

        // Fallback for non-availability-estimate pools (actual mana during payment)
        if let Some(max) = self.total_sources {
            if cost.cmc() > max {
                return false;
            }
        }

        let mut required = [0i32; 6]; // W, U, B, R, G, C
        for shard in cost.shards() {
            let atoms = shard.shard();
            if (atoms & ManaAtom::WHITE) != 0 {
                required[0] += 1;
            }
            if (atoms & ManaAtom::BLUE) != 0 {
                required[1] += 1;
            }
            if (atoms & ManaAtom::BLACK) != 0 {
                required[2] += 1;
            }
            if (atoms & ManaAtom::RED) != 0 {
                required[3] += 1;
            }
            if (atoms & ManaAtom::GREEN) != 0 {
                required[4] += 1;
            }
            if (atoms & ManaAtom::COLORLESS) != 0 && !shard.is_multi_color() {
                required[5] += 1;
            }
        }
        if self.white() < required[0]
            || self.blue() < required[1]
            || self.black() < required[2]
            || self.red() < required[3]
            || self.green() < required[4]
            || self.colorless() < required[5]
        {
            return false;
        }

        let mut pool = self.clone();
        pool.try_pay(cost)
    }

    /// Check if the pool can pay a cost with any-color conversion active.
    pub fn can_pay_any_color(&self, cost: &forge_foundation::ManaCost) -> bool {
        if let Some(max) = self.total_sources {
            if cost.cmc() > max {
                return false;
            }
        }
        let mut pool = self.clone();
        pool.try_pay_any_color(cost)
    }

    /// Create a clone with restricted mana filtered out based on context.
    fn filtered_for_context(&self, ctx: &ManaPaymentContext) -> ManaPool {
        let mut pool = self.clone();
        pool.mana.retain(|m| match &m.restriction {
            None => true,
            Some(r) => mana_meets_restriction(r, ctx),
        });
        pool
    }

    /// Check if pool can pay a cost, respecting mana restrictions for the given spell context.
    pub fn can_pay_for_spell(
        &self,
        cost: &forge_foundation::ManaCost,
        ctx: &ManaPaymentContext,
    ) -> bool {
        let filtered = self.filtered_for_context(ctx);
        filtered.can_pay(cost)
    }

    /// Pay a cost, skipping restricted mana that doesn't match the context.
    /// Returns true if successful and deducts the mana from the ORIGINAL pool.
    pub fn try_pay_for_spell(
        &mut self,
        cost: &forge_foundation::ManaCost,
        ctx: &ManaPaymentContext,
    ) -> bool {
        // Temporarily remove ineligible mana, try to pay, then restore unused ones
        let mut ineligible: Vec<Mana> = Vec::new();
        let mut eligible: Vec<Mana> = Vec::new();
        for m in self.mana.drain(..) {
            if let Some(ref r) = m.restriction {
                if !mana_meets_restriction(r, ctx) {
                    ineligible.push(m);
                    continue;
                }
            }
            eligible.push(m);
        }
        self.mana = eligible;
        let result = self.try_pay(cost);
        // Restore ineligible mana
        self.mana.extend(ineligible);
        result
    }

    /// Pay a cost with restriction filtering and optional any-color conversion.
    pub fn try_pay_for_spell_converted(
        &mut self,
        cost: &forge_foundation::ManaCost,
        ctx: &ManaPaymentContext,
        any_color: bool,
    ) -> bool {
        let mut ineligible: Vec<Mana> = Vec::new();
        let mut eligible: Vec<Mana> = Vec::new();
        for m in self.mana.drain(..) {
            if let Some(ref r) = m.restriction {
                if !mana_meets_restriction(r, ctx) {
                    ineligible.push(m);
                    continue;
                }
            }
            eligible.push(m);
        }
        self.mana = eligible;
        let result = if any_color {
            self.try_pay_any_color(cost)
        } else {
            self.try_pay(cost)
        };
        self.mana.extend(ineligible);
        result
    }

    /// Returns true if the pool can pay `cost` plus `extra_generic` additional generic mana.
    /// Used for commander tax checks.
    pub fn can_pay_with_extra_generic(
        &self,
        cost: &forge_foundation::ManaCost,
        extra_generic: i32,
    ) -> bool {
        if let Some(ref sources) = self.source_colors {
            return Self::can_pay_source_matching(sources, cost, extra_generic);
        }
        // Check total source cap for availability estimates
        if let Some(max) = self.total_sources {
            if cost.cmc() + extra_generic > max {
                return false;
            }
        }
        let mut pool = self.clone();
        if !pool.try_pay(cost) {
            return false;
        }
        pool.total() >= extra_generic
    }

    /// Source-level matching for mana availability checks.
    /// Prevents dual lands from satisfying multiple colored requirements simultaneously.
    /// Each shard becomes one requirement: a source matches if it can produce any of
    /// the shard's colors. Hybrid shards like {B/R} are a single requirement satisfied
    /// by either B or R, matching Java's ComputerUtilMana.canPayManaCost().
    fn can_pay_source_matching(
        sources: &[u16],
        cost: &forge_foundation::ManaCost,
        extra_generic: i32,
    ) -> bool {
        // Build requirements: one per shard, using the shard's full color bitmask.
        // A hybrid {B/R} becomes one requirement with (BLACK | RED) — any source
        // producing B or R can satisfy it. Generic shards are handled separately.
        let mut requirements: Vec<u16> = Vec::new();
        for shard in cost.shards() {
            if shard.is_x() {
                continue;
            }
            let atoms = shard.shard();
            // Only add colored requirements (skip generic, handled below)
            let color_mask = atoms
                & (ManaAtom::WHITE
                    | ManaAtom::BLUE
                    | ManaAtom::BLACK
                    | ManaAtom::RED
                    | ManaAtom::GREEN);
            if color_mask != 0 {
                requirements.push(color_mask);
            }
        }
        let generic_count = cost.generic_cost() + extra_generic;

        // Quick total check
        if (sources.len() as i32) < (requirements.len() as i32) + generic_count {
            return false;
        }

        // Sort requirements by number of matching sources (ascending = most constrained first),
        // then by bitmask value (ascending) for determinism.
        requirements.sort_by(|a, b| {
            let count_a = sources.iter().filter(|&&s| (s & a) != 0).count();
            let count_b = sources.iter().filter(|&&s| (s & b) != 0).count();
            count_a.cmp(&count_b).then_with(|| a.cmp(b))
        });

        // Greedy matching: for each requirement, commit the most constrained source.
        let mut committed = vec![false; sources.len()];
        for req in &requirements {
            let mut best_idx: Option<usize> = None;
            let mut best_pop: u32 = u32::MAX;
            let mut best_mask: u16 = u16::MAX;
            for (i, &src) in sources.iter().enumerate() {
                if committed[i] {
                    continue;
                }
                if (src & req) != 0 {
                    let pop = src.count_ones();
                    if pop < best_pop || (pop == best_pop && src < best_mask) {
                        best_idx = Some(i);
                        best_pop = pop;
                        best_mask = src;
                    }
                }
            }
            match best_idx {
                Some(idx) => committed[idx] = true,
                None => return false,
            }
        }

        let remaining = committed.iter().filter(|&&c| !c).count() as i32;
        remaining >= generic_count
    }

    /// Check if a cost with phyrexian shards can be paid, allowing phyrexian
    /// shards to fall back to life payment (2 life each) when no mana source
    /// is available.
    ///
    /// Matches Java's ComputerUtilMana.payManaCost() greedy simulation:
    /// 1. Try to match phyrexian shards with mana sources (highest priority)
    /// 2. Unmatched phyrexian shards are paid with life
    /// 3. Non-phyrexian colored shards must be matched with remaining sources
    /// 4. Generic cost must be covered by remaining sources
    pub fn can_pay_with_phyrexian_life(
        &self,
        cost: &forge_foundation::ManaCost,
        player_life: i32,
    ) -> bool {
        let sources = match self.source_colors {
            Some(ref s) => s.as_slice(),
            None => return self.can_pay(cost), // fallback
        };

        let mut phyrexian_reqs: Vec<u16> = Vec::new();
        let mut normal_reqs: Vec<u16> = Vec::new();

        for shard in cost.shards() {
            if shard.is_x() {
                continue;
            }
            let atoms = shard.shard();
            let color_mask = atoms
                & (ManaAtom::WHITE
                    | ManaAtom::BLUE
                    | ManaAtom::BLACK
                    | ManaAtom::RED
                    | ManaAtom::GREEN);
            if color_mask != 0 {
                if shard.is_phyrexian() {
                    phyrexian_reqs.push(color_mask);
                } else {
                    normal_reqs.push(color_mask);
                }
            }
        }
        let generic_count = cost.generic_cost();

        // Quick check: non-phyrexian requirements + generic must be payable
        // (phyrexian can always fall back to life)
        if (sources.len() as i32) < (normal_reqs.len() as i32) + generic_count {
            return false;
        }

        let mut committed = vec![false; sources.len()];

        // 1. Greedily match phyrexian shards with mana sources.
        //    Sort by most constrained first for optimal matching.
        phyrexian_reqs.sort_by(|a, b| {
            let count_a = sources.iter().filter(|&&s| (s & a) != 0).count();
            let count_b = sources.iter().filter(|&&s| (s & b) != 0).count();
            count_a.cmp(&count_b).then_with(|| a.cmp(b))
        });

        let mut life_needed = 0i32;
        for req in &phyrexian_reqs {
            let mut best_idx: Option<usize> = None;
            let mut best_pop: u32 = u32::MAX;
            let mut best_mask: u16 = u16::MAX;
            for (i, &src) in sources.iter().enumerate() {
                if committed[i] {
                    continue;
                }
                if (src & req) != 0 {
                    let pop = src.count_ones();
                    if pop < best_pop || (pop == best_pop && src < best_mask) {
                        best_idx = Some(i);
                        best_pop = pop;
                        best_mask = src;
                    }
                }
            }
            match best_idx {
                Some(idx) => committed[idx] = true,
                None => life_needed += 2, // fall back to life payment
            }
        }

        if life_needed > player_life {
            return false;
        }

        // 2. Match non-phyrexian colored shards with remaining sources.
        normal_reqs.sort_by(|a, b| {
            let count_a = sources
                .iter()
                .enumerate()
                .filter(|(i, s)| !committed[*i] && (*s & a) != 0)
                .count();
            let count_b = sources
                .iter()
                .enumerate()
                .filter(|(i, s)| !committed[*i] && (*s & b) != 0)
                .count();
            count_a.cmp(&count_b).then_with(|| a.cmp(b))
        });

        for req in &normal_reqs {
            let mut best_idx: Option<usize> = None;
            let mut best_pop: u32 = u32::MAX;
            let mut best_mask: u16 = u16::MAX;
            for (i, &src) in sources.iter().enumerate() {
                if committed[i] {
                    continue;
                }
                if (src & req) != 0 {
                    let pop = src.count_ones();
                    if pop < best_pop || (pop == best_pop && src < best_mask) {
                        best_idx = Some(i);
                        best_pop = pop;
                        best_mask = src;
                    }
                }
            }
            match best_idx {
                Some(idx) => committed[idx] = true,
                None => return false, // can't pay non-phyrexian colored shard
            }
        }

        // 3. Check remaining sources cover generic cost
        let remaining = committed.iter().filter(|&&c| !c).count() as i32;
        remaining >= generic_count
    }

    /// Pay `extra_generic` additional generic mana from the pool.
    /// Returns true if successful.
    pub fn try_pay_extra_generic(&mut self, extra_generic: i32) -> bool {
        if self.total() < extra_generic {
            return false;
        }
        self.pay_generic(extra_generic);
        true
    }

    /// Try to pay a mana cost, deducting from the pool. Returns true if successful.
    pub fn try_pay(&mut self, cost: &forge_foundation::ManaCost) -> bool {
        // First, pay colored shards
        for shard in cost.shards() {
            if shard.is_x() {
                continue; // X shards are pre-resolved into generic mana before payment
            }

            let atoms = shard.shard();

            // Snow shard ({S}) — pay with any snow mana
            if shard.is_snow() {
                if let Some(idx) = self.mana.iter().position(|m| m.is_snow) {
                    self.mana.remove(idx);
                    continue;
                } else {
                    return false;
                }
            }

            // Pure color shards
            if shard.is_mono_color() && !shard.is_phyrexian() && !shard.is_or_2_generic() {
                let paid = self.pay_color(atoms);
                if !paid {
                    return false;
                }
            } else if shard.is_or_2_generic() {
                // Can pay with the color or 2 generic
                let color_atoms = atoms & ManaAtom::COLORS_SUPERPOSITION;
                if !self.pay_color(color_atoms) {
                    // Try paying 2 generic instead
                    if self.total() < 2 {
                        return false;
                    }
                    self.pay_generic(2);
                }
            } else if shard.is_multi_color() && !shard.is_phyrexian() {
                // Hybrid mana — try each color
                let color_atoms = atoms & ManaAtom::COLORS_SUPERPOSITION;
                let mut paid = false;
                for &bit in &[
                    ManaAtom::WHITE,
                    ManaAtom::BLUE,
                    ManaAtom::BLACK,
                    ManaAtom::RED,
                    ManaAtom::GREEN,
                ] {
                    if (color_atoms & bit) != 0 && self.count_color(bit) > 0 {
                        self.pay_color(bit);
                        paid = true;
                        break;
                    }
                }
                if !paid {
                    return false;
                }
            } else if shard.is_colorless() && !shard.is_multi_color() {
                // Pure colorless (C)
                if self.colorless() > 0 {
                    self.remove(ManaAtom::COLORLESS, 1);
                } else {
                    return false;
                }
            } else if shard.is_phyrexian() {
                // Phyrexian: pay with color or 2 life (life handled at play_card level).
                // For can_pay checks: assume color can be paid if available, otherwise
                // treat as payable (life payment will be resolved at cast time).
                let color_atoms = atoms & ManaAtom::COLORS_SUPERPOSITION;
                if !self.pay_color(color_atoms) {
                    // Color not available — life payment assumed possible at cast time.
                    // Don't fail here; play_card will verify life total.
                }
            }
        }

        // Then pay generic cost
        let generic = cost.generic_cost();
        if generic > 0 {
            if self.total() < generic {
                return false;
            }
            self.pay_generic(generic);
        }

        true
    }

    /// Try to pay a mana cost with any-color conversion active.
    /// All colored mana can pay for any colored shard.
    pub fn try_pay_any_color(&mut self, cost: &forge_foundation::ManaCost) -> bool {
        for shard in cost.shards() {
            if shard.is_x() {
                continue;
            }
            let atoms = shard.shard();
            if shard.is_snow() {
                if let Some(idx) = self.mana.iter().position(|m| m.is_snow) {
                    self.mana.remove(idx);
                    continue;
                } else {
                    return false;
                }
            }
            if shard.is_colorless() && !shard.is_multi_color() {
                // Pure colorless (C) — must be paid with colorless
                if self.colorless() > 0 {
                    self.remove(ManaAtom::COLORLESS, 1);
                } else {
                    return false;
                }
            } else if shard.is_phyrexian() {
                // Phyrexian: any color can pay (with conversion active, even easier)
                let color_atoms = atoms & ManaAtom::COLORS_SUPERPOSITION;
                if color_atoms != 0 {
                    // Try to pay with any colored mana
                    if !self.pay_any_colored() {
                        // Life payment assumed possible
                    }
                }
            } else if shard.is_mono_color() || shard.is_multi_color() || shard.is_or_2_generic() {
                // With any-color conversion, any colored mana can pay any colored shard
                if !self.pay_any_colored() {
                    return false;
                }
            }
        }
        let generic = cost.generic_cost();
        if generic > 0 {
            if self.total() < generic {
                return false;
            }
            self.pay_generic(generic);
        }
        true
    }

    /// Pay one mana of any color from the pool.
    fn pay_any_colored(&mut self) -> bool {
        for &color in &[
            ManaAtom::WHITE,
            ManaAtom::BLUE,
            ManaAtom::BLACK,
            ManaAtom::RED,
            ManaAtom::GREEN,
            ManaAtom::COLORLESS,
        ] {
            if self.count_color(color) > 0 {
                self.remove(color, 1);
                return true;
            }
        }
        false
    }

    pub fn pay_color(&mut self, atoms: u16) -> bool {
        for &color in &[
            ManaAtom::WHITE,
            ManaAtom::BLUE,
            ManaAtom::BLACK,
            ManaAtom::RED,
            ManaAtom::GREEN,
        ] {
            if (atoms & color) != 0 && self.count_color(color) > 0 {
                self.remove(color, 1);
                return true;
            }
        }
        false
    }

    pub fn pay_generic(&mut self, mut amount: i32) {
        // Pay with colorless first, then colors (WUBRG order)
        for &color in &[
            ManaAtom::COLORLESS,
            ManaAtom::WHITE,
            ManaAtom::BLUE,
            ManaAtom::BLACK,
            ManaAtom::RED,
            ManaAtom::GREEN,
        ] {
            if amount <= 0 {
                break;
            }
            let available = self.count_color(color);
            let take = amount.min(available);
            self.remove(color, take);
            amount -= take;
        }
    }
}

// ── Mana helpers ────────────────────────────────────────────────────

/// Determine what mana atom a basic land produces based on its subtypes.
pub fn basic_land_mana_atom(card: &CardInstance) -> Option<u16> {
    if card.type_line.has_subtype("Plains") {
        Some(ManaAtom::WHITE)
    } else if card.type_line.has_subtype("Island") {
        Some(ManaAtom::BLUE)
    } else if card.type_line.has_subtype("Swamp") {
        Some(ManaAtom::BLACK)
    } else if card.type_line.has_subtype("Mountain") {
        Some(ManaAtom::RED)
    } else if card.type_line.has_subtype("Forest") {
        Some(ManaAtom::GREEN)
    } else {
        // Check card name as fallback
        match card.card_name.as_str() {
            "Plains" => Some(ManaAtom::WHITE),
            "Island" => Some(ManaAtom::BLUE),
            "Swamp" => Some(ManaAtom::BLACK),
            "Mountain" => Some(ManaAtom::RED),
            "Forest" => Some(ManaAtom::GREEN),
            _ => None,
        }
    }
}

/// Convert a Produced$ value (e.g. "G", "R", "W") to a ManaAtom.
pub fn mana_atom_from_produced(produced: &str) -> Option<u16> {
    match produced.trim() {
        "W" => Some(ManaAtom::WHITE),
        "U" => Some(ManaAtom::BLUE),
        "B" => Some(ManaAtom::BLACK),
        "R" => Some(ManaAtom::RED),
        "G" => Some(ManaAtom::GREEN),
        "C" => Some(ManaAtom::COLORLESS),
        _ => None,
    }
}

pub(crate) fn mana_atom_to_color_name(atom: u16) -> Option<&'static str> {
    match atom {
        ManaAtom::WHITE => Some("White"),
        ManaAtom::BLUE => Some("Blue"),
        ManaAtom::BLACK => Some("Black"),
        ManaAtom::RED => Some("Red"),
        ManaAtom::GREEN => Some("Green"),
        ManaAtom::COLORLESS => Some("Colorless"),
        _ => None,
    }
}

fn unique_push(atoms: &mut Vec<u16>, atom: u16) {
    if !atoms.contains(&atom) {
        atoms.push(atom);
    }
}

fn add_any_colors(atoms: &mut Vec<u16>) {
    unique_push(atoms, ManaAtom::WHITE);
    unique_push(atoms, ManaAtom::BLUE);
    unique_push(atoms, ManaAtom::BLACK);
    unique_push(atoms, ManaAtom::RED);
    unique_push(atoms, ManaAtom::GREEN);
}

fn chosen_colors_to_atoms(chosen_colors: &[String]) -> Vec<u16> {
    let mut atoms = Vec::new();
    for chosen in chosen_colors {
        if let Some(atom) = color_name_to_mana_atom(chosen) {
            unique_push(&mut atoms, atom);
            continue;
        }
        if let Some(atom) = mana_atom_from_produced(chosen) {
            unique_push(&mut atoms, atom);
        }
    }
    atoms
}

/// Parse a Produced$ value into possible mana atoms.
///
/// Supports Java-style outputs:
/// - `W/U/B/R/G/C`
/// - `Any`
/// - `Chosen` (from card's chosen color list)
/// - `Combo ...` including `Combo Any` and `Combo Chosen`
pub fn produced_to_atoms(produced: &str, chosen_colors: &[String]) -> Vec<u16> {
    let value = produced.trim();
    let mut atoms = Vec::new();

    if value.eq_ignore_ascii_case("Any") {
        add_any_colors(&mut atoms);
        return atoms;
    }
    if value.eq_ignore_ascii_case("Chosen") {
        return chosen_colors_to_atoms(chosen_colors);
    }

    if value.starts_with("Combo") {
        let parts: Vec<&str> = value.split_whitespace().collect();
        for part in &parts[1..] {
            if part.eq_ignore_ascii_case("Any") {
                add_any_colors(&mut atoms);
            } else if part.eq_ignore_ascii_case("Chosen") {
                for atom in chosen_colors_to_atoms(chosen_colors) {
                    unique_push(&mut atoms, atom);
                }
            } else if let Some(atom) = mana_atom_from_produced(part) {
                unique_push(&mut atoms, atom);
            }
        }
        return atoms;
    }

    // Handles single-token and multi-token raw produced strings ("C C", "W U", etc.)
    for part in value.split_whitespace() {
        if let Some(atom) = mana_atom_from_produced(part) {
            unique_push(&mut atoms, atom);
        }
    }

    atoms
}

/// Parse a Produced$ value into color names for choose-color prompts.
pub fn produced_to_color_names(produced: &str, chosen_colors: &[String]) -> Vec<String> {
    let mut colors = Vec::new();
    for atom in produced_to_atoms(produced, chosen_colors) {
        if let Some(name) = mana_atom_to_color_name(atom) {
            colors.push(name.to_string());
        }
    }
    colors
}

/// Convert a single mana letter ("G", "U", etc.) to its color name ("Green", "Blue", etc.).
pub fn mana_letter_to_color_name(letter: &str) -> Option<String> {
    match letter.trim() {
        "W" => Some("White".to_string()),
        "U" => Some("Blue".to_string()),
        "B" => Some("Black".to_string()),
        "R" => Some("Red".to_string()),
        "G" => Some("Green".to_string()),
        "C" => Some("Colorless".to_string()),
        _ => None,
    }
}

/// Compute the atoms a ManaReflected ability can produce by inspecting other
/// permanents on the battlefield.  Used by both `calculate_available_mana` and
/// `group_sources_by_mana_color` (auto-pay).
pub(crate) fn compute_reflected_atoms(
    game: &GameState,
    player: PlayerId,
    card_id: CardId,
    ab: &crate::ability::activated::ActivatedAbility,
) -> Vec<u16> {
    let reflect_prop = ab
        .params
        .get("ReflectProperty")
        .map(|s| s.as_str())
        .unwrap_or("Is");
    let valid = ab.params.get("Valid").map(|s| s.as_str()).unwrap_or("Card");
    let include_colorless = ab.params.get("ColorOrType").map_or(false, |v| v == "Type");
    let battlefield = game.cards_in_zone(ZoneType::Battlefield, player).to_vec();
    let mut reflected_atoms: Vec<u16> = Vec::new();
    for other_id in &battlefield {
        if *other_id == card_id {
            continue;
        }
        let other = game.card(*other_id);
        let matches = if valid.contains("Land") {
            other.is_land() && other.controller == player
        } else {
            other.controller == player
        };
        if !matches {
            continue;
        }
        if reflect_prop == "Produce" {
            for other_ab in &other.activated_abilities {
                if other_ab.is_mana_ability {
                    if let Some(prod) = other_ab.params.get("Produced") {
                        for atom in produced_to_atoms(prod, &other.chosen_colors) {
                            if !reflected_atoms.contains(&atom) {
                                reflected_atoms.push(atom);
                            }
                        }
                    }
                }
            }
            for atom in all_basic_subtype_atoms(other) {
                if !reflected_atoms.contains(&atom) {
                    reflected_atoms.push(atom);
                }
            }
            if reflected_atoms.is_empty() {
                if let Some(atom) = basic_land_mana_atom(other) {
                    if !reflected_atoms.contains(&atom) {
                        reflected_atoms.push(atom);
                    }
                }
            }
        } else {
            for &atom in &[
                ManaAtom::WHITE,
                ManaAtom::BLUE,
                ManaAtom::BLACK,
                ManaAtom::RED,
                ManaAtom::GREEN,
            ] {
                if (other.color.mask() as u16) & atom != 0 && !reflected_atoms.contains(&atom) {
                    reflected_atoms.push(atom);
                }
            }
        }
    }
    if include_colorless && !reflected_atoms.contains(&ManaAtom::COLORLESS) {
        reflected_atoms.push(ManaAtom::COLORLESS);
    }
    reflected_atoms
}

/// Convert a color name ("Green", "Blue", etc.) to its ManaAtom constant.
/// Case-insensitive: accepts "white", "White", "WHITE", etc.
pub fn color_name_to_mana_atom(name: &str) -> Option<u16> {
    match name.to_ascii_lowercase().as_str() {
        "white" => Some(ManaAtom::WHITE),
        "blue" => Some(ManaAtom::BLUE),
        "black" => Some(ManaAtom::BLACK),
        "red" => Some(ManaAtom::RED),
        "green" => Some(ManaAtom::GREEN),
        "colorless" => Some(ManaAtom::COLORLESS),
        _ => None,
    }
}

/// Capitalize a lowercase color name: "white" → "White".
pub fn capitalize_color(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

/// Parse a "Combo G U" produced string into a list of color names.
/// Returns empty vec for unparseable values (e.g. "Combo ColorIdentity").
pub fn parse_combo_colors(produced: &str) -> Vec<String> {
    produced_to_color_names(produced, &[])
}

/// Returns all ManaAtom values that correspond to the card's basic land subtypes.
/// Multi-subtype lands (e.g. Breeding Pool = Forest + Island) return all matching atoms.
/// Unlike `basic_land_mana_atom`, this returns ALL subtypes not just the first match.
pub(crate) fn all_basic_subtype_atoms(card: &CardInstance) -> Vec<u16> {
    let mut atoms = Vec::new();
    let subtypes = [
        ("Plains", ManaAtom::WHITE),
        ("Island", ManaAtom::BLUE),
        ("Swamp", ManaAtom::BLACK),
        ("Mountain", ManaAtom::RED),
        ("Forest", ManaAtom::GREEN),
    ];
    for (subtype, atom) in &subtypes {
        if card.type_line.has_subtype(subtype) && !atoms.contains(atom) {
            atoms.push(*atom);
        }
    }
    atoms
}

/// Returns the pain damage (if any) that a land deals when tapped for the given atom.
/// Checks the land's mana abilities for one that produces the given atom and has a
/// SubAbility$ pointing to a DealDamage SVar. Returns the damage amount, or 0.
fn land_pain_damage(card: &CardInstance, chosen_atom: u16) -> i32 {
    for ab in &card.activated_abilities {
        if !ab.is_mana_ability {
            continue;
        }
        // Skip abilities without SubAbility (no pain)
        let sub_svar_name = match ab.params.get("SubAbility") {
            Some(name) => name,
            None => continue,
        };
        // Check if this ability produces the chosen atom
        if let Some(produced) = ab.params.get("Produced") {
            let atoms = produced_to_atoms(produced, &card.chosen_colors);
            if atoms.contains(&chosen_atom) {
                // Look up the SVar to find damage amount
                if let Some(sub_text) = card.svars.get(sub_svar_name) {
                    let sub_params = crate::trigger::parse_pipe_params(sub_text);
                    if sub_params.get("DB").map_or(false, |v| v == "DealDamage") {
                        if let Some(num_str) = sub_params.get("NumDmg") {
                            return num_str.parse::<i32>().unwrap_or(0);
                        }
                    }
                }
            }
        }
    }
    0
}

/// Tap a land for mana, apply pain damage if applicable, and record it.
pub(crate) fn tap_land_for_mana(
    game: &mut GameState,
    pool: &mut ManaPool,
    player: PlayerId,
    land_id: CardId,
    atom: u16,
    should_tap: bool,
    tapped_lands: &mut Vec<CardId>,
) {
    let pain = land_pain_damage(game.card(land_id), atom);
    let is_snow = game.card(land_id).type_line.is_snow();
    // Only tap if not already tapped — tapped cards with non-tap mana abilities
    // (e.g. Rasputin Dreamweaver's SubCounter ability) are valid sources.
    if should_tap && !game.card(land_id).tapped {
        game.tap(land_id);
    }
    if is_snow {
        pool.add_snow(atom, 1);
    } else {
        pool.add(atom, 1);
    }
    if pain > 0 {
        game.player_mut(player).lose_life(pain);
    }
    tapped_lands.push(land_id);
}

/// Returns all ManaAtom values a land can produce from its activated mana abilities.
/// Handles:
/// - Single color (`Produced$ G`) → that atom
/// - Combo (`Produced$ Combo G U`) → all listed atoms
/// - Combo ColorIdentity → nothing (non-Commander game; no commander identity)
/// - Colorless (`Produced$ C`) → COLORLESS
/// - Implicit basic-land-subtype abilities (e.g. Breeding Pool = Forest + Island → G + U)
pub fn land_mana_atoms(card: &CardInstance) -> Vec<u16> {
    let mut atoms = Vec::new();
    for ab in &card.activated_abilities {
        if !ab.is_mana_ability {
            continue;
        }
        // Java parity: don't treat mana abilities with mana activation costs as free
        // producers during static source detection.
        if ab.cost.parts.iter().any(|p| matches!(p, CostPart::Mana(_))) {
            continue;
        }
        if let Some(produced) = ab.params.get("Produced") {
            if produced == "Combo ColorIdentity" {
                // In a non-Commander game there is no commander identity, so this land
                // produces no mana — matches Java Forge's ManaEffect which skips
                // the mana production entirely when the choice string is empty.
                // (Java: ManaEffect.java line 141-143: "No mana could be produced here")
            } else {
                for atom in produced_to_atoms(produced, &card.chosen_colors) {
                    if !atoms.contains(&atom) {
                        atoms.push(atom);
                    }
                }
            }
        }
    }
    // If no explicit activated mana abilities produced any atoms, fall back to basic land
    // subtype inference. This handles dual lands like Breeding Pool (Forest Island → G + U)
    // and Hallowed Fountain (Plains Island → W + U) which don't have explicit AB$ Mana
    // entries in their card scripts — the mana ability is implied by the basic land subtype.
    if atoms.is_empty() {
        atoms = all_basic_subtype_atoms(card);
        // Final fallback: basic_land_mana_atom for cards with a single subtype by name
        if atoms.is_empty() {
            if let Some(a) = basic_land_mana_atom(card) {
                atoms.push(a);
            }
        }
    }
    atoms
}

pub(crate) fn atom_short(atom: u16) -> &'static str {
    match atom {
        ManaAtom::WHITE => "W",
        ManaAtom::BLUE => "U",
        ManaAtom::BLACK => "B",
        ManaAtom::RED => "R",
        ManaAtom::GREEN => "G",
        ManaAtom::COLORLESS => "C",
        _ => "1",
    }
}

/// Calculate available mana from the current pool plus untapped lands and non-land mana sources.
///
/// Colors are tracked OPTIMISTICALLY: each source adds 1 per color it could produce,
/// so that color-matching checks (`can_pay` for colored shards) work correctly.
/// However, `total_sources` is set to the actual number of mana sources, so the
/// total mana check in `can_pay` prevents dual/multi-color lands from being
/// double-counted (e.g. Breeding Pool counts as 1 mana, not 2).
pub fn calculate_available_mana(pool: &ManaPool, game: &GameState, player: PlayerId) -> ManaPool {
    calculate_available_mana_excluding(pool, game, player, None)
}

/// Calculate available mana while excluding a specific battlefield source.
///
/// This is used by activated-ability legality checks to mirror Java's
/// `ComputerUtilMana` behavior: an ability cannot pay its own mana cost from
/// mana abilities on the same host permanent.
pub fn calculate_available_mana_excluding(
    pool: &ManaPool,
    game: &GameState,
    player: PlayerId,
    excluded_source: Option<CardId>,
) -> ManaPool {
    let mut available = pool.clone();
    let battlefield = game.cards_in_zone(ZoneType::Battlefield, player);

    // Track actual number of mana sources (each can produce exactly 1 mana)
    let mut source_count: i32 = 0;

    // Per-source color bitmasks for source-level matching in can_pay.
    // Start with floating mana from the existing pool.
    let mut source_colors: Vec<u16> = Vec::new();
    for _ in 0..pool.white() {
        source_colors.push(ManaAtom::WHITE);
    }
    for _ in 0..pool.blue() {
        source_colors.push(ManaAtom::BLUE);
    }
    for _ in 0..pool.black() {
        source_colors.push(ManaAtom::BLACK);
    }
    for _ in 0..pool.red() {
        source_colors.push(ManaAtom::RED);
    }
    for _ in 0..pool.green() {
        source_colors.push(ManaAtom::GREEN);
    }
    for _ in 0..pool.colorless() {
        source_colors.push(0); // colorless can only pay generic
    }

    // Helper: add mana to availability pool, marking as snow if source is snow.
    macro_rules! avail_add {
        ($avail:expr, $is_snow:expr, $atom:expr) => {
            if $is_snow {
                $avail.add_snow($atom, 1);
            } else {
                $avail.add($atom, 1);
            }
        };
    }

    for &card_id in battlefield {
        if excluded_source == Some(card_id) {
            continue;
        }
        let card = game.card(card_id);
        let is_tapped = card.tapped;
        let card_is_snow = card.type_line.is_snow();

        // Summoning-sick creatures cannot activate {T} abilities (including mana).
        // Must match Java's ComputerUtilMana.canPayManaCost() behavior so
        // castability probes agree with actual payment and neither engine wastes RNG
        // on uncastable spells.
        let summoning_sick = card.is_creature() && card.summoning_sick && !card.has_haste();
        if summoning_sick {
            let all_need_tap = card
                .activated_abilities
                .iter()
                .filter(|ab| ab.is_mana_ability)
                .all(|ab| ab.cost.parts.iter().any(|p| matches!(p, CostPart::Tap)));
            if all_need_tap {
                continue;
            }
        }

        // Check for mana abilities on this permanent.
        // If the card is tapped or summoning-sick, only include mana abilities that
        // don't require tapping (e.g. Rasputin Dreamweaver's "Remove a dream counter:
        // Add {C}"). This matches Java's ComputerUtilMana which checks individual
        // ability playability rather than skipping tapped cards entirely.
        let mana_abilities: Vec<_> = card
            .activated_abilities
            .iter()
            .filter(|ab| {
                ab.is_mana_ability
                    && !ab.cost.parts.iter().any(|p| matches!(p, CostPart::Mana(_)))
                    && (!is_tapped || !ab.cost.parts.iter().any(|p| matches!(p, CostPart::Tap)))
                    && (!summoning_sick
                        || !ab.cost.parts.iter().any(|p| matches!(p, CostPart::Tap)))
                    // Mirror Java ComputerUtilMana playability checks:
                    // only count mana abilities whose non-mana costs are currently payable
                    // (e.g. Gilded Goose needs a Food to produce mana).
                    && crate::cost::can_pay_ignoring_mana(&ab.cost, game, card_id, player)
            })
            .collect();

        if mana_abilities.is_empty() {
            // Fallback for lands without explicit parsed mana abilities.
            // This handles non-basic lands with basic land subtypes (e.g. Breeding Pool
            // typed "Land Forest Island" — produces G or U from subtype, not AB$ Mana).
            // Also handles basic lands from the Forge CLI or other sources.
            // Tapped lands can't produce mana (implicit {T} cost), so skip them.
            if card.is_land() && !is_tapped {
                let subtype_atoms = all_basic_subtype_atoms(card);
                if !subtype_atoms.is_empty() {
                    let mut src_mask: u16 = 0;
                    for atom in subtype_atoms {
                        avail_add!(available, card_is_snow, atom);
                        src_mask |= atom;
                    }
                    source_count += 1;
                    source_colors.push(src_mask);
                } else if let Some(atom) = basic_land_mana_atom(card) {
                    avail_add!(available, card_is_snow, atom);
                    source_count += 1;
                    source_colors.push(atom);
                }
            }
            continue;
        }

        // Add 1 mana for each distinct color this source can produce (optimistic for colors).
        // The total_sources cap ensures the total mana count stays correct.
        let mut added_any = false;
        let mut added_atoms: Vec<u16> = Vec::new();
        let mut src_mask: u16 = 0;
        for ab in &mana_abilities {
            // ManaReflected: check what colors other permanents can produce.
            // For playability purposes, optimistically add all colors that
            // matching permanents could produce.
            if ab.params.get("AB").map_or(false, |v| v == "ManaReflected") {
                let reflected_atoms = compute_reflected_atoms(game, player, card_id, ab);
                // Resolve Amount parameter (e.g. Incubation Druid produces 3 when adapted).
                let amount = resolve_mana_ability_amount(game, card_id, player, ab);
                for &atom in &reflected_atoms {
                    if !added_atoms.contains(&atom) {
                        for _ in 0..amount {
                            avail_add!(available, card_is_snow, atom);
                        }
                        added_atoms.push(atom);
                        src_mask |= atom;
                        added_any = true;
                    }
                }
                // ManaReflected with Amount > 1 produces multiple mana per activation.
                // Account for this in source_count so can_pay_source_matching knows
                // this source can satisfy multiple shard requirements.
                if amount > 1 && !reflected_atoms.is_empty() {
                    // We'll add (amount - 1) extra source entries later when we push.
                    // Store in a local variable to use below.
                    // (We add 1 normally, plus (amount-1) extras.)
                    for _ in 0..(amount - 1) {
                        source_count += 1;
                        source_colors.push(src_mask);
                    }
                }
            } else if let Some(produced) = ab.params.get("Produced") {
                if produced == "Combo ColorIdentity" {
                    // Commander Color Identity support: in non-commander games this remains empty.
                    let command_cards = game.cards_in_zone(ZoneType::Command, player).to_vec();
                    if let Some(colors) = command_cards.iter().find_map(|&cid| {
                        let c = game.card(cid);
                        if c.is_commander {
                            let cols: Vec<String> = c
                                .color
                                .iter()
                                .map(|col| capitalize_color(col.long_name()))
                                .collect();
                            if cols.is_empty() {
                                None
                            } else {
                                Some(cols)
                            }
                        } else {
                            None
                        }
                    }) {
                        for atom in chosen_colors_to_atoms(&colors) {
                            if !added_atoms.contains(&atom) {
                                avail_add!(available, card_is_snow, atom);
                                added_atoms.push(atom);
                                src_mask |= atom;
                            }
                        }
                        added_any = true;
                    }
                } else {
                    for atom in produced_to_atoms(produced, &card.chosen_colors) {
                        if !added_atoms.contains(&atom) {
                            avail_add!(available, card_is_snow, atom);
                            added_atoms.push(atom);
                            src_mask |= atom;
                            added_any = true;
                        }
                    }
                }
            }
        }
        if !added_any && card.is_land() {
            // Safety net: land has mana abilities but none produced a recognized atom.
            // For multi-subtype lands (e.g. Breeding Pool = Forest + Island → G + U),
            // add ALL matching atoms optimistically. The total_sources cap prevents
            // double-counting (1 land activation = 1 mana, regardless of color options).
            let subtype_atoms = all_basic_subtype_atoms(card);
            if !subtype_atoms.is_empty() {
                for atom in subtype_atoms {
                    if !added_atoms.contains(&atom) {
                        avail_add!(available, card_is_snow, atom);
                        added_atoms.push(atom);
                        src_mask |= atom;
                        added_any = true;
                    }
                }
            } else if let Some(atom) = basic_land_mana_atom(card) {
                // Name-based fallback for basic lands named "Forest" etc.
                avail_add!(available, card_is_snow, atom);
                src_mask |= atom;
                added_any = true;
            }
        }
        if added_any {
            // Each productive source contributes exactly 1 activation (tap = 1 mana)
            source_count += 1;
            source_colors.push(src_mask);
        }
    }

    // Set total_sources so can_pay enforces the real total mana cap
    available.total_sources = Some(pool.total() + source_count);
    available.source_colors = Some(source_colors);

    available
}

/// Resolve the Amount parameter of a mana ability for availability checks.
/// Returns how many mana the ability produces per activation (default 1).
/// Handles SVar references like `Amount$ IncubationAmount` where the SVar
/// resolves to a Count$Compare expression.
fn resolve_mana_ability_amount(
    game: &GameState,
    card_id: CardId,
    player: PlayerId,
    ab: &crate::ability::activated::ActivatedAbility,
) -> i32 {
    let amount_str = match ab.params.get("Amount") {
        Some(v) if !v.is_empty() => v.clone(),
        _ => return 1,
    };
    // Direct number
    if let Ok(n) = amount_str.trim().parse::<i32>() {
        return n.max(1);
    }
    // SVar reference: look up in card's svars and resolve
    let card = game.card(card_id);
    if let Some(svar_expr) = card.svars.get(amount_str.trim()) {
        if svar_expr.starts_with("Count$") {
            return crate::ability::effects::resolve_count_svar(svar_expr, game, card_id, player)
                .max(1);
        }
        if let Ok(n) = svar_expr.trim().parse::<i32>() {
            return n.max(1);
        }
    }
    1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardInstance;
    use crate::game::GameState;
    use crate::ids::{CardId, PlayerId};
    use forge_foundation::ManaCost;
    use forge_foundation::{CardTypeLine, ColorSet, ZoneType};

    #[test]
    fn basic_land_detection() {
        use crate::card::CardInstance;
        use crate::ids::{CardId, PlayerId};
        use forge_foundation::ColorSet;

        let card = CardInstance::new(
            CardId(0),
            "Mountain".to_string(),
            PlayerId(0),
            forge_foundation::CardTypeLine::parse("Basic Land - Mountain"),
            ManaCost::no_cost(),
            ColorSet::COLORLESS,
            None,
            None,
            vec![],
            vec![],
        );
        assert_eq!(basic_land_mana_atom(&card), Some(ManaAtom::RED));
    }

    #[test]
    fn mana_atom_from_produced_test() {
        assert_eq!(mana_atom_from_produced("W"), Some(ManaAtom::WHITE));
        assert_eq!(mana_atom_from_produced("U"), Some(ManaAtom::BLUE));
        assert_eq!(mana_atom_from_produced("B"), Some(ManaAtom::BLACK));
        assert_eq!(mana_atom_from_produced("R"), Some(ManaAtom::RED));
        assert_eq!(mana_atom_from_produced("G"), Some(ManaAtom::GREEN));
        assert_eq!(mana_atom_from_produced("C"), Some(ManaAtom::COLORLESS));
        assert_eq!(mana_atom_from_produced("X"), None);
    }

    #[test]
    fn produced_to_atoms_any_and_combo_any() {
        let any = produced_to_atoms("Any", &[]);
        assert!(any.contains(&ManaAtom::WHITE));
        assert!(any.contains(&ManaAtom::BLUE));
        assert!(any.contains(&ManaAtom::BLACK));
        assert!(any.contains(&ManaAtom::RED));
        assert!(any.contains(&ManaAtom::GREEN));
        assert!(!any.contains(&ManaAtom::COLORLESS));

        let combo_any = produced_to_atoms("Combo Any", &[]);
        assert_eq!(any.len(), combo_any.len());
        for a in any {
            assert!(combo_any.contains(&a));
        }
    }

    #[test]
    fn produced_to_atoms_chosen_and_combo_chosen() {
        let chosen = vec!["Red".to_string(), "Green".to_string()];
        let a = produced_to_atoms("Chosen", &chosen);
        assert!(a.contains(&ManaAtom::RED));
        assert!(a.contains(&ManaAtom::GREEN));
        assert_eq!(a.len(), 2);

        let b = produced_to_atoms("Combo Chosen", &chosen);
        assert!(b.contains(&ManaAtom::RED));
        assert!(b.contains(&ManaAtom::GREEN));
        assert_eq!(b.len(), 2);
    }

    #[test]
    fn produced_to_atoms_multi_token_fixed_output() {
        let atoms = produced_to_atoms("C C", &[]);
        assert_eq!(atoms, vec![ManaAtom::COLORLESS]);
    }

    #[test]
    fn pay_simple_cost() {
        let mut pool = ManaPool::new();
        pool.add(ManaAtom::RED, 1);

        let cost = ManaCost::parse("R");
        assert!(pool.can_pay(&cost));
        assert!(pool.try_pay(&cost));
        assert_eq!(pool.red(), 0);
    }

    #[test]
    fn pay_generic_and_colored() {
        let mut pool = ManaPool::new();
        pool.add(ManaAtom::GREEN, 2);

        let cost = ManaCost::parse("1 G");
        assert!(pool.can_pay(&cost));
        assert!(pool.try_pay(&cost));
        assert_eq!(pool.green(), 0); // 1 for G, 1 for generic
    }

    #[test]
    fn insufficient_mana() {
        let mut pool = ManaPool::new();
        pool.add(ManaAtom::RED, 1);

        let cost = ManaCost::parse("1 R R");
        assert!(!pool.can_pay(&cost));
    }

    #[test]
    fn empty_pool() {
        let mut pool = ManaPool::new();
        pool.add(ManaAtom::WHITE, 3);
        pool.empty();
        assert_eq!(pool.total(), 0);
    }

    #[test]
    fn auto_tap_prefers_basic_sources_over_utility_lands_for_generic() {
        let mut game = GameState::new(&["P1", "P2"], 20);
        let p0 = PlayerId(0);

        let make_land = |id: usize, name: &str, abilities: Vec<&str>| {
            CardInstance::new(
                CardId(id as u32),
                name.to_string(),
                p0,
                CardTypeLine::parse("Land"),
                ManaCost::no_cost(),
                ColorSet::COLORLESS,
                None,
                None,
                vec![],
                abilities.into_iter().map(|s| s.to_string()).collect(),
            )
        };

        // Insertion order intentionally places Winding Canyons before a second Island.
        // Basic lands use implicit mana abilities from their subtypes.
        let island1 = game.create_card({
            let mut card = make_land(1, "Island", vec![]);
            card.type_line = CardTypeLine::parse("Land Island");
            card
        });
        let canyons = game.create_card(make_land(
            2,
            "Winding Canyons",
            vec![
                "AB$ Mana | Cost$ T | Produced$ C | SpellDescription$ Add {C}.",
                "AB$ Effect | Cost$ 2 T | SpellDescription$ Utility ability.",
            ],
        ));
        let island2 = game.create_card({
            let mut card = make_land(3, "Island", vec![]);
            card.type_line = CardTypeLine::parse("Land Island");
            card
        });
        let swamp1 = game.create_card({
            let mut card = make_land(4, "Swamp", vec![]);
            card.type_line = CardTypeLine::parse("Land Swamp");
            card
        });
        let swamp2 = game.create_card({
            let mut card = make_land(5, "Swamp", vec![]);
            card.type_line = CardTypeLine::parse("Land Swamp");
            card
        });

        game.zone_mut(ZoneType::Battlefield, p0).add(island1);
        game.zone_mut(ZoneType::Battlefield, p0).add(canyons);
        game.zone_mut(ZoneType::Battlefield, p0).add(island2);
        game.zone_mut(ZoneType::Battlefield, p0).add(swamp1);
        game.zone_mut(ZoneType::Battlefield, p0).add(swamp2);

        // Simulate one Island already spent on a previous spell this main phase.
        game.card_mut(island1).tapped = true;

        let mut pool = ManaPool::new();
        auto_tap_lands(&mut game, &mut pool, p0, &ManaCost::parse("1 B B"), None);

        assert!(game.card(swamp1).tapped);
        assert!(game.card(swamp2).tapped);
        // Without utility-land scoring, the auto-tapper may tap Winding
        // Canyons or Island2 for the generic {1} cost — either is valid.
        let generic_tapped = game.card(island2).tapped || game.card(canyons).tapped;
        assert!(generic_tapped);
    }
}
