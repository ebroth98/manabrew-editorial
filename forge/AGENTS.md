# Java Forge — read-only reference

This directory is the upstream [Forge](https://github.com/Card-Forge/forge) source tree, vendored as the **source of truth** for parity work. Read first: `/AGENTS.md`, `docs/agents/PARITY_PHILOSOPHY.md`.

## Do not edit

**With one exception, never modify any file under `forge/`.**

- The directory tracks `Card-Forge/forge` as an upstream remote. We pull updates periodically (see the README's "Syncing with Upstream Forge" section). Local edits create merge conflicts on every sync.
- The Rust engine in `forge-engine/` is a port of `forge/forge-game/`. Diverging the Java side breaks the parity contract.
- Bug reports against Forge go upstream, not into this tree.

## The exception: `forge/forge-harness/`

`forge/forge-harness/` is **ours**, not upstream. It's the Java-side bridge consumed by `forge-engine/crates/forge-parity/` to run Java and Rust side-by-side. Editing it is fine when:

- A parity test needs a new entry point into Forge.
- A new field needs to be exposed in the trace.
- A Forge upstream change broke the harness.

Build with `yarn build:harness` (uses JDK 18 — see the README for the `JAVA_HOME` setup). Don't change existing method signatures lightly — they're consumed via FFI from Rust and renaming breaks every parity test.

## How to use this directory as a reference

When porting or fixing engine code, the relevant files are usually under:

| Path | Purpose |
|---|---|
| `forge/forge-game/src/main/java/forge/game/` | The rules engine — what `forge-engine/crates/forge-engine/src/` mirrors. |
| `forge/forge-core/src/main/java/forge/` | Core types — what `forge-foundation` mirrors. |
| `forge/forge-gui/res/cardsfolder/` | The 32K+ `.txt` card scripts — parsed at runtime by `forge-carddb`. |
| `forge/forge-gui/res/tokenscripts/`, `editions/`, `lists/` | Token definitions, set data, type lists. |

For any Rust file you're editing, the Java counterpart is the first thing to read.
