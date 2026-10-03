/// High-level `SweEnvironment` — the main entry point for forge-env.
///
/// Combines a [`DockerContainer`], a [`BashSession`], and a [`RepoConfig`]
/// into a single convenient API that mirrors the TypeScript `SWEEnv` class.
use forge_types::ForgeError;

use crate::bash_session::{BashSession, CommandOutput};
use crate::docker::{extract_tar_first_file, DockerContainer};
use crate::repo::RepoConfig;
use crate::utils::base64_encode;

/// Configuration for a `SweEnvironment`.
#[derive(Debug, Clone)]
pub struct EnvironmentConfig {
    /// Docker image to use.
    pub image: String,
    /// Optional name for the container (useful for debugging).
    pub container_name: Option<String>,
    /// Absolute path to the repository inside the container.
    pub repo_path: String,
    /// Shell commands to run after the session is created (e.g. `source /root/.bashrc`).
    pub startup_commands: Vec<String>,
    /// Environment variables to set in the container process.
    pub env_vars: Vec<(String, String)>,
    /// Default timeout (seconds) for command execution.
    pub timeout_secs: u64,
    /// Git commit to reset the repo to on `reset_repo()`.
    pub base_commit: Option<String>,
}

impl Default for EnvironmentConfig {
    fn default() -> Self {
        Self {
            image: "sweagent/swe-agent:latest".to_string(),
            container_name: None,
            repo_path: "/repo".to_string(),
            startup_commands: vec![],
            env_vars: vec![],
            timeout_secs: 30,
            base_commit: None,
        }
    }
}

/// A fully running SWE environment: Docker container + persistent bash session
/// + repository lifecycle helpers.
pub struct SweEnvironment {
    container: DockerContainer,
    session: BashSession,
    repo: RepoConfig,
    pub config: EnvironmentConfig,
}

impl SweEnvironment {
    /// Create and start the full environment.
    ///
    /// 1. Pulls / starts the Docker container.
    /// 2. Opens a persistent bash session.
    /// 3. Runs any `startup_commands`.
    pub async fn create(config: EnvironmentConfig) -> Result<Self, ForgeError> {
        // Start the container.
        let container = DockerContainer::create_and_start(
            &config.image,
            config.container_name.as_deref(),
            &config.env_vars,
            &[], // We use the session for startup commands, not the container Cmd.
        )
        .await?;

        // Open the bash session — if this fails, clean up the container first.
        let mut session = match BashSession::new(container.docker.clone(), &container.container_id).await {
            Ok(s) => s,
            Err(e) => {
                let _ = container.remove().await;
                return Err(e);
            }
        };

        // Run startup commands (e.g. `source /root/.bashrc`).
        for cmd in &config.startup_commands {
            // Use a generous timeout for startup commands.
            let out = match session.run_command(cmd, config.timeout_secs.max(30)).await {
                Ok(o) => o,
                Err(e) => {
                    let _ = container.remove().await;
                    return Err(e);
                }
            };
            if out.exit_code != 0 {
                let _ = container.remove().await;
                return Err(ForgeError::Environment(format!(
                    "startup command '{}' failed (exit {}): {}",
                    cmd, out.exit_code, out.stderr
                )));
            }
        }

        // Set standard locale / pager variables (mirrors Python/TS swe-env).
        let env_setup = [
            "export LANG=C.UTF-8",
            "export LC_ALL=C.UTF-8",
            "export PIP_PROGRESS_BAR=off",
            "export PAGER=cat",
        ]
        .join(" && ");

        match session.run_command(&env_setup, config.timeout_secs).await {
            Ok(_) => {}
            Err(e) => {
                let _ = container.remove().await;
                return Err(e);
            }
        }

        let repo = RepoConfig::new(config.repo_path.clone(), config.base_commit.clone());

        Ok(Self {
            container,
            session,
            repo,
            config,
        })
    }

    // ------------------------------------------------------------------
    // Command execution
    // ------------------------------------------------------------------

    /// Execute a bash command using the default timeout from config.
    pub async fn execute(&mut self, command: &str) -> Result<CommandOutput, ForgeError> {
        self.session
            .run_command(command, self.config.timeout_secs)
            .await
    }

    /// Execute a bash command with a custom timeout (overrides the default).
    pub async fn execute_with_timeout(
        &mut self,
        command: &str,
        timeout_secs: u64,
    ) -> Result<CommandOutput, ForgeError> {
        self.session.run_command(command, timeout_secs).await
    }

    // ------------------------------------------------------------------
    // File I/O
    // ------------------------------------------------------------------

    /// Read a file from the container and return its content as a UTF-8 string.
    pub async fn read_file(&self, path: &str) -> Result<String, ForgeError> {
        let tar_bytes = self.container.copy_out(path).await?;
        let raw = extract_tar_first_file(&tar_bytes)?;
        String::from_utf8(raw)
            .map_err(|e| ForgeError::Environment(format!("file is not valid UTF-8: {}", e)))
    }

    /// Write `content` to `path` inside the container.
    ///
    /// Uses base64 via the bash session to avoid quoting pitfalls.
    pub async fn write_file(&mut self, path: &str, content: &str) -> Result<(), ForgeError> {
        // Ensure parent directory exists.
        let dir = parent_dir(path);
        if !dir.is_empty() && dir != "/" {
            let mkdir_cmd = format!("mkdir -p '{}'", dir);
            let out = self
                .session
                .run_command(&mkdir_cmd, self.config.timeout_secs)
                .await?;
            if out.exit_code != 0 {
                return Err(ForgeError::Environment(format!(
                    "mkdir -p '{}' failed: {}",
                    dir, out.stderr
                )));
            }
        }

        // Encode content as base64 and pipe through base64 -d.
        let encoded = base64_encode(content.as_bytes());
        let write_cmd = format!("echo '{}' | base64 -d > '{}'", encoded, path);
        let out = self
            .session
            .run_command(&write_cmd, self.config.timeout_secs)
            .await?;

        if out.exit_code != 0 {
            return Err(ForgeError::Environment(format!(
                "write_file '{}' failed: {}",
                path, out.stderr
            )));
        }

        Ok(())
    }

    // ------------------------------------------------------------------
    // Repository helpers
    // ------------------------------------------------------------------

    /// Apply a unified diff/patch to the repository.
    pub async fn apply_patch(&mut self, patch: &str) -> Result<(), ForgeError> {
        self.repo.apply_patch(&mut self.session, patch).await
    }

    /// Reset the repository to the base commit specified in config.
    pub async fn reset_repo(&mut self) -> Result<(), ForgeError> {
        self.repo.reset(&mut self.session).await
    }

    /// Get the current git diff of the repository.
    pub async fn get_diff(&mut self) -> Result<String, ForgeError> {
        self.repo.get_diff(&mut self.session).await
    }

    // ------------------------------------------------------------------
    // Lifecycle
    // ------------------------------------------------------------------

    /// Tear down the environment, removing the container.
    pub async fn close(self) -> Result<(), ForgeError> {
        self.container.remove().await
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Return the parent directory component of `path`, or `""` if none.
fn parent_dir(path: &str) -> &str {
    if let Some(pos) = path.rfind('/') {
        if pos == 0 {
            "/"
        } else {
            &path[..pos]
        }
    } else {
        ""
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn environment_config_defaults() {
        let cfg = EnvironmentConfig::default();
        assert_eq!(cfg.image, "sweagent/swe-agent:latest");
        assert!(cfg.container_name.is_none());
        assert_eq!(cfg.repo_path, "/repo");
        assert!(cfg.startup_commands.is_empty());
        assert!(cfg.env_vars.is_empty());
        assert_eq!(cfg.timeout_secs, 30);
        assert!(cfg.base_commit.is_none());
    }

    #[test]
    fn parent_dir_basic() {
        assert_eq!(parent_dir("/tmp/foo/bar.txt"), "/tmp/foo");
        assert_eq!(parent_dir("/file.txt"), "/");
        assert_eq!(parent_dir("file.txt"), "");
    }

    #[test]
    fn base64_encode_correctness() {
        // "hello" in standard base64 is "aGVsbG8="
        assert_eq!(base64_encode(b"hello"), "aGVsbG8=");
        // Empty input produces empty output
        assert_eq!(base64_encode(b""), "");
    }

    #[tokio::test]
    #[ignore = "requires Docker daemon"]
    async fn environment_create_execute_close() {
        let cfg = EnvironmentConfig {
            image: "alpine:latest".to_string(),
            container_name: None,
            repo_path: "/tmp".to_string(),
            startup_commands: vec![],
            env_vars: vec![],
            timeout_secs: 15,
            base_commit: None,
        };

        let mut env = SweEnvironment::create(cfg).await.expect("create");

        let out = env.execute("echo hello-forge").await.expect("execute");
        assert_eq!(out.stdout.trim(), "hello-forge");
        assert_eq!(out.exit_code, 0);

        env.close().await.expect("close");
    }

    #[tokio::test]
    #[ignore = "requires Docker daemon"]
    async fn environment_write_read_file() {
        let cfg = EnvironmentConfig {
            image: "alpine:latest".to_string(),
            container_name: None,
            repo_path: "/tmp".to_string(),
            startup_commands: vec![],
            env_vars: vec![],
            timeout_secs: 15,
            base_commit: None,
        };

        let mut env = SweEnvironment::create(cfg).await.expect("create");

        env.write_file("/tmp/forge_test.txt", "forge content 123")
            .await
            .expect("write_file");

        let content = env.read_file("/tmp/forge_test.txt").await.expect("read_file");
        assert_eq!(content, "forge content 123");

        env.close().await.expect("close");
    }
}
