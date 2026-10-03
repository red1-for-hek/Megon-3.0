use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use serde_json::{json, Value};

use forge_types::{
    ContentBlock, ForgeError, History, HistoryItem, MessageContent, ModelOutput, Role,
    ToolCall, ToolFunction,
};

use crate::pricing::calculate_cost;
use crate::traits::{AbstractModel, InstanceStats};

#[derive(Debug, Clone)]
pub struct OpenAICompatConfig {
    pub base_url: String,
    pub api_key: String,
    pub model_name: String,
    pub temperature: f32,
    pub max_tokens: Option<u32>,
    pub extra_headers: HashMap<String, String>,
    /// Cost per million input tokens in USD. None = unknown (cost tracked as 0).
    pub cost_per_million_input: Option<f64>,
    /// Cost per million output tokens in USD. None = unknown (cost tracked as 0).
    pub cost_per_million_output: Option<f64>,
}

impl OpenAICompatConfig {
    pub fn new(base_url: &str, api_key: &str, model_name: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.into(),
            model_name: model_name.into(),
            temperature: 1.0,
            max_tokens: None,
            extra_headers: Default::default(),
            cost_per_million_input: None,
            cost_per_million_output: None,
        }
    }
}

pub struct OpenAICompatModel {
    config: OpenAICompatConfig,
    client: reqwest::Client,
    stats: Mutex<InstanceStats>,
}

impl OpenAICompatModel {
    pub fn new(config: OpenAICompatConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
            stats: Mutex::new(InstanceStats::default()),
        }
    }
}

/// Convert a History (Vec<HistoryItem>) to OpenAI-format messages (Vec<Value>).
pub(crate) fn history_to_messages(history: &History) -> Vec<Value> {
    history.iter().map(item_to_message).collect()
}

fn item_to_message(item: &HistoryItem) -> Value {
    let role = match item.role {
        Role::System => "system",
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::Tool => "tool",
    };

    let content = content_to_value(&item.content);

    let mut msg = json!({
        "role": role,
        "content": content,
    });

    // Attach tool_calls if present
    if let Some(ref tcs) = item.tool_calls {
        let calls: Vec<Value> = tcs.iter().map(tool_call_to_value).collect();
        msg["tool_calls"] = Value::Array(calls);
    }

    // Attach tool_call_id for tool-role messages
    if let Some(ref ids) = item.tool_call_ids {
        if let Some(id) = ids.first() {
            msg["tool_call_id"] = Value::String(id.clone());
        }
    }

    msg
}

fn content_to_value(content: &MessageContent) -> Value {
    match content {
        MessageContent::Text(s) => Value::String(s.clone()),
        MessageContent::Blocks(blocks) => {
            let parts: Vec<Value> = blocks.iter().map(block_to_value).collect();
            Value::Array(parts)
        }
    }
}

fn block_to_value(block: &ContentBlock) -> Value {
    match block {
        ContentBlock::Text { text } => json!({ "type": "text", "text": text }),
        ContentBlock::Image { image_url } => json!({
            "type": "image_url",
            "image_url": { "url": image_url.url }
        }),
        ContentBlock::ToolUse { id, name, input } => json!({
            "type": "tool_use",
            "id": id,
            "name": name,
            "input": input,
        }),
        ContentBlock::ToolResult { tool_use_id, content } => json!({
            "type": "tool_result",
            "tool_use_id": tool_use_id,
            "content": content,
        }),
        ContentBlock::Thinking { thinking } => json!({
            "type": "thinking",
            "thinking": thinking,
        }),
    }
}

fn tool_call_to_value(tc: &ToolCall) -> Value {
    json!({
        "id": tc.id,
        "type": "function",
        "function": {
            "name": tc.function.name,
            "arguments": tc.function.arguments.to_string(),
        }
    })
}

fn parse_tool_calls(calls: &[Value]) -> Vec<ToolCall> {
    calls
        .iter()
        .filter_map(|c| {
            let id = c["id"].as_str().map(String::from);
            let name = c["function"]["name"].as_str()?.to_string();
            let args_str = c["function"]["arguments"].as_str()?;
            let arguments: Value = serde_json::from_str(args_str).ok()?;
            Some(ToolCall {
                id,
                tool_type: Some("function".to_string()),
                function: ToolFunction { name, arguments },
            })
        })
        .collect()
}

#[async_trait]
impl AbstractModel for OpenAICompatModel {
    async fn query(&self, history: &History) -> Result<ModelOutput, ForgeError> {
        let messages = history_to_messages(history);

        let mut body = json!({
            "model": self.config.model_name,
            "messages": messages,
            "temperature": self.config.temperature,
        });

        if let Some(max_tokens) = self.config.max_tokens {
            body["max_tokens"] = json!(max_tokens);
        }

        let url = format!("{}/chat/completions", self.config.base_url);

        let mut req = self
            .client
            .post(&url)
            .bearer_auth(&self.config.api_key)
            .json(&body);

        for (k, v) in &self.config.extra_headers {
            req = req.header(k.as_str(), v.as_str());
        }

        let resp = req
            .send()
            .await
            .map_err(|e| ForgeError::Http(e.to_string()))?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(ForgeError::Http(format!("{}: {}", status, text)));
        }

        let json: Value = resp
            .json()
            .await
            .map_err(|e| ForgeError::Http(e.to_string()))?;

        let choice = json["choices"]
            .get(0)
            .ok_or_else(|| ForgeError::Model("no choices in response".into()))?;

        let message_val = &choice["message"];
        let text = message_val["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let tool_calls = message_val["tool_calls"]
            .as_array()
            .map(|arr| parse_tool_calls(arr));

        let input_tokens = json["usage"]["prompt_tokens"].as_u64().map(|v| v as u32);
        let output_tokens = json["usage"]["completion_tokens"].as_u64().map(|v| v as u32);
        let cost = calculate_cost(
            self.config.cost_per_million_input,
            self.config.cost_per_million_output,
            input_tokens.unwrap_or(0),
            output_tokens.unwrap_or(0),
        );

        {
            let mut stats = self.stats.lock().unwrap_or_else(|p| p.into_inner());
            stats.add_tokens(
                input_tokens.unwrap_or(0),
                output_tokens.unwrap_or(0),
                cost,
            );
        }

        Ok(ModelOutput {
            message: text,
            tool_calls: tool_calls.filter(|v| !v.is_empty()),
            thinking_blocks: None,
            input_tokens,
            output_tokens,
            cost: Some(cost),
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
    use forge_types::{HistoryItem, MessageContent, Role};

    #[test]
    fn config_builder_defaults() {
        let cfg = OpenAICompatConfig::new("https://api.openai.com/v1", "sk-test", "gpt-4o");
        assert_eq!(cfg.temperature, 1.0);
        assert!(cfg.cost_per_million_input.is_none());
        assert!(cfg.cost_per_million_output.is_none());
        assert_eq!(cfg.base_url, "https://api.openai.com/v1");
        assert_eq!(cfg.model_name, "gpt-4o");
    }

    #[test]
    fn config_trims_trailing_slash() {
        let cfg = OpenAICompatConfig::new("https://api.openai.com/v1/", "key", "model");
        assert_eq!(cfg.base_url, "https://api.openai.com/v1");
    }

    #[test]
    fn history_to_messages_converts_roles() {
        let history = vec![
            HistoryItem {
                role: Role::System,
                content: MessageContent::Text("You are helpful.".into()),
                ..Default::default()
            },
            HistoryItem {
                role: Role::User,
                content: MessageContent::Text("Hello".into()),
                ..Default::default()
            },
            HistoryItem {
                role: Role::Assistant,
                content: MessageContent::Text("Hi!".into()),
                ..Default::default()
            },
        ];

        let msgs = history_to_messages(&history);
        assert_eq!(msgs.len(), 3);
        assert_eq!(msgs[0]["role"], "system");
        assert_eq!(msgs[0]["content"], "You are helpful.");
        assert_eq!(msgs[1]["role"], "user");
        assert_eq!(msgs[1]["content"], "Hello");
        assert_eq!(msgs[2]["role"], "assistant");
        assert_eq!(msgs[2]["content"], "Hi!");
    }

    #[test]
    fn history_to_messages_tool_role() {
        let history = vec![HistoryItem {
            role: Role::Tool,
            content: MessageContent::Text("result".into()),
            tool_call_ids: Some(vec!["tc_1".into()]),
            ..Default::default()
        }];
        let msgs = history_to_messages(&history);
        assert_eq!(msgs[0]["role"], "tool");
        assert_eq!(msgs[0]["tool_call_id"], "tc_1");
    }

    #[test]
    fn history_to_messages_with_tool_calls() {
        use forge_types::{ToolCall, ToolFunction};
        let history = vec![HistoryItem {
            role: Role::Assistant,
            content: MessageContent::Text("calling tool".into()),
            tool_calls: Some(vec![ToolCall {
                id: Some("call_1".into()),
                tool_type: Some("function".into()),
                function: ToolFunction {
                    name: "bash".into(),
                    arguments: serde_json::json!({"command": "ls"}),
                },
            }]),
            ..Default::default()
        }];
        let msgs = history_to_messages(&history);
        assert!(msgs[0]["tool_calls"].is_array());
        let tc = &msgs[0]["tool_calls"][0];
        assert_eq!(tc["function"]["name"], "bash");
        assert_eq!(tc["type"], "function");
    }
}
