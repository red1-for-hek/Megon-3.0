# Baseline Test Results — Megon 3.0

This document records the baseline state of each project BEFORE any modifications.
Generated: 2026-10-03

---

## 1. AgentForge

### Install Command
```bash
cd agentforge
npm install -g bun
bun install          # uses bun.lock, falls back to resolving
patch-package        # runs automatically via postinstall
cd frontend && npm install
```

### Development Command
```bash
bun run dev
# Runs concurrently: "elizaos dev" + "cd frontend && npm run dev"
# Backend on :3000 (ElizaOS), Fleet API on :3001
# Frontend Vite dev server on :5173
```

### Build Command
```bash
bun run build         # tsc && cd frontend && npm run build
bun run build:worker  # tsc only (for worker container)
cd frontend && npm run build  # vite build → frontend/dist/
```

### Test Command
```bash
bun run test   # npx tsx tests/unit.test.ts
```

### Required Environment Variables
| Variable | Required | Purpose |
|----------|----------|--------|
| OPENAI_API_KEY | Yes | LLM API key (or "ollama" for local) |
| OPENAI_API_URL | Yes | LLM endpoint URL |
| MODEL_NAME | Yes | Model identifier |
| TAVILY_API_KEY | For researcher agents | Web search |
| NOSANA_API_KEY | For GPU deployment | Nosana network access |
| SERVER_PORT | No (default: 3000) | ElizaOS port |
| FLEET_API_PORT | No (default: 3001) | Fleet API port |

### Ports Used
- 3000: Nginx reverse proxy (production) / ElizaOS (dev)
- 3001: Fleet API (Express + Socket.IO)
- 3002: ElizaOS internal (production, behind nginx)
- 5173: Vite dev server (frontend development)

### Important Dependencies
- @elizaos/core ^1.0.0 (backend agent framework)
- @elizaos/cli ^1.0.0 (dev tooling)
- @nosana/kit ^2.2.4 (GPU network SDK)
- express ^5.2.1 (Fleet API)
- socket.io ^4.8.3 (real-time communication)
- react ^19.2.4 (frontend)
- @xyflow/react ^12.10.2 (DAG canvas)
- zustand ^5.0.12 (state management)
- vite ^8.0.1 (build tool)
- typescript ^5.0.0
- bun (runtime + package manager)

### Baseline Status
- **Install:** NOT RUN (requires bun install + npm install; skipped pending credentials)
- **Start:** NOT RUN (requires OPENAI_API_KEY, MODEL_NAME at minimum)
- **Missing for full test:** OPENAI_API_KEY, NOSANA_API_KEY, TAVILY_API_KEY
- **Known issues:** Worker Dockerfile references patches that may not exist in repo
  (@elizaos%2Fserver@1.7.2.patch is referenced but not found in worker/patches/)

---

## 2. Forge

### Install Command
```bash
cd forge
# Rust toolchain required (rustup)
cargo build --release           # builds all workspace crates
cargo build --release -p forge  # CLI binary only
cargo build --release -p forge-api  # API server only
```

### Development Command
```bash
# CLI mode (one-shot):
cargo run -- run --repo owner/repo --issue 42

# API mode:
cargo run -p forge-api
# Listens on FORGE_API_PORT (default 5000)

# Docker Compose (no Rust install needed):
docker compose up forge-api     # starts API server
docker compose run --rm forge   # one-shot fix
docker compose up watch         # continuous mode
```

### Build Command
```bash
cargo build --release
# Output: target/release/forge, target/release/forge-api

# Docker build:
docker build -t forge:latest .
docker build -t forge-api:latest --build-arg BIN=forge-api .
docker build -t forge-sandbox:latest -f Dockerfile.sandbox .
```

### Test Command
```bash
cargo test          # runs all workspace tests
cargo test -p forge # tests for specific crate
```

### Required Environment Variables
| Variable | Required | Purpose |
|----------|----------|--------|
| FORGE_MODEL | Yes | LLM model name |
| FORGE_BASE_URL | Yes | OpenAI-compatible API URL |
| FORGE_API_KEY | Yes | LLM API key |
| GITHUB_TOKEN | For GitHub ops | GitHub PAT for API access |
| FORGE_API_PORT | No (default: 5000) | HTTP API port |
| RUST_LOG | No (default: forge=warn) | Log verbosity |
| DOCKER_GID | No (default: 999) | Docker group ID for socket |
| E2B_API_KEY | For E2B sandboxes | Alternative to Docker sandbox |

### Ports Used
- 5000: forge-api HTTP server (configurable via FORGE_API_PORT)
- 80/443: Caddy reverse proxy (production only, docker-compose.prod.yml)

### Important Dependencies
- Rust workspace with 9 crates
- tokio 1 (async runtime)
- axum 0.8 (HTTP framework)
- bollard 0.18 (Docker SDK)
- reqwest 0.12 (HTTP client)
- serde/serde_json 1 (serialization)
- clap 4 (CLI parsing)
- tower-http 0.6 (CORS, tracing)
- sha2 0.10, hex 0.4 (hashing)

### Baseline Status
- **Install:** NOT RUN (Rust compilation requires ~5-10 min, needs libssl-dev)
- **Start:** NOT RUN (requires FORGE_MODEL, FORGE_BASE_URL, FORGE_API_KEY)
- **Missing for full test:** FORGE_MODEL, FORGE_BASE_URL, FORGE_API_KEY, GITHUB_TOKEN
- **Note:** Legacy /api/run endpoint requires FORGE_ENABLE_LEGACY_RUN_API=true
  (disabled by default in production config)
- **Docker socket:** forge-env/src/docker.rs connects to local Docker daemon;
  requires /var/run/docker.sock mount in containers

---

## 3. Airlock

### Install Command
```bash
cd airlock
pip install -r requirements.txt
# Dependencies: daytona>=0.198, openai>=1.40, python-dotenv>=1.0,
#               setuptools>=68, wheel>=0.40, certifi>=2024.2
```

### Development Command
```bash
# Single package check:
python -m airlock <package-name>

# Code snippet check:
python -m airlock --code "import os; os.system('curl evil.com')"

# Requirements file fan-out:
python -m airlock -r requirements.txt

# JSON output:
python -m airlock <package> --json

# Sponsor self-check:
python -m airlock --sponsors
```

### Build Command
```bash
# No build step — pure Python package
# Can be installed as a package:
pip install -e .
```

### Test Command
```bash
python -m pytest tests/
# Tests: test_hook.py, test_fanout.py
```

### Required Environment Variables
| Variable | Required | Purpose |
|----------|----------|--------|
| DAYTONA_API_KEY | Yes (for detonation) | Daytona sandbox creation |
| AIAND_API_KEY | Yes (for judgment) | ai& LLM API key |
| AIAND_BASE_URL | No (default: https://api.aiand.com/v1) | ai& endpoint |
| AIAND_MODEL | No (default: deepseek-v4-flash) | Judge model |
| NOSANA_ENDPOINT | Optional | Nosana-hosted code model for static read |
| NOSANA_MODEL | No (default: Qwen/Qwen2.5-Coder-7B-Instruct) | Static read model |
| DOUBLEWORD_API_KEY | Optional | Malware similarity matching |
| OXYLABS_USERNAME | Optional | Web reputation intel |
| OXYLABS_PASSWORD | Optional | Web reputation intel |

### Ports Used
- None (CLI tool, no HTTP server)
- **NOTE:** For Megon 3.0 integration, we need to add an HTTP wrapper on port 5001

### Important Dependencies
- daytona >= 0.198 (sandbox SDK)
- openai >= 1.40 (LLM client)
- python-dotenv >= 1.0 (env loading)
- certifi >= 2024.2 (CA bundle)

### Baseline Status
- **Install:** NOT RUN (pip install needed)
- **Start:** NOT RUN (requires DAYTONA_API_KEY at minimum for detonation)
- **Missing for full test:** DAYTONA_API_KEY, AIAND_API_KEY
- **Degraded mode possible:** Without NOSANA_ENDPOINT, falls back to ai& for
  static read. Without DOUBLEWORD_API_KEY, skips similarity matching.
  Without AIAND_API_KEY, falls back to tripwires only.
- **No HTTP API:** This is a gap — must create a FastAPI/Flask wrapper for
  Megon 3.0 integration.

---

## Summary Matrix

| Aspect | AgentForge | Forge | Airlock |
|--------|-----------|-------|---------|
| Language | TypeScript | Rust | Python |
| Runtime | Bun/Node 23 | Native binary | Python 3.11+ |
| Package Mgr | Bun + npm | Cargo | pip |
| Framework | ElizaOS v2 | Axum 0.8 | None (CLI) |
| Sandbox | Nosana GPU containers | Docker/E2B | Daytona |
| LLM Interface | @elizaos/plugin-openai | reqwest (OpenAI-compat) | openai Python SDK |
| HTTP API | Express + ElizaOS routes | Axum REST | **NONE** (needs wrapper) |
| WebSocket | Socket.IO | None | None |
| Database | SQLite (ElizaOS) | None (trajectory files) | None |
| Docker | Yes (node:23-slim) | Yes (multi-stage Rust) | No (uses Daytona) |
| Nosana | @nosana/kit SDK | Job definition JSON | OpenAI-compat endpoint |
| GitHub | Not integrated | Full integration | Not integrated |
| Tests | tsx unit tests | cargo test | pytest |
| Starts OK | Needs API keys | Needs API keys + Rust | Needs API keys + pip |
