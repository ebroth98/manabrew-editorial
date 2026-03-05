//! Axum HTTP server for the continuous parity dashboard.
//!
//! Provides REST API endpoints and an embedded single-page dashboard.

use std::sync::{Arc, Mutex};

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Json},
    routing::get,
    Router,
};

use crate::storage::Storage;

/// Shared application state for the web server.
pub struct AppState {
    pub storage: Mutex<Storage>,
    pub start_time: std::time::Instant,
    pub start_time_iso: String,
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

/// Build the Axum router with all API routes and the dashboard.
pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(dashboard_handler))
        .route("/api/stats", get(stats_handler))
        .route("/api/trend", get(trend_handler))
        .route("/api/failures", get(failures_handler))
        .route("/api/matrix", get(matrix_handler))
        .route("/api/run/{id}", get(run_handler))
        .with_state(state)
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
        Err(e) => (StatusCode::NOT_FOUND, e.to_string()).into_response(),
    }
}
