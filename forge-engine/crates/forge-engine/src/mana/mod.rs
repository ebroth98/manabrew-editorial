use forge_foundation::mana::ManaAtom;
use forge_foundation::{ManaCost, ZoneType};
use serde::{Deserialize, Serialize};

use crate::card::CardInstance;
use crate::cost::{can_pay_ignoring_mana, CostPart};
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
    /// When set, caps total producible mana for playability checks.
    /// Used by `calculate_available_mana` to prevent multi-color sources
    /// (dual lands, Command Tower) from being counted as multiple mana.
    #[serde(skip)]
    pub total_sources: Option<i32>,
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
        // If total_sources is set (availability estimate), check that total cost
        // doesn't exceed real producible mana. This prevents dual lands from
        // being counted as producing 2+ mana when they can only produce 1.
        if let Some(max) = self.total_sources {
            if cost.cmc() > max {
                return false;
            }
        }
        let mut pool = self.clone();
        pool.try_pay(cost)
    }

    /// Returns true if the pool can pay `cost` plus `extra_generic` additional generic mana.
    /// Used for commander tax checks.
    pub fn can_pay_with_extra_generic(
        &self,
        cost: &forge_foundation::ManaCost,
        extra_generic: i32,
    ) -> bool {
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

    pub fn pay_color(&mut self, atoms: u16) -> bool {
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

fn mana_atom_to_color_name(atom: u16) -> Option<&'static str> {
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
fn all_basic_subtype_atoms(card: &CardInstance) -> Vec<u16> {
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

/// Auto-tap lands to produce the required mana.
/// Respects mana already in the pool — only taps additional lands for the deficit.
/// Uses each land's actual mana abilities (including non-basic and dual lands).
///
/// Prefers single-color sources over multi-color ones for colored shards, so
/// dual lands are preserved for the colors that truly need them.
pub fn auto_tap_lands(
    game: &mut GameState,
    pool: &mut ManaPool,
    player: PlayerId,
    cost: &ManaCost,
) -> Vec<CardId> {
    let all_lands: Vec<CardId> = game.cards_in_zone(ZoneType::Battlefield, player).to_vec();

    // Pre-compute atoms for each untapped land, sorted by specificity (fewest colors first).
    // This ensures single-color lands are tapped before dual/multi-color lands.
    let mut candidates: Vec<(CardId, Vec<u16>)> = Vec::new();
    let mut tapped_lands: Vec<CardId> = Vec::new();
    for &land_id in &all_lands {
        let land = game.card(land_id);
        if !land.is_land() || land.tapped {
            continue;
        }
        let mut atoms = land_mana_atoms(land);
        if atoms.is_empty() {
            if let Some(a) = basic_land_mana_atom(land) {
                atoms.push(a);
            }
        }
        if !atoms.is_empty() {
            candidates.push((land_id, atoms));
        }
    }
    // Sort: fewest colors first (basic lands before duals before Command Tower)
    candidates.sort_by_key(|(_, atoms)| atoms.len());

    // Track what the pool can already cover (virtual copy, not mutated on real pool)
    let mut pool_tracker = pool.clone();

    // First, tap lands for colored requirements, using pool mana first
    for shard in cost.shards() {
        if shard.is_x() {
            continue;
        }
        let atoms = shard.shard();
        let color_atoms = atoms & ManaAtom::COLORS_SUPERPOSITION;

        if color_atoms != 0 {
            // Try to satisfy from existing pool first
            if pool_tracker.pay_color(color_atoms) {
                continue;
            }
            // Find the most specialized untapped land that produces this color
            if let Some(idx) = candidates
                .iter()
                .position(|(_, land_atoms)| land_atoms.iter().any(|&a| (a & color_atoms) != 0))
            {
                let (land_id, land_atoms) = candidates.remove(idx);
                let atom = *land_atoms
                    .iter()
                    .find(|&&a| (a & color_atoms) != 0)
                    .unwrap();
                game.tap(land_id);
                pool.add(atom, 1);
                tapped_lands.push(land_id);
            }
        }
    }

    // Then tap lands for generic cost, subtracting what the pool tracker still has
    let generic_needed = (cost.generic_cost() - pool_tracker.total()).max(0);
    if generic_needed > 0 {
        let mut remaining = generic_needed;
        // For generic, tap from the FRONT of the sorted list (zone/entry order within
        // same atom count), matching Java's ComputerUtilMana which iterates mana sources
        // in zone order (FIFO: first-entered lands used first for generic costs).
        while remaining > 0 && !candidates.is_empty() {
            let (land_id, land_atoms) = candidates.remove(0);
            if let Some(&atom) = land_atoms.first() {
                game.tap(land_id);
                pool.add(atom, 1);
                remaining -= 1;
                tapped_lands.push(land_id);
            }
        }
    }

    tapped_lands
}

/// Auto-tap untapped lands to produce `needed` additional generic mana.
/// Used for paying commander tax on top of the regular cost.
/// Respects mana already in the pool — only taps for the deficit.
/// Uses each land's actual mana abilities (including non-basic and dual lands).
///
/// Prefers multi-color lands for generic costs, preserving single-color
/// lands for future colored requirements.
pub fn auto_tap_lands_generic(
    game: &mut GameState,
    pool: &mut ManaPool,
    player: PlayerId,
    needed: i32,
) -> Vec<CardId> {
    // Subtract what the pool can already cover
    let deficit = (needed - pool.total()).max(0);
    if deficit <= 0 {
        return Vec::new();
    }
    let mut remaining = deficit;
    let lands: Vec<CardId> = game.cards_in_zone(ZoneType::Battlefield, player).to_vec();

    // Collect untapped lands with atoms, sorted by MOST colors first
    // (multi-color lands tapped first for generic, preserving basics for colored)
    let mut candidates: Vec<(CardId, u16)> = Vec::new();
    let mut tapped_lands: Vec<CardId> = Vec::new();
    for land_id in lands {
        let land = game.card(land_id);
        if !land.is_land() || land.tapped {
            continue;
        }
        let atoms = land_mana_atoms(land);
        let atom = if atoms.is_empty() {
            basic_land_mana_atom(land)
        } else {
            atoms.into_iter().next()
        };
        if let Some(a) = atom {
            candidates.push((land_id, a));
        }
    }

    // Tap from the end — no sort needed, just iterate
    for (land_id, atom) in candidates {
        if remaining <= 0 {
            break;
        }
        game.tap(land_id);
        pool.add(atom, 1);
        remaining -= 1;
        tapped_lands.push(land_id);
    }

    tapped_lands
}

/// Calculate available mana from the current pool plus untapped lands and non-land mana sources.
///
/// Colors are tracked OPTIMISTICALLY: each source adds 1 per color it could produce,
/// so that color-matching checks (`can_pay` for colored shards) work correctly.
/// However, `total_sources` is set to the actual number of mana sources, so the
/// total mana check in `can_pay` prevents dual/multi-color lands from being
/// double-counted (e.g. Breeding Pool counts as 1 mana, not 2).
pub fn calculate_available_mana(pool: &ManaPool, game: &GameState, player: PlayerId) -> ManaPool {
    let mut available = pool.clone();
    let battlefield = game.cards_in_zone(ZoneType::Battlefield, player);

    // Track actual number of mana sources (each can produce exactly 1 mana)
    let mut source_count: i32 = 0;

    for &card_id in battlefield {
        let card = game.card(card_id);
        if card.tapped {
            continue;
        }

        // Check for mana abilities on this permanent
        let mana_abilities: Vec<_> = card
            .activated_abilities
            .iter()
            .filter(|ab| {
                ab.is_mana_ability
                    && !ab.cost.parts.iter().any(|p| matches!(p, CostPart::Mana(_)))
                    && can_pay_ignoring_mana(&ab.cost, game, card_id, player)
            })
            .collect();

        if mana_abilities.is_empty() {
            // Fallback for lands without explicit parsed mana abilities.
            // This handles non-basic lands with basic land subtypes (e.g. Breeding Pool
            // typed "Land Forest Island" — produces G or U from subtype, not AB$ Mana).
            // Also handles basic lands from the Forge CLI or other sources.
            if card.is_land() {
                let subtype_atoms = all_basic_subtype_atoms(card);
                if !subtype_atoms.is_empty() {
                    // Multi-subtype dual lands: add all producing colors optimistically.
                    // The total_sources cap prevents double-counting.
                    for atom in subtype_atoms {
                        available.add(atom, 1);
                    }
                    source_count += 1;
                } else if let Some(atom) = basic_land_mana_atom(card) {
                    available.add(atom, 1);
                    source_count += 1;
                }
            }
            continue;
        }

        // Add 1 mana for each distinct color this source can produce (optimistic for colors).
        // The total_sources cap ensures the total mana count stays correct.
        let mut added_any = false;
        let mut added_atoms: Vec<u16> = Vec::new();
        for ab in &mana_abilities {
            if let Some(produced) = ab.params.get("Produced") {
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
                                available.add(atom, 1);
                                added_atoms.push(atom);
                            }
                        }
                        added_any = true;
                    }
                } else {
                    for atom in produced_to_atoms(produced, &card.chosen_colors) {
                        if !added_atoms.contains(&atom) {
                            available.add(atom, 1);
                            added_atoms.push(atom);
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
                        available.add(atom, 1);
                        added_atoms.push(atom);
                        added_any = true;
                    }
                }
            } else if let Some(atom) = basic_land_mana_atom(card) {
                // Name-based fallback for basic lands named "Forest" etc.
                available.add(atom, 1);
                added_any = true;
            }
        }
        if added_any {
            // Each productive source contributes exactly 1 activation (tap = 1 mana)
            source_count += 1;
        }
    }

    // Set total_sources so can_pay enforces the real total mana cap
    available.total_sources = Some(pool.total() + source_count);

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
