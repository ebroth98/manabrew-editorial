# syntax=docker/dockerfile:1
# ── Stage 1: Build Java harness JAR ─────────────────────────────────
FROM maven:3.9-eclipse-temurin-17 AS java-builder

# Cap JVM heap so Maven doesn't get OOM-killed when building alongside Rust
ENV MAVEN_OPTS="-Xmx512m"

WORKDIR /build/forge

# Copy only POMs first so dependency download is cached separately.
# Modules needed by `mvn -pl forge-harness -am`: parent + core, game, ai, gui, harness.
COPY forge/pom.xml                   pom.xml
COPY forge/checkstyle.xml            checkstyle.xml

# Strip module declarations for modules excluded by .dockerignore
# (Maven validates all <module> entries exist before -pl filtering)
RUN sed -i '/<module>forge-gui-mobile<\/module>/d; \
            /<module>forge-gui-mobile-dev<\/module>/d; \
            /<module>forge-gui-desktop<\/module>/d; \
            /<module>forge-gui-ios<\/module>/d; \
            /<module>forge-lda<\/module>/d; \
            /<module>adventure-editor<\/module>/d; \
            /<module>forge-gui-android<\/module>/d; \
            /<module>forge-installer<\/module>/d' pom.xml

COPY forge/forge-core/pom.xml        forge-core/pom.xml
COPY forge/forge-game/pom.xml        forge-game/pom.xml
COPY forge/forge-ai/pom.xml          forge-ai/pom.xml
COPY forge/forge-gui/pom.xml         forge-gui/pom.xml
COPY forge/forge-harness/pom.xml     forge-harness/pom.xml

# Create stub source dirs so Maven resolves modules without error
RUN for m in forge-core forge-game forge-ai forge-gui forge-harness; do \
      mkdir -p "$m/src/main/java"; \
    done

# Download all dependencies (cached until a pom.xml changes)
RUN --mount=type=cache,target=/root/.m2/repository \
    mvn -pl forge-harness -am dependency:go-offline -Dcheckstyle.skip -q || true

# Now copy the real source code
COPY forge/forge-core/    forge-core/
COPY forge/forge-game/    forge-game/
COPY forge/forge-ai/      forge-ai/
COPY forge/forge-gui/     forge-gui/
COPY forge/forge-harness/ forge-harness/

# Build the fat JAR (deps already cached in .m2)
RUN --mount=type=cache,target=/root/.m2/repository \
    mvn -pl forge-harness -am -DskipTests -Dcheckstyle.skip package -q

# ── Stage 2: Build Rust binary ──────────────────────────────────────
FROM rust:1.88-bookworm AS rust-builder

WORKDIR /build

# Copy workspace manifests for dependency caching
COPY Cargo.toml Cargo.lock ./
COPY forge-engine/ forge-engine/
COPY src-tauri/Cargo.toml src-tauri/Cargo.toml
RUN mkdir -p src-tauri/src && echo "fn main() {}" > src-tauri/src/main.rs && touch src-tauri/src/lib.rs

# Build with BuildKit cache mounts for cargo registry + target dir
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/build/target \
    cargo build -p forge-parity --features analyze --release \
    && cp /build/target/release/forge-parity /usr/local/bin/forge-parity

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

COPY --from=rust-builder /usr/local/bin/forge-parity /app/forge-parity
COPY --from=java-builder /build/forge/forge-harness/target/forge-harness-jar-with-dependencies.jar /app/forge-harness.jar
COPY forge/forge-gui/res/ /app/forge-gui/res/
COPY preset_decks/ /app/preset_decks/

# Download MTG Comprehensive Rules for agent analysis
RUN curl -fsSL -o /app/mtg_rules.txt "https://media.wizards.com/2026/downloads/MagicCompRules%2020260227.txt" || true

VOLUME /app/data

EXPOSE 8080

ENTRYPOINT ["/bin/sh", "-c", "\
    ARGS=\"--serve --port ${PORT:-8080} \
    --db-path /app/data/parity.db \
    --java-jar /app/forge-harness.jar \
    --cards-dir /app/forge-gui/res/cardsfolder \
    --decks-dir /app/preset_decks \
    --java-heap ${JAVA_HEAP:-512m} \
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
