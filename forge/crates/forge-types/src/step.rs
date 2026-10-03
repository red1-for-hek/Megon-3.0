use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::error::ExitStatus;
use crate::history::{ThinkingBlock, ToolCall};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StepOutput {
    pub thought: String,
    pub action: String,
    pub observation: String,
    pub execution_time: f64,
    pub done: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_status: Option<ExitStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub submission: Option<String>,
    pub state: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_blocks: Option<Vec<ThinkingBlock>>,
    #[serde(default)]
    pub extra_info: HashMap<String, serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ExitStatus;

    #[test]
    fn step_output_camel_case_wire_format() {
        // StepOutput uses rename_all = "camelCase", verify key wire names
        let step = StepOutput {
            thought: "thinking".to_string(),
            action: "ls".to_string(),
            observation: "file.txt".to_string(),
            execution_time: 1.5,
            done: false,
            exit_status: None,
            submission: None,
            state: std::collections::HashMap::new(),
            tool_calls: None,
            thinking_blocks: None,
            extra_info: std::collections::HashMap::new(),
        };
        let json = serde_json::to_string(&step).unwrap();
        assert!(json.contains("\"executionTime\""), "executionTime key must be camelCase in JSON");
        assert!(json.contains("\"exitStatus\"") || !json.contains("exitStatus"), "exitStatus key must be camelCase or absent");
        assert!(json.contains("\"toolCalls\"") || !json.contains("tool_calls"), "toolCalls must be camelCase or absent");
    }

    #[test]
    fn step_output_with_exit_status() {
        let step = StepOutput {
            thought: String::new(),
            action: String::new(),
            observation: String::new(),
            execution_time: 0.0,
            done: true,
            exit_status: Some(ExitStatus::Submitted),
            submission: Some("patch content".to_string()),
            state: Default::default(),
            tool_calls: None,
            thinking_blocks: None,
            extra_info: Default::default(),
        };
        let json = serde_json::to_string(&step).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["exitStatus"], "submitted");
    }
}
