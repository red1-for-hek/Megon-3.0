use forge_run::config::{AgentConfigSerde, EnvConfigSerde, ProblemStatementConfigSerde, RunConfig};
use forge_run::run_single::RunSingle;
use forge_types::error::ForgeError;
use serde::{Deserialize, Serialize};

/// Parameters for the solve_issue action (JSON-deserializable from ElizaOS message content).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SolveIssueParams {
    /// GitHub issue URL, e.g. "https://github.com/owner/repo/issues/123"
    pub github_url: Option<String>,
    /// Direct problem text
    pub problem_text: Option<String>,
    /// Docker image to use (default: "sweagent/swe-agent:latest")
    pub docker_image: Option<String>,
    /// Model name
    pub model_name: Option<String>,
    /// Model base URL (OpenAI-compatible endpoint)
    pub base_url: Option<String>,
    /// API key
    pub api_key: Option<String>,
    /// Output directory for trajectories
    pub output_dir: Option<String>,
}


/// Result returned by the solve_issue action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolveIssueResult {
    pub success: bool,
    pub exit_status: Option<String>,
    pub submission: Option<String>,
    pub trajectory_path: Option<String>,
    pub error: Option<String>,
}

/// ElizaOS-compatible action handler for solving GitHub issues or text problems.
///
/// Usage (standalone, without ElizaOS runtime):
/// ```no_run
/// # use forge_plugin::{SolveIssueAction, SolveIssueParams};
/// # async fn example() {
/// let action = SolveIssueAction::new();
/// let params = SolveIssueParams {
///     github_url: Some("https://github.com/owner/repo/issues/1".into()),
///     ..Default::default()
/// };
/// let result = action.handle(params).await.unwrap();
/// # }
/// ```
pub struct SolveIssueAction;

impl SolveIssueAction {
    pub fn new() -> Self {
        Self
    }

    /// Core handler — builds RunConfig from params and runs the agent.
    pub async fn handle(&self, params: SolveIssueParams) -> Result<SolveIssueResult, ForgeError> {
        let problem = if let Some(url) = params.github_url {
            ProblemStatementConfigSerde::GithubIssue { url }
        } else if let Some(text) = params.problem_text {
            ProblemStatementConfigSerde::Text { text }
        } else {
            return Err(ForgeError::Config(
                "SolveIssueParams: provide github_url or problem_text".into(),
            ));
        };

        let config = RunConfig {
            agent: AgentConfigSerde {
                model_name: params.model_name,
                base_url: params.base_url,
                api_key: params.api_key,
                ..Default::default()
            },
            env: EnvConfigSerde {
                image: params.docker_image,
                ..Default::default()
            },
            problem_statement: problem,
            output_dir: params.output_dir.unwrap_or_else(|| "trajectories".into()),
        };

        let run = RunSingle::from_run_config(config)?;
        match run.run().await {
            Ok(result) => Ok(SolveIssueResult {
                success: true,
                exit_status: result.info.exit_status,
                submission: result.info.submission,
                trajectory_path: None, // filled by caller if needed
                error: None,
            }),
            Err(e) => Ok(SolveIssueResult {
                success: false,
                exit_status: None,
                submission: None,
                trajectory_path: None,
                error: Some(e.to_string()),
            }),
        }
    }
}

impl Default for SolveIssueAction {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_params_default() {
        let p = SolveIssueParams::default();
        assert!(p.github_url.is_none());
        assert!(p.problem_text.is_none());
        assert!(p.docker_image.is_none());
        assert!(p.model_name.is_none());
        assert!(p.base_url.is_none());
        assert!(p.api_key.is_none());
        assert!(p.output_dir.is_none());
    }

    #[test]
    fn test_params_serialize_roundtrip() {
        let p = SolveIssueParams {
            github_url: Some("https://github.com/test/repo/issues/1".into()),
            model_name: Some("gemini-2.5-flash".into()),
            ..Default::default()
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: SolveIssueParams = serde_json::from_str(&json).unwrap();
        assert_eq!(
            back.github_url,
            Some("https://github.com/test/repo/issues/1".into())
        );
        assert_eq!(back.model_name, Some("gemini-2.5-flash".into()));
        assert!(back.problem_text.is_none());
    }

    #[tokio::test]
    async fn test_handle_missing_params() {
        let action = SolveIssueAction::new();
        let result = action.handle(SolveIssueParams::default()).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("github_url or problem_text"));
    }

    #[test]
    fn test_solve_issue_result_serialize() {
        let result = SolveIssueResult {
            success: true,
            exit_status: Some("submitted".into()),
            submission: Some("diff --git a/file.py ...".into()),
            trajectory_path: Some("/trajs/issue.traj".into()),
            error: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("submitted"));
        assert!(json.contains("success"));

        let back: SolveIssueResult = serde_json::from_str(&json).unwrap();
        assert!(back.success);
        assert_eq!(back.exit_status, Some("submitted".into()));
    }

    #[test]
    fn test_solve_issue_result_error_case() {
        let result = SolveIssueResult {
            success: false,
            exit_status: None,
            submission: None,
            trajectory_path: None,
            error: Some("Docker not available".into()),
        };
        assert!(!result.success);
        assert!(result.error.is_some());
        assert!(result.submission.is_none());
    }

    #[test]
    fn test_action_default_and_new_are_equivalent() {
        // Both constructors should produce a valid SolveIssueAction
        let _a1 = SolveIssueAction::new();
        let _a2 = SolveIssueAction::default();
    }

    #[test]
    fn test_params_full_construction() {
        let p = SolveIssueParams {
            github_url: None,
            problem_text: Some("Fix the failing test in test_foo.py".into()),
            docker_image: Some("sweagent/swe-agent:latest".into()),
            model_name: Some("gemini-2.5-flash".into()),
            base_url: Some("https://generativelanguage.googleapis.com/v1beta/openai".into()),
            api_key: Some("AIzaSy_test".into()),
            output_dir: Some("my_trajs".into()),
        };
        assert!(p.github_url.is_none());
        assert_eq!(p.problem_text.as_deref(), Some("Fix the failing test in test_foo.py"));
        assert_eq!(p.docker_image.as_deref(), Some("sweagent/swe-agent:latest"));
        assert_eq!(p.output_dir.as_deref(), Some("my_trajs"));
    }
}
