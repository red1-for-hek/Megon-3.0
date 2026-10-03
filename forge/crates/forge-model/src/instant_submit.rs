use std::sync::Mutex;

use async_trait::async_trait;

use forge_types::{ForgeError, History, ModelOutput};

use crate::traits::{AbstractModel, InstanceStats};

pub struct InstantSubmitModel {
    stats: Mutex<InstanceStats>,
    action_idx: Mutex<u32>,
}

impl Default for InstantSubmitModel {
    fn default() -> Self {
        Self::new()
    }
}

impl InstantSubmitModel {
    pub fn new() -> Self {
        Self {
            stats: Mutex::new(InstanceStats::default()),
            action_idx: Mutex::new(0),
        }
    }
}

#[async_trait]
impl AbstractModel for InstantSubmitModel {
    async fn query(&self, _history: &History) -> Result<ModelOutput, ForgeError> {
        let mut idx = self.action_idx.lock().unwrap_or_else(|p| p.into_inner());
        let action = if *idx == 0 {
            *idx = 1;
            "DISCUSSION\nLet's reproduce the bug by creating a `reproduce.py` file.\n\n```\ntouch reproduce.py\n```\n".to_string()
        } else {
            *idx = 0;
            format!(
                "DISCUSSION\nThe task should be resolved, so let's submit the patch.\n\n```\nsubmit\n{}\n```\n",
                forge_types::special_tokens::SUBMISSION
            )
        };

        {
            let mut stats = self.stats.lock().unwrap_or_else(|p| p.into_inner());
            stats.add_tokens(0, 0, 0.0);
        }

        Ok(ModelOutput {
            message: action,
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

    #[tokio::test]
    async fn instant_submit_first_response() {
        let model = InstantSubmitModel::new();
        let history = vec![];
        let output = model.query(&history).await.unwrap();
        assert!(output.message.contains("reproduce.py"));
    }

    #[tokio::test]
    async fn instant_submit_second_response_has_submission_token() {
        let model = InstantSubmitModel::new();
        let history = vec![];
        let _ = model.query(&history).await.unwrap();
        let output = model.query(&history).await.unwrap();
        assert!(output.message.contains(forge_types::special_tokens::SUBMISSION));
    }

    #[tokio::test]
    async fn instant_submit_tracks_api_calls() {
        let model = InstantSubmitModel::new();
        let history = vec![];
        model.query(&history).await.unwrap();
        model.query(&history).await.unwrap();
        let stats = model.stats();
        assert_eq!(stats.api_calls, 2);
    }

    #[tokio::test]
    async fn instant_submit_reset_stats() {
        let model = InstantSubmitModel::new();
        let history = vec![];
        model.query(&history).await.unwrap();
        model.reset_stats();
        let stats = model.stats();
        assert_eq!(stats.api_calls, 0);
    }
}
