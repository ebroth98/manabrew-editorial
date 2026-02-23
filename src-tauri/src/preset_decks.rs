use forge_engine_core::game::GameState;
use forge_engine_core::ids::PlayerId;
use forge_foundation::ZoneType;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

use crate::card_db::{card_rules_to_instance, get_card_db};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardIdentity {
    pub name: String,
    pub set_code: String,
}

// ── Preset deck registry ───────────────────────────────────────────

/// Metadata for a preset deck, returned to the frontend via `get_preset_decks`.
///
/// Adding a new preset deck requires:
/// 1. Add a `const MY_DECK: &[(&str, usize, &str)]` below.
/// 2. Add a `PresetDeckInfo` entry to `list_preset_decks()`.
/// 3. Add the `"my_id"` arm to the `match` in `build_preset_decks()`.
/// 4. Add the id to `is_preset_id()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetDeckInfo {
    pub id: String,
    pub label: String,
    pub desc: String,
    /// Tailwind CSS text-color class used for the deck title in the UI.
    pub color: String,
}

/// Return the ordered list of all available preset decks.
///
/// This is the single source of truth consumed by the `get_preset_decks`
/// Tauri command — the frontend no longer hardcodes deck names.
pub fn list_preset_decks() -> Vec<PresetDeckInfo> {
    vec![
        PresetDeckInfo {
            id: "red_burn".into(),
            label: "Red Burn".into(),
            desc: "Bolts + Shocks + Ogres + Giants".into(),
            color: "text-red-500".into(),
        },
        PresetDeckInfo {
            id: "green_stompy".into(),
            label: "Green Stompy".into(),
            desc: "Giant Growth + Trample + Reach + Wurms".into(),
            color: "text-green-500".into(),
        },
        PresetDeckInfo {
            id: "white_aggro".into(),
            label: "White Aggro".into(),
            desc: "Savannah Lions + First Strike + Flying".into(),
            color: "text-yellow-500".into(),
        },
        PresetDeckInfo {
            id: "black_control".into(),
            label: "Black Control".into(),
            desc: "Doom Blade + Divination + Deathtouch".into(),
            color: "text-purple-500".into(),
        },
        PresetDeckInfo {
            id: "white_static".into(),
            label: "White Static".into(),
            desc: "Glorious Anthem + Indestructible + Layer effects".into(),
            color: "text-white".into(),
        },
        PresetDeckInfo {
            id: "zone_change".into(),
            label: "Zone Change".into(),
            desc: "Bounce + Sacrifice + Reanimate (ChangeZone / Sacrifice effects)".into(),
            color: "text-blue-400".into(),
        },
        PresetDeckInfo {
            id: "token_swarm".into(),
            label: "Token Swarm".into(),
            desc: "Raise the Alarm + Krenko's Command + token creation effects".into(),
            color: "text-orange-400".into(),
        },
        PresetDeckInfo {
            id: "sac_cost".into(),
            label: "Sacrifice Cost".into(),
            desc: "Village Rites + sacrifice-as-additional-cost mechanics".into(),
            color: "text-gray-400".into(),
        },
        PresetDeckInfo {
            id: "library_manipulation".into(),
            label: "Library Manipulation".into(),
            desc: "Scry + Surveil + Mill + Dig (Preordain, Ransack the Lab, Thought Scour)".into(),
            color: "text-cyan-400".into(),
        },
        PresetDeckInfo {
            id: "blue_control".into(),
            label: "Blue Control".into(),
            desc: "Counterspell + Mind Rot + Control Magic (Counter / Discard / ControlGain effects)".into(),
            color: "text-blue-500".into(),
        },
        PresetDeckInfo {
            id: "green_fight".into(),
            label: "Green Fight".into(),
            desc: "Prey Upon + Ram Through + big green creatures (Fight effects)".into(),
            color: "text-green-400".into(),
        },
        PresetDeckInfo {
            id: "showcase".into(),
            label: "Showcase".into(),
            desc: "All mechanics: Counter + Burn + Bounce + Token + Scry + Discard + Fight + Sac + Control + Mill".into(),
            color: "text-pink-400".into(),
        },
        PresetDeckInfo {
            id: "mass_effects".into(),
            label: "Mass Effects".into(),
            desc: "Wrath of God + Pyroclasm + Righteous Charge (DestroyAll / DamageAll / PumpAll effects)".into(),
            color: "text-amber-500".into(),
        },
        PresetDeckInfo {
            id: "charm_modal".into(),
            label: "Charm & Modal".into(),
            desc: "Izzet Charm + Grixis Charm — choose-your-mode modal spells (SP$ Charm)".into(),
            color: "text-violet-400".into(),
        },
    ]
}

/// Returns `true` if `id` matches a known preset deck.
pub fn is_preset_id(id: &str) -> bool {
    matches!(
        id,
        "red_burn"
            | "green_stompy"
            | "white_aggro"
            | "black_control"
            | "white_static"
            | "zone_change"
            | "token_swarm"
            | "sac_cost"
            | "library_manipulation"
            | "blue_control"
            | "green_fight"
            | "showcase"
            | "mass_effects"
            | "charm_modal"
    )
}

/// Build decks for both players given a preset id.
///
/// Panics if `preset_id` is not a known preset — callers should gate with
/// `is_preset_id` first.
pub fn build_preset_decks(game: &mut GameState, preset_id: &str, p0: PlayerId, p1: PlayerId) {
    match preset_id {
        "green_stompy" => {
            build_named_deck(game, p0, GREEN_STOMPY);
            build_named_deck(game, p1, RED_BURN);
        }
        "white_aggro" => {
            build_named_deck(game, p0, WHITE_AGGRO);
            build_named_deck(game, p1, BLACK_CONTROL);
        }
        "white_static" => {
            build_named_deck(game, p0, WHITE_STATIC);
            build_named_deck(game, p1, GREEN_STOMPY);
        }
        "zone_change" => {
            build_named_deck(game, p0, ZONE_CHANGE);
            build_named_deck(game, p1, GREEN_STOMPY);
        }
        "token_swarm" => {
            build_named_deck(game, p0, TOKEN_SWARM);
            build_named_deck(game, p1, RED_BURN);
        }
        "sac_cost" => {
            build_named_deck(game, p0, SAC_COST);
            build_named_deck(game, p1, GREEN_STOMPY);
        }
        "black_control" => {
            build_named_deck(game, p0, BLACK_CONTROL);
            build_named_deck(game, p1, WHITE_AGGRO);
        }
        "library_manipulation" => {
            build_named_deck(game, p0, LIBRARY_MANIPULATION);
            build_named_deck(game, p1, RED_BURN);
        }
        "blue_control" => {
            build_named_deck(game, p0, BLUE_CONTROL);
            build_named_deck(game, p1, RED_BURN);
        }
        "green_fight" => {
            build_named_deck(game, p0, GREEN_FIGHT);
            build_named_deck(game, p1, WHITE_AGGRO);
        }
        "showcase" => {
            build_named_deck(game, p0, SHOWCASE);
            build_named_deck(game, p1, random_ai_deck());
        }
        "mass_effects" => {
            build_named_deck(game, p0, MASS_EFFECTS);
            build_named_deck(game, p1, GREEN_STOMPY);
        }
        "charm_modal" => {
            build_named_deck(game, p0, CHARM_MODAL);
            build_named_deck(game, p1, RED_BURN);
        }
        _ => {
            // red_burn (default)
            build_named_deck(game, p0, RED_BURN);
            build_named_deck(game, p1, GREEN_STOMPY);
        }
    }
}

// ── Preset deck lists ──────────────────────────────────────────────
//
// Each entry is (card_name, count, set_code). The set_code is the Scryfall
// set abbreviation (e.g. "m21", "isd") used by the UI to fetch the specific
// printing artwork. An empty string means no set preference (Scryfall default).

const RED_BURN: &[(&str, usize, &str)] = &[
    ("Mountain", 17, "akh"),      // Amonkhet full-art basics
    ("Lightning Bolt", 4, "m11"), // Magic 2011 — iconic artwork
    ("Shock", 4, "m21"),          // Core Set 2021
    ("Gray Ogre", 3, "7ed"),      // Seventh Edition
    ("Hill Giant", 3, "m14"),     // Magic 2014
    ("Guttersnipe", 3, "rtr"),    // Return to Ravnica — original printing
];

const GREEN_STOMPY: &[(&str, usize, &str)] = &[
    ("Forest", 17, "akh"),            // Amonkhet full-art basics
    ("Giant Growth", 4, "m11"),       // Magic 2011
    ("Grizzly Bears", 3, "m12"),      // Magic 2012
    ("Centaur Courser", 2, "m14"),    // Magic 2014
    ("Garruk's Companion", 3, "m13"), // Magic 2013
    ("Giant Spider", 2, "m14"),       // Magic 2014
    ("Wall of Ice", 2, "7ed"),        // Seventh Edition
    ("Craw Wurm", 2, "m11"),          // Magic 2011
];

const WHITE_AGGRO: &[(&str, usize, &str)] = &[
    ("Plains", 17, "akh"),        // Amonkhet full-art basics
    ("Savannah Lions", 4, "m10"), // Magic 2010
    ("White Knight", 3, "m10"),   // Magic 2010
    ("Serra Angel", 3, "m21"),    // Core Set 2021
    ("Soul Warden", 3, "m11"),    // Magic 2011
];

/// Exercises PR #26 (static abilities / layer system) and PR #27 (replacement effects).
/// Glorious Anthem tests the YouCtrl alias fix and layer 7c anthem stacking.
/// Darksteel Myr tests the indestructible Destroy replacement effect.
/// Honor of the Pure tests color-based filtering (White creatures).
const WHITE_STATIC: &[(&str, usize, &str)] = &[
    ("Plains", 16, "akh"),           // Amonkhet full-art basics
    ("Glorious Anthem", 3, "m14"),   // Magic 2014
    ("Honor of the Pure", 3, "m11"), // Magic 2011
    ("Darksteel Myr", 3, "som"),     // Scars of Mirrodin — original printing
    ("Savannah Lions", 4, "m10"),    // Magic 2010
    ("White Knight", 4, "m10"),      // Magic 2010
    ("Serra Angel", 3, "m21"),       // Core Set 2021
];

const BLACK_CONTROL: &[(&str, usize, &str)] = &[
    ("Swamp", 13, "akh"), // Amonkhet full-art basics
    ("Island", 4, "akh"),
    ("Doom Blade", 4, "m14"),        // Magic 2014
    ("Divination", 2, "m14"),        // Magic 2014
    ("Typhoid Rats", 3, "isd"),      // Innistrad — original printing
    ("Vampire Nighthawk", 3, "m13"), // Magic 2013
    ("Mulldrifter", 2, "mma"),       // Modern Masters
];

/// Exercises issue #13: ChangeZone (bounce / reanimate), Sacrifice, and SacrificeAll effects.
/// Unsummon / Boomerang test Battlefield→Hand ChangeZone.
/// Raise Dead tests Graveyard→Hand ChangeZone.
/// Diabolic Edict tests targeted Sacrifice (opponent's creature).
/// Innocent Blood tests Sacrifice applied to all players (Defined$ Player).
const ZONE_CHANGE: &[(&str, usize, &str)] = &[
    ("Swamp", 12, "akh"),
    ("Island", 4, "akh"),
    ("Unsummon", 4, "m14"),      // Magic 2014 — ChangeZone: Battlefield → Hand
    ("Boomerang", 2, "3ed"),     // Revised Edition — ChangeZone: any permanent
    ("Raise Dead", 13, "m13"),   // Magic 2013 — ChangeZone: Graveyard → Hand
    ("Diabolic Edict", 3, "te"), // Tempest — targeted Sacrifice
    ("Innocent Blood", 3, "od"), // Odyssey — each player sacrifices
    ("Typhoid Rats", 4, "isd"),
    ("Vampire Nighthawk", 3, "m13"),
    ("Doom Blade", 2, "m14"),
];

/// Exercises sacrifice-as-additional-cost (CostSacrifice).
/// Village Rites tests `Cost$ B Sac<1/Creature>` — sacrifice a creature to draw two cards.
/// Cheap creatures provide sacrifice fodder; Raise Dead lets you recur them.
const SAC_COST: &[(&str, usize, &str)] = &[
    ("Swamp", 17, "akh"),
    ("Severed Strands", 10, "grn"), // Guilds of Ravnica — original printing
    ("Typhoid Rats", 4, "isd"),     // Cheap sacrifice fodder (1/1 deathtouch)
    ("Vampire Nighthawk", 3, "m13"), // Evasive threat
    ("Doom Blade", 3, "m14"),       // Removal
    ("Raise Dead", 3, "m13"),       // Recur creatures from graveyard
];

/// Exercises issue #14: Token creation (SP$ Token) and CopyPermanent effects.
/// Raise the Alarm / Krenko's Command / Dragon Fodder create tokens directly.
/// Goblin creatures give the AI something to fight against.
const TOKEN_SWARM: &[(&str, usize, &str)] = &[
    ("Plains", 8, "akh"),
    ("Mountain", 8, "akh"),
    ("Raise the Alarm", 4, "m15"),  // Magic 2015 — 2× 1/1 white Soldier
    ("Krenko's Command", 4, "m13"), // Magic 2013 — 2× 1/1 red Goblin
    ("Dragon Fodder", 4, "ktk"),    // Khans of Tarkir — 2× 1/1 red Goblin
    ("Savannah Lions", 4, "m10"),
    ("Lightning Bolt", 4, "m11"),
    ("Shock", 4, "m21"),
];

/// Exercises issue #15: library manipulation — Scry, Surveil, Mill, Dig, RearrangeTopOfLibrary.
/// Preordain tests Scry 2 + Draw.
/// Ponder tests RearrangeTopOfLibrary (3 cards) + Draw.
/// Thought Scour tests Mill 2 (targeted) + Draw.
/// Ransack the Lab tests Dig 3 / take 1 to hand / rest to graveyard.
/// Taigam's Scheming tests Surveil 5.
/// Notion Rain tests Surveil 2 + Draw 2 + DealDamage to self.
const LIBRARY_MANIPULATION: &[(&str, usize, &str)] = &[
    ("Island", 16, "akh"),
    ("Swamp", 4, "akh"),
    ("Preordain", 4, "m11"),         // Magic 2011 — Scry 2, draw
    ("Ponder", 4, "m12"),            // Magic 2012 — rearrange top 3, draw
    ("Thought Scour", 4, "dka"),     // Dark Ascension — Mill 2, draw
    ("Ransack the Lab", 4, "mh1"),   // Modern Horizons — Dig 3
    ("Taigam's Scheming", 2, "ktk"), // Khans of Tarkir — Surveil 5
    ("Notion Rain", 2, "grn"),       // Guilds of Ravnica — Surveil 2, draw 2
    ("Divination", 4, "m14"),        // Draw 2
    ("Mulldrifter", 4, "mma"),       // Modern Masters — Flying ETB draw 2
    ("Typhoid Rats", 4, "isd"),      // 1/1 Deathtouch
    ("Doom Blade", 4, "m14"),        // Destroy non-black creature
    ("Vampire Nighthawk", 4, "m13"), // 2/3 Flying Deathtouch Lifelink
];

/// Exercises issue #16: Counter (Counterspell, Cancel), Discard (Mind Rot),
/// and ControlGain (Control Magic) effects.
const BLUE_CONTROL: &[(&str, usize, &str)] = &[
    ("Island", 17, "akh"),
    ("Counterspell", 4, "mma"),  // Modern Masters — counter target spell
    ("Cancel", 4, "m14"),        // Magic 2014 — counter target spell (3 mana)
    ("Mind Rot", 4, "m14"),      // Magic 2014 — target player discards 2
    ("Control Magic", 3, "mma"), // Modern Masters — gain control of creature
    ("Mulldrifter", 3, "mma"),   // 2/2 Flying; ETB draw 2
    ("Divination", 4, "m14"),    // Draw 2
    ("Wall of Ice", 4, "7ed"),   // Blocker
    ("Sea Serpent", 4, "m10"),   // Magic 2010 — big blue creature
];

/// Exercises issue #16: Fight effects (Prey Upon, Ram Through).
/// Big green creatures provide good fight targets.
const GREEN_FIGHT: &[(&str, usize, &str)] = &[
    ("Forest", 17, "akh"),
    ("Prey Upon", 4, "isd"),          // Innistrad — creature fights target
    ("Ram Through", 4, "iko"),        // Ikoria — similar fight spell
    ("Garruk's Companion", 4, "m13"), // 3/2 Trample
    ("Centaur Courser", 4, "m14"),    // 3/3
    ("Giant Spider", 3, "m14"),       // 2/4 Reach
    ("Craw Wurm", 3, "m11"),          // 6/4
    ("Giant Growth", 4, "m11"),       // Pump before fighting
    ("Grizzly Bears", 4, "m12"),      // 2/2
];

/// Exercises all implemented mechanics in one deck (Grixis: U/B/R).
/// - Burn: Lightning Bolt, Shock (DealDamage)
/// - Counter: Counterspell (Counter target spell)
/// - Bounce: Unsummon (ChangeZone Battlefield→Hand)
/// - Token: Dragon Fodder (2× 1/1 Goblin)
/// - Scry/Draw: Preordain (Scry 2 + Draw)
/// - Surveil/Draw: Notion Rain (Surveil 2, Draw 2)
/// - Dig: Ransack the Lab (Dig 3)
/// - Discard: Mind Rot (target player discards 2)
/// - Mill: Thought Scour (Mill 2 + Draw)
/// - Reanimate: Raise Dead (GY→Hand ChangeZone)
/// - SacrificeCost: Severed Strands (Sac creature, destroy target)
/// - ControlGain: Control Magic (gain control of creature)
/// - Fight: Prey Upon (creature you control fights target)
/// - Keywords: Typhoid Rats (Deathtouch), Vampire Nighthawk (Flying/Deathtouch/Lifelink)
/// - ETB draw: Mulldrifter (Flying, ETB draw 2)
const SHOWCASE: &[(&str, usize, &str)] = &[
    ("Island", 7, "akh"),
    ("Swamp", 6, "akh"),
    ("Mountain", 4, "akh"),
    // Burn
    ("Lightning Bolt", 2, "m11"),
    ("Shock", 2, "m21"),
    // Counter
    ("Counterspell", 2, "mma"),
    // Bounce
    ("Unsummon", 2, "m14"),
    // Token
    ("Dragon Fodder", 2, "ktk"),
    // Scry + Draw
    ("Preordain", 2, "m11"),
    // Surveil + Draw
    ("Notion Rain", 2, "grn"),
    // Dig
    ("Ransack the Lab", 2, "mh1"),
    // Discard
    ("Mind Rot", 2, "m14"),
    // Mill + Draw
    ("Thought Scour", 2, "dka"),
    // Reanimate
    ("Raise Dead", 2, "m13"),
    // Sacrifice cost
    ("Severed Strands", 2, "grn"),
    // Control gain
    ("Control Magic", 1, "mma"),
    // Fight
    ("Prey Upon", 2, "isd"),
    // Keywords / threats
    ("Typhoid Rats", 2, "isd"),
    ("Vampire Nighthawk", 2, "m13"),
    ("Mulldrifter", 2, "mma"),
];

/// Exercises issue #17: mass/board-wide effects.
/// - Wrath of God: DestroyAll Creature (board wipe, respects Indestructible)
/// - Pyroclasm: DamageAll 2 to each creature
/// - Righteous Charge: PumpAll +2/+2 to all your creatures until EOT
/// - Rising Miasma: PumpAll -2/-2 to all creatures until EOT
/// - Savannah Lions / White Knight / Serra Angel: creatures to interact with mass effects
const MASS_EFFECTS: &[(&str, usize, &str)] = &[
    ("Plains", 18, "akh"),
    ("Wrath of God", 4, "m14"),     // Magic 2014
    ("Pyroclasm", 4, "m11"),        // Magic 2011
    ("Righteous Charge", 4, "m13"), // Magic 2013
    ("Rising Miasma", 4, "m15"),    // Magic 2015
    ("Savannah Lions", 4, "m10"),
    ("White Knight", 4, "m10"),
    ("Serra Angel", 4, "m21"),
    ("Darksteel Myr", 4, "som"), // Scars of Mirrodin — survives Wrath
];

/// Exercises issue #18: modal spells (SP$ Charm).
/// - Izzet Charm: choose from Counter / DealDamage / Draw+Discard
/// - Grixis Charm: choose from Bounce / Pump -4/-4 / PumpAll +2/+0
/// - Guttersnipe: burns opponent for each instant/sorcery cast
/// - Delver of Secrets: evasive threat that pairs with instants
const CHARM_MODAL: &[(&str, usize, &str)] = &[
    ("Island", 8, "m11"),
    ("Mountain", 6, "m11"),
    ("Swamp", 4, "m11"),
    ("Izzet Charm", 4, "rtr"),  // Return to Ravnica — original printing
    ("Grixis Charm", 4, "con"), // Conflux — original printing
    ("Lightning Bolt", 4, "m11"),
    ("Counterspell", 3, "mma"),
    ("Guttersnipe", 4, "rtr"),        // Return to Ravnica
    ("Delver of Secrets", 24, "isd"), // Innistrad — original printing (DFC)
    ("Gray Ogre", 3, "7ed"),
];

/// All AI-eligible deck lists, used for random opponent selection.
const AI_DECK_OPTIONS: &[&[(&str, usize, &str)]] = &[
    RED_BURN,
    GREEN_STOMPY,
    WHITE_AGGRO,
    BLACK_CONTROL,
    WHITE_STATIC,
    ZONE_CHANGE,
    TOKEN_SWARM,
    SAC_COST,
    LIBRARY_MANIPULATION,
    BLUE_CONTROL,
    GREEN_FIGHT,
];

/// Pick a random deck from all AI-eligible presets.
fn random_ai_deck() -> &'static [(&'static str, usize, &'static str)] {
    let mut rng = rand::thread_rng();
    AI_DECK_OPTIONS
        .choose(&mut rng)
        .copied()
        .unwrap_or(RED_BURN)
}

// ── Deck builders ──────────────────────────────────────────────────

/// Build the default AI opponent deck (Red Burn) for a single player.
///
/// Used when the human plays a custom deck so the AI still gets a deck.
pub fn build_ai_opponent(game: &mut GameState, owner: PlayerId) {
    build_named_deck(game, owner, RED_BURN);
}

/// Build a preset deck from a (name, count, set_code) list, loading each card
/// definition from the global CardDatabase (parsed from the Forge card scripts).
/// The set_code is stored on each card instance so the UI can request the
/// specific printing from Scryfall. An empty set_code means no preference.
fn build_named_deck(game: &mut GameState, owner: PlayerId, deck: &[(&str, usize, &str)]) {
    let db = get_card_db();
    for (name, count, set_code) in deck {
        match db.get_by_card_name(name) {
            Some(rules) => {
                for _ in 0..*count {
                    let mut card = card_rules_to_instance(rules, owner);
                    if !set_code.is_empty() {
                        card.set_code = Some(set_code.to_string());
                    }
                    let id = game.create_card(card);
                    game.move_card(id, ZoneType::Library, owner);
                }
            }
            None => eprintln!("[deck] Unknown card '{}' — skipped", name),
        }
    }
}

/// Build a custom deck for `owner` from a list of card identities (one per
/// copy), loading each definition from the global CardDatabase.
/// Unrecognised names are skipped with a log message.
pub fn build_custom_deck(game: &mut GameState, owner: PlayerId, identities: &[CardIdentity]) {
    let db = get_card_db();
    for identity in identities {
        let name = &identity.name;
        match db.get_by_card_name(name) {
            Some(rules) => {
                let mut card = card_rules_to_instance(rules, owner);
                if !identity.set_code.is_empty() {
                    card.set_code = Some(identity.set_code.clone());
                }
                let id = game.create_card(card);
                game.move_card(id, ZoneType::Library, owner);
            }
            None => eprintln!("[custom_deck] Unknown card '{}' — skipped", name),
        }
    }
}
