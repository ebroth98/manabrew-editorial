# ── Stage 1: Build Java harness JAR ─────────────────────────────────
FROM maven:3.9-eclipse-temurin-17 AS java-builder

WORKDIR /build
COPY forge/ forge/
RUN cd forge && mvn -pl forge-harness -am -DskipTests package -q

# ── Stage 2: Build Rust binary ──────────────────────────────────────
FROM rust:1.88-bookworm AS rust-builder

WORKDIR /build

# Copy workspace manifests for dependency caching
COPY Cargo.toml Cargo.lock ./
COPY forge-engine/ forge-engine/
COPY src-tauri/Cargo.toml src-tauri/Cargo.toml
RUN mkdir -p src-tauri/src && echo "fn main() {}" > src-tauri/src/main.rs && touch src-tauri/src/lib.rs

RUN cargo build -p forge-parity --features analyze --release

# ── Stage 3: Runtime ────────────────────────────────────────────────
FROM eclipse-temurin:17-jre-jammy

RUN apt-get update && apt-get install -y --no-install-recommends sqlite3 curl \
    && curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg \
       | dd of=/usr/share/keyrings/githubcli-archive-keyring.gpg \
    && echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" \
       > /etc/apt/sources.list.d/github-cli.list \
    && apt-get update && apt-get install -y --no-install-recommends gh \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=rust-builder /build/target/release/forge-parity /app/forge-parity
COPY --from=java-builder /build/forge/forge-harness/target/forge-harness-jar-with-dependencies.jar /app/forge-harness.jar
COPY forge/forge-gui/res/ /app/forge-gui/res/
COPY preset_decks/ /app/preset_decks/

VOLUME /app/data

EXPOSE 8080

ENTRYPOINT ["/bin/sh", "-c", "\
    ARGS=\"--serve --port ${PORT:-8080} \
    --db-path /app/data/parity.db \
    --java-jar /app/forge-harness.jar \
    --cards-dir /app/forge-gui/res/cardsfolder \
    --decks-dir /app/preset_decks \
    --threshold ${THRESHOLD:-0.90} \
    --fuzz-per-batch ${FUZZ_PER_BATCH:-0} \
    --max-turns ${MAX_TURNS:-10}\"; \
    if [ -n \"${MAX_GAMES}\" ]; then ARGS=\"${ARGS} --max-games ${MAX_GAMES}\"; fi; \
    if [ -n \"${DECKS}\" ]; then ARGS=\"${ARGS} --decks ${DECKS}\"; fi; \
    if [ -n \"${ANALYZE}\" ]; then ARGS=\"${ARGS} --analyze\"; fi; \
    if [ -n \"${POLL_INTERVAL}\" ]; then ARGS=\"${ARGS} --poll-interval ${POLL_INTERVAL}\"; fi; \
    if [ -n \"${SUMMARY_INTERVAL}\" ]; then ARGS=\"${ARGS} --summary-interval ${SUMMARY_INTERVAL}\"; fi; \
    if [ -n \"${ISSUE_THRESHOLD}\" ]; then ARGS=\"${ARGS} --issue-threshold ${ISSUE_THRESHOLD}\"; fi; \
    if [ -n \"${GITHUB_REPO}\" ]; then ARGS=\"${ARGS} --github-repo ${GITHUB_REPO}\"; fi; \
    if [ -n \"${EXTRA_ARGS}\" ]; then ARGS=\"${ARGS} ${EXTRA_ARGS}\"; fi; \
    exec /app/forge-parity ${ARGS}"]
