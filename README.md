# Megon 3.0 — Decentralized AI Software Engineering Agent

Megon 3.0 is a **decentralized autonomous AI agent** that orchestrates coding,
research, and security verification workloads. It is designed to run on the
**Nosana decentralized GPU network**, where each agent workload executes on
community-provided GPU compute.

## What It Is

Megon 3.0 unifies three open-source projects into a single AI agent system:

| Component | Source | Role |
|-----------|--------|------|
| **AgentForge** | [Andy00L/agent-challenge-4](https://github.com/Andy00L/agent-challenge-4) | Orchestrator, UI, Nosana GPU plugin, mission pipeline |
| **Forge** | [OkeyAmy/forge](https://github.com/OkeyAmy/forge) | Autonomous coding agent (Rust), sandboxed execution, GitHub integration |
| **Airlock** | [Wnayar/airlock](https://github.com/Wnayar/airlock) | Security gate: sandbox detonation, static analysis, LLM judgment |

The result is an AI agent that can:
- Understand natural-language tasks (no GitHub issue required)
- Plan multi-step workflows autonomously
- Generate, test, and verify code in sandboxed environments
- Run AI inference on decentralized GPU infrastructure
- Return verified results without centralized cloud dependency

## Architecture

```
                    USER
                     │
                     ▼
                MEGON 3.0
                     │
                 Planner
                     │
          ┌──────────┼──────────┐
          ▼          ▼          ▼
      Research    Coding     Security
       Agent       Agent       Agent
          │          │          │
          └──────────┼──────────┘
                     ▼
              Verification
                     │
                     ▼
               Final Result

        Decentralized Execution (Nosana)
                     │
                     ▼
             NOSANA NETWORK
                     │
              ┌──────┼──────┐
              ▼      ▼      ▼
             GPU    GPU    GPU
           Worker Worker Worker
```

See [ARCHITECTURE.md](./ARCHITECTURE.md) for detailed Mermaid diagrams.

## Current Status

### ✅ Working Locally (MVP)
- Pure Node.js orchestrator (`megon/orchestrator.mjs`) — zero dependencies
- Full pipeline: Plan → Code → Security Check → Test → Result
- Configurable OpenAI-compatible LLM (works with free OpenRouter models)
- Airlock HTTP API wrapper (`airlock/api.py`) for security verification
- Local static-analysis fallback when Airlock services are unavailable
- Docker Compose setup for full-stack local development
- All source code from upstream projects preserved and functional

### 🔧 Prepared for Nosana Deployment
- Unified `Dockerfile.nosana` combining all three services
- Nosana job definition (`nosana/megon_job_definition.json`)
- Start script (`start.sh`) that boots all services in one container
- Environment variables isolated for Nosana credential injection
- AgentForge's existing `@nosana/kit` SDK integration preserved

> **Note:** Nosana deployment requires GPU compute credits. The architecture
> is fully prepared but not yet deployed to the Nosana network.

## Quick Start

### Prerequisites
- Node.js 23+ (for orchestrator)
- A free API key from [OpenRouter](https://openrouter.ai/keys) (or any OpenAI-compatible provider)

### Setup
```bash
cp .env.example .env
# Edit .env and set your API_KEY (free from OpenRouter)
```

### Run the MVP
```bash
node megon/orchestrator.mjs "Create a Python function that checks whether a number is prime, test it with 2, 17, and 25, and explain the result."
```

### Run Full Stack (Docker Compose)
```bash
docker compose up --build
# UI: http://localhost:3000
# Forge API: http://localhost:5000
# Airlock API: http://localhost:5001
```

## Environment Variables

| Variable | Required | Description | Cost |
|----------|----------|-------------|------|
| `API_KEY` | Yes | OpenAI-compatible API key | Free (OpenRouter) |
| `MODEL` | Yes | Model identifier | Free |
| `BASE_URL` | Yes | API endpoint URL | Free |
| `NOSANA_API_KEY` | No | Nosana GPU network access | Free testnet |
| `GITHUB_TOKEN` | No | GitHub API (optional PR creation) | Free |
| `DAYTONA_API_KEY` | No | Airlock sandbox detonation | Optional |

See `.env.example` for the complete template.

## Example Workflow

```
User: "Build a Python function that checks if a number is prime"
  ↓
Planner: Analyzes task → decides code generation + testing needed
  ↓
Coding Agent: Generates is_prime() function with test cases
  ↓
Security Gate: Static analysis → SAFE verdict
  ↓
Test Runner: Executes in sandbox → all tests pass
  ↓
Result: Verified code + explanation returned to user
```

Run the demo: `node megon/orchestrator.mjs` (uses default prime-checker task)

## Nosana Integration Path

When Nosana GPU credits are available:

1. Build the unified image: `docker build -f Dockerfile.nosana -t megon-3.0 .`
2. Deploy via Nosana CLI: `nosana job deploy nosana/megon_job_definition.json`
3. The container runs all three services on a decentralized GPU node
4. Users interact via the same interface — no code changes needed

The transition from local to decentralized execution requires only adding
Nosana credentials to the environment. The agent workflow remains identical.

## License & Attribution

All three upstream projects are MIT-licensed. See [LICENSES_AND_ATTRIBUTION.md](./LICENSES_AND_ATTRIBUTION.md)
for full provenance and attribution details.

## Documentation

- [ARCHITECTURE.md](./ARCHITECTURE.md) — System design and data flow diagrams
- [MERGE_PLAN.md](./MERGE_PLAN.md) — Detailed integration plan (17 sections)
- [BASELINE.md](./BASELINE.md) — Per-project baseline test results
- [LICENSES_AND_ATTRIBUTION.md](./LICENSES_AND_ATTRIBUTION.md) — License compliance
