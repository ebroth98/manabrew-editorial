use forge_engine_core::game::GameState;
use forge_engine_core::ids::PlayerId;
use forge_foundation::ZoneType;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

use crate::card_db::{card_rules_to_instance, get_card_db};

// ── Preset deck registry ───────────────────────────────────────────

/// Metadata for a preset deck, returned to the frontend via `get_preset_decks`.
///
/// Adding a new preset deck requires:
/// 1. Add a `const MY_DECK: &[(&str, usize)]` below.
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
        _ => {
            // red_burn (default)
            build_named_deck(game, p0, RED_BURN);
            build_named_deck(game, p1, GREEN_STOMPY);
        }
    }
}

// ── Preset deck lists ──────────────────────────────────────────────
//
// Each entry is (card_name, count). Card definitions come exclusively from
// the Forge card scripts in forge/forge-gui/res/cardsfolder/ — no stats are
// hardcoded here.

const RED_BURN: &[(&str, usize)] = &[
    ("Mountain", 17),
    ("Lightning Bolt", 4),
    ("Shock", 4),
    ("Gray Ogre", 3),
    ("Hill Giant", 3),
    ("Guttersnipe", 3),
];

const GREEN_STOMPY: &[(&str, usize)] = &[
    ("Forest", 17),
    ("Giant Growth", 4),
    ("Grizzly Bears", 3),
    ("Centaur Courser", 2),
    ("Garruk's Companion", 3),
    ("Giant Spider", 2),
    ("Wall of Ice", 2),
    ("Craw Wurm", 2),
];

const WHITE_AGGRO: &[(&str, usize)] = &[
    ("Plains", 17),
    ("Savannah Lions", 4),
    ("White Knight", 3),
    ("Serra Angel", 3),
    ("Soul Warden", 3),
];

/// Exercises PR #26 (static abilities / layer system) and PR #27 (replacement effects).
/// Glorious Anthem tests the YouCtrl alias fix and layer 7c anthem stacking.
/// Darksteel Myr tests the indestructible Destroy replacement effect.
/// Honor of the Pure tests color-based filtering (White creatures).
const WHITE_STATIC: &[(&str, usize)] = &[
    ("Plains", 16),
    ("Glorious Anthem", 3),
    ("Honor of the Pure", 3),
    ("Darksteel Myr", 3),
    ("Savannah Lions", 4),
    ("White Knight", 4),
    ("Serra Angel", 3),
];

const BLACK_CONTROL: &[(&str, usize)] = &[
    ("Swamp", 13),
    ("Island", 4),
    ("Doom Blade", 4),
    ("Divination", 2),
    ("Typhoid Rats", 3),
    ("Vampire Nighthawk", 3),
    ("Mulldrifter", 2),
];

/// Exercises issue #13: ChangeZone (bounce / reanimate), Sacrifice, and SacrificeAll effects.
/// Unsummon / Boomerang test Battlefield→Hand ChangeZone.
/// Raise Dead tests Graveyard→Hand ChangeZone.
/// Diabolic Edict tests targeted Sacrifice (opponent's creature).
/// Innocent Blood tests Sacrifice applied to all players (Defined$ Player).
const ZONE_CHANGE: &[(&str, usize)] = &[
    ("Swamp", 12),
    ("Island", 4),
    ("Unsummon", 4),       // ChangeZone: Battlefield → Hand
    ("Boomerang", 2),      // ChangeZone: Battlefield → Hand (any permanent)
    ("Raise Dead", 13),    // ChangeZone: Graveyard → Hand
    ("Diabolic Edict", 3), // Sacrifice: target player sacrifices a creature
    ("Innocent Blood", 3), // Sacrifice: each player sacrifices a creature
    ("Typhoid Rats", 4),
    ("Vampire Nighthawk", 3),
    ("Doom Blade", 2),
];

/// Exercises sacrifice-as-additional-cost (CostSacrifice).
/// Village Rites tests `Cost$ B Sac<1/Creature>` — sacrifice a creature to draw two cards.
/// Cheap creatures provide sacrifice fodder; Raise Dead lets you recur them.
const SAC_COST: &[(&str, usize)] = &[
    ("Swamp", 17),
    ("Severed Strands", 10),
    ("Typhoid Rats", 4),        // Cheap sacrifice fodder (1/1 deathtouch)
    ("Vampire Nighthawk", 3),   // Evasive threat
    ("Doom Blade", 3),          // Removal
    ("Raise Dead", 3),          // Recur creatures from graveyard
];

/// Exercises issue #14: Token creation (SP$ Token) and CopyPermanent effects.
/// Raise the Alarm / Krenko's Command / Dragon Fodder create tokens directly.
/// Goblin creatures give the AI something to fight against.
const TOKEN_SWARM: &[(&str, usize)] = &[
    ("Plains", 8),
    ("Mountain", 8),
    ("Raise the Alarm", 4),  // SP$ Token: 2× 1/1 white Soldier
    ("Krenko's Command", 4), // SP$ Token: 2× 1/1 red Goblin
    ("Dragon Fodder", 4),    // SP$ Token: 2× 1/1 red Goblin
    ("Savannah Lions", 4),
    ("Lightning Bolt", 4),
    ("Shock", 4),
];

/// Exercises issue #15: library manipulation — Scry, Surveil, Mill, Dig, RearrangeTopOfLibrary.
/// Preordain tests Scry 2 + Draw.
/// Ponder tests RearrangeTopOfLibrary (3 cards) + Draw.
/// Thought Scour tests Mill 2 (targeted) + Draw.
/// Ransack the Lab tests Dig 3 / take 1 to hand / rest to graveyard.
/// Taigam's Scheming tests Surveil 5.
/// Notion Rain tests Surveil 2 + Draw 2 + DealDamage to self.
const LIBRARY_MANIPULATION: &[(&str, usize)] = &[
    ("Island", 16),
    ("Swamp", 4),
    ("Preordain", 4),          // SP$ Scry 2, then draw a card
    ("Ponder", 4),             // SP$ RearrangeTopOfLibrary 3, may shuffle, draw
    ("Thought Scour", 4),      // SP$ Mill 2 target player, draw a card
    ("Ransack the Lab", 4),    // SP$ Dig 3, take 1 to hand, rest to graveyard
    ("Taigam's Scheming", 2),  // SP$ Surveil 5
    ("Notion Rain", 2),        // SP$ Surveil 2, draw 2, deal 2 to self
    ("Divination", 4),         // Draw 2
    ("Mulldrifter", 4),        // Flying ETB draw 2
    ("Typhoid Rats", 4),       // 1/1 Deathtouch
    ("Doom Blade", 4),         // Destroy non-black creature
    ("Vampire Nighthawk", 4),  // 2/3 Flying Deathtouch Lifelink
];

/// Exercises issue #16: Counter (Counterspell, Cancel), Discard (Mind Rot),
/// and ControlGain (Control Magic) effects.
const BLUE_CONTROL: &[(&str, usize)] = &[
    ("Island", 17),
    ("Counterspell", 4),    // SP$ Counter: counter target spell
    ("Cancel", 4),          // SP$ Counter: counter target spell (3 mana)
    ("Mind Rot", 4),        // SP$ Discard: target player discards 2 cards
    ("Control Magic", 3),   // ControlGain: gain control of target creature
    ("Mulldrifter", 3),     // 2/2 Flying; ETB draw 2
    ("Divination", 4),      // Draw 2
    ("Wall of Ice", 4),     // Blocker
    ("Sea Serpent", 4),     // Big blue creature
];

/// Exercises issue #16: Fight effects (Prey Upon, Ram Through).
/// Big green creatures provide good fight targets.
const GREEN_FIGHT: &[(&str, usize)] = &[
    ("Forest", 17),
    ("Prey Upon", 4),         // SP$ Fight: creature you control fights target creature
    ("Ram Through", 4),       // SP$ Fight: similar fight spell
    ("Garruk's Companion", 4), // 3/2 Trample
    ("Centaur Courser", 4),   // 3/3
    ("Giant Spider", 3),      // 2/4 Reach
    ("Craw Wurm", 3),         // 6/4
    ("Giant Growth", 4),      // Pump before fighting
    ("Grizzly Bears", 4),     // 2/2
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
const SHOWCASE: &[(&str, usize)] = &[
    ("Island", 7),
    ("Swamp", 6),
    ("Mountain", 4),
    // Burn
    ("Lightning Bolt", 2),
    ("Shock", 2),
    // Counter
    ("Counterspell", 2),
    // Bounce
    ("Unsummon", 2),
    // Token
    ("Dragon Fodder", 2),
    // Scry + Draw
    ("Preordain", 2),
    // Surveil + Draw
    ("Notion Rain", 2),
    // Dig
    ("Ransack the Lab", 2),
    // Discard
    ("Mind Rot", 2),
    // Mill + Draw
    ("Thought Scour", 2),
    // Reanimate
    ("Raise Dead", 2),
    // Sacrifice cost
    ("Severed Strands", 2),
    // Control gain
    ("Control Magic", 1),
    // Fight
    ("Prey Upon", 2),
    // Keywords / threats
    ("Typhoid Rats", 2),
    ("Vampire Nighthawk", 2),
    ("Mulldrifter", 2),
];

/// Exercises issue #17: mass/board-wide effects.
/// - Wrath of God: DestroyAll Creature (board wipe, respects Indestructible)
/// - Pyroclasm: DamageAll 2 to each creature
/// - Righteous Charge: PumpAll +2/+2 to all your creatures until EOT
/// - Rising Miasma: PumpAll -2/-2 to all creatures until EOT
/// - Savannah Lions / White Knight / Serra Angel: creatures to interact with mass effects
const MASS_EFFECTS: &[(&str, usize)] = &[
    ("Plains", 18),
    ("Wrath of God", 4),     // SP$ DestroyAll | ValidCards$ Creature | NoRegen$ True
    ("Pyroclasm", 4),         // SP$ DamageAll | NumDmg$ 2 | ValidCards$ Creature
    ("Righteous Charge", 4), // SP$ PumpAll | ValidCards$ Creature.YouCtrl | NumAtt$ +2 | NumDef$ +2
    ("Rising Miasma", 4),    // SP$ PumpAll | ValidCards$ Creature | NumAtt$ -2 | NumDef$ -2
    ("Savannah Lions", 4),   // 2/1 attacker
    ("White Knight", 4),     // 2/2 First Strike + Protection
    ("Serra Angel", 4),      // 4/4 Flying + Vigilance
    ("Darksteel Myr", 4),    // Indestructible (survives Wrath of God)
];

/// All AI-eligible deck lists, used for random opponent selection.
const AI_DECK_OPTIONS: &[&[(&str, usize)]] = &[
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
fn random_ai_deck() -> &'static [(&'static str, usize)] {
    let mut rng = rand::thread_rng();
    AI_DECK_OPTIONS.choose(&mut rng).copied().unwrap_or(RED_BURN)
}

// ── Deck builders ──────────────────────────────────────────────────

/// Build the default AI opponent deck (Red Burn) for a single player.
///
/// Used when the human plays a custom deck so the AI still gets a deck.
pub fn build_ai_opponent(game: &mut GameState, owner: PlayerId) {
    build_named_deck(game, owner, RED_BURN);
}

/// Build a preset deck from a (name, count) list, loading each card definition
/// from the global CardDatabase (parsed from the Forge card scripts).
fn build_named_deck(game: &mut GameState, owner: PlayerId, deck: &[(&str, usize)]) {
    let db = get_card_db();
    for (name, count) in deck {
        match db.get_by_card_name(name) {
            Some(rules) => {
                for _ in 0..*count {
                    let card = card_rules_to_instance(rules, owner);
                    let id = game.create_card(card);
                    game.move_card(id, ZoneType::Library, owner);
                }
            }
            None => eprintln!("[deck] Unknown card '{}' — skipped", name),
        }
    }
}

/// Build a custom deck for `owner` from a list of card names (one name per
/// copy), loading each definition from the global CardDatabase.
/// Unrecognised names are skipped with a log message.
pub fn build_custom_deck(game: &mut GameState, owner: PlayerId, names: &[String]) {
    let db = get_card_db();
    for name in names {
        match db.get_by_card_name(name) {
            Some(rules) => {
                let card = card_rules_to_instance(rules, owner);
                let id = game.create_card(card);
                game.move_card(id, ZoneType::Library, owner);
            }
            None => eprintln!("[custom_deck] Unknown card '{}' — skipped", name),
        }
    }
}
