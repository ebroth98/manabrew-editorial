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
//!
//! # Performance: Three-tier optimization
//!
//! 1. **Parallel engines** (single-matchup modes): Rust and Java run concurrently
//!    via `std::thread::scope`. Matchup time = `max(rust, java)` not `rust + java`.
//!
//! 2. **Streaming diff** (matrix mode): Java snapshots are compared against
//!    pre-computed Rust results as they arrive via `run_matchup_streaming`. On
//!    divergence, remaining Java snapshots are skipped (not parsed), saving JSON
//!    deserialization time on long games.
//!
//! 3. **Rust-ahead batching** (matrix mode): All Rust games run first in a
//!    parallel burst (phase 1), then Java servers process results with streaming
//!    comparison (phase 2). Java servers are never idle waiting for Rust.

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
use forge_parity::java_bridge::{
    JavaBridge, JavaBridgeConfig, JavaBridgeError, JavaMatchupData, JavaServer, JavaServerConfig,
};
use forge_parity::java_cache::{self, JavaCache};
use forge_parity::java_random::JavaRandom;
use forge_parity::protocol::{
    DecisionRecord, Divergence, FuzzReport, FuzzResult, MatchupResult, MatchupStatus, MatrixReport,
    MechanicSignal,
};
use forge_parity::report;
use forge_parity::runner::{self, available_presets, LoadedData, RunConfig, DEFAULT_DECKS_DIR};

/// Filter out decks matching any of the given prefixes.
fn filter_decks(decks: Vec<String>, exclude_prefixes: &[String]) -> Vec<String> {
    if exclude_prefixes.is_empty() {
        return decks;
    }
    decks
        .into_iter()
        .filter(|d| !exclude_prefixes.iter().any(|p| d.starts_with(p)))
        .collect()
}

/// Truncate a trace string to at most `max_lines` lines to limit memory usage.
#[allow(dead_code)]
fn truncate_trace(trace: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = trace.lines().collect();
    if lines.len() <= max_lines {
        return trace.to_string();
    }
    let half = max_lines / 2;
    let mut result = lines[..half].join("\n");
    result.push_str(&format!(
        "\n... ({} lines omitted) ...\n",
        lines.len() - max_lines
    ));
    result.push_str(&lines[lines.len() - half..].join("\n"));
    result
}

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

    /// Path to the preset deck JSON files directory
    #[arg(long)]
    decks_dir: Option<String>,

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

    /// Maximum JVM heap size per Java worker (e.g. "512m", "1g"). Default: "512m".
    /// On memory-constrained VMs, this prevents each JVM from consuming all available RAM.
    #[arg(long, default_value = "512m")]
    java_heap: String,

    /// Run continuous parity testing: execute games, store in SQLite, exit with threshold check
    #[arg(long)]
    continuous: bool,

    /// Start continuous parity server with web dashboard
    #[arg(long)]
    serve: bool,

    /// CI regression mode: process only queued jobs from API, exit when batch completes
    #[arg(long)]
    ci: bool,

    /// Maximum number of games for continuous mode (default: unlimited for serve, 100 for continuous)
    #[arg(long)]
    max_games: Option<usize>,

    /// Pass rate threshold (0.0-1.0); exit 1 if below (continuous mode only, default: 0.90)
    #[arg(long, default_value_t = 0.90)]
    threshold: f64,

    /// SQLite database path for continuous mode (default: parity.db)
    #[arg(long, default_value = "parity.db")]
    db_path: String,

    /// HTTP port for serve mode (default: 8080)
    #[arg(long, default_value_t = 8080)]
    port: u16,

    /// Number of fuzz games per batch in continuous mode (0 to disable)
    #[arg(long, default_value_t = 0)]
    fuzz_per_batch: usize,

    /// Enable the analysis daemon (clusters failures, LLM analysis, Discord/GitHub)
    #[arg(long)]
    analyze: bool,

    /// Seconds between analysis DB checks (default: 60)
    #[arg(long, default_value_t = 60)]
    poll_interval: u64,

    /// Seconds between Discord summary posts (default: 3600)
    #[arg(long, default_value_t = 3600)]
    summary_interval: u64,

    /// Minimum failures in a cluster before opening a GitHub issue (default: 5)
    #[arg(long, default_value_t = 5)]
    issue_threshold: i64,

    /// GitHub repo for issues in owner/repo format
    #[arg(long)]
    github_repo: Option<String>,

    /// Disable Java output cache (always run Java)
    #[arg(long)]
    no_cache: bool,

    /// Directory for the Java output cache (default: .parity-cache)
    #[arg(long, default_value = ".parity-cache")]
    cache_dir: String,

    /// Tracing log level for serve mode (default: warn). Accepts: error, warn, info, debug, trace.
    /// Can also be set via RUST_LOG env var which takes precedence.
    #[arg(long, default_value = "warn")]
    log_level: String,

    /// Comma-separated deck name prefixes to exclude from scheduling (default: "real_").
    /// Decks matching any prefix are skipped in matrix/continuous/serve modes.
    #[arg(long, value_delimiter = ',', default_value = "real_")]
    exclude_prefix: Vec<String>,
}

/// Resolve issue_threshold: CLI flag > env var > default (5)
#[allow(dead_code)]
fn resolve_issue_threshold(cli_val: i64) -> i64 {
    if cli_val != 5 {
        return cli_val;
    }
    std::env::var("ISSUE_THRESHOLD")
        .ok()
        .filter(|s| !s.is_empty())
        .and_then(|s| s.parse().ok())
        .unwrap_or(cli_val)
}

/// Resolve github_repo: CLI flag > env var
#[allow(dead_code)]
fn resolve_github_repo(cli_val: Option<String>) -> Option<String> {
    cli_val.or_else(|| std::env::var("GITHUB_REPO").ok().filter(|s| !s.is_empty()))
}

fn main() {
    let cli = Cli::parse();
    let games_flag_present =
        std::env::args().any(|arg| arg == "--games" || arg.starts_with("--games="));

    if cli.analyze && !cli.serve {
        #[cfg(feature = "analyze")]
        {
            run_analyze_only(&cli);
            return;
        }
        #[cfg(not(feature = "analyze"))]
        {
            eprintln!("[parity] --analyze requires the 'analyze' feature. Build with: cargo build --features analyze");
            std::process::exit(1);
        }
    }

    if cli.serve || cli.ci {
        #[cfg(feature = "serve")]
        {
            run_serve_mode(&cli);
        }
        #[cfg(not(feature = "serve"))]
        {
            eprintln!("[parity] --serve requires the 'serve' feature. Build with: cargo build --features serve");
            std::process::exit(1);
        }
    } else if cli.continuous {
        #[cfg(feature = "storage")]
        {
            run_continuous_mode(&cli);
        }
        #[cfg(not(feature = "storage"))]
        {
            eprintln!("[parity] --continuous requires the 'storage' feature. Build with: cargo build --features storage");
            std::process::exit(1);
        }
    } else if cli.fuzz {
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
            decks_dir: cli.decks_dir.clone(),
            verbose: cli.verbose,
            java_heap: cli.java_heap.clone(),
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
            decks_dir: cli.decks_dir.clone(),
            verbose: cli.verbose,
            prefer_actions: cli.prefer_actions,
            java_heap: cli.java_heap.clone(),
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

    let passed = results
        .iter()
        .filter(|r| r.status == MatchupStatus::Pass)
        .count();
    let failed = results
        .iter()
        .filter(|r| r.status == MatchupStatus::Fail)
        .count();
    let errors = results
        .iter()
        .filter(|r| r.status == MatchupStatus::Error)
        .count();

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
            let dd = cli.decks_dir.as_deref().unwrap_or(DEFAULT_DECKS_DIR);
            out.push_str(&build_coverage_report(
                &cli.deck1,
                &cli.deck2,
                &report_data.results,
                dd,
            ));
            out.push_str(&build_mechanics_report(
                &cli.deck1,
                &cli.deck2,
                &report_data.results,
                Some(&data.db),
                dd,
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
        decks_dir: cli.decks_dir.clone(),
        verbose: cli.verbose,
        prefer_actions: cli.prefer_actions,
        java_heap: cli.java_heap.clone(),
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
                let dd = cli.decks_dir.as_deref().unwrap_or(DEFAULT_DECKS_DIR);
                output.push_str(&build_coverage_report_from_cards(
                    &collect_unique_deck_cards(&cli.deck1, &cli.deck2, dd),
                    &trace.covered_cards,
                ));
                let defs = collect_defined_mechanics(&cli.deck1, &cli.deck2, &data.db, dd);
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
        decks_dir: cli.decks_dir.clone(),
        verbose: cli.verbose,
        prefer_actions: cli.prefer_actions,
        java_heap: cli.java_heap.clone(),
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
        decks_dir: cli.decks_dir.clone(),
        verbose: cli.verbose,
        java_heap: cli.java_heap.clone(),
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
            decisions: vec![],
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
        let dd = cli.decks_dir.as_deref().unwrap_or(DEFAULT_DECKS_DIR);
        output.push_str(&build_coverage_report(
            &cli.deck1,
            &cli.deck2,
            std::slice::from_ref(&result),
            dd,
        ));
        output.push_str(&build_mechanics_report(
            &cli.deck1,
            &cli.deck2,
            std::slice::from_ref(&result),
            Some(&data.db),
            dd,
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
/// Rust engine runs on a background thread while Java runs on the current thread,
/// so both engines execute in parallel.
fn run_single_matchup_server(
    config: &RunConfig,
    data: &LoadedData,
    server: &mut JavaServer,
) -> MatchupResult {
    // Run Rust and Java engines concurrently:
    // Rust on a scoped thread, Java on the current thread (needs &mut server).
    let (rust_result, java_result) = std::thread::scope(|s| {
        let rust_handle = s.spawn(|| runner::run_with_data(config, data));

        // Java runs on this thread since it needs exclusive &mut server
        let java_result = server.run_matchup(
            &config.deck1,
            &config.deck2,
            config.seed,
            config.max_turns,
            config.prefer_actions,
        );

        let rust_result = rust_handle.join().expect("Rust engine thread panicked");
        (rust_result, java_result)
    });

    let rust_trace = match rust_result {
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

    let java_data = match java_result {
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

    let mut result = compare_snapshots(
        config,
        &rust_trace.snapshots,
        &rust_trace.decisions,
        &java_data,
    );
    result.covered_cards = rust_trace.covered_cards;
    result.mechanic_signals = rust_trace.mechanic_signals;
    result
}

/// Run a single matchup using one-shot JavaBridge (fallback mode).
/// Rust and Java engines run in parallel using scoped threads.
fn run_single_matchup_oneshot(
    config: &RunConfig,
    data: &LoadedData,
    jar_path: &PathBuf,
) -> MatchupResult {
    // Run Rust and Java engines concurrently
    let (rust_result, java_result) = std::thread::scope(|s| {
        let rust_handle = s.spawn(|| runner::run_with_data(config, data));

        let java_handle = s.spawn(|| {
            let bridge_config = JavaBridgeConfig {
                jar_path: jar_path.clone(),
                seed: config.seed,
                max_turns: config.max_turns,
                deck1: config.deck1.clone(),
                deck2: config.deck2.clone(),
                forge_home: None,
                decks_dir: config.decks_dir.clone(),
                verbose: config.verbose,
                prefer_actions: config.prefer_actions,
                java_heap: config.java_heap.clone(),
            };
            let bridge = JavaBridge::new(bridge_config);
            bridge.run()
        });

        (
            rust_handle.join().expect("Rust engine thread panicked"),
            java_handle.join().expect("Java bridge thread panicked"),
        )
    });

    let rust_trace = match rust_result {
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

    let java_data = match java_result {
        Ok(data) => data,
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

    let mut result = compare_snapshots(
        config,
        &rust_trace.snapshots,
        &rust_trace.decisions,
        &java_data,
    );
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
            finished_turn: trace.snapshots.last().and_then(|s| {
                if s.game_over {
                    Some(s.turn)
                } else {
                    None
                }
            }),
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
    rust_decisions: &[DecisionRecord],
    java_data: &JavaMatchupData,
) -> MatchupResult {
    let java_snapshots = &java_data.snapshots;
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
        trace: first_divergence.as_ref().map(|_| {
            format_rust_trace(
                config,
                &rust_snapshots[..compared_until.min(rust_snapshots.len())],
                rust_decisions,
            )
        }),
        java_trace: first_divergence.as_ref().map(|_| {
            format_java_trace(
                config,
                &java_snapshots[..compared_until.min(java_snapshots.len())],
                &java_data.decisions,
            )
        }),
        first_divergence,
        error_message: None,
        covered_cards: vec![],
        mechanic_signals: vec![],
        finished_turn: rust_snapshots.last().and_then(|s| {
            if s.game_over {
                Some(s.turn)
            } else {
                None
            }
        }),
    }
}

fn format_rust_trace(
    config: &RunConfig,
    rust_snapshots: &[forge_parity::protocol::StateSnapshot],
    rust_decisions: &[DecisionRecord],
) -> String {
    report::format_trace_text(&forge_parity::protocol::GameTrace {
        seed: config.seed,
        deck1: config.deck1.clone(),
        deck2: config.deck2.clone(),
        max_turns: config.max_turns,
        snapshots: rust_snapshots.to_vec(),
        decisions: rust_decisions.to_vec(),
        covered_cards: vec![],
        mechanic_signals: vec![],
    })
}

fn format_java_trace(
    config: &RunConfig,
    java_snapshots: &[forge_parity::protocol::StateSnapshot],
    java_decisions: &[DecisionRecord],
) -> String {
    report::format_trace_text(&forge_parity::protocol::GameTrace {
        seed: config.seed,
        deck1: config.deck1.clone(),
        deck2: config.deck2.clone(),
        max_turns: config.max_turns,
        snapshots: java_snapshots.to_vec(),
        decisions: java_decisions.to_vec(),
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

    /// Run a matchup on any available server with streaming snapshot comparison.
    /// The callback `on_snapshot(index, &snapshot)` is called for each Java snapshot.
    /// Return `false` to signal divergence — remaining snapshots are skipped (not parsed)
    /// but output is drained to keep the protocol in sync.
    fn run_matchup_streaming<F>(
        &self,
        deck1: &str,
        deck2: &str,
        seed: u64,
        max_turns: u32,
        prefer_actions: bool,
        on_snapshot: F,
    ) -> Result<JavaMatchupData, JavaBridgeError>
    where
        F: FnMut(usize, &forge_parity::protocol::StateSnapshot) -> bool,
    {
        for server_mutex in &self.servers {
            if let Ok(mut server) = server_mutex.try_lock() {
                if !server.is_alive() {
                    continue;
                }
                return server.run_matchup_streaming(
                    deck1,
                    deck2,
                    seed,
                    max_turns,
                    prefer_actions,
                    on_snapshot,
                );
            }
        }
        let mut server = self.servers[0]
            .lock()
            .map_err(|e| JavaBridgeError::ProtocolError(format!("Mutex poisoned: {}", e)))?;
        server.run_matchup_streaming(deck1, deck2, seed, max_turns, prefer_actions, on_snapshot)
    }

    /// Shutdown all servers in parallel.
    fn shutdown(self) {
        let handles: Vec<_> = self
            .servers
            .into_iter()
            .map(|server_mutex| {
                std::thread::spawn(move || {
                    if let Ok(server) = server_mutex.into_inner() {
                        server.shutdown();
                    }
                })
            })
            .collect();
        for h in handles {
            let _ = h.join();
        }
    }
}

fn run_matrix_mode(cli: &Cli) {
    let decks_dir = cli.decks_dir.as_deref().unwrap_or(DEFAULT_DECKS_DIR);
    let seeds = cli.seeds.clone().unwrap_or_else(|| vec![42, 100, 999]);
    let deck_names: Vec<String> = filter_decks(
        cli.decks
            .clone()
            .unwrap_or_else(|| available_presets(decks_dir)),
        &cli.exclude_prefix,
    );

    // Validate deck names
    let valid = available_presets(decks_dir);
    for d in &deck_names {
        if !valid.contains(d) {
            eprintln!("[parity] Unknown deck '{}'. Available: {:?}", d, valid);
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

    // Spawn server pool if Java JAR is provided.
    // Default worker count is memory-aware: caps to what fits in RAM at the given heap size.
    let num_workers = cli.java_workers.unwrap_or_else(|| {
        if cli.java_jar.is_some() {
            max_workers_for_memory(&cli.java_heap)
        } else {
            0
        }
    });

    let pool = if let Some(ref jar_path) = cli.java_jar {
        let server_config = JavaServerConfig {
            jar_path: jar_path.clone(),
            forge_home: None,
            decks_dir: cli.decks_dir.clone(),
            verbose: cli.verbose,
            java_heap: cli.java_heap.clone(),
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

    // Open Java output cache (unless --no-cache).
    // Source hash covers all Java source + deck definitions — when any of
    // these change the entire cache is wiped automatically.
    let java_cache: Option<JavaCache> = if !cli.no_cache && cli.java_jar.is_some() {
        let project_root = std::env::current_dir().unwrap_or_default();
        let source_hash = if project_root.join("forge/forge-harness/src").exists() {
            java_cache::compute_source_hash(&project_root)
        } else if let Some(ref jar) = cli.java_jar {
            java_cache::compute_jar_hash(jar).unwrap_or_default()
        } else {
            String::new()
        };
        match JavaCache::open(std::path::Path::new(&cli.cache_dir), source_hash) {
            Ok(c) => {
                eprintln!(
                    "[parity] Java cache: {} (hash={})",
                    cli.cache_dir,
                    c.source_hash()
                );
                Some(c)
            }
            Err(e) => {
                eprintln!(
                    "[parity] Failed to open Java cache: {} (continuing without)",
                    e
                );
                None
            }
        }
    } else {
        None
    };

    // Two-phase execution: run all Rust games first (very fast), then feed
    // Java servers with streaming comparison. This maximizes Java server
    // utilization — servers are never idle waiting for Rust to finish.

    // Phase 1: Run ALL Rust games in parallel (typically finishes in seconds)
    let rust_completed = AtomicUsize::new(0);
    let rust_phase: Vec<(RunConfig, Result<forge_parity::protocol::GameTrace, String>)> = jobs
        .par_iter()
        .map(|&(d1, d2, seed)| {
            let config = RunConfig {
                deck1: d1.to_string(),
                deck2: d2.to_string(),
                seed,
                max_turns: cli.max_turns,
                cards_dir: cli.cards_dir.clone(),
                decks_dir: cli.decks_dir.clone(),
                verbose: cli.verbose,
                prefer_actions: cli.prefer_actions,
                java_heap: cli.java_heap.clone(),
            };
            let result = runner::run_with_data(&config, &data);
            if cli.verbose {
                let n = rust_completed.fetch_add(1, Ordering::Relaxed) + 1;
                eprintln!(
                    "[parity] [Rust {}/{}] {} vs {} seed={} ... {}",
                    n,
                    total,
                    d1,
                    d2,
                    seed,
                    if result.is_ok() { "OK" } else { "ERROR" }
                );
            }
            (config, result)
        })
        .collect();

    if cli.verbose {
        eprintln!("[parity] Phase 1 complete: all Rust games finished");
    }

    // Phase 2: Compare Rust results against Java output.
    // Uses cached Java output when available, falling back to live Java.
    let cache_hits = AtomicUsize::new(0);
    let cache_misses = AtomicUsize::new(0);
    let results: Vec<MatchupResult> = rust_phase
        .par_iter()
        .map(|(config, rust_result)| {
            let result = match rust_result {
                Ok(trace) => {
                    // Check Java cache first
                    if let Some(ref cache) = java_cache {
                        if let Some(cached_java) = cache.get(
                            &config.deck1,
                            &config.deck2,
                            config.seed,
                            config.max_turns,
                            config.prefer_actions,
                        ) {
                            cache_hits.fetch_add(1, Ordering::Relaxed);
                            let mut result = compare_snapshots(
                                config,
                                &trace.snapshots,
                                &trace.decisions,
                                &cached_java,
                            );
                            result.covered_cards = trace.covered_cards.clone();
                            result.mechanic_signals = trace.mechanic_signals.clone();
                            return result;
                        }
                        cache_misses.fetch_add(1, Ordering::Relaxed);
                    }

                    // Cache miss — run Java live and cache the result
                    if let Some(ref pool) = pool {
                        run_java_compare_and_cache(config, trace, pool, &java_cache)
                    } else if let Some(ref jar_path) = cli.java_jar {
                        run_java_streaming_compare_oneshot(config, trace, jar_path)
                    } else {
                        build_rust_only_result(config, trace)
                    }
                }
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
            };

            if cli.verbose {
                let n = completed.fetch_add(1, Ordering::Relaxed) + 1;
                match result.status {
                    MatchupStatus::Pass => {
                        eprintln!(
                            "[parity] [{}/{}] {} vs {} seed={} ... PASS ({} snapshots)",
                            n,
                            total,
                            config.deck1,
                            config.deck2,
                            config.seed,
                            result.snapshots_compared
                        );
                    }
                    MatchupStatus::Fail => {
                        eprintln!(
                            "[parity] [{}/{}] {} vs {} seed={} ... FAIL ({} divergences)",
                            n,
                            total,
                            config.deck1,
                            config.deck2,
                            config.seed,
                            result.divergence_count
                        );
                    }
                    MatchupStatus::Error => {
                        eprintln!(
                            "[parity] [{}/{}] {} vs {} seed={} ... ERROR: {}",
                            n,
                            total,
                            config.deck1,
                            config.deck2,
                            config.seed,
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

    // Log cache stats
    let hits = cache_hits.load(Ordering::Relaxed);
    let misses = cache_misses.load(Ordering::Relaxed);
    if hits + misses > 0 {
        eprintln!(
            "[parity] Java cache: {} hits, {} misses ({:.0}% hit rate)",
            hits,
            misses,
            if hits + misses > 0 {
                (hits as f64 / (hits + misses) as f64) * 100.0
            } else {
                0.0
            }
        );
    }

    let passed = results
        .iter()
        .filter(|r| r.status == MatchupStatus::Pass)
        .count();
    let failed = results
        .iter()
        .filter(|r| r.status == MatchupStatus::Fail)
        .count();
    let errors = results
        .iter()
        .filter(|r| r.status == MatchupStatus::Error)
        .count();

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

/// Run Java via server pool with streaming comparison against pre-computed Rust trace.
/// Compares each Java snapshot as it arrives, skipping JSON parsing after divergence.
/// Run Java via pool, compare against Rust trace, and cache the Java output on pass.
/// Uses a non-streaming run so the full `JavaMatchupData` is available for caching.
fn run_java_compare_and_cache(
    config: &RunConfig,
    rust_trace: &forge_parity::protocol::GameTrace,
    pool: &ServerPool,
    cache: &Option<JavaCache>,
) -> MatchupResult {
    // Run Java (collect all data — needed for caching)
    let java_data = match pool.run_matchup_streaming(
        &config.deck1,
        &config.deck2,
        config.seed,
        config.max_turns,
        config.prefer_actions,
        |_, _| true, // collect all snapshots
    ) {
        Ok(data) => data,
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
                covered_cards: rust_trace.covered_cards.clone(),
                mechanic_signals: rust_trace.mechanic_signals.clone(),
                finished_turn: None,
            };
        }
    };

    let mut result = compare_snapshots(
        config,
        &rust_trace.snapshots,
        &rust_trace.decisions,
        &java_data,
    );
    result.covered_cards = rust_trace.covered_cards.clone();
    result.mechanic_signals = rust_trace.mechanic_signals.clone();

    // Cache Java output — Java is deterministic for a given source hash,
    // so both passes and failures produce the same Java data next time.
    if result.status != MatchupStatus::Error {
        if let Some(ref cache) = cache {
            let _ = cache.put(
                &config.deck1,
                &config.deck2,
                config.seed,
                config.max_turns,
                config.prefer_actions,
                &java_data,
            );
        }
    }

    result
}

/// Run Java via server pool with streaming comparison against pre-computed Rust trace.
/// Compares each Java snapshot as it arrives, skipping JSON parsing after divergence.
#[allow(dead_code)]
fn run_java_streaming_compare_pool(
    config: &RunConfig,
    rust_trace: &forge_parity::protocol::GameTrace,
    pool: &ServerPool,
) -> MatchupResult {
    let rust_snapshots = &rust_trace.snapshots;
    let mut first_divergence: Option<Divergence> = None;
    let mut compared_until: usize = 0;

    let java_result = pool.run_matchup_streaming(
        &config.deck1,
        &config.deck2,
        config.seed,
        config.max_turns,
        config.prefer_actions,
        |idx, java_snap| {
            if let Some(rust_snap) = rust_snapshots.get(idx) {
                let divs = comparator::compare(idx, rust_snap, java_snap);
                if let Some(div) = divs.into_iter().next() {
                    first_divergence = Some(div);
                    compared_until = idx + 1;
                    return false; // divergence found — stop storing
                }
            } else {
                // Java has more snapshots than Rust
                first_divergence = Some(Divergence {
                    snapshot_index: idx,
                    turn: java_snap.turn,
                    phase: java_snap.phase.clone(),
                    field: "snapshot.exists".into(),
                    rust_value: "missing".into(),
                    java_value: "present".into(),
                });
                compared_until = idx + 1;
                return false;
            }
            compared_until = idx + 1;
            true
        },
    );

    let java_data = match java_result {
        Ok(data) => data,
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
                covered_cards: rust_trace.covered_cards.clone(),
                mechanic_signals: rust_trace.mechanic_signals.clone(),
                finished_turn: None,
            };
        }
    };

    // Check if Rust has more snapshots than Java
    if first_divergence.is_none() && rust_snapshots.len() > java_data.snapshots.len() {
        let idx = java_data.snapshots.len();
        if let Some(rs) = rust_snapshots.get(idx) {
            first_divergence = Some(Divergence {
                snapshot_index: idx,
                turn: rs.turn,
                phase: rs.phase.clone(),
                field: "snapshot.exists".into(),
                rust_value: "present".into(),
                java_value: "missing".into(),
            });
            compared_until = idx + 1;
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
        snapshots_compared: if first_divergence.is_some() {
            compared_until
        } else {
            rust_snapshots.len().max(java_data.snapshots.len())
        },
        divergence_count,
        trace: first_divergence.as_ref().map(|_| {
            format_rust_trace(
                config,
                &rust_snapshots[..compared_until.min(rust_snapshots.len())],
                &rust_trace.decisions,
            )
        }),
        java_trace: first_divergence.as_ref().map(|_| {
            format_java_trace(
                config,
                &java_data.snapshots[..compared_until.min(java_data.snapshots.len())],
                &java_data.decisions,
            )
        }),
        first_divergence,
        error_message: None,
        covered_cards: rust_trace.covered_cards.clone(),
        mechanic_signals: rust_trace.mechanic_signals.clone(),
        finished_turn: rust_snapshots.last().and_then(|s| {
            if s.game_over {
                Some(s.turn)
            } else {
                None
            }
        }),
    }
}

/// Run Java via one-shot bridge with streaming comparison against pre-computed Rust trace.
fn run_java_streaming_compare_oneshot(
    config: &RunConfig,
    rust_trace: &forge_parity::protocol::GameTrace,
    jar_path: &PathBuf,
) -> MatchupResult {
    // For one-shot mode, run Java and compare after (no streaming support on subprocess)
    let bridge_config = JavaBridgeConfig {
        jar_path: jar_path.clone(),
        seed: config.seed,
        max_turns: config.max_turns,
        deck1: config.deck1.clone(),
        deck2: config.deck2.clone(),
        forge_home: None,
        decks_dir: config.decks_dir.clone(),
        verbose: config.verbose,
        prefer_actions: config.prefer_actions,
        java_heap: config.java_heap.clone(),
    };
    let bridge = JavaBridge::new(bridge_config);
    let java_data = match bridge.run() {
        Ok(data) => data,
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
                covered_cards: rust_trace.covered_cards.clone(),
                mechanic_signals: rust_trace.mechanic_signals.clone(),
                finished_turn: None,
            };
        }
    };
    let mut result = compare_snapshots(
        config,
        &rust_trace.snapshots,
        &rust_trace.decisions,
        &java_data,
    );
    result.covered_cards = rust_trace.covered_cards.clone();
    result.mechanic_signals = rust_trace.mechanic_signals.clone();
    result
}

/// Build a MatchupResult from a completed Rust-only trace (no Java).
fn build_rust_only_result(
    config: &RunConfig,
    trace: &forge_parity::protocol::GameTrace,
) -> MatchupResult {
    MatchupResult {
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
        covered_cards: trace.covered_cards.clone(),
        mechanic_signals: trace.mechanic_signals.clone(),
        finished_turn: trace.snapshots.last().and_then(|s| {
            if s.game_over {
                Some(s.turn)
            } else {
                None
            }
        }),
    }
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
            decks_dir: cli.decks_dir.clone(),
            verbose: cli.verbose,
            java_heap: cli.java_heap.clone(),
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
            decks_dir: cli.decks_dir.clone(),
            verbose: cli.verbose,
            prefer_actions: cli.prefer_actions,
            java_heap: cli.java_heap.clone(),
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
                    decks_dir: cli.decks_dir.clone(),
                    verbose: cli.verbose,
                    java_heap: cli.java_heap.clone(),
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

fn collect_unique_deck_cards(deck1: &str, deck2: &str, decks_dir: &str) -> Vec<String> {
    let mut cards: BTreeSet<String> = BTreeSet::new();
    for deck in [deck1, deck2] {
        match runner::resolve_deck_spec(deck, decks_dir) {
            Ok(spec) => {
                for (name, _) in spec {
                    cards.insert(name);
                }
            }
            Err(e) => {
                eprintln!(
                    "[parity] Coverage warning: failed to parse deck '{}': {}",
                    deck, e
                );
            }
        }
    }
    cards.into_iter().collect()
}

fn build_coverage_report(
    deck1: &str,
    deck2: &str,
    results: &[MatchupResult],
    decks_dir: &str,
) -> String {
    let deck_cards = collect_unique_deck_cards(deck1, deck2, decks_dir);
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
    decks_dir: &str,
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
    let defs = db.map(|card_db| collect_defined_mechanics(deck1, deck2, card_db, decks_dir));
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

fn collect_defined_mechanics(
    deck1: &str,
    deck2: &str,
    db: &CardDatabase,
    decks_dir: &str,
) -> DefinedMechanics {
    let mut card_names: BTreeSet<String> = BTreeSet::new();
    for deck in [deck1, deck2] {
        match runner::resolve_deck_spec(deck, decks_dir) {
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
            defs.ability_keys
                .insert(format!("Activated ability: {}", ab));
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
            defs.ability_keys
                .insert(format!("Activated ability: {}", ab));
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

/// Parse a JVM heap size string (e.g. "512m", "1g") into bytes.
fn parse_heap_bytes(heap: &str) -> u64 {
    let heap = heap.trim().to_lowercase();
    if let Some(n) = heap.strip_suffix('g') {
        n.parse::<u64>().unwrap_or(1) * 1024 * 1024 * 1024
    } else if let Some(n) = heap.strip_suffix('m') {
        n.parse::<u64>().unwrap_or(512) * 1024 * 1024
    } else if let Some(n) = heap.strip_suffix('k') {
        n.parse::<u64>().unwrap_or(512_000) * 1024
    } else {
        heap.parse::<u64>().unwrap_or(512 * 1024 * 1024)
    }
}

/// Compute max Java workers that fit in available system memory.
/// Reserves 512MB for OS + Rust process, then divides remaining by per-worker heap.
fn max_workers_for_memory(java_heap: &str) -> usize {
    let heap_per_worker = parse_heap_bytes(java_heap);
    if heap_per_worker == 0 {
        return 1;
    }
    // Read total system memory from /proc/meminfo (Linux) or sysctl (macOS)
    let total_mem = get_total_memory_bytes();
    if total_mem == 0 {
        return num_cpus(); // can't detect, fall back to CPU count
    }
    let reserved = 512 * 1024 * 1024u64; // 512MB for OS + Rust
    let available = total_mem.saturating_sub(reserved);
    let max = (available / heap_per_worker) as usize;
    max.max(1).min(num_cpus()) // at least 1, at most num_cpus
}

fn get_total_memory_bytes() -> u64 {
    // Try /proc/meminfo first (Linux / Docker)
    if let Ok(contents) = std::fs::read_to_string("/proc/meminfo") {
        for line in contents.lines() {
            if let Some(rest) = line.strip_prefix("MemTotal:") {
                let rest = rest.trim();
                if let Some(kb_str) = rest.strip_suffix("kB").or(rest.strip_suffix("KB")) {
                    if let Ok(kb) = kb_str.trim().parse::<u64>() {
                        return kb * 1024;
                    }
                }
            }
        }
    }
    // macOS: sysctl hw.memsize
    if let Ok(output) = std::process::Command::new("sysctl")
        .arg("-n")
        .arg("hw.memsize")
        .output()
    {
        if let Ok(s) = std::str::from_utf8(&output.stdout) {
            if let Ok(bytes) = s.trim().parse::<u64>() {
                return bytes;
            }
        }
    }
    0 // unknown
}

// ── Continuous Parity Mode ──────────────────────────────────────────

#[cfg(feature = "storage")]
fn run_continuous_mode(cli: &Cli) {
    use forge_parity::scheduler::Scheduler;
    use forge_parity::storage::Storage;
    use std::time::Instant;

    let jar_path = match &cli.java_jar {
        Some(p) => p.clone(),
        None => {
            eprintln!("[parity] --continuous requires --java-jar");
            std::process::exit(1);
        }
    };

    let max_games = cli.max_games.unwrap_or(100);
    if cli.verbose {
        eprintln!(
            "[parity] Continuous mode: max_games={}, threshold={:.1}%, db={}",
            max_games,
            cli.threshold * 100.0,
            cli.db_path
        );
    }

    // Open database
    let db = match Storage::open(&cli.db_path) {
        Ok(db) => db,
        Err(e) => {
            eprintln!("[parity] Failed to open database: {}", e);
            std::process::exit(1);
        }
    };

    // Load card database
    let data = match runner::load_data(cli.cards_dir.as_deref(), cli.verbose) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[parity] Load error: {}", e);
            std::process::exit(1);
        }
    };

    // Discover preset decks
    let dd = cli.decks_dir.as_deref().unwrap_or(DEFAULT_DECKS_DIR);
    let deck_names: Vec<String> = filter_decks(
        match &cli.decks {
            Some(d) => d.clone(),
            None => available_presets(dd),
        },
        &cli.exclude_prefix,
    );

    if deck_names.is_empty() {
        eprintln!("[parity] No preset decks found in {}", dd);
        std::process::exit(1);
    }
    if cli.verbose {
        eprintln!(
            "[parity] Using {} preset decks: {:?}",
            deck_names.len(),
            deck_names
        );
    }

    // Initialize scheduler
    let fuzz_db = if cli.fuzz_per_batch > 0 {
        Some(&data.db)
    } else {
        None
    };
    let mut scheduler =
        Scheduler::new(&deck_names, cli.seed, cli.fuzz_per_batch, fuzz_db, false, 1);

    // Resume from the last pair played so the matrix completes across restarts.
    // Seeds restart from cli.seed so the Java cache gives instant hits.
    if let Ok(Some((d1, d2))) = db.last_preset_pair() {
        if scheduler.resume_after(&d1, &d2) {
            eprintln!("[parity] Resuming after {} vs {}", d1, d2);
        }
    }

    // Spawn Java server
    let server_config = JavaServerConfig {
        jar_path: jar_path.clone(),
        forge_home: None,
        decks_dir: cli.decks_dir.clone(),
        verbose: cli.verbose,
        java_heap: cli.java_heap.clone(),
    };
    let mut server = match JavaServer::spawn(&server_config) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[parity] Failed to spawn Java server: {}", e);
            std::process::exit(1);
        }
    };

    let start = Instant::now();
    let mut completed = 0usize;
    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut errors = 0usize;

    // Main loop
    while completed < max_games {
        let job = scheduler.next_job();

        let config = RunConfig {
            deck1: job.deck1.clone(),
            deck2: job.deck2.clone(),
            seed: job.seed,
            max_turns: cli.max_turns,
            cards_dir: cli.cards_dir.clone(),
            decks_dir: cli.decks_dir.clone(),
            verbose: cli.verbose,
            prefer_actions: cli.prefer_actions,
            java_heap: cli.java_heap.clone(),
        };

        let game_start = Instant::now();

        // Respawn server if dead
        if !server.is_alive() {
            eprintln!("[parity] Java server died, respawning...");
            server = match JavaServer::spawn(&server_config) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("[parity] Failed to respawn Java server: {}", e);
                    break;
                }
            };
        }

        let result = run_single_matchup_server(&config, &data, &mut server);
        let duration_ms = game_start.elapsed().as_millis() as u64;

        match result.status {
            MatchupStatus::Pass => passed += 1,
            MatchupStatus::Fail => failed += 1,
            MatchupStatus::Error => errors += 1,
        }
        completed += 1;

        if let Err(e) = db.insert_run(job.batch_id, &result, duration_ms, job.is_fuzz, None) {
            eprintln!("[parity] DB insert error: {}", e);
        }

        // Progress logging
        if cli.verbose {
            let status_char = match result.status {
                MatchupStatus::Pass => "\x1b[32mPASS\x1b[0m",
                MatchupStatus::Fail => "\x1b[31mFAIL\x1b[0m",
                MatchupStatus::Error => "\x1b[33mERROR\x1b[0m",
            };
            let current_rate = if passed + failed > 0 {
                passed as f64 / (passed + failed) as f64
            } else {
                0.0
            };
            eprintln!(
                "[parity] [{}/{}] {} vs {} seed={} {} (rate={:.1}%, {}ms)",
                completed,
                max_games,
                short_deck(&job.deck1),
                short_deck(&job.deck2),
                job.seed,
                status_char,
                current_rate * 100.0,
                duration_ms
            );
        }
    }

    server.shutdown();

    let elapsed = start.elapsed();
    let pass_rate = if passed + failed > 0 {
        passed as f64 / (passed + failed) as f64
    } else {
        0.0
    };
    let gpm = if elapsed.as_secs() > 0 {
        completed as f64 / (elapsed.as_secs() as f64 / 60.0)
    } else {
        0.0
    };

    eprintln!();
    eprintln!("=== Continuous Parity Summary ===");
    eprintln!("  Total games:    {}", completed);
    eprintln!(
        "  Passed:         {} ({:.1}%)",
        passed,
        if completed > 0 {
            passed as f64 / completed as f64 * 100.0
        } else {
            0.0
        }
    );
    eprintln!("  Failed:         {}", failed);
    eprintln!("  Errors:         {}", errors);
    eprintln!(
        "  Pass rate:      {:.1}% (threshold: {:.1}%)",
        pass_rate * 100.0,
        cli.threshold * 100.0
    );
    eprintln!(
        "  Duration:       {:.1}s ({:.1} games/min)",
        elapsed.as_secs_f64(),
        gpm
    );
    eprintln!("  Database:       {}", cli.db_path);
    eprintln!();

    if pass_rate >= cli.threshold {
        eprintln!(
            "\x1b[32mPASSED\x1b[0m — pass rate {:.1}% >= threshold {:.1}%",
            pass_rate * 100.0,
            cli.threshold * 100.0
        );
        std::process::exit(0);
    } else {
        eprintln!(
            "\x1b[31mFAILED\x1b[0m — pass rate {:.1}% < threshold {:.1}%",
            pass_rate * 100.0,
            cli.threshold * 100.0
        );
        std::process::exit(1);
    }
}

/// Shorten an inline deck spec for display.
#[cfg(feature = "storage")]
fn short_deck(spec: &str) -> &str {
    if let Some(rest) = spec.strip_prefix("inline:") {
        let first_pipe = rest.find('|').unwrap_or(rest.len());
        &spec[..("inline:".len() + first_pipe).min(spec.len()).min(30)]
    } else if spec.len() > 20 {
        &spec[..20]
    } else {
        spec
    }
}

// ── Serve Mode ──────────────────────────────────────────────────────

#[cfg(feature = "serve")]
fn run_serve_mode(cli: &Cli) {
    use forge_parity::log_buffer::{BufferLayer, LogBuffer};
    use forge_parity::scheduler::Scheduler;
    use forge_parity::storage::Storage;
    use forge_parity::web::{self, DashboardConfig};
    use std::backtrace::Backtrace;
    use std::sync::atomic::Ordering;
    use std::sync::Arc;
    use std::time::Instant;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    // Init tracing with ring-buffer layer + stderr output
    let log_buffer = LogBuffer::new();
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_writer(std::io::stderr),
        )
        .with(BufferLayer::new(log_buffer.clone()))
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&cli.log_level)),
        )
        .init();

    std::panic::set_hook(Box::new(|panic_info| {
        let location = panic_info
            .location()
            .map(|loc| format!("{}:{}", loc.file(), loc.line()))
            .unwrap_or_else(|| "unknown location".to_string());
        let payload = panic_info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| (*s).to_string())
            .or_else(|| panic_info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "non-string panic payload".to_string());
        let backtrace = Backtrace::force_capture();
        tracing::error!(
            location = %location,
            payload = %payload,
            backtrace = %backtrace,
            "forge-parity panicked"
        );
    }));

    let jar_path = match &cli.java_jar {
        Some(p) => p.clone(),
        None => {
            tracing::error!("--serve requires --java-jar");
            std::process::exit(1);
        }
    };

    let max_games = cli.max_games; // None = unlimited
    let port = cli.port;

    tracing::info!(
        port,
        max_games = max_games.map(|n| n as i64).unwrap_or(-1),
        threshold = cli.threshold * 100.0,
        db = %cli.db_path,
        "Serve mode starting"
    );

    // Open database
    let db = match Storage::open(&cli.db_path) {
        Ok(db) => db,
        Err(e) => {
            tracing::error!(%e, "Failed to open database");
            std::process::exit(1);
        }
    };

    // Load card database
    let data = match runner::load_data(cli.cards_dir.as_deref(), cli.verbose) {
        Ok(d) => d,
        Err(e) => {
            tracing::error!(%e, "Failed to load card data");
            std::process::exit(1);
        }
    };

    // Discover preset decks
    let dd = cli.decks_dir.as_deref().unwrap_or(DEFAULT_DECKS_DIR);
    let deck_names: Vec<String> = filter_decks(
        match &cli.decks {
            Some(d) => d.clone(),
            None => available_presets(dd),
        },
        &cli.exclude_prefix,
    );

    if deck_names.is_empty() {
        tracing::error!(dir = dd, "No preset decks found");
        std::process::exit(1);
    }
    tracing::info!(count = deck_names.len(), decks = ?deck_names, "Preset decks loaded");

    // Create shared dashboard config
    let dashboard_config = Arc::new(DashboardConfig::new());
    dashboard_config
        .fuzz_enabled
        .store(cli.fuzz_per_batch > 0, Ordering::Relaxed);
    if cli.analyze {
        dashboard_config
            .analysis_running
            .store(true, Ordering::Relaxed);
    }

    let initial_fuzz = cli.fuzz_per_batch > 0;
    let fuzz_db = if initial_fuzz { Some(&data.db) } else { None };
    let initial_fuzz_per_batch = if initial_fuzz { cli.fuzz_per_batch } else { 0 };
    let mut scheduler = Scheduler::new(
        &deck_names,
        cli.seed,
        initial_fuzz_per_batch,
        fuzz_db,
        dashboard_config.self_matchups.load(Ordering::Relaxed),
        dashboard_config.games_per_matchup.load(Ordering::Relaxed),
    );

    // Resume from the last pair played so the matrix completes across restarts.
    // Seeds restart from cli.seed so the Java cache gives instant hits.
    if let Ok(Some((d1, d2))) = db.last_preset_pair() {
        if scheduler.resume_after(&d1, &d2) {
            tracing::info!(
                last_deck1 = %d1,
                last_deck2 = %d2,
                "Resuming matrix after last played pair"
            );
        }
    }

    // Spawn Java server
    let server_config = JavaServerConfig {
        jar_path: jar_path.clone(),
        forge_home: None,
        decks_dir: cli.decks_dir.clone(),
        verbose: cli.verbose,
        java_heap: cli.java_heap.clone(),
    };
    let mut java_server = match JavaServer::spawn(&server_config) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(%e, "Failed to spawn Java server");
            std::process::exit(1);
        }
    };

    let job_queue = Arc::new(web::JobQueue::new());

    let now_iso = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let start = Instant::now();
    // Detect git commit SHA at startup
    let commit_sha = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
        .or_else(|| std::env::var("GIT_COMMIT_SHA").ok());

    let app_state = Arc::new(web::AppState {
        storage: std::sync::Mutex::new(db),
        start_time: start,
        start_time_iso: now_iso,
        config: Arc::clone(&dashboard_config),
        logs: log_buffer,
        job_queue: Arc::clone(&job_queue),
        commit_sha,
        exclude_prefixes: cli.exclude_prefix.clone(),
    });

    // Build tokio runtime
    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");

    // Spawn web server in background
    let app_state_web = Arc::clone(&app_state);
    rt.spawn(async move {
        let router = web::build_router(app_state_web);
        let listener = match tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await {
            Ok(listener) => listener,
            Err(e) => {
                tracing::error!(%e, port, "Failed to bind dashboard port");
                return;
            }
        };
        tracing::info!(port, "Dashboard available at http://localhost:{}", port);
        if let Err(e) = axum::serve(listener, router).await {
            tracing::error!(%e, port, "Dashboard server exited");
        }
    });

    // Always spawn analysis daemon (paused by default, toggle via dashboard)
    #[cfg(feature = "analyze")]
    {
        use forge_parity::analyzer::{self, AnalyzerConfig};

        let analyzer_db = match Storage::open(&cli.db_path) {
            Ok(db) => db,
            Err(e) => {
                tracing::error!(%e, "Failed to open analyzer database");
                std::process::exit(1);
            }
        };
        let analyzer_storage = Arc::new(std::sync::Mutex::new(analyzer_db));
        let analysis_running = Arc::clone(&dashboard_config.analysis_running);
        let analyzer_config = AnalyzerConfig {
            poll_interval: std::time::Duration::from_secs(cli.poll_interval),
            summary_interval: std::time::Duration::from_secs(cli.summary_interval),
            issue_threshold: resolve_issue_threshold(cli.issue_threshold),
            github_repo: resolve_github_repo(cli.github_repo.clone()),
            dashboard_url: Some(format!("http://localhost:{}", port)),
            java_jar: cli
                .java_jar
                .as_ref()
                .map(|p| p.to_string_lossy().to_string()),
            cards_dir: cli.cards_dir.clone(),
            project_root: std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".to_string()),
        };

        rt.spawn(async move {
            analyzer::run(analyzer_storage, analyzer_config, analysis_running).await;
        });
        tracing::info!(running = cli.analyze, "Analysis daemon spawned");
    }

    // Run game loop on main thread (blocking)
    let ci_mode = cli.ci;
    let cli_max_turns = cli.max_turns;
    let cli_cards_dir = cli.cards_dir.clone();
    let cli_decks_dir = cli.decks_dir.clone();
    let cli_verbose = cli.verbose;
    let cli_prefer_actions = cli.prefer_actions;
    let cli_java_heap = cli.java_heap.clone();
    let cfg = Arc::clone(&dashboard_config);

    let mut completed = 0usize;
    // Track config values to detect changes and rebuild scheduler
    let mut prev_games_per_matchup = cfg.games_per_matchup.load(Ordering::Relaxed);
    let mut prev_fuzz_enabled = cfg.fuzz_enabled.load(Ordering::Relaxed);
    let mut prev_self_matchups = cfg.self_matchups.load(Ordering::Relaxed);

    loop {
        // 1. Check job queue first (priority over scheduler)
        let queued = job_queue.queue.lock().unwrap().pop_front();
        if let Some(queued_job) = queued {
            {
                let mut batches = job_queue.batches.lock().unwrap();
                if let Some(batch) = batches.get_mut(&queued_job.batch_id) {
                    batch.active_job = Some(web::ActiveJob {
                        regression_name: queued_job.regression_name.clone(),
                        deck1: queued_job.deck1.clone(),
                        deck2: queued_job.deck2.clone(),
                        seed: queued_job.seed,
                        max_turns: queued_job.max_turns,
                    });
                }
            }

            let config = RunConfig {
                deck1: queued_job.deck1.clone(),
                deck2: queued_job.deck2.clone(),
                seed: queued_job.seed,
                max_turns: queued_job.max_turns,
                cards_dir: cli_cards_dir.clone(),
                decks_dir: cli_decks_dir.clone(),
                verbose: cli_verbose,
                prefer_actions: cli_prefer_actions,
                java_heap: cli_java_heap.clone(),
            };

            let game_start = Instant::now();

            // Respawn server if dead
            if !java_server.is_alive() {
                tracing::warn!("Java server died, respawning...");
                java_server = match JavaServer::spawn(&server_config) {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::error!(%e, "Failed to respawn Java server");
                        // Record error for batch
                        let mut batches = job_queue.batches.lock().unwrap();
                        if let Some(batch) = batches.get_mut(&queued_job.batch_id) {
                            batch.completed += 1;
                            batch.errors += 1;
                            batch.active_job = None;
                            batch.push_result(web::JobResult {
                                deck1: queued_job.deck1.clone(),
                                deck2: queued_job.deck2.clone(),
                                seed: queued_job.seed,
                                status: "error".into(),
                                error: Some(format!("Java server respawn failed: {}", e)),
                                divergence_field: None,
                                rust_value: None,
                                java_value: None,
                                divergence_location: None,
                                rust_trace: None,
                                java_trace: None,
                            });
                            if batch.completed >= batch.total {
                                batch.done = true;
                            }
                        }
                        std::thread::sleep(std::time::Duration::from_secs(5));
                        continue;
                    }
                };
            }

            let result = run_single_matchup_server(&config, &data, &mut java_server);
            let duration_ms = game_start.elapsed().as_millis() as u64;

            let status_str = match result.status {
                MatchupStatus::Pass => "pass",
                MatchupStatus::Fail => "fail",
                MatchupStatus::Error => "error",
            };

            tracing::info!(
                batch = queued_job.batch_id,
                regression = %queued_job.regression_name,
                deck1 = %short_deck(&queued_job.deck1),
                deck2 = %short_deck(&queued_job.deck2),
                seed = queued_job.seed,
                ms = duration_ms,
                status = status_str,
                "CI job"
            );

            // Update batch status
            {
                let mut batches = job_queue.batches.lock().unwrap();
                if let Some(batch) = batches.get_mut(&queued_job.batch_id) {
                    batch.completed += 1;
                    batch.active_job = None;
                    match result.status {
                        MatchupStatus::Pass => batch.passed += 1,
                        MatchupStatus::Fail => batch.failed += 1,
                        MatchupStatus::Error => batch.errors += 1,
                    }
                    let (div_field, rust_val, java_val, div_loc) =
                        if let Some(ref div) = result.first_divergence {
                            (
                                Some(div.field.clone()),
                                Some(div.rust_value.clone()),
                                Some(div.java_value.clone()),
                                Some(format!("turn {} {}", div.turn, div.phase)),
                            )
                        } else {
                            (None, None, None, None)
                        };
                    batch.push_result(web::JobResult {
                        deck1: queued_job.deck1.clone(),
                        deck2: queued_job.deck2.clone(),
                        seed: queued_job.seed,
                        status: status_str.to_string(),
                        error: result.error_message.clone(),
                        divergence_field: div_field,
                        rust_value: rust_val,
                        java_value: java_val,
                        divergence_location: div_loc,
                        rust_trace: result.trace.as_ref().map(|t| truncate_trace(t, 200)),
                        java_trace: result.java_trace.as_ref().map(|t| truncate_trace(t, 200)),
                    });
                    if batch.completed >= batch.total {
                        batch.done = true;
                    }
                }
            }

            // Store in DB
            {
                let storage = app_state.storage.lock().unwrap();
                if let Err(e) = storage.insert_run(
                    0,
                    &result,
                    duration_ms,
                    false,
                    app_state.commit_sha.as_deref(),
                ) {
                    tracing::error!(%e, "DB insert error");
                }
            }

            completed += 1;
            continue;
        }

        // 2. CI mode: if queue is empty, check if we should exit
        if ci_mode {
            let batches = job_queue.batches.lock().unwrap();
            if batches.is_empty() {
                // No batch submitted yet — idle-wait
                drop(batches);
                std::thread::sleep(std::time::Duration::from_millis(100));
                continue;
            }
            let all_done = batches.values().all(|b| b.done);
            if all_done {
                // Log summary but keep server alive for CI to poll results
                for (id, batch) in batches.iter() {
                    tracing::info!(
                        batch_id = id,
                        name = %batch.name,
                        total = batch.total,
                        passed = batch.passed,
                        failed = batch.failed,
                        errors = batch.errors,
                        "Batch complete — waiting for CI to poll results"
                    );
                }
            }
            drop(batches);
            // Keep server alive — CI will kill via server.pid after reading results
            std::thread::sleep(std::time::Duration::from_millis(100));
            continue;
        }

        // 3. Normal scheduler path (non-CI mode)

        // Pause check: if games are paused, sleep and retry
        if cfg.games_paused.load(Ordering::Relaxed) {
            std::thread::sleep(std::time::Duration::from_secs(1));
            continue;
        }

        if let Some(max) = max_games {
            if completed >= max {
                break;
            }
        }

        // Read live config from dashboard UI
        let games_per_matchup = cfg.games_per_matchup.load(Ordering::Relaxed);
        let fuzz_enabled = cfg.fuzz_enabled.load(Ordering::Relaxed);
        let self_matchups = cfg.self_matchups.load(Ordering::Relaxed);

        // Rebuild scheduler if config changed
        if games_per_matchup != prev_games_per_matchup
            || fuzz_enabled != prev_fuzz_enabled
            || self_matchups != prev_self_matchups
        {
            let fuzz_per = if fuzz_enabled {
                cli.fuzz_per_batch.max(5)
            } else {
                0
            };
            let fdb = if fuzz_enabled { Some(&data.db) } else { None };
            scheduler = Scheduler::new(
                &deck_names,
                cli.seed,
                fuzz_per,
                fdb,
                self_matchups,
                games_per_matchup,
            );
            // Resume from last pair played
            {
                let storage = app_state.storage.lock().unwrap();
                if let Ok(Some((d1, d2))) = storage.last_preset_pair() {
                    scheduler.resume_after(&d1, &d2);
                }
            }
            tracing::info!(
                games_per_matchup,
                fuzz_enabled,
                self_matchups,
                pairs = scheduler.preset_pairs_count(),
                "Config changed — scheduler rebuilt"
            );
            prev_games_per_matchup = games_per_matchup;
            prev_fuzz_enabled = fuzz_enabled;
            prev_self_matchups = self_matchups;
        }

        let job = scheduler.next_job();

        let config = RunConfig {
            deck1: job.deck1.clone(),
            deck2: job.deck2.clone(),
            seed: job.seed,
            max_turns: cli_max_turns,
            cards_dir: cli_cards_dir.clone(),
            decks_dir: cli_decks_dir.clone(),
            verbose: cli_verbose,
            prefer_actions: cli_prefer_actions,
            java_heap: cli_java_heap.clone(),
        };

        let game_start = Instant::now();

        // Respawn server if dead
        if !java_server.is_alive() {
            tracing::warn!("Java server died, respawning...");
            java_server = match JavaServer::spawn(&server_config) {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!(%e, "Failed to respawn Java server");
                    std::thread::sleep(std::time::Duration::from_secs(5));
                    continue;
                }
            };
        }

        let result = run_single_matchup_server(&config, &data, &mut java_server);
        let duration_ms = game_start.elapsed().as_millis() as u64;

        completed += 1;

        match result.status {
            MatchupStatus::Pass => {
                tracing::info!(
                    game = completed,
                    deck1 = %short_deck(&job.deck1),
                    deck2 = %short_deck(&job.deck2),
                    seed = job.seed,
                    ms = duration_ms,
                    "PASS"
                );
            }
            MatchupStatus::Fail => {
                tracing::warn!(
                    game = completed,
                    deck1 = %short_deck(&job.deck1),
                    deck2 = %short_deck(&job.deck2),
                    seed = job.seed,
                    ms = duration_ms,
                    field = result.first_divergence.as_ref().map(|d| d.field.as_str()).unwrap_or("-"),
                    "FAIL"
                );
            }
            MatchupStatus::Error => {
                tracing::error!(
                    game = completed,
                    deck1 = %short_deck(&job.deck1),
                    deck2 = %short_deck(&job.deck2),
                    seed = job.seed,
                    ms = duration_ms,
                    "ERROR"
                );
            }
        }

        // Write to storage under lock
        {
            let storage = app_state.storage.lock().unwrap();
            if let Err(e) = storage.insert_run(
                job.batch_id,
                &result,
                duration_ms,
                job.is_fuzz,
                app_state.commit_sha.as_deref(),
            ) {
                tracing::error!(%e, "DB insert error");
            }
        }

        // Throttle: sleep between games to avoid pegging CPU
        let delay = cfg.game_delay_ms.load(Ordering::Relaxed);
        if delay > 0 {
            std::thread::sleep(std::time::Duration::from_millis(delay as u64));
        }
    }

    java_server.shutdown();
    tracing::info!(games = completed, "Serve mode complete");
}

// ── Analyze-only Mode ──────────────────────────────────────────────

#[cfg(feature = "analyze")]
fn run_analyze_only(cli: &Cli) {
    use forge_parity::analyzer::{self, AnalyzerConfig};
    use forge_parity::storage::Storage;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    if cli.verbose {
        eprintln!(
            "[parity] Analyze-only mode: db={}, poll={}s, summary={}s, threshold={}",
            cli.db_path, cli.poll_interval, cli.summary_interval, cli.issue_threshold
        );
    }

    let db = match Storage::open(&cli.db_path) {
        Ok(db) => db,
        Err(e) => {
            eprintln!("[parity] Failed to open database: {}", e);
            std::process::exit(1);
        }
    };

    let storage = Arc::new(std::sync::Mutex::new(db));
    // In analyze-only mode, always running
    let running = Arc::new(AtomicBool::new(true));
    let config = AnalyzerConfig {
        poll_interval: std::time::Duration::from_secs(cli.poll_interval),
        summary_interval: std::time::Duration::from_secs(cli.summary_interval),
        issue_threshold: cli.issue_threshold,
        github_repo: cli.github_repo.clone(),
        dashboard_url: Some(format!("http://localhost:{}", cli.port)),
        java_jar: cli
            .java_jar
            .as_ref()
            .map(|p| p.to_string_lossy().to_string()),
        cards_dir: cli.cards_dir.clone(),
        project_root: std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".to_string()),
    };

    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    rt.block_on(async {
        analyzer::run(storage, config, running).await;
    });
}
