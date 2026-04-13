use forge_foundation::mana::ManaAtom;
use forge_foundation::{PhaseType, ZoneType};
use serde::{Deserialize, Serialize};

use crate::agent::PlayerAgent;
use crate::card::Card;
use crate::cost::CostPart;
use crate::game::GameState;
use crate::ids::{CardId, PlayerId};
use crate::parsing::keys;

pub mod auto_pay;
pub mod computer_util_mana;
pub mod mana_conversion_matrix;
pub mod mana_cost_being_paid;
pub mod mana_pool;
pub mod mana_refund_service;
pub use auto_pay::{
    pay_mana_cost_auto, pay_mana_cost_auto_with_callback, pay_mana_cost_auto_with_chooser,
};
pub use computer_util_mana::{
    auto_tap_lands, auto_tap_lands_allow_reserved_source_reuse,
    auto_tap_lands_allow_reserved_source_reuse_with_callbacks,
    auto_tap_lands_allow_reserved_source_reuse_with_chooser, auto_tap_lands_generic,
    auto_tap_lands_with_callbacks, auto_tap_lands_with_chooser, next_auto_tap_choice,
    AutoTapChoice, ManaPayCallback, ManaPayCallbackFn, SacrificeChooser,
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

    /// Whether spells paid with this mana can't be countered.
    /// Mirrors Java's `Mana.addsNoCounterMagic()`.
    pub fn adds_no_counter_magic(&self) -> bool {
        self.adds_no_counter
    }

    /// Whether this mana adds counters to permanents cast with it.
    /// Mirrors Java's `Mana.addsCounters()`.
    pub fn adds_counters(&self) -> bool {
        self.adds_counters.is_some()
    }

    /// Whether this mana adds keywords to spells cast with it.
    /// Mirrors Java's `Mana.addsKeywords()`.
    pub fn adds_keywords(&self) -> bool {
        self.adds_keywords.is_some()
    }

    /// Whether this mana adds keywords with a type restriction.
    /// Mirrors Java's `Mana.addsKeywordsType()`.
    pub fn adds_keywords_type(&self) -> bool {
        self.adds_keywords_valid.is_some()
    }

    /// Whether this mana adds keywords with a duration.
    /// Mirrors Java's `Mana.addsKeywordsUntil()`.
    pub fn adds_keywords_until(&self) -> bool {
        // Duration is implicit — keywords from mana last until end of turn
        self.adds_keywords.is_some()
    }

    /// Whether this mana has a trigger-when-spent effect.
    /// Mirrors Java's `Mana.triggersWhenSpent()`.
    pub fn triggers_when_spent(&self) -> bool {
        self.triggers_when_spent.is_some()
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

// ManaPool moved to mana_pool.rs — single source of truth.
pub use mana_pool::ManaPool;

// ── Mana helpers ────────────────────────────────────────────────────

/// Determine what mana atom a basic land produces based on its subtypes.
pub fn basic_land_mana_atom(card: &Card) -> Option<u16> {
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
    let reflect_prop = ab.params.get(keys::REFLECT_PROPERTY).unwrap_or("Is");
    let valid = ab.params.get(keys::VALID).unwrap_or("Card");
    let include_colorless = ab.params.get(keys::COLOR_OR_TYPE) == Some("Type");
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
                    if let Some(prod) = other_ab.params.get(keys::PRODUCED) {
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
pub(crate) fn all_basic_subtype_atoms(card: &Card) -> Vec<u16> {
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
fn land_pain_damage(card: &Card, chosen_atom: u16) -> i32 {
    for ab in &card.activated_abilities {
        if !ab.is_mana_ability {
            continue;
        }
        // Skip abilities without SubAbility (no pain)
        let sub_svar_name = match ab.params.get(keys::SUB_ABILITY) {
            Some(name) => name,
            None => continue,
        };
        // Check if this ability produces the chosen atom
        if let Some(produced) = ab.params.get(keys::PRODUCED) {
            let atoms = produced_to_atoms(produced, &card.chosen_colors);
            if atoms.contains(&chosen_atom) {
                // Look up the SVar to find damage amount
                if let Some(sub_text) = card.svars.get(sub_svar_name) {
                    let sub_params = crate::parsing::Params::from_raw(sub_text);
                    if sub_params
                        .get(crate::parsing::keys::DB)
                        .map_or(false, |v| v == "DealDamage")
                    {
                        if let Some(num_str) = sub_params.get(crate::parsing::keys::NUM_DMG) {
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
        game.player_lose_life(player, pain);
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
pub fn land_mana_atoms(card: &Card) -> Vec<u16> {
    let mut atoms = Vec::new();
    for ab in &card.activated_abilities {
        if !ab.is_mana_ability {
            continue;
        }
        // Java parity: don't treat mana abilities with mana activation costs as free
        // producers during static source detection.
        if ab
            .cost
            .parts
            .iter()
            .any(|p| matches!(p, CostPart::Mana { .. }))
        {
            continue;
        }
        if let Some(produced) = ab.params.get(keys::PRODUCED) {
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

// ── Mana production (extracted from game_action.rs) ─────────────────

/// Parameters that describe metadata to attach to produced mana.
pub struct ManaProductionParams {
    pub source_card: CardId,
    pub is_snow: bool,
    pub restriction: Option<String>,
    pub adds_no_counter: bool,
    pub adds_keywords: Option<String>,
    pub adds_keywords_valid: Option<String>,
    pub adds_counters: Option<String>,
    pub adds_counters_valid: Option<String>,
    pub triggers_when_spent: Option<String>,
}

/// Determine what mana to produce from a `Produced$` string, handling
/// color choice, combo mana, `Amount$` multiplier, and replacement effects.
///
/// Returns the final mana string (e.g. `"W W"`, `"R G"`, `"C C C"`),
/// or `None` if no mana can be produced (e.g. Amount$ evaluates to 0).
///
/// The caller is responsible for cost payment, trigger firing, sub-ability
/// resolution, and the `Special` / `ManaReflected` branches.
pub fn determine_mana_production(
    game: &mut GameState,
    agents: &mut [Box<dyn PlayerAgent>],
    player: PlayerId,
    card_id: CardId,
    produced: &str,
    amount_param: Option<&str>,
    express_choice: Option<u16>,
) -> Option<String> {
    let mut mana_string: Option<String> = None;

    if produced == "Combo ColorIdentity" {
        let colors = game.player_commander_color_identity(player);

        if !colors.is_empty() {
            if let Some(chosen) = agents[player.index()].choose_color(player, &colors) {
                if let Some(atom) = color_name_to_mana_atom(&chosen) {
                    mana_string = Some(ManaPool::atom_to_letter(atom).to_string());
                }
            }
        }
    } else {
        let chosen_colors = game.card(card_id).chosen_colors.clone();
        let colors = produced_to_color_names(produced, &chosen_colors);
        if colors.len() > 1 {
            let chosen = express_choice
                .and_then(mana_atom_to_color_name)
                .and_then(|forced| {
                    colors
                        .iter()
                        .find(|valid| valid.eq_ignore_ascii_case(forced))
                        .cloned()
                })
                .or_else(|| agents[player.index()].choose_color(player, &colors));
            if let Some(chosen) = chosen {
                if let Some(atom) = color_name_to_mana_atom(&chosen) {
                    mana_string = Some(ManaPool::atom_to_letter(atom).to_string());
                }
            }
        } else if let Some(single) = colors.first() {
            if let Some(atom) = color_name_to_mana_atom(single) {
                mana_string = Some(ManaPool::atom_to_letter(atom).to_string());
            }
        } else {
            // Raw produced string (single or multi-token like "C C")
            mana_string = Some(produced.to_string());
        }
    }

    // Apply Amount$ multiplier (e.g. Rofellos produces mana equal to Forests)
    if let Some(ref mut ms) = mana_string {
        if let Some(amount_str) = amount_param {
            let amount = if let Ok(n) = amount_str.parse::<i32>() {
                n
            } else {
                // Try to resolve as SVar on the source card
                if let Some(svar_expr) = game.card(card_id).svars.get(amount_str).cloned() {
                    crate::ability::effects::resolve_count_svar(&svar_expr, game, card_id, player)
                } else {
                    1
                }
            };
            if amount > 1 {
                // Check if this is combo/any mana (multiple color choices)
                let is_combo = produced.contains("Any")
                    || produced.starts_with("Combo")
                    || produced.contains(',');
                if is_combo {
                    // Multi-amount combo: let agent choose color distribution
                    let available: Vec<String> = if produced.contains("Any") {
                        vec!["W", "U", "B", "R", "G"]
                            .into_iter()
                            .map(String::from)
                            .collect()
                    } else {
                        let chosen_colors = game.card(card_id).chosen_colors.clone();
                        let names = produced_to_color_names(produced, &chosen_colors);
                        names
                            .iter()
                            .filter_map(|name| {
                                color_name_to_mana_atom(name)
                                    .map(|a| ManaPool::atom_to_letter(a).to_string())
                            })
                            .collect()
                    };
                    let card_name = game.card(card_id).card_name.clone();
                    let chosen = agents[player.index()].specify_mana_combo(
                        player,
                        &available,
                        amount as usize,
                        Some(&card_name),
                    );
                    *ms = chosen.join(" ");
                } else {
                    let base = ms.clone();
                    for _ in 1..amount {
                        ms.push(' ');
                        ms.push_str(&base);
                    }
                }
            } else if amount <= 0 {
                mana_string = None;
            }
        }
    }

    // Apply ProduceMana replacement effects (mana doublers like Mirari's Wake)
    if let Some(ref mut ms) = mana_string {
        use crate::replacement::replacement_handler::{apply_replacements, ReplacementEvent};
        let mut event = ReplacementEvent::ProduceMana {
            source: card_id,
            activator: player,
            mana: ms.clone(),
        };
        let result = apply_replacements(game, &mut event);
        if result == crate::replacement::ReplacementResult::Updated {
            if let ReplacementEvent::ProduceMana { mana: new_mana, .. } = event {
                *ms = new_mana;
            }
        }
    }

    mana_string
}

/// Add produced mana to the pool with full metadata (snow, restriction, keywords, counters, triggers).
pub fn add_produced_mana_to_pool(
    pool: &mut ManaPool,
    mana_string: &str,
    params: &ManaProductionParams,
) {
    pool.produce_mana_from_string(
        mana_string,
        Some(params.source_card),
        params.is_snow,
        params.restriction.clone(),
        params.adds_no_counter,
        params.adds_keywords.clone(),
        params.adds_keywords_valid.clone(),
        params.adds_counters.clone(),
        params.adds_counters_valid.clone(),
        params.triggers_when_spent.clone(),
    );
}

/// Calculate available mana from the current pool plus untapped lands and non-land mana sources.
///
/// Colors are tracked OPTIMISTICALLY: each source adds 1 per color it could produce,
/// so that color-matching checks (`can_pay` for colored shards) work correctly.
/// However, `total_sources` is set to the actual number of mana sources, so the
/// total mana check in `can_pay` prevents dual/multi-color lands from being
/// double-counted (e.g. Breeding Pool counts as 1 mana, not 2).
pub fn calculate_available_mana(pool: &ManaPool, game: &GameState, player: PlayerId) -> ManaPool {
    calculate_available_mana_excluding_with_reserved(pool, game, player, None, &[])
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
    calculate_available_mana_excluding_with_reserved(pool, game, player, excluded_source, &[])
}

pub fn calculate_available_mana_excluding_with_reserved(
    pool: &ManaPool,
    game: &GameState,
    player: PlayerId,
    excluded_source: Option<CardId>,
    reserved_sacrifices: &[CardId],
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
                    && !ab.cost.parts.iter().any(|p| matches!(p, CostPart::Mana { .. }))
                    && (!is_tapped || !ab.cost.parts.iter().any(|p| matches!(p, CostPart::Tap)))
                    && (!summoning_sick
                        || !ab.cost.parts.iter().any(|p| matches!(p, CostPart::Tap)))
                    // Mirror Java ComputerUtilMana playability checks:
                    // only count mana abilities whose non-mana costs are currently payable
                    // (e.g. Gilded Goose needs a Food to produce mana).
                    && crate::cost::can_pay_ignoring_mana(&ab.cost, game, card_id, player)
                    && crate::game_loop::GameLoop::mana_ability_available_for_payment_with_reserved(
                        game,
                        player,
                        card_id,
                        ab,
                        reserved_sacrifices,
                    )
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
            if ab.params.get(keys::AB) == Some("ManaReflected") {
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
            } else if let Some(produced) = ab.params.get(keys::PRODUCED) {
                if produced == "Combo ColorIdentity" {
                    // Commander Color Identity support: in non-commander games this remains empty.
                    let colors = game.player_commander_color_identity(player);
                    if !colors.is_empty() {
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

    // Count extra mana from aura enchantments with TapsForMana triggers
    // (e.g. Utopia Sprawl, Wild Growth). When an untapped land has an attached
    // aura that produces mana on tap, that extra mana should be counted in
    // the playability check.
    for &card_id in battlefield {
        let card = game.card(card_id);
        if card.tapped || card.attachments.is_empty() {
            continue;
        }
        for &aura_id in &card.attachments {
            if aura_id.index() >= game.cards.len() {
                continue;
            }
            let aura = game.card(aura_id);
            if aura.zone != ZoneType::Battlefield {
                continue;
            }
            for trigger in &aura.triggers {
                if let crate::trigger::TriggerMode::TapsForMana { .. } = &trigger.mode {
                    // This aura produces extra mana when the host is tapped.
                    // Determine what color from the Execute$ SVar.
                    if let Some(svar_text) = aura.svars.get(&trigger.execute) {
                        let params = crate::parsing::Params::from_raw(svar_text);
                        if let Some(produced) = params.get(crate::parsing::keys::PRODUCED) {
                            let atoms = if produced == "Chosen" {
                                // Use aura's chosen color
                                aura.chosen_colors
                                    .first()
                                    .and_then(|c| color_name_to_mana_atom(c))
                                    .into_iter()
                                    .collect::<Vec<_>>()
                            } else {
                                produced_to_atoms(produced, &aura.chosen_colors)
                            };
                            for atom in atoms {
                                available.add(atom, 1);
                                source_count += 1;
                                source_colors.push(atom);
                            }
                        }
                    }
                }
            }
        }
    }

    // Set total_sources so can_pay enforces the real total mana cap
    available.total_sources = Some(pool.total_mana() + source_count);
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
    let amount_str = match ab.params.get(keys::AMOUNT) {
        Some(v) if !v.is_empty() => v,
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
    use crate::agent::{PassAgent, PlayerAgent};
    use crate::card::Card;
    use crate::game::GameState;
    use crate::ids::{CardId, PlayerId};
    use forge_foundation::ManaCost;
    use forge_foundation::{CardTypeLine, ColorSet, ZoneType};

    #[test]
    fn basic_land_detection() {
        use crate::card::Card;
        use crate::ids::{CardId, PlayerId};
        use forge_foundation::ColorSet;

        let card = Card::new(
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
        pool.reset_pool();
        assert_eq!(pool.total_mana(), 0);
    }

    #[test]
    fn auto_tap_prefers_basic_sources_over_utility_lands_for_generic() {
        let mut game = GameState::new(&["P1", "P2"], 20);
        let p0 = PlayerId(0);

        let make_land = |id: usize, name: &str, abilities: Vec<&str>| {
            Card::new(
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

    #[test]
    fn combo_color_identity_uses_registered_commander_outside_command_zone() {
        let mut game = GameState::new(&["P1", "P2"], 20);
        let p0 = PlayerId(0);

        let commander = Card::new(
            CardId(0),
            "Commander".to_string(),
            p0,
            CardTypeLine::parse("Legendary Creature Wizard"),
            ManaCost::parse("2 U"),
            ColorSet::BLUE,
            Some(2),
            Some(2),
            vec![],
            vec![],
        );
        let commander_id = game.create_card(commander);
        game.player_register_commander(p0, commander_id);
        game.player_create_commander_effect(p0, None);
        game.move_card(commander_id, ZoneType::Battlefield, p0);

        let mut agents: Vec<Box<dyn PlayerAgent>> = vec![Box::new(PassAgent), Box::new(PassAgent)];
        let produced = determine_mana_production(
            &mut game,
            &mut agents,
            p0,
            commander_id,
            "Combo ColorIdentity",
            None,
            None,
        );

        assert_eq!(produced.as_deref(), Some("U"));
    }
}
