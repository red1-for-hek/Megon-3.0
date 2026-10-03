# Method 3 — Watch mode (continuous, label-triggered)

Forge runs as a long-lived service. Any issue labelled **`forge`** on the repo is picked up
automatically and fixed — no manual trigger needed.

**Issue being fixed:** [#29 — Add truncateAddress()](https://github.com/OkeyAmy/Axioschat-Onboard/issues/29)
*(already labelled `forge` — Forge will pick it up within `FORGE_WATCH_INTERVAL` seconds)*

## Setup

```bash
cp ../. env.example .env
# Edit .env: set FORGE_API_KEY, GITHUB_TOKEN, DOCKER_GID

# Start the watcher (runs until you stop it)
docker compose up watch
```

## To trigger a new fix without stopping the service

1. Go to any issue on `https://github.com/OkeyAmy/Axioschat-Onboard/issues`
2. Add the label **`forge`**
3. Within `FORGE_WATCH_INTERVAL` seconds (default 60), Forge picks it up and starts working

## Stop

```bash
docker compose down
```

## View trajectory stats

```bash
docker compose run --rm quick-stats
```

## Why watch mode?

- Zero-touch: label → fix → patch, no human needed in the loop
- Runs on Nosana decentralised compute for free/cheap GPU time
- Ideal for a team workflow: any developer labels an issue and Forge handles it
