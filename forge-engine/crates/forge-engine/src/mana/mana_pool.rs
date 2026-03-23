//! ManaPool — floating mana pool for a player.
//!
//! Mirrors Java's `ManaPool.java`.
//! Manages floating mana objects, payment, clearing at phase transitions,
//! and mana restriction checking.

use forge_foundation::mana::ManaAtom;
use forge_foundation::PhaseType;
use serde::{Deserialize, Serialize};

use super::{Mana, ManaPaymentContext, mana_meets_restriction};
use crate::ids::CardId;

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

    /// Total floating mana count.
    /// Mirrors Java's `ManaPool.totalMana()`.
    pub fn total_mana(&self) -> i32 {
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
        let spent = amount.min(self.total_mana());
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

    /// Reset the pool completely (empties all floating mana).
    /// Mirrors Java's `ManaPool.resetPool()`.
    pub fn reset_pool(&mut self) {
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
        pool.total_mana() >= extra_generic
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
        if self.total_mana() < extra_generic {
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
                    if self.total_mana() < 2 {
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
            if self.total_mana() < generic {
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
            if self.total_mana() < generic {
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

    // ── Java parity methods (ManaPool.java) ────────────────────────

    /// Whether floating mana will be lost at end of phase.
    /// Mirrors Java's `ManaPool.willManaBeLostAtEndOfPhase()`.
    pub fn will_mana_be_lost_at_end_of_phase(&self) -> bool {
        !self.mana.is_empty()
    }

    /// Whether the game has mana burn rules active.
    /// Mirrors Java's `ManaPool.hasBurn()`.
    pub fn has_burn(&self) -> bool {
        false // Mana burn removed in modern rules
    }

    /// Remove a specific Mana object from the pool.
    /// Mirrors Java's `ManaPool.removeMana(Mana)`.
    pub fn remove_mana(&mut self, mana: &Mana) -> bool {
        if let Some(pos) = self.mana.iter().position(|m| {
            m.color == mana.color && m.source_card == mana.source_card
        }) {
            self.mana.remove(pos);
            true
        } else {
            false
        }
    }

    /// Pay mana cost using mana produced by a mana ability.
    /// Mirrors Java's `ManaPool.payManaFromAbility()`.
    pub fn pay_mana_from_ability(&mut self, produced_color: u16, amount: i32) {
        for _ in 0..amount {
            self.add(produced_color, 1);
        }
    }

    /// Try to pay a cost shard using floating mana of a specific color.
    /// Mirrors Java's `ManaPool.tryPayCostWithColor()`.
    pub fn try_pay_cost_with_color(&mut self, color: u16) -> bool {
        if self.count_color(color) > 0 {
            self.remove(color, 1);
            true
        } else {
            false
        }
    }

    /// Try to pay with a specific Mana object.
    /// Mirrors Java's `ManaPool.tryPayCostWithMana()`.
    pub fn try_pay_cost_with_mana(&mut self, mana: &Mana) -> bool {
        self.remove_mana(mana)
    }

    /// Account for mana produced by a mana ability (verify it's in the pool).
    /// Mirrors Java's `ManaPool.accountFor()`.
    pub fn account_for(&self, color: u16) -> bool {
        self.count_color(color) > 0
    }

    /// Refund mana back to the pool.
    /// Mirrors Java's `ManaPool.refundMana()`.
    pub fn refund_mana(&mut self, mana_spent: &mut Vec<Mana>) {
        for m in mana_spent.drain(..) {
            self.add_mana(m);
        }
    }

    /// Check if a mana cost shard can be paid by a given color.
    /// Mirrors Java's `ManaPool.canPayForShardWithColor()`.
    pub fn can_pay_for_shard_with_color(&self, shard_color: u16, pay_color: u16) -> bool {
        if shard_color == 0 {
            return true;
        }
        (shard_color & pay_color) != 0
    }

    /// Pay an entire mana cost from floating mana.
    /// Mirrors Java's `ManaPool.payManaCostFromPool()`.
    pub fn pay_mana_cost_from_pool(&mut self, cost: &forge_foundation::ManaCost) -> bool {
        self.try_pay(cost)
    }

    /// Iterator over all floating mana.
    /// Mirrors Java's `ManaPool.iterator()`.
    pub fn iterator(&self) -> impl Iterator<Item = &Mana> {
        self.mana.iter()
    }

    // ── Mana production (extracted from game_loop/game_action.rs) ────

    /// Produce mana from a mana string (e.g. "W", "U U", "R G") and add to pool.
    /// Handles source tracking, snow, restrictions, keywords, counters, triggers.
    /// This is the core mana production logic — the single source of truth.
    ///
    /// Call this from game_action.rs::resolve_mana_ability after determining
    /// what mana string to produce.
    pub fn produce_mana_from_string(
        &mut self,
        mana_string: &str,
        source_card: Option<CardId>,
        is_snow: bool,
        restriction: Option<String>,
        adds_no_counter: bool,
        adds_keywords: Option<String>,
        adds_keywords_valid: Option<String>,
        adds_counters: Option<String>,
        adds_counters_valid: Option<String>,
        triggers_when_spent: Option<String>,
    ) {
        for tok in mana_string.split_whitespace() {
            if let Some(atom) = super::mana_atom_from_produced(tok) {
                let mut m = Mana::simple(atom);
                m.source_card = source_card;
                m.is_snow = is_snow;
                m.restriction = restriction.clone();
                m.adds_no_counter = adds_no_counter;
                m.adds_keywords = adds_keywords.clone();
                m.adds_keywords_valid = adds_keywords_valid.clone();
                m.adds_counters = adds_counters.clone();
                m.adds_counters_valid = adds_counters_valid.clone();
                m.triggers_when_spent = triggers_when_spent.clone();
                self.add_mana(m);
            }
        }
    }

    /// Convert a ManaAtom to its short letter string.
    pub fn atom_to_letter(atom: u16) -> &'static str {
        match atom {
            ManaAtom::WHITE => "W",
            ManaAtom::BLUE => "U",
            ManaAtom::BLACK => "B",
            ManaAtom::RED => "R",
            ManaAtom::GREEN => "G",
            ManaAtom::COLORLESS => "C",
            _ => "C",
        }
    }
}
