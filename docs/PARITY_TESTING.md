# Parity Testing Guide

How to run parity tests comparing the Rust engine against the Java Forge reference implementation.

## Prerequisites

1. **Java 18** (Zulu recommended):
   ```bash
   export JAVA_HOME=/Library/Java/JavaVirtualMachines/zulu-18.jdk/Contents/Home
   ```

2. **Build the Java harness JAR** (from repo root):
   ```bash
   cd forge && JAVA_HOME=/Library/Java/JavaVirtualMachines/zulu-18.jdk/Contents/Home \
     mvn -pl forge-harness -am -DskipTests package
   ```
   This produces: `forge/forge-harness/target/forge-harness-jar-with-dependencies.jar`

## Common Commands

All commands run from the repo root (`/Users/emanueledivizio/dev/mtg/bardidinaXmageUI`).

### Full 7-deck parity matrix (3 seeds each = 126 matchups)

```bash
JAVA_HOME=/Library/Java/JavaVirtualMachines/zulu-18.jdk/Contents/Home \
  cargo run -p forge-parity -- \
  --matrix --seeds 42,100,999 \
  --decks red_burn,green_stompy,white_aggro,black_control,comprehensive_test,trigger_expanded,staticability_test \
  --cards-dir forge/forge-gui/res/cardsfolder \
  --java-jar forge/forge-harness/target/forge-harness-jar-with-dependencies.jar
```

### Single matchup (verbose)

```bash
JAVA_HOME=/Library/Java/JavaVirtualMachines/zulu-18.jdk/Contents/Home \
  cargo run -p forge-parity -- \
  --deck1 <DECK1> --deck2 <DECK2> \
  --seed <SEED> --max-turns 30 -v \
  --cards-dir forge/forge-gui/res/cardsfolder \
  --java-jar forge/forge-harness/target/forge-harness-jar-with-dependencies.jar
```

### Staticability mirror test

```bash
JAVA_HOME=/Library/Java/JavaVirtualMachines/zulu-18.jdk/Contents/Home \
  cargo run -p forge-parity -- \
  --deck1 staticability_test --deck2 staticability_test \
  --max-turns 30 -v \
  --java-jar forge/forge-harness/target/forge-harness-jar-with-dependencies.jar
```

### Run only non-black_control matchups (avoids Hypnotic Specter RNG gap)

```bash
JAVA_HOME=/Library/Java/JavaVirtualMachines/zulu-18.jdk/Contents/Home \
  cargo run -p forge-parity -- \
  --matrix --seeds 42,100,999 \
  --decks red_burn,green_stompy,white_aggro,comprehensive_test,trigger_expanded,staticability_test \
  --cards-dir forge/forge-gui/res/cardsfolder \
  --java-jar forge/forge-harness/target/forge-harness-jar-with-dependencies.jar
```

## Available Decks

| Deck | Focus |
|------|-------|
| `red_burn` | Direct damage, aggro |
| `green_stompy` | Big creatures, ramp |
| `white_aggro` | Tokens, anthems |
| `black_control` | Discard, removal (has Hypnotic Specter RNG gap) |
| `comprehensive_test` | Broad mechanics coverage (pain lands, charms, kicker, etc.) |
| `trigger_expanded` | Trigger-heavy (ETB, cast, damage, surveil) |
| `staticability_test` | Static abilities (anthems, protection, etc.) |

## Known Limitations

- **Hypnotic Specter** (in `black_control`): Random discard uses Java's `MyRandom` which is separate from the shared agent RNG. This causes inherent divergence in all `black_control` matchups (~12 failures expected).
- **Flashback**: Graveyard flashback is intentionally excluded from the deterministic action list to match Java's `chooseSpellAbilityToPlay()` which only queries Hand and Battlefield.

## Interpreting Results

- **PASS**: Rust and Java produce identical game state snapshots at each turn
- **FAIL**: Divergence detected — the output shows the first turn where states differ, with details on what's different (life totals, hand sizes, battlefield, etc.)

## Troubleshooting

If the JAR doesn't exist or is stale, rebuild it:
```bash
cd forge && JAVA_HOME=/Library/Java/JavaVirtualMachines/zulu-18.jdk/Contents/Home \
  mvn -pl forge-harness -am -DskipTests package
```

If `cargo run` fails to compile, check:
```bash
cargo check -p forge-engine-core
cargo check -p forge-parity
```
