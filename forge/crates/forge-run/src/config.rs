use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunConfig {
    #[serde(default)]
    pub agent: AgentConfigSerde,
    #[serde(default)]
    pub env: EnvConfigSerde,
    #[serde(default)]
    pub problem_statement: ProblemStatementConfigSerde,
    #[serde(default = "default_output_dir")]
    pub output_dir: String,
}

fn default_output_dir() -> String {
    "trajectories".to_string()
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            agent: AgentConfigSerde::default(),
            env: EnvConfigSerde::default(),
            problem_statement: ProblemStatementConfigSerde::default(),
            output_dir: default_output_dir(),
        }
    }
}

/// Serde-friendly agent config subset.
/// The full AgentConfig has Box<dyn> fields (history processors, parser),
/// so we use a simpler subset for YAML loading.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentConfigSerde {
    pub model_name: Option<String>,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub parser_type: Option<String>,
    pub max_steps: Option<u32>,
    pub max_requeries: Option<u32>,
    pub system_template: Option<String>,
    pub instance_template: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnvConfigSerde {
    pub image: Option<String>,
    pub container_name: Option<String>,
    pub repo_path: Option<String>,
    pub timeout_secs: Option<u64>,
    pub startup_commands: Option<Vec<String>>,
    pub env_vars: Option<Vec<(String, String)>>,
    pub base_commit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProblemStatementConfigSerde {
    Text { text: String },
    TextFile { path: PathBuf },
    GithubIssue { url: String },
    Empty,
}

impl Default for ProblemStatementConfigSerde {
    fn default() -> Self {
        Self::Empty
    }
}

impl RunConfig {
    pub fn from_yaml_file(path: &std::path::Path) -> Result<Self, forge_types::error::ForgeError> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            forge_types::error::ForgeError::Config(format!("cannot read config: {e}"))
        })?;
        serde_yaml::from_str(&content)
            .map_err(|e| forge_types::error::ForgeError::Config(format!("invalid YAML: {e}")))
    }

    pub fn from_yaml_str(s: &str) -> Result<Self, forge_types::error::ForgeError> {
        serde_yaml::from_str(s)
            .map_err(|e| forge_types::error::ForgeError::Config(format!("invalid YAML: {e}")))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = RunConfig::default();
        assert_eq!(config.output_dir, "trajectories");
        assert!(config.agent.model_name.is_none());
        assert!(config.env.image.is_none());
        assert!(matches!(config.problem_statement, ProblemStatementConfigSerde::Empty));
    }

    #[test]
    fn test_default_config_from_yaml_str_minimal() {
        let yaml = "output_dir: my_trajs";
        let config = RunConfig::from_yaml_str(yaml).unwrap();
        assert_eq!(config.output_dir, "my_trajs");
    }

    #[test]
    fn test_default_config_from_yaml_str_empty() {
        let yaml = "{}";
        let config = RunConfig::from_yaml_str(yaml).unwrap();
        assert_eq!(config.output_dir, "trajectories");
    }

    #[test]
    fn test_agent_config_serde_roundtrip() {
        let agent = AgentConfigSerde {
            model_name: Some("gemini-2.5-flash".into()),
            base_url: Some("https://api.example.com/v1".into()),
            api_key: Some("test-key".into()),
            parser_type: Some("thought_action".into()),
            max_steps: Some(50),
            max_requeries: Some(3),
            system_template: Some("You are a helpful agent.".into()),
            instance_template: Some("Solve: {problem_statement}".into()),
        };
        let json = serde_json::to_string(&agent).unwrap();
        let back: AgentConfigSerde = serde_json::from_str(&json).unwrap();
        assert_eq!(back.model_name, Some("gemini-2.5-flash".into()));
        assert_eq!(back.max_steps, Some(50));
        assert_eq!(back.base_url, Some("https://api.example.com/v1".into()));
    }

    #[test]
    fn test_env_config_serde_defaults() {
        let env = EnvConfigSerde::default();
        assert!(env.image.is_none());
        assert!(env.container_name.is_none());
        assert!(env.repo_path.is_none());
    }

    #[test]
    fn test_problem_statement_text_variant() {
        let yaml = r#"
type: text
text: "Fix the bug in the code"
"#;
        let ps: ProblemStatementConfigSerde = serde_yaml::from_str(yaml).unwrap();
        match ps {
            ProblemStatementConfigSerde::Text { text } => {
                assert_eq!(text, "Fix the bug in the code");
            }
            _ => panic!("expected Text variant"),
        }
    }

    #[test]
    fn test_problem_statement_github_issue_variant() {
        let yaml = r#"
type: github_issue
url: "https://github.com/owner/repo/issues/42"
"#;
        let ps: ProblemStatementConfigSerde = serde_yaml::from_str(yaml).unwrap();
        match ps {
            ProblemStatementConfigSerde::GithubIssue { url } => {
                assert_eq!(url, "https://github.com/owner/repo/issues/42");
            }
            _ => panic!("expected GithubIssue variant"),
        }
    }

    #[test]
    fn test_run_config_full_yaml() {
        let yaml = r#"
agent:
  model_name: "gemini-2.5-flash"
  base_url: "https://generativelanguage.googleapis.com/v1beta/openai"
  api_key: "test-api-key"
  max_steps: 75
env:
  image: "sweagent/swe-agent:latest"
  repo_path: "/repo"
  timeout_secs: 60
problem_statement:
  type: text
  text: "Fix the failing tests"
output_dir: "my_trajectories"
"#;
        let config = RunConfig::from_yaml_str(yaml).unwrap();
        assert_eq!(config.agent.model_name.as_deref(), Some("gemini-2.5-flash"));
        assert_eq!(config.agent.max_steps, Some(75));
        assert_eq!(config.env.image.as_deref(), Some("sweagent/swe-agent:latest"));
        assert_eq!(config.env.timeout_secs, Some(60));
        assert_eq!(config.output_dir, "my_trajectories");
        match config.problem_statement {
            ProblemStatementConfigSerde::Text { text } => {
                assert_eq!(text, "Fix the failing tests");
            }
            _ => panic!("expected Text variant"),
        }
    }
}
