use forge_types::error::ForgeError;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

pub type ExtraFields = HashMap<String, serde_json::Value>;

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

#[async_trait::async_trait]
pub trait ProblemStatement: Send + Sync {
    fn id(&self) -> &str;

    async fn get_problem_statement(&self) -> Result<String, ForgeError>;

    fn get_extra_fields(&self) -> ExtraFields {
        HashMap::new()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

fn short_id(data: &[u8]) -> String {
    sha256_hex(data).chars().take(6).collect()
}

// ---------------------------------------------------------------------------
// EmptyProblemStatement
// ---------------------------------------------------------------------------

pub struct EmptyProblemStatement {
    id: String,
}

impl Default for EmptyProblemStatement {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
        }
    }
}

impl EmptyProblemStatement {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait::async_trait]
impl ProblemStatement for EmptyProblemStatement {
    fn id(&self) -> &str {
        &self.id
    }

    async fn get_problem_statement(&self) -> Result<String, ForgeError> {
        Ok(String::new())
    }
}

// ---------------------------------------------------------------------------
// TextProblemStatement
// ---------------------------------------------------------------------------

pub struct TextProblemStatement {
    pub text: String,
    pub extra_fields: ExtraFields,
    pub id: String,
}

impl TextProblemStatement {
    /// Create with an explicit id.
    pub fn new(text: impl Into<String>, extra_fields: ExtraFields, id: impl Into<String>) -> Self {
        let text = text.into();
        Self {
            id: id.into(),
            text,
            extra_fields,
        }
    }

    /// Create with an auto-generated id (sha256 of text, first 6 hex chars).
    pub fn from_text(text: impl Into<String>) -> Self {
        let text = text.into();
        let id = short_id(text.as_bytes());
        Self {
            id,
            text,
            extra_fields: HashMap::new(),
        }
    }
}

#[async_trait::async_trait]
impl ProblemStatement for TextProblemStatement {
    fn id(&self) -> &str {
        &self.id
    }

    async fn get_problem_statement(&self) -> Result<String, ForgeError> {
        Ok(self.text.clone())
    }

    fn get_extra_fields(&self) -> ExtraFields {
        self.extra_fields.clone()
    }
}

// ---------------------------------------------------------------------------
// FileProblemStatement
// ---------------------------------------------------------------------------

pub struct FileProblemStatement {
    pub path: PathBuf,
    pub extra_fields: ExtraFields,
    pub id: String,
}

impl FileProblemStatement {
    /// Create with explicit id (e.g., for testing or when file isn't ready yet).
    pub fn with_id(path: impl Into<PathBuf>, id: impl Into<String>, extra_fields: ExtraFields) -> Self {
        Self {
            id: id.into(),
            path: path.into(),
            extra_fields,
        }
    }

    /// Async constructor: computes id from file content.
    pub async fn from_path(path: impl Into<PathBuf>, extra_fields: ExtraFields) -> Result<Self, ForgeError> {
        let path = path.into();
        let contents = tokio::fs::read(&path).await.map_err(ForgeError::Io)?;
        let id = short_id(&contents);
        Ok(Self { path, extra_fields, id })
    }
}

#[async_trait::async_trait]
impl ProblemStatement for FileProblemStatement {
    fn id(&self) -> &str {
        &self.id
    }

    async fn get_problem_statement(&self) -> Result<String, ForgeError> {
        tokio::fs::read_to_string(&self.path).await.map_err(ForgeError::Io)
    }

    fn get_extra_fields(&self) -> ExtraFields {
        self.extra_fields.clone()
    }
}

// ---------------------------------------------------------------------------
// GithubIssueProblemStatement
// ---------------------------------------------------------------------------

pub struct GithubIssueProblemStatement {
    pub github_url: String,
    pub extra_fields: ExtraFields,
    pub id: String,
    owner: String,
    repo: String,
    number: u64,
    client: reqwest::Client,
}

impl GithubIssueProblemStatement {
    /// Parse `github.com/{owner}/{repo}/issues/{number}` and build the struct.
    pub fn from_url(github_url: impl Into<String>, extra_fields: ExtraFields) -> Result<Self, ForgeError> {
        let url = github_url.into();
        let (owner, repo, number) = parse_github_issue_url(&url)?;
        let id = format!("{}__{}-i{}", owner, repo, number);
        Ok(Self {
            github_url: url,
            extra_fields,
            id,
            owner,
            repo,
            number,
            client: reqwest::Client::new(),
        })
    }
}

/// Parse `https://github.com/{owner}/{repo}/issues/{number}` (or without scheme).
fn parse_github_issue_url(url: &str) -> Result<(String, String, u64), ForgeError> {
    // Strip protocol prefix if present.
    let stripped = url
        .trim_start_matches("https://")
        .trim_start_matches("http://");

    // Now expect: github.com/{owner}/{repo}/issues/{number}
    let rest = stripped
        .strip_prefix("github.com/")
        .ok_or_else(|| ForgeError::Config(format!("not a github.com URL: {}", url)))?;

    let parts: Vec<&str> = rest.split('/').collect();
    if parts.len() < 4 || parts[2] != "issues" {
        return Err(ForgeError::Config(format!(
            "cannot parse github issue URL: {}",
            url
        )));
    }

    let owner = parts[0].to_string();
    let repo = parts[1].to_string();
    let number: u64 = parts[3]
        .parse()
        .map_err(|_| ForgeError::Config(format!("invalid issue number in URL: {}", url)))?;

    Ok((owner, repo, number))
}

#[async_trait::async_trait]
impl ProblemStatement for GithubIssueProblemStatement {
    fn id(&self) -> &str {
        &self.id
    }

    async fn get_problem_statement(&self) -> Result<String, ForgeError> {
        let api_url = format!(
            "https://api.github.com/repos/{}/{}/issues/{}",
            self.owner, self.repo, self.number
        );

        let mut req = self.client
            .get(&api_url)
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "Forge");

        if let Ok(token) = std::env::var("GITHUB_TOKEN") {
            if !token.is_empty() {
                req = req.header("Authorization", format!("Bearer {}", token));
            }
        }

        let response = req.send().await.map_err(|e| ForgeError::Http(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ForgeError::Http(format!(
                "GitHub API returned status {}",
                response.status()
            )));
        }

        let json: serde_json::Value =
            response.json().await.map_err(|e| ForgeError::Http(e.to_string()))?;

        let title = json["title"].as_str().unwrap_or("").to_string();
        let body = json["body"].as_str().unwrap_or("").to_string();

        Ok(format!("{}\n\n{}", title, body))
    }

    fn get_extra_fields(&self) -> ExtraFields {
        self.extra_fields.clone()
    }
}

// ---------------------------------------------------------------------------
// AnyProblemStatement enum wrapper
// ---------------------------------------------------------------------------

pub enum AnyProblemStatement {
    Empty(EmptyProblemStatement),
    Text(TextProblemStatement),
    File(FileProblemStatement),
    GithubIssue(GithubIssueProblemStatement),
}

#[async_trait::async_trait]
impl ProblemStatement for AnyProblemStatement {
    fn id(&self) -> &str {
        match self {
            AnyProblemStatement::Empty(ps) => ps.id(),
            AnyProblemStatement::Text(ps) => ps.id(),
            AnyProblemStatement::File(ps) => ps.id(),
            AnyProblemStatement::GithubIssue(ps) => ps.id(),
        }
    }

    async fn get_problem_statement(&self) -> Result<String, ForgeError> {
        match self {
            AnyProblemStatement::Empty(ps) => ps.get_problem_statement().await,
            AnyProblemStatement::Text(ps) => ps.get_problem_statement().await,
            AnyProblemStatement::File(ps) => ps.get_problem_statement().await,
            AnyProblemStatement::GithubIssue(ps) => ps.get_problem_statement().await,
        }
    }

    fn get_extra_fields(&self) -> ExtraFields {
        match self {
            AnyProblemStatement::Empty(ps) => ps.get_extra_fields(),
            AnyProblemStatement::Text(ps) => ps.get_extra_fields(),
            AnyProblemStatement::File(ps) => ps.get_extra_fields(),
            AnyProblemStatement::GithubIssue(ps) => ps.get_extra_fields(),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn empty_problem_statement_has_uuid_id() {
        let ps = EmptyProblemStatement::new();
        assert!(!ps.id().is_empty());
        // Should be parseable as a UUID
        assert!(Uuid::parse_str(ps.id()).is_ok(), "id should be a valid UUID");
    }

    #[tokio::test]
    async fn empty_problem_statement_returns_empty_string() {
        let ps = EmptyProblemStatement::new();
        let text = ps.get_problem_statement().await.unwrap();
        assert!(text.is_empty());
    }

    #[tokio::test]
    async fn text_problem_statement_auto_id_is_6_hex_chars() {
        let ps = TextProblemStatement::from_text("hello world");
        assert_eq!(ps.id().len(), 6);
        // All hex chars
        assert!(ps.id().chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[tokio::test]
    async fn text_problem_statement_returns_text() {
        let ps = TextProblemStatement::from_text("my problem statement");
        let text = ps.get_problem_statement().await.unwrap();
        assert_eq!(text, "my problem statement");
    }

    #[tokio::test]
    async fn text_problem_statement_custom_id() {
        let ps = TextProblemStatement::new("text", HashMap::new(), "custom-id");
        assert_eq!(ps.id(), "custom-id");
    }

    #[tokio::test]
    async fn file_problem_statement_reads_file() {
        // Create a temp file.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("problem.txt");
        std::fs::write(&path, "my file problem").unwrap();

        let ps = FileProblemStatement::from_path(&path, HashMap::new()).await.unwrap();
        assert_eq!(ps.id().len(), 6);

        let text = ps.get_problem_statement().await.unwrap();
        assert_eq!(text, "my file problem");
    }

    #[test]
    fn parse_github_issue_url_with_https() {
        let (owner, repo, number) =
            parse_github_issue_url("https://github.com/SWE-agent/SWE-agent/issues/42").unwrap();
        assert_eq!(owner, "SWE-agent");
        assert_eq!(repo, "SWE-agent");
        assert_eq!(number, 42);
    }

    #[test]
    fn parse_github_issue_url_without_scheme() {
        let (owner, repo, number) =
            parse_github_issue_url("github.com/owner/repo/issues/123").unwrap();
        assert_eq!(owner, "owner");
        assert_eq!(repo, "repo");
        assert_eq!(number, 123);
    }

    #[test]
    fn parse_github_issue_url_invalid() {
        assert!(parse_github_issue_url("https://example.com/owner/repo/issues/1").is_err());
        assert!(parse_github_issue_url("github.com/owner/repo/pull/1").is_err());
    }

    #[test]
    fn github_issue_statement_id_format() {
        let ps = GithubIssueProblemStatement::from_url(
            "https://github.com/SWE-agent/SWE-agent/issues/42",
            HashMap::new(),
        )
        .unwrap();
        assert_eq!(ps.id(), "SWE-agent__SWE-agent-i42");
    }

    #[tokio::test]
    async fn any_problem_statement_empty() {
        let any = AnyProblemStatement::Empty(EmptyProblemStatement::new());
        let text = any.get_problem_statement().await.unwrap();
        assert!(text.is_empty());
        assert!(!any.id().is_empty());
    }

    #[tokio::test]
    async fn any_problem_statement_text() {
        let any = AnyProblemStatement::Text(TextProblemStatement::from_text("hello"));
        let text = any.get_problem_statement().await.unwrap();
        assert_eq!(text, "hello");
    }
}
