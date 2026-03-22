//! Dynamic card pool discovery for fuzz parity testing.
//!
//! Scans the `CardDatabase` and includes only cards whose abilities the Rust
//! engine can fully parse. As the engine implements more effects, the pool
//! automatically expands.

use std::collections::BTreeMap;

use forge_carddb::{CardDatabase, CardFace};
use forge_engine_core::ability::api_type::ApiType;
use forge_engine_core::ability::effects::IMPLEMENTED_API_TYPES;
use forge_engine_core::replacement::parse_replacement_effect;
use forge_engine_core::staticability::parse_static_ability;
use forge_engine_core::parsing::{keys, Params};
use forge_engine_core::trigger::parse_trigger;
use forge_foundation::color::Color;
use forge_foundation::CardSplitType;

/// A card in the fuzz pool with metadata for deck generation.
#[derive(Debug, Clone)]
pub struct PoolCard {
    pub name: String,
    pub colors: Vec<Color>,
    pub is_creature: bool,
    pub is_instant: bool,
    pub is_sorcery: bool,
    pub is_enchantment: bool,
    pub is_land: bool,
    pub cmc: i32,
}

/// Statistics about pool discovery.
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub total_scanned: usize,
    pub included: usize,
    pub excluded_multi_faced: usize,
    pub excluded_no_mana_cost: usize,
    pub excluded_unusable_type: usize,
    pub excluded_parse_failure: usize,
    pub excluded_unimplemented_effect: usize,
}

impl std::fmt::Display for PoolStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Pool: {}/{} cards supported ({:.1}%) [excluded: {} multi-faced, {} no cost, {} unusable type, {} parse failure, {} unimplemented effect]",
            self.included,
            self.total_scanned,
            if self.total_scanned > 0 {
                self.included as f64 / self.total_scanned as f64 * 100.0
            } else {
                0.0
            },
            self.excluded_multi_faced,
            self.excluded_no_mana_cost,
            self.excluded_unusable_type,
            self.excluded_parse_failure,
            self.excluded_unimplemented_effect,
        )
    }
}

/// The discovered card pool, partitioned for efficient deck generation.
pub struct CardPool {
    pub cards: Vec<PoolCard>,
}

const BASIC_LANDS: &[&str] = &["Plains", "Island", "Swamp", "Mountain", "Forest"];

impl CardPool {
    /// Discover all cards in the database that the Rust engine can fully handle.
    ///
    /// For each card, checks:
    /// 1. Single-faced only (no split/transform/meld/adventure/modal)
    /// 2. Has a mana cost (unless it's a basic land)
    /// 3. Is a usable type: Creature, Instant, Sorcery, Enchantment, or basic Land
    /// 4. All triggers, static abilities, and replacement effects parse successfully
    pub fn discover(db: &CardDatabase) -> (CardPool, PoolStats) {
        let mut cards = Vec::new();
        let mut stats = PoolStats {
            total_scanned: 0,
            included: 0,
            excluded_multi_faced: 0,
            excluded_no_mana_cost: 0,
            excluded_unusable_type: 0,
            excluded_parse_failure: 0,
            excluded_unimplemented_effect: 0,
        };

        // Always include basic lands
        for &land_name in BASIC_LANDS {
            let color = match land_name {
                "Plains" => vec![Color::White],
                "Island" => vec![Color::Blue],
                "Swamp" => vec![Color::Black],
                "Mountain" => vec![Color::Red],
                "Forest" => vec![Color::Green],
                _ => vec![],
            };
            cards.push(PoolCard {
                name: land_name.to_string(),
                colors: color,
                is_creature: false,
                is_instant: false,
                is_sorcery: false,
                is_enchantment: false,
                is_land: true,
                cmc: 0,
            });
        }

        for (_name, rules) in db.iter() {
            stats.total_scanned += 1;

            // 1. Skip multi-faced cards
            if rules.split_type != CardSplitType::None {
                stats.excluded_multi_faced += 1;
                continue;
            }

            let face = &rules.main_part;
            let type_line = &face.type_line;

            // Skip basic lands from the iteration (already added above)
            if type_line.is_land() && type_line.is_basic() {
                continue;
            }

            // 2. Must have a castable mana cost (unless basic land, handled above)
            if face.mana_cost.is_no_cost() {
                stats.excluded_no_mana_cost += 1;
                continue;
            }

            // 3. Must be a usable type
            let is_creature = type_line.is_creature();
            let is_instant = type_line.is_instant();
            let is_sorcery = type_line.is_sorcery();
            let is_enchantment = type_line.is_enchantment();

            if !is_creature && !is_instant && !is_sorcery && !is_enchantment {
                stats.excluded_unusable_type += 1;
                continue;
            }

            // 4. All abilities must parse successfully
            let mut all_parse = true;

            // Check triggers
            let mut next_id = 0u32;
            for raw in &face.triggers {
                if parse_trigger(raw, &mut next_id).is_none() {
                    all_parse = false;
                    break;
                }
            }

            // Check static abilities
            if all_parse {
                for raw in &face.static_abilities {
                    let prefixed = format!("S$ {}", raw);
                    if parse_static_ability(&prefixed).is_none() {
                        all_parse = false;
                        break;
                    }
                }
            }

            // Check replacement effects
            if all_parse {
                for raw in &face.replacements {
                    let prefixed = format!("R$ {}", raw);
                    if parse_replacement_effect(&prefixed).is_none() {
                        all_parse = false;
                        break;
                    }
                }
            }

            if !all_parse {
                stats.excluded_parse_failure += 1;
                continue;
            }

            // 5. All effect API types must be implemented
            if !check_abilities_implemented(face) {
                stats.excluded_unimplemented_effect += 1;
                continue;
            }

            let color_set = face.resolved_color();
            let colors: Vec<Color> = color_set.iter().collect();

            cards.push(PoolCard {
                name: face.name.clone(),
                colors,
                is_creature,
                is_instant,
                is_sorcery,
                is_enchantment,
                is_land: false,
                cmc: rules.cmc(),
            });

            stats.included += 1;
        }

        // Add basic lands to the included count
        stats.included += BASIC_LANDS.len();

        // Sort cards by name for deterministic iteration
        cards.sort_by(|a, b| a.name.cmp(&b.name));

        (CardPool { cards }, stats)
    }

    /// Get all non-land spells matching any of the given colors.
    /// Colorless spells are included for any color selection.
    pub fn spells_for_colors(&self, colors: &[Color]) -> Vec<&PoolCard> {
        self.cards
            .iter()
            .filter(|c| {
                if c.is_land {
                    return false;
                }
                // Include colorless spells for any deck
                if c.colors.is_empty() {
                    return true;
                }
                // Include if card's colors are a subset of chosen colors
                c.colors.iter().all(|cc| colors.contains(cc))
            })
            .collect()
    }

    /// Get basic lands for the given colors.
    pub fn lands_for_colors(&self, colors: &[Color]) -> Vec<&PoolCard> {
        self.cards
            .iter()
            .filter(|c| {
                c.is_land && !c.colors.is_empty() && c.colors.iter().any(|cc| colors.contains(cc))
            })
            .collect()
    }
}

/// Check that all effect API types referenced by a card's abilities (and their
/// sub-ability chains via SVars) are in the implemented set.
fn check_abilities_implemented(face: &CardFace) -> bool {
    // Check all spell abilities
    for raw in &face.abilities {
        if !check_ability_chain_implemented(raw, &face.svars, 0) {
            return false;
        }
    }

    // Check trigger execute SVars
    for raw in &face.triggers {
        let params = Params::from_raw(raw);
        if let Some(execute_svar) = params.get(keys::EXECUTE) {
            if let Some(svar_text) = face.svars.get(execute_svar) {
                if !check_ability_chain_implemented(svar_text, &face.svars, 0) {
                    return false;
                }
            }
        }
    }

    // Check replacement effect execute SVars
    for raw in &face.replacements {
        let params = Params::from_raw(raw);
        if let Some(execute_svar) = params.get(keys::EXECUTE) {
            if let Some(svar_text) = face.svars.get(execute_svar) {
                if !check_ability_chain_implemented(svar_text, &face.svars, 0) {
                    return false;
                }
            }
        }
    }

    true
}

/// Recursively validate that an ability string and its SubAbility chain
/// only reference implemented API types. Depth-limited to 10 to prevent
/// infinite loops from circular SVar references.
fn check_ability_chain_implemented(
    raw: &str,
    svars: &BTreeMap<String, String>,
    depth: usize,
) -> bool {
    if depth > 10 {
        return false;
    }

    let params = Params::from_raw(raw);

    // Extract API type from SP$, DB$, or AB$
    let api_type = params
        .get(keys::SP)
        .or_else(|| params.get(keys::DB))
        .or_else(|| params.get(keys::AB));

    if let Some(api_str) = api_type {
        match ApiType::smart_value_of(api_str) {
            Some(api) => {
                if !IMPLEMENTED_API_TYPES.contains(&api) {
                    return false;
                }
            }
            None => {
                return false;
            }
        }
    }

    // Follow SubAbility chain
    if let Some(sub_svar_name) = params.get(keys::SUB_ABILITY) {
        if let Some(sub_text) = svars.get(sub_svar_name) {
            if !check_ability_chain_implemented(sub_text, svars, depth + 1) {
                return false;
            }
        }
    }

    true
}
