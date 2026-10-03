# Method 1 — One-shot fix (Docker, no Rust needed)

Fixes a single GitHub issue then exits. No Rust, no local compilation.

**Issue being fixed:** [#27 — Add isValidWalletAddress()](https://github.com/OkeyAmy/Axioschat-Onboard/issues/27)

## Setup

```bash
# 1. Copy env and fill in your API key + GitHub token
cp ../. env.example .env
# Edit .env: set FORGE_API_KEY, GITHUB_TOKEN, DOCKER_GID

# 2. Run
docker compose run --rm forge
```

## What happens

1. Forge pulls `akachiokey/forge:latest` from Docker Hub
2. Fetches issue #27 from GitHub
3. Starts an isolated Docker sandbox (`akachiokey/forge-sandbox:latest`)
4. Clones `OkeyAmy/Axioschat-Onboard` inside the sandbox
5. Autonomously explores the repo and implements `isValidWalletAddress()`
6. Runs `submit` — captures the git diff
7. Exits. Trajectory saved to `../trajectories/`

## Override repo/issue inline

```bash
# Fix a different issue without editing .env
docker compose run --rm forge run \
  --repo OkeyAmy/Axioschat-Onboard \
  --issue 28
```

## View trajectory stats

```bash
docker compose run --rm quick-stats
```
