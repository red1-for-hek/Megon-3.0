# Licenses and Attribution — Megon 3.0

Megon 3.0 is a composite project integrating three open-source repositories.
This document records provenance, licensing, and attribution for each component.

---

## Megon 3.0 Overall License

**License:** MIT License

Copyright (c) 2026 Megon 3.0 Contributors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

> **Rationale:** All three upstream projects are MIT-licensed (or compatible).
> The MIT license permits combining, modifying, and relicensing under MIT.

---

## Component Attribution

### 1. AgentForge (Orchestrator / UI / Nosana Integration)

- **Source:** https://github.com/Andy00L/agent-challenge-4
- **License:** MIT License — Copyright (c) Atai Barkai
- **Role in Megon 3.0:** Main orchestrator, multi-agent DAG workflow, React
  frontend, Nosana GPU fleet management, ElizaOS plugin system, Fleet API,
  Socket.IO communication layer.
- **Components reused:**
  - `src/plugins/nosana/` — Full Nosana integration (NosanaManager, WorkerClient,
    MissionOrchestrator, media services)
  - `frontend/` — React 19 + Vite + ReactFlow UI
  - `worker/` — ElizaOS worker container deployed to Nosana GPUs
  - `nos_job_def/` — Nosana job definitions
  - `src/index.ts` — ElizaOS project entrypoint with dynamic agent creation
  - `nginx.conf`, `entrypoint.sh` — Production reverse proxy setup
  - `characters/` — Agent character definitions
- **Original license preserved at:** `agentforge/LICENSE`

### 2. Forge (Autonomous Coding Agent)

- **Source:** https://github.com/OkeyAmy/forge
- **License:** MIT License — Copyright (c) 2025 Shaw Walters and ElizaOS Contributors
- **Additional attribution:** Port of SWE-agent. Copyright (c) 2024 John Yang,
  Carlos E. Jimenez, Alexander Wettig, Shunyu Yao, Karthik Narasimhan, Ofir Press.
- **Role in Megon 3.0:** Autonomous coding agent — GitHub issue resolution, code
  modification, patch/diff generation, sandboxed Docker execution, trajectory
  recording, REST API for CI/CD integration.
- **Components reused:**
  - `crates/forge-agent/` — Core SWE-agent loop
  - `crates/forge-env/` — Docker/E2B sandbox environment
  - `crates/forge-model/` — LLM interface (OpenAI-compatible, Anthropic)
  - `crates/forge-tools/` — Tool parsers (XML, function calling, thought-action)
  - `crates/forge-types/` — Shared types (trajectories, steps, history)
  - `crates/forge-run/` — Single/batch run orchestration
  - `crates/forge-api/` — Axum HTTP server (POST /api/run, webhooks)
  - `crates/forge-plugin/` — ElizaOS plugin adapter
  - `nos_job_def/` — Nosana job definition
  - `Dockerfile.sandbox` — Sandbox container image
- **Original license preserved at:** `forge/LICENSE`

### 3. Airlock (Security Verification Layer)

- **Source:** https://github.com/Wnayar/airlock
- **License:** No explicit LICENSE file found. Created for Daytona HackSprint
  (NUS Singapore, 18 Jul 2026). Treated as available for integration under MIT
  terms based on hackathon context. Will update if clarified.
- **Role in Megon 3.0:** Security verification — sandbox detonation (Daytona),
  static code analysis (Nosana GPU), LLM security judgment (ai&), malware
  similarity matching (Doubleword), reputation checking (Oxylabs/PyPI/OSV).
- **Components reused:**
  - `airlock/gate.py` — Main orchestrator: check(package) → Verdict
  - `airlock/sandbox/` — Daytona sandbox detonation, tripwire evaluation
  - `airlock/analysis/` — Static read, LLM judge, Doubleword, Oxylabs
  - `airlock/hook.py` — PreToolUse hook for pip install interception
  - `airlock/types.py` — Verdict, Event, StaticReport dataclasses
  - `airlock/config.py` — Configuration loading
  - `airlock/cli.py` — Command-line interface
  - `airlock/fanout.py` — Parallel package checking
- **Original design doc preserved at:** `airlock/DESIGN.md`

---

## Sponsor / Service Attribution

| Service | Integrated By | Purpose |
|---------|--------------|--------|
| Nosana | AgentForge, Forge, Airlock | Decentralized GPU compute |
| ElizaOS | AgentForge, Forge | AI agent framework |
| Daytona | Airlock | Disposable sandbox environments |
| ai& | Airlock | LLM-based security judgment |
| Doubleword | Airlock | Malware similarity embeddings |
| Oxylabs | Airlock | Web intelligence / reputation |
| Tavily | AgentForge | Web search API |

---

## How to Comply

1. Preserve this file and original LICENSE files in agentforge/ and forge/.
2. Include copyright notices above in any distribution.
3. If you fork Megon 3.0, maintain attribution to all three upstream projects.
4. The SWE-agent attribution (Princeton) must be preserved per Forge's license.
