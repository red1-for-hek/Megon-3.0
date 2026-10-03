# Forge Public E2B Product Design

## Summary

Forge will become an issue-first GitHub automation product that plans from an issue, waits for maintainer approval, executes the fix inside E2B, pushes a fix branch, and posts verification evidence back to GitHub.

The public product is not a generic PR reviewer and not a broad "AI developer" clone. Its wedge is trustable issue repair: Forge asks before changing code, runs in an isolated E2B sandbox, and shows proof before a maintainer decides what to merge.

## Target User

The first public audience is small teams and open-source maintainers who want a low-friction way to turn clear GitHub issues into reviewed fix branches. The first install path is a GitHub App backed by the hosted Forge API.

This audience values:

- Easy deployment without managing Docker on their own machine.
- Clear control before code changes happen.
- Branches and diffs they can inspect in normal GitHub workflows.
- Review comments and PR summaries that are useful but not noisy.
- Evidence: tests run, commands executed, files changed, known risks.

## Product Positioning

Existing AI review tools focus on reviewing pull requests. Existing coding agents focus on broad autonomous execution. Forge sits between those categories:

- Issue-first, not PR-first.
- Approval-gated by default, not fully autonomous by default.
- Branch-pushing first, not PR-opening first.
- E2B-only for public sandbox execution.
- PR-Agent-inspired PR review commands as a secondary workflow.

The product promise:

> Label an issue or ask Forge to plan. Forge proposes a fix, waits for approval, works in a cloud sandbox, pushes a branch, and reports exactly what it proved.

## Deployment Model

### MVP: GitHub App Trigger

The first deployment path is a GitHub App. The app receives GitHub webhooks and sends work to the hosted Forge API/worker. It does not run the heavy agent job inside GitHub infrastructure.

Responsibilities:

- Receive issue, issue comment, pull request, and pull request review/comment events.
- Parse Forge commands.
- Validate that a command is relevant to the current event.
- Verify GitHub webhook signatures before acting.
- Use installation-scoped credentials for repository reads, comments, and branch pushes.

### Hosted Forge API And Worker

The hosted service runs on infrastructure such as Vultr. It owns orchestration, state, logs, model calls, E2B sessions, GitHub comments, and branch publishing.

Responsibilities:

- Receive events from the GitHub App webhook route.
- Authenticate event payloads.
- Create and update jobs.
- Render GitHub comments.
- Run issue planning.
- Start E2B execution jobs after approval.
- Run PR review commands using PR-Agent architecture.
- Push branches through GitHub credentials.

### Later: GitHub Action Compatibility

A GitHub Action compatibility layer can be added later for self-hosted users, but it is not the first public install path.

## Command UX

Forge commands are GitHub comments.

Issue commands:

- `/forge plan`: analyze the issue and post a plan.
- `/forge approve`: approve the latest plan and start the E2B fix job.
- `/forge status`: show current job state and links to logs/evidence.
- `/forge cancel`: cancel queued or running work when possible.

PR commands:

- `/forge review`: run PR review using PR-Agent-style review behavior.
- `/forge improve`: generate improvement suggestions.
- `/forge ask <question>`: answer a question about the PR diff.
- `/forge fix`: non-MVP workflow for addressing review comments or failing CI on an existing PR branch.

Default behavior:

- Labeling an issue with `forge` is equivalent to `/forge plan`.
- Forge does not edit code until a maintainer comments `/forge approve`.
- Forge pushes a branch after successful execution instead of opening a PR in the MVP.

## Issue Workflow

1. A maintainer labels an issue `forge` or comments `/forge plan`.
2. The GitHub App webhook sends the event to Forge API.
3. Forge reads the issue title/body/comments and selected repository context.
4. Forge posts a plan comment containing:
   - interpreted problem statement
   - likely files or areas to inspect
   - proposed fix strategy
   - check/test strategy
   - expected branch name
   - risk level
   - any blocking questions
5. The maintainer comments `/forge approve`.
6. Forge starts an E2B sandbox.
7. Forge clones the repo and checks out the target branch.
8. Forge runs the coding agent loop inside E2B.
9. Forge runs configured verification commands.
10. Forge extracts the final diff.
11. Forge pushes a branch named like `forge/issue-123-short-title`.
12. Forge comments with:
    - branch link
    - changed files summary
    - verification commands and results
    - unresolved risks
    - next suggested command or manual next step

## PR Workflow

PR support is integrated, but secondary to issue fixing.

Forge reuses PR-Agent architecture for the first implementation rather than reimplementing its mature PR logic immediately.

MVP approach:

- Add a PR-Agent adapter behind Forge commands.
- For `/forge review`, `/forge improve`, and `/forge ask`, invoke PR-Agent behavior from the hosted worker.
- Normalize PR-Agent output through Forge's comment renderer when possible.
- Keep PR-Agent configuration and prompts as the reference for review behavior.

Later:

- Port only the necessary PR-Agent ideas into Forge-native Rust modules:
  - provider abstraction
  - diff compression
  - file filtering
  - persistent comments
  - inline suggestions
  - prompt/config structure

## E2B Sandbox Execution

Public Forge execution uses E2B only.

Required E2B capabilities:

- Create an isolated Linux sandbox per job.
- Run shell commands.
- Set environment variables for commands.
- Access the internet to clone repositories and install dependencies.
- Read and write files.
- Use templates for faster startup once the default environment is known.

Forge must not require mounting the host Docker socket in the public product.

Execution model:

- The hosted worker creates a sandbox for each approved issue job.
- The sandbox clones the repository using a scoped token.
- The agent executes commands through the E2B command API.
- Forge captures stdout, stderr, exit code, and command timing.
- Forge asks the sandbox for `git diff` and verification logs.
- Forge terminates the sandbox after diff extraction and branch publication.

## Branch Publishing

The MVP pushes a branch, not a PR.

Branch naming:

- `forge/issue-<number>-<slug>`

Branch contents:

- The agent's final code changes.
- No generated trajectory files committed into the target repo.
- No secrets or Forge internal metadata committed.

Forge comments include a compare link so maintainers can open a PR manually.

## State Model

Forge represents work as jobs.

Core job states:

- `received`
- `planning`
- `waiting_for_approval`
- `approved`
- `queued`
- `running`
- `verifying`
- `branch_pushed`
- `failed`
- `cancelled`
- `needs_input`

Each job stores:

- repository
- issue or PR reference
- triggering actor
- command
- plan
- approval actor
- branch name
- E2B sandbox id while active
- command logs
- verification summary
- final comment ids

## Comment Design

GitHub comments are the first public UI. They must be concise and polished.

Plan comment sections:

- `Forge Plan`
- `What I Think This Issue Needs`
- `Proposed Change`
- `Checks I Will Run`
- `Risk`
- `Approve`

Completion comment sections:

- `Forge Branch Ready`
- `Branch`
- `Changed Files`
- `Verification`
- `Risks And Notes`
- `Next Step`

Failure comment sections:

- `Forge Could Not Complete`
- `Where It Failed`
- `Relevant Logs`
- `What You Can Try Next`

## Security And Trust Boundaries

Public execution must be conservative.

Rules:

- No code changes before `/forge approve`.
- E2B is the only public code execution sandbox.
- GitHub tokens must be scoped as narrowly as possible.
- Model provider keys are stored in hosted Forge infrastructure, not in user repositories unless a self-hosted path is added later.
- Forge verifies GitHub webhook signatures before accepting events.
- Forge uses GitHub App installation tokens for repository actions.
- Forge validates repository identity and command actor permissions before acting.
- Forge does not expose arbitrary API execution endpoints without authentication.
- Forge does not let external users choose arbitrary sandbox images in public mode.
- Forge redacts secrets from logs where possible.

## Configuration

Repository config lives in `.forge.yml`.

Initial fields:

```yaml
forge:
  issue_label: forge
  default_branch: main
  approval_required: true
  branch_prefix: forge
  checks:
    - cargo test --workspace
  pr:
    review_enabled: true
    improve_enabled: true
```

If no config exists, Forge uses safe defaults:

- approval required
- no destructive commands
- branch push only
- infer checks from repo files when possible: Cargo uses `cargo test --workspace`, Node with a package test script uses the package manager test command, and unknown stacks get no automatic check until configured

## Non-Goals For MVP

- No billing.
- No multi-provider Git support beyond GitHub.
- No GitHub Action install flow yet.
- No fully autonomous mode by default.
- No automatic PR creation by default.
- No public arbitrary API endpoint for running jobs.
- No deep Rust rewrite of all PR-Agent internals in the first pass.

## Acceptance Criteria

The MVP is usable when:

- A GitHub App webhook can send issue and PR comment events to Forge API.
- `/forge plan` posts a useful issue plan.
- `/forge approve` starts an E2B execution job.
- The E2B job can clone a repo, run commands, produce a diff, and return logs.
- Forge can push a branch for an approved issue.
- Forge posts a completion comment with branch link and verification evidence.
- `/forge review` can run through the PR-Agent adapter and post a PR review comment.
- Job failures produce clear comments instead of silent failures.

## Implementation Sequence

1. Define Forge job, command, and GitHub event types.
2. Add hosted Forge API endpoints for GitHub App webhook events.
3. Add GitHub comment renderer for plan, status, success, and failure.
4. Add E2B runner module and map existing environment operations onto E2B command execution.
5. Add issue planning workflow.
6. Add approval workflow and branch publisher.
7. Add PR-Agent adapter for `/forge review`.
8. Add repository config loading.
9. Add smoke tests and GitHub App setup documentation.
