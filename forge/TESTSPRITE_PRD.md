# Forge — Product Requirements Document (TestSprite)

## Product Overview

Forge is an autonomous AI software-engineering agent that fixes GitHub issues using any
OpenAI-compatible language model. It clones a repository into an isolated Docker sandbox,
analyses the problem statement, and produces a git diff (patch) that resolves the issue.
The patch is optionally submitted as a GitHub pull request.

`forge-api` is the HTTP server wrapper around the Forge CLI. It exposes REST endpoints so
external tools (like TestSprite) can trigger runs, query GitHub issues, and read trajectory
statistics without invoking the CLI directly.

---

## Core Goals

- Allow automated tests to trigger Forge agent runs via HTTP POST.
- Expose GitHub issue listing so test runners can discover real issues to fix.
- Expose trajectory statistics so test pipelines can assert on outcomes (submitted, forfeited, etc.).
- Provide a `/health` endpoint for readiness and liveness checks.

---

## Key Features

1. **Agent Run (POST /api/run)** — Start the Forge agent on a GitHub issue or plain-text problem.
2. **Issue Listing (GET /api/issues)** — Query open GitHub issues for a repository.
3. **Trajectory Stats (GET /api/stats)** — Aggregate counts from `.traj` output files.
4. **Health Check (GET /health)** — Simple liveness probe.

---

## API Endpoints

### GET /health

**Purpose:** Liveness / readiness probe.

**Response (200 OK):**
```json
{
  "status": "ok",
  "version": "0.1.0"
}
```

---

### POST /api/run

**Purpose:** Run the Forge agent on a GitHub issue or a plain-text problem statement.

> **Note:** This is a long-running operation (minutes). The request blocks until the agent
> finishes. Timeout your HTTP client accordingly (recommend ≥ 10 minutes).

**Request body (JSON):**
```json
{
  "github_url": "https://github.com/owner/repo/issues/42",
  "model": "gpt-4o",
  "base_url": "https://api.openai.com/v1",
  "api_key": "sk-...",
  "image": "akachiokey/forge-sandbox:latest",
  "max_steps": 100,
  "output_dir": "trajectories"
}
```

Alternatively, supply `repo` + `issue` instead of `github_url`:
```json
{
  "repo": "owner/repo",
  "issue": 42,
  "problem_text": "Optional plain-text override of problem statement"
}
```

**Field defaults:**
- `model` → `FORGE_MODEL` env var
- `base_url` → `FORGE_BASE_URL` env var
- `api_key` → `FORGE_API_KEY` env var
- `image` → `"forge-sandbox:latest"`
- `max_steps` → `100`
- `output_dir` → `"trajectories"`

**Response (200 OK):**
```json
{
  "exit_status": "submitted",
  "has_submission": true,
  "submission_preview": "diff --git a/src/..."
}
```

**Possible `exit_status` values:**
| Value | Meaning |
|---|---|
| `submitted` | Agent produced a patch and submitted it |
| `forfeited` | Agent gave up without a solution |
| `step_limit_reached` | Max steps exhausted |
| `error` | Agent encountered a fatal error |

**Error responses:**
- `400 Bad Request` — Missing or invalid fields (e.g. no problem source provided).
- `500 Internal Server Error` — Agent runtime failure.

---

### GET /api/issues

**Purpose:** List open GitHub issues for a repository.

**Query parameters:**
| Param | Required | Default | Description |
|---|---|---|---|
| `repo` | Yes | — | `owner/repo` format |
| `label` | No | — | Filter by label name |
| `limit` | No | `30` | Max issues returned (max 100) |

**Example:** `GET /api/issues?repo=OkeyAmy/Axioschat-Onboard&label=bug&limit=10`

**Response (200 OK):**
```json
{
  "repo": "OkeyAmy/Axioschat-Onboard",
  "count": 2,
  "issues": [
    {
      "number": 27,
      "title": "Add input validation to onboarding form",
      "url": "https://github.com/OkeyAmy/Axioschat-Onboard/issues/27",
      "labels": ["bug", "good first issue"]
    }
  ]
}
```

**Error responses:**
- `502 Bad Gateway` — GitHub API call failed or returned an error.

**Auth:** Set `GITHUB_TOKEN` in the server environment. Without it, GitHub rate limits
unauthenticated requests to 60/hour.

---

### GET /api/stats

**Purpose:** Read aggregate statistics from `.traj` output files.

**Query parameters:**
| Param | Required | Default | Description |
|---|---|---|---|
| `dir` | No | `"trajectories"` | Path to the trajectory directory |

**Example:** `GET /api/stats?dir=/trajectories`

**Response (200 OK):**
```json
{
  "directory": "/trajectories",
  "total": 10,
  "submitted": 7,
  "forfeited": 1,
  "errors": 1,
  "step_limit_reached": 1,
  "other": 0
}
```

**Error responses:**
- `404 Not Found` — The directory does not exist.
- `500 Internal Server Error` — Directory read failure.

---

## User Flow Summary

1. **Health check** — TestSprite calls `GET /health` to confirm the server is up.
2. **List issues** — TestSprite calls `GET /api/issues?repo=...` to find a real issue.
3. **Trigger run** — TestSprite calls `POST /api/run` with the issue URL.
4. **Verify outcome** — TestSprite asserts `exit_status == "submitted"` and `has_submission == true`.
5. **Check stats** — TestSprite calls `GET /api/stats` and asserts `submitted >= 1`.

---

## Validation Criteria

- `GET /health` returns 200 with `{"status": "ok"}`.
- `GET /api/issues` with a valid public repo returns a non-empty list.
- `GET /api/issues` with an invalid repo returns a 502 with a descriptive error.
- `POST /api/run` with missing problem source returns 400.
- `POST /api/run` with a valid GitHub issue returns 200 with `exit_status` set.
- `GET /api/stats` with a valid directory returns correct numeric totals.
- `GET /api/stats` with a missing directory returns 404.

---

## Environment Variables

| Variable | Required | Description |
|---|---|---|
| `FORGE_MODEL` | Yes | Model name, e.g. `"gpt-4o"` |
| `FORGE_BASE_URL` | Yes | OpenAI-compatible base URL |
| `FORGE_API_KEY` | Yes | API key for the model provider |
| `GITHUB_TOKEN` | Recommended | GitHub PAT for issue reads and PR creation |
| `FORGE_API_PORT` | No (default: 5000) | Port the HTTP server binds to |
| `RUST_LOG` | No | Log level filter, e.g. `"forge_api=info"` |

---

## Tech Stack

- **Language:** Rust (2021 edition)
- **HTTP framework:** axum 0.8
- **Runtime:** tokio (async)
- **Serialization:** serde + serde_json
- **Docker orchestration:** bollard (for sandbox containers)
- **Model client:** reqwest (OpenAI-compatible REST)
