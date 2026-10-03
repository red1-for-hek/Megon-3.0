use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageType {
    Thought,
    Action,
    Observation,
    System,
    User,
    Assistant,
    Demonstration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text { text: String },
    Image { image_url: ImageUrl },
    ToolUse { id: String, name: String, input: serde_json::Value },
    ToolResult { tool_use_id: String, content: String },
    Thinking { thinking: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUrl {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheControl {
    #[serde(rename = "type")]
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThinkingBlock {
    #[serde(rename = "type")]
    block_type: String,  // always "thinking" — use ThinkingBlock::new()
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<f64>,
}

impl ThinkingBlock {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            block_type: "thinking".to_string(),
            content: content.into(),
            start_time: None,
            end_time: None,
        }
    }

    pub fn block_type(&self) -> &str {
        &self.block_type
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCall {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub tool_type: Option<String>,
    pub function: ToolFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFunction {
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItem {
    pub role: Role,
    pub content: MessageContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_type: Option<MessageType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(default)]
    pub is_demo: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thought: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControl>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_blocks: Option<Vec<ThinkingBlock>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

impl Default for HistoryItem {
    fn default() -> Self {
        Self {
            role: Role::User,
            content: MessageContent::Text(String::new()),
            message_type: None,
            agent: None,
            is_demo: false,
            thought: None,
            action: None,
            tool_calls: None,
            tool_call_ids: None,
            cache_control: None,
            thinking_blocks: None,
            tags: None,
        }
    }
}

pub type History = Vec<HistoryItem>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_item_text_serializes() {
        let item = HistoryItem {
            role: Role::User,
            content: MessageContent::Text("hello".into()),
            ..Default::default()
        };
        let json = serde_json::to_value(&item).unwrap();
        assert_eq!(json["role"], "user");
        assert_eq!(json["content"], "hello");
    }

    #[test]
    fn history_item_multipart_content() {
        let item = HistoryItem {
            role: Role::Assistant,
            content: MessageContent::Blocks(vec![
                ContentBlock::Text { text: "hello".into() },
            ]),
            ..Default::default()
        };
        let json = serde_json::to_value(&item).unwrap();
        assert!(json["content"].is_array());
        assert_eq!(json["role"], "assistant");
    }

    #[test]
    fn round_trip_with_tool_calls() {
        let item = HistoryItem {
            role: Role::Assistant,
            content: MessageContent::Text("using tool".into()),
            tool_calls: Some(vec![ToolCall {
                id: Some("tc_1".into()),
                tool_type: None,
                function: ToolFunction {
                    name: "bash".into(),
                    arguments: serde_json::json!({"command": "ls"}),
                },
            }]),
            ..Default::default()
        };
        let json = serde_json::to_string(&item).unwrap();
        let back: HistoryItem = serde_json::from_str(&json).unwrap();
        assert_eq!(back.tool_calls.unwrap()[0].function.name, "bash");
    }
}
