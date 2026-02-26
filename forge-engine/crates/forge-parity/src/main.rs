//! CLI entry point for `forge-parity`.
//!
//! ```text
//! forge-parity --deck1 <name> --deck2 <name> [--seed N] [--max-turns N]
//!              [--java-jar <path>]
//!              [--output <path>] [--format json|text] [--verbose]
//!              [--matrix] [--seeds 42,100,999] [--decks red_burn,green_stompy]
//! ```
//!
//! **Rust-only mode** (default, no `--java-jar`):
//! Dumps per-phase JSONL snapshots. Useful for golden files and debugging.
//!
//! **Full parity mode** (`--java-jar`):
//! Runs both engines, compares snapshots, reports divergences.
//!
//! **Matrix mode** (`--matrix`):
//! Runs all deck pair combinations across multiple seeds.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use clap::Parser;
use rayon::prelude::*;

use forge_parity::comparator;
use forge_parity::java_bridge::{JavaBridge, JavaBridgeConfig};
use forge_parity::protocol::{
    Divergence, MatchupResult, MatchupStatus, MatrixReport,
};
use forge_parity::report;
use forge_parity::runner::{self, available_presets, LoadedData, RunConfig};

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

    /// Run all deck pair combinations across multiple seeds
    #[arg(long)]
    matrix: bool,

    /// Comma-separated seeds for matrix mode (default: 42,100,999)
    #[arg(long, value_delimiter = ',')]
    seeds: Option<Vec<u64>>,

    /// Comma-separated deck names for matrix mode (default: all presets)
    #[arg(long, value_delimiter = ',')]
    decks: Option<Vec<String>>,
}

fn main() {
    let cli = Cli::parse();

    if cli.matrix {
        run_matrix_mode(&cli);
    } else {
        eprintln!(
            "[parity] Running: {} vs {} | seed={} | max_turns={}",
            cli.deck1, cli.deck2, cli.seed, cli.max_turns
        );

        if let Some(ref jar_path) = cli.java_jar {
            run_parity_mode(&cli, jar_path);
        } else {
            run_rust_only_mode(&cli);
        }
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

    let config = RunConfig {
        deck1: cli.deck1.clone(),
        deck2: cli.deck2.clone(),
        seed: cli.seed,
        max_turns: cli.max_turns,
        cards_dir: cli.cards_dir.clone(),
        verbose: cli.verbose,
    };

    let data = match runner::load_data(config.cards_dir.as_deref()) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[parity] Load error: {}", e);
            std::process::exit(1);
        }
    };

    let result = run_single_matchup(&config, &data, Some(jar_path));

    let parity_report = report::build_report(
        // Build a minimal trace for the report
        &forge_parity::protocol::GameTrace {
            seed: config.seed,
            deck1: config.deck1.clone(),
            deck2: config.deck2.clone(),
            max_turns: config.max_turns,
            snapshots: vec![], // not used by build_report beyond len
        },
        match &result.status {
            MatchupStatus::Pass => vec![],
            _ => result.first_divergence.clone().into_iter().collect(),
        },
    );

    // Override total_snapshots from the actual result
    let parity_report = forge_parity::protocol::ParityReport {
        total_snapshots: result.snapshots_compared,
        ..parity_report
    };

    let output = match cli.format.as_str() {
        "json" => report::format_json(&parity_report),
        _ => report::format_text(&parity_report),
    };

    write_output(cli, &output);

    if result.status == MatchupStatus::Pass {
        eprintln!(
            "[parity] PASS — engines agree on all {} snapshots",
            result.snapshots_compared
        );
    } else {
        eprintln!(
            "[parity] FAIL — {} divergence(s) found across {} snapshots",
            result.divergence_count, result.snapshots_compared
        );
        std::process::exit(1);
    }
}

/// Run a single matchup (Rust, and optionally Java) and return a structured result.
fn run_single_matchup(
    config: &RunConfig,
    data: &LoadedData,
    jar_path: Option<&PathBuf>,
) -> MatchupResult {
    // Run Rust engine
    let rust_trace = match runner::run_with_data(config, data) {
        Ok(trace) => trace,
        Err(e) => {
            return MatchupResult {
                deck1: config.deck1.clone(),
                deck2: config.deck2.clone(),
                seed: config.seed,
                status: MatchupStatus::Error,
                snapshots_compared: 0,
                divergence_count: 0,
                first_divergence: None,
                error_message: Some(format!("Rust engine error: {}", e)),
            };
        }
    };

    // If no Java JAR, it's Rust-only — just check it didn't panic
    let jar_path = match jar_path {
        Some(p) => p,
        None => {
            return MatchupResult {
                deck1: config.deck1.clone(),
                deck2: config.deck2.clone(),
                seed: config.seed,
                status: MatchupStatus::Pass,
                snapshots_compared: rust_trace.snapshots.len(),
                divergence_count: 0,
                first_divergence: None,
                error_message: None,
            };
        }
    };

    // Run Java engine
    let bridge_config = JavaBridgeConfig {
        jar_path: jar_path.clone(),
        seed: config.seed,
        max_turns: config.max_turns,
        deck1: config.deck1.clone(),
        deck2: config.deck2.clone(),
        forge_home: None,
    };

    let bridge = JavaBridge::new(bridge_config);
    let java_snapshots = match bridge.run() {
        Ok(snaps) => snaps,
        Err(e) => {
            return MatchupResult {
                deck1: config.deck1.clone(),
                deck2: config.deck2.clone(),
                seed: config.seed,
                status: MatchupStatus::Error,
                snapshots_compared: 0,
                divergence_count: 0,
                first_divergence: None,
                error_message: Some(format!("Java engine error: {}", e)),
            };
        }
    };

    // Compare snapshots
    let max_snapshots = rust_trace.snapshots.len().max(java_snapshots.len());
    let mut all_divergences: Vec<Divergence> = Vec::new();

    for i in 0..max_snapshots {
        match (rust_trace.snapshots.get(i), java_snapshots.get(i)) {
            (Some(rs), Some(js)) => {
                let divs = comparator::compare(i, rs, js);
                all_divergences.extend(divs);
            }
            (Some(_rs), None) => {
                all_divergences.push(Divergence {
                    snapshot_index: i,
                    turn: _rs.turn,
                    phase: _rs.phase.clone(),
                    field: "snapshot.exists".into(),
                    rust_value: "present".into(),
                    java_value: "missing".into(),
                });
            }
            (None, Some(_js)) => {
                all_divergences.push(Divergence {
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

    let divergence_count = all_divergences.len();
    let first_divergence = all_divergences.into_iter().next();
    let status = if divergence_count == 0 {
        MatchupStatus::Pass
    } else {
        MatchupStatus::Fail
    };

    MatchupResult {
        deck1: config.deck1.clone(),
        deck2: config.deck2.clone(),
        seed: config.seed,
        status,
        snapshots_compared: max_snapshots,
        divergence_count,
        first_divergence,
        error_message: None,
    }
}

fn run_matrix_mode(cli: &Cli) {
    let seeds = cli.seeds.clone().unwrap_or_else(|| vec![42, 100, 999]);
    let deck_names: Vec<String> = cli
        .decks
        .clone()
        .unwrap_or_else(|| available_presets().into_iter().map(String::from).collect());

    // Validate deck names
    let valid = available_presets();
    for d in &deck_names {
        if !valid.contains(&d.as_str()) {
            eprintln!(
                "[parity] Unknown deck '{}'. Available: {:?}",
                d, valid
            );
            std::process::exit(1);
        }
    }

    // Build ordered pairs (d1, d2) where d1 != d2
    let mut pairs: Vec<(&str, &str)> = Vec::new();
    for d1 in &deck_names {
        for d2 in &deck_names {
            if d1 != d2 {
                pairs.push((d1, d2));
            }
        }
    }

    let total = pairs.len() * seeds.len();
    eprintln!(
        "[parity] Matrix mode: {} decks × {} seeds = {} matchups",
        deck_names.len(),
        seeds.len(),
        total
    );

    // Load data once
    let data = match runner::load_data(cli.cards_dir.as_deref()) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[parity] Load error: {}", e);
            std::process::exit(1);
        }
    };

    // Build flat list of (d1, d2, seed) jobs for parallel execution
    let jobs: Vec<(&str, &str, u64)> = pairs
        .iter()
        .flat_map(|&(d1, d2)| seeds.iter().map(move |&s| (d1, d2, s)))
        .collect();

    let completed = AtomicUsize::new(0);

    let results: Vec<MatchupResult> = jobs
        .par_iter()
        .map(|&(d1, d2, seed)| {
            let config = RunConfig {
                deck1: d1.to_string(),
                deck2: d2.to_string(),
                seed,
                max_turns: cli.max_turns,
                cards_dir: cli.cards_dir.clone(),
                verbose: cli.verbose,
            };

            let result = run_single_matchup(&config, &data, cli.java_jar.as_ref());

            let n = completed.fetch_add(1, Ordering::Relaxed) + 1;
            match result.status {
                MatchupStatus::Pass => {
                    eprintln!(
                        "[parity] [{}/{}] {} vs {} seed={} ... PASS ({} snapshots)",
                        n, total, d1, d2, seed, result.snapshots_compared
                    );
                }
                MatchupStatus::Fail => {
                    eprintln!(
                        "[parity] [{}/{}] {} vs {} seed={} ... FAIL ({} divergences)",
                        n, total, d1, d2, seed, result.divergence_count
                    );
                }
                MatchupStatus::Error => {
                    eprintln!(
                        "[parity] [{}/{}] {} vs {} seed={} ... ERROR: {}",
                        n, total, d1, d2, seed,
                        result.error_message.as_deref().unwrap_or("unknown")
                    );
                }
            }

            result
        })
        .collect();

    let passed = results.iter().filter(|r| r.status == MatchupStatus::Pass).count();
    let failed = results.iter().filter(|r| r.status == MatchupStatus::Fail).count();
    let errors = results.iter().filter(|r| r.status == MatchupStatus::Error).count();

    let matrix_report = MatrixReport {
        total_matchups: total,
        passed,
        failed,
        errors,
        seeds: seeds.clone(),
        decks: deck_names.clone(),
        max_turns: cli.max_turns,
        results,
    };

    let output = match cli.format.as_str() {
        "json" => report::format_matrix_json(&matrix_report),
        _ => report::format_matrix_text(&matrix_report),
    };

    write_output(cli, &output);

    if failed > 0 || errors > 0 {
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
