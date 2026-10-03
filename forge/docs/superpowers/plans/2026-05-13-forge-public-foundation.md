# Forge Public Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first public foundation for Forge: GitHub command parsing, job state types, GitHub comment rendering, an E2B runner boundary, and an API endpoint that can accept GitHub App webhook events.

**Architecture:** Keep public workflow contracts in `forge-types` so CLI, API, workers, and later compatibility layers share the same data model. Add E2B execution as a separate `forge-env` module with an explicit runner interface, while leaving the old Docker runner available for existing local paths until a later migration. Add an API route that parses incoming GitHub App webhook payloads and returns the planned Forge action without starting long-running jobs yet.

**Tech Stack:** Rust workspace, Axum API, Serde data models, Reqwest HTTP client, E2B command API boundary, existing Forge agent/model crates.

---

## File Structure

- Create `crates/forge-types/src/public_workflow.rs`: shared public product types, commands, job state, GitHub event shape, comment render models.
- Modify `crates/forge-types/src/lib.rs`: export `public_workflow`.
- Create `crates/forge-env/src/e2b.rs`: E2B runner client and command result mapping.
- Modify `crates/forge-env/src/lib.rs`: export `e2b`.
- Create `crates/forge-api/src/routes/github_app.rs`: endpoint for GitHub App webhook events.
- Modify `crates/forge-api/src/routes/mod.rs`: export route.
- Modify `crates/forge-api/src/main.rs`: register `POST /api/github/webhook`.
- Modify `crates/forge-api/Cargo.toml`: add `uuid` dependency if needed by route responses.
- Test with `cargo test --workspace` and `cargo check --workspace`.

## Task 1: Shared Public Workflow Types

**Files:**
- Create: `crates/forge-types/src/public_workflow.rs`
- Modify: `crates/forge-types/src/lib.rs`

- [ ] **Step 1: Add failing command parser tests**

Create `crates/forge-types/src/public_workflow.rs` with tests first:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForgeCommand {
    Plan,
    Approve,
    Status,
    Cancel,
    Review,
    Improve,
    Ask { question: String },
    Fix,
}

impl ForgeCommand {
    pub fn parse(input: &str) -> Option<Self> {
        let trimmed = input.trim();
        let rest = trimmed.strip_prefix("/forge")?.trim();
        if rest.is_empty() {
            return Some(Self::Plan);
        }
        let mut parts = rest.splitn(2, char::is_whitespace);
        let command = parts.next()?.to_ascii_lowercase();
        let arg = parts.next().unwrap_or("").trim().to_string();
        match command.as_str() {
            "plan" => Some(Self::Plan),
            "approve" => Some(Self::Approve),
            "status" => Some(Self::Status),
            "cancel" => Some(Self::Cancel),
            "review" => Some(Self::Review),
            "improve" => Some(Self::Improve),
            "ask" if !arg.is_empty() => Some(Self::Ask { question: arg }),
            "fix" => Some(Self::Fix),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_default_forge_command_as_plan() {
        assert_eq!(ForgeCommand::parse("/forge"), Some(ForgeCommand::Plan));
    }

    #[test]
    fn parse_issue_commands() {
        assert_eq!(ForgeCommand::parse("/forge plan"), Some(ForgeCommand::Plan));
        assert_eq!(ForgeCommand::parse("/forge approve"), Some(ForgeCommand::Approve));
        assert_eq!(ForgeCommand::parse("/forge status"), Some(ForgeCommand::Status));
        assert_eq!(ForgeCommand::parse("/forge cancel"), Some(ForgeCommand::Cancel));
    }

    #[test]
    fn parse_pr_commands() {
        assert_eq!(ForgeCommand::parse("/forge review"), Some(ForgeCommand::Review));
        assert_eq!(ForgeCommand::parse("/forge improve"), Some(ForgeCommand::Improve));
        assert_eq!(ForgeCommand::parse("/forge fix"), Some(ForgeCommand::Fix));
        assert_eq!(
            ForgeCommand::parse("/forge ask why did this fail?"),
            Some(ForgeCommand::Ask {
                question: "why did this fail?".to_string()
            })
        );
    }

    #[test]
    fn parse_rejects_unknown_or_empty_ask() {
        assert_eq!(ForgeCommand::parse("forge plan"), None);
        assert_eq!(ForgeCommand::parse("/forge deploy"), None);
        assert_eq!(ForgeCommand::parse("/forge ask"), None);
    }
}
```

- [ ] **Step 2: Run targeted test to verify compile baseline**

Run: `cargo test -p forge-types public_workflow`

Expected: tests compile and pass because the first step includes the minimal parser implementation. If this command fails because the module is not exported yet, continue to Step 3 and rerun.

- [ ] **Step 3: Export the module**

Modify `crates/forge-types/src/lib.rs`:

```rust
pub mod public_workflow;
```

Add this near the other `pub mod` lines.

- [ ] **Step 4: Expand public workflow data types**

Replace `crates/forge-types/src/public_workflow.rs` with the parser plus these public structs and enums:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForgeCommand {
    Plan,
    Approve,
    Status,
    Cancel,
    Review,
    Improve,
    Ask { question: String },
    Fix,
}

impl ForgeCommand {
    pub fn parse(input: &str) -> Option<Self> {
        let trimmed = input.trim();
        let rest = trimmed.strip_prefix("/forge")?.trim();
        if rest.is_empty() {
            return Some(Self::Plan);
        }
        let mut parts = rest.splitn(2, char::is_whitespace);
        let command = parts.next()?.to_ascii_lowercase();
        let arg = parts.next().unwrap_or("").trim().to_string();
        match command.as_str() {
            "plan" => Some(Self::Plan),
            "approve" => Some(Self::Approve),
            "status" => Some(Self::Status),
            "cancel" => Some(Self::Cancel),
            "review" => Some(Self::Review),
            "improve" => Some(Self::Improve),
            "ask" if !arg.is_empty() => Some(Self::Ask { question: arg }),
            "fix" => Some(Self::Fix),
            _ => None,
        }
    }

    pub fn requires_issue_context(&self) -> bool {
        matches!(self, Self::Plan | Self::Approve | Self::Status | Self::Cancel)
    }

    pub fn requires_pr_context(&self) -> bool {
        matches!(self, Self::Review | Self::Improve | Self::Ask { .. } | Self::Fix)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForgeJobState {
    Received,
    Planning,
    WaitingForApproval,
    Approved,
    Queued,
    Running,
    Verifying,
    BranchPushed,
    Failed,
    Cancelled,
    NeedsInput,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHubRepositoryRef {
    pub owner: String,
    pub name: String,
    pub full_name: String,
    pub default_branch: String,
    pub clone_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHubActor {
    pub login: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitHubSubject {
    Issue { number: u64, title: String, body: Option<String>, html_url: String },
    PullRequest { number: u64, title: String, body: Option<String>, html_url: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgeGitHubEvent {
    pub delivery_id: String,
    pub repository: GitHubRepositoryRef,
    pub actor: GitHubActor,
    pub subject: GitHubSubject,
    pub command: ForgeCommand,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgePlan {
    pub summary: String,
    pub proposed_change: String,
    pub checks: Vec<String>,
    pub risk: String,
    pub branch_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgeVerificationSummary {
    pub commands: Vec<ForgeCheckResult>,
    pub risks: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgeCheckResult {
    pub command: String,
    pub exit_code: i32,
    pub passed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_default_forge_command_as_plan() {
        assert_eq!(ForgeCommand::parse("/forge"), Some(ForgeCommand::Plan));
    }

    #[test]
    fn parse_issue_commands() {
        assert_eq!(ForgeCommand::parse("/forge plan"), Some(ForgeCommand::Plan));
        assert_eq!(ForgeCommand::parse("/forge approve"), Some(ForgeCommand::Approve));
        assert_eq!(ForgeCommand::parse("/forge status"), Some(ForgeCommand::Status));
        assert_eq!(ForgeCommand::parse("/forge cancel"), Some(ForgeCommand::Cancel));
    }

    #[test]
    fn parse_pr_commands() {
        assert_eq!(ForgeCommand::parse("/forge review"), Some(ForgeCommand::Review));
        assert_eq!(ForgeCommand::parse("/forge improve"), Some(ForgeCommand::Improve));
        assert_eq!(ForgeCommand::parse("/forge fix"), Some(ForgeCommand::Fix));
        assert_eq!(
            ForgeCommand::parse("/forge ask why did this fail?"),
            Some(ForgeCommand::Ask {
                question: "why did this fail?".to_string()
            })
        );
    }

    #[test]
    fn parse_rejects_unknown_or_empty_ask() {
        assert_eq!(ForgeCommand::parse("forge plan"), None);
        assert_eq!(ForgeCommand::parse("/forge deploy"), None);
        assert_eq!(ForgeCommand::parse("/forge ask"), None);
    }

    #[test]
    fn command_context_flags_are_stable() {
        assert!(ForgeCommand::Plan.requires_issue_context());
        assert!(ForgeCommand::Review.requires_pr_context());
        assert!(!ForgeCommand::Review.requires_issue_context());
    }
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p forge-types public_workflow`

Expected: all `public_workflow` tests pass.

## Task 2: GitHub Comment Renderer

**Files:**
- Create: `crates/forge-types/src/public_workflow.rs`

- [ ] **Step 1: Add renderer tests**

Append these tests inside the existing `#[cfg(test)] mod tests`:

```rust
#[test]
fn render_plan_comment_contains_approval_command() {
    let plan = ForgePlan {
        summary: "The login form accepts blank emails.".to_string(),
        proposed_change: "Add validation before submit.".to_string(),
        checks: vec!["npm test".to_string()],
        risk: "low".to_string(),
        branch_name: "forge/issue-12-login-validation".to_string(),
    };

    let rendered = render_plan_comment(&plan);

    assert!(rendered.contains("## Forge Plan"));
    assert!(rendered.contains("/forge approve"));
    assert!(rendered.contains("forge/issue-12-login-validation"));
}

#[test]
fn render_branch_comment_contains_checks_and_compare_url() {
    let verification = ForgeVerificationSummary {
        commands: vec![ForgeCheckResult {
            command: "cargo test --workspace".to_string(),
            exit_code: 0,
            passed: true,
        }],
        risks: vec!["No browser test was run.".to_string()],
    };

    let rendered = render_branch_ready_comment(
        "forge/issue-8-fix-api",
        "https://github.com/acme/app/compare/main...forge/issue-8-fix-api",
        &["src/api.rs".to_string()],
        &verification,
    );

    assert!(rendered.contains("## Forge Branch Ready"));
    assert!(rendered.contains("cargo test --workspace"));
    assert!(rendered.contains("https://github.com/acme/app/compare/main...forge/issue-8-fix-api"));
}
```

- [ ] **Step 2: Add renderer functions**

Add below the structs in `crates/forge-types/src/public_workflow.rs`:

```rust
pub fn render_plan_comment(plan: &ForgePlan) -> String {
    let checks = if plan.checks.is_empty() {
        "- No automatic checks configured.".to_string()
    } else {
        plan.checks
            .iter()
            .map(|check| format!("- `{check}`"))
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        "## Forge Plan\n\n\
         ### What I Think This Issue Needs\n{summary}\n\n\
         ### Proposed Change\n{proposed_change}\n\n\
         ### Checks I Will Run\n{checks}\n\n\
         ### Risk\n{risk}\n\n\
         ### Branch\n`{branch_name}`\n\n\
         ### Approve\nComment `/forge approve` to let Forge make changes in an E2B sandbox.",
        summary = plan.summary,
        proposed_change = plan.proposed_change,
        checks = checks,
        risk = plan.risk,
        branch_name = plan.branch_name
    )
}

pub fn render_branch_ready_comment(
    branch_name: &str,
    compare_url: &str,
    changed_files: &[String],
    verification: &ForgeVerificationSummary,
) -> String {
    let files = if changed_files.is_empty() {
        "- No changed files reported.".to_string()
    } else {
        changed_files
            .iter()
            .map(|file| format!("- `{file}`"))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let checks = if verification.commands.is_empty() {
        "- No verification commands were run.".to_string()
    } else {
        verification
            .commands
            .iter()
            .map(|check| {
                let status = if check.passed { "passed" } else { "failed" };
                format!("- `{}`: {} (exit {})", check.command, status, check.exit_code)
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let risks = if verification.risks.is_empty() {
        "- No known remaining risks.".to_string()
    } else {
        verification
            .risks
            .iter()
            .map(|risk| format!("- {risk}"))
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        "## Forge Branch Ready\n\n\
         ### Branch\n`{branch_name}`\n\n\
         Compare: {compare_url}\n\n\
         ### Changed Files\n{files}\n\n\
         ### Verification\n{checks}\n\n\
         ### Risks And Notes\n{risks}\n\n\
         ### Next Step\nOpen a pull request from `{branch_name}` when the diff looks right.",
    )
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p forge-types public_workflow`

Expected: renderer and parser tests pass.

## Task 3: E2B Runner Boundary

**Files:**
- Create: `crates/forge-env/src/e2b.rs`
- Modify: `crates/forge-env/src/lib.rs`
- Modify: `crates/forge-env/Cargo.toml`

- [ ] **Step 1: Add E2B request-building tests**

Create `crates/forge-env/src/e2b.rs`:

```rust
use forge_types::ForgeError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct E2bConfig {
    pub api_key: String,
    pub base_url: String,
    pub template: Option<String>,
    pub timeout_secs: u64,
}

impl E2bConfig {
    pub fn from_env() -> Result<Self, ForgeError> {
        let api_key = std::env::var("E2B_API_KEY")
            .map_err(|_| ForgeError::Config("E2B_API_KEY is required for E2B execution".into()))?;
        Ok(Self {
            api_key,
            base_url: std::env::var("E2B_BASE_URL")
                .unwrap_or_else(|_| "https://api.e2b.dev".to_string()),
            template: std::env::var("E2B_TEMPLATE").ok(),
            timeout_secs: std::env::var("E2B_TIMEOUT_SECS")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(1800),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct E2bCommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

#[derive(Debug, Clone)]
pub struct E2bRunner {
    config: E2bConfig,
    client: reqwest::Client,
}

impl E2bRunner {
    pub fn new(config: E2bConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    pub fn sandbox_create_payload(&self) -> serde_json::Value {
        let mut payload = serde_json::json!({
            "timeout": self.config.timeout_secs,
        });
        if let Some(template) = &self.config.template {
            payload["template"] = serde_json::Value::String(template.clone());
        }
        payload
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sandbox_create_payload_includes_timeout_and_template() {
        let runner = E2bRunner::new(E2bConfig {
            api_key: "test".to_string(),
            base_url: "https://api.e2b.dev".to_string(),
            template: Some("forge-node".to_string()),
            timeout_secs: 900,
        });

        let payload = runner.sandbox_create_payload();

        assert_eq!(payload["timeout"], 900);
        assert_eq!(payload["template"], "forge-node");
    }
}
```

- [ ] **Step 2: Add dependencies**

Modify `crates/forge-env/Cargo.toml` dependencies:

```toml
reqwest = { workspace = true }
```

- [ ] **Step 3: Export module**

Modify `crates/forge-env/src/lib.rs`:

```rust
pub mod e2b;
```

- [ ] **Step 4: Run targeted test**

Run: `cargo test -p forge-env e2b`

Expected: E2B payload test passes.

- [ ] **Step 5: Add command execution method shell**

Add to `impl E2bRunner`:

```rust
pub async fn run_command(
    &self,
    sandbox_id: &str,
    command: &str,
) -> Result<E2bCommandOutput, ForgeError> {
    if sandbox_id.trim().is_empty() {
        return Err(ForgeError::Config("sandbox_id cannot be empty".into()));
    }
    if command.trim().is_empty() {
        return Err(ForgeError::Config("command cannot be empty".into()));
    }

    let url = format!(
        "{}/sandboxes/{}/commands",
        self.config.base_url.trim_end_matches('/'),
        sandbox_id
    );
    let response = self
        .client
        .post(url)
        .bearer_auth(&self.config.api_key)
        .json(&serde_json::json!({ "command": command }))
        .send()
        .await
        .map_err(|e| ForgeError::Http(e.to_string()))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(ForgeError::Http(format!("E2B command failed {status}: {body}")));
    }

    response
        .json::<E2bCommandOutput>()
        .await
        .map_err(|e| ForgeError::Http(e.to_string()))
}
```

- [ ] **Step 6: Run package tests**

Run: `cargo test -p forge-env e2b`

Expected: E2B unit tests pass without network calls.

## Task 4: GitHub App Webhook Intake

**Files:**
- Create: `crates/forge-api/src/routes/github_app.rs`
- Modify: `crates/forge-api/src/routes/mod.rs`
- Modify: `crates/forge-api/src/main.rs`

- [ ] **Step 1: Add route module**

Create `crates/forge-api/src/routes/github_app.rs`:

```rust
use axum::http::StatusCode;
use axum::Json;
use forge_types::public_workflow::{
    ForgeCommand, ForgeGitHubEvent, ForgeJobState, GitHubSubject,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct GitHubAppWebhookRequest {
    pub delivery_id: String,
    pub repository: forge_types::public_workflow::GitHubRepositoryRef,
    pub actor: forge_types::public_workflow::GitHubActor,
    pub subject: GitHubSubject,
    pub comment_body: Option<String>,
    pub label_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GitHubAppWebhookResponse {
    pub accepted: bool,
    pub state: ForgeJobState,
    pub command: Option<ForgeCommand>,
    pub message: String,
}

pub async fn handler(
    Json(req): Json<GitHubAppWebhookRequest>,
) -> Result<Json<GitHubAppWebhookResponse>, (StatusCode, String)> {
    let command = resolve_command(&req)?;
    let event = ForgeGitHubEvent {
        delivery_id: req.delivery_id,
        repository: req.repository,
        actor: req.actor,
        subject: req.subject,
        command: command.clone(),
    };

    validate_command_context(&event)?;

    Ok(Json(GitHubAppWebhookResponse {
        accepted: true,
        state: ForgeJobState::Received,
        command: Some(command),
        message: "Forge event accepted".to_string(),
    }))
}

fn resolve_command(req: &GitHubAppWebhookRequest) -> Result<ForgeCommand, (StatusCode, String)> {
    if req.label_name.as_deref() == Some("forge") {
        return Ok(ForgeCommand::Plan);
    }
    if let Some(body) = &req.comment_body {
        if let Some(command) = ForgeCommand::parse(body) {
            return Ok(command);
        }
    }
    Err((
        StatusCode::BAD_REQUEST,
        "No supported Forge command found".to_string(),
    ))
}

fn validate_command_context(event: &ForgeGitHubEvent) -> Result<(), (StatusCode, String)> {
    match (&event.subject, &event.command) {
        (GitHubSubject::Issue { .. }, command) if command.requires_pr_context() => Err((
            StatusCode::BAD_REQUEST,
            "PR command cannot run on an issue".to_string(),
        )),
        (GitHubSubject::PullRequest { .. }, command) if command.requires_issue_context() => Err((
            StatusCode::BAD_REQUEST,
            "Issue command cannot run on a pull request".to_string(),
        )),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_types::public_workflow::{GitHubActor, GitHubRepositoryRef};

    fn request(comment_body: Option<&str>, label_name: Option<&str>, subject: GitHubSubject) -> GitHubAppWebhookRequest {
        GitHubAppWebhookRequest {
            delivery_id: "delivery-1".to_string(),
            repository: GitHubRepositoryRef {
                owner: "acme".to_string(),
                name: "app".to_string(),
                full_name: "acme/app".to_string(),
                default_branch: "main".to_string(),
                clone_url: "https://github.com/acme/app.git".to_string(),
            },
            actor: GitHubActor {
                login: "maintainer".to_string(),
            },
            subject,
            comment_body: comment_body.map(str::to_string),
            label_name: label_name.map(str::to_string),
        }
    }

    #[test]
    fn label_forge_resolves_to_plan() {
        let req = request(
            None,
            Some("forge"),
            GitHubSubject::Issue {
                number: 1,
                title: "Bug".to_string(),
                body: None,
                html_url: "https://github.com/acme/app/issues/1".to_string(),
            },
        );

        assert_eq!(resolve_command(&req).unwrap(), ForgeCommand::Plan);
    }

    #[test]
    fn issue_rejects_pr_review_command() {
        let event = ForgeGitHubEvent {
            delivery_id: "delivery-1".to_string(),
            repository: GitHubRepositoryRef {
                owner: "acme".to_string(),
                name: "app".to_string(),
                full_name: "acme/app".to_string(),
                default_branch: "main".to_string(),
                clone_url: "https://github.com/acme/app.git".to_string(),
            },
            actor: GitHubActor {
                login: "maintainer".to_string(),
            },
            subject: GitHubSubject::Issue {
                number: 1,
                title: "Bug".to_string(),
                body: None,
                html_url: "https://github.com/acme/app/issues/1".to_string(),
            },
            command: ForgeCommand::Review,
        };

        assert!(validate_command_context(&event).is_err());
    }
}
```

- [ ] **Step 2: Export the route**

Modify `crates/forge-api/src/routes/mod.rs`:

```rust
pub mod github_app;
```

- [ ] **Step 3: Register the route**

Modify `crates/forge-api/src/main.rs` router:

```rust
.route("/api/github/webhook", post(routes::github_app::handler))
```

Place it beside the existing `/api/run` route.

- [ ] **Step 4: Run route tests**

Run: `cargo test -p forge-api github_app`

Expected: route helper tests pass.

## Task 5: Workspace Verification

**Files:**
- No new files.

- [ ] **Step 1: Run full Rust tests**

Run: `cargo test --workspace`

Expected: all non-ignored tests pass. Docker-dependent ignored tests remain ignored.

- [ ] **Step 2: Run workspace check**

Run: `cargo check --workspace`

Expected: workspace check completes without errors.

- [ ] **Step 3: Inspect git status**

Run: `git status --short`

Expected changed files include only `.gitignore`, the design/plan docs, and the Phase 1 source files.

## Out Of Scope For This Plan

- Real E2B API contract validation with live credentials.
- GitHub request signature verification.
- Persistent job storage.
- Background queue execution.
- PR-Agent subprocess adapter.
- Branch push implementation.
- GitHub Action compatibility packaging.
