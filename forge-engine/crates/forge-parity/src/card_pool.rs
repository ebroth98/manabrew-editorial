//! Dynamic card pool discovery for fuzz parity testing.
//!
//! Scans the `CardDatabase` and includes only cards whose abilities the Rust
//! engine can fully parse. As the engine implements more effects, the pool
//! automatically expands.

use std::collections::BTreeMap;
use std::path::Path;

use forge_carddb::{CardDatabase, CardFace};
use forge_engine_core::ability::api_type::ApiType;
use forge_engine_core::ability::effects::IMPLEMENTED_API_TYPES;
use forge_engine_core::parsing::{
    keys, ParamDiagnosticKind, Params, ParsedCardScript, ParsedParams, ScriptAbility,
    ScriptDiagnosticKind, ScriptLineKind, ScriptParamRecord, ScriptSVarValue,
    SemanticParamValueKind,
};
use forge_engine_core::replacement::parse_replacement_effect;
use forge_engine_core::staticability::parse_static_ability;
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
    pub param_diagnostics_missing_delimiter: usize,
    pub param_diagnostics_empty_key: usize,
    pub param_diagnostics_duplicate_key_same_value: usize,
    pub param_diagnostics_duplicate_key_different_value: usize,
    pub script_diagnostics_missing_colon: usize,
    pub script_diagnostics_empty_key: usize,
    pub script_diagnostics_unknown_field: usize,
    pub script_diagnostics_missing_ability_record: usize,
    pub script_diagnostics_missing_svar_name: usize,
    pub examples: Vec<ScriptScanExample>,
}

impl std::fmt::Display for PoolStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Pool: {}/{} cards supported ({:.1}%) [excluded: {} multi-faced, {} no cost, {} unusable type, {} parse failure, {} unimplemented effect; param diagnostics: {} missing delimiter, {} empty key, {} duplicate same-value key, {} duplicate different-value key; script diagnostics: {} missing colon, {} empty key, {} unknown field, {} missing ability record, {} missing SVar name]",
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
            self.param_diagnostics_missing_delimiter,
            self.param_diagnostics_empty_key,
            self.param_diagnostics_duplicate_key_same_value,
            self.param_diagnostics_duplicate_key_different_value,
            self.script_diagnostics_missing_colon,
            self.script_diagnostics_empty_key,
            self.script_diagnostics_unknown_field,
            self.script_diagnostics_missing_ability_record,
            self.script_diagnostics_missing_svar_name,
        )
    }
}

impl PoolStats {
    pub fn example_lines(&self) -> impl Iterator<Item = String> + '_ {
        self.examples.iter().map(format_example_line)
    }
}

/// The discovered card pool, partitioned for efficient deck generation.
pub struct CardPool {
    pub cards: Vec<PoolCard>,
}

#[derive(Debug, Clone, Default)]
pub struct ScriptScanStats {
    pub files: usize,
    pub lines: usize,
    pub abilities: usize,
    pub svars: usize,
    pub script_diagnostics_missing_colon: usize,
    pub script_diagnostics_empty_key: usize,
    pub script_diagnostics_unknown_field: usize,
    pub script_diagnostics_missing_ability_record: usize,
    pub script_diagnostics_missing_svar_name: usize,
    pub param_diagnostics_missing_delimiter: usize,
    pub param_diagnostics_empty_key: usize,
    pub param_diagnostics_duplicate_key_same_value: usize,
    pub param_diagnostics_duplicate_key_different_value: usize,
    pub semantic_values: usize,
    pub semantic_values_ability_record: usize,
    pub semantic_values_symbol: usize,
    pub semantic_values_boolean: usize,
    pub semantic_values_integer: usize,
    pub semantic_values_amount: usize,
    pub semantic_values_zone_list: usize,
    pub semantic_values_selector: usize,
    pub semantic_values_reference: usize,
    pub semantic_values_svar_reference: usize,
    pub semantic_values_cost: usize,
    pub semantic_values_text: usize,
    pub semantic_values_delimited_list: usize,
    pub semantic_values_transform: usize,
    pub semantic_values_comparison: usize,
    pub semantic_values_expression: usize,
    pub semantic_values_raw: usize,
    pub examples: Vec<ScriptScanExample>,
    pub semantic_raw_examples: Vec<ScriptScanExample>,
}

#[derive(Debug, Clone)]
pub struct ScriptScanExample {
    pub file: String,
    pub line_no: usize,
    pub kind: String,
    pub segment: String,
    pub key: Option<String>,
    pub previous_value: Option<String>,
    pub value: Option<String>,
}

impl std::fmt::Display for ScriptScanStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Script scan: {} files, {} lines, {} abilities, {} SVars [script diagnostics: {} missing colon, {} empty key, {} unknown field, {} missing ability record, {} missing SVar name; param diagnostics: {} missing delimiter, {} empty key, {} duplicate same-value key, {} duplicate different-value key; semantic values: {} total, {} ability record, {} symbol, {} boolean, {} integer, {} amount, {} zone list, {} selector, {} reference, {} SVar reference, {} cost, {} text, {} delimited list, {} transform, {} comparison, {} expression, {} raw]",
            self.files,
            self.lines,
            self.abilities,
            self.svars,
            self.script_diagnostics_missing_colon,
            self.script_diagnostics_empty_key,
            self.script_diagnostics_unknown_field,
            self.script_diagnostics_missing_ability_record,
            self.script_diagnostics_missing_svar_name,
            self.param_diagnostics_missing_delimiter,
            self.param_diagnostics_empty_key,
            self.param_diagnostics_duplicate_key_same_value,
            self.param_diagnostics_duplicate_key_different_value,
            self.semantic_values,
            self.semantic_values_ability_record,
            self.semantic_values_symbol,
            self.semantic_values_boolean,
            self.semantic_values_integer,
            self.semantic_values_amount,
            self.semantic_values_zone_list,
            self.semantic_values_selector,
            self.semantic_values_reference,
            self.semantic_values_svar_reference,
            self.semantic_values_cost,
            self.semantic_values_text,
            self.semantic_values_delimited_list,
            self.semantic_values_transform,
            self.semantic_values_comparison,
            self.semantic_values_expression,
            self.semantic_values_raw,
        )
    }
}

impl ScriptScanStats {
    pub fn example_lines(&self) -> impl Iterator<Item = String> + '_ {
        self.examples.iter().map(format_example_line)
    }

    pub fn semantic_raw_example_lines(&self) -> impl Iterator<Item = String> + '_ {
        self.semantic_raw_examples.iter().map(format_example_line)
    }
}

fn format_example_line(example: &ScriptScanExample) -> String {
    let key = example
        .key
        .as_ref()
        .map(|key| format!(" key={}", key))
        .unwrap_or_default();
    let values = match (&example.previous_value, &example.value) {
        (Some(previous), Some(value)) => format!(" [{} -> {}]", previous, value),
        (_, Some(value)) => format!(" [{}]", value),
        _ => String::new(),
    };
    format!(
        "{}:{}: {}{}{}: {}",
        example.file, example.line_no, example.kind, key, values, example.segment
    )
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
            param_diagnostics_missing_delimiter: 0,
            param_diagnostics_empty_key: 0,
            param_diagnostics_duplicate_key_same_value: 0,
            param_diagnostics_duplicate_key_different_value: 0,
            script_diagnostics_missing_colon: 0,
            script_diagnostics_empty_key: 0,
            script_diagnostics_unknown_field: 0,
            script_diagnostics_missing_ability_record: 0,
            script_diagnostics_missing_svar_name: 0,
            examples: Vec::new(),
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
            record_script_diagnostics(face, &mut stats);
            record_param_diagnostics(face, &mut stats);

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

pub fn scan_raw_script_diagnostics(cards_dir: &Path) -> ScriptScanStats {
    let mut stats = ScriptScanStats::default();
    scan_raw_script_diagnostics_inner(cards_dir, &mut stats);
    stats
}

fn scan_raw_script_diagnostics_inner(path: &Path, stats: &mut ScriptScanStats) {
    let Ok(entries) = std::fs::read_dir(path) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_raw_script_diagnostics_inner(&path, stats);
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("txt") {
            continue;
        }
        let Ok(raw) = std::fs::read_to_string(&path) else {
            continue;
        };
        stats.files += 1;
        let script = ParsedCardScript::parse(&raw);
        stats.lines += script.lines().len();
        stats.abilities += script.abilities().count();
        stats.svars += script
            .lines()
            .iter()
            .filter(|line| matches!(line.kind, ScriptLineKind::SVar(_)))
            .count();
        record_semantic_value_stats(&script, &path, stats);
        for diagnostic in script.diagnostics() {
            let kind = format!("{:?}", diagnostic.kind);
            match diagnostic.kind {
                ScriptDiagnosticKind::MissingColon => stats.script_diagnostics_missing_colon += 1,
                ScriptDiagnosticKind::EmptyKey => stats.script_diagnostics_empty_key += 1,
                ScriptDiagnosticKind::UnknownField => stats.script_diagnostics_unknown_field += 1,
                ScriptDiagnosticKind::MissingAbilityRecord => {
                    stats.script_diagnostics_missing_ability_record += 1;
                }
                ScriptDiagnosticKind::MissingSVarName => {
                    stats.script_diagnostics_missing_svar_name += 1
                }
                ScriptDiagnosticKind::Param(ParamDiagnosticKind::MissingDelimiter) => {
                    stats.param_diagnostics_missing_delimiter += 1;
                }
                ScriptDiagnosticKind::Param(ParamDiagnosticKind::EmptyKey) => {
                    stats.param_diagnostics_empty_key += 1;
                }
                ScriptDiagnosticKind::Param(ParamDiagnosticKind::DuplicateKeySameValue) => {
                    stats.param_diagnostics_duplicate_key_same_value += 1;
                }
                ScriptDiagnosticKind::Param(ParamDiagnosticKind::DuplicateKeyDifferentValue) => {
                    stats.param_diagnostics_duplicate_key_different_value += 1;
                }
            }
            if stats.examples.len() < 16 {
                stats.examples.push(ScriptScanExample {
                    file: path.display().to_string(),
                    line_no: diagnostic.line_no,
                    kind,
                    segment: diagnostic.segment.to_string(),
                    key: diagnostic.key.map(str::to_string),
                    previous_value: diagnostic.previous_value.map(str::to_string),
                    value: diagnostic.value.map(str::to_string),
                });
            }
        }
    }
}

fn record_semantic_value_stats(
    script: &ParsedCardScript<'_>,
    path: &Path,
    stats: &mut ScriptScanStats,
) {
    for line in script.lines() {
        match &line.kind {
            ScriptLineKind::Ability(ability) => {
                let ScriptAbility {
                    params: parsed_params,
                    ..
                } = ability;
                record_semantic_params(parsed_params.semantic_entries(), path, line.line_no, stats);
            }
            ScriptLineKind::Trigger(record)
            | ScriptLineKind::StaticAbility(record)
            | ScriptLineKind::Replacement(record) => {
                let ScriptParamRecord {
                    params: parsed_params,
                } = record;
                record_semantic_params(parsed_params.semantic_entries(), path, line.line_no, stats);
            }
            ScriptLineKind::SVar(svar) => match &svar.value_kind {
                ScriptSVarValue::Ability(ability) => {
                    let ScriptAbility {
                        params: parsed_params,
                        ..
                    } = ability;
                    record_semantic_params(
                        parsed_params.semantic_entries(),
                        path,
                        line.line_no,
                        stats,
                    );
                }
                ScriptSVarValue::Params(record) => {
                    let ScriptParamRecord {
                        params: parsed_params,
                    } = record;
                    record_semantic_params(
                        parsed_params.semantic_entries(),
                        path,
                        line.line_no,
                        stats,
                    );
                }
                ScriptSVarValue::Raw(_) => {}
            },
            _ => {}
        }
    }
}

fn record_semantic_params<'a>(
    params: impl Iterator<Item = forge_engine_core::parsing::SemanticParam<'a>>,
    path: &Path,
    line_no: usize,
    stats: &mut ScriptScanStats,
) {
    for param in params {
        stats.semantic_values += 1;
        let kind = param.value.kind();
        match kind {
            SemanticParamValueKind::AbilityRecord => stats.semantic_values_ability_record += 1,
            SemanticParamValueKind::Symbol => stats.semantic_values_symbol += 1,
            SemanticParamValueKind::Boolean => stats.semantic_values_boolean += 1,
            SemanticParamValueKind::Integer => stats.semantic_values_integer += 1,
            SemanticParamValueKind::Amount => stats.semantic_values_amount += 1,
            SemanticParamValueKind::ZoneList => stats.semantic_values_zone_list += 1,
            SemanticParamValueKind::Selector => stats.semantic_values_selector += 1,
            SemanticParamValueKind::Reference => stats.semantic_values_reference += 1,
            SemanticParamValueKind::SVarReference => stats.semantic_values_svar_reference += 1,
            SemanticParamValueKind::Cost => stats.semantic_values_cost += 1,
            SemanticParamValueKind::Text => stats.semantic_values_text += 1,
            SemanticParamValueKind::DelimitedList => stats.semantic_values_delimited_list += 1,
            SemanticParamValueKind::Transform => stats.semantic_values_transform += 1,
            SemanticParamValueKind::Comparison => stats.semantic_values_comparison += 1,
            SemanticParamValueKind::Expression => stats.semantic_values_expression += 1,
            SemanticParamValueKind::Raw => stats.semantic_values_raw += 1,
        }
        if kind == SemanticParamValueKind::Raw && stats.semantic_raw_examples.len() < 16 {
            stats.semantic_raw_examples.push(ScriptScanExample {
                file: path.display().to_string(),
                line_no,
                kind: "SemanticRaw".to_string(),
                segment: param.raw_value.to_string(),
                key: Some(param.key.to_string()),
                previous_value: None,
                value: None,
            });
        }
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

fn record_param_diagnostics(face: &CardFace, stats: &mut PoolStats) {
    for (idx, raw) in face.abilities.iter().enumerate() {
        record_raw_param_diagnostics(raw, &face.name, idx + 1, "Ability", stats);
    }
    for (idx, raw) in face.triggers.iter().enumerate() {
        record_raw_param_diagnostics(raw, &face.name, idx + 1, "Trigger", stats);
    }
    for (idx, raw) in face.replacements.iter().enumerate() {
        record_raw_param_diagnostics(raw, &face.name, idx + 1, "Replacement", stats);
    }
    for (idx, raw) in face.static_abilities.iter().enumerate() {
        let prefixed = format!("S$ {}", raw);
        record_raw_param_diagnostics(&prefixed, &face.name, idx + 1, "StaticAbility", stats);
    }
}

fn record_script_diagnostics(face: &CardFace, stats: &mut PoolStats) {
    let raw = synthesize_face_script(face);
    let script = ParsedCardScript::parse(&raw);
    for diagnostic in script.diagnostics() {
        match diagnostic.kind {
            ScriptDiagnosticKind::MissingColon => stats.script_diagnostics_missing_colon += 1,
            ScriptDiagnosticKind::EmptyKey => stats.script_diagnostics_empty_key += 1,
            ScriptDiagnosticKind::UnknownField => stats.script_diagnostics_unknown_field += 1,
            ScriptDiagnosticKind::MissingAbilityRecord => {
                stats.script_diagnostics_missing_ability_record += 1;
            }
            ScriptDiagnosticKind::MissingSVarName => {
                stats.script_diagnostics_missing_svar_name += 1
            }
            ScriptDiagnosticKind::Param(_) => {}
        }
    }
}

fn synthesize_face_script(face: &CardFace) -> String {
    let mut raw = String::new();
    push_script_line(&mut raw, "Name", &face.name);
    push_script_line(&mut raw, "ManaCost", &face.mana_cost.to_string());
    push_script_line(&mut raw, "Types", &face.type_line.to_string());
    if let (Some(power), Some(toughness)) = (face.int_power, face.int_toughness) {
        push_script_line(&mut raw, "PT", &format!("{}/{}", power, toughness));
    }
    for keyword in &face.keywords {
        push_script_line(&mut raw, "K", keyword);
    }
    for ability in &face.abilities {
        push_script_line(&mut raw, "A", ability);
    }
    for static_ability in &face.static_abilities {
        push_script_line(&mut raw, "S", static_ability);
    }
    for trigger in &face.triggers {
        push_script_line(&mut raw, "T", trigger);
    }
    for replacement in &face.replacements {
        push_script_line(&mut raw, "R", replacement);
    }
    for (name, value) in &face.svars {
        push_script_line(&mut raw, "SVar", &format!("{}:{}", name, value));
    }
    raw
}

fn push_script_line(raw: &mut String, key: &str, value: &str) {
    raw.push_str(key);
    raw.push(':');
    raw.push_str(value);
    raw.push('\n');
}

fn record_raw_param_diagnostics(
    raw: &str,
    card_name: &str,
    line_no: usize,
    source: &str,
    stats: &mut PoolStats,
) {
    let report = ParsedParams::parse_with_diagnostics(raw);
    for diagnostic in report.diagnostics {
        let kind = format!("{:?}", diagnostic.kind);
        match diagnostic.kind {
            ParamDiagnosticKind::MissingDelimiter => {
                stats.param_diagnostics_missing_delimiter += 1;
            }
            ParamDiagnosticKind::EmptyKey => {
                stats.param_diagnostics_empty_key += 1;
            }
            ParamDiagnosticKind::DuplicateKeySameValue => {
                stats.param_diagnostics_duplicate_key_same_value += 1;
            }
            ParamDiagnosticKind::DuplicateKeyDifferentValue => {
                stats.param_diagnostics_duplicate_key_different_value += 1;
            }
        }
        if stats.examples.len() < 16 {
            stats.examples.push(ScriptScanExample {
                file: format!("{} {}", card_name, source),
                line_no,
                kind,
                segment: diagnostic.segment.to_string(),
                key: diagnostic.key.map(str::to_string),
                previous_value: diagnostic.previous_value.map(str::to_string),
                value: diagnostic.value.map(str::to_string),
            });
        }
    }
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
