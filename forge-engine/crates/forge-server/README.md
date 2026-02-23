# Forge Server

WebSocket-based game lobby server for Forge.

## Running with Docker

```bash
# From this directory (forge-engine/crates/forge-server/)

# Start the server
docker compose up -d

# View logs
docker compose logs -f

# Stop the server
docker compose down
```

The server listens on **ws://localhost:9443** by default.

## Configuration

All config is via environment variables. Edit `compose.yml` or pass overrides:

| Variable | Default | Description |
|---|---|---|
| `FORGE_HOST` | `0.0.0.0` | Bind address |
| `FORGE_PORT` | `9443` | Listen port |
| `FORGE_MAX_ROOMS` | `100` | Max concurrent rooms |
| `FORGE_SERVER_KEY` | `forge` | Server authentication key |
| `RUST_LOG` | `forge_server=info` | Log level filter |

```bash
# Example: custom port and key
FORGE_PORT=8080 FORGE_SERVER_KEY=mysecret docker compose up -d
```

## Building without Docker

```bash
cargo build --release -p forge-server
FORGE_PORT=9443 ./target/release/forge-server
```
