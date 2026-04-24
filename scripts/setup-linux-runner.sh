#!/usr/bin/env bash
# Setup script for a Linux self-hosted GitHub Actions runner that builds
# the WASM + card data, runs the TypeScript checker, and compiles the
# Rust workspace for this repo.
#
# Run as a user with sudo (not as root directly):
#   $ chmod +x scripts/setup-linux-runner.sh
#   $ ./scripts/setup-linux-runner.sh
#
# Installs: apt core deps (build-essential, pkg-config, libssl-dev, git,
# curl, jq, ca-certificates), Node.js LTS (via NodeSource), Yarn (via
# corepack), Rust (rustup, stable MSVC-equivalent = gnu on Linux),
# wasm32-unknown-unknown target, and wasm-pack.
#
# Idempotent: each step skips work already done. Safe to re-run.
#
# Target distros: Debian/Ubuntu family (apt-get). Other families will
# abort early with a clear message.

set -euo pipefail

# ---------- helpers -------------------------------------------------------

section() {
    local msg="$1"
    local bar
    bar=$(printf '%.0s-' $(seq 1 $((70 - ${#msg}))))
    printf '\n\033[1;36m== %s %s\033[0m\n' "$msg" "$bar"
}

has_cmd() { command -v "$1" >/dev/null 2>&1; }

# Re-exec under sudo if the invoking user is not root. Keeps HOME so that
# rustup writes to the user's profile rather than /root.
require_root() {
    if [[ $EUID -ne 0 ]]; then
        if ! has_cmd sudo; then
            echo "This script needs root privileges and 'sudo' is not installed." >&2
            exit 1
        fi
        # Preserve the invoking user so Rust / Node user-level installs
        # land in the runner account's home, not root's.
        export RUNNER_USER="${RUNNER_USER:-$USER}"
        export RUNNER_HOME="${RUNNER_HOME:-$HOME}"
        exec sudo -E RUNNER_USER="$RUNNER_USER" RUNNER_HOME="$RUNNER_HOME" bash "$0" "$@"
    fi
    : "${RUNNER_USER:=${SUDO_USER:-root}}"
    : "${RUNNER_HOME:=$(getent passwd "$RUNNER_USER" | cut -d: -f6)}"
    if [[ -z "$RUNNER_HOME" ]]; then
        echo "Could not resolve HOME for user '$RUNNER_USER'." >&2
        exit 1
    fi
}

# Run a command as the runner user (not root). Used for rustup / cargo
# installs that must write to $RUNNER_HOME.
as_runner() {
    sudo -u "$RUNNER_USER" -H bash -lc "$*"
}

# ---------- 1. sanity checks ---------------------------------------------

require_root "$@"

section "Distro detection"
if [[ ! -f /etc/os-release ]]; then
    echo "Cannot detect distro: /etc/os-release missing." >&2
    exit 1
fi
# shellcheck disable=SC1091
. /etc/os-release
echo "Detected: $PRETTY_NAME (ID=$ID, ID_LIKE=${ID_LIKE:-n/a})"
case "${ID,,} ${ID_LIKE:-}" in
    *debian*|*ubuntu*) : ;;
    *) echo "This script supports Debian/Ubuntu only. Port the apt-get lines for $ID." >&2; exit 1 ;;
esac
echo "Runner user: $RUNNER_USER (HOME=$RUNNER_HOME)"

# ---------- 2. apt core packages -----------------------------------------

section "apt packages (build tools, TLS, jq, git)"
export DEBIAN_FRONTEND=noninteractive
apt-get update -y
# build-essential  : gcc/g++/make — cargo linker + native node modules
# pkg-config       : cargo build scripts (openssl-sys, etc.)
# libssl-dev       : openssl headers for cargo crates
# ca-certificates  : TLS trust for curl / rustup / cargo
# curl, wget, git  : checkout + installers
# jq               : release workflow payload assembly
# unzip, xz-utils  : occasional tarball helpers
# python3          : some build scripts (node-gyp fallback)
apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    libssl-dev \
    ca-certificates \
    curl \
    wget \
    git \
    jq \
    unzip \
    xz-utils \
    python3

# ---------- 3. Node.js LTS via NodeSource ---------------------------------

section "Node.js LTS"
NODE_MAJOR_TARGET=20
if has_cmd node; then
    node_ver=$(node --version | sed 's/^v//')
    node_major=${node_ver%%.*}
    echo "node already installed: v$node_ver"
    if (( node_major < NODE_MAJOR_TARGET )); then
        echo "  upgrading from v$node_ver to Node.js $NODE_MAJOR_TARGET.x (LTS)."
        curl -fsSL "https://deb.nodesource.com/setup_${NODE_MAJOR_TARGET}.x" | bash -
        apt-get install -y nodejs
    fi
else
    curl -fsSL "https://deb.nodesource.com/setup_${NODE_MAJOR_TARGET}.x" | bash -
    apt-get install -y nodejs
fi

# ---------- 4. Yarn via corepack -----------------------------------------

# Yarn is enabled through corepack (ships with Node >=16) rather than
# the legacy `yarn` apt package, which is unmaintained on some mirrors.
section "Yarn (via corepack)"
corepack enable
# Pin Classic (1.x) — matches the `yarn.lock` format the workflow expects.
corepack prepare yarn@1.22.22 --activate
yarn --version

# ---------- 5. Rust (rustup) for the runner user --------------------------

section "Rust toolchain (stable, gnu)"
CARGO_BIN="$RUNNER_HOME/.cargo/bin"
if as_runner "command -v rustc >/dev/null 2>&1"; then
    echo "rustc already installed: $(as_runner 'rustc --version')"
    as_runner "rustup update stable"
else
    echo "Installing rustup for $RUNNER_USER..."
    as_runner "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable"
fi

# ---------- 5b. wasm32 target --------------------------------------------

section "wasm32-unknown-unknown target"
if as_runner "$CARGO_BIN/rustup target list --installed" | grep -q '^wasm32-unknown-unknown$'; then
    echo "wasm32-unknown-unknown already installed."
else
    as_runner "$CARGO_BIN/rustup target add wasm32-unknown-unknown"
fi

# ---------- 6. wasm-pack --------------------------------------------------

section "wasm-pack"
if as_runner "command -v wasm-pack >/dev/null 2>&1" || [[ -x "$CARGO_BIN/wasm-pack" ]]; then
    echo "wasm-pack already installed: $(as_runner "$CARGO_BIN/wasm-pack --version" 2>/dev/null || echo unknown)"
else
    # Prefer the prebuilt installer (fast, no compile) and fall back to
    # `cargo install` if the upstream script ever misses this arch.
    if ! as_runner "curl -fsSL https://rustwasm.github.io/wasm-pack/installer/init.sh | sh"; then
        echo "Prebuilt installer failed — falling back to cargo install."
        as_runner "$CARGO_BIN/cargo install wasm-pack --locked"
    fi
fi

# ---------- 7. Ensure cargo is on PATH for the runner shell ---------------

# The GitHub Actions runner executes steps via bash -l, which sources
# ~/.profile / ~/.bashrc. rustup appends its PATH export to ~/.profile,
# but older ~/.bashrc files may not source it. Add an idempotent guard.
section "Shell PATH (runner login shells)"
for rc in "$RUNNER_HOME/.bashrc" "$RUNNER_HOME/.profile"; do
    if [[ -f "$rc" ]] && ! grep -q '.cargo/env' "$rc"; then
        echo '[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"' >> "$rc"
        chown "$RUNNER_USER:$RUNNER_USER" "$rc" 2>/dev/null || true
        echo "Appended cargo env to $rc"
    fi
done

# ---------- 8. GitHub Actions runner service -----------------------------

# On Linux the runner is typically installed per-user under
# ~/actions-runner and registered as a systemd service via svc.sh. If
# that service is active, restart it so it picks up the new PATH entries
# (cargo, wasm-pack) from the updated shell profile.
section "GitHub Actions runner service"
runner_units=$(systemctl list-units --type=service --all --no-legend 2>/dev/null \
    | awk '{print $1}' \
    | grep -E '^actions\.runner\..*\.service$' || true)

if [[ -z "$runner_units" ]]; then
    echo "No 'actions.runner.*.service' systemd unit found."
    echo "If you installed the runner under ~/actions-runner, register the"
    echo "service with:  cd ~/actions-runner && sudo ./svc.sh install && sudo ./svc.sh start"
else
    while IFS= read -r unit; do
        echo "Restarting $unit to pick up new PATH..."
        systemctl restart "$unit"
    done <<< "$runner_units"
    sleep 2
    systemctl --no-pager --no-legend status $runner_units | head -n 20 || true
fi

# ---------- 9. Sanity check ----------------------------------------------

section "Versions"
try_version() {
    local cmd="$1"
    local arg="${2:---version}"
    if as_runner "command -v $cmd >/dev/null 2>&1"; then
        printf '%-12s %s\n' "$cmd" "$(as_runner "$cmd $arg 2>&1 | head -n1")"
    else
        printf '%-12s NOT FOUND\n' "$cmd"
    fi
}

try_version git
try_version node
try_version npm
try_version yarn
try_version rustc
try_version cargo
try_version wasm-pack
try_version jq
try_version pkg-config

if as_runner "$CARGO_BIN/rustup target list --installed" | grep -q '^wasm32-unknown-unknown$'; then
    echo "wasm32       installed"
else
    echo "wasm32       NOT INSTALLED"
fi

section "Next steps"
cat <<EOF
1. Confirm the runner service is online in GitHub: Settings -> Actions ->
   Runners. It should show 'Idle'.
2. Trigger a quick smoke test by re-running any job from
   .github/workflows/build-checks.yml (TypeScript, WASM build, Rust engine).
3. If the runner still reports 'cargo: command not found', the runner
   service environment did not inherit the shell profile. Either:
     a) Edit ~/actions-runner/.env to append
          PATH=$RUNNER_HOME/.cargo/bin:\$PATH
        then restart the service, or
     b) Move cargo/rustc onto the system PATH by symlinking into
        /usr/local/bin:
          sudo ln -sf $CARGO_BIN/{cargo,rustc,rustup,wasm-pack} /usr/local/bin/
4. This script does NOT install Tauri bundling deps (webkit2gtk, libsoup,
   libayatana-appindicator, etc.) because the Linux workflows in this repo
   only run checks + WASM + release publish. Add them here if you later
   start producing Linux .AppImage/.deb artifacts on this runner.
EOF
