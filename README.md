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

### Java Forge Backend (WIP)

The app normally runs the native Rust engine. The `java-forge` backend is an opt-in bridge that runs the Java Forge rules engine through the existing game API/runtime shape. Use it when testing the Java-backed Forge integration instead of the Rust engine.

#### Build the Java Harness

Use JDK 18 for the Forge harness. On macOS with Zulu 18 installed:

```bash
export JAVA_HOME=/Library/Java/JavaVirtualMachines/zulu-18.jdk/Contents/Home
```

Build the harness jar and update its checksum:

```bash
JAVA_HOME=/Library/Java/JavaVirtualMachines/zulu-18.jdk/Contents/Home yarn build:harness
```

Useful checks while working on this backend:

```bash
yarn exec tsc -b
cargo check -p forge-web --features java-forge
cargo check -p self-hosted-node --features java-forge
```

#### Run Tauri with Java Forge

Start the desktop app with the Java backend selected:

```bash
OPEN_MAGIC_ENGINE_BACKEND=java-forge yarn tauri:dev -- --features java-forge
```

If running the compiled debug binary directly:

```bash
OPEN_MAGIC_ENGINE_BACKEND=java-forge ./target/debug/forge-web
```

The UI alone does not prove which backend is active. Confirm the Java path in the terminal logs:

```text
[game_thread] Starting game: ... backend=java-forge
The Callbacks are not initialized because the j4rs lib was not found. You may ignore this error if you do not use callbacks.
Language 'en-US' loaded successfully.
Read cards: ...
[parity-reset] Resetting all forge-game ID counters via reflection
[java_game_thread] Java Forge session started: ...
```

Without `OPEN_MAGIC_ENGINE_BACKEND=java-forge`, the app can still run the normal Rust backend.

Current WIP behavior:

- Java Forge priority prompts are normalized into the existing `AgentPrompt` / `GameView` UI flow.
- Java hand and command-zone actions are shown through the existing playable-card UI.
- Commander games are detected from player count, 40 life, or commander names.
- Java discard prompts are routed through the existing discard UI.
- Non-human Java prompts currently use a simple first-available-action fallback, not a full Forge AI policy.
- Mana action coverage depends on what the Java harness `ActionSpace` exposes.

## Commands Reference

### Frontend

| Command                                       | Description                                                                            |
| --------------------------------------------- | -------------------------------------------------------------------------------------- |
| `yarn dev`                                    | Start Tauri desktop app in development mode                                            |
| `yarn build`                                  | Build production Tauri app                                                             |
| `yarn vite:dev`                               | Start Vite dev server only (no Tauri)                                                  |
| `yarn vite:build`                             | Build frontend assets only                                                             |
| `yarn lint`                                   | Run ESLint                                                                             |
| `yarn preview`                                | Preview production build locally                                                       |
| `yarn ios`                                    | Start iOS development build                                                            |
| `yarn android`                                | Start Android development build                                                        |
| `yarn import-deck`                            | Import a deck from Archidekt/Moxfield into `preset_decks/` ([details](#deck-importer)) |
| `node scripts/generate-theme-css.mjs --write` | Regenerate game-theme `@theme` CSS block ([details](#theme-css-generator))             |

### Deck Importer

Pull decks from [Archidekt](https://archidekt.com) or [Moxfield](https://moxfield.com) and write them out as a preset deck JSON in `preset_decks/`. Useful for bootstrapping new test decks without hand-typing card lists.

```bash
# Search Archidekt by name, pick a result, preview, then import
yarn import-deck "jund wildfire"

# Jump straight to a specific deck by URL (Archidekt or Moxfield)
yarn import-deck --url=https://archidekt.com/decks/16127024/jund_lands
yarn import-deck --url=https://moxfield.com/decks/6S6hLVDqKkqWEl8hAgSYkw

# Tag the preset with a format (default: standard)
yarn import-deck "korvold" --format=commander
```

The interactive flow searches Archidekt (URL mode dispatches to the matching provider), lists results with author/format/blurb, lets you pick one to preview the full card list, and on confirm prompts for label / description / filename before writing the preset JSON. Commanders are detected from deck categories and folded into the preset's `cards` field.

| Flag              | Default    | Description                                            |
| ----------------- | ---------- | ------------------------------------------------------ |
| _(positional)_    | —          | Search query. Ignored when `--url` is passed.          |
| `--url=<url>`     | —          | Archidekt or Moxfield deck URL; skips the search step. |
| `--format=<name>` | `standard` | Value written to the preset's `format` field.          |

The same Archidekt / Moxfield core powers the in-app **Import from URL** and **Search deck** entries in the deck editor menu, so the CLI and the UI stay in sync.

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

| Variable                | Description                                                                    |
| ----------------------- | ------------------------------------------------------------------------------ |
| `FORGE_RNG_TRACE=1`     | Log every RNG call in both engines (agent + game)                              |
| `FORGE_TRIGGER_TRACE=1` | Log trigger processing, DisableTriggers checks, optional trigger confirmations |
| `FORGE_LIFE_TRACE=1`    | Log every life gain/loss event                                                 |

### Theme CSS Generator

When you add a new colour token to `GameThemeColors`, run this script to regenerate the Tailwind `@theme` block in `src/index.css`:

```bash
# Preview the generated CSS (no file changes)
node scripts/generate-theme-css.mjs

# Update src/index.css in place
node scripts/generate-theme-css.mjs --write
```

The script reads all dot-notation keys from `buildGameColors.ts`, converts them to kebab-case CSS variable names, and writes the `--color-*: var(--*)` mappings that Tailwind needs to generate utility classes like `bg-pointer-hostile` or `text-mana-w`. Idempotent — safe to re-run.

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

| Upstream Area                               | Rust Counterpart                    | Action                                                              |
| ------------------------------------------- | ----------------------------------- | ------------------------------------------------------------------- |
| `forge-game/` (rules engine)                | `forge-engine/crates/forge-engine/` | Port rule changes to Rust                                           |
| `forge-gui/res/cardsfolder/` (card scripts) | Parsed by `forge-carddb` at runtime | Usually automatic — new cards work if ability types are implemented |
| `forge-game/` API changes                   | `forge/forge-harness/`              | Update harness if parity tests break                                |

### Recommended Cadence

Monthly manual pulls work well. Most upstream changes are card-level (new cards, script fixes) that work automatically. Pull sooner if a relevant engine change lands.

## Tech Stack

| Layer      | Technology                                            |
| ---------- | ----------------------------------------------------- |
| Frontend   | React 19, TypeScript, Vite                            |
| Styling    | Tailwind CSS 4, Shadcn/UI                             |
| State      | Zustand, TanStack Query                               |
| Routing    | React Router                                          |
| Engine     | Rust (compiled natively for Tauri)                    |
| Desktop    | Tauri v2                                              |
| Networking | WebRTC data channels (planned)                        |
| Card Data  | Forge card scripts (.txt), Scryfall API (images)      |
| Themes     | 12 built-in presets (Nord, Catppuccin, Dracula, etc.) |

## Development Workflow

1. Create a feature branch from `main`
2. Reference the Java source for any engine work
3. Run `yarn scan` to check file parity
4. Run `yarn parity <test>` to verify behavioral parity
5. Open a PR — never push directly to `main`

## Releases

Releases are published by pushing a git tag that matches `v*` (e.g. `v0.1.0`). Tag pushes trigger the `.github/workflows/release-artifacts.yml` workflow, which builds the macOS `.dmg` and Windows `.exe` / `.msi` on self-hosted runners and publishes a GitHub Release with all three binaries attached.

Non-tag events also run the workflow but do **not** publish a Release:

- **Push to `main`** — builds `.dmg` / `.exe` only if the corresponding checkbox in the PR body is ticked (see `.github/pull_request_template.md`). Artifacts are uploaded to the Actions run with 30-day retention.
- **`workflow_dispatch`** — prompts for `build_macos` / `build_windows` toggles. Same 30-day artifact retention.

### Step-by-step

1. **Confirm `main` is in the state you want to release.**

   ```bash
   git checkout main
   git pull --ff-only
   git status               # must be clean
   ```

2. **Pick a version number.** Use semver: `v<major>.<minor>.<patch>`. Prereleases add a suffix (`v0.2.0-rc1`, `v0.2.0-beta`) and are automatically flagged as "Pre-release" on GitHub.

3. **Bump the app version** so the installer filenames match the tag. Update `version` in:
   - `package.json`
   - `src-tauri/tauri.conf.json`
   - `src-tauri/Cargo.toml`

   Commit and push:

   ```bash
   git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml
   git commit -m "chore: bump version to 0.2.0"
   git push origin main
   ```

   Wait for CI to go green on that commit.

4. **Tag and push.** The tag name must start with `v`.

   ```bash
   git tag v0.2.0
   git push origin v0.2.0
   ```

5. **Watch the workflow** in the Actions tab. Four jobs run in sequence:

   ```
   gate ──► macos-dmg ─┐
        └─► windows-exe ─► release
   ```

   Windows builds take ~15–25 min; mac varies. The `release` job fires once both installers upload.

6. **Verify the release.** Repo → **Releases** should now show the new tag with:
   - Auto-generated changelog (commits since the previous tag, grouped by conventional-commit prefix)
   - `OpenMagic_X.Y.Z_x64-setup.exe` (Windows NSIS installer)
   - `OpenMagic_X.Y.Z_x64_en-US.msi` (Windows MSI)
   - `.dmg` (macOS)

### Re-running or fixing a bad release

- **A build job failed:** land a fix on `main`, delete the tag, re-tag the new commit.
  ```bash
  git tag -d v0.2.0
  git push origin :refs/tags/v0.2.0
  # after the fix merges:
  git tag v0.2.0 <new-sha>
  git push origin v0.2.0
  ```
- **Only the `release` job failed (builds succeeded):** re-run just that job from the Actions UI — the artifacts are still attached to the run.
- **Release published but wrong:** `gh release delete v0.2.0 --cleanup-tag`, then re-tag.

### Runner prerequisites

The workflow runs on self-hosted runners (`self-hosted, macOS` and `self-hosted, Windows`). The Windows runner requires a one-time setup via `scripts/setup-windows-runner.ps1` — installs Rust, MSVC Build Tools, Tauri CLI, `wasm-pack`, and configures the runner service to run as `.\Administrator` (needed so the service can read cargo bins under the Administrator profile).

## License

GPL-3.0 — same as [Forge](https://github.com/Card-Forge/forge).
