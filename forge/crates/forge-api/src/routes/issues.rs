use axum::extract::Query;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Query params / response types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct IssuesQuery {
    /// GitHub repository in "owner/repo" format (required)
    pub repo: String,
    /// Optional label filter
    pub label: Option<String>,
    /// Max issues to return (default: 30, max: 100)
    #[serde(default = "default_limit")]
    pub limit: u32,
}

fn default_limit() -> u32 {
    30
}

#[derive(Debug, Serialize, Deserialize)]
struct GitHubLabel {
    name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct GitHubIssue {
    number: u64,
    title: String,
    html_url: String,
    labels: Vec<GitHubLabel>,
}

#[derive(Debug, Serialize)]
pub struct IssuesResponse {
    pub repo: String,
    pub count: usize,
    pub issues: Vec<IssueItem>,
}

#[derive(Debug, Serialize)]
pub struct IssueItem {
    pub number: u64,
    pub title: String,
    pub url: String,
    pub labels: Vec<String>,
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

/// GET /api/issues?repo=owner/repo&label=bug&limit=30
///
/// Lists open GitHub issues for the given repository.
/// Reads GITHUB_TOKEN from the environment for authentication.
pub async fn handler(
    Query(params): Query<IssuesQuery>,
) -> Result<Json<IssuesResponse>, (StatusCode, String)> {
    let client = build_github_client();
    let per_page = params.limit.min(100);

    let mut url = format!(
        "https://api.github.com/repos/{}/issues?state=open&per_page={}",
        params.repo, per_page
    );
    if let Some(ref label) = params.label {
        url.push_str(&format!("&labels={}", label));
    }

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("GitHub request failed: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err((
            StatusCode::BAD_GATEWAY,
            format!("GitHub API returned {status}: {body}"),
        ));
    }

    let raw: Vec<GitHubIssue> = resp
        .json()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("JSON parse error: {e}")))?;

    let issues: Vec<IssueItem> = raw
        .into_iter()
        .map(|i| IssueItem {
            number: i.number,
            title: i.title,
            url: i.html_url,
            labels: i.labels.into_iter().map(|l| l.name).collect(),
        })
        .collect();

    let count = issues.len();
    Ok(Json(IssuesResponse {
        repo: params.repo,
        count,
        issues,
    }))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn build_github_client() -> reqwest::Client {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::ACCEPT,
        "application/vnd.github.v3+json".parse().unwrap(),
    );
    headers.insert(
        reqwest::header::USER_AGENT,
        "Forge/0.1".parse().unwrap(),
    );
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        if !token.is_empty() {
            if let Ok(val) = format!("Bearer {}", token).parse() {
                headers.insert(reqwest::header::AUTHORIZATION, val);
            }
        }
    }
    reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .expect("failed to build HTTP client")
}
