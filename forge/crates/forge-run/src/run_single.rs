use std::path::PathBuf;
use std::sync::Arc;

use forge_agent::agent::{AgentConfig, DefaultAgent};
use forge_agent::problem_statement::{
    AnyProblemStatement, EmptyProblemStatement, FileProblemStatement,
    GithubIssueProblemStatement, TextProblemStatement,
};
use forge_env::environment::{EnvironmentConfig, SweEnvironment};
use forge_model::openai_compat::{OpenAICompatConfig, OpenAICompatModel};
use forge_model::traits::SharedModel;
use forge_types::error::ForgeError;

use crate::config::{AgentConfigSerde, EnvConfigSerde, ProblemStatementConfigSerde, RunConfig};

pub use forge_agent::agent::AgentRunResult;

pub struct RunSingle {
    pub agent_config: AgentConfig,
    /// Model credentials retained for deferred model construction.
    model_name: Option<String>,
    base_url: Option<String>,
    api_key: Option<String>,
    pub env_config: EnvironmentConfig,
    pub problem: AnyProblemStatement,
    pub output_dir: PathBuf,
}

impl RunSingle {
    /// Build a `RunSingle` from a `RunConfig`.
    pub fn from_run_config(config: RunConfig) -> Result<Self, ForgeError> {
        let agent_config = build_agent_config(&config.agent)?;
        let mut env_config = build_env_config(&config.env);
        let problem = build_problem_statement(&config.problem_statement)?;
        let output_dir = PathBuf::from(&config.output_dir);

        // When using a GitHub issue with no custom startup_commands, auto-derive
        // the clone + git config + submit script so users don't have to write them.
        if env_config.startup_commands.is_empty() {
            if let ProblemStatementConfigSerde::GithubIssue { url } = &config.problem_statement {
                if let Some((owner, repo)) = extract_github_owner_repo(url) {
                    let repo_path = env_config.repo_path.clone();
                    let clone_url = format!("https://github.com/{}/{}", owner, repo);
                    env_config.startup_commands = vec![
                        format!("git clone --depth 1 {} {}", clone_url, repo_path),
                        format!(
                            "git -C {rp} config user.email forge@forge.local && git -C {rp} config user.name Forge",
                            rp = repo_path
                        ),
                        format!(
                            "printf '#!/bin/sh\\ncd {rp} && git add -A 2>/dev/null && git -c color.diff=false diff --cached\\n' > /usr/local/bin/submit && chmod +x /usr/local/bin/submit",
                            rp = repo_path
                        ),
                    ];
                }
            }
        }

        Ok(Self {
            agent_config,
            model_name: config.agent.model_name,
            base_url: config.agent.base_url,
            api_key: config.agent.api_key,
            env_config,
            problem,
            output_dir,
        })
    }

    /// Run the agent: create environment, run the agent loop, clean up.
    pub async fn run(self) -> Result<AgentRunResult, ForgeError> {
        // Build model (resolves from config fields or env vars)
        let model = build_model(
            self.model_name.as_deref(),
            self.base_url.as_deref(),
            self.api_key.as_deref(),
        )?;

        // Create output directory if needed
        if !self.output_dir.exists() {
            tokio::fs::create_dir_all(&self.output_dir)
                .await
                .map_err(ForgeError::Io)?;
        }

        // Create and start the environment
        let mut env = SweEnvironment::create(self.env_config).await?;

        // Run the agent
        let mut agent = DefaultAgent::new(self.agent_config, model);
        let result = agent
            .run(&mut env, &self.problem, Some(&self.output_dir))
            .await;

        // Always clean up the container
        let _ = env.close().await;

        result
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn build_agent_config(serde_cfg: &AgentConfigSerde) -> Result<AgentConfig, ForgeError> {
    let mut cfg = AgentConfig::default();
    if let Some(ref parser) = serde_cfg.parser_type {
        cfg.parser_type = parser.clone();
    }
    if let Some(steps) = serde_cfg.max_steps {
        cfg.max_steps = steps;
    }
    if let Some(requeries) = serde_cfg.max_requeries {
        cfg.max_requeries = requeries;
    }
    if let Some(ref tmpl) = serde_cfg.system_template {
        cfg.system_template = tmpl.clone();
    }
    if let Some(ref tmpl) = serde_cfg.instance_template {
        cfg.instance_template = tmpl.clone();
    }
    Ok(cfg)
}

fn build_env_config(serde_cfg: &EnvConfigSerde) -> EnvironmentConfig {
    let mut cfg = EnvironmentConfig::default();
    if let Some(ref image) = serde_cfg.image {
        cfg.image = image.clone();
    }
    if let Some(ref name) = serde_cfg.container_name {
        cfg.container_name = Some(name.clone());
    }
    if let Some(ref path) = serde_cfg.repo_path {
        cfg.repo_path = path.clone();
    }
    if let Some(secs) = serde_cfg.timeout_secs {
        cfg.timeout_secs = secs;
    }
    if let Some(ref cmds) = serde_cfg.startup_commands {
        cfg.startup_commands = cmds.clone();
    }
    if let Some(ref vars) = serde_cfg.env_vars {
        cfg.env_vars = vars.clone();
    }
    if let Some(ref commit) = serde_cfg.base_commit {
        cfg.base_commit = Some(commit.clone());
    }
    cfg
}

fn build_problem_statement(
    ps_config: &ProblemStatementConfigSerde,
) -> Result<AnyProblemStatement, ForgeError> {
    match ps_config {
        ProblemStatementConfigSerde::Empty => {
            Ok(AnyProblemStatement::Empty(EmptyProblemStatement::new()))
        }
        ProblemStatementConfigSerde::Text { text } => Ok(AnyProblemStatement::Text(
            TextProblemStatement::from_text(text),
        )),
        ProblemStatementConfigSerde::TextFile { path } => {
            // Use with_id since we can't do async here; id will be based on the filename.
            let id = format!(
                "file-{}",
                path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
            );
            Ok(AnyProblemStatement::File(FileProblemStatement::with_id(
                path,
                id,
                Default::default(),
            )))
        }
        ProblemStatementConfigSerde::GithubIssue { url } => {
            let ps = GithubIssueProblemStatement::from_url(url, Default::default())?;
            Ok(AnyProblemStatement::GithubIssue(ps))
        }
    }
}

// ---------------------------------------------------------------------------
// Public helper: build a SharedModel
// ---------------------------------------------------------------------------

/// Build a `SharedModel` from explicit values with env var fallbacks.
///
/// Priority per field: explicit value > env var.
/// Env vars: `FORGE_MODEL`, `FORGE_BASE_URL`, `FORGE_API_KEY`.
pub fn build_model(
    model_name: Option<&str>,
    base_url: Option<&str>,
    api_key: Option<&str>,
) -> Result<SharedModel, ForgeError> {
    let model_name = model_name
        .map(|s| s.to_string())
        .or_else(|| std::env::var("FORGE_MODEL").ok())
        .ok_or_else(|| {
            ForgeError::Config(
                "model name not set. Provide agent.model_name in config or FORGE_MODEL env var."
                    .into(),
            )
        })?;

    let base_url = base_url
        .map(|s| s.to_string())
        .or_else(|| std::env::var("FORGE_BASE_URL").ok())
        .ok_or_else(|| {
            ForgeError::Config(
                "base URL not set. Provide agent.base_url in config or FORGE_BASE_URL env var."
                    .into(),
            )
        })?;

    let api_key = api_key
        .map(|s| s.to_string())
        .or_else(|| std::env::var("FORGE_API_KEY").ok())
        .ok_or_else(|| {
            ForgeError::Config(
                "API key not set. Provide agent.api_key in config or FORGE_API_KEY env var.".into(),
            )
        })?;

    let oai_config = OpenAICompatConfig::new(&base_url, &api_key, &model_name);
    Ok(Arc::new(OpenAICompatModel::new(oai_config)))
}

/// Extract `(owner, repo)` from a GitHub issue URL.
/// Accepts `https://github.com/owner/repo/issues/N` or without scheme.
fn extract_github_owner_repo(url: &str) -> Option<(String, String)> {
    let stripped = url
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    let rest = stripped.strip_prefix("github.com/")?;
    let parts: Vec<&str> = rest.split('/').collect();
    if parts.len() >= 2 {
        Some((parts[0].to_string(), parts[1].to_string()))
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AgentConfigSerde, EnvConfigSerde, ProblemStatementConfigSerde, RunConfig};

    #[test]
    fn test_run_config_from_yaml() {
        let yaml = r#"
agent:
  model_name: "test-model"
  max_steps: 50
env:
  image: "alpine:latest"
problem_statement:
  type: text
  text: "Test problem"
output_dir: "test_trajs"
"#;
        let config = RunConfig::from_yaml_str(yaml).unwrap();
        assert_eq!(config.agent.model_name.as_deref(), Some("test-model"));
        assert_eq!(config.agent.max_steps, Some(50));
        assert_eq!(config.env.image.as_deref(), Some("alpine:latest"));
        assert_eq!(config.output_dir, "test_trajs");
    }

    #[test]
    fn test_build_env_config_defaults() {
        let serde_cfg = EnvConfigSerde::default();
        let env_cfg = build_env_config(&serde_cfg);
        assert_eq!(env_cfg.image, "sweagent/swe-agent:latest");
        assert_eq!(env_cfg.repo_path, "/repo");
        assert_eq!(env_cfg.timeout_secs, 30);
    }

    #[test]
    fn test_build_env_config_overrides() {
        let serde_cfg = EnvConfigSerde {
            image: Some("my-image:v2".into()),
            repo_path: Some("/workspace".into()),
            timeout_secs: Some(120),
            ..Default::default()
        };
        let env_cfg = build_env_config(&serde_cfg);
        assert_eq!(env_cfg.image, "my-image:v2");
        assert_eq!(env_cfg.repo_path, "/workspace");
        assert_eq!(env_cfg.timeout_secs, 120);
    }

    #[test]
    fn test_problem_statement_from_config_empty() {
        let ps_config = ProblemStatementConfigSerde::Empty;
        let ps = build_problem_statement(&ps_config).unwrap();
        match ps {
            AnyProblemStatement::Empty(_) => {}
            _ => panic!("expected Empty"),
        }
    }

    #[test]
    fn test_problem_statement_from_config_text() {
        let ps_config = ProblemStatementConfigSerde::Text {
            text: "Fix the bug".into(),
        };
        let ps = build_problem_statement(&ps_config).unwrap();
        match ps {
            AnyProblemStatement::Text(t) => {
                assert!(!t.id.is_empty());
            }
            _ => panic!("expected Text"),
        }
    }

    #[test]
    fn test_problem_statement_from_config_text_file() {
        let ps_config = ProblemStatementConfigSerde::TextFile {
            path: "/tmp/problem.txt".into(),
        };
        let ps = build_problem_statement(&ps_config).unwrap();
        match ps {
            AnyProblemStatement::File(f) => {
                assert!(f.id.starts_with("file-"));
            }
            _ => panic!("expected File"),
        }
    }

    #[test]
    fn test_problem_statement_from_config_github_issue() {
        let ps_config = ProblemStatementConfigSerde::GithubIssue {
            url: "https://github.com/owner/repo/issues/123".into(),
        };
        let ps = build_problem_statement(&ps_config).unwrap();
        match ps {
            AnyProblemStatement::GithubIssue(g) => {
                assert!(g.id.contains("owner"));
                assert!(g.id.contains("repo"));
            }
            _ => panic!("expected GithubIssue"),
        }
    }

    #[test]
    fn test_build_agent_config_defaults() {
        let serde_cfg = AgentConfigSerde::default();
        let agent_cfg = build_agent_config(&serde_cfg).unwrap();
        // Should use AgentConfig defaults when serde fields are None
        assert_eq!(agent_cfg.max_steps, 100);
        assert_eq!(agent_cfg.max_requeries, 3);
        assert_eq!(agent_cfg.parser_type, "thought_action");
    }

    #[test]
    fn test_build_agent_config_overrides() {
        let serde_cfg = AgentConfigSerde {
            max_steps: Some(42),
            max_requeries: Some(5),
            parser_type: Some("action_only".into()),
            system_template: Some("Be helpful.".into()),
            ..Default::default()
        };
        let agent_cfg = build_agent_config(&serde_cfg).unwrap();
        assert_eq!(agent_cfg.max_steps, 42);
        assert_eq!(agent_cfg.max_requeries, 5);
        assert_eq!(agent_cfg.parser_type, "action_only");
        assert_eq!(agent_cfg.system_template, "Be helpful.");
    }

    #[test]
    fn test_run_single_from_config() {
        let config = RunConfig {
            agent: AgentConfigSerde {
                model_name: Some("gemini-2.5-flash".into()),
                base_url: Some("https://api.example.com/v1".into()),
                api_key: Some("test-key".into()),
                max_steps: Some(25),
                ..Default::default()
            },
            env: EnvConfigSerde {
                image: Some("alpine:latest".into()),
                ..Default::default()
            },
            problem_statement: ProblemStatementConfigSerde::Text {
                text: "Test problem".into(),
            },
            output_dir: "test_output".into(),
        };
        let run = RunSingle::from_run_config(config).unwrap();
        assert_eq!(run.agent_config.max_steps, 25);
        assert_eq!(run.env_config.image, "alpine:latest");
        assert_eq!(run.output_dir, PathBuf::from("test_output"));
        assert_eq!(run.model_name.as_deref(), Some("gemini-2.5-flash"));
    }

    #[test]
    fn test_extract_github_owner_repo() {
        let (o, r) = extract_github_owner_repo("https://github.com/owner/repo/issues/42").unwrap();
        assert_eq!(o, "owner");
        assert_eq!(r, "repo");
        let (o, r) = extract_github_owner_repo("github.com/foo/bar/issues/1").unwrap();
        assert_eq!(o, "foo");
        assert_eq!(r, "bar");
        assert!(extract_github_owner_repo("https://example.com/foo").is_none());
    }

    #[test]
    fn test_auto_derive_startup_commands_for_github_issue() {
        let config = RunConfig {
            agent: AgentConfigSerde {
                model_name: Some("test-model".into()),
                base_url: Some("https://api.example.com/v1".into()),
                api_key: Some("key".into()),
                ..Default::default()
            },
            env: EnvConfigSerde::default(), // no startup_commands
            problem_statement: ProblemStatementConfigSerde::GithubIssue {
                url: "https://github.com/owner/repo/issues/5".into(),
            },
            output_dir: "trajectories".into(),
        };
        let run = RunSingle::from_run_config(config).unwrap();
        assert_eq!(run.env_config.startup_commands.len(), 3);
        assert!(run.env_config.startup_commands[0].contains("https://github.com/owner/repo"));
        assert!(run.env_config.startup_commands[1].contains("forge@forge.local"));
        assert!(run.env_config.startup_commands[2].contains("submit"));
    }

    #[test]
    fn test_no_auto_derive_when_startup_commands_provided() {
        let config = RunConfig {
            agent: AgentConfigSerde::default(),
            env: EnvConfigSerde {
                startup_commands: Some(vec!["echo custom".into()]),
                ..Default::default()
            },
            problem_statement: ProblemStatementConfigSerde::GithubIssue {
                url: "https://github.com/owner/repo/issues/5".into(),
            },
            output_dir: "trajectories".into(),
        };
        let run = RunSingle::from_run_config(config).unwrap();
        assert_eq!(run.env_config.startup_commands, vec!["echo custom"]);
    }

    // Serialize env-var-mutating tests to prevent data races between threads.
    static ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_build_model_missing_no_env() {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());

        // Remove env vars for this test.
        let orig_model = std::env::var("FORGE_MODEL").ok();
        let orig_url = std::env::var("FORGE_BASE_URL").ok();
        let orig_key = std::env::var("FORGE_API_KEY").ok();

        std::env::remove_var("FORGE_MODEL");
        std::env::remove_var("FORGE_BASE_URL");
        std::env::remove_var("FORGE_API_KEY");

        let result = build_model(None, None, None);

        // Restore before asserting so we don't leave env dirty on failure.
        if let Some(v) = orig_model {
            std::env::set_var("FORGE_MODEL", v);
        }
        if let Some(v) = orig_url {
            std::env::set_var("FORGE_BASE_URL", v);
        }
        if let Some(v) = orig_key {
            std::env::set_var("FORGE_API_KEY", v);
        }

        assert!(result.is_err(), "expected error when no model configured");
    }

    #[test]
    fn test_build_model_with_explicit_values() {
        let model = build_model(
            Some("gemini-2.5-flash"),
            Some("https://generativelanguage.googleapis.com/v1beta/openai"),
            Some("test-api-key"),
        );
        assert!(model.is_ok(), "expected model to build successfully");
    }
}
