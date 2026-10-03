# Method 2 — env vars vs YAML config (same image, your choice)

Shows that YAML is **completely optional**. The default run uses only `.env` — no YAML file needed.
YAML is available as a power-user option when you want to pin prompts or max steps in a file.

**Issue being fixed:** [#28 — Add formatTokenAmount()](https://github.com/OkeyAmy/Axioschat-Onboard/issues/28)

---

## Default: just .env (no YAML)

```bash
cp ../.env.example .env
# Set FORGE_API_KEY, GITHUB_TOKEN, DOCKER_GID in .env

docker compose run --rm forge
```

That's it. Forge reads `FORGE_REPO` and `FORGE_ISSUE` from `.env` and runs.

---

## Optional: YAML config (advanced)

Use this when you want to version-control the exact prompts, step limit, or parser type.

```bash
docker compose run --rm forge-yaml
```

The YAML file (`forge-issue28.yaml`) is mounted into the container at runtime —
the image itself does not change.

**YAML gives you control over:**
- `max_steps` — cap how long the agent runs
- `parser_type` — `thought_action` / `action_only` / `function_calling`
- `system_template` — the agent's persona and instructions
- `instance_template` — how the problem statement is presented

**Everything else works identically.** The YAML is just a structured `.env` with extra fields.

---

## The key point

```
docker compose run --rm forge          ← env vars only, no YAML
docker compose run --rm forge-yaml     ← same image + optional YAML on top
```

Both produce the same output format. YAML adds control, not complexity for basic use.
