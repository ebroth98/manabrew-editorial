# OpenMagic

A modern desktop and web client for Magic: The Gathering, powered by a Rust rewrite of the [Forge](https://github.com/Card-Forge/forge) engine. Play as a native desktop app with P2P multiplayer — no dedicated server required.

## Vision

[Forge](https://github.com/Card-Forge/forge) is one of the most complete MTG implementations (~30,000+ cards), but its Java/Swing stack confines it to desktop. This project rewrites the Forge rules engine in Rust and wraps it in a modern UI:

- **Tauri desktop** — Native app wrapping a React UI with a local Rust backend
- **P2P multiplayer** — WebRTC data channels for peer-to-peer games, no dedicated server
- **1:1 Forge parity** — Same card scripts, same rules behavior, same 30K+ card pool
- **Strict Java parity** — Rust engine mirrors Java's file structure, naming, and behavior exactly

## Architecture

```
+----------------------------------------------------------+
|                  Tauri Desktop Shell                      |
|           Native window  ·  Local Rust backend            |
+----------------------------------------------------------+
|                     Web Frontend                          |
|        React + Vite + Tailwind + Shadcn/UI                |
|     Views: Login, Lobby, Deck Editor, Game, Draft         |
|     State: Zustand stores + TanStack Query                |
+----------------------------------------------------------+
|                    Networking Layer                        |
|           WebRTC P2P  /  Broadcast channels                |
+----------------------------------------------------------+
|                  forge-engine (Rust)                       |
|    Rules engine, game loop, combat, triggers, stack,      |
|    replacement effects, static abilities, cost framework  |
+----------------------------------------------------------+
|                    forge-carddb                            |
|        Parses Forge's 32,000+ card scripts                |
+----------------------------------------------------------+
|                  forge-foundation                          |
|        Core MTG types (colors, mana, phases, zones)       |
+----------------------------------------------------------+
```

## Repository Structure

```
.
├── src/                        # React web frontend
│   ├── components/             # UI components (Shadcn + domain-specific)
│   ├── stores/                 # Zustand state (auth, connection, deck, game)
│   ├── views/                  # Pages (Login, Lobby, DeckEditor, Game, Draft)
│   ├── hooks/                  # Custom React hooks
│   ├── themes/                 # Color theme presets (12 themes)
│   └── types/                  # TypeScript interfaces
├── src-tauri/                  # Tauri v2 desktop shell (Rust)
├── forge-engine/               # Rust engine workspace
│   └── crates/
│       ├── forge-foundation/   # Core MTG types, no I/O
│       ├── forge-carddb/       # Card script parser (32K+ cards)
│       ├── forge-engine/       # Game state, rules, combat, stack, effects
│       ├── forge-parity/       # Parity test harness (Rust vs Java comparison)
├── forge/                      # Java Forge source (reference implementation)
│   ├── forge-game/             # Java rules engine (the source of truth)
│   ├── forge-gui/res/          # Card scripts, tokens, editions
│   └── forge-harness/          # Java parity harness (our addition)
├── scripts/                    # Build and test scripts
├── docs/                       # Project documentation
│   ├── STYLE_GUIDELINES.md     # UI/UX style guide
│   ├── PARITY_TESTING.md       # Parity test documentation
│   └── DEPLOY.md               # Deployment guide
├── features.md                 # Java-to-Rust porting progress tracker
├── CLAUDE.md                   # AI agent instructions and project guidelines
└── scan_structure.cjs          # Java vs Rust file parity scanner
```

## Getting Started

### Prerequisites

- [Node.js](https://nodejs.org/) 18+
- [Yarn](https://yarnpkg.com/) (v1)
- [Rust](https://rustup.rs/) (stable toolchain)
- [Java JDK](https://adoptium.net/) 11+ and [Maven](https://maven.apache.org/) (for parity tests)
- On macOS: Xcode Command Line Tools — `xcode-select --install`
- On Linux: `libwebkit2gtk-4.1-dev` and other [Tauri system dependencies](https://tauri.app/start/prerequisites/)

### Install Dependencies

```bash
yarn install
```

### Run the Desktop App

```bash
yarn dev
```

This builds the Rust backend (`src-tauri/`), starts Vite on `:1420`, and opens the native Tauri window. Hot-reload is enabled for the React frontend.

### Build for Production

```bash
yarn build
```

Creates a distributable native app in `src-tauri/target/release/`.

## Commands Reference

### Frontend

| Command | Description |
|---------|-------------|
| `yarn dev` | Start Tauri desktop app in development mode |
| `yarn build` | Build production Tauri app |
| `yarn vite:dev` | Start Vite dev server only (no Tauri) |
| `yarn vite:build` | Build frontend assets only |
| `yarn lint` | Run ESLint |
| `yarn preview` | Preview production build locally |
| `yarn ios` | Start iOS development build |
| `yarn android` | Start Android development build |


### Parity Testing

Parity tests compare the Rust engine against the Java Forge engine to ensure identical behavior. Both engines play the same game with the same seed and the traces are compared decision-by-decision.

```bash
# Build the Java parity harness (required once, or after Java changes)
yarn build:harness

# Run a specific parity test
yarn parity <test-name>

# Examples:
yarn parity basic_red_vs_green
yarn parity payments
yarn parity staticability
yarn parity realMonoBlackSacrice

# Run parity directly with custom args
yarn parity:test -- --deck1 red_burn --deck2 green_stompy --seed 42 --max-turns 30 --games 10
```

Available parity tests are defined in `forge-engine/crates/forge-parity/regression.json`. Each entry specifies decks, seeds, max turns, and number of games.

#### Parity Test Environment Variables

| Variable | Description |
|----------|-------------|
| `FORGE_RNG_TRACE=1` | Log every RNG call in both engines (agent + game) |
| `FORGE_TRIGGER_TRACE=1` | Log trigger processing, DisableTriggers checks, optional trigger confirmations |
| `FORGE_LIFE_TRACE=1` | Log every life gain/loss event |

### Structure Scanner

Compare Java engine files against their Rust counterparts to track porting progress:

```bash
yarn scan
```

This scans `forge/forge-game/src/main/java/forge/game/` and compares against `forge-engine/crates/forge-engine/src/`, printing a tree with coverage percentages per module.

## Engine Parity Philosophy

This project maintains **strict 1:1 parity** with the Java Forge engine:

### File and Symbol Parity

- **Every Java file** in `forge/forge-game/src/main/java/forge/game/` should have a corresponding Rust file in `forge-engine/crates/forge-engine/src/`
- **File names match** — `ChangeZoneEffect.java` becomes `change_zone_effect.rs` (snake_case conversion)
- **Module structure matches** — `forge/game/ability/effects/` maps to `ability/effects/`
- **API type strings match** — card scripts use the same `SP$`, `DB$`, `AB$` prefixes and effect names
- **Behavior matches** — given the same inputs, both engines must produce identical game states

### What This Means in Practice

- When implementing a feature, always reference the Java source first
- Don't invent new file names or module structures — mirror Java's organization
- Use `yarn scan` to check which Java files still need Rust counterparts
- Use `yarn parity` to verify behavioral parity after changes
- The `features.md` file tracks porting progress across all subsystems

### Effect and Replacement Coverage

The Rust engine currently implements:
- **232 effect API types** (all dispatched with real game logic)
- **37 replacement event types** (all with `can_replace` + `execute` + game loop callsites)
- **Full trigger system** with LKI, DisableTriggers, optional triggers, and death-return triggers

## Syncing with Upstream Forge

This project tracks [Card-Forge/forge](https://github.com/Card-Forge/forge) as an upstream remote. The Java source in `forge/` serves as the reference implementation. Periodically pull upstream changes to stay current with new cards, rules fixes, and engine improvements.

### One-Time Setup

```bash
git remote add upstream https://github.com/Card-Forge/forge.git
```

Verify:

```bash
git remote -v
# origin    https://github.com/<your-org>/openmagic.git (fetch)
# upstream  https://github.com/Card-Forge/forge.git (fetch)
```

### Pulling Upstream Changes

```bash
git fetch upstream
git checkout main
git merge upstream/master --allow-unrelated-histories
```

Conflicts are typically limited to `forge/` — the Rust engine, frontend, and preset decks are our own code. The `forge/forge-harness/` module is ours and doesn't exist upstream, so it merges cleanly.

### Reviewing Engine Changes

After merging, check what changed in the Java engine that might need porting:

```bash
# Core rules engine changes
git diff HEAD~1...HEAD -- forge/forge-game/src/main/java/forge/game/

# New or modified card scripts
git diff HEAD~1...HEAD -- forge/forge-gui/res/cardsfolder/

# Summary of changed files
git diff HEAD~1...HEAD --stat -- forge/forge-game/ forge/forge-gui/res/cardsfolder/
```

### What to Port

| Upstream Area | Rust Counterpart | Action |
|---|---|---|
| `forge-game/` (rules engine) | `forge-engine/crates/forge-engine/` | Port rule changes to Rust |
| `forge-gui/res/cardsfolder/` (card scripts) | Parsed by `forge-carddb` at runtime | Usually automatic — new cards work if ability types are implemented |
| `forge-game/` API changes | `forge/forge-harness/` | Update harness if parity tests break |

### Recommended Cadence

Monthly manual pulls work well. Most upstream changes are card-level (new cards, script fixes) that work automatically. Pull sooner if a relevant engine change lands.

## Tech Stack

| Layer | Technology |
|---|---|
| Frontend | React 19, TypeScript, Vite |
| Styling | Tailwind CSS 4, Shadcn/UI |
| State | Zustand, TanStack Query |
| Routing | React Router |
| Engine | Rust (compiled natively for Tauri) |
| Desktop | Tauri v2 |
| Networking | WebRTC data channels (planned) |
| Card Data | Forge card scripts (.txt), Scryfall API (images) |
| Themes | 12 built-in presets (Nord, Catppuccin, Dracula, etc.) |

## Development Workflow

1. Create a feature branch from `main`
2. Reference the Java source for any engine work
3. Run `yarn scan` to check file parity
4. Run `yarn parity <test>` to verify behavioral parity
5. Open a PR — never push directly to `main`

## License

GPL-3.0 — same as [Forge](https://github.com/Card-Forge/forge).
