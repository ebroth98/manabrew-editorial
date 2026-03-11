#!/usr/bin/env bash
#
# check-harness.sh - Check if the Java harness JAR needs rebuilding
#
# Computes a SHA-256 checksum of all Java source files and pom.xml files
# that affect the harness build, compares against a stored checksum, and
# rebuilds if anything changed (or the JAR doesn't exist).
#
# Usage:
#   ./scripts/check-harness.sh          # check + rebuild if needed
#   ./scripts/check-harness.sh --check  # check only, exit 0=fresh 1=stale
#
# Exit codes (--check mode):
#   0 = JAR is up-to-date
#   1 = JAR is stale or missing
#
# Exit codes (default mode):
#   0 = JAR is up-to-date (no rebuild needed, or rebuild succeeded)
#   1 = rebuild failed

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

JAR_PATH="$PROJECT_ROOT/forge/forge-harness/target/forge-harness-jar-with-dependencies.jar"
CHECKSUM_FILE="$PROJECT_ROOT/forge/forge-harness/target/.harness-sources-checksum"

CHECK_ONLY=false
if [[ "${1:-}" == "--check" ]]; then
    CHECK_ONLY=true
fi

# Directories containing Java sources that affect the harness
SOURCE_DIRS=(
    "$PROJECT_ROOT/forge/forge-core/src"
    "$PROJECT_ROOT/forge/forge-game/src"
    "$PROJECT_ROOT/forge/forge-ai/src"
    "$PROJECT_ROOT/forge/forge-gui/src"
    "$PROJECT_ROOT/forge/forge-harness/src"
)

# POM files that affect the build
POM_FILES=(
    "$PROJECT_ROOT/forge/pom.xml"
    "$PROJECT_ROOT/forge/forge-core/pom.xml"
    "$PROJECT_ROOT/forge/forge-game/pom.xml"
    "$PROJECT_ROOT/forge/forge-ai/pom.xml"
    "$PROJECT_ROOT/forge/forge-gui/pom.xml"
    "$PROJECT_ROOT/forge/forge-harness/pom.xml"
)

compute_checksum() {
    local hash_input=""

    # Hash all Java source files (sorted for determinism)
    for dir in "${SOURCE_DIRS[@]}"; do
        if [[ -d "$dir" ]]; then
            hash_input+="$(find "$dir" -type f -name '*.java' | sort | xargs shasum -a 256 2>/dev/null)"
            hash_input+=$'\n'
        fi
    done

    # Hash all POM files
    for pom in "${POM_FILES[@]}"; do
        if [[ -f "$pom" ]]; then
            hash_input+="$(shasum -a 256 "$pom")"
            hash_input+=$'\n'
        fi
    done

    # Compute a single checksum of all the individual checksums
    echo "$hash_input" | shasum -a 256 | cut -d' ' -f1
}

is_stale() {
    # JAR doesn't exist
    if [[ ! -f "$JAR_PATH" ]]; then
        echo "harness: JAR not found at $JAR_PATH"
        return 0
    fi

    local current_checksum
    current_checksum="$(compute_checksum)"

    # No stored checksum
    if [[ ! -f "$CHECKSUM_FILE" ]]; then
        echo "harness: no stored checksum, assuming stale"
        return 0
    fi

    local stored_checksum
    stored_checksum="$(cat "$CHECKSUM_FILE")"

    if [[ "$current_checksum" != "$stored_checksum" ]]; then
        echo "harness: sources changed (checksum mismatch)"
        return 0
    fi

    return 1
}

rebuild() {
    echo "harness: rebuilding JAR..."
    (cd "$PROJECT_ROOT/forge" && mvn -pl forge-harness -am package -DskipTests -q)
    local exit_code=$?

    if [[ $exit_code -eq 0 ]]; then
        # Store new checksum after successful build
        local checksum
        checksum="$(compute_checksum)"
        mkdir -p "$(dirname "$CHECKSUM_FILE")"
        echo "$checksum" > "$CHECKSUM_FILE"
        echo "harness: rebuild complete"
    else
        echo "harness: rebuild FAILED (exit code $exit_code)"
        return 1
    fi
}

# Main
if is_stale; then
    if $CHECK_ONLY; then
        exit 1
    fi
    rebuild
else
    echo "harness: JAR is up-to-date"
fi
