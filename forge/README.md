# Forge

**Forge** is an autonomous AI software-engineering agent. Point it at any GitHub issue and it will clone the repository, write the fix, and produce a verified git diff patch — all inside an isolated Docker sandbox, driven by any OpenAI-compatible model API.

It integrates with [ElizaOS](https://elizaos.com) as a first-class action handler and can be deployed on decentralised compute via [Nosana](https://nosana.com).

---

## Quick start — no Rust required

The only prerequisite is **Docker**.

### 1. Get the config files

```bash
git clone <your-repo-url>
cd forge
```

### 2. Configure credentials

```bash
cp .env.example .env
```

Open `.env` and fill in three required values:

```dotenv
FORGE_MODEL=your-model-name        # e.g. gpt-4o, models/gemini-2.0-flash-001
FORGE_BASE_URL=https://...         # OpenAI-compatible endpoint
FORGE_API_KEY=your-api-key
```

### 3. See what issues are open on a repo

```bash
docker compose run --rm list-issues
# or specify a different repo:
docker compose run --rm list-issues --repo owner/repo
```

Output:

```
Open issues on owner/repo (5 shown):

#      Title
------------------------------------------------------------------------
#3     Add input validation to signup form  [bug]
#7     Dark mode flicker on page load       [enhancement]
#12    Upgrade to Node 20
```

### 4. Fix an issue

Set `FORGE_REPO` and `FORGE_ISSUE` in your `.env`, then:

```bash
docker compose run --rm forge
```

Or pass them directly:

```bash
docker compose run --rm forge run --repo owner/repo --issue 12
```

Forge will clone the repo, work autonomously, and print a patch when done.

### 5. Always-on watch mode

Set it up once — then just label issues on GitHub and Forge handles the rest.

**Add to `.env`:**

```dotenv
FORGE_WATCH_REPO=owner/repo
FORGE_WATCH_LABEL=forge        # label to watch for
FORGE_WATCH_INTERVAL=60        # seconds between polls
GITHUB_TOKEN=ghp_...           # required to push the fix branch back
```

**Start in the background:**

```bash
docker compose up watch -d
```

**That's it.** Go to GitHub, open any issue in your repo, and add the label **`forge`**. Within 60 seconds Forge picks it up, fixes it, and pushes the result to a branch named `forge/issue-{N}`. Review the branch and merge when you're happy.

- Already-processed issues are tracked in `trajectories/watch_state.json` — Forge never double-processes the same issue.
- The service has `restart: unless-stopped` — it survives crashes and Docker restarts automatically.
- Stop it any time with `docker compose down`.

---

## How Forge works

```
GitHub issue  ──►  add label "forge"
                          │
                    Forge detects it
                    (within 60 s)
                          │
               ┌──────────────────────┐
               │  Docker sandbox      │
               │  1. Clone repo       │
               │  2. Agent loop       │
               │     think → act      │
               │     → observe        │
               │  3. submit           │
               │     git diff patch   │
               └──────────────────────┘
                          │
                    Push branch
                    forge/issue-{N}
                          │
                   You review & merge
```

---

## CLI reference

### `forge list-issues`

Scan a GitHub repo and show open issues.

```bash
forge list-issues --repo owner/repo
forge list-issues --repo owner/repo --label bug
forge list-issues --repo owner/repo --limit 50
```

| Flag | Default | Description |
|---|---|---|
| `--repo` | — | GitHub repository (owner/repo) |
| `--label` | — | Filter by label |
| `--limit` | 30 | Maximum issues to show |

### `forge run`

Run the agent on a single issue or problem.

```bash
# Recommended — pick from list-issues output
forge run --repo owner/repo --issue 42

# Or with a full URL
forge run --github-url https://github.com/owner/repo/issues/42

# Inline problem text (no GitHub needed)
forge run --problem-text "Add rate-limiting to the /api/login endpoint"
```

| Flag | Env var | Default | Description |
|---|---|---|---|
| `--repo` | — | — | GitHub repository (use with `--issue`) |
| `--issue` | — | — | Issue number |
| `--github-url` | — | — | Full GitHub issue URL |
| `--problem-text` | — | — | Inline problem statement |
| `--problem-file` | — | — | Path to plain-text problem file |
| `--model` | `FORGE_MODEL` | — | Model identifier |
| `--base-url` | `FORGE_BASE_URL` | — | OpenAI-compatible API URL |
| `--api-key` | `FORGE_API_KEY` | — | API key |
| `--image` | — | `forge-sandbox:latest` | Docker sandbox image |
| `--output-dir` | — | `trajectories` | Where to save `.traj` files |
| `--max-steps` | — | `100` | Step limit |
| `--config` | — | — | YAML config file (see Advanced) |

### `forge watch`

Poll a repo and automatically fix every issue that carries a given label.
When `GITHUB_TOKEN` is set, the fix is pushed to branch `forge/issue-{N}` automatically.

```bash
forge watch --repo owner/repo --label forge --interval 60
```

| Flag | Env var | Default | Description |
|---|---|---|---|
| `--repo` | `FORGE_WATCH_REPO` | — | GitHub repository |
| `--label` | `FORGE_WATCH_LABEL` | `forge` | Label to watch for |
| `--interval` | `FORGE_WATCH_INTERVAL` | `60` | Poll interval (seconds) |
| `--model` | `FORGE_MODEL` | — | Model identifier |
| `--base-url` | `FORGE_BASE_URL` | — | API base URL |
| `--api-key` | `FORGE_API_KEY` | — | API key |
| `--image` | `FORGE_SANDBOX_IMAGE` | `forge-sandbox:latest` | Sandbox image |
| `--output-dir` | — | `trajectories` | Trajectory output dir |

> **Branch push:** set `GITHUB_TOKEN` in `.env` and every completed fix is pushed to `forge/issue-{N}` on GitHub automatically.

### `forge quick-stats`

Summarise trajectory results.

```bash
forge quick-stats                   # scans ./trajectories
forge quick-stats /path/to/trajs
```

---

## Environment variables

| Variable | Required | Description |
|---|---|---|
| `FORGE_MODEL` | Yes | Model identifier passed to the API |
| `FORGE_BASE_URL` | Yes | Base URL of an OpenAI-compatible endpoint |
| `FORGE_API_KEY` | Yes | API key for the endpoint |
| `FORGE_REPO` | For one-shot | GitHub repo (owner/repo) |
| `FORGE_ISSUE` | For one-shot | Issue number |
| `FORGE_WATCH_REPO` | For watch | Repo to monitor |
| `FORGE_WATCH_LABEL` | No | Label to watch (default: `forge`) |
| `FORGE_WATCH_INTERVAL` | No | Poll interval in seconds (default: `60`) |
| `GITHUB_TOKEN` | For branch push | PAT — required to push fix branches; also raises API rate limit and enables private repos |
| `RUST_LOG` | No | Log filter — e.g. `forge=debug` (default: `forge=warn`) |
| `DOCKER_GID` | No | Docker group GID on host (docker-compose socket mount) |

---

## Deploying to Nosana

Forge runs as a standard Docker container on [Nosana](https://nosana.com)'s decentralised compute network.

### 1. Configure the job definition

Edit `nos_job_def/forge_job_definition.json` — replace the `$FORGE_*` placeholders with your actual values, or pass them as environment variables via the Nosana CLI.

### 2. Deploy

```bash
npm install -g @nosana/cli

nosana job post \
  --file ./nos_job_def/forge_job_definition.json \
  --market nvidia-4090 \
  --timeout 120 \
  --api <YOUR_NOSANA_API_KEY>
```

Or paste the job definition JSON into the [Nosana Dashboard](https://dashboard.nosana.com/deploy).

---

## Output — trajectories

Every run produces a `<instance-id>.traj` JSON file in `output_dir` with the complete agent history: every command, every output, every model response, and timing data.

```bash
forge quick-stats          # summary counts
cat trajectories/*.traj    # full detail
```

---

## Advanced — YAML config

Most users only need environment variables and CLI flags. For advanced use (custom system prompts, extended timeouts, batch runs), use a YAML config:

```bash
forge run --config example.yaml
```

See `example.yaml` in this repository for all available fields with comments.

---

## ElizaOS integration

`forge-plugin` exposes a `SolveIssueAction` that plugs into any [ElizaOS](https://elizaos.com) agent:

```rust
use forge_plugin::action::{SolveIssueAction, SolveIssueParams};

let result = SolveIssueAction::new()
    .handle(SolveIssueParams {
        github_url: Some("https://github.com/owner/repo/issues/42".into()),
        model_name: Some("your-model".into()),
        base_url:   Some("https://your-provider.example.com/v1".into()),
        api_key:    Some("your-api-key".into()),
        ..Default::default()
    })
    .await?;

println!("status: {:?}", result.exit_status);
println!("patch:  {:?}", result.submission);
```

---

## Building from source

Rust 1.82+ required. Docker still needed at runtime.

```bash
# Build the sandbox image once
docker build -f Dockerfile.sandbox -t forge-sandbox:latest .

# Build the forge binary
cargo build --release -p forge

# Run
./target/release/forge list-issues --repo owner/repo
```

Run all tests:

```bash
cargo test --workspace
# Include Docker-gated integration tests:
cargo test --workspace -- --include-ignored
```

---

## Crate architecture

```
forge/crates/
├── forge-types      Shared data types
├── forge-tools      Response parsers
├── forge-model      Model backends (OpenAI-compat, Anthropic, Replay, Human)
├── forge-env        Docker runtime, bash sessions
├── forge-agent      Agent loop, history processors
├── forge-run        RunSingle, RunBatch, YAML config
├── forge-plugin     ElizaOS integration
└── forge            CLI binary (list-issues, run, watch, quick-stats)
```

---

## License

MIT
