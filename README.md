# Bardidina Magica

A modern desktop and web client for Magic: The Gathering, powered by a Rust rewrite of the [Forge](https://github.com/Card-Forge/forge) engine compiled to WebAssembly. Play in the browser or as a native desktop app — P2P multiplayer with no dedicated server required.

## Vision

[Forge](https://github.com/Card-Forge/forge) is one of the most complete MTG implementations (~30,000+ cards), but its Java/Swing stack confines it to desktop. This project rewrites the Forge rules engine in Rust and wraps it in a modern UI:

- **Browser-first** — Rust engine compiled to WASM, runs entirely client-side
- **P2P multiplayer** — WebRTC data channels for peer-to-peer games, no dedicated server
- **Broadcast/spectator** — Watch games in real time
- **Tauri desktop** — Native app wrapping the same React UI with a local Rust backend
- **1:1 Forge parity** — Same card scripts, same rules behavior, same 30K+ card pool

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│                  Tauri Desktop Shell                      │
│           Native window  ·  Local Rust backend           │
├──────────────────────────────────────────────────────────┤
│                     Web Frontend                         │
│        React + Vite + Tailwind + Shadcn/UI               │
│     Views: Login, Lobby, Deck Editor, Game, Draft        │
│     State: Zustand stores + TanStack Query               │
├──────────────────────────────────────────────────────────┤
│                    Networking Layer                       │
│           WebRTC P2P  /  Broadcast channels               │
├──────────────────────────────────────────────────────────┤
│                  forge-engine (WASM)                      │
│        Rules engine, game loop, state management         │
├──────────────────────────────────────────────────────────┤
│                    forge-carddb                           │
│        Parses Forge's 32,000+ card scripts               │
├──────────────────────────────────────────────────────────┤
│                  forge-foundation                         │
│        Core MTG types (colors, mana, phases, zones)      │
└──────────────────────────────────────────────────────────┘
```

## Repository Structure

```
├── src/                    # React web frontend
│   ├── api/                # WebSocket, Scryfall, middleware mock
│   ├── components/         # UI components (Shadcn + domain-specific)
│   ├── stores/             # Zustand state (auth, connection, deck, game)
│   ├── views/              # Pages (Login, Lobby, DeckEditor, Game, Draft)
│   ├── hooks/              # Custom React hooks
│   ├── types/              # TypeScript interfaces
│   └── lib/                # Utilities
├── src-tauri/              # Tauri v2 desktop shell (Rust)
│   ├── src/                # Rust entry point and lib
│   ├── tauri.conf.json     # Tauri configuration
│   └── capabilities/       # Tauri permission capabilities
├── forge-engine/           # Rust engine (see forge-engine/README.md)
│   └── crates/
│       ├── forge-foundation/   # Core MTG types, no I/O
│       ├── forge-carddb/       # Card script parser (32K+ cards)
│       ├── forge-engine/       # Game state, rules, combat, stack
│       └── forge-cli/          # Terminal client for dev/testing
├── forge/                  # Java Forge source (reference implementation)
├── vite.config.ts
├── package.json
└── tsconfig.json
```

## Tech Stack

| Layer | Technology |
|---|---|
| Frontend | React 19, TypeScript, Vite |
| Styling | Tailwind CSS 4, Shadcn/UI |
| State | Zustand, TanStack Query |
| Routing | React Router |
| Engine | Rust → WebAssembly (wasm-bindgen) |
| Desktop | Tauri v2 |
| Networking | WebRTC data channels (planned) |
| Card data | Forge card scripts (.txt), Scryfall API (images) |

## Getting Started

### Prerequisites

- [Node.js](https://nodejs.org/) 18+
- [Rust](https://rustup.rs/) (stable toolchain)
- On macOS: Xcode Command Line Tools — `xcode-select --install`
- On Linux: `libwebkit2gtk-4.1-dev` and other [Tauri system dependencies](https://tauri.app/start/prerequisites/)

### Desktop App (Tauri)

```bash
npm install
npm run dev          # Builds src-tauri, starts Vite on :1420, opens native window
```

### Rust Engine

```bash
cd forge-engine
cargo build --workspace
cargo test --workspace          # 68 tests
cargo run --package forge-cli   # Interactive terminal game
```

See [`forge-engine/README.md`](forge-engine/README.md) for engine architecture and roadmap.

## Roadmap

### Engine (Rust)
1. ~~Foundation types~~ — Done
2. ~~Card database (32K+ cards)~~ — Done
3. ~~Game skeleton~~ — Done
4. ~~First playable~~ — Done
5. ~~Keywords & targeting~~ — Done
6. Triggers & triggered abilities
7. Static abilities & continuous effects
8. Replacement effects
9. Activated abilities & cost framework
10. Full Forge API type coverage (~150+ ability types)
11. WASM bindings (wasm-bindgen exports)

### Frontend (React)
1. Lobby & matchmaking
2. Deck editor with card search
3. Game battlefield UI
4. Game interaction (targeting, combat, stack)
5. Draft/sealed interface
6. Settings & preferences

### Platform
1. WebRTC P2P networking
2. Broadcast/spectator mode
3. ~~Tauri desktop shell~~ — Done

## Syncing with Upstream Forge

This project is a fork of [Card-Forge/forge](https://github.com/Card-Forge/forge). The Java source in `forge/` serves as the reference implementation. Periodically pull upstream changes to stay current with new cards, rules fixes, and engine improvements.

### One-time setup

```bash
git remote add upstream https://github.com/Card-Forge/forge.git
```

Verify it's configured:

```bash
git remote -v
# origin    https://github.com/<your-org>/xmage.git (fetch)
# upstream  https://github.com/Card-Forge/forge.git (fetch)
```

### Pulling upstream changes

```bash
git fetch upstream
git checkout main
git merge upstream/master --allow-unrelated-histories
```

Resolve any conflicts (typically limited to `forge/` — the Rust engine, frontend, and preset decks are our own code and won't conflict). The `forge/forge-harness/` module is ours and doesn't exist upstream, so it merges cleanly.

### Reviewing engine changes

After merging, check what changed in the Java engine that might need porting to Rust:

```bash
# Changes in the core game engine
git diff HEAD~1...HEAD -- forge/forge-game/src/main/java/forge/game/

# Changes in AI logic
git diff HEAD~1...HEAD -- forge/forge-ai/src/main/java/forge/ai/

# Changes in card scripts (new cards, fixes)
git diff HEAD~1...HEAD -- forge/forge-gui/res/cardsfolder/

# Summary of changed files only
git diff HEAD~1...HEAD --stat -- forge/forge-game/ forge/forge-ai/ forge/forge-gui/res/cardsfolder/
```

### What to look for

| Upstream area | Local counterpart | Action needed |
|---|---|---|
| `forge-game/` (rules engine) | `forge-engine/crates/forge-engine/` | Port rule changes to Rust |
| `forge-ai/` (AI logic) | `src-tauri/src/ai_agent.rs` | Update AI behavior if relevant |
| `forge-gui/res/cardsfolder/` (card scripts) | Parsed by `forge-carddb` at runtime | Usually automatic — new cards just work if the ability types are already implemented |
| `forge-game/` API changes | `forge/forge-harness/` | Update harness if parity tests break |

### Recommended cadence

Monthly manual pulls work well. Automated daily syncing creates noise since most upstream changes are card-level (new cards, script fixes) that work automatically. Pull sooner if you know a relevant engine change landed.

## License

GPL-3.0 — same as [Forge](https://github.com/Card-Forge/forge).
