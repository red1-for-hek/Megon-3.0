# Forge Phase 1 — Types & Tools Implementation Plan

**Date:** 2026-04-09  
**Status:** Draft, corrected against current TypeScript repo  
**Phase goal:** Establish the Rust implementation foundation for SWEagent shared contracts and tool behavior  
**Scope:** `forge-types` and `forge-tools` only

---

## 1. Purpose and Scope

This phase does **not** assume a Rust workspace already exists.

SWE stands for **Software Engineering**. In this repo, `SWEagent` means an AI software-engineering agent that works with code, tools, and environment/runtime workflows.

Its purpose is to create the first Rust foundation beside the current TypeScript implementation by defining:

- shared Rust types for compatibility-sensitive data
- error and marker handling
- parser contracts and parser implementations
- windowed file/tool behavior that downstream agent/run logic depends on

This phase should be driven by the actual TypeScript repo and test-observed behavior, not by speculative Rust-first structure.

---

## 2. Preconditions and Non-Goals

### Preconditions

Before implementation, treat these as already established:

- the repo is currently implemented in TypeScript
- the Rust implementation is side-by-side, not in-place
- CLI/config/tool/env/trajectory contracts are the parity target
- runtime/docker/repo/tool behavior must be real, not stubbed

### Non-goals for Phase 1

- no live model provider integration yet
- no full agent loop yet
- no run-single/run-batch implementation yet
- no shell-mode completion yet
- no assumption that ElizaOS integration is required in this phase

---

## 3. Authoritative TS Inputs

Phase 1 must be implemented against these current TS sources and contracts:

### Shared contracts

- `src/types.ts`
- `src/exceptions.ts`
- run/output-related types consumed by replay/stats/inspector paths

### Tool and parser behavior

- `src/agent/tools/parsing.ts`
- `src/tools/commands.ts`
- `src/tools/bundle.ts`
- `src/tools/registry.ts`
- `src/tools/windowed-file.ts`

### Contract verification via tests

- parser-related tests
- tool tests
- windowed-file tests
- replay / trajectory-consuming tests where types leak into phase-1 contracts

If the TS code and docs disagree, follow the real contracts used by code and tests.

---

## 4. Current TS Gaps and Partial Areas

These gaps matter even in Phase 1 because they affect what should be treated as authoritative.

- `src/run/run-shell.ts`
  - shell mode exists conceptually
  - `runShellFromConfig()` is not fully implemented
- `src/run/batch-instances.ts`
  - SWE-bench loading/normalization is partial
- trajectory consumers and producers are not perfectly consistent about field naming
  - support both `exitStatus` and `exit_status` on read
  - support both `modelStats` and `model_stats` on read if needed

Phase 1 should build tolerant readers and contract-focused types rather than codifying one accidental TS naming quirk as absolute truth.

---

## 5. Proposed Side-by-Side Phase 1 Scope

### Workspace intent

Create the beginning of a Rust workspace under a new top-level `forge/` directory.

Initial crates:

```text
forge/
├── Cargo.toml
└── crates/
    ├── forge-types/
    └── forge-tools/
```

### `forge-types` responsibilities

- shared serde types for phase-1-compatible data
- error definitions
- special tokens / markers
- trajectory-adjacent DTOs needed by parsers and tooling
- compatibility serializers/deserializers where legacy field variants matter

### `forge-tools` responsibilities

- parser trait and parser implementations
- parser selection/factory behavior
- command schema abstractions
- windowed file/editor behavior
- small tool-support utilities needed by later crates

---

## 6. TDD Workstreams

Phase 1 should be executed as focused workstreams, each validated by tests.

### Workstream A — workspace bootstrap

Deliverables:

- `forge/Cargo.toml`
- `forge/crates/forge-types`
- `forge/crates/forge-tools`
- minimal compilable crate skeletons

Acceptance:

- `cargo check` succeeds at workspace root

### Workstream B — errors and special tokens

Deliverables:

- Rust error model for phase-1 crates
- exact marker constants for submission/retry/forfeit handling

Acceptance:

- error display and classification tests pass
- exact marker string tests pass

### Workstream C — shared types and compatibility serde

Deliverables:

- history/message-compatible DTOs as needed by parsers and tool handling
- trajectory-adjacent DTOs needed in phase 1
- compatibility parsing for legacy field names where required

Acceptance:

- serde round-trip tests pass
- legacy read-compat tests pass for camelCase and snake_case variants where relevant

### Workstream D — parser layer

Deliverables:

- parser trait
- thought/action parser
- action-only parser
- XML-style parser
- function-calling parser
- bash-block parser(s)
- parser factory

Acceptance:

- parser tests cover strict/non-strict behavior
- parser outputs match current TS behavior on representative fixtures

### Workstream E — windowed file behavior

Deliverables:

- windowed file state model
- goto/scroll/range logic
- edit/replace/insert support as required by current behavior
- search behavior needed by existing TS tests/contracts

Acceptance:

- tests match TS windowing behavior closely enough for downstream parity

---

## 7. Compatibility and Parity Matrix

Phase 1 must preserve the following behavior-sensitive areas.

| Area | Requirement |
|---|---|
| Special markers | Exact byte-for-byte string compatibility |
| Parser behavior | Match current TS parser outputs for supported formats |
| Tool-call parsing | Preserve function-calling/tool-call semantics used today |
| Windowed file behavior | Preserve navigation/window semantics used by TS tests/tools |
| Legacy field reads | Accept both snake_case and camelCase where current ecosystem requires it |
| Future crate stability | `forge-types` and `forge-tools` should be reusable by `forge-env`, `forge-agent`, and `forge-run` without refactor churn |

---

## 8. Verification and Exit Criteria

Phase 1 is complete only when all of the following are true:

- workspace builds cleanly
- crate tests pass
- compatibility tests cover parser and marker behavior
- compatibility tests cover legacy field parsing where required
- windowed file behavior is exercised by real tests, not placeholders
- no dummy implementations are left in the exported phase-1 APIs

Recommended verification commands once implemented:

```bash
cargo check
cargo test -p forge-types
cargo test -p forge-tools
cargo test --workspace
```

---

## 9. Deferred Follow-Up Phases

Not in this phase, but downstream consumers of this work are already known:

- `forge-env`
  - Docker runtime
  - bash sessions
  - repo lifecycle
- `forge-model`
  - replay/test models first
  - provider-backed models later
- `forge-agent`
  - agent loop
  - problem statements
  - history processors
- `forge-run`
  - run-single, replay, batch, hooks, stats, inspector

This phase should make those later crates easier, not prematurely implement them.

---

## 10. Recommended First Files to Create

1. `forge/Cargo.toml`
2. `forge/crates/forge-types/Cargo.toml`
3. `forge/crates/forge-tools/Cargo.toml`
4. `forge/crates/forge-types/src/lib.rs`
5. `forge/crates/forge-types/src/error.rs`
6. `forge/crates/forge-types/src/special_tokens.rs`
7. `forge/crates/forge-types/src/history.rs`
8. `forge/crates/forge-types/src/trajectory.rs`
9. `forge/crates/forge-types/src/model_output.rs`
10. `forge/crates/forge-types/src/step.rs`
11. `forge/crates/forge-tools/src/lib.rs`
12. `forge/crates/forge-tools/src/parsers/mod.rs`
13. `forge/crates/forge-tools/src/parsers/thought_action.rs`
14. `forge/crates/forge-tools/src/windowed_file/mod.rs`

Create them only with tests and real contract intent in mind.
