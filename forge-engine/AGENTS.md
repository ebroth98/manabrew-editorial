# Rust engine workspace

This is the Rust port of Forge's Java rules engine. Read the root `/AGENTS.md` first, plus `docs/agents/PARITY_PHILOSOPHY.md`. Most parity work bottoms out here.

## Required reading: the DSL spec

Card scripts are the source of truth for behavior; the engine evaluates them. Before any work that touches abilities, triggers, replacements, static abilities, costs, the stack, SVars, selectors, or layered effects, read:

- **`docs/forge-dsl-semantics.md`** — runtime model: execution pipeline, stack semantics, targeting, cost payment, layer system, SVar resolution, determinism invariants, known CR deviations.
- **`docs/forge-dsl-grammar.md`** — syntax / parsing model. Read this when working on the parser, the IR, or interpreting an unfamiliar param.

When you write or change engine logic, write IRs and resolution code that mirror the semantics in the spec exactly. If your implementation diverges from the spec, either the implementation is wrong or the spec needs updating — don't silently drift.

## Crate map

All under `forge-engine/crates/`:

| Crate | Role | Java mirror |
|---|---|---|
| `forge-foundation` | Pure types: colors, mana, card types, zones, phases. No I/O. | `forge/forge-core/` |
| `forge-card-script` | Card script IR shared between parser and engine. | — |
| `forge-cardset-archive` | rkyv archive format for the pre-compiled cardset, plus a `build-cardset-archive` binary. Consumed by `forge-carddb` for ~6× faster cold start in packaged builds. | — |
| `forge-carddb` | Parses Forge's 32K+ `.txt` card scripts into `CardRules`, with an rkyv fast-path via `forge-cardset-archive`. WASM-safe (no `std::fs`). | `forge/forge-game/.../CardRules.java` etc. |
| `forge-engine-macros` | Build-time proc macros. No runtime logic. | — |
| **`forge-engine`** | The rules engine. Game state, turn loop, combat, stack, abilities, replacements, triggers, statics, costs, mana. | `forge/forge-game/src/main/java/forge/game/` |
| `forge-agent-interface` | DTO layer between the engine and any agent (UI, AI, network). Defines the prompt protocol. | — |
| `forge-game-runtime` | Hosts a game session over an mpsc transport. Glue. | — |
| `forge-parity` | Differential test harness — runs Rust and Java side-by-side and compares. See its own AGENTS.md. | — |
| `forge-server` | Standalone matchmaking/lobby server. Optional. | — |
| `forge-wasm` | wasm-bindgen exports for browser builds. | — |
| `self-hosted-node` | Headless engine node (AI bots, automation). | — |

Dependency direction: `foundation`, `card-script`, `cardset-archive` ← `carddb` ← `engine` ← everything else. Don't introduce cycles.

## `forge-engine` crate — module map

The engine itself lives at `forge-engine/crates/forge-engine/src/`. Each module mirrors a Java package under `forge/forge-game/src/main/java/forge/game/`.

| Module | What it owns |
|---|---|
| `game.rs`, `core.rs`, `action.rs` | Central `Game` state, state-based actions, top-level mutations. |
| `game_loop/` | Turn driver, priority loop, stack resolution, phase handler, action_space (legal-action enumeration). The hot path. |
| `phase/` | Phase types, extra phases, extra turns, untap step. |
| `ability/` | `SpellAbility` factory, IR, dispatch. `api_type.rs` lists every supported `SP$/AB$/DB$` API. |
| `ability/effects/` | The 200+ `*_effect.rs` resolvers. **Most parity bugs land here.** Has its own AGENTS.md. |
| `spellability/` | `SpellAbility` family — targeting, restrictions, conditions, optional costs. |
| `staticability/` | Continuous/static effects with the CR 613 layer system. |
| `replacement/` | Replacement effects (37 types). Each `replace_*.rs` provides `can_replace` + `execute` + a callsite into the loop. |
| `trigger/` | Triggered abilities (~120 `trigger_*.rs`). Each maps to a Forge `Mode$`. |
| `cost/` | Cost framework (`CostX` family), payment, decision-maker, visitor. |
| `combat/` | Attack/block declaration, constraints, damage assignment. |
| `mana/` | Mana pool, payment, conversion matrix, auto-pay. |
| `card/` | `CardInstance`, collections, perpetual effects (`perpetual/`), tokens (`token/`), card factory. |
| `player/` | `PlayerState`, life, statistics, predicates, controller, plus `actions/` (typed player actions). |
| `zone/` | `MagicStack`, cost-payment stack. (Zone *types* live in `forge-foundation`.) |
| `keyword/` | Static keywords (Flying, Trample, Suspend, Kicker, Modular, …). One file per keyword. |
| `agent/` | `PlayerAgent` trait and helpers. The engine never decides — it asks the agent. |
| `parsing/` | Engine-side parsers (amount expressions, comparators, key/value). Distinct from `forge-carddb`. |
| `svar/` | Forge SVar evaluator (variables embedded in scripts). |
| `mulligan/` | Opening hand / London mulligan. |
| `lki.rs`, `game_snapshot.rs`, `game_log*` | Bookkeeping: last-known-info, snapshots, log entries. |

## Symptom → folder

Use this when a parity report points at a specific failure mode:

| Symptom | Likely folder |
|---|---|
| Wrong life total / wrong damage | `replacement/` (prevention/redirection), `combat/`, `ability/effects/damage_*` |
| Trigger fires when it shouldn't / doesn't fire | `trigger/`, `game_loop/trigger_handler.rs`, `staticability/static_ability_disable_triggers.rs` |
| Continuous effect (anthem, protection) wrong | `staticability/` + `staticability/layer.rs` |
| Cost paid incorrectly | `cost/`, `game_loop/cost_payment.rs`, `mana/` |
| Spell resolves wrong | `ability/effects/<api>_effect.rs` |
| Attack/block illegal or missing | `combat/attack_constraints.rs`, `combat/attack_restriction*.rs` |
| Zone change misroutes | `replacement/replace_moved.rs`, `ability/effects/change_zone_effect/` |
| Wrong card available to play | `game_loop/playability.rs`, `card/card_play_option.rs` |

## Conventions

- **Mirror Java exactly.** See `docs/agents/PARITY_PHILOSOPHY.md`.
- **One Forge concept per file.** New effects, triggers, replacements, statics, keywords each get their own file named after the Java class, registered in their module's `mod.rs`.
- **`Game` owns everything.** All mutation flows through `&mut Game`. No interior mutability, no shared references.
- **Crate-wide `#![allow(...)]`s in `lib.rs` are intentional.** They document parity trade-offs.
- **Minimize SVar parsing — this is critical.** SVar resolution is late-bound by spec (`docs/forge-dsl-semantics.md` §5.2). Resolve SVars at the call site, lazily, and only for the SVar the engine actually needs. Do not eagerly walk the SVar graph at card construction or ability registration time. Do not pre-expand sub-ability chains into IR before they are reached. Do not re-parse the same SVar string on every reference — parse once, then reuse the parsed form. Aggressive eager parsing inflates load time, breaks the lazy semantics the spec relies on, and produces stale IR when card state changes (transform, copy, exile) alter the available SVars per invariant §10.7.

## Build & test

```bash
cargo build --workspace
cargo test --workspace
cargo check -p forge-engine
yarn parity <test-name>          # see forge-parity AGENTS.md
yarn scan                         # Java vs Rust file coverage
```
