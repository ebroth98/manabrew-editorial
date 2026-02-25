# Forge Engine

A Rust rewrite of the [Forge](https://github.com/Card-Forge/forge) Magic: The Gathering engine, targeting **1:1 behavioral parity** with the Java backend. Built for **WebAssembly** (browser play via P2P/broadcast) and **Tauri** (native desktop).

## Why Rust?

Forge is one of the most complete MTG implementations (~30,000+ cards), but its Java/Swing stack limits where it can run. This rewrite preserves Forge's rules fidelity while enabling:

- **Web** — Compile to WASM, run the full rules engine in-browser with zero server costs
- **P2P multiplayer** — WebRTC data channels for peer-to-peer games, no dedicated server needed
- **Native desktop** — Tauri app with a web UI frontend and native Rust backend
- **Deterministic replay** — Serializable game state enables spectating, replays, and undo
- **Performance** — Arena-based ECS, zero-copy where possible, no GC pauses

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                    Frontend (Web/Tauri)              │
│              HTML/CSS/JS  ←→  WASM bindings          │
├─────────────────────────────────────────────────────┤
│                      forge-cli                       │
│              Terminal UI for development/testing      │
├─────────────────────────────────────────────────────┤
│                     forge-engine                     │
│     GameState, GameLoop, combat, mana, stack,        │
│     state-based actions, PlayerAgent trait            │
├─────────────────────────────────────────────────────┤
│                     forge-carddb                     │
│     Card script parser, CardRules, CardDatabase      │
│     Parses all 32,000+ Forge card definitions        │
├─────────────────────────────────────────────────────┤
│                   forge-foundation                   │
│     Color, ManaCost, CardTypeLine, ZoneType,         │
│     PhaseType — pure types, no I/O                   │
└─────────────────────────────────────────────────────┘
```

### Crate Overview

| Crate | Purpose |
|---|---|
| `forge-foundation` | Core MTG types: colors (5-color bitmask), mana costs (45 shard types incl. hybrid/phyrexian), card types, 19 zone types, 13 phase types |
| `forge-carddb` | Parses Forge's `.txt` card scripts into structured `CardRules`. Loads via string iterators (no filesystem — WASM-ready). **Parses all 32,000+ cards with zero failures.** |
| `forge-engine` | Mutable game state & rules enforcement. Arena-based entity system (`Vec<CardInstance>`, typed `CardId`/`PlayerId` indices). GameLoop drives turns; `PlayerAgent` trait decouples decisions from engine. |
| `forge-cli` | Interactive terminal client for development. ANSI-colored board display, 4 prebuilt decks, human vs AI play. |

### Key Design Decisions

- **Arena ECS** — Cards and players live in flat `Vec`s, referenced by typed IDs (`CardId(u32)`, `PlayerId(u32)`). No reference cycles, trivially serializable, cache-friendly.
- **`GameState` owns everything** — All mutation through `&mut GameState`. No shared references, no interior mutability.
- **Immutable card definitions** — `CardRules` (from carddb) never change. `CardInstance` (in engine) is the mutable in-game representation.
- **No `std::fs` in core** — Card loading uses `impl Iterator<Item = (String, String)>`. The engine compiles to WASM without feature flags.
- **`PlayerAgent` trait** — All player decisions (attacks, blocks, targets, mulligans) go through a trait. Implementations: interactive CLI, AI, scripted (tests), or network-backed.
- **Forge script compatibility** — Ability strings use Forge's pipe-delimited format (`SP$ DealDamage | ValidTgts$ Any | NumDmg$ 3`). The goal is to reuse Forge's 30K+ card definitions as-is.

## Current Status

### Implemented

- **Turn structure** — Full 13-phase turn cycle (Untap → Cleanup), multiplayer turn order
- **Zones** — 19 zone types with ownership tracking, zone-change state resets
- **Mana system** — 45 mana shard types, mana pool with colored/generic payment
- **Combat** — Attack/block declaration, damage assignment, first strike / double strike (two-step resolution), combat state tracking
- **9 combat keywords** — Flying, Reach, First Strike, Double Strike, Trample, Deathtouch, Lifelink, Vigilance, Defender
- **Creature targeting** — `ValidTgts$ Any` (player or creature), `ValidTgts$ Creature.nonBlack` (filtered), target validation before spell resolution
- **4 spell effects** — DealDamage, Pump (Giant Growth), Destroy (Doom Blade), Draw (Divination)
- **State-based actions** — Lethal damage, zero toughness, zero life, poison (10+), deathtouch
- **Card database** — Parses all 32,000+ Forge card scripts, split/transform/meld cards, planeswalker loyalty, SVars
- **Counters** — +1/+1, -1/-1, loyalty, charge, and 12 other counter types
- **Serialization** — Full `GameState` serializable via serde (save/load/replay)
- **CLI client** — Interactive terminal game with 4 themed decks, human vs simple AI

### Not Yet Implemented

- Triggered abilities and the trigger system
- Static/continuous effects (anthems, auras)
- Replacement effects (damage prevention, redirection)
- Activated abilities beyond mana
- Planeswalker rules
- Token generation
- Full ability API type coverage (~150+ Forge API types)
- WASM bindings and web frontend
- P2P networking layer
- Tauri desktop shell

## Building

```bash
# Prerequisites: Rust 1.70+ (rustup.rs)

# Build all crates
cargo build --workspace

# Run tests (68 unit + integration tests)
cargo test --workspace

# Parse all 32K+ Forge card scripts (requires Forge res/ directory)
cargo test --test parse_all_cards -- --ignored

# Run the CLI game
cargo run --package forge-cli
```

## Roadmap

The rewrite follows Forge's own architecture — the card script format is the contract. Each phase expands engine coverage toward full parity.

| Phase | Status | What |
|---|---|---|
| 1. Foundation | Done | Core types: Color, ManaCost, CardTypeLine, ZoneType, PhaseType |
| 2. Card Database | Done | Script parser, 32K+ cards parsed at 100% success rate |
| 3. Game Skeleton | Done | GameState, arenas, zones, turns, PlayerAgent trait |
| 4. First Playable | Done | Mana pool, combat, DealDamage, CLI client, end-to-end tests |
| 5. Keywords & Targeting | Done | 9 combat keywords, creature targeting, Pump/Destroy/Draw effects |
| 6. Triggers | Next | When/Whenever triggers, ETB/LTB/dies, trigger stack ordering |
| 7. Static Abilities | — | Continuous effects, anthems, auras, layering system (CR 613) |
| 8. Replacement Effects | — | Prevention, redirection, "instead" effects |
| 9. Activated Abilities | — | Tap abilities, loyalty abilities, cost payment framework |
| 10. API Type Expansion | — | Systematic coverage of Forge's ~150+ ability API types |
| 11. WASM + Web | — | wasm-bindgen exports, JS/TS bindings, web UI |
| 12. Networking | — | WebRTC P2P, game state sync, broadcast/spectator mode |
| 13. Tauri Desktop | — | Native wrapper, local game storage, deck builder |

## Project Structure

```
forge-engine/
├── Cargo.toml                          # Workspace root
├── crates/
│   ├── forge-foundation/
│   │   └── src/
│   │       ├── lib.rs                  # Re-exports all types
│   │       ├── color.rs                # Color (5 enum), ColorSet (bitmask)
│   │       ├── mana.rs                 # ManaCostShard (45 variants), ManaCost
│   │       ├── card_type.rs            # CoreType, Supertype, CardTypeLine
│   │       ├── card_split.rs           # Split/transform/meld card handling
│   │       ├── zone.rs                 # ZoneType (19 zones)
│   │       └── phase.rs               # PhaseType (13 phases)
│   ├── forge-carddb/
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── card_face.rs            # CardFace — printed card data
│   │   │   ├── card_rules.rs           # CardRules — complete card definition
│   │   │   ├── parser.rs              # Forge .txt script parser
│   │   │   └── database.rs            # CardDatabase — lookup & loading
│   │   └── tests/
│   │       └── parse_all_cards.rs      # Validates all 32K+ scripts
│   ├── forge-engine/
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── ids.rs                  # CardId, PlayerId (typed indices)
│   │   │   ├── game.rs                 # GameState (central state container)
│   │   │   ├── card.rs                 # CardInstance (mutable in-game card)
│   │   │   ├── player.rs              # PlayerState (life, counters, flags)
│   │   │   ├── zone.rs                 # Zone (ordered card list per owner)
│   │   │   ├── phase.rs               # TurnState (turn/phase/priority)
│   │   │   ├── stack.rs               # MagicStack, StackEntry
│   │   │   ├── combat.rs              # CombatState (attackers/blockers)
│   │   │   ├── mana_pool.rs           # ManaPool (WUBRG+colorless)
│   │   │   ├── game_loop.rs           # GameLoop (turn driver, spell resolution)
│   │   │   ├── action.rs              # State mutations, SBAs, draw/discard
│   │   │   └── agent.rs               # PlayerAgent trait, TargetChoice
│   │   └── tests/
│   │       └── first_game.rs           # End-to-end game integration tests
│   └── forge-cli/
│       └── src/
│           └── main.rs                 # Interactive terminal game client
```

## Parity Engine

The `forge-parity` crate is a differential testing framework that validates the Rust engine produces identical game behavior to the Java Forge reference implementation.

### Prerequisites

- Rust 1.70+ (`rustup.rs`)
- Java 11+ and Maven (for the Java harness)

### 1. Build the Java harness

```bash
cd forge/forge-harness
mvn package -q
```

This produces `forge/forge-harness/target/forge-harness-jar-with-dependencies.jar`.

### 2. Run in Rust-only mode (snapshot dump)

Runs the Rust engine with a deterministic agent and outputs per-phase snapshots. Useful for debugging and golden file testing.

```bash
cargo run -p forge-parity -- \
  --deck1 red_burn --deck2 green_stompy \
  --seed 42 --max-turns 10 \
  --format json
```

### 3. Run in full parity mode (Rust vs Java comparison)

Runs both engines side-by-side with the same seed and decks, then compares snapshots field-by-field and reports divergences.

```bash
cargo run -p forge-parity -- \
  --deck1 red_burn --deck2 green_stompy \
  --seed 42 --max-turns 10 \
  --java-jar forge/forge-harness/target/forge-harness-jar-with-dependencies.jar \
  --format text
```

### Available preset decks

`red_burn`, `green_stompy`, `white_aggro`, `black_control`

### CLI flags

| Flag | Description |
|---|---|
| `--deck1 <name>` | Preset deck for player 1 |
| `--deck2 <name>` | Preset deck for player 2 |
| `--seed <N>` | Deterministic RNG seed |
| `--max-turns <N>` | Stop after N turns |
| `--java-jar <path>` | Path to Java harness JAR (enables full parity mode) |
| `--format json\|text` | Output format |
| `--output <file>` | Write to file instead of stdout |

### How it works

Both engines use a **DeterministicAgent** that makes reproducible decisions:
- Always keeps opening hand (no mulligan)
- Plays first playable card (alphabetically by name), then passes
- Attacks with all eligible creatures (sorted by name)
- Never blocks
- Targets opponent for player targets; first alphabetical card for card targets
- Discards first N cards alphabetically

Snapshots are captured at each phase transition and compared by card name (not internal IDs) with alphabetic sorting for deterministic ordering.

## Relationship to Forge

This project reuses [Forge's card script format](https://github.com/Card-Forge/forge) as the source of truth for card definitions. The ~32,000 `.txt` files in Forge's `res/cardsfolder/` directory define every card's abilities, types, costs, and rules text. The Rust engine parses these scripts identically and aims to produce the same game behavior as Forge's Java `forge-game` module.

**What we reuse from Forge:** Card script definitions, edition data, card art references.
**What we rewrite:** The rules engine, game loop, UI layer, and networking — all in Rust.

## License

GPL-3.0 — same as [Forge](https://github.com/Card-Forge/forge).
