# Forge — Production Examples

**No Rust. No YAML. No compiling.**
Just Docker + a `.env` file with 3 lines.

## The only thing you need to start

```bash
cp .env.example method-1-oneshot/.env
# Edit .env — set these 3:
#   FORGE_MODEL=your-model-name
#   FORGE_BASE_URL=https://your-provider.example.com/v1
#   FORGE_API_KEY=your-api-key

cd method-1-oneshot && docker compose run --rm forge
```

Done. Forge pulls the image, fixes the issue, exits.

---

## Issues created for testing

| # | Title | Method |
|---|---|---|
| [#27](https://github.com/OkeyAmy/Axioschat-Onboard/issues/27) | Add `isValidWalletAddress()` | Method 1 — one-shot |
| [#28](https://github.com/OkeyAmy/Axioschat-Onboard/issues/28) | Add `formatTokenAmount()` | Method 2 — same image, optional YAML |
| [#29](https://github.com/OkeyAmy/Axioschat-Onboard/issues/29) | Add `truncateAddress()` *(labelled `forge`)* | Method 3 — continuous watch |

---

## Method 1 — One-shot (`method-1-oneshot/`)

**Use case:** Fix a specific issue once and get the patch.

```bash
cd method-1-oneshot
cp ../.env.example .env    # fill in FORGE_API_KEY, GITHUB_TOKEN, DOCKER_GID
docker compose run --rm forge
```

Override issue without editing `.env`:
```bash
docker compose run --rm forge run --repo OkeyAmy/Axioschat-Onboard --issue 27
```

---

## Method 2 — env vars vs YAML (`method-2-config-file/`)

**Use case:** Understand that YAML is optional — not required.

```bash
cd method-2-config-file
cp ../.env.example .env

docker compose run --rm forge        # ← default: just .env, no YAML
docker compose run --rm forge-yaml   # ← optional: YAML config for advanced control
```

Both fix issue #28 identically. YAML lets you pin prompts and step limits — it's not required.

---

## Method 3 — Watch mode (`method-3-watch/`)

**Use case:** Runs forever. Label any issue `forge` on GitHub → Forge picks it up automatically.
Issue #29 is already labelled — it will be fixed within 60 seconds of the service starting.

```bash
cd method-3-watch
cp ../.env.example .env
docker compose up watch    # runs until Ctrl+C
```

---

## Shared trajectories

All 3 methods write to `examples/trajectories/`.

```bash
# From any method directory:
docker compose run --rm quick-stats
```

---

## Required env vars

| Variable | Description |
|---|---|
| `FORGE_MODEL` | Model name e.g. `gpt-4o` or `models/gemini-2.0-flash-001` |
| `FORGE_BASE_URL` | OpenAI-compatible API base URL |
| `FORGE_API_KEY` | API key |
| `GITHUB_TOKEN` | Recommended — raises rate limit, required for private repos |
| `DOCKER_GID` | Run `getent group docker \| cut -d: -f3` to find yours |
