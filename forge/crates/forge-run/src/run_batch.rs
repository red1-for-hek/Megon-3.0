use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use forge_types::error::ForgeError;

use crate::config::RunConfig;
use crate::run_single::RunSingle;

pub struct RunBatch {
    configs: Vec<RunConfig>,
    output_dir: PathBuf,
    num_workers: usize,
}

impl RunBatch {
    pub fn new(configs: Vec<RunConfig>, output_dir: PathBuf, num_workers: usize) -> Self {
        Self {
            configs,
            output_dir,
            num_workers,
        }
    }

    /// Run all configs in parallel (bounded by `num_workers`).
    /// Returns a list of `(instance_id, result)` for each run, where the id is
    /// derived from the problem statement (or the config index as a fallback).
    pub async fn run(self) -> Vec<(String, Result<(), ForgeError>)> {
        let sem = Arc::new(Semaphore::new(self.num_workers.max(1)));
        let mut join_set: JoinSet<(String, Result<(), ForgeError>)> = JoinSet::new();

        for (idx, mut config) in self.configs.into_iter().enumerate() {
            // Derive an instance id from the problem statement.
            let instance_id = match &config.problem_statement {
                crate::config::ProblemStatementConfigSerde::GithubIssue { url } => url.clone(),
                crate::config::ProblemStatementConfigSerde::Text { text } => {
                    let preview: String = text.chars().take(20).collect();
                    format!("text-{preview}")
                }
                crate::config::ProblemStatementConfigSerde::TextFile { path } => {
                    path.to_string_lossy().to_string()
                }
                crate::config::ProblemStatementConfigSerde::Empty => format!("instance-{idx}"),
            };

            // Override output dir with the batch-level one.
            config.output_dir = self.output_dir.to_string_lossy().to_string();

            let sem = Arc::clone(&sem);
            join_set.spawn(async move {
                let _permit = sem.acquire().await.expect("batch semaphore was unexpectedly closed");
                let result = match RunSingle::from_run_config(config) {
                    Ok(run) => run.run().await.map(|_| ()),
                    Err(e) => Err(e),
                };
                (instance_id, result)
            });
        }

        let mut results = Vec::new();
        while let Some(res) = join_set.join_next().await {
            match res {
                Ok(pair) => results.push(pair),
                Err(e) => {
                    tracing::error!("batch worker panicked: {e}");
                }
            }
        }

        results
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AgentConfigSerde, ProblemStatementConfigSerde, RunConfig};

    fn make_text_config(text: &str) -> RunConfig {
        RunConfig {
            agent: AgentConfigSerde::default(),
            env: Default::default(),
            problem_statement: ProblemStatementConfigSerde::Text { text: text.into() },
            output_dir: "trajectories".into(),
        }
    }

    #[test]
    fn test_run_batch_new() {
        let configs = vec![make_text_config("problem 1"), make_text_config("problem 2")];
        let batch = RunBatch::new(configs, PathBuf::from("trajs"), 4);
        assert_eq!(batch.num_workers, 4);
        assert_eq!(batch.configs.len(), 2);
        assert_eq!(batch.output_dir, PathBuf::from("trajs"));
    }

    #[test]
    fn test_run_batch_zero_workers_clamped() {
        // num_workers = 0 is clamped to 1 in run()
        let batch = RunBatch::new(vec![], PathBuf::from("trajs"), 0);
        assert_eq!(batch.num_workers, 0); // stored as given
        // The actual semaphore gets max(0,1)=1 permits
    }
}
