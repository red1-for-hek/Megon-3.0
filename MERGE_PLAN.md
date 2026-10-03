# Merge Plan — Megon 3.0

This document defines exactly how AgentForge, Forge, and Airlock will be
integrated into a single coherent application.

---

## 1. Current Architecture of AgentForge

**Language:** TypeScript (ES2022 modules)
**Runtime:** Bun + Node.js 23
**Framework:** ElizaOS v2 (agent framework)
**Frontend:** React 19 + Vite 8 + TailwindCSS 4 + ReactFlow (@xyflow/react) + Zustand + shadcn/ui
**Backend:** ElizaOS runtime + Express 5 Fleet API (port 3001) + Socket.IO
**Reverse Proxy:** Nginx (port 3000 → ElizaOS:3002 + FleetAPI:3001)
**Package Manager:** Bun (backend), npm (frontend)
**Database:** SQLite (ElizaOS internal)
**LLM:** OpenAI-compatible via @elizaos/plugin-openai (supports any endpoint)
**Nosana SDK:** @nosana/kit v2.2.4
**Docker:** Single Dockerfile (node:23-slim base), worker/Dockerfile for GPU nodes
**Ports:** 3000 (nginx/public), 3001 (Fleet API), 3002 (ElizaOS internal)
**Entrypoint:** src/index.ts → ElizaOS project definition
**Agent Templates:** researcher, writer, monitor, publisher, analyst, scene-writer,
  image-generator, video-generator, narrator
**Key Services:**
  - NosanaManager: GPU market selection, deployment lifecycle, credit management
  - WorkerClient: HTTP communication with deployed ElizaOS workers on Nosana
  - MissionOrchestrator: DAG pipeline planning and execution
  - MediaAssembler, ImageGenRouter, TTSClient, VideoGenRouter: multimodal output
  - Fleet API: 16 REST endpoints + Socket.IO for real-time progress

## 2. Current Architecture of Forge

**Language:** Rust (edition 2021, resolver v2)
**Build System:** Cargo workspace with 9 crates
**Framework:** Axum 0.8 (HTTP API), Tokio (async runtime), Clap (CLI)
**Sandbox:** Docker containers via bollard crate (or E2B cloud sandboxes)
**LLM:** OpenAI-compatible API + Anthropic native support
**GitHub Integration:** GitHub App webhooks, PAT-based API, issue listing, PR creation
**Docker:** Multi-stage build (rust:slim-bookworm → debian:bookworm-slim)
  - Dockerfile.sandbox: python:3.11-slim for code execution
  - docker-compose.yml: forge, watch, list-issues, quick-stats, forge-api services
**Ports:** 5000 (forge-api default)
**Entrypoints:**
  - CLI: `forge run --repo owner/repo --issue 42`
  - API: `forge-api` binary (Axum server)
**Key Crates:**
  - forge-types: Trajectory, Step, History, Error types
  - forge-tools: Action parsers (XML, function-calling, thought-action)
  - forge-model: LLM clients (OpenAI-compat, Anthropic, replay, human)
  - forge-env: Docker/E2B sandbox, bash sessions, repo cloning
  - forge-agent: Core SWE-agent loop, problem statement handling
  - forge-run: Single/batch execution orchestration
  - forge-api: HTTP routes (/api/run, /api/issues, GitHub webhooks)
  - forge-plugin: ElizaOS plugin adapter
**Output:** Trajectory JSON files, git diff patches

## 3. Current Architecture of Airlock

**Language:** Python 3.11+
**Package Manager:** pip (requirements.txt)
**Framework:** No web framework — CLI tool + importable library
**Sandbox:** Daytona SDK (disposable cloud sandboxes)
**LLM:** OpenAI-compatible via openai Python SDK (ai& provider)
**Security Analysis Pipeline:**
  1. RUN: Detonate in Daytona sandbox with audit hooks → behavior trace
  2. READ: Static analysis via Nosana GPU or ai& fallback → suspicion score
  3. MATCH: Doubleword embeddings similarity to known malware
  4. REPUTATION: PyPI age/downloads, OSV advisories, Oxylabs web intel
  5. JUDGE: ai& LLM weighs all evidence → SAFE/BLOCK verdict
**Entrypoint:** `python -m airlock <package>` or `airlock.check(package)`
**Hook System:** PreToolUse hook for Claude Code (.claude/settings.json)
**Key Modules:**
  - gate.py: Main orchestrator, monotonic combine (tripwire OR judge = BLOCK)
  - sandbox/detonate.py: Daytona sandbox creation, script execution, cleanup
  - sandbox/tripwires.py: Deterministic rule-based blocking
  - sandbox/runner.py: Audit hook script generation
  - analysis/static_read.py: Source fetching from PyPI, LLM analysis
  - analysis/judge.py: LLM-based verdict generation
  - analysis/doubleword.py: Embedding similarity matching
  - analysis/reputation.py: Package reputation signals
  - fanout.py: Parallel multi-package checking
  - types.py: Verdict, Event, StaticReport, Reputation dataclasses

## 4. Proposed Megon 3.0 Architecture

Megon 3.0 unifies these three projects into a single application where:
- AgentForge provides the UI, orchestrator, and Nosana GPU management
- Forge provides the autonomous coding agent capability
- Airlock provides security verification of generated code

### High-Level Flow:
```
User Input (Chat UI)
    ↓
Megon Planner (ElizaOS + LLM on Nosana)
    ↓
┌─────────────────────────────────┐
│  Research Agent (Nosana GPU)    │ ← AgentForge worker template
│  Analyzes requirements/context  │
└──────────────┬──────────────────┘
               ↓
┌─────────────────────────────────┐
│  Coding Agent (Forge)           │ ← Forge via API or Nosana job
│  Clones repo, writes fix,       │
│  generates patch                │
└──────────────┬──────────────────┘
               ↓
┌─────────────────────────────────┐
│  Security Gate (Airlock)        │ ← Airlock as Python service
│  Detonates + reads + judges     │
│  the generated code/patch       │
└──────────────┬──────────────────┘
               ↓
┌─────────────────────────────────┐
│  Testing Agent (Forge sandbox)  │ ← Forge test execution
│  Runs tests in Docker sandbox   │
└──────────────┬──────────────────┘
               ↓
    Verified Result / Patch
    ↓ (only if user requests)
    GitHub PR Creation
```

### Communication Model:
- **AgentForge ↔ Forge:** HTTP REST API (Forge's forge-api on port 5000)
  - POST /api/run with problem_text or github_url
  - Response includes exit_status, submission (patch)
- **AgentForge ↔ Airlock:** HTTP wrapper needed (new adapter)
  - Airlock currently has no HTTP API — we add a thin FastAPI/Flask wrapper
  - POST /api/check with package name or code snippet
  - Response: Verdict JSON
- **Forge ↔ Docker:** Direct Docker socket (bollard) for sandbox
- **All ↔ Nosana:** @nosana/kit SDK (TS) or Nosana job definitions (JSON)

## 5. Exact Components That Will Be Reused

### From AgentForge (keep as-is):
- Entire `src/plugins/nosana/` directory
- Entire `frontend/` directory
- `worker/` directory (for deploying agents to Nosana GPUs)
- `characters/` directory
- `nos_job_def/` directory
- `nginx.conf`, `entrypoint.sh`, `Dockerfile`
- `src/index.ts` (extended with new agent templates)
- All ElizaOS plugin infrastructure

### From Forge (keep as-is):
- Entire `crates/` workspace (all 9 crates)
- `Dockerfile`, `Dockerfile.release`, `Dockerfile.sandbox`
- `nos_job_def/forge_job_definition.json`
- `example.yaml`, `examples/` directory
- `scripts/` directory
- The `forge-api` binary is the primary integration point

### From Airlock (keep as-is):
- Entire `airlock/` Python package
- `requirements.txt`
- `.claude/` settings (for reference)
- `tests/` directory
- `DESIGN.md` (reference documentation)

## 6. Exact Components That Will Be Moved

Nothing will be physically moved from its original subdirectory during Phase 1-6.
The three repos remain in their respective directories. Integration happens via:
- Docker Compose networking
- HTTP API calls between services
- A new top-level orchestrator that coordinates them

In future phases, specific files may be relocated into a unified structure.

## 7. Exact Components That Will Be Adapted

1. **Airlock HTTP Wrapper (NEW):** Airlock has no HTTP API. We need a thin
   Python HTTP server (FastAPI or Flask) wrapping `airlock.gate.check()` so
   AgentForge can call it via REST.

2. **Forge Nosana Job Definition (ADAPT):** The existing nos_job_def runs Forge
   as a one-shot CLI. For Megon 3.0, we may want forge-api running persistently
   on Nosana.

3. **AgentForge Worker Templates (EXTEND):** Add a "coder" template type that
   delegates to Forge instead of using an ElizaOS worker.

4. **Unified Environment Configuration (NEW):** A single .env file at the root
   that maps to each component's expected variables.

5. **Docker Compose (NEW):** A top-level docker-compose.yml that starts all
   three services with correct networking.

6. **Airlock static_read adaptation:** Point Airlock's Nosana endpoint at the
   same LLM infrastructure AgentForge manages.

## 8. Exact Components That Should NOT Be Merged

1. **Forge's landing page** (`landing/`) — marketing site, not needed
2. **AgentForge's media services** (ComfyUI, TTS, video gen) — out of scope
   for a coding agent; keep available but don't integrate into main flow
3. **Airlock's pip hook** (`.claude/settings.json`) — designed for Claude Code
   specifically; our integration is at the orchestrator level, not pip interception
4. **Forge's Caddy reverse proxy** (`deploy/Caddyfile`) — AgentForge already
   has nginx; use one reverse proxy
5. **Duplicate Dockerfiles** — keep each project's own Dockerfile; the top-level
   compose references them via build contexts
6. **Airlock's sponsors.py self-check** — hackathon-specific utility

## 9. Dependency Conflicts

| Conflict | Details | Resolution |
|----------|---------|------------|
| **Language mismatch** | TS (AgentForge) vs Rust (Forge) vs Python (Airlock) | Run as separate containers/processes; communicate via HTTP |
| **Node version** | AgentForge requires Node 23 + Bun | Isolated in its container |
| **Rust toolchain** | Forge requires rust:slim-bookworm | Isolated in its container |
| **Python version** | Airlock requires Python 3.11+ | Isolated in its container |
| **@elizaos versions** | AgentForge uses ^1.0.0, worker uses 1.7.2 | Pin to 1.7.2 everywhere |
| **Docker socket** | Both Forge and AgentForge need Docker access | Forge needs it for sandboxes; AgentForge for Nosana deployments. Separate sockets or shared carefully |
| **OpenAI SDK** | Airlock uses `openai>=1.40` (Python); AgentForge uses `@elizaos/plugin-openai` (TS) | No conflict — different languages |

## 10. Environment Variable Conflicts

| Variable | AgentForge | Forge | Airlock | Resolution |
|----------|-----------|-------|---------|------------|
| OPENAI_API_KEY | LLM key | Not used directly | Not used | Keep for AgentForge |
| FORGE_MODEL | Not used | Model name | Not used | Keep for Forge |
| FORGE_BASE_URL | Not used | LLM endpoint | Not used | Keep for Forge |
| FORGE_API_KEY | Not used | LLM API key | Not used | Keep for Forge |
| NOSANA_API_KEY | GPU deploy key | Not used | Not used (uses NOSANA_ENDPOINT) | Shared; rename Airlock's to NOSANA_LLM_ENDPOINT |
| GITHUB_TOKEN | Not used | GitHub access | Not used | Keep for Forge |
| SERVER_PORT | 3000 | Not used | Not used | Keep for AgentForge |
| MODEL_NAME | LLM model | Not used | Not used | Keep for AgentForge |

**Strategy:** Use prefixed variables in a unified .env:
- MEGON_* for shared config
- Each service reads its own prefixed vars
- A startup script maps MEGON_ vars to service-specific vars

## 11. Port Conflicts

| Service | Default Port | Megon 3.0 Port | Notes |
|---------|-------------|----------------|-------|
| AgentForge nginx | 3000 | 3000 | Public-facing |
| AgentForge Fleet API | 3001 | 3001 | Internal |
| AgentForge ElizaOS | 3002 | 3002 | Internal |
| Forge API | 5000 | 5000 | Internal |
| Airlock HTTP (new) | N/A | 5001 | New wrapper |
| Frontend dev (Vite) | 5173 | 5173 | Dev only |

No conflicts detected. All ports are distinct.

## 12. Docker Integration Plan

Create a top-level `docker-compose.yml` at `/workspaces/Megon-3.0/`:

```yaml
services:
  orchestrator:      # AgentForge (build context: ./agentforge)
    build: ./agentforge
    ports: ["3000:3000"]
    env_file: .env
    depends_on: [forge-api, airlock-api]

  forge-api:         # Forge REST API (build context: ./forge)
    build:
      context: ./forge
      args: { BIN: forge-api }
    ports: ["5000:5000"]
    env_file: .env
    volumes: ["/var/run/docker.sock:/var/run/docker.sock"]

  airlock-api:       # Airlock HTTP wrapper (NEW, build context: ./airlock)
    build: ./airlock
    ports: ["5001:5001"]
    env_file: .env
```

Each service keeps its own Dockerfile. The top-level compose orchestrates networking.

## 13. Nosana Integration Plan

1. **Orchestrator on Nosana:** AgentForge's existing nos_job_def deploys the
   orchestrator itself to a Nosana GPU node. This continues to work.

2. **Forge workers on Nosana:** Instead of running forge-api locally, deploy
   it as a Nosana job using the existing forge_job_definition.json. The
   orchestrator sends POST /api/run to the Nosana-deployed Forge URL.

3. **Airlock static analysis on Nosana:** Airlock already supports pointing
   NOSANA_ENDPOINT at a Nosana-hosted LLM for static code reading. AgentForge's
   NosanaManager can deploy a code-reading model and pass the URL to Airlock.

4. **GPU Market Selection:** Use AgentForge's existing market selection logic
   (cheapest available GPU with sufficient VRAM) for all Nosana deployments.

## 14. Communication Interfaces Between Components

### AgentForge → Forge API
- Protocol: HTTP REST
- Endpoint: POST http://forge-api:5000/api/run
- Request: `{ "problem_text": "...", "model": "...", "base_url": "...", "api_key": "..." }`
- Response: `{ "exit_status": "submitted", "has_submission": true, "submission_preview": "diff..." }`
- Auth: None initially (internal network); add FORGE_API_TOKEN later

### AgentForge → Airlock API (NEW)
- Protocol: HTTP REST
- Endpoint: POST http://airlock-api:5001/api/check
- Request: `{ "code": "...", "mode": "code" }` or `{ "package": "name" }`
- Response: `{ "verdict": "SAFE"|"BLOCK", "reasons": [...], "timings": {...} }`
- Implementation: New Python file wrapping gate.check() with FastAPI

### Forge → Docker (sandbox)
- Protocol: Docker socket (bollard crate)
- Creates containers from forge-sandbox image
- Executes bash commands, reads/writes files

### Frontend → Backend
- Protocol: Socket.IO (real-time) + HTTP REST
- Socket.IO path: /fleet/socket.io
- REST: /fleet/* (Fleet API), /api/* (ElizaOS)

## 15. Startup Sequence

1. Start forge-api (Rust, fast boot ~2s)
2. Start airlock-api (Python, fast boot ~3s)
3. Start AgentForge orchestrator (ElizaOS, slower boot ~15-30s)
   - ElizaOS initializes
   - Nosana plugin loads, connects to GPU network
   - Fleet API starts on port 3001
   - Nginx starts on port 3000
4. Frontend connects to Socket.IO on /fleet/socket.io
5. System ready for user input

## 16. Development Workflow

- **Option A (Docker Compose):** `docker compose up` starts everything
- **Option B (Individual):** Run each service separately in its own terminal
  - Terminal 1: `cd agentforge && bun run dev`
  - Terminal 2: `cd forge && cargo run -p forge-api`
  - Terminal 3: `cd airlock && python -m uvicorn api:app --port 5001` (new)
- **Testing:** Each project keeps its own test suite
  - AgentForge: `bun run test`
  - Forge: `cargo test`
  - Airlock: `python -m pytest tests/`

## 17. Production / Deployment Workflow

1. Build all Docker images
2. Push to container registry
3. Deploy orchestrator to Nosana GPU using updated nos_job_def
4. Orchestrator auto-deploys Forge and Airlock workers as needed
5. Alternatively: deploy entire stack via docker-compose.prod.yml on a VPS

---

*This plan is frozen. Do not modify source code until Phase 7 begins.*
