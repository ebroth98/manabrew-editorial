# Deployment Guide

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
GITHUB_TOKEN=ghp_...          # for gh CLI inside container
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
