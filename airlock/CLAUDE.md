# CLAUDE.md

Behavioral guidelines to reduce common LLM coding mistakes. Merge with project-specific instructions as needed.

**Tradeoff:** These guidelines bias toward caution over speed. For trivial tasks, use judgment.

## 1. Think Before Coding

**Don't assume. Don't hide confusion. Surface tradeoffs.**

Before implementing:
- State your assumptions explicitly. If uncertain, ask.
- If multiple interpretations exist, present them - don't pick silently.
- If a simpler approach exists, say so. Push back when warranted.
- If something is unclear, stop. Name what's confusing. Ask.

## 2. Simplicity First

**Minimum code that solves the problem. Nothing speculative.**

- No features beyond what was asked.
- No abstractions for single-use code.
- No "flexibility" or "configurability" that wasn't requested.
- No error handling for impossible scenarios.
- If you write 200 lines and it could be 50, rewrite it.

Ask yourself: "Would a senior engineer say this is overcomplicated?" If yes, simplify.

## 3. Surgical Changes

**Touch only what you must. Clean up only your own mess.**

When editing existing code:
- Don't "improve" adjacent code, comments, or formatting.
- Don't refactor things that aren't broken.
- Match existing style, even if you'd do it differently.
- If you notice unrelated dead code, mention it - don't delete it.

When your changes create orphans:
- Remove imports/variables/functions that YOUR changes made unused.
- Don't remove pre-existing dead code unless asked.

The test: Every changed line should trace directly to the user's request.

## 4. Goal-Driven Execution

**Define success criteria. Loop until verified.**

Transform tasks into verifiable goals:
- "Add validation" → "Write tests for invalid inputs, then make them pass"
- "Fix the bug" → "Write a test that reproduces it, then make it pass"
- "Refactor X" → "Ensure tests pass before and after"

For multi-step tasks, state a brief plan:
```
1. [Step] → verify: [check]
2. [Step] → verify: [check]
3. [Step] → verify: [check]
```

Strong success criteria let you loop independently. Weak criteria ("make it work") require constant clarification.

---

**These guidelines are working if:** fewer unnecessary changes in diffs, fewer rewrites due to overcomplication, and clarifying questions come before implementation rather than after mistakes.

---

## Project context

**Airlock** — a safety gate for AI agents: detonate a package in a Daytona sandbox, read the source (Nosana), match it against known malware (Doubleword), check its reputation (Oxylabs), let ai& judge it → block malicious installs. For the Daytona HackSprint (NUS, 18 Jul 2026).

Pointers (read these; don't restate their content here):
- **`DESIGN.md`** — design source of truth. Conventions legend + Terms glossary up top. Locked: enforcement is a **hook**, not MCP (§5); Python/pip only; simplicity first.
- **`DESIGN.md` §9** — the build checklist. Work top-down (must-have → extras); flip `- [ ]` → `- [x]` as each step is built *and* verified.
- **`README.md`** — setup, how to run (`python -m airlock <pkg>`), and the `check()` contract the hook consumes.
- **`.env`** — API keys, git-ignored (`.env.example` is the template).
- **Code:** `airlock/sandbox/` (detonation), `airlock/analysis/` (judge / read / reputation), `airlock/gate.py` (orchestrator).
