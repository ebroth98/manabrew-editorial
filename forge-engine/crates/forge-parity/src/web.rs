//! Axum HTTP server for the continuous parity dashboard.
//!
//! Provides REST API endpoints and an embedded single-page dashboard.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Json},
    routing::{get, post},
    Router,
};

use crate::log_buffer::LogBuffer;
use crate::storage::Storage;

/// Runtime-adjustable configuration, shared between web server, game loop, and analyzer.
pub struct DashboardConfig {
    /// Games per matchup per batch (10, 30, or 40).
    pub games_per_matchup: AtomicUsize,
    /// Whether fuzz deck generation is enabled.
    pub fuzz_enabled: AtomicBool,
    /// Whether to include self-matchups (deck vs itself).
    pub self_matchups: AtomicBool,
    /// Current LLM model name.
    pub llm_model: Mutex<String>,
    /// Whether the analysis daemon is running (shared with analyzer).
    pub analysis_running: Arc<AtomicBool>,
}

impl DashboardConfig {
    /// Create with sensible defaults. Analysis starts paused.
    pub fn new() -> Self {
        Self {
            games_per_matchup: AtomicUsize::new(10),
            fuzz_enabled: AtomicBool::new(false),
            self_matchups: AtomicBool::new(true),
            llm_model: Mutex::new(
                std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o-mini".into()),
            ),
            analysis_running: Arc::new(AtomicBool::new(false)),
        }
    }
}

/// Shared application state for the web server.
pub struct AppState {
    pub storage: Mutex<Storage>,
    pub start_time: std::time::Instant,
    pub start_time_iso: String,
    pub config: Arc<DashboardConfig>,
    pub logs: LogBuffer,
}

#[derive(serde::Deserialize)]
pub struct TrendQuery {
    #[serde(default = "default_bucket")]
    bucket: String,
    #[serde(default = "default_limit")]
    limit: usize,
}

fn default_bucket() -> String {
    "hour".into()
}

fn default_limit() -> usize {
    24
}

#[derive(serde::Deserialize)]
pub struct FailuresQuery {
    #[serde(default = "default_failures_limit")]
    limit: usize,
}

fn default_failures_limit() -> usize {
    50
}

#[derive(serde::Deserialize)]
pub struct LogsQuery {
    /// Return entries with id > since (for incremental polling).
    #[serde(default)]
    since: u64,
    /// Max entries to return (default 200).
    #[serde(default = "default_logs_limit")]
    limit: usize,
}

fn default_logs_limit() -> usize {
    200
}

/// JSON shape for partial config updates.
#[derive(serde::Deserialize)]
struct ConfigUpdate {
    games_per_matchup: Option<usize>,
    fuzz_enabled: Option<bool>,
    self_matchups: Option<bool>,
    llm_model: Option<String>,
}

/// JSON shape returned by GET /api/config.
#[derive(serde::Serialize)]
struct ConfigResponse {
    games_per_matchup: usize,
    fuzz_enabled: bool,
    self_matchups: bool,
    llm_model: String,
    analysis_running: bool,
}

/// JSON shape returned by GET /api/analysis/status.
#[derive(serde::Serialize)]
struct AnalysisStatusResponse {
    running: bool,
    backend: String,
    model: String,
}

/// Build the Axum router with all API routes and the dashboard.
pub fn build_router(state: Arc<AppState>) -> Router {
    let router = Router::new()
        .route("/", get(dashboard_handler))
        .route("/api/stats", get(stats_handler))
        .route("/api/trend", get(trend_handler))
        .route("/api/failures", get(failures_handler))
        .route("/api/matrix", get(matrix_handler))
        .route("/api/run/:id", get(run_handler))
        .route("/api/clusters", get(clusters_handler))
        .route("/api/config", get(config_get_handler).post(config_post_handler))
        .route("/api/analysis/toggle", post(analysis_toggle_handler))
        .route("/api/analysis/status", get(analysis_status_handler))
        .route("/api/logs", get(logs_handler))
        .route("/api/fuzz/recent", get(fuzz_recent_handler));

    #[cfg(feature = "analyze")]
    let router = router.route("/api/models", get(models_handler));

    router.with_state(state)
}

async fn dashboard_handler() -> Html<&'static str> {
    Html(include_str!("dashboard.html"))
}

async fn stats_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let uptime = state.start_time.elapsed().as_secs();
    let storage = state.storage.lock().unwrap();
    match storage.stats(uptime, &state.start_time_iso) {
        Ok(stats) => Json(stats).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn trend_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<TrendQuery>,
) -> impl IntoResponse {
    let storage = state.storage.lock().unwrap();
    match storage.trend(&params.bucket, params.limit) {
        Ok(trend) => Json(trend).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn failures_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<FailuresQuery>,
) -> impl IntoResponse {
    let storage = state.storage.lock().unwrap();
    match storage.recent_failures(params.limit) {
        Ok(failures) => Json(failures).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn matrix_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let storage = state.storage.lock().unwrap();
    match storage.deck_pair_matrix() {
        Ok(matrix) => Json(matrix).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn run_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let storage = state.storage.lock().unwrap();
    match storage.get_run(id) {
        Ok(record) => Json(record).into_response(),
        Err(e) => {
            tracing::error!(id, %e, "Failed to load run");
            (StatusCode::NOT_FOUND, format!("Run {id} not found: {e}")).into_response()
        }
    }
}

async fn clusters_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let storage = state.storage.lock().unwrap();
    match storage.get_clusters_by_field() {
        Ok(clusters) => Json(clusters).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// ── Logs endpoint ─────────────────────────────────────────────────

async fn logs_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<LogsQuery>,
) -> impl IntoResponse {
    let entries = if params.since > 0 {
        state.logs.entries_since(params.since, params.limit)
    } else {
        state.logs.recent(params.limit)
    };
    Json(entries)
}

// ── Fuzz endpoint ─────────────────────────────────────────────────

async fn fuzz_recent_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let storage = state.storage.lock().unwrap();
    match storage.recent_fuzz_games(20) {
        Ok(games) => Json(games).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// ── Config endpoints ──────────────────────────────────────────────

async fn config_get_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let cfg = &state.config;
    let model = cfg.llm_model.lock().unwrap().clone();
    Json(ConfigResponse {
        games_per_matchup: cfg.games_per_matchup.load(Ordering::Relaxed),
        fuzz_enabled: cfg.fuzz_enabled.load(Ordering::Relaxed),
        self_matchups: cfg.self_matchups.load(Ordering::Relaxed),
        llm_model: model,
        analysis_running: cfg.analysis_running.load(Ordering::Relaxed),
    })
}

async fn config_post_handler(
    State(state): State<Arc<AppState>>,
    Json(update): Json<ConfigUpdate>,
) -> impl IntoResponse {
    let cfg = &state.config;
    if let Some(n) = update.games_per_matchup {
        let n = n.clamp(1, 100);
        cfg.games_per_matchup.store(n, Ordering::Relaxed);
    }
    if let Some(v) = update.fuzz_enabled {
        cfg.fuzz_enabled.store(v, Ordering::Relaxed);
    }
    if let Some(v) = update.self_matchups {
        cfg.self_matchups.store(v, Ordering::Relaxed);
    }
    if let Some(model) = update.llm_model {
        if !model.is_empty() {
            *cfg.llm_model.lock().unwrap() = model;
        }
    }

    // Return current config
    let model = cfg.llm_model.lock().unwrap().clone();
    Json(ConfigResponse {
        games_per_matchup: cfg.games_per_matchup.load(Ordering::Relaxed),
        fuzz_enabled: cfg.fuzz_enabled.load(Ordering::Relaxed),
        self_matchups: cfg.self_matchups.load(Ordering::Relaxed),
        llm_model: model,
        analysis_running: cfg.analysis_running.load(Ordering::Relaxed),
    })
}

// ── Analysis control endpoints ────────────────────────────────────

async fn analysis_toggle_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let running = &state.config.analysis_running;
    let was_running = running.load(Ordering::Relaxed);
    running.store(!was_running, Ordering::Relaxed);
    let now_running = !was_running;
    tracing::info!(running = now_running, "Analysis toggled");
    Json(serde_json::json!({ "running": now_running }))
}

async fn analysis_status_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let cfg = &state.config;
    let running = cfg.analysis_running.load(Ordering::Relaxed);
    let model = cfg.llm_model.lock().unwrap().clone();

    // Detect backend from env
    let backend = if std::env::var("ANTHROPIC_API_KEY").map(|k| !k.is_empty()).unwrap_or(false) {
        "anthropic".to_string()
    } else if std::env::var("OPENAI_API_KEY").map(|k| !k.is_empty()).unwrap_or(false) {
        let base = std::env::var("OPENAI_API_BASE").unwrap_or_else(|_| "https://api.openai.com".into());
        format!("openai ({})", base)
    } else {
        "none".to_string()
    };

    Json(AnalysisStatusResponse {
        running,
        backend,
        model,
    })
}

// ── Models proxy (LiteLLM / OpenAI-compatible) ────────────────────

#[cfg(feature = "analyze")]
/// Proxy GET /api/models to the upstream LLM provider's /v1/models endpoint.
///
/// Works with LiteLLM, llama-server, vLLM, or any OpenAI-compatible API.
/// Returns `[{id: "model-name", ...}]` — the dashboard uses this to populate
/// a model selector dropdown.
async fn models_handler() -> impl IntoResponse {
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    let base_url = std::env::var("OPENAI_API_BASE")
        .unwrap_or_else(|_| "https://api.openai.com".into());
    let url = format!("{}/v1/models", base_url.trim_end_matches('/'));

    let client = reqwest::Client::new();
    let mut req = client.get(&url);
    if !api_key.is_empty() {
        req = req.header("Authorization", format!("Bearer {api_key}"));
    }

    match req.send().await {
        Ok(resp) if resp.status().is_success() => {
            match resp.json::<serde_json::Value>().await {
                Ok(json) => {
                    // Extract just the model IDs into a simple array
                    let models: Vec<String> = json["data"]
                        .as_array()
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|m| m["id"].as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default();
                    Json(serde_json::json!({ "models": models })).into_response()
                }
                Err(e) => {
                    (StatusCode::BAD_GATEWAY, format!("Failed to parse models response: {e}"))
                        .into_response()
                }
            }
        }
        Ok(resp) => {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            (StatusCode::BAD_GATEWAY, format!("Upstream {status}: {text}")).into_response()
        }
        Err(e) => {
            (StatusCode::BAD_GATEWAY, format!("Failed to reach LLM provider: {e}")).into_response()
        }
    }
}
