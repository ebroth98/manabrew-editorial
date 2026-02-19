# Forge Web

A modern web client for Magic: The Gathering, powered by a Rust rewrite of the [Forge](https://github.com/Card-Forge/forge) engine compiled to WebAssembly. Play in the browser with P2P multiplayer — no server required.

## Vision

[Forge](https://github.com/Card-Forge/forge) is one of the most complete MTG implementations (~30,000+ cards), but its Java/Swing stack confines it to desktop. This project rewrites the Forge rules engine in Rust and wraps it in a modern web UI:

- **Browser-first** — Rust engine compiled to WASM, runs entirely client-side
- **P2P multiplayer** — WebRTC data channels for peer-to-peer games, no dedicated server
- **Broadcast/spectator** — Watch games in real time
- **Tauri desktop** — Native app with the same web UI and local Rust backend
- **1:1 Forge parity** — Same card scripts, same rules behavior, same 30K+ card pool

## Architecture

```
┌──────────────────────────────────────────────────────────┐
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
| Desktop | Tauri (planned) |
| Networking | WebRTC data channels (planned) |
| Card data | Forge card scripts (.txt), Scryfall API (images) |

## Getting Started

### Web Frontend

```bash
npm install
npm run dev          # http://localhost:5173
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
3. Tauri desktop shell

## License

GPL-3.0 — same as [Forge](https://github.com/Card-Forge/forge).
