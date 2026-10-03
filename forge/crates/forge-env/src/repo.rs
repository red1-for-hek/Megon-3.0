/// Repository management inside the Docker container.
///
/// Mirrors the patterns in `src/environment/repo.ts`:
/// - reset to a specific base commit
/// - apply a diff/patch
/// - get the current diff
use crate::bash_session::BashSession;
use crate::utils::base64_encode;
use forge_types::ForgeError;

/// Repository configuration for the containerized repo.
#[derive(Debug, Clone)]
pub struct RepoConfig {
    /// Absolute path to the repository *inside* the container.
    pub repo_path: String,
    /// Git commit (SHA, branch, tag, or HEAD) to reset to on `reset()`.
    pub base_commit: Option<String>,
}

impl RepoConfig {
    /// Create a new `RepoConfig`.
    pub fn new(repo_path: impl Into<String>, base_commit: Option<impl Into<String>>) -> Self {
        Self {
            repo_path: repo_path.into(),
            base_commit: base_commit.map(|s| s.into()),
        }
    }

    /// Apply a unified diff/patch to the repository.
    ///
    /// The patch is written to a temporary file inside the container and then
    /// applied with `git apply`.
    pub async fn apply_patch(
        &self,
        session: &mut BashSession,
        patch: &str,
    ) -> Result<(), ForgeError> {
        if patch.trim().is_empty() {
            return Ok(());
        }

        // Write the patch to a temp file using a here-document so we don't
        // have to worry about shell escaping.
        let tmp_path = "/tmp/__forge_patch__.diff";

        // Use base64 to transfer the patch safely (avoids quoting issues).
        let encoded = base64_encode(patch.as_bytes());
        let write_cmd = format!(
            "echo '{}' | base64 -d > {}",
            encoded, tmp_path
        );

        let write_out = session.run_command(&write_cmd, 10).await?;
        if write_out.exit_code != 0 {
            return Err(ForgeError::Environment(format!(
                "failed to write patch file: {}",
                write_out.stderr
            )));
        }

        // Apply the patch.
        let apply_cmd = format!("cd {} && git apply {}", self.repo_path, tmp_path);
        let apply_out = session.run_command(&apply_cmd, 30).await?;
        if apply_out.exit_code != 0 {
            return Err(ForgeError::Environment(format!(
                "git apply failed (exit {}): {}",
                apply_out.exit_code, apply_out.stderr
            )));
        }

        // Cleanup.
        let _ = session.run_command(&format!("rm -f {}", tmp_path), 5).await;

        Ok(())
    }

    /// Reset the repository to `base_commit` (or `HEAD` if not set).
    ///
    /// Runs the same sequence of git commands as the TS implementation:
    /// `git fetch`, `git status`, `git restore .`, `git reset --hard`,
    /// `git checkout <commit>`, `git clean -fdq`.
    pub async fn reset(&self, session: &mut BashSession) -> Result<(), ForgeError> {
        let commit = self
            .base_commit
            .as_deref()
            .unwrap_or("HEAD");

        let commands = [
            format!("cd {}", self.repo_path),
            "git fetch --quiet".to_string(),
            "git status".to_string(),
            "git restore .".to_string(),
            "git reset --hard".to_string(),
            format!("git checkout {}", commit),
            "git clean -fdq".to_string(),
        ];

        let combined = commands.join(" && ");
        let out = session.run_command(&combined, 120).await?;

        if out.exit_code != 0 {
            return Err(ForgeError::Environment(format!(
                "repo reset failed (exit {}): {}",
                out.exit_code, out.stderr
            )));
        }

        Ok(())
    }

    /// Return the current `git diff` of the repository.
    pub async fn get_diff(&self, session: &mut BashSession) -> Result<String, ForgeError> {
        let cmd = format!("cd {} && git diff", self.repo_path);
        let out = session.run_command(&cmd, 30).await?;

        if out.exit_code != 0 {
            return Err(ForgeError::Environment(format!(
                "git diff failed (exit {}): {}",
                out.exit_code, out.stderr
            )));
        }

        Ok(out.stdout)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_config_defaults() {
        let r = RepoConfig::new("/repo", None::<String>);
        assert_eq!(r.repo_path, "/repo");
        assert!(r.base_commit.is_none());
    }

    #[test]
    fn repo_config_with_commit() {
        let r = RepoConfig::new("/myrepo", Some("abc123"));
        assert_eq!(r.base_commit.as_deref(), Some("abc123"));
    }

    #[tokio::test]
    #[ignore = "requires Docker daemon"]
    async fn apply_patch_in_container() {
        use crate::docker::DockerContainer;

        // 1. Create a container from a simple linux image with git available.
        let container = DockerContainer::create_and_start("ubuntu:22.04", None, &[], &[])
            .await
            .expect("Docker required");

        // 2. Create a bash session.
        let mut session = BashSession::new(container.docker.clone(), &container.container_id)
            .await
            .expect("bash session");

        // 3. Install git and set up a minimal git repo in /tmp/testrepo.
        let setup = "apt-get update -qq && apt-get install -y -qq git > /dev/null 2>&1 && \
            git init /tmp/testrepo && \
            cd /tmp/testrepo && \
            git config user.email 'test@test.com' && \
            git config user.name 'Test' && \
            echo 'hello world' > hello.txt && \
            git add . && \
            git commit -m 'init'";
        let out = session.run_command(setup, 120).await.expect("setup");
        assert_eq!(out.exit_code, 0, "setup failed: {}", out.stderr);

        // 4. Create a RepoConfig.
        let repo = RepoConfig {
            repo_path: "/tmp/testrepo".to_string(),
            base_commit: Some("HEAD".to_string()),
        };

        // 5. Apply a patch that adds a new file.
        let patch = "diff --git a/new.txt b/new.txt\n\
            new file mode 100644\n\
            index 0000000..8ab686e\n\
            --- /dev/null\n\
            +++ b/new.txt\n\
            @@ -0,0 +1 @@\n\
            +patched\n";
        repo.apply_patch(&mut session, patch)
            .await
            .expect("apply_patch");

        // 6. Verify the file exists.
        let check = session
            .run_command("cat /tmp/testrepo/new.txt", 10)
            .await
            .expect("check");
        assert!(
            check.stdout.contains("patched"),
            "patch not applied: {}",
            check.stdout
        );

        // 7. Test get_diff.
        let diff = repo.get_diff(&mut session).await.expect("get_diff");
        assert!(
            diff.contains("new.txt") || diff.contains("patched"),
            "diff empty: {}",
            diff
        );

        // 8. Test reset.
        repo.reset(&mut session).await.expect("reset");
        let after_reset = session
            .run_command("ls /tmp/testrepo/", 10)
            .await
            .expect("ls");
        assert!(
            !after_reset.stdout.contains("new.txt"),
            "reset did not remove new file"
        );

        // 9. Cleanup.
        drop(session);
        container.remove().await.expect("cleanup");
    }
}
