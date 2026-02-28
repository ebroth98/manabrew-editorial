# forge-parity

Cross-engine differential testing tool that compares the Rust Forge engine against the Java reference implementation snapshot-by-snapshot.

## Building

```bash
cargo build -p forge-parity
```

For the Java harness (required for full parity mode):

```bash
cd forge/forge-harness
mvn package -DskipTests
```

## Usage

### Rust-only mode (default)

Runs the Rust engine and dumps per-phase JSONL snapshots. Useful for debugging and golden file generation.

```bash
cargo run -p forge-parity -- --deck1 red_burn --deck2 green_stompy
```

### Full parity mode

Runs both engines with identical inputs and compares snapshots field-by-field.

```bash
cargo run -p forge-parity -- \
  --deck1 red_burn --deck2 green_stompy \
  --java-jar forge/forge-harness/target/forge-harness-jar-with-dependencies.jar
```

### Matrix mode

Runs all deck pair combinations across multiple seeds in parallel (uses rayon).

```bash
cargo run -p forge-parity -- --matrix \
  --java-jar forge/forge-harness/target/forge-harness-jar-with-dependencies.jar
```

## Flags

| Flag | Default | Description |
|------|---------|-------------|
| `--deck1 <name>` | `red_burn` | Preset deck for player 1 |
| `--deck2 <name>` | `green_stompy` | Preset deck for player 2 |
| `--seed <N>` | `42` | RNG seed for reproducibility |
| `--max-turns <N>` | `10` | Maximum turns before stopping |
| `--java-jar <path>` | _(none)_ | Path to Java harness JAR (enables parity mode) |
| `--cards-dir <path>` | _(none)_ | Path to Forge card scripts directory |
| `--output <path>` / `-o` | _(stdout)_ | Write report to file instead of stdout |
| `--format <fmt>` | `text` | Output format: `text` or `json` |
| `--verbose` / `-v` | `false` | Print agent decisions to stderr |
| `--matrix` | `false` | Run all deck pairs × seeds |
| `--seeds <list>` | `42,100,999` | Comma-separated seeds for matrix mode |
| `--decks <list>` | _(all presets)_ | Comma-separated deck names for matrix mode |

## Available preset decks

| Deck | Focus |
|------|-------|
| `red_burn` | Direct damage spells, haste creatures |
| `green_stompy` | Big creatures, fight effects |
| `white_aggro` | Small creatures, tokens, lifegain |
| `black_control` | Removal, discard, sacrifice |
| `blue_control` | Counterspells, draw, bounce |
| `comprehensive_test` | Mixed card types for broad coverage |
| `zone_change`` | ETB/LTB/dies triggers, zone movement |
| `token_swarm` | Token generation and anthem effects |
| `library_manipulation` | Scry, surveil, tutors |
| `green_fight` | Fight and bite effects |
| `mass_effects` | Board wipes, mass pump |
| `trigger_test` | Core trigger types |
| `keyword_test` | Keyword abilities (flying, trample, etc.) |
| `trigger_expanded` | Extended trigger coverage (68 types) |

## Common commands

```bash
# Quick smoke test — Rust only
cargo run -p forge-parity

# Full parity check with a specific seed
cargo run -p forge-parity -- \
  --deck1 trigger_expanded --deck2 comprehensive_test --seed 42 \
  --java-jar forge/forge-harness/target/forge-harness-jar-with-dependencies.jar

# JSON output for CI
cargo run -p forge-parity -- \
  --deck1 red_burn --deck2 green_stompy --format json \
  --java-jar forge/forge-harness/target/forge-harness-jar-with-dependencies.jar

# Matrix — all decks, custom seeds, save report
cargo run -p forge-parity -- --matrix --seeds 42,100,200,300 \
  --java-jar forge/forge-harness/target/forge-harness-jar-with-dependencies.jar \
  -o parity-report.txt

# Matrix — specific decks only
cargo run -p forge-parity -- --matrix \
  --decks red_burn,green_stompy,white_aggro \
  --java-jar forge/forge-harness/target/forge-harness-jar-with-dependencies.jar

# Verbose mode — see every agent decision
cargo run -p forge-parity -- -v --deck1 trigger_test --deck2 keyword_test

# Rust-only with more turns
cargo run -p forge-parity -- --deck1 black_control --deck2 blue_control --max-turns 20
```

## Exit codes

- `0` — All snapshots match (parity pass) or Rust-only completed successfully
- `1` — Divergence detected or engine error
