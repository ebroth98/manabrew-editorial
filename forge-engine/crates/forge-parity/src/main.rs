//! CLI entry point for `forge-parity`.
//!
//! ```text
//! forge-parity --deck1 <name> --deck2 <name> [--seed N] [--max-turns N]
//!              [--java-jar <path>]
//!              [--output <path>] [--format json|text] [--verbose]
//! ```
//!
//! **Rust-only mode** (default, no `--java-jar`):
//! Dumps per-phase JSONL snapshots. Useful for golden files and debugging.
//!
//! **Full parity mode** (`--java-jar`):
//! Runs both engines, compares snapshots, reports divergences.

use std::path::PathBuf;

use clap::Parser;

use forge_parity::comparator;
use forge_parity::java_bridge::{JavaBridge, JavaBridgeConfig};
use forge_parity::report;
use forge_parity::runner::{self, RunConfig};

#[derive(Parser, Debug)]
#[command(
    name = "forge-parity",
    about = "Cross-engine differential testing for Forge MTG engine"
)]
struct Cli {
    /// Preset deck name for player 1
    #[arg(long, default_value = "red_burn")]
    deck1: String,

    /// Preset deck name for player 2
    #[arg(long, default_value = "green_stompy")]
    deck2: String,

    /// RNG seed for reproducibility
    #[arg(long, default_value_t = 42)]
    seed: u64,

    /// Maximum number of turns before stopping
    #[arg(long, default_value_t = 10)]
    max_turns: u32,

    /// Path to the Java Forge harness JAR (enables full parity mode)
    #[arg(long)]
    java_jar: Option<PathBuf>,

    /// Path to the Forge card scripts directory
    #[arg(long)]
    cards_dir: Option<String>,

    /// Output file path (stdout if not specified)
    #[arg(long, short)]
    output: Option<PathBuf>,

    /// Output format: "json" or "text"
    #[arg(long, default_value = "text")]
    format: String,

    /// Verbose output (print decisions to stderr)
    #[arg(long, short)]
    verbose: bool,
}

fn main() {
    let cli = Cli::parse();

    eprintln!(
        "[parity] Running: {} vs {} | seed={} | max_turns={}",
        cli.deck1, cli.deck2, cli.seed, cli.max_turns
    );

    if let Some(ref jar_path) = cli.java_jar {
        // Full parity mode: run both engines and compare
        run_parity_mode(&cli, jar_path);
    } else {
        // Rust-only mode: dump snapshots
        run_rust_only_mode(&cli);
    }
}

fn run_rust_only_mode(cli: &Cli) {
    let config = RunConfig {
        deck1: cli.deck1.clone(),
        deck2: cli.deck2.clone(),
        seed: cli.seed,
        max_turns: cli.max_turns,
        cards_dir: cli.cards_dir.clone(),
        verbose: cli.verbose,
    };

    match runner::run_rust_only(&config) {
        Ok(trace) => {
            let output = match cli.format.as_str() {
                "json" => serde_json::to_string_pretty(&trace)
                    .unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
                _ => report::format_trace_text(&trace),
            };

            write_output(cli, &output);

            eprintln!(
                "[parity] Done: {} snapshots collected",
                trace.snapshots.len()
            );
        }
        Err(e) => {
            eprintln!("[parity] Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn run_parity_mode(cli: &Cli, jar_path: &PathBuf) {
    eprintln!("[parity] Full parity mode — running both engines");

    // 1. Run Rust engine
    eprintln!("[parity] Running Rust engine...");
    let config = RunConfig {
        deck1: cli.deck1.clone(),
        deck2: cli.deck2.clone(),
        seed: cli.seed,
        max_turns: cli.max_turns,
        cards_dir: cli.cards_dir.clone(),
        verbose: cli.verbose,
    };

    let rust_trace = match runner::run_rust_only(&config) {
        Ok(trace) => trace,
        Err(e) => {
            eprintln!("[parity] Rust engine error: {}", e);
            std::process::exit(1);
        }
    };
    eprintln!(
        "[parity] Rust engine: {} snapshots",
        rust_trace.snapshots.len()
    );

    // 2. Run Java engine
    eprintln!("[parity] Running Java engine...");
    let bridge_config = JavaBridgeConfig {
        jar_path: jar_path.clone(),
        seed: cli.seed,
        max_turns: cli.max_turns,
        deck1: cli.deck1.clone(),
        deck2: cli.deck2.clone(),
        forge_home: None, // auto-detected from JAR path
    };

    let bridge = JavaBridge::new(bridge_config);
    let java_snapshots = match bridge.run() {
        Ok(snaps) => snaps,
        Err(e) => {
            eprintln!("[parity] Java engine error: {}", e);
            std::process::exit(1);
        }
    };
    eprintln!("[parity] Java engine: {} snapshots", java_snapshots.len());

    // 3. Compare snapshots
    let max_snapshots = rust_trace.snapshots.len().max(java_snapshots.len());
    let mut all_divergences = Vec::new();

    for i in 0..max_snapshots {
        match (rust_trace.snapshots.get(i), java_snapshots.get(i)) {
            (Some(rs), Some(js)) => {
                let divs = comparator::compare(i, rs, js);
                all_divergences.extend(divs);
            }
            (Some(_rs), None) => {
                all_divergences.push(forge_parity::protocol::Divergence {
                    snapshot_index: i,
                    turn: _rs.turn,
                    phase: _rs.phase.clone(),
                    field: "snapshot.exists".into(),
                    rust_value: "present".into(),
                    java_value: "missing".into(),
                });
            }
            (None, Some(_js)) => {
                all_divergences.push(forge_parity::protocol::Divergence {
                    snapshot_index: i,
                    turn: _js.turn,
                    phase: _js.phase.clone(),
                    field: "snapshot.exists".into(),
                    rust_value: "missing".into(),
                    java_value: "present".into(),
                });
            }
            (None, None) => {}
        }
    }

    // 4. Build and output report
    let parity_report = report::build_report(&rust_trace, all_divergences);

    let output = match cli.format.as_str() {
        "json" => report::format_json(&parity_report),
        _ => report::format_text(&parity_report),
    };

    write_output(cli, &output);

    if parity_report.passed {
        eprintln!("[parity] PASS — engines agree on all {} snapshots", max_snapshots);
    } else {
        eprintln!(
            "[parity] FAIL — {} divergence(s) found across {} snapshots",
            parity_report.divergences.len(),
            max_snapshots
        );
        std::process::exit(1);
    }
}

fn write_output(cli: &Cli, output: &str) {
    if let Some(ref path) = cli.output {
        match std::fs::write(path, output) {
            Ok(_) => eprintln!("[parity] Report written to {:?}", path),
            Err(e) => {
                eprintln!("[parity] Failed to write report: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        println!("{}", output);
    }
}
