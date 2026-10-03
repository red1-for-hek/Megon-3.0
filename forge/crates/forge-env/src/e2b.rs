use forge_types::ForgeError;
#[derive(Debug, Clone)]
pub struct E2bConfig {
    pub api_key: String,
    pub template: Option<String>,
    pub timeout_secs: u64,
}

impl E2bConfig {
    pub fn from_env() -> Result<Self, ForgeError> {
        let api_key = std::env::var("E2B_API_KEY")
            .map_err(|_| ForgeError::Config("E2B_API_KEY is required for E2B execution".into()))?;

        Ok(Self {
            api_key,
            template: std::env::var("E2B_TEMPLATE").ok(),
            timeout_secs: std::env::var("E2B_TIMEOUT_SECS")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(1800),
        })
    }
}

#[derive(Debug, Clone)]
pub struct E2bRunner {
    config: E2bConfig,
}

impl E2bRunner {
    pub fn new(config: E2bConfig) -> Self {
        Self { config }
    }

    pub fn sdk_env(&self) -> Vec<(String, String)> {
        let mut env = vec![
            ("E2B_API_KEY".to_string(), self.config.api_key.clone()),
            (
                "E2B_TIMEOUT_SECS".to_string(),
                self.config.timeout_secs.to_string(),
            ),
        ];
        if let Some(template) = &self.config.template {
            env.push(("E2B_TEMPLATE".to_string(), template.clone()));
        }
        env
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::process::Command;

    #[test]
    fn e2b_live_sdk_creates_sandbox_and_runs_command_when_key_is_present() {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workspace_root = manifest_dir
            .parent()
            .and_then(|path| path.parent())
            .expect("forge-env is under crates/forge-env");

        let e2b_key = std::env::var("E2B_API_KEY")
            .ok()
            .or_else(|| read_env_file_value(&workspace_root.join(".env"), "E2B_API_KEY"));

        let Some(e2b_key) = e2b_key else {
            eprintln!("skipping live E2B test: E2B_API_KEY is not set");
            return;
        };

        let smoke_dir = workspace_root.join("scripts/e2b-smoke");
        let node_modules = smoke_dir.join("node_modules/e2b");

        assert!(
            node_modules.exists(),
            "E2B SDK dependency is missing. Run: npm --prefix {} install",
            smoke_dir.display()
        );

        let output = Command::new("npm")
            .args(["--prefix", smoke_dir.to_str().expect("utf-8 path"), "test"])
            .env("E2B_API_KEY", e2b_key)
            .output()
            .expect("failed to run E2B live smoke test");

        assert!(
            output.status.success(),
            "E2B live smoke test failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn read_env_file_value(path: &std::path::Path, key: &str) -> Option<String> {
        let content = std::fs::read_to_string(path).ok()?;
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (line_key, line_value) = line.split_once('=')?;
            if line_key.trim() == key {
                return Some(line_value.trim().trim_matches('"').to_string());
            }
        }
        None
    }
}
