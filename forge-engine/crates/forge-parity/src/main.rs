//! CLI entry point for `forge-parity`.
//!
//! ```text
//! forge-parity --deck1 <name> --deck2 <name> [--seed N] [--max-turns N]
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

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use clap::Parser;
use rayon::prelude::*;

use forge_parity::card_pool::CardPool;
use forge_parity::comparator;
use forge_parity::deck_generator;
use forge_parity::java_bridge::{JavaBridge, JavaBridgeConfig, JavaBridgeError, JavaServer, JavaServerConfig};
use forge_parity::java_random::JavaRandom;
use forge_parity::protocol::{
    Divergence, FuzzReport, FuzzResult, MatchupResult, MatchupStatus, MatrixReport,
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

    if cli.fuzz {
        run_fuzz_mode(&cli);
    } else if cli.matrix {
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

    // Spawn a Java server for the single matchup
    let server_config = JavaServerConfig {
        jar_path: jar_path.clone(),
        forge_home: None,
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
            };
        }
    };

    // Run Java engine via server
    let java_snapshots =
        match server.run_matchup(&config.deck1, &config.deck2, config.seed, config.max_turns) {
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
                };
            }
        };

    compare_snapshots(config, &rust_trace.snapshots, &java_snapshots)
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

    compare_snapshots(config, &rust_trace.snapshots, &java_snapshots)
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
    let mut all_divergences: Vec<Divergence> = Vec::new();

    for i in 0..max_snapshots {
        match (rust_snapshots.get(i), java_snapshots.get(i)) {
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

/// A pool of JavaServer instances behind mutexes for parallel access.
struct ServerPool {
    servers: Vec<Mutex<JavaServer>>,
}

impl ServerPool {
    /// Spawn N server instances.
    fn spawn(n: usize, config: &JavaServerConfig) -> Result<Self, JavaBridgeError> {
        let mut servers = Vec::with_capacity(n);
        for i in 0..n {
            eprintln!("[parity] Spawning Java worker {}/{}", i + 1, n);
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
    ) -> Result<Vec<forge_parity::protocol::StateSnapshot>, JavaBridgeError> {
        // Try each server — grab whichever lock is available
        for server_mutex in &self.servers {
            if let Ok(mut server) = server_mutex.try_lock() {
                if !server.is_alive() {
                    continue;
                }
                return server.run_matchup(deck1, deck2, seed, max_turns);
            }
        }
        // All busy on try_lock — block on the first one
        let mut server = self.servers[0]
            .lock()
            .map_err(|e| JavaBridgeError::ProtocolError(format!("Mutex poisoned: {}", e)))?;
        server.run_matchup(deck1, deck2, seed, max_turns)
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

    // Spawn server pool if Java JAR is provided
    let num_workers = cli
        .java_workers
        .unwrap_or_else(|| if cli.java_jar.is_some() { num_cpus() } else { 0 });

    let pool = if let Some(ref jar_path) = cli.java_jar {
        let server_config = JavaServerConfig {
            jar_path: jar_path.clone(),
            forge_home: None,
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
            };

            let result = if let Some(ref pool) = pool {
                run_single_matchup_with_pool(&config, &data, pool)
            } else if let Some(ref jar_path) = cli.java_jar {
                run_single_matchup_oneshot(&config, &data, jar_path)
            } else {
                run_single_matchup_rust_only(&config, &data)
            };

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
            };
        }
    };

    // Run Java via pool
    let java_snapshots =
        match pool.run_matchup(&config.deck1, &config.deck2, config.seed, config.max_turns) {
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
                };
            }
        };

    compare_snapshots(config, &rust_trace.snapshots, &java_snapshots)
}

fn run_fuzz_mode(cli: &Cli) {
    eprintln!(
        "[parity] Fuzz mode: {} iterations, master_seed={}",
        cli.iterations, cli.master_seed
    );

    // Load data once
    let data = match runner::load_data(cli.cards_dir.as_deref()) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[parity] Load error: {}", e);
            std::process::exit(1);
        }
    };

    // Discover card pool
    let (pool, pool_stats) = CardPool::discover(&data.db);
    eprintln!("[parity] {}", pool_stats);

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
        };

        let matchup_result = if let Some(ref mut srv) = server {
            if srv.is_alive() {
                run_single_matchup_server(&config, &data, srv)
            } else {
                // Server crashed — try to respawn
                eprintln!("[parity] Java server crashed, attempting respawn...");
                match JavaServer::spawn(&JavaServerConfig {
                    jar_path: cli.java_jar.as_ref().unwrap().clone(),
                    forge_home: None,
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

/// Get the number of available CPUs (capped at a reasonable number).
fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .min(8)
}
