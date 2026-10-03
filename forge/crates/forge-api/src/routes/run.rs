use axum::http::StatusCode;
use axum::Json;
use forge_run::config::{AgentConfigSerde, EnvConfigSerde, ProblemStatementConfigSerde, RunConfig};
use forge_run::run_single::RunSingle;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

/// Request body for POST /api/run.
///
/// All fields are optional — unset fields fall back to environment variables
/// (FORGE_MODEL, FORGE_BASE_URL, FORGE_API_KEY) the same way the CLI does.
#[derive(Debug, Deserialize)]
pub struct RunRequest {
    /// GitHub issue URL  e.g. "https://github.com/owner/repo/issues/42"
    pub github_url: Option<String>,
    /// owner/repo + issue number as an alternative to github_url
    pub repo: Option<String>,
    pub issue: Option<u64>,
    /// Plain-text problem statement (alternative to GitHub issue)
    pub problem_text: Option<String>,
    /// Model name override
    pub model: Option<String>,
    /// Base URL override
    pub base_url: Option<String>,
    /// API key override
    pub api_key: Option<String>,
    /// Sandbox Docker image override
    pub image: Option<String>,
    /// Max agent steps (default: 100)
    pub max_steps: Option<u32>,
    /// Directory for trajectory output (default: "trajectories")
    pub output_dir: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RunResponse {
    pub exit_status: Option<String>,
    pub has_submission: bool,
    pub submission_preview: Option<String>,
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

/// POST /api/run
///
/// Kicks off the forge agent on a GitHub issue or a plain-text problem.
/// Returns when the agent finishes (this may take several minutes).
pub async fn handler(
    Json(req): Json<RunRequest>,
) -> Result<Json<RunResponse>, (StatusCode, String)> {
    if std::env::var("FORGE_ENABLE_LEGACY_RUN_API").as_deref() != Ok("true") {
        return Err((
            StatusCode::NOT_FOUND,
            "Legacy /api/run is disabled. Use the GitHub App webhook flow.".to_string(),
        ));
    }

    let config = build_run_config(req)?;

    let run = RunSingle::from_run_config(config)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let result = run
        .run()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let submission_preview = result.info.submission.as_deref().map(|s| {
        let chars: String = s.chars().take(500).collect();
        if s.len() > 500 {
            format!("{chars}…")
        } else {
            chars
        }
    });

    Ok(Json(RunResponse {
        exit_status: result.info.exit_status,
        has_submission: result.info.submission.is_some(),
        submission_preview,
    }))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn build_run_config(req: RunRequest) -> Result<RunConfig, (StatusCode, String)> {
    let problem_statement = match (req.github_url, req.repo, req.issue, req.problem_text) {
        (Some(url), _, _, _) => ProblemStatementConfigSerde::GithubIssue { url },
        (_, Some(repo), Some(issue), _) => {
            let url = format!("https://github.com/{}/issues/{}", repo, issue);
            ProblemStatementConfigSerde::GithubIssue { url }
        }
        (_, _, _, Some(text)) => ProblemStatementConfigSerde::Text { text },
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                "Provide github_url, repo+issue, or problem_text".to_string(),
            ))
        }
    };

    let config = RunConfig {
        agent: AgentConfigSerde {
            model_name: req.model,
            base_url: req.base_url,
            api_key: req.api_key,
            max_steps: req.max_steps,
            ..Default::default()
        },
        env: EnvConfigSerde {
            image: req.image,
            ..Default::default()
        },
        problem_statement,
        output_dir: req.output_dir.unwrap_or_else(|| "trajectories".to_string()),
    };

    Ok(config)
}
