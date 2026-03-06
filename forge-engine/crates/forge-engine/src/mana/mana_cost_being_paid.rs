use forge_foundation::mana::ManaAtom;
use forge_foundation::ManaCost;
use forge_foundation::ManaCostShard;
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
struct ShardCount {
    total_count: i32,
    /// How many of these shards are the X portion (for xManaCostPaidByColor tracking).
    x_count: i32,
}

/// Rust mirror of Java `forge.game.mana.ManaCostBeingPaid` focused on AI auto-pay needs.
#[derive(Debug, Clone, Default)]
pub(crate) struct ManaCostBeingPaid {
    unpaid_shards: HashMap<ManaCostShard, ShardCount>,
    /// Tracks which colors were used to pay X costs (for colored X restrictions).
    /// Maps color short string ("W","U","B","R","G") to count paid.
    pub(crate) x_mana_cost_paid_by_color: HashMap<String, i32>,
    /// Bitmask of all colors paid (for Sunburst/Converge).
    pub(crate) sunburst_map: u16,
    /// Number of X shards in the original cost.
    cnt_x: i32,
}

impl ManaCostBeingPaid {
    pub(crate) fn from_mana_cost(cost: &ManaCost) -> Self {
        let mut out = Self::default();
        for &shard in cost.shards() {
            if shard.is_x() {
                out.cnt_x += 1;
                continue;
            }
            out.increase_shard(shard, 1);
        }
        out.increase_generic_mana(cost.generic_cost());
        out
    }

    /// Set the X mana payment — converts X shards into concrete shards.
    /// Mirrors Java `ManaCostBeingPaid.setXManaCostPaid()`.
    pub(crate) fn set_x_mana_cost_paid(&mut self, x_paid: i32, x_color: &str) {
        let x_cost = x_paid * self.cnt_x;
        self.cnt_x = 0;
        let shard = match x_color {
            "W" => ManaCostShard::White,
            "U" => ManaCostShard::Blue,
            "B" => ManaCostShard::Black,
            "R" => ManaCostShard::Red,
            "G" => ManaCostShard::Green,
            "C" => ManaCostShard::Colorless,
            _ => ManaCostShard::Generic,
        };
        self.increase_shard_with_x(shard, x_cost);
    }

    fn increase_shard_with_x(&mut self, shard: ManaCostShard, amount: i32) {
        if amount <= 0 {
            return;
        }
        let entry = self.unpaid_shards.entry(shard).or_default();
        entry.total_count += amount;
        entry.x_count += amount;
    }

    pub(crate) fn is_paid(&self) -> bool {
        self.unpaid_shards.is_empty()
    }

    pub(crate) fn get_distinct_shards(&self) -> Vec<ManaCostShard> {
        self.unpaid_shards.keys().copied().collect()
    }

    pub(crate) fn get_unpaid_shards(&self, shard: ManaCostShard) -> i32 {
        self.unpaid_shards
            .get(&shard)
            .map(|s| s.total_count)
            .unwrap_or(0)
    }

    pub(crate) fn get_generic_mana_amount(&self) -> i32 {
        self.get_unpaid_shards(ManaCostShard::Generic)
    }

    pub(crate) fn has_any_kind(&self, kind: u16) -> bool {
        self.unpaid_shards
            .iter()
            .any(|(shard, count)| (shard.shard() & kind) != 0 && count.total_count > 0)
    }

    pub(crate) fn increase_generic_mana(&mut self, amount: i32) {
        self.increase_shard(ManaCostShard::Generic, amount);
    }

    pub(crate) fn increase_shard(&mut self, shard: ManaCostShard, amount: i32) {
        if amount <= 0 {
            return;
        }
        let entry = self.unpaid_shards.entry(shard).or_default();
        entry.total_count += amount;
    }

    pub(crate) fn decrease_shard(&mut self, shard: ManaCostShard, amount: i32) {
        if amount <= 0 {
            return;
        }

        if let Some(entry) = self.unpaid_shards.get_mut(&shard) {
            entry.total_count -= amount;
            if entry.total_count <= 0 {
                self.unpaid_shards.remove(&shard);
            }
            return;
        }

        // Java behavior: if mono-color shard is requested but absent, consume compatible
        // hybrid/phyrexian/colorless-hybrid shards before generic.
        if !shard.is_mono_color() && shard != ManaCostShard::Generic {
            return;
        }

        let mut remaining = amount;

        if shard.is_mono_color() {
            let color_mask = shard.shard() & ManaAtom::COLORS_SUPERPOSITION;
            let compatible: Vec<ManaCostShard> = self
                .unpaid_shards
                .keys()
                .copied()
                .filter(|s| {
                    (s.shard() & color_mask) != 0
                        && (s.is_multi_color()
                            || s.is_or_2_generic()
                            || s.is_colorless()
                            || s.is_phyrexian())
                })
                .collect();

            for s in compatible {
                if remaining <= 0 {
                    break;
                }
                let current = self.get_unpaid_shards(s);
                let take = current.min(remaining);
                if take > 0 {
                    self.decrease_shard(s, take);
                    remaining -= take;
                }
            }
        }

        if remaining > 0 {
            let generic = self.get_unpaid_shards(ManaCostShard::Generic);
            let take = generic.min(remaining);
            if take > 0 {
                self.decrease_shard(ManaCostShard::Generic, take);
            }
        }
    }

    pub(crate) fn get_shard_to_pay_by_priority(
        &self,
        payable_shards: &[ManaCostShard],
        possible_uses: u8,
    ) -> Option<ManaCostShard> {
        let mut choice: Option<ManaCostShard> = None;
        let mut priority = i32::MIN;

        for &to_pay in payable_shards {
            let p = get_pay_priority(to_pay, possible_uses);
            if p > priority {
                priority = p;
                choice = Some(to_pay);
            }
        }

        choice
    }

    pub(crate) fn try_pay_mana(
        &mut self,
        color_mask: u16,
        possible_uses: u8,
    ) -> Option<ManaCostShard> {
        let payable: Vec<ManaCostShard> = self
            .get_distinct_shards()
            .into_iter()
            .filter(|&s| can_pay_for_shard_with_color(s, color_mask))
            .collect();

        let chosen = self.get_shard_to_pay_by_priority(&payable, possible_uses)?;

        // Track X color payment before decreasing
        if let Some(sc) = self.unpaid_shards.get_mut(&chosen) {
            if sc.x_count > 0 {
                sc.x_count -= 1;
                let color_str = color_mask_to_short(color_mask);
                if !color_str.is_empty() {
                    *self.x_mana_cost_paid_by_color.entry(color_str).or_insert(0) += 1;
                }
            }
        }

        self.decrease_shard(chosen, 1);

        // Java behavior for 2/C: if paid using the generic route, add 1 generic back.
        if chosen.is_or_2_generic() && (chosen.color_mask() & possible_uses) == 0 {
            self.increase_generic_mana(1);
        }

        // Track sunburst
        self.sunburst_map |= color_mask;

        Some(chosen)
    }
}

fn color_mask_to_short(mask: u16) -> String {
    if (mask & ManaAtom::WHITE) != 0 {
        return "W".into();
    }
    if (mask & ManaAtom::BLUE) != 0 {
        return "U".into();
    }
    if (mask & ManaAtom::BLACK) != 0 {
        return "B".into();
    }
    if (mask & ManaAtom::RED) != 0 {
        return "R".into();
    }
    if (mask & ManaAtom::GREEN) != 0 {
        return "G".into();
    }
    String::new()
}

pub(crate) fn can_pay_for_shard_with_color(shard: ManaCostShard, color: u16) -> bool {
    if shard == ManaCostShard::Generic {
        return true;
    }
    if shard.is_or_2_generic() {
        return true;
    }

    let atoms = shard.shard();

    if (atoms & ManaAtom::COLORLESS) != 0 && color == ManaAtom::COLORLESS {
        return true;
    }

    let color_atoms = atoms & ManaAtom::COLORS_SUPERPOSITION;
    color_atoms != 0 && (color_atoms & color) != 0
}

pub(crate) fn get_pay_priority(shard: ManaCostShard, payment_color: u8) -> i32 {
    if shard == ManaCostShard::Generic {
        return 2;
    }

    if shard.is_mono_color() {
        if shard.is_or_2_generic() {
            return if (shard.color_mask() & payment_color) != 0 {
                9
            } else {
                1
            };
        }
        if shard.is_phyrexian() {
            return 8;
        }
        return 10;
    }

    5
}
