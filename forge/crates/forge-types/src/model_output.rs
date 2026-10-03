use serde::{Deserialize, Serialize};
use crate::history::{ThinkingBlock, ToolCall};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelOutput {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_blocks: Option<Vec<ThinkingBlock>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_output_minimal_deserializes() {
        // Only `message` is required; all optional fields can be absent
        let json = r#"{"message": "hello"}"#;
        let output: ModelOutput = serde_json::from_str(json).unwrap();
        assert_eq!(output.message, "hello");
        assert!(output.tool_calls.is_none());
        assert!(output.input_tokens.is_none());
        assert!(output.cost.is_none());
    }

    #[test]
    fn model_output_with_token_counts() {
        let json = r#"{"message": "hi", "input_tokens": 10, "output_tokens": 5, "cost": 0.001}"#;
        let output: ModelOutput = serde_json::from_str(json).unwrap();
        assert_eq!(output.input_tokens, Some(10));
        assert_eq!(output.output_tokens, Some(5));
        assert!((output.cost.unwrap() - 0.001).abs() < 1e-9);
    }

    #[test]
    fn model_output_round_trip() {
        let output = ModelOutput {
            message: "test".to_string(),
            tool_calls: None,
            thinking_blocks: None,
            input_tokens: Some(100),
            output_tokens: Some(50),
            cost: Some(0.005),
        };
        let json = serde_json::to_string(&output).unwrap();
        let deserialized: ModelOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.message, output.message);
        assert_eq!(deserialized.input_tokens, output.input_tokens);
    }
}
