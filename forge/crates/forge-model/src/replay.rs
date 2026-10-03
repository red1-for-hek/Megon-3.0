use std::sync::Mutex;

use async_trait::async_trait;

use forge_types::{ForgeError, History, ModelOutput, TrajFile, TrajectoryStep};

use crate::traits::{AbstractModel, InstanceStats};

pub struct ReplayModel {
    responses: Vec<String>,
    cursor: Mutex<usize>,
    stats: Mutex<InstanceStats>,
}

impl ReplayModel {
    /// Build a ReplayModel from a trajectory file.
    /// Extracts the `response` field from each TrajectoryStep.
    pub fn from_traj(traj: &TrajFile) -> Self {
        let responses: Vec<String> = traj
            .trajectory
            .iter()
            .map(|step: &TrajectoryStep| step.response.clone())
            .collect();

        Self {
            responses,
            cursor: Mutex::new(0),
            stats: Mutex::new(InstanceStats::default()),
        }
    }

    /// Build a ReplayModel directly from a list of response strings.
    pub fn from_responses(responses: Vec<String>) -> Self {
        Self {
            responses,
            cursor: Mutex::new(0),
            stats: Mutex::new(InstanceStats::default()),
        }
    }
}

#[async_trait]
impl AbstractModel for ReplayModel {
    async fn query(&self, _history: &History) -> Result<ModelOutput, ForgeError> {
        let response = {
            let mut cursor = self.cursor.lock().unwrap_or_else(|p| p.into_inner());
            if *cursor >= self.responses.len() {
                return Err(ForgeError::Model("replay trajectory exhausted".into()));
            }
            let r = self.responses[*cursor].clone();
            *cursor += 1;
            r
        };

        {
            let mut stats = self.stats.lock().unwrap_or_else(|p| p.into_inner());
            stats.add_tokens(0, 0, 0.0);
        }

        Ok(ModelOutput {
            message: response,
            tool_calls: None,
            thinking_blocks: None,
            input_tokens: None,
            output_tokens: None,
            cost: Some(0.0),
        })
    }

    fn stats(&self) -> InstanceStats {
        self.stats.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }

    fn reset_stats(&self) {
        *self.stats.lock().unwrap_or_else(|p| p.into_inner()) = InstanceStats::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_types::{AgentInfo, TrajFile, TrajectoryStep};

    fn make_traj(responses: Vec<&str>) -> TrajFile {
        let trajectory: Vec<TrajectoryStep> = responses
            .iter()
            .map(|r| TrajectoryStep {
                response: r.to_string(),
                ..Default::default()
            })
            .collect();

        TrajFile {
            trajectory,
            history: None,
            info: AgentInfo::default(),
            replay_config: None,
            environment: "docker".into(),
        }
    }

    #[tokio::test]
    async fn replay_from_traj_returns_responses_in_order() {
        let traj = make_traj(vec!["first response", "second response"]);
        let model = ReplayModel::from_traj(&traj);
        let history = vec![];

        let out1 = model.query(&history).await.unwrap();
        assert_eq!(out1.message, "first response");

        let out2 = model.query(&history).await.unwrap();
        assert_eq!(out2.message, "second response");
    }

    #[tokio::test]
    async fn errors_when_exhausted() {
        let model = ReplayModel::from_traj(&make_traj(vec!["only"]));
        let history = vec![];
        model.query(&history).await.unwrap();
        let result = model.query(&history).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn replay_from_responses() {
        let model = ReplayModel::from_responses(vec!["alpha".into(), "beta".into()]);
        let history = vec![];

        let out = model.query(&history).await.unwrap();
        assert_eq!(out.message, "alpha");
        let out = model.query(&history).await.unwrap();
        assert_eq!(out.message, "beta");
    }

    #[tokio::test]
    async fn replay_tracks_api_calls() {
        let model = ReplayModel::from_responses(vec!["a".into(), "b".into()]);
        let history = vec![];
        model.query(&history).await.unwrap();
        model.query(&history).await.unwrap();
        assert_eq!(model.stats().api_calls, 2);
    }

    #[tokio::test]
    async fn replay_reset_stats() {
        let model = ReplayModel::from_responses(vec!["a".into()]);
        let history = vec![];
        model.query(&history).await.unwrap();
        model.reset_stats();
        assert_eq!(model.stats().api_calls, 0);
    }

    #[test]
    fn traj_file_has_none_history() {
        let traj = make_traj(vec!["r"]);
        assert!(traj.history.is_none());
    }
}
