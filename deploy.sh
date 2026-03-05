#!/usr/bin/env bash
# deploy.sh — Smart rebuild: only rebuilds what changed since last deploy.
# Triggered automatically via n8n webhook on push to main.
# Docker BuildKit layer caching handles unchanged layers within each build.
set -euo pipefail

REPO_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$REPO_DIR"

COMPOSE_FILE="forge-engine/crates/forge-server/compose.yml"
LOG_PREFIX="[deploy]"

log() { echo "$LOG_PREFIX $*"; }

# ── Load .env (for GITHUB_TOKEN) ────────────────────────────────────
if [ -f "$REPO_DIR/.env" ]; then
    set -a
    # shellcheck disable=SC1091
    source "$REPO_DIR/.env"
    set +a
fi

# ── Configure git to use PAT instead of SSH ─────────────────────────
if [ -n "${GITHUB_TOKEN:-}" ]; then
    git remote set-url origin "https://x-access-token:${GITHUB_TOKEN}@github.com/fedepoi/bardidinaXmageUI.git"
    log "Using GITHUB_TOKEN for git pull"
fi

# ── Pull latest changes ──────────────────────────────────────────────
PREV=$(git rev-parse HEAD)
git pull origin main --ff-only
CURR=$(git rev-parse HEAD)

if [ "$PREV" = "$CURR" ]; then
    log "No new commits. Nothing to deploy."
    exit 0
fi

log "Updating $PREV → $CURR"

# ── Determine what changed ───────────────────────────────────────────
CHANGED=$(git diff --name-only "$PREV" "$CURR")

JAVA_CHANGED=false
RUST_CHANGED=false
INFRA_CHANGED=false

while IFS= read -r file; do
    case "$file" in
        forge/forge-core/*|forge/forge-game/*|forge/forge-ai/*|forge/forge-gui/*|forge/forge-harness/*|forge/pom.xml)
            JAVA_CHANGED=true ;;
        forge-engine/*|Cargo.toml|Cargo.lock)
            RUST_CHANGED=true ;;
        *Dockerfile*|*compose*|.dockerignore|deploy.sh)
            INFRA_CHANGED=true ;;
    esac
done <<< "$CHANGED"

# ── Build & deploy ───────────────────────────────────────────────────
export DOCKER_BUILDKIT=1

SERVICES_TO_RESTART=""

# -- parity-dashboard (Java + Rust) --
if $INFRA_CHANGED; then
    log "Infrastructure changed — full rebuild of parity-dashboard"
    docker compose -f "$COMPOSE_FILE" build --no-cache parity-dashboard
    SERVICES_TO_RESTART="parity-dashboard"
elif $JAVA_CHANGED || $RUST_CHANGED; then
    log "Source changed (java=$JAVA_CHANGED rust=$RUST_CHANGED) — rebuilding parity-dashboard"
    docker compose -f "$COMPOSE_FILE" build parity-dashboard
    SERVICES_TO_RESTART="parity-dashboard"
fi

# -- forge-server (Rust only) --
if $INFRA_CHANGED; then
    log "Infrastructure changed — full rebuild of forge-server"
    docker compose -f "$COMPOSE_FILE" build --no-cache forge-server
    SERVICES_TO_RESTART="$SERVICES_TO_RESTART forge-server"
elif $RUST_CHANGED; then
    log "Rust changed — rebuilding forge-server"
    docker compose -f "$COMPOSE_FILE" build forge-server
    SERVICES_TO_RESTART="$SERVICES_TO_RESTART forge-server"
fi

if [ -z "$SERVICES_TO_RESTART" ]; then
    log "No Java/Rust/infra changes — skipping build"
    exit 0
fi

log "Restarting: $SERVICES_TO_RESTART"
docker compose -f "$COMPOSE_FILE" up -d $SERVICES_TO_RESTART

log "Deploy complete. Container status:"
docker compose -f "$COMPOSE_FILE" ps
