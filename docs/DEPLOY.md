# Deployment Guide

## Internal Web Deployment (Twingate + SSO)

For the browser/WASM client, the critical requirement is not public internet exposure, it is preserving cross-origin isolation through every proxy layer. The web game worker uses `SharedArrayBuffer`, so the final browser response must include:

```http
Cross-Origin-Opener-Policy: same-origin
Cross-Origin-Embedder-Policy: credentialless
```

If your Twingate or SSO layer strips or overrides those headers, browser gameplay will fail even if the app shell loads.

### Internal alpha checklist

- Build and publish with `npm run build:web`
- Serve `dist/` as a static site
- Ensure the final HTML, JS worker, and WASM responses preserve `COOP` and `COEP`
- Verify the app is loaded from a single origin
- Confirm `/wasm/cards-bundle.json` and `/wasm/preset-decks.json` are reachable through the same internal path

### Verify in browser

Open DevTools on the deployed site and check:

```js
window.crossOriginIsolated
typeof SharedArrayBuffer !== "undefined"
```

Expected result:

- `window.crossOriginIsolated === true`
- `SharedArrayBuffer` is available

The app now also emits a toast and console error when this is misconfigured.

### Verify at the edge

Check the final response headers after Twingate/SSO, not just the origin server:

```bash
curl -I https://<internal-host>/
curl -I https://<internal-host>/assets/game-engine.worker-<hash>.js
curl -I https://<internal-host>/assets/forge_wasm_bg-<hash>.wasm
```

You should see:

```http
Cross-Origin-Opener-Policy: same-origin
Cross-Origin-Embedder-Policy: credentialless
```

### Nginx example

```nginx
location / {
  add_header Cross-Origin-Opener-Policy same-origin always;
  add_header Cross-Origin-Embedder-Policy credentialless always;
  try_files $uri /index.html;
}
```

### Caddy example

```caddy
header {
  Cross-Origin-Opener-Policy same-origin
  Cross-Origin-Embedder-Policy credentialless
}
```

### Current internal-scope caveats

- The browser card bundle currently covers preset-deck cards, not the full Forge card pool
- The generated bundle still reports two missing preset scripts: `Thrum of the Vestige` and `Leonardo, Big Brother`
- The web path is not offline-capable today
- Scryfall metadata/images are still fetched remotely from the browser
## Prerequisites

- Docker + Docker Compose (with BuildKit support)
- Git
- n8n instance (for auto-deploy webhook)

## Initial Server Setup

### 1. Clone the repo

```bash
cd ~
git clone git@github.com:fedepoi/bardidinaXmageUI.git
cd bardidinaXmageUI
```

### 2. Create a `.env` file

```bash
cp forge-engine/crates/forge-server/.env.example .env  # or create manually
```

Add your keys:

```env
ANALYZE=1
ANTHROPIC_API_KEY=sk-ant-...
# Or use local LLM:
# OPENAI_API_BASE=http://localhost:8190/v1
# OPENAI_MODEL=qwen3-14b
# OPENAI_API_KEY=not-needed
DISCORD_WEBHOOK_URL=https://discord.com/api/webhooks/...
GITHUB_TOKEN=ghp_...          # PAT with `repo` scope — used by the GitHub REST API to create parity issues
GITHUB_REPO=fedepoi/bardidinaXmageUI
```

### 3. Create preset decks directory

```bash
mkdir -p preset_decks/
# Copy deck JSON files into preset_decks/
```

### 4. First build & run

```bash
export DOCKER_BUILDKIT=1
docker compose -f forge-engine/crates/forge-server/compose.yml build parity-dashboard
docker compose -f forge-engine/crates/forge-server/compose.yml up -d parity-dashboard
```

Dashboard will be at `http://<server-ip>:8080`.

## Auto-Deploy with n8n

### 1. Import the n8n workflow

1. Open your n8n instance
2. Go to **Workflows** → **Import from File**
3. Select `n8n-webhook-workflow.json` from the repo root
4. **Edit the "Run deploy.sh" node** — update the `cd` path to match your server:
   ```
   cd ~/bardidinaXmageUI && ./deploy.sh 2>&1 | tee /tmp/deploy.log
   ```
5. **Activate** the workflow

### 2. Add GitHub webhook

1. Go to the repo on GitHub → **Settings** → **Webhooks** → **Add webhook**
2. **Payload URL**: `https://<your-n8n-host>/webhook/github-deploy`
3. **Content type**: `application/json`
4. **Secret**: leave empty (or add one and configure n8n header auth)
5. **Events**: select **Just the push event**
6. Click **Add webhook**

### 3. Test it

Push a commit to `main` and check:
- n8n execution log shows successful run
- `cat /tmp/deploy.log` on server shows deploy output
- `docker compose -f forge-engine/crates/forge-server/compose.yml ps` shows container running

## Manual Deploy

```bash
cd ~/bardidinaXmageUI
./deploy.sh
```

The script will:
- `git pull origin main`
- Diff what changed since last deploy
- Only rebuild if Java/Rust/infra files changed
- Restart the container

## Useful Commands

```bash
# View logs
docker compose -f forge-engine/crates/forge-server/compose.yml logs -f parity-dashboard

# Restart without rebuild
docker compose -f forge-engine/crates/forge-server/compose.yml restart parity-dashboard

# Full rebuild (no cache)
docker compose -f forge-engine/crates/forge-server/compose.yml build --no-cache parity-dashboard

# Check parity database
docker compose -f forge-engine/crates/forge-server/compose.yml exec parity-dashboard sqlite3 /app/data/parity.db ".tables"
```
