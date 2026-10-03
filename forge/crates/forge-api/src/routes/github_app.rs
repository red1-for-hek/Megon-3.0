use axum::body::Bytes;
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use forge_types::ForgeError;
use forge_types::public_workflow::{
    render_plan_comment, ForgeCommand, ForgeGitHubEvent, ForgeJobState, ForgePlan,
    GitHubActor, GitHubRepositoryRef, GitHubSubject,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Serialize)]
pub struct GitHubAppWebhookResponse {
    pub accepted: bool,
    pub state: ForgeJobState,
    pub command: ForgeCommand,
    pub message: String,
}

pub async fn handler(
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<GitHubAppWebhookResponse>, (StatusCode, String)> {
    verify_signature_if_configured(&headers, &body)?;

    let event_name = header_value(&headers, "x-github-event")?;
    let delivery_id = header_value(&headers, "x-github-delivery")?;
    let event = normalize_webhook_event(event_name, delivery_id, &body)?;

    validate_command_context(&event)?;
    let comment_id = post_github_comment_for_event(&event)
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

    Ok(Json(GitHubAppWebhookResponse {
        accepted: true,
        state: ForgeJobState::Received,
        command: event.command,
        message: if let Some(comment_id) = comment_id {
            format!("Forge GitHub App event accepted and comment {comment_id} posted")
        } else {
            "Forge GitHub App event accepted; GitHub App credentials not configured for posting"
                .to_string()
        },
    }))
}

fn header_value<'a>(headers: &'a HeaderMap, name: &str) -> Result<&'a str, (StatusCode, String)> {
    headers
        .get(name)
        .ok_or_else(|| (StatusCode::BAD_REQUEST, format!("missing {name} header")))?
        .to_str()
        .map_err(|_| (StatusCode::BAD_REQUEST, format!("invalid {name} header")))
}

fn verify_signature_if_configured(
    headers: &HeaderMap,
    body: &[u8],
) -> Result<(), (StatusCode, String)> {
    let Ok(secret) = std::env::var("GITHUB_WEBHOOK_SECRET") else {
        return Ok(());
    };
    if secret.is_empty() {
        return Ok(());
    }

    let signature = header_value(headers, "x-hub-signature-256")?;
    let expected = format!("sha256={}", hmac_sha256_hex(secret.as_bytes(), body));
    if constant_time_eq(signature.as_bytes(), expected.as_bytes()) {
        Ok(())
    } else {
        Err((StatusCode::UNAUTHORIZED, "invalid GitHub webhook signature".to_string()))
    }
}

fn hmac_sha256_hex(key: &[u8], message: &[u8]) -> String {
    const BLOCK_SIZE: usize = 64;

    let mut normalized_key = if key.len() > BLOCK_SIZE {
        Sha256::digest(key).to_vec()
    } else {
        key.to_vec()
    };
    normalized_key.resize(BLOCK_SIZE, 0);

    let mut outer = [0x5c_u8; BLOCK_SIZE];
    let mut inner = [0x36_u8; BLOCK_SIZE];
    for (idx, byte) in normalized_key.iter().enumerate() {
        outer[idx] ^= byte;
        inner[idx] ^= byte;
    }

    let mut inner_hash = Sha256::new();
    inner_hash.update(inner);
    inner_hash.update(message);
    let inner_result = inner_hash.finalize();

    let mut outer_hash = Sha256::new();
    outer_hash.update(outer);
    outer_hash.update(inner_result);
    hex::encode(outer_hash.finalize())
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right.iter())
        .fold(0_u8, |acc, (a, b)| acc | (a ^ b))
        == 0
}

fn normalize_webhook_event(
    event_name: &str,
    delivery_id: &str,
    body: &[u8],
) -> Result<ForgeGitHubEvent, (StatusCode, String)> {
    match event_name {
        "issues" => normalize_issues_event(delivery_id, body),
        "issue_comment" => normalize_issue_comment_event(delivery_id, body),
        _ => Err((
            StatusCode::BAD_REQUEST,
            format!("unsupported GitHub event: {event_name}"),
        )),
    }
}

fn normalize_issues_event(
    delivery_id: &str,
    body: &[u8],
) -> Result<ForgeGitHubEvent, (StatusCode, String)> {
    let payload: IssuesWebhookPayload = serde_json::from_slice(body)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid issues payload: {e}")))?;

    if payload.action != "labeled" || payload.label.name != "forge" {
        return Err((
            StatusCode::BAD_REQUEST,
            "issues webhook did not contain a forge label action".to_string(),
        ));
    }

    Ok(ForgeGitHubEvent {
        delivery_id: delivery_id.to_string(),
        installation_id: payload.installation.map(|installation| installation.id),
        repository: payload.repository.into(),
        actor: payload.sender.into(),
        subject: payload.issue.into_subject(),
        command: ForgeCommand::Plan,
    })
}

fn normalize_issue_comment_event(
    delivery_id: &str,
    body: &[u8],
) -> Result<ForgeGitHubEvent, (StatusCode, String)> {
    let payload: IssueCommentWebhookPayload = serde_json::from_slice(body).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("invalid issue_comment payload: {e}"),
        )
    })?;

    if payload.action != "created" {
        return Err((
            StatusCode::BAD_REQUEST,
            "issue_comment webhook action is not created".to_string(),
        ));
    }

    let command = ForgeCommand::parse(&payload.comment.body).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "comment does not contain a supported Forge command".to_string(),
        )
    })?;

    Ok(ForgeGitHubEvent {
        delivery_id: delivery_id.to_string(),
        installation_id: payload.installation.map(|installation| installation.id),
        repository: payload.repository.into(),
        actor: payload.sender.into(),
        subject: payload.issue.into_subject(),
        command,
    })
}

async fn post_github_comment_for_event(event: &ForgeGitHubEvent) -> Result<Option<u64>, ForgeError> {
    let Some(installation_id) = event.installation_id else {
        return Ok(None);
    };
    let Ok(config) = crate::github_app_auth::GitHubAppConfig::from_env() else {
        return Ok(None);
    };

    let (number, title) = match &event.subject {
        GitHubSubject::Issue { number, title, .. } => (*number, title.as_str()),
        GitHubSubject::PullRequest { number, title, .. } => (*number, title.as_str()),
    };

    let body = match &event.command {
        ForgeCommand::Plan => render_plan_comment(&ForgePlan {
            summary: format!(
                "Forge received this issue and will inspect `{title}` before changing code."
            ),
            proposed_change:
                "A detailed code plan will be generated by the worker before execution."
                    .to_string(),
            checks: vec![],
            risk: "pending repo inspection".to_string(),
            branch_name: format!("forge/issue-{number}"),
        }),
        command => format!(
            "## Forge\n\nReceived `{command:?}`. This command is queued for Forge's worker."
        ),
    };

    let repo_name = event
        .repository
        .full_name
        .split_once('/')
        .map(|(_, repo)| repo)
        .unwrap_or(event.repository.name.as_str());
    let client = crate::github_app_auth::GitHubAppClient::new(config);
    let comment = client
        .post_issue_comment(
            installation_id,
            &event.repository.owner,
            repo_name,
            number,
            &body,
        )
        .await?;

    Ok(Some(comment.id))
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

#[derive(Debug, Deserialize)]
struct IssuesWebhookPayload {
    action: String,
    installation: Option<GitHubInstallationPayload>,
    repository: GitHubRepositoryPayload,
    issue: GitHubIssuePayload,
    label: GitHubLabelPayload,
    sender: GitHubUserPayload,
}

#[derive(Debug, Deserialize)]
struct IssueCommentWebhookPayload {
    action: String,
    installation: Option<GitHubInstallationPayload>,
    repository: GitHubRepositoryPayload,
    issue: GitHubIssuePayload,
    comment: GitHubCommentPayload,
    sender: GitHubUserPayload,
}

#[derive(Debug, Deserialize)]
struct GitHubInstallationPayload {
    id: u64,
}

#[derive(Debug, Deserialize)]
struct GitHubRepositoryPayload {
    name: String,
    full_name: String,
    default_branch: String,
    clone_url: String,
    owner: GitHubUserPayload,
}

impl From<GitHubRepositoryPayload> for GitHubRepositoryRef {
    fn from(value: GitHubRepositoryPayload) -> Self {
        Self {
            owner: value.owner.login,
            name: value.name,
            full_name: value.full_name,
            default_branch: value.default_branch,
            clone_url: value.clone_url,
        }
    }
}

#[derive(Debug, Deserialize)]
struct GitHubIssuePayload {
    number: u64,
    title: String,
    body: Option<String>,
    html_url: String,
    pull_request: Option<serde_json::Value>,
}

impl GitHubIssuePayload {
    fn into_subject(self) -> GitHubSubject {
        if self.pull_request.is_some() {
            GitHubSubject::PullRequest {
                number: self.number,
                title: self.title,
                body: self.body,
                html_url: self.html_url,
            }
        } else {
            GitHubSubject::Issue {
                number: self.number,
                title: self.title,
                body: self.body,
                html_url: self.html_url,
            }
        }
    }
}

#[derive(Debug, Deserialize)]
struct GitHubLabelPayload {
    name: String,
}

#[derive(Debug, Deserialize)]
struct GitHubCommentPayload {
    body: String,
}

#[derive(Debug, Deserialize)]
struct GitHubUserPayload {
    login: String,
}

impl From<GitHubUserPayload> for GitHubActor {
    fn from(value: GitHubUserPayload) -> Self {
        Self { login: value.login }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn repo_json() -> serde_json::Value {
        json!({
            "name": "app",
            "full_name": "acme/app",
            "default_branch": "main",
            "clone_url": "https://github.com/acme/app.git",
            "owner": { "login": "acme" }
        })
    }

    fn issue_json() -> serde_json::Value {
        json!({
            "number": 1,
            "title": "Bug",
            "body": "Fix it",
            "html_url": "https://github.com/acme/app/issues/1"
        })
    }

    fn pull_request_issue_json() -> serde_json::Value {
        json!({
            "number": 2,
            "title": "Feature",
            "body": "Review it",
            "html_url": "https://github.com/acme/app/pull/2",
            "pull_request": { "url": "https://api.github.com/repos/acme/app/pulls/2" }
        })
    }

    #[test]
    fn issues_labeled_forge_normalizes_to_plan_event() {
        let payload = json!({
            "action": "labeled",
            "repository": repo_json(),
            "issue": issue_json(),
            "label": { "name": "forge" },
            "sender": { "login": "maintainer" }
        });

        let event = normalize_webhook_event(
            "issues",
            "delivery-1",
            serde_json::to_vec(&payload).unwrap().as_slice(),
        )
        .unwrap();

        assert_eq!(event.command, ForgeCommand::Plan);
        assert_eq!(event.actor.login, "maintainer");
        assert!(matches!(event.subject, GitHubSubject::Issue { number: 1, .. }));
    }

    #[test]
    fn issue_comment_on_pull_request_accepts_review_command() {
        let payload = json!({
            "action": "created",
            "repository": repo_json(),
            "issue": pull_request_issue_json(),
            "comment": { "body": "/forge review" },
            "sender": { "login": "maintainer" }
        });

        let event = normalize_webhook_event(
            "issue_comment",
            "delivery-2",
            serde_json::to_vec(&payload).unwrap().as_slice(),
        )
        .unwrap();

        validate_command_context(&event).unwrap();
        assert_eq!(event.command, ForgeCommand::Review);
        assert!(matches!(event.subject, GitHubSubject::PullRequest { number: 2, .. }));
    }

    #[test]
    fn issue_comment_on_issue_rejects_review_command() {
        let payload = json!({
            "action": "created",
            "repository": repo_json(),
            "issue": issue_json(),
            "comment": { "body": "/forge review" },
            "sender": { "login": "maintainer" }
        });

        let event = normalize_webhook_event(
            "issue_comment",
            "delivery-3",
            serde_json::to_vec(&payload).unwrap().as_slice(),
        )
        .unwrap();

        assert!(validate_command_context(&event).is_err());
    }

    #[test]
    fn hmac_signature_matches_github_header_format() {
        let signature = hmac_sha256_hex(b"secret", b"payload");
        assert_eq!(
            signature,
            "b82fcb791acec57859b989b430a826488ce2e479fdf92326bd0a2e8375a42ba4"
        );
    }
}
