#!/usr/bin/env bash
# deploy.sh — Smart rebuild: only rebuilds what changed since last deploy.
# Triggered automatically via n8n webhook on push to main.
# See DEPLOY.md for setup instructions.
# Docker BuildKit layer caching handles unchanged layers within each build.
#
# stdout  = clean summary (suitable for Discord)
# Raw build output goes to /tmp/deploy-raw.log
set -euo pipefail

REPO_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$REPO_DIR"

COMPOSE_FILE="forge-engine/crates/forge-server/compose.yml"
RAW_LOG="/tmp/deploy-raw.log"
: > "$RAW_LOG"   # truncate

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
fi

# ── Pull latest changes ──────────────────────────────────────────────
PREV=$(git rev-parse --short HEAD)
git pull origin main --ff-only >> "$RAW_LOG" 2>&1
CURR=$(git rev-parse --short HEAD)

if [ "$PREV" = "$CURR" ]; then
    echo "No new commits. Nothing to deploy."
    exit 0
fi

# Gather commit info
COMMIT_COUNT=$(git rev-list "${PREV}..${CURR}" --count)
COMMIT_MSG=$(git log -1 --format="%s" "$CURR")
AUTHOR=$(git log -1 --format="%an" "$CURR")

# ── Determine what changed ───────────────────────────────────────────
CHANGED=$(git diff --name-only "${PREV}..${CURR}")

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
export BUILDKIT_PROGRESS=plain

SERVICES_TO_RESTART=""
BUILD_START=$(date +%s)

# -- parity-dashboard (Java + Rust) --
if $INFRA_CHANGED; then
    echo "Building parity-dashboard (full)..." >> "$RAW_LOG"
    docker compose -f "$COMPOSE_FILE" build --progress=plain --no-cache parity-dashboard >> "$RAW_LOG" 2>&1
    SERVICES_TO_RESTART="parity-dashboard"
elif $JAVA_CHANGED || $RUST_CHANGED; then
    echo "Building parity-dashboard (cached)..." >> "$RAW_LOG"
    docker compose -f "$COMPOSE_FILE" build --progress=plain parity-dashboard >> "$RAW_LOG" 2>&1
    SERVICES_TO_RESTART="parity-dashboard"
fi

# -- forge-server (Rust only) --
if $INFRA_CHANGED; then
    echo "Building forge-server (full)..." >> "$RAW_LOG"
    docker compose -f "$COMPOSE_FILE" build --progress=plain --no-cache forge-server >> "$RAW_LOG" 2>&1
    SERVICES_TO_RESTART="$SERVICES_TO_RESTART forge-server"
elif $RUST_CHANGED; then
    echo "Building forge-server (cached)..." >> "$RAW_LOG"
    docker compose -f "$COMPOSE_FILE" build --progress=plain forge-server >> "$RAW_LOG" 2>&1
    SERVICES_TO_RESTART="$SERVICES_TO_RESTART forge-server"
fi

if [ -z "$SERVICES_TO_RESTART" ]; then
    echo "No Java/Rust/infra changes — skipping build."
    exit 0
fi

docker compose -f "$COMPOSE_FILE" up -d $SERVICES_TO_RESTART >> "$RAW_LOG" 2>&1

BUILD_END=$(date +%s)
BUILD_DURATION=$(( BUILD_END - BUILD_START ))

# ── Pretty summary for Discord ───────────────────────────────────────
SERVICES_FMT=$(echo "$SERVICES_TO_RESTART" | xargs -n1 | sed 's/^/  - /' | tr '\n' '\n')

# Build change flags string
CHANGES=""
$JAVA_CHANGED && CHANGES="${CHANGES} Java"
$RUST_CHANGED && CHANGES="${CHANGES} Rust"
$INFRA_CHANGED && CHANGES="${CHANGES} Infra"
CHANGES=$(echo "$CHANGES" | xargs)

cat <<EOF
**Deploy complete** (\`${PREV}\` -> \`${CURR}\`)

> ${COMMIT_MSG}
> — ${AUTHOR} (${COMMIT_COUNT} commit(s))

**Changed:** ${CHANGES}
**Services rebuilt:**
${SERVICES_FMT}
**Build time:** ${BUILD_DURATION}s
**Log:** \`${RAW_LOG}\`
EOF
