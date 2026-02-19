# Implementation Plan: Forge Web

## Project Overview
**Goal:** Rewrite the Forge MTG engine in Rust, compile to WASM, and build a modern web client with P2P multiplayer. Achieve 1:1 behavioral parity with the Java Forge backend.

**Stack:**
- **Engine:** Rust → WebAssembly (wasm-bindgen/wasm-pack)
- **Frontend:** React 19 + Vite + TypeScript
- **Styling:** Tailwind CSS 4 + Shadcn/UI
- **State:** Zustand (game/UI state) + TanStack Query (card data/images)
- **Networking:** WebRTC data channels (P2P), broadcast channels (spectating)
- **Desktop:** Tauri (Rust backend + web frontend)
- **Card data:** Forge `.txt` scripts (rules), Scryfall API (images)

**Key Constraint:** The Rust engine must parse and execute Forge's 32,000+ card script files identically to the Java `forge-game` module. Card scripts are the contract between old and new.

---

## Phase 1: Rust Engine — Foundation (DONE)

**Objective:** Core MTG types and card database parser.

1. **Foundation types** (`forge-foundation`)
   - Color enum (5 colors) + ColorSet bitmask (32 combinations)
   - ManaCost with 45 shard variants (hybrid, phyrexian, snow, X)
   - CardTypeLine parser ("Legendary Creature — Human Wizard")
   - ZoneType (19 zones), PhaseType (13 phases)
   - CardSplitType (split, transform, meld, adventure, modal)

2. **Card script parser** (`forge-carddb`)
   - Line-by-line parser for Forge `.txt` format
   - CardFace (printed card data), CardRules (complete definition)
   - CardDatabase with lookup by name
   - WASM-compatible loading via string iterators (no `std::fs`)
   - **Result:** Parses all 32,000+ cards with zero failures

---

## Phase 2: Rust Engine — Game State (DONE)

**Objective:** Mutable game state with arena-based entity system.

1. **Entity system**
   - `CardId(u32)`, `PlayerId(u32)` — typed indices into `Vec` arenas
   - `CardInstance` — mutable in-game card state (tapped, damage, counters, modifiers)
   - `PlayerState` — life, poison, lands played, hand size, loss/win flags

2. **Zone management**
   - `Zone` per (ZoneType, PlayerId) — ordered card lists
   - Zone-change state resets (untap, remove damage, reset controller)

3. **Turn structure**
   - `TurnState` — 13-phase cycle, active player, priority tracking
   - Multiplayer turn order support

4. **Stack**
   - `MagicStack` with `StackEntry` (source, controller, targets, ability text)
   - LIFO resolution

---

## Phase 3: Rust Engine — First Playable (DONE)

**Objective:** End-to-end games with combat and basic spells.

1. **Mana system**
   - `ManaPool` (WUBRG + colorless) with `can_pay()` / `try_pay()`
   - Colored-first payment algorithm for generic costs

2. **Game loop** (`GameLoop`)
   - Full turn cycle: untap → upkeep → draw → main1 → combat → main2 → end → cleanup
   - Land plays (1/turn), spell casting (pay mana → stack → resolve)
   - `PlayerAgent` trait for decisions (attacks, blocks, targets, mulligans)

3. **Combat**
   - Attack/block declaration via `PlayerAgent`
   - Damage assignment, unblocked damage to player
   - State-based actions after damage

4. **DealDamage effect**
   - Parse `SP$ DealDamage | ValidTgts$ Player | NumDmg$ 3`
   - Lightning Bolt as proof of concept

5. **CLI client** (`forge-cli`)
   - ANSI-colored terminal game, human vs simple AI
   - Board display, hand display, interactive input

6. **Integration tests**
   - 4 end-to-end scenarios (combat, blocking, damage, full game)

---

## Phase 4: Rust Engine — Keywords & Targeting (DONE)

**Objective:** Tactical depth through keywords and creature targeting.

1. **9 combat keywords**
   - Flying (only blocked by flying/reach), Reach
   - First Strike, Double Strike (two-step damage resolution with SBA between)
   - Trample (excess damage to defending player)
   - Deathtouch (1 damage kills, SBA flag), Lifelink (controller gains life)
   - Vigilance (no tap to attack), Defender (cannot attack)

2. **Creature targeting**
   - `TargetKind` enum: Player, Any, Creature(filter), None
   - `ValidTgts$ Any` — target player or creature
   - `ValidTgts$ Creature.nonBlack` — filtered creature targeting
   - Target validation before listing playable spells

3. **New spell effects**
   - Pump: `SP$ Pump | NumAtt$ +3 | NumDef$ +3` (Giant Growth)
   - Destroy: `SP$ Destroy` (Doom Blade)
   - Draw: `SP$ Draw | NumCards$ 2` (Divination)

4. **4 themed CLI decks**
   - Red Burn, Green Stompy, White Aggro, Black Control
   - Showcase all keywords and effects

---

## Phase 5: Rust Engine — Triggers

**Objective:** Event-driven trigger system matching Forge's `T$` format.

1. **Trigger infrastructure**
   - `GameEvent` enum (CardEntersBattlefield, CardDies, CardLeavesZone, DamageDealt, LifeGained, SpellCast, PhaseBegins, AttackerDeclared, etc.)
   - `TriggerDefinition` parsed from card scripts (`T$ ChangesZone | Origin$ Any | Destination$ Battlefield`)
   - Trigger registry on `GameState` — cards register triggers when entering battlefield
   - Trigger matching: event → scan registered triggers → collect matches

2. **Trigger resolution**
   - Triggered abilities go on stack (APNAP order for simultaneous triggers)
   - `StackEntry` extended with trigger source info
   - "When", "Whenever", "At" trigger types

3. **Common triggers to implement**
   - ETB (enters the battlefield): `T$ ChangesZone | Origin$ Any | Destination$ Battlefield`
   - Dies: `T$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard`
   - LTB (leaves the battlefield): `T$ ChangesZone | Origin$ Battlefield | Destination$ Any`
   - Combat damage to player: `T$ DamageDone | ValidTarget$ Player`
   - Beginning of upkeep: `T$ Phase | Phase$ Upkeep`
   - Spell cast: `T$ SpellCast`

4. **Trigger conditions**
   - `TriggerConditions$` parsing
   - "You control" / "an opponent controls" filtering
   - Card type filtering on trigger source

5. **Test cards**
   - Mulldrifter (ETB draw 2), Blood Artist (dies trigger), Llanowar Elves variant (tap trigger), Soul Warden (ETB life gain)

---

## Phase 6: Rust Engine — Static Abilities & Continuous Effects

**Objective:** Implement the layer system (CR 613) for continuous effects.

1. **Layer system**
   - 7 layers per MTG comprehensive rules:
     1. Copy effects
     2. Control-changing effects
     3. Text-changing effects
     4. Type-changing effects
     5. Color-changing effects
     6. Ability adding/removing
     7. Power/toughness (7a: CDA, 7b: set, 7c: modify, 7d: counters, 7e: switching)
   - Dependency resolution within layers
   - Timestamp ordering

2. **Static ability types**
   - Anthems: "Other creatures you control get +1/+1" (`S$ Continuous | Affected$ Creature.YouCtrl+Other | AddPower$ 1 | AddToughness$ 1`)
   - Auras: continuous effects attached to a permanent
   - Type-granting: "Creatures you control have flying"
   - Color-changing: "Target creature becomes blue"
   - Lordship: "Other Elves get +1/+1"

3. **Recalculation engine**
   - Recalculate all continuous effects when game state changes
   - Cache and invalidate efficiently

4. **Test cards**
   - Glorious Anthem (+1/+1 to your creatures), Honor of the Pure (+1/+1 to white creatures), Elvish Archdruid (lord), Pacifism (aura — can't attack/block)

---

## Phase 7: Rust Engine — Replacement Effects

**Objective:** "Instead" effects and damage prevention.

1. **Replacement effect system**
   - `R$ BeforeDraw`, `R$ DamageDone`, `R$ Destroy`, etc.
   - Replacement chain (self-replacement rules, CR 614.6)
   - Player chooses order when multiple apply

2. **Common replacements**
   - Damage prevention: "Prevent the next N damage"
   - Damage redirection: "redirect to another target"
   - Enter-with-counters: "enters the battlefield with N +1/+1 counters"
   - Draw replacement: "If you would draw a card, instead..."
   - Death replacement: "If ~ would die, exile it instead"

3. **Test cards**
   - Fog (prevent all combat damage), Rest in Peace (exile instead of graveyard), Hardened Scales (extra +1/+1 counter)

---

## Phase 8: Rust Engine — Activated Abilities

**Objective:** Tap abilities, loyalty abilities, and cost framework.

1. **Cost framework**
   - Mana costs, tap costs, sacrifice costs, life payment, discard costs
   - Cost parsing from Forge `AB$` format
   - Cost payment UI integration (PlayerAgent methods)

2. **Activated ability types**
   - Mana abilities (Llanowar Elves: `{T}: Add {G}`)
   - Damage abilities (Prodigal Sorcerer: `{T}: Deal 1 damage`)
   - Pump abilities (Nantuko Shade: `{B}: +1/+1 until end of turn`)
   - Sacrifice abilities (Sakura-Tribe Elder: sacrifice → search for land)

3. **Planeswalker rules**
   - Loyalty counters, loyalty abilities (+N/-N)
   - One loyalty ability per turn
   - Planeswalker damage redirection
   - Planeswalker uniqueness rule

4. **Test cards**
   - Llanowar Elves, Birds of Paradise, Prodigal Sorcerer, Jace Beleren (planeswalker)

---

## Phase 9: Rust Engine — API Type Expansion

**Objective:** Systematic coverage of Forge's ~150+ ability API types to reach critical mass.

1. **Priority API types** (by card coverage)
   - Counter manipulation (AddCounter, RemoveCounter, MoveCounter)
   - Token creation (Token)
   - Card selection (ChangeZone with choices — tutor, mill, exile from hand)
   - Bounce (ChangeZone back to hand)
   - Sacrifice (Sacrifice)
   - Discard (Discard, DiscardHand)
   - Life manipulation (GainLife, LoseLife, SetLife)
   - Card filtering (DigMultiple, Scry, Surveil, Reveal)

2. **Combat API types**
   - Fight, Goad, Provoke, Menace, Battle Cry, Exalted

3. **Zone manipulation**
   - Search library, mill, exile from graveyard, flashback, cascade

4. **Progress metric**
   - Track % of Forge API types covered
   - Track % of card scripts that can be fully executed
   - Target: 80%+ of commonly played cards

---

## Phase 10: WASM Bindings

**Objective:** Expose the Rust engine to JavaScript.

1. **wasm-bindgen exports**
   - `GameState` creation, serialization (JSON)
   - Action submission (play card, declare attackers, pass priority)
   - Game state queries (hand, battlefield, life totals, legal actions)
   - Card database loading from bundled card scripts

2. **TypeScript type generation**
   - `tsify` or manual TS declarations matching Rust types
   - Shared types between engine and frontend

3. **wasm-pack build pipeline**
   - `wasm-pack build --target web`
   - Integrate into Vite build (`vite-plugin-wasm`)
   - Lazy-load WASM module on game start

4. **Performance validation**
   - Benchmark: full game simulation in WASM < 100ms
   - Card database load time < 2s for 32K cards
   - Memory footprint profiling

---

## Phase 11: Web Frontend — Game UI

**Objective:** Playable browser game using the WASM engine.

1. **Game state bridge**
   - `useGameStore` (Zustand) synced with WASM `GameState`
   - Action dispatch: UI → Zustand → WASM engine → state update → React re-render
   - Game event log from engine

2. **Battlefield layout**
   - Zones: opponent hand (face-down), opponent battlefield, stack, player battlefield, player hand
   - Sidebar: life totals, mana pool, phase indicator, graveyard/exile peek
   - Card rendering: Scryfall images (lazy-loaded), text fallback

3. **Game interactions**
   - Card selection (play from hand, activate ability)
   - Targeting (click card/player, arrow overlay)
   - Combat (select attackers → confirm → select blockers → confirm)
   - Priority passing (space bar), auto-yield (F2)
   - Stack visualization with resolve/respond

4. **Phase/turn indicator**
   - Visual phase bar (Untap → Cleanup)
   - Mana pool display (WUBRG)
   - Turn counter, active player indicator

---

## Phase 12: Web Frontend — Lobby & Deck Editor

**Objective:** Pre-game experience.

1. **Deck editor**
   - Card search (by name, type, color, CMC) via card database
   - Drag-and-drop or click to add/remove cards
   - Main deck + sideboard
   - Mana curve chart, color distribution
   - Import/export (Forge `.dck` format, Arena format, clipboard)
   - Save/load from localStorage

2. **Lobby**
   - Create game (format, starting life, deck selection)
   - Join game (game list, P2P connection setup)
   - Chat (WebRTC data channel)

3. **Login/identity**
   - Username selection (stored locally)
   - Avatar/icon picker

---

## Phase 13: Networking — P2P Multiplayer

**Objective:** Peer-to-peer games with no server.

1. **WebRTC signaling**
   - Signaling server (minimal — exchange SDP offers/answers)
   - Or: manual offer/answer exchange (paste codes)
   - Or: use a free TURN/STUN relay

2. **Game state sync protocol**
   - Host runs the WASM engine, sends serialized `GameState` diffs
   - Client sends actions (play card, declare attackers, pass)
   - Deterministic replay: seed + action log = reproducible game

3. **Broadcast/spectator mode**
   - Read-only WebRTC data channel
   - Spectators receive state updates, cannot send actions
   - Join mid-game with full state snapshot

4. **Reconnection**
   - Full state snapshot on reconnect
   - Action log replay for verification

---

## Phase 14: Tauri Desktop

**Objective:** Native desktop app with the same web UI.

1. **Tauri shell**
   - Rust backend with direct engine access (no WASM overhead)
   - Web frontend loaded from local files
   - Tauri commands for file I/O (deck save/load, card script directory)

2. **Local features**
   - Local AI games (engine runs natively in Rust)
   - Deck storage on filesystem
   - Card image caching

3. **Distribution**
   - macOS, Windows, Linux builds
   - Auto-update via Tauri updater

---

## Current Status

| Phase | Status |
|---|---|
| 1. Foundation types | Done |
| 2. Game state | Done |
| 3. First playable | Done |
| 4. Keywords & targeting | Done |
| 5. Triggers | Next |
| 6. Static abilities | — |
| 7. Replacement effects | — |
| 8. Activated abilities | — |
| 9. API type expansion | — |
| 10. WASM bindings | — |
| 11. Game UI | — |
| 12. Lobby & deck editor | — |
| 13. P2P networking | — |
| 14. Tauri desktop | — |
