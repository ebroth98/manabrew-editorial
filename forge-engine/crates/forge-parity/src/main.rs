//! CLI entry point for `forge-parity`.
//!
//! ```text
//! forge-parity --deck1 <name> --deck2 <name> [--seed N] [--max-turns N]
//!              [--games N]
//!              [--java-jar <path>]
//!              [--output <path>] [--format json|text] [--verbose]
//!              [--matrix] [--seeds 42,100,999] [--decks red_burn,green_stompy]
//!              [--java-workers N]
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
//!
//! **Fuzz mode** (`--fuzz`):
//! Generates random decks from the parseable card pool and runs parity tests.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use clap::Parser;
use rayon::prelude::*;

use forge_carddb::CardDatabase;
use forge_parity::card_pool::CardPool;
use forge_parity::comparator;
use forge_parity::deck_generator;
use forge_parity::java_bridge::{JavaBridge, JavaBridgeConfig, JavaBridgeError, JavaServer, JavaServerConfig};
use forge_parity::java_random::JavaRandom;
use forge_parity::protocol::{
    Divergence, FuzzReport, FuzzResult, MatchupResult, MatchupStatus, MatrixReport, MechanicSignal,
};
use forge_parity::report;
use forge_parity::runner::{self, available_presets, LoadedData, RunConfig};

#[derive(Parser, Debug)]
#[command(
    name = "forge-parity",
    about = "Cross-engine differential testing for Forge MTG engine"
)]
struct Cli {
    /// Deck for player 1: preset name, "file:path/to/deck.txt", or "inline:Name*Count|..."
    #[arg(long, default_value = "red_burn")]
    deck1: String,

    /// Deck for player 2: preset name, "file:path/to/deck.txt", or "inline:Name*Count|..."
    #[arg(long, default_value = "green_stompy")]
    deck2: String,

    /// RNG seed for reproducibility
    #[arg(long, default_value_t = 42)]
    seed: u64,

    /// Maximum number of turns before stopping
    #[arg(long, default_value_t = 10)]
    max_turns: u32,

    /// Number of games to run (single-match mode only); seeds increment from --seed
    #[arg(long, default_value_t = 1)]
    games: usize,

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

    /// Verbose output (log step-by-step decisions and per-game progress)
    #[arg(long, short)]
    verbose: bool,

    /// Bias random main-phase decisions toward taking an action instead of passing.
    #[arg(long)]
    prefer_actions: bool,

    /// Run all deck pair combinations across multiple seeds
    #[arg(long)]
    matrix: bool,

    /// Comma-separated seeds for matrix mode (default: 42,100,999)
    #[arg(long, value_delimiter = ',')]
    seeds: Option<Vec<u64>>,

    /// Comma-separated deck names for matrix mode (default: all presets)
    #[arg(long, value_delimiter = ',')]
    decks: Option<Vec<String>>,

    /// Run fuzz random deck testing
    #[arg(long)]
    fuzz: bool,

    /// Number of fuzz iterations (default: 100)
    #[arg(long, default_value_t = 100)]
    iterations: usize,

    /// Master seed for fuzz reproducibility (default: 42)
    #[arg(long, default_value_t = 42)]
    master_seed: u64,

    /// Number of Java server worker processes (default: 1 for fuzz/single, num_cpus for matrix)
    #[arg(long)]
    java_workers: Option<usize>,
}

fn main() {
    let cli = Cli::parse();
    let games_flag_present = std::env::args()
        .any(|arg| arg == "--games" || arg.starts_with("--games="));

    if cli.fuzz {
        run_fuzz_mode(&cli);
    } else if cli.matrix {
        run_matrix_mode(&cli);
    } else if cli.java_jar.is_some() {
        // In Java parity mode, always use multi-game report output (including games table),
        // even for the default single game.
        run_multi_game_mode(&cli);
    } else if games_flag_present || cli.games > 1 {
        run_multi_game_mode(&cli);
    } else {
        run_rust_only_mode(&cli);
    }
}

fn run_multi_game_mode(cli: &Cli) {
    if cli.verbose {
        eprintln!(
            "[parity] Running: {} vs {} | games={} | seed_start={} | max_turns={}",
            cli.deck1, cli.deck2, cli.games, cli.seed, cli.max_turns
        );
    }

    let seeds = game_seeds(cli.seed, cli.games);
    let data = match runner::load_data(cli.cards_dir.as_deref(), cli.verbose) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[parity] Load error: {}", e);
            std::process::exit(1);
        }
    };

    // Reuse one Java server across all games when possible.
    let mut server = if let Some(ref jar_path) = cli.java_jar {
        let server_config = JavaServerConfig {
            jar_path: jar_path.clone(),
            forge_home: None,
            verbose: cli.verbose,
        };
        match JavaServer::spawn(&server_config) {
            Ok(s) => Some(s),
            Err(e) => {
                eprintln!("[parity] Failed to spawn Java server: {}", e);
                eprintln!("[parity] Falling back to one-shot mode");
                None
            }
        }
    } else {
        None
    };

    let total = seeds.len();
    let mut results: Vec<MatchupResult> = Vec::with_capacity(total);
    for (i, seed) in seeds.iter().copied().enumerate() {
        let config = RunConfig {
            deck1: cli.deck1.clone(),
            deck2: cli.deck2.clone(),
            seed,
            max_turns: cli.max_turns,
            cards_dir: cli.cards_dir.clone(),
            verbose: cli.verbose,
            prefer_actions: cli.prefer_actions,
        };

        let result = if let Some(ref mut srv) = server {
            if srv.is_alive() {
                run_single_matchup_server(&config, &data, srv)
            } else {
                if let Some(ref jar_path) = cli.java_jar {
                    run_single_matchup_oneshot(&config, &data, jar_path)
                } else {
                    run_single_matchup_rust_only(&config, &data)
                }
            }
        } else if let Some(ref jar_path) = cli.java_jar {
            run_single_matchup_oneshot(&config, &data, jar_path)
        } else {
            run_single_matchup_rust_only(&config, &data)
        };

        if cli.verbose {
            let n = i + 1;
            match result.status {
                MatchupStatus::Pass => {
                    eprintln!(
                        "[parity] [{}/{}] seed={} ... PASS ({} snapshots)",
                        n, total, seed, result.snapshots_compared
                    );
                }
                MatchupStatus::Fail => {
                    eprintln!(
                        "[parity] [{}/{}] seed={} ... FAIL ({} divergences)",
                        n, total, seed, result.divergence_count
                    );
                }
                MatchupStatus::Error => {
                    eprintln!(
                        "[parity] [{}/{}] seed={} ... ERROR: {}",
                        n,
                        total,
                        seed,
                        result.error_message.as_deref().unwrap_or("unknown")
                    );
                }
            }
        }
        results.push(result);
    }

    if let Some(srv) = server {
        srv.shutdown();
    }

    let passed = results.iter().filter(|r| r.status == MatchupStatus::Pass).count();
    let failed = results.iter().filter(|r| r.status == MatchupStatus::Fail).count();
    let errors = results.iter().filter(|r| r.status == MatchupStatus::Error).count();

    let report_data = MatrixReport {
        total_matchups: total,
        passed,
        failed,
        errors,
        seeds,
        decks: vec![cli.deck1.clone(), cli.deck2.clone()],
        max_turns: cli.max_turns,
        results,
    };

    let output = match cli.format.as_str() {
        "json" => report::format_matrix_json(&report_data),
        _ => {
            let mut out = String::new();
            out.push_str(&build_coverage_report(&cli.deck1, &cli.deck2, &report_data.results));
            out.push_str(&build_mechanics_report(
                &cli.deck1,
                &cli.deck2,
                &report_data.results,
                Some(&data.db),
            ));
            out.push_str(&report::format_matrix_text(&report_data));
            out
        }
    };
    write_output(cli, &output);

    if failed > 0 || errors > 0 {
        std::process::exit(1);
    }
}

fn game_seeds(start_seed: u64, games: usize) -> Vec<u64> {
    (0..games)
        .map(|i| {
            start_seed.checked_add(i as u64).unwrap_or_else(|| {
                eprintln!(
                    "[parity] Seed overflow: start_seed={} games={}",
                    start_seed, games
                );
                std::process::exit(1);
            })
        })
        .collect()
}

fn run_rust_only_mode(cli: &Cli) {
    let config = RunConfig {
        deck1: cli.deck1.clone(),
        deck2: cli.deck2.clone(),
        seed: cli.seed,
        max_turns: cli.max_turns,
        cards_dir: cli.cards_dir.clone(),
        verbose: cli.verbose,
        prefer_actions: cli.prefer_actions,
    };

    let data = match runner::load_data(config.cards_dir.as_deref(), cli.verbose) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[parity] Load error: {}", e);
            std::process::exit(1);
        }
    };

    match runner::run_with_data(&config, &data) {
        Ok(trace) => {
            let mut output = match cli.format.as_str() {
                "json" => serde_json::to_string_pretty(&trace)
                    .unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
                _ => report::format_trace_text(&trace),
            };
            if cli.format != "json" {
                output.push_str(&build_coverage_report_from_cards(
                    &collect_unique_deck_cards(&cli.deck1, &cli.deck2),
                    &trace.covered_cards,
                ));
                let defs = collect_defined_mechanics(&cli.deck1, &cli.deck2, &data.db);
                output.push_str(&build_mechanics_report_from_signals(
                    &trace.mechanic_signals,
                    Some(&defs),
                ));
            }

            if cli.verbose {
                eprintln!(
                    "[parity] Done: {} snapshots collected",
                    trace.snapshots.len()
                );
            }
            write_output(cli, &output);
        }
        Err(e) => {
            eprintln!("[parity] Error: {}", e);
            std::process::exit(1);
        }
    }
}

#[allow(dead_code)]
fn run_parity_mode(cli: &Cli, jar_path: &PathBuf) {
    if cli.verbose {
        eprintln!("[parity] Full parity mode — running both engines");
    }

    let config = RunConfig {
        deck1: cli.deck1.clone(),
        deck2: cli.deck2.clone(),
        seed: cli.seed,
        max_turns: cli.max_turns,
        cards_dir: cli.cards_dir.clone(),
        verbose: cli.verbose,
        prefer_actions: cli.prefer_actions,
    };

    let data = match runner::load_data(config.cards_dir.as_deref(), cli.verbose) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[parity] Load error: {}", e);
            std::process::exit(1);
        }
    };

    // Spawn a Java server for the single matchup
    let server_config = JavaServerConfig {
        jar_path: jar_path.clone(),
        forge_home: None,
        verbose: cli.verbose,
    };

    let result = match JavaServer::spawn(&server_config) {
        Ok(mut server) => {
            let result = run_single_matchup_server(&config, &data, &mut server);
            server.shutdown();
            result
        }
        Err(e) => {
            eprintln!(
                "[parity] Failed to spawn Java server, falling back to one-shot mode: {}",
                e
            );
            run_single_matchup_oneshot(&config, &data, jar_path)
        }
    };

    let parity_report = report::build_report(
        // Build a minimal trace for the report
        &forge_parity::protocol::GameTrace {
            seed: config.seed,
            deck1: config.deck1.clone(),
            deck2: config.deck2.clone(),
            max_turns: config.max_turns,
            snapshots: vec![], // not used by build_report beyond len
            covered_cards: vec![],
            mechanic_signals: vec![],
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

    let mut output = match cli.format.as_str() {
        "json" => report::format_json(&parity_report),
        _ => report::format_text(&parity_report),
    };
    if cli.format != "json" {
        output.push_str(&build_coverage_report(
            &cli.deck1,
            &cli.deck2,
            std::slice::from_ref(&result),
        ));
        output.push_str(&build_mechanics_report(
            &cli.deck1,
            &cli.deck2,
            std::slice::from_ref(&result),
            Some(&data.db),
        ));
    }

    let mut exit_code = 0;
    if result.status == MatchupStatus::Pass {
        if cli.verbose {
            eprintln!(
                "[parity] PASS — engines agree on all {} snapshots",
                result.snapshots_compared
            );
        }
    } else {
        eprintln!(
            "[parity] FAIL — {} divergence(s) found across {} snapshots",
            result.divergence_count, result.snapshots_compared
        );
        exit_code = 1;
    }

    write_output(cli, &output);

    if exit_code != 0 {
        std::process::exit(exit_code);
    }
}

/// Run a single matchup using a JavaServer (server mode).
fn run_single_matchup_server(
    config: &RunConfig,
    data: &LoadedData,
    server: &mut JavaServer,
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
                trace: None,
                java_trace: None,
                covered_cards: vec![],
                mechanic_signals: vec![],
                finished_turn: None,
            };
        }
    };

    // Run Java engine via server
    let java_snapshots =
        match server.run_matchup(
            &config.deck1,
            &config.deck2,
            config.seed,
            config.max_turns,
            config.prefer_actions,
        ) {
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
                    error_message: Some(format!("Java server error: {}", e)),
                    trace: None,
                java_trace: None,
                    covered_cards: vec![],
                    mechanic_signals: vec![],
                    finished_turn: None,
                };
            }
        };
    let mut result = compare_snapshots(config, &rust_trace.snapshots, &java_snapshots);
    result.covered_cards = rust_trace.covered_cards;
    result.mechanic_signals = rust_trace.mechanic_signals;
    result
}

/// Run a single matchup using one-shot JavaBridge (fallback mode).
fn run_single_matchup_oneshot(
    config: &RunConfig,
    data: &LoadedData,
    jar_path: &PathBuf,
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
                trace: None,
                java_trace: None,
                covered_cards: vec![],
                mechanic_signals: vec![],
                finished_turn: None,
            };
        }
    };

    // Run Java engine via one-shot bridge
    let bridge_config = JavaBridgeConfig {
        jar_path: jar_path.clone(),
        seed: config.seed,
        max_turns: config.max_turns,
        deck1: config.deck1.clone(),
        deck2: config.deck2.clone(),
        forge_home: None,
        verbose: config.verbose,
        prefer_actions: config.prefer_actions,
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
                trace: None,
                java_trace: None,
                covered_cards: vec![],
                mechanic_signals: vec![],
                finished_turn: None,
            };
        }
    };
    let mut result = compare_snapshots(config, &rust_trace.snapshots, &java_snapshots);
    result.covered_cards = rust_trace.covered_cards;
    result.mechanic_signals = rust_trace.mechanic_signals;
    result
}

/// Run a single matchup: Rust only (no Java). Used when no JAR is provided.
fn run_single_matchup_rust_only(config: &RunConfig, data: &LoadedData) -> MatchupResult {
    match runner::run_with_data(config, data) {
        Ok(trace) => MatchupResult {
            deck1: config.deck1.clone(),
            deck2: config.deck2.clone(),
            seed: config.seed,
            status: MatchupStatus::Pass,
            snapshots_compared: trace.snapshots.len(),
            divergence_count: 0,
            first_divergence: None,
            error_message: None,
            trace: None,
                java_trace: None,
            covered_cards: trace.covered_cards,
            mechanic_signals: trace.mechanic_signals,
            finished_turn: trace
                .snapshots
                .last()
                .and_then(|s| if s.game_over { Some(s.turn) } else { None }),
        },
        Err(e) => MatchupResult {
            deck1: config.deck1.clone(),
            deck2: config.deck2.clone(),
            seed: config.seed,
            status: MatchupStatus::Error,
            snapshots_compared: 0,
            divergence_count: 0,
            first_divergence: None,
            error_message: Some(format!("Rust engine error: {}", e)),
            trace: None,
                java_trace: None,
            covered_cards: vec![],
            mechanic_signals: vec![],
            finished_turn: None,
        },
    }
}

/// Compare Rust and Java snapshot lists and build a MatchupResult.
fn compare_snapshots(
    config: &RunConfig,
    rust_snapshots: &[forge_parity::protocol::StateSnapshot],
    java_snapshots: &[forge_parity::protocol::StateSnapshot],
) -> MatchupResult {
    let max_snapshots = rust_snapshots.len().max(java_snapshots.len());
    let mut first_divergence: Option<Divergence> = None;
    let mut compared_until = max_snapshots;

    for i in 0..max_snapshots {
        match (rust_snapshots.get(i), java_snapshots.get(i)) {
            (Some(rs), Some(js)) => {
                let divs = comparator::compare(i, rs, js);
                if let Some(div) = divs.into_iter().next() {
                    first_divergence = Some(div);
                    compared_until = i + 1;
                    break;
                }
            }
            (Some(_rs), None) => {
                first_divergence = Some(Divergence {
                    snapshot_index: i,
                    turn: _rs.turn,
                    phase: _rs.phase.clone(),
                    field: "snapshot.exists".into(),
                    rust_value: "present".into(),
                    java_value: "missing".into(),
                });
                compared_until = i + 1;
                break;
            }
            (None, Some(_js)) => {
                first_divergence = Some(Divergence {
                    snapshot_index: i,
                    turn: _js.turn,
                    phase: _js.phase.clone(),
                    field: "snapshot.exists".into(),
                    rust_value: "missing".into(),
                    java_value: "present".into(),
                });
                compared_until = i + 1;
                break;
            }
            (None, None) => {}
        }
    }

    let divergence_count = usize::from(first_divergence.is_some());
    let status = if first_divergence.is_none() {
        MatchupStatus::Pass
    } else {
        MatchupStatus::Fail
    };

    MatchupResult {
        deck1: config.deck1.clone(),
        deck2: config.deck2.clone(),
        seed: config.seed,
        status,
        snapshots_compared: compared_until,
        divergence_count,
        trace: first_divergence
            .as_ref()
            .map(|_| format_rust_trace(config, &rust_snapshots[..compared_until.min(rust_snapshots.len())])),
        java_trace: first_divergence
            .as_ref()
            .map(|_| format_java_trace(config, &java_snapshots[..compared_until.min(java_snapshots.len())])),
        first_divergence,
        error_message: None,
        covered_cards: vec![],
        mechanic_signals: vec![],
        finished_turn: rust_snapshots
            .last()
            .and_then(|s| if s.game_over { Some(s.turn) } else { None }),
    }
}

fn format_rust_trace(
    config: &RunConfig,
    rust_snapshots: &[forge_parity::protocol::StateSnapshot],
) -> String {
    report::format_trace_text(&forge_parity::protocol::GameTrace {
        seed: config.seed,
        deck1: config.deck1.clone(),
        deck2: config.deck2.clone(),
        max_turns: config.max_turns,
        snapshots: rust_snapshots.to_vec(),
        covered_cards: vec![],
        mechanic_signals: vec![],
    })
}

fn format_java_trace(
    config: &RunConfig,
    java_snapshots: &[forge_parity::protocol::StateSnapshot],
) -> String {
    report::format_trace_text(&forge_parity::protocol::GameTrace {
        seed: config.seed,
        deck1: config.deck1.clone(),
        deck2: config.deck2.clone(),
        max_turns: config.max_turns,
        snapshots: java_snapshots.to_vec(),
        covered_cards: vec![],
        mechanic_signals: vec![],
    })
    .replace("(Rust-only)", "(Java-only)")
}

/// A pool of JavaServer instances behind mutexes for parallel access.
struct ServerPool {
    servers: Vec<Mutex<JavaServer>>,
}

impl ServerPool {
    /// Spawn N server instances.
    fn spawn(n: usize, config: &JavaServerConfig) -> Result<Self, JavaBridgeError> {
        let mut servers = Vec::with_capacity(n);
        for i in 0..n {
            if config.verbose {
                eprintln!("[parity] Spawning Java worker {}/{}", i + 1, n);
            }
            let server = JavaServer::spawn(config)?;
            servers.push(Mutex::new(server));
        }
        Ok(Self { servers })
    }

    /// Run a matchup on any available server. Tries each server in round-robin.
    /// If a server has crashed, marks it as dead and tries the next.
    fn run_matchup(
        &self,
        deck1: &str,
        deck2: &str,
        seed: u64,
        max_turns: u32,
        prefer_actions: bool,
    ) -> Result<Vec<forge_parity::protocol::StateSnapshot>, JavaBridgeError> {
        // Try each server — grab whichever lock is available
        for server_mutex in &self.servers {
            if let Ok(mut server) = server_mutex.try_lock() {
                if !server.is_alive() {
                    continue;
                }
                return server.run_matchup(deck1, deck2, seed, max_turns, prefer_actions);
            }
        }
        // All busy on try_lock — block on the first one
        let mut server = self.servers[0]
            .lock()
            .map_err(|e| JavaBridgeError::ProtocolError(format!("Mutex poisoned: {}", e)))?;
        server.run_matchup(deck1, deck2, seed, max_turns, prefer_actions)
    }

    /// Shutdown all servers.
    fn shutdown(self) {
        for server_mutex in self.servers {
            if let Ok(server) = server_mutex.into_inner() {
                server.shutdown();
            }
        }
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
    if cli.verbose {
        eprintln!(
            "[parity] Matrix mode: {} decks × {} seeds = {} matchups",
            deck_names.len(),
            seeds.len(),
            total
        );
    }

    // Load data once
    let data = match runner::load_data(cli.cards_dir.as_deref(), cli.verbose) {
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

    // Spawn server pool if Java JAR is provided
    let num_workers = cli
        .java_workers
        .unwrap_or_else(|| if cli.java_jar.is_some() { num_cpus() } else { 0 });

    let pool = if let Some(ref jar_path) = cli.java_jar {
        let server_config = JavaServerConfig {
            jar_path: jar_path.clone(),
            forge_home: None,
            verbose: cli.verbose,
        };
        match ServerPool::spawn(num_workers.max(1), &server_config) {
            Ok(pool) => Some(pool),
            Err(e) => {
                eprintln!("[parity] Failed to spawn Java server pool: {}", e);
                eprintln!("[parity] Falling back to one-shot mode");
                None
            }
        }
    } else {
        None
    };

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
                prefer_actions: cli.prefer_actions,
            };

            let result = if let Some(ref pool) = pool {
                run_single_matchup_with_pool(&config, &data, pool)
            } else if let Some(ref jar_path) = cli.java_jar {
                run_single_matchup_oneshot(&config, &data, jar_path)
            } else {
                run_single_matchup_rust_only(&config, &data)
            };

            if cli.verbose {
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
            }

            result
        })
        .collect();

    // Shutdown pool
    if let Some(pool) = pool {
        pool.shutdown();
    }

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

/// Run a single matchup using the server pool.
fn run_single_matchup_with_pool(
    config: &RunConfig,
    data: &LoadedData,
    pool: &ServerPool,
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
                trace: None,
                java_trace: None,
                covered_cards: vec![],
                mechanic_signals: vec![],
                finished_turn: None,
            };
        }
    };

    // Run Java via pool
    let java_snapshots = match pool.run_matchup(
            &config.deck1,
            &config.deck2,
            config.seed,
            config.max_turns,
            config.prefer_actions,
        ) {
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
                    error_message: Some(format!("Java server error: {}", e)),
                    trace: None,
                java_trace: None,
                    covered_cards: vec![],
                    mechanic_signals: vec![],
                    finished_turn: None,
                };
            }
        };
    let mut result = compare_snapshots(config, &rust_trace.snapshots, &java_snapshots);
    result.covered_cards = rust_trace.covered_cards;
    result.mechanic_signals = rust_trace.mechanic_signals;
    result
}

fn run_fuzz_mode(cli: &Cli) {
    if cli.verbose {
        eprintln!(
            "[parity] Fuzz mode: {} iterations, master_seed={}",
            cli.iterations, cli.master_seed
        );
    }

    // Load data once
    let data = match runner::load_data(cli.cards_dir.as_deref(), cli.verbose) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[parity] Load error: {}", e);
            std::process::exit(1);
        }
    };

    // Discover card pool
    let (pool, pool_stats) = CardPool::discover(&data.db);
    if cli.verbose {
        eprintln!("[parity] {}", pool_stats);
    }

    if pool.cards.iter().filter(|c| !c.is_land).count() == 0 {
        eprintln!("[parity] No spells in pool — nothing to test");
        std::process::exit(1);
    }

    let total_cards = pool_stats.total_scanned;
    let pool_size = pool_stats.included;

    // Spawn a single Java server for all iterations (if Java JAR provided)
    let mut server = if let Some(ref jar_path) = cli.java_jar {
        let server_config = JavaServerConfig {
            jar_path: jar_path.clone(),
            forge_home: None,
            verbose: cli.verbose,
        };
        match JavaServer::spawn(&server_config) {
            Ok(s) => Some(s),
            Err(e) => {
                eprintln!("[parity] Failed to spawn Java server: {}", e);
                eprintln!("[parity] Falling back to one-shot mode");
                None
            }
        }
    } else {
        None
    };

    // Derive per-iteration seeds from master seed
    let mut master_rng = JavaRandom::new(cli.master_seed as i64);

    let mut results: Vec<FuzzResult> = Vec::new();
    let total = cli.iterations;

    for iteration in 0..total {
        let deck1_seed = master_rng.next_int(i32::MAX) as u64;
        let deck2_seed = master_rng.next_int(i32::MAX) as u64;
        let game_seed = master_rng.next_int(i32::MAX) as u64;

        // Generate random decks
        let mut deck1_rng = JavaRandom::new(deck1_seed as i64);
        let mut deck2_rng = JavaRandom::new(deck2_seed as i64);
        let deck1_spec = deck_generator::generate_deck(&mut deck1_rng, &pool);
        let deck2_spec = deck_generator::generate_deck(&mut deck2_rng, &pool);

        let deck1_inline = deck_generator::format_inline(&deck1_spec);
        let deck2_inline = deck_generator::format_inline(&deck2_spec);

        let config = RunConfig {
            deck1: format!("inline:{}", deck1_inline),
            deck2: format!("inline:{}", deck2_inline),
            seed: game_seed,
            max_turns: cli.max_turns,
            cards_dir: cli.cards_dir.clone(),
            verbose: cli.verbose,
            prefer_actions: cli.prefer_actions,
        };

        let matchup_result = if let Some(ref mut srv) = server {
            if srv.is_alive() {
                run_single_matchup_server(&config, &data, srv)
            } else {
                // Server crashed — try to respawn
                if cli.verbose {
                    eprintln!("[parity] Java server crashed, attempting respawn...");
                }
                match JavaServer::spawn(&JavaServerConfig {
                    jar_path: cli.java_jar.as_ref().unwrap().clone(),
                    forge_home: None,
                    verbose: cli.verbose,
                }) {
                    Ok(new_srv) => {
                        *srv = new_srv;
                        run_single_matchup_server(&config, &data, srv)
                    }
                    Err(e) => {
                        eprintln!("[parity] Failed to respawn Java server: {}", e);
                        // Fall back to one-shot for this iteration
                        if let Some(ref jar_path) = cli.java_jar {
                            run_single_matchup_oneshot(&config, &data, jar_path)
                        } else {
                            run_single_matchup_rust_only(&config, &data)
                        }
                    }
                }
            }
        } else if let Some(ref jar_path) = cli.java_jar {
            run_single_matchup_oneshot(&config, &data, jar_path)
        } else {
            run_single_matchup_rust_only(&config, &data)
        };

        if cli.verbose {
            let n = iteration + 1;
            match matchup_result.status {
                MatchupStatus::Pass => {
                    eprintln!(
                        "[parity] [{}/{}] iteration={} seed={} ... PASS ({} snapshots)",
                        n, total, iteration, game_seed, matchup_result.snapshots_compared
                    );
                }
                MatchupStatus::Fail => {
                    eprintln!(
                        "[parity] [{}/{}] iteration={} seed={} ... FAIL ({} divergences)",
                        n, total, iteration, game_seed, matchup_result.divergence_count
                    );
                }
                MatchupStatus::Error => {
                    eprintln!(
                        "[parity] [{}/{}] iteration={} seed={} ... ERROR: {}",
                        n,
                        total,
                        iteration,
                        game_seed,
                        matchup_result.error_message.as_deref().unwrap_or("unknown")
                    );
                }
            }
        }

        results.push(FuzzResult {
            iteration,
            game_seed,
            deck1_spec: deck1_inline,
            deck2_spec: deck2_inline,
            result: matchup_result,
        });
    }

    // Shutdown server
    if let Some(srv) = server {
        srv.shutdown();
    }

    let passed = results
        .iter()
        .filter(|r| r.result.status == MatchupStatus::Pass)
        .count();
    let failed = results
        .iter()
        .filter(|r| r.result.status == MatchupStatus::Fail)
        .count();
    let errors = results
        .iter()
        .filter(|r| r.result.status == MatchupStatus::Error)
        .count();

    let fuzz_report = FuzzReport {
        master_seed: cli.master_seed,
        iterations: total,
        max_turns: cli.max_turns,
        pool_size,
        total_cards,
        passed,
        failed,
        errors,
        results,
    };

    let output = match cli.format.as_str() {
        "json" => report::format_fuzz_json(&fuzz_report),
        _ => report::format_fuzz_text(&fuzz_report),
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

fn collect_unique_deck_cards(deck1: &str, deck2: &str) -> Vec<String> {
    let mut cards: BTreeSet<String> = BTreeSet::new();
    for deck in [deck1, deck2] {
        match runner::resolve_deck_spec(deck) {
            Ok(spec) => {
                for (name, _) in spec {
                    cards.insert(name);
                }
            }
            Err(e) => {
                eprintln!("[parity] Coverage warning: failed to parse deck '{}': {}", deck, e);
            }
        }
    }
    cards.into_iter().collect()
}

fn build_coverage_report(deck1: &str, deck2: &str, results: &[MatchupResult]) -> String {
    let deck_cards = collect_unique_deck_cards(deck1, deck2);
    let mut covered: BTreeSet<String> = BTreeSet::new();
    for r in results {
        for c in &r.covered_cards {
            covered.insert(c.clone());
        }
    }
    let covered_cards: Vec<String> = covered.into_iter().collect();
    build_coverage_report_from_cards(&deck_cards, &covered_cards)
}

#[derive(Debug, Default, Clone)]
struct DefinedMechanics {
    trigger_keys: BTreeSet<String>,
    effect_keys: BTreeSet<String>,
    ability_keys: BTreeSet<String>,
}

fn build_mechanics_report(
    deck1: &str,
    deck2: &str,
    results: &[MatchupResult],
    db: Option<&CardDatabase>,
) -> String {
    let mut aggregated: BTreeMap<String, usize> = BTreeMap::new();
    for r in results {
        for sig in &r.mechanic_signals {
            *aggregated.entry(sig.label.clone()).or_insert(0) += sig.count;
        }
    }
    let signals: Vec<MechanicSignal> = aggregated
        .into_iter()
        .map(|(label, count)| MechanicSignal { label, count })
        .collect();
    let defs = db.map(|card_db| collect_defined_mechanics(deck1, deck2, card_db));
    build_mechanics_report_from_signals(&signals, defs.as_ref())
}

fn build_mechanics_report_from_signals(
    signals: &[MechanicSignal],
    defs: Option<&DefinedMechanics>,
) -> String {
    let mut triggers: BTreeSet<String> = BTreeSet::new();
    let mut effects: BTreeSet<String> = BTreeSet::new();
    let mut abilities: BTreeSet<String> = BTreeSet::new();
    let mut other: BTreeSet<String> = BTreeSet::new();

    for sig in signals {
        let label = sig.label.clone();
        if label.starts_with("Trigger fired:") {
            triggers.insert(label);
        } else if label.starts_with("Effect resolved:") {
            effects.insert(label);
        } else if label.starts_with("Activated ability:")
            || label.starts_with("Suspend:")
            || label.starts_with("Foretold:")
            || label.starts_with("Rebound:")
            || label.starts_with("Cascade")
            || label.starts_with("Storm")
            || label.starts_with("Replicate")
        {
            abilities.insert(label);
        } else {
            other.insert(label);
        }
    }

    let mut out = String::new();
    out.push_str("\n=== Ability/Effect/Trigger Signals (Low-Effort) ===\n\n");
    if triggers.is_empty() && effects.is_empty() && abilities.is_empty() && other.is_empty() {
        out.push_str("No mechanic signals observed.\n");
        return out;
    }

    if let Some(d) = defs {
        out.push_str(&coverage_line(
            "Trigger coverage",
            &d.trigger_keys,
            &triggers,
        ));
    }
    out.push_str(&format!("Unique triggers: {}\n", triggers.len()));
    for label in &triggers {
        out.push_str(&format!("  - {}\n", label));
    }

    if let Some(d) = defs {
        out.push_str(&format!(
            "\n{}\n",
            coverage_line("Effect coverage", &d.effect_keys, &effects).trim_end()
        ));
    }
    out.push_str(&format!("\nUnique effects: {}\n", effects.len()));
    for label in &effects {
        out.push_str(&format!("  - {}\n", label));
    }

    if let Some(d) = defs {
        out.push_str(&format!(
            "\n{}\n",
            coverage_line("Ability coverage", &d.ability_keys, &abilities).trim_end()
        ));
    }
    out.push_str(&format!("\nUnique abilities: {}\n", abilities.len()));
    for label in &abilities {
        out.push_str(&format!("  - {}\n", label));
    }

    if !other.is_empty() {
        out.push_str(&format!("\nOther signals: {}\n", other.len()));
        for label in &other {
            out.push_str(&format!("  - {}\n", label));
        }
    }
    out
}

fn coverage_line(label: &str, defined: &BTreeSet<String>, observed: &BTreeSet<String>) -> String {
    let total = defined.len();
    let matched = observed.iter().filter(|k| defined.contains(*k)).count();
    let pct = if total == 0 {
        0.0
    } else {
        (matched as f64 / total as f64) * 100.0
    };
    format!("{}: {}/{} ({:.1}%)\n", label, matched, total, pct)
}

fn collect_defined_mechanics(deck1: &str, deck2: &str, db: &CardDatabase) -> DefinedMechanics {
    let mut card_names: BTreeSet<String> = BTreeSet::new();
    for deck in [deck1, deck2] {
        match runner::resolve_deck_spec(deck) {
            Ok(spec) => {
                for (name, _) in spec {
                    card_names.insert(name);
                }
            }
            Err(e) => {
                eprintln!(
                    "[parity] Mechanics warning: failed to parse deck '{}': {}",
                    deck, e
                );
            }
        }
    }

    let mut defs = DefinedMechanics::default();
    for name in card_names {
        let Some(rules) = db.get_by_card_name(&name) else {
            continue;
        };
        collect_face_mechanics(&rules.main_part, &mut defs);
        if let Some(other) = &rules.other_part {
            collect_face_mechanics(other, &mut defs);
        }
    }
    defs
}

fn collect_face_mechanics(face: &forge_carddb::CardFace, defs: &mut DefinedMechanics) {
    for raw in &face.abilities {
        if let Some(ab) = extract_param(raw, "AB") {
            defs.ability_keys.insert(format!("Activated ability: {}", ab));
        }
        if let Some(sp) = extract_param(raw, "SP") {
            defs.effect_keys.insert(format!("Effect resolved: {}", sp));
        }
        if let Some(db) = extract_param(raw, "DB") {
            defs.effect_keys.insert(format!("Effect resolved: {}", db));
        }
    }

    for raw in face.svars.values() {
        if let Some(ab) = extract_param(raw, "AB") {
            defs.ability_keys.insert(format!("Activated ability: {}", ab));
        }
        if let Some(sp) = extract_param(raw, "SP") {
            defs.effect_keys.insert(format!("Effect resolved: {}", sp));
        }
        if let Some(db) = extract_param(raw, "DB") {
            defs.effect_keys.insert(format!("Effect resolved: {}", db));
        }
    }

    for raw in &face.triggers {
        let mode = extract_param(raw, "Mode").unwrap_or_else(|| "Unknown".to_string());
        let api = extract_param(raw, "Execute")
            .and_then(|exec| face.svars.get(&exec).cloned())
            .as_deref()
            .and_then(extract_ability_api)
            .unwrap_or_else(|| "Unknown".to_string());
        defs.trigger_keys
            .insert(format!("Trigger fired: mode={} | api={}", mode, api));
    }
}

fn extract_ability_api(raw: &str) -> Option<String> {
    extract_param(raw, "SP")
        .or_else(|| extract_param(raw, "DB"))
        .or_else(|| extract_param(raw, "AB"))
}

fn extract_param(raw: &str, key: &str) -> Option<String> {
    raw.split('|').find_map(|part| {
        let trimmed = part.trim();
        let (lhs, rhs) = trimmed.split_once('$')?;
        if lhs.trim().eq_ignore_ascii_case(key) {
            Some(rhs.trim().to_string())
        } else {
            None
        }
    })
}

fn build_coverage_report_from_cards(deck_cards: &[String], covered_cards: &[String]) -> String {
    let deck_set: BTreeSet<&str> = deck_cards.iter().map(|s| s.as_str()).collect();
    let covered_set: BTreeSet<&str> = covered_cards
        .iter()
        .map(|s| s.as_str())
        .filter(|name| deck_set.contains(name))
        .collect();

    let total = deck_set.len();
    let covered = covered_set.len();
    let pct = if total == 0 {
        0.0
    } else {
        (covered as f64 / total as f64) * 100.0
    };

    let uncovered: Vec<&str> = deck_set
        .iter()
        .copied()
        .filter(|name| !covered_set.contains(name))
        .collect();

    let mut out = String::new();
    out.push_str("\n=== Coverage Report ===\n\n");
    out.push_str(&format!(
        "Covered cards: {}/{} ({:.1}%)\n",
        covered, total, pct
    ));
    if uncovered.is_empty() {
        out.push_str("Uncovered cards: none\n");
    } else {
        out.push_str("Uncovered cards:\n");
        for name in uncovered {
            out.push_str(&format!("  - {}\n", name));
        }
    }
    out
}

/// Get the number of available CPUs (capped at a reasonable number).
fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .min(8)
}
