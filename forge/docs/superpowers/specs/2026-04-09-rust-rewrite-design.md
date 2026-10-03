# Forge — Rust Implementation — Design Spec

**Date:** 2026-04-09  
**Status:** Draft, validated against current SWEagent behavior and implementation notes  
**Source of truth today:** OkeyAmy previous `SWEagent` project implementation and test suite  
**Target state:** Side-by-side Rust implementation with production parity  
**Platform target:** Linux-first

---

## 1. Purpose and Status

This document describes the **proposed** Rust implementation architecture for the current `SWEagent` codebase.

SWE stands for **Software Engineering**. `SWEagent` in this repository is an AI software-engineering agent system for code editing, tooling, and runtime-environment workflows.

Important reality check:

- The repository is currently implemented in a non-Rust stack, not Rust.
- There is **no existing Rust workspace** in this repo yet.
- Any Rust crate layout in this document is a **design target**, not implemented reality.
- The authoritative behavior to preserve comes from the current codebase, its tests, and stable on-disk/output contracts.

This document should be read as a migration design, not as a description of code that already exists.

---

## 2. Current Implementation Reality

The current implementation is organized around these subsystems:

- `src/agent` — agent loop, model adapters, parsers, hooks, problem statements, history processors
- `src/environment` — Docker deployment, runtime abstraction, repo setup/reset, environment lifecycle
- `src/run` — CLI, single run, batch run, replay, run hooks, stats/progress helpers
- `src/tools` — command definitions, bundles, parsing helpers, registry, windowed file support
- `src/rules` — rules CLI and rule definitions
- `src/inspector` — trajectory inspector CLI/server
- `src/types.ts` — shared types including trajectory/history-related contracts
- `tests` — the strongest parity specification in the repo

The Rust implementation must preserve external behavior, but it should **not** mirror the current file layout line-by-line.

---

## 3. Constraints and Non-Goals

| Area | Decision |
|---|---|
| Migration shape | Build Rust side-by-side, cut over only after parity passes |
| Behavior source | Prefer current implementation behavior where coherent; use tests and actual outputs as contract |
| Partial areas | Do not treat incomplete implementations as authoritative |
| Tooling/runtime | Docker, repo handling, shell state, file IO, and tools must be real |
| Compatibility | Preserve CLI/config/env/tool/output contracts |
| Trajectories | Read legacy snake_case and camelCase variants; write one canonical format |
| Prompt text | Do not line-port prompt/template text unless explicitly required |
| Port strategy | Contract-first and test-first, not line-by-line source translation |

Non-goals:

- preserving legacy internal names or class hierarchy
- preserving legacy file layout
- assuming every README claim is backed by complete implementation

---

## 4. Authoritative Contract Surfaces

These are the compatibility surfaces the Rust implementation must preserve.

### 4.1 CLI surface

The Rust CLI must preserve the existing command surface and option names exposed by `src/run/cli.ts`, including the major commands:

- `run`
- `run-batch`
- `run-replay`
- `inspect`
- `quick-stats`
- `merge-preds`
- `compare-runs`
- `remove-unfinished`
- `extract-pred`
- `traj-to-demo`
- `shell`

Behavioral parity matters more than implementation strategy.

### 4.2 Config surface

Preserve the current config concepts and naming used by the existing schemas:

- run-single config
- run-batch config
- environment config
- repo config
- agent model/config surface
- problem statement config

The Rust implementation should accept the same YAML/JSON config shapes where those are already used by the current CLI and tests.

### 4.3 Environment and repo lifecycle

The Rust system must preserve the observable behavior of:

- Docker-backed execution
- repo provisioning/reset
- post-startup commands
- file read/write/upload behavior
- command execution with timeout and failure handling
- submission/patch extraction flow

This area is one of the highest-risk parts of the rewrite and should be implemented early.

### 4.4 Tool contracts

Preserve:

- tool command names
- tool markers and special strings
- parser selection behavior
- windowed file behavior
- environment registry behavior where tests rely on it

### 4.5 On-disk outputs

Preserve or intentionally normalize with backward-compatible readers:

- trajectory files
- predictions output
- replay inputs
- inspector-readable output

### 4.6 Test-observed behavior

When code, docs, and tests disagree, the parity target should prefer:

1. actual runtime behavior and persisted output
2. tests
3. existing schemas/code
4. README/docs claims

---

## 5. Current Gaps and Partial Areas

Not all parts of the current repository are equally authoritative.

Known partial or inconsistent areas:

- `src/run/run-shell.ts`:
  - `RunShell` exists
  - `runShellFromConfig()` is explicitly not implemented
- `src/run/batch-instances.ts`:
  - SWE-bench-related loading/normalization is partial
  - shape handling is inconsistent in places such as image field naming
- trajectory/info field naming is inconsistent across code/tests/consumers:
  - `exitStatus` vs `exit_status`
  - `modelStats` vs `model_stats`
- some README claims overstate completeness relative to the implementation

Implication: the Rust implementation should preserve the real contract surface, but it should **properly implement** incomplete areas rather than reproducing partial legacy behavior blindly.

---

## 6. Proposed Side-by-Side Rust Architecture

The Rust implementation should be introduced as a new workspace beside the current codebase, not as an in-place mutation of the existing source tree.

Proposed high-level layout:

```text
forge/
├── Cargo.toml
├── crates/
│   ├── forge-types/
│   ├── forge-tools/
│   ├── forge-model/
│   ├── forge-env/
│   ├── forge-agent/
│   ├── forge-run/
│   ├── forge-plugin/
│   └── forge/
├── tools/
└── tests/
```

This is a **proposed** workspace layout only.

### Crate intent

- `forge-types` — shared DTOs, serde types, error model, output contracts
- `forge-tools` — parsers, tool definitions, windowed file/editor behavior, helpers
- `forge-model` — model abstraction and provider adapters
- `forge-env` — Docker runtime, bash session management, repo handling, environment lifecycle
- `forge-agent` — agent loop, history processors, problem statements, retry/reviewer logic
- `forge-run` — CLI-facing orchestration, run-single, run-batch, replay, hooks, inspector integration
- `forge-plugin` — optional ElizaOS integration boundary
- `forge` — binary crate only

---

## 7. ElizaOS Integration Guidance

ElizaOS Rust support exists and is viable, but it should be treated as an integration option, not the primary behavioral spec.

Use ElizaOS where it helps with:

- runtime lifecycle
- plugin/service composition
- character/runtime setup
- optional multi-agent or integration scenarios

Do **not** let ElizaOS redefine the SWEagent compatibility target.

For this Rust implementation:

- the current repository remains the behavior source of truth
- ElizaOS should stay behind a narrow integration boundary
- the core run/tool/env/trajectory contracts should remain owned by Forge crates

If ElizaOS APIs change, the implementation should not collapse because the contract surface is preserved in Forge itself.

---

## 8. Migration Phases

Recommended order:

1. `forge-types`
   - shared types
   - error model
   - special tokens
   - trajectory/prediction DTOs
2. `forge-tools`
   - parsers
   - command schemas
   - windowed file behavior
   - tool registry contracts
3. `forge-env`
   - Docker runtime
   - bash session persistence
   - repo lifecycle
   - file IO / upload
4. `forge-model`
   - deterministic/replay/test models first
   - then provider-backed models
5. `forge-agent`
   - agent loop
   - history processors
   - problem statements
   - retry/reviewer logic
6. `forge-run`
   - run-single
   - replay
   - batch
   - hooks / stats / inspector wiring
7. `forge-plugin`
   - optional ElizaOS adapter layer
8. `forge`
   - final CLI surface and cutover wrapper

---

## 9. Parity and Compatibility Requirements

### 9.1 Trajectory compatibility

The Rust implementation must handle trajectory compatibility explicitly.

Requirements:

- read legacy files using either camelCase or snake_case where current readers/consumers effectively require tolerance
- base the canonical write format on the actual ecosystem of readers/consumers, not on assumptions
- ensure replay, stats, inspector, and downstream tools continue to work on rewritten outputs

### 9.2 Tool and marker compatibility

Exact strings matter for:

- submission markers
- retry markers
- forfeit markers
- tool output expected by tests and replay logic

### 9.3 Runtime fidelity

The Rust environment layer must preserve:

- persistent shell state where required
- correct exit codes and timeout behavior
- repo reset semantics
- startup command behavior
- file operations and uploads

### 9.4 Deterministic verification first

Before using live model providers, verify parity with deterministic paths:

- replay model
- fixed-output/test models
- fixture-based run/replay tests

---

## 10. Risks and Open Questions

### Risks

- Docker/bash session fidelity is easy to get subtly wrong
- trajectory and stats naming is inconsistent in current outputs
- incomplete areas can mislead implementation if treated as complete
- README/docs currently overstate some areas relative to implementation

### Open questions

- which on-disk format should be canonical on write once compatibility readers are in place?
- should ElizaOS integration ship in phase 1, or follow after core parity?
- should helper tools live as Rust binaries from the beginning, or be temporarily shimmed during migration?

---

## 11. What Is Not Ported Verbatim

- internal legacy class names
- legacy file layout
- prompt/template text unless explicitly needed
- incomplete legacy behavior as a goal in itself
- line-by-line source translation

The target is behavioral parity with a clean Rust implementation, not source resemblance.
