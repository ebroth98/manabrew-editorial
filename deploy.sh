#!/usr/bin/env bash
# deploy.sh — Smart rebuild: only rebuilds what changed since last deploy.
# Docker BuildKit layer caching handles unchanged layers within each build.
set -euo pipefail

REPO_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$REPO_DIR"

COMPOSE_FILE="forge-engine/crates/forge-server/compose.yml"
SERVICE="parity-dashboard"
LOG_PREFIX="[deploy]"

log() { echo "$LOG_PREFIX $*"; }

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

if $INFRA_CHANGED; then
    log "Infrastructure changed — full rebuild"
    docker compose -f "$COMPOSE_FILE" build --no-cache "$SERVICE"
elif $JAVA_CHANGED || $RUST_CHANGED; then
    log "Source changed (java=$JAVA_CHANGED rust=$RUST_CHANGED) — rebuilding with cache"
    docker compose -f "$COMPOSE_FILE" build "$SERVICE"
else
    log "No Java/Rust/infra changes — skipping build"
fi

log "Restarting service..."
docker compose -f "$COMPOSE_FILE" up -d "$SERVICE"

log "Deploy complete. Container status:"
docker compose -f "$COMPOSE_FILE" ps "$SERVICE"
