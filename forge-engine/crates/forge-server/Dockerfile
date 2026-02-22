FROM rust:1.83-slim AS builder

WORKDIR /build

# Create a minimal workspace so `workspace = true` deps resolve
COPY Cargo.toml ./crates/forge-server/Cargo.toml

RUN mkdir -p crates/forge-server/src && \
    cat <<'EOF' > Cargo.toml
[workspace]
resolver = "2"
members = ["crates/forge-server"]

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
EOF

# Dummy main for dependency caching
RUN echo 'fn main() {}' > crates/forge-server/src/main.rs && \
    cargo build --release -p forge-server && \
    rm -rf crates/forge-server/src

# Copy real source and rebuild
COPY src/ ./crates/forge-server/src/
RUN cargo build --release -p forge-server

# --- Runtime ---
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/forge-server /usr/local/bin/forge-server

ENV FORGE_HOST=0.0.0.0
ENV FORGE_PORT=9443

EXPOSE 9443

ENTRYPOINT ["forge-server"]
