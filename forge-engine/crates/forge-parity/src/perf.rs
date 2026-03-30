use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

#[derive(Clone, Copy, Default)]
struct Stat {
    count: u64,
    total: Duration,
    max: Duration,
}

#[derive(Clone, Copy)]
struct Config {
    enabled: bool,
    min_ms: u128,
}

fn parse_bool_env(name: &str) -> bool {
    matches!(
        std::env::var(name)
            .ok()
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn config() -> Config {
    static CFG: OnceLock<Config> = OnceLock::new();
    *CFG.get_or_init(|| {
        let enabled = parse_bool_env("FORGE_PARITY_PROFILE");
        let min_ms = std::env::var("FORGE_PARITY_PROFILE_MIN_MS")
            .ok()
            .and_then(|s| s.parse::<u128>().ok())
            .unwrap_or(0);
        Config { enabled, min_ms }
    })
}

fn stats() -> &'static Mutex<HashMap<String, Stat>> {
    static STATS: OnceLock<Mutex<HashMap<String, Stat>>> = OnceLock::new();
    STATS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn enabled() -> bool {
    config().enabled
}

pub fn record(label: &str, elapsed: Duration) {
    let cfg = config();
    if !cfg.enabled {
        return;
    }
    let mut guard = stats().lock().unwrap();
    let entry = guard.entry(label.to_string()).or_default();
    entry.count += 1;
    entry.total += elapsed;
    if elapsed > entry.max {
        entry.max = elapsed;
    }
}

pub fn print_summary(context: &str) {
    if !enabled() {
        return;
    }
    let guard = stats().lock().unwrap();
    if guard.is_empty() {
        eprintln!("[parity-prof] {}: no samples", context);
        return;
    }

    let mut rows: Vec<(String, Stat)> = guard.iter().map(|(k, v)| (k.clone(), *v)).collect();
    let min_ms = config().min_ms;
    rows.retain(|(_, stat)| stat.max.as_millis() >= min_ms);
    if rows.is_empty() {
        eprintln!(
            "[parity-prof] {}: no samples above {}ms max",
            context, min_ms
        );
        return;
    }
    rows.sort_by(|a, b| b.1.total.cmp(&a.1.total));

    eprintln!("[parity-prof] {}: timing summary (sorted by total)", context);
    for (label, stat) in rows {
        let total_ms = stat.total.as_secs_f64() * 1000.0;
        let max_ms = stat.max.as_secs_f64() * 1000.0;
        let avg_ms = if stat.count == 0 {
            0.0
        } else {
            total_ms / stat.count as f64
        };
        eprintln!(
            "[parity-prof] {:44} count={:<6} total={:>10.3}ms avg={:>8.3}ms max={:>8.3}ms",
            label, stat.count, total_ms, avg_ms, max_ms
        );
    }
}
