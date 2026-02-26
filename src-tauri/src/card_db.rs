use std::path::PathBuf;
use std::sync::OnceLock;

use forge_carddb::{CardDatabase, CardRules};
use forge_engine_core::ability::activated::parse_activated_ability;
use forge_engine_core::card::{CardInstance, CardOtherPart};
use forge_engine_core::ids::{CardId, PlayerId};
use forge_engine_core::replacement::parse_replacement_effect;
use forge_engine_core::staticability::parse_static_ability;
use forge_engine_core::trigger::parse_trigger;

static CARD_DB: OnceLock<CardDatabase> = OnceLock::new();
static TOKEN_DB: OnceLock<CardDatabase> = OnceLock::new();

/// Returns the path to the Forge card scripts directory.
/// Checks the CARDS_DIR env var first; falls back to the path adjacent
/// to this crate's manifest (works during `cargo tauri dev`).
fn cards_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("CARDS_DIR") {
        PathBuf::from(dir)
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../forge/forge-gui/res/cardsfolder")
    }
}

/// Returns the path to the Forge token scripts directory.
fn token_scripts_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("TOKEN_SCRIPTS_DIR") {
        PathBuf::from(dir)
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../forge/forge-gui/res/tokenscripts")
    }
}

/// Returns the global CardDatabase, loading it on first call.
///
/// Loads all card scripts from the Forge cardsfolder — the same source of
/// truth used by the Java Forge engine. No card data is hardcoded here.
pub fn get_card_db() -> &'static CardDatabase {
    CARD_DB.get_or_init(|| {
        let dir = cards_dir();
        eprintln!("[carddb] Loading cards from {:?} …", dir);
        let (db, result) = CardDatabase::load_from_directory(&dir);
        eprintln!(
            "[carddb] Loaded {} cards ({} failed)",
            result.loaded, result.failed
        );
        if !result.errors.is_empty() {
            for (file, err) in result.errors.iter().take(10) {
                eprintln!("[carddb]   parse error in {}: {}", file, err);
            }
        }
        db
    })
}

/// Returns the global token-script database, loading it on first call.
///
/// Token scripts live in `forge/forge-gui/res/tokenscripts/` and are keyed
/// by their filename stem (e.g. "r_1_1_goblin" for `r_1_1_goblin.txt`).
pub fn get_token_db() -> &'static CardDatabase {
    TOKEN_DB.get_or_init(|| {
        let dir = token_scripts_dir();
        eprintln!("[tokendb] Loading token scripts from {:?} …", dir);
        let (db, result) = CardDatabase::load_from_directory(&dir);
        eprintln!(
            "[tokendb] Loaded {} token scripts ({} failed)",
            result.loaded, result.failed
        );
        db
    })
}

/// Convert an immutable `CardRules` definition into a mutable `CardInstance`
/// ready to be inserted into a game.
///
/// Mirrors Java's `CardFactory.readCard()` + `readCardFace()`:
/// - copies base stats (name, mana cost, type line, color, P/T, keywords,
///   raw ability strings)
/// - parses `T:` trigger strings into `Trigger` structs via `parse_trigger()`
/// - copies SVars into `card.svars` (must be present before triggers fire)
///
/// The `CardId` inside the returned instance is a placeholder (0); the real
/// ID is assigned by `game.create_card()`.
pub fn card_rules_to_instance(rules: &CardRules, owner: PlayerId) -> CardInstance {
    let face = &rules.main_part;
    let mut next_trigger_id = 0u32;

    // Parse each raw trigger string (T: line) into a Trigger struct.
    // Unknown/unsupported trigger modes return None and are skipped with a warning.
    let mut triggers: Vec<_> = Vec::new();
    // Track which T: lines used SpellCastOrCopy so we can duplicate as SpellCopied.
    let mut spell_cast_or_copy_raw: Vec<String> = Vec::new();
    for raw in &face.triggers {
        let result = parse_trigger(raw, &mut next_trigger_id);
        if let Some(trig) = result {
            triggers.push(trig);
            // If the raw trigger used SpellCastOrCopy mode, record it for duplication
            if raw.contains("Mode$ SpellCastOrCopy") {
                spell_cast_or_copy_raw.push(raw.clone());
            }
        } else {
            eprintln!(
                "[carddb] Unsupported trigger on '{}': {:?} — skipped",
                face.name, raw
            );
        }
    }
    // Duplicate SpellCastOrCopy triggers as SpellCopied variants (for Magecraft).
    for raw in &spell_cast_or_copy_raw {
        let converted = raw.replace("Mode$ SpellCastOrCopy", "Mode$ SpellCopied");
        if let Some(trig) = parse_trigger(&converted, &mut next_trigger_id) {
            triggers.push(trig);
        }
    }

    // Auto-generate triggers from keywords that imply triggers.
    // Mirrors Java's CardFactoryUtil.addTriggerAbility() which auto-creates
    // trigger objects for keywords like Prowess.
    auto_generate_keyword_triggers(&face.keywords, &mut triggers, &mut next_trigger_id);

    let mut card = CardInstance::new(
        CardId(0),
        face.name.clone(),
        owner,
        face.type_line.clone(),
        face.mana_cost.clone(),
        face.resolved_color(),
        face.int_power,
        face.int_toughness,
        face.keywords.clone(),
        face.abilities.clone(),
    );

    // Auto-generate intrinsic mana abilities for basic land subtypes.
    // Mirrors Java's CardFactoryUtil.addIntrinsicAbilities(): lands with
    // basic subtypes (Plains, Island, Swamp, Mountain, Forest) implicitly
    // have "{T}: Add {color}" even without an explicit A: line.
    // This handles basic lands (Forest), shock lands (Breeding Pool = Forest Island),
    // and any other land with basic subtypes.
    const SUBTYPE_MANA: &[(&str, &str, &str)] = &[
        ("Plains", "W", "Add {W}."),
        ("Island", "U", "Add {U}."),
        ("Swamp", "B", "Add {B}."),
        ("Mountain", "R", "Add {R}."),
        ("Forest", "G", "Add {G}."),
    ];
    for &(subtype, letter, desc) in SUBTYPE_MANA {
        if card.type_line.has_subtype(subtype) {
            // Check if an existing mana ability already produces this color
            let already_produces = card.activated_abilities.iter().any(|ab| {
                ab.is_mana_ability && ab.params.get("Produced").map_or(false, |p| p == letter)
            });
            if !already_produces {
                let raw = format!(
                    "AB$ Mana | Cost$ T | Produced$ {} | SpellDescription$ {}",
                    letter, desc
                );
                let idx = card.abilities.len();
                card.abilities.push(raw.clone());
                if let Some(ab) = parse_activated_ability(&raw, idx) {
                    card.activated_abilities.push(ab);
                }
            }
        }
    }

    card.triggers = triggers;
    // SVars must be copied so trigger Execute$ references resolve correctly.
    card.svars = face.svars.clone();

    // Inject SVars for auto-generated keyword triggers (e.g. Prowess pump).
    if face.keywords.iter().any(|k| k == "Prowess") && !card.svars.contains_key("TrigProwess") {
        card.svars.insert(
            "TrigProwess".to_string(),
            "DB$ Pump | Defined$ Self | NumAtt$ 1 | NumDef$ 1".to_string(),
        );
    }

    // Load static abilities from S: lines (stored separately from A: ability lines
    // in Forge card scripts).  The parser strips the "S:" key and stores only the
    // value, so we re-prefix with "S$ " to match parse_static_ability's format.
    for raw in &face.static_abilities {
        let prefixed = format!("S$ {}", raw);
        if let Some(sa) = parse_static_ability(&prefixed) {
            card.static_abilities.push(sa);
        }
    }

    // Load replacement effects from R: lines in the same way.
    for raw in &face.replacements {
        let prefixed = format!("R$ {}", raw);
        if let Some(re) = parse_replacement_effect(&prefixed) {
            card.replacement_effects.push(re);
        }
    }

    // Populate alternate face for double-faced cards (Transform, Meld, Modal DFC).
    if rules.split_type.is_dual_faced() {
        if let Some(ref back_face) = rules.other_part {
            let mut back_trigger_id = 0u32;
            let back_triggers: Vec<_> = back_face
                .triggers
                .iter()
                .filter_map(|raw| parse_trigger(raw, &mut back_trigger_id))
                .collect();

            card.other_part = Some(CardOtherPart {
                name: back_face.name.clone(),
                type_line: back_face.type_line.clone(),
                mana_cost: back_face.mana_cost.clone(),
                color: back_face.resolved_color(),
                base_power: back_face.int_power,
                base_toughness: back_face.int_toughness,
                keywords: back_face.keywords.clone(),
                abilities: back_face.abilities.clone(),
                triggers: back_triggers,
                svars: back_face.svars.clone(),
            });
        }
    }
    card
}

/// Auto-generate triggers from keyword abilities that imply triggered effects.
/// Mirrors Java's CardFactoryUtil.addTriggerAbility() for K: keyword lines.
fn auto_generate_keyword_triggers(
    keywords: &[String],
    triggers: &mut Vec<forge_engine_core::trigger::Trigger>,
    next_id: &mut u32,
) {
    for kw in keywords {
        if kw == "Prowess" {
            // Prowess: "Whenever you cast a noncreature spell, this creature gets +1/+1 until EOT."
            let raw = "Mode$ SpellCast | ValidCard$ Card.nonCreature | ValidActivatingPlayer$ You | Execute$ TrigProwess | TriggerZones$ Battlefield | TriggerDescription$ Prowess";
            if let Some(mut trig) = parse_trigger(raw, next_id) {
                trig.execute = "TrigProwess".to_string();
                triggers.push(trig);
            }
        }
    }
}
