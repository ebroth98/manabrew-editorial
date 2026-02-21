use forge_foundation::mana::ManaAtom;
use forge_foundation::{ManaCost, ZoneType};
use serde::{Deserialize, Serialize};

use crate::card::CardInstance;
use crate::cost::can_pay_ignoring_mana;
use crate::game::GameState;
use crate::ids::{CardId, PlayerId};

/// Tracks available mana for a player during a turn.
/// Simplified model: tracks count of each color + colorless + generic.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ManaPool {
    pub white: i32,
    pub blue: i32,
    pub black: i32,
    pub red: i32,
    pub green: i32,
    pub colorless: i32,
}

impl ManaPool {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, atom: u16, amount: i32) {
        match atom {
            ManaAtom::WHITE => self.white += amount,
            ManaAtom::BLUE => self.blue += amount,
            ManaAtom::BLACK => self.black += amount,
            ManaAtom::RED => self.red += amount,
            ManaAtom::GREEN => self.green += amount,
            ManaAtom::COLORLESS => self.colorless += amount,
            _ => self.colorless += amount,
        }
    }

    pub fn total(&self) -> i32 {
        self.white + self.blue + self.black + self.red + self.green + self.colorless
    }

    /// Remove `amount` of a given mana atom from the pool, saturating at 0.
    pub fn remove(&mut self, atom: u16, amount: i32) {
        match atom {
            ManaAtom::WHITE => self.white = (self.white - amount).max(0),
            ManaAtom::BLUE => self.blue = (self.blue - amount).max(0),
            ManaAtom::BLACK => self.black = (self.black - amount).max(0),
            ManaAtom::RED => self.red = (self.red - amount).max(0),
            ManaAtom::GREEN => self.green = (self.green - amount).max(0),
            _ => self.colorless = (self.colorless - amount).max(0),
        }
    }

    /// Returns true if the pool contains at least `amount` of the given atom.
    pub fn has_atom(&self, atom: u16, amount: i32) -> bool {
        let available = match atom {
            ManaAtom::WHITE => self.white,
            ManaAtom::BLUE => self.blue,
            ManaAtom::BLACK => self.black,
            ManaAtom::RED => self.red,
            ManaAtom::GREEN => self.green,
            _ => self.colorless,
        };
        available >= amount
    }

    pub fn empty(&mut self) {
        self.white = 0;
        self.blue = 0;
        self.black = 0;
        self.red = 0;
        self.green = 0;
        self.colorless = 0;
    }

    /// Try to pay a mana cost. Returns true if successful and deducts the mana.
    /// This is a simplified payment algorithm that handles colored and generic mana.
    pub fn can_pay(&self, cost: &forge_foundation::ManaCost) -> bool {
        let mut pool = self.clone();
        pool.try_pay(cost)
    }

    /// Returns true if the pool can pay `cost` plus `extra_generic` additional generic mana.
    /// Used for commander tax checks.
    pub fn can_pay_with_extra_generic(&self, cost: &forge_foundation::ManaCost, extra_generic: i32) -> bool {
        let mut pool = self.clone();
        if !pool.try_pay(cost) {
            return false;
        }
        pool.total() >= extra_generic
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
                continue; // X = 0 for now
            }

            let atoms = shard.shard();

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
                for &(bit, pool_ref) in &[
                    (ManaAtom::WHITE, self.white),
                    (ManaAtom::BLUE, self.blue),
                    (ManaAtom::BLACK, self.black),
                    (ManaAtom::RED, self.red),
                    (ManaAtom::GREEN, self.green),
                ] {
                    if (color_atoms & bit) != 0 && pool_ref > 0 {
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
                if self.colorless > 0 {
                    self.colorless -= 1;
                } else {
                    return false;
                }
            } else if shard.is_phyrexian() {
                // Phyrexian: pay with color or 2 life (we just try color for now)
                let color_atoms = atoms & ManaAtom::COLORS_SUPERPOSITION;
                if !self.pay_color(color_atoms) {
                    // Would need to pay life — handled at a higher level
                    return false;
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

    fn pay_color(&mut self, atoms: u16) -> bool {
        if (atoms & ManaAtom::WHITE) != 0 && self.white > 0 {
            self.white -= 1;
            return true;
        }
        if (atoms & ManaAtom::BLUE) != 0 && self.blue > 0 {
            self.blue -= 1;
            return true;
        }
        if (atoms & ManaAtom::BLACK) != 0 && self.black > 0 {
            self.black -= 1;
            return true;
        }
        if (atoms & ManaAtom::RED) != 0 && self.red > 0 {
            self.red -= 1;
            return true;
        }
        if (atoms & ManaAtom::GREEN) != 0 && self.green > 0 {
            self.green -= 1;
            return true;
        }
        false
    }

    fn pay_generic(&mut self, mut amount: i32) {
        // Pay with colorless first, then colors (WUBRG order)
        let pools = [
            &mut self.colorless,
            &mut self.white,
            &mut self.blue,
            &mut self.black,
            &mut self.red,
            &mut self.green,
        ];
        for pool in pools {
            let take = amount.min(*pool);
            *pool -= take;
            amount -= take;
            if amount == 0 {
                break;
            }
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

/// Auto-tap lands to produce the required mana.
pub fn auto_tap_lands(
    game: &mut GameState,
    pool: &mut ManaPool,
    player: PlayerId,
    cost: &ManaCost,
) {
    let lands: Vec<CardId> = game
        .cards_in_zone(ZoneType::Battlefield, player)
        .to_vec();

    // First, tap lands for colored requirements
    for shard in cost.shards() {
        if shard.is_x() {
            continue;
        }
        let atoms = shard.shard();
        let color_atoms = atoms & ManaAtom::COLORS_SUPERPOSITION;

        if color_atoms != 0 {
            for &land_id in &lands {
                let land = game.card(land_id);
                if land.is_land() && !land.tapped {
                    if let Some(atom) = basic_land_mana_atom(land) {
                        if (atom & color_atoms) != 0 {
                            game.tap(land_id);
                            pool.add(atom, 1);
                            break;
                        }
                    }
                }
            }
        }
    }

    // Then tap lands for generic cost
    let mut generic_needed = cost.generic_cost();
    if generic_needed > 0 {
        for &land_id in &lands {
            if generic_needed <= 0 {
                break;
            }
            let land = game.card(land_id);
            if land.is_land() && !land.tapped {
                if let Some(atom) = basic_land_mana_atom(land) {
                    game.tap(land_id);
                    pool.add(atom, 1);
                    generic_needed -= 1;
                }
            }
        }
    }
}

/// Auto-tap untapped lands to produce `needed` additional generic mana.
/// Used for paying commander tax on top of the regular cost.
pub fn auto_tap_lands_generic(
    game: &mut GameState,
    pool: &mut ManaPool,
    player: PlayerId,
    needed: i32,
) {
    let mut remaining = needed;
    let lands: Vec<CardId> = game.cards_in_zone(ZoneType::Battlefield, player).to_vec();
    for land_id in lands {
        if remaining <= 0 {
            break;
        }
        let eligible = {
            let land = game.card(land_id);
            land.is_land() && !land.tapped && basic_land_mana_atom(land).is_some()
        };
        if eligible {
            let atom = basic_land_mana_atom(game.card(land_id)).unwrap();
            game.tap(land_id);
            pool.add(atom, 1);
            remaining -= 1;
        }
    }
}

/// Calculate available mana from the current pool plus untapped lands and non-land mana sources.
pub fn calculate_available_mana(
    pool: &ManaPool,
    game: &GameState,
    player: PlayerId,
) -> ManaPool {
    let mut available = pool.clone();
    let battlefield = game.cards_in_zone(ZoneType::Battlefield, player);

    for &card_id in battlefield {
        let card = game.card(card_id);
        if card.is_land() && !card.tapped {
            if let Some(atom) = basic_land_mana_atom(card) {
                available.add(atom, 1);
            }
        } else {
            // Check non-land permanents for mana abilities
            for ab in &card.activated_abilities {
                if ab.is_mana_ability
                    && can_pay_ignoring_mana(&ab.cost, game, card_id, player)
                {
                    if let Some(produced) = ab.params.get("Produced") {
                        if let Some(atom) = mana_atom_from_produced(produced) {
                            available.add(atom, 1);
                        }
                    }
                }
            }
        }
    }

    available
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_foundation::ManaCost;

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
    fn pay_simple_cost() {
        let mut pool = ManaPool::new();
        pool.red = 1;

        let cost = ManaCost::parse("R");
        assert!(pool.can_pay(&cost));
        assert!(pool.try_pay(&cost));
        assert_eq!(pool.red, 0);
    }

    #[test]
    fn pay_generic_and_colored() {
        let mut pool = ManaPool::new();
        pool.green = 2;

        let cost = ManaCost::parse("1 G");
        assert!(pool.can_pay(&cost));
        assert!(pool.try_pay(&cost));
        assert_eq!(pool.green, 0); // 1 for G, 1 for generic
    }

    #[test]
    fn insufficient_mana() {
        let mut pool = ManaPool::new();
        pool.red = 1;

        let cost = ManaCost::parse("1 R R");
        assert!(!pool.can_pay(&cost));
    }

    #[test]
    fn empty_pool() {
        let mut pool = ManaPool::new();
        pool.white = 3;
        pool.empty();
        assert_eq!(pool.total(), 0);
    }
}
