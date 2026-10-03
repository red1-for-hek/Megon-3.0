use std::sync::Mutex;

use async_trait::async_trait;
use serde_json::{json, Value};

use forge_types::{
    ContentBlock, ForgeError, History, HistoryItem, MessageContent, ModelOutput, Role,
    ThinkingBlock, ToolCall, ToolFunction,
};

use crate::pricing::calculate_cost;
use crate::traits::{AbstractModel, InstanceStats};

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const DEFAULT_MAX_TOKENS: u32 = 16384;

#[derive(Debug, Clone)]
pub struct AnthropicConfig {
    pub api_key: String,
    pub model_name: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub extended_thinking: bool,
    pub thinking_budget_tokens: Option<u32>,
    pub cache_last_n: usize,
    /// Cost per million input tokens in USD. None = unknown (cost tracked as 0).
    pub cost_per_million_input: Option<f64>,
    /// Cost per million output tokens in USD. None = unknown (cost tracked as 0).
    pub cost_per_million_output: Option<f64>,
}

impl AnthropicConfig {
    pub fn new(api_key: &str, model_name: &str) -> Self {
        Self {
            api_key: api_key.into(),
            model_name: model_name.into(),
            temperature: 1.0,
            max_tokens: DEFAULT_MAX_TOKENS,
            extended_thinking: false,
            thinking_budget_tokens: None,
            cache_last_n: 0,
            cost_per_million_input: None,
            cost_per_million_output: None,
        }
    }
}

pub struct AnthropicModel {
    config: AnthropicConfig,
    client: reqwest::Client,
    stats: Mutex<InstanceStats>,
}

impl AnthropicModel {
    pub fn new(config: AnthropicConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
            stats: Mutex::new(InstanceStats::default()),
        }
    }
}

/// Split History into (system_prompt, non-system items).
pub fn split_system_message(history: &History) -> (Option<String>, Vec<&HistoryItem>) {
    let mut system_text: Option<String> = None;
    let mut rest = Vec::new();

    for item in history {
        if item.role == Role::System {
            let text = match &item.content {
                MessageContent::Text(t) => t.clone(),
                MessageContent::Blocks(blocks) => blocks
                    .iter()
                    .filter_map(|b| {
                        if let ContentBlock::Text { text } = b {
                            Some(text.as_str())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n"),
            };
            system_text = Some(
                system_text
                    .map(|s| format!("{}\n{}", s, text))
                    .unwrap_or(text),
            );
        } else {
            rest.push(item);
        }
    }

    (system_text, rest)
}

/// Convert history items to Anthropic message format, applying cache_control to last N user messages.
pub fn history_to_anthropic_messages(
    history: &History,
    apply_cache: bool,
    cache_last_n: usize,
) -> Vec<Value> {
    let (_, items) = split_system_message(history);

    // Find indices of user messages for cache marking
    let user_indices: Vec<usize> = items
        .iter()
        .enumerate()
        .filter(|(_, item)| item.role == Role::User)
        .map(|(i, _)| i)
        .collect();

    let cache_from: Option<usize> = if apply_cache && cache_last_n > 0 && !user_indices.is_empty() {
        let start = user_indices.len().saturating_sub(cache_last_n);
        user_indices.get(start).copied()
    } else {
        None
    };

    // Anthropic supports caching any message type (user or assistant).
    // Applying to the last N items (regardless of role) matches the TS reference behavior.
    items
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            let apply_cache_here = cache_from.map(|from| idx >= from).unwrap_or(false);
            anthropic_item(item, apply_cache_here)
        })
        .collect()
}

fn anthropic_item(item: &HistoryItem, apply_cache: bool) -> Value {
    let role = match item.role {
        Role::User | Role::Tool | Role::System => "user",
        Role::Assistant => "assistant",
    };

    let mut content = item_content_to_anthropic(item);

    if apply_cache {
        // Add cache_control to the last content block
        if let Some(last) = content.as_array_mut().and_then(|arr| arr.last_mut()) {
            last["cache_control"] = json!({"type": "ephemeral"});
        }
    }

    json!({
        "role": role,
        "content": content,
    })
}

fn item_content_to_anthropic(item: &HistoryItem) -> Value {
    // Handle tool-role messages (tool results)
    if item.role == Role::Tool {
        let text = match &item.content {
            MessageContent::Text(t) => t.clone(),
            MessageContent::Blocks(blocks) => blocks
                .iter()
                .filter_map(|b| {
                    if let ContentBlock::Text { text } = b {
                        Some(text.as_str())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("\n"),
        };

        let tool_use_id = item
            .tool_call_ids
            .as_ref()
            .and_then(|ids| ids.first())
            .cloned()
            .unwrap_or_default();

        return json!([{
            "type": "tool_result",
            "tool_use_id": tool_use_id,
            "content": text,
        }]);
    }

    let mut blocks: Vec<Value> = Vec::new();

    // Add thinking blocks first if present (assistant)
    if let Some(thinking) = &item.thinking_blocks {
        for tb in thinking {
            blocks.push(json!({
                "type": "thinking",
                "thinking": tb.content,
            }));
        }
    }

    // Add main content
    match &item.content {
        MessageContent::Text(t) => {
            if !t.is_empty() {
                blocks.push(json!({"type": "text", "text": t}));
            }
        }
        MessageContent::Blocks(bks) => {
            for b in bks {
                blocks.push(content_block_to_value(b));
            }
        }
    }

    // Add tool_calls as tool_use blocks (for assistant messages)
    if let Some(ref tcs) = item.tool_calls {
        for tc in tcs {
            blocks.push(json!({
                "type": "tool_use",
                "id": tc.id.as_deref().unwrap_or(""),
                "name": tc.function.name,
                "input": tc.function.arguments,
            }));
        }
    }

    if blocks.is_empty() {
        json!([{"type": "text", "text": ""}])
    } else {
        Value::Array(blocks)
    }
}

fn content_block_to_value(block: &ContentBlock) -> Value {
    match block {
        ContentBlock::Text { text } => json!({"type": "text", "text": text}),
        ContentBlock::Image { image_url } => json!({
            "type": "image",
            "source": {"type": "url", "url": image_url.url},
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

fn parse_anthropic_response(json: &Value) -> Result<ModelOutput, ForgeError> {
    let content = json["content"]
        .as_array()
        .ok_or_else(|| ForgeError::Model("missing content array".into()))?;

    let mut message_parts: Vec<String> = Vec::new();
    let mut tool_calls: Vec<ToolCall> = Vec::new();
    let mut thinking_blocks: Vec<ThinkingBlock> = Vec::new();

    for block in content {
        match block["type"].as_str().unwrap_or("") {
            "text" => {
                if let Some(t) = block["text"].as_str() {
                    message_parts.push(t.to_string());
                }
            }
            "tool_use" => {
                let id = block["id"].as_str().map(String::from);
                let name = match block["name"].as_str() {
                    Some(n) if !n.is_empty() => n.to_string(),
                    _ => continue, // skip malformed tool_use blocks
                };
                let input = block["input"].clone();
                tool_calls.push(ToolCall {
                    id,
                    tool_type: Some("function".to_string()),
                    function: ToolFunction {
                        name,
                        arguments: input,
                    },
                });
            }
            "thinking" => {
                if let Some(t) = block["thinking"].as_str() {
                    thinking_blocks.push(ThinkingBlock::new(t));
                }
            }
            _ => {}
        }
    }

    let input_tokens = json["usage"]["input_tokens"].as_u64().map(|v| v as u32);
    let output_tokens = json["usage"]["output_tokens"].as_u64().map(|v| v as u32);

    Ok(ModelOutput {
        message: message_parts.join(""),
        tool_calls: if tool_calls.is_empty() { None } else { Some(tool_calls) },
        thinking_blocks: if thinking_blocks.is_empty() { None } else { Some(thinking_blocks) },
        input_tokens,
        output_tokens,
        cost: None, // will be filled after
    })
}

#[async_trait]
impl AbstractModel for AnthropicModel {
    async fn query(&self, history: &History) -> Result<ModelOutput, ForgeError> {
        let (system_prompt, _) = split_system_message(history);
        let messages = history_to_anthropic_messages(
            history,
            self.config.cache_last_n > 0,
            self.config.cache_last_n,
        );

        let mut body = json!({
            "model": self.config.model_name,
            "max_tokens": self.config.max_tokens,
            "temperature": self.config.temperature,
            "messages": messages,
        });

        if let Some(sys) = system_prompt {
            body["system"] = Value::String(sys);
        }

        if self.config.extended_thinking {
            let budget = self.config.thinking_budget_tokens.unwrap_or(8000);
            body["thinking"] = json!({
                "type": "enabled",
                "budget_tokens": budget,
            });
        }

        let resp = self
            .client
            .post(ANTHROPIC_API_URL)
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&body)
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

        let mut output = parse_anthropic_response(&json)?;

        let cost = calculate_cost(
            self.config.cost_per_million_input,
            self.config.cost_per_million_output,
            output.input_tokens.unwrap_or(0),
            output.output_tokens.unwrap_or(0),
        );
        output.cost = Some(cost);

        {
            let mut stats = self.stats.lock().unwrap_or_else(|p| p.into_inner());
            stats.add_tokens(
                output.input_tokens.unwrap_or(0),
                output.output_tokens.unwrap_or(0),
                cost,
            );
        }

        Ok(output)
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
    fn config_builder() {
        let cfg = AnthropicConfig::new("sk-ant", "claude-3-5-sonnet-20241022");
        assert_eq!(cfg.max_tokens, DEFAULT_MAX_TOKENS);
        assert!(!cfg.extended_thinking);
        assert!(cfg.cost_per_million_input.is_none());
        assert!(cfg.cost_per_million_output.is_none());
        assert_eq!(cfg.temperature, 1.0);
    }

    #[test]
    fn split_system_message_finds_system() {
        let history = vec![
            HistoryItem {
                role: Role::System,
                content: MessageContent::Text("You are an agent.".into()),
                ..Default::default()
            },
            HistoryItem {
                role: Role::User,
                content: MessageContent::Text("Hello".into()),
                ..Default::default()
            },
        ];
        let (sys, rest) = split_system_message(&history);
        assert_eq!(sys.as_deref(), Some("You are an agent."));
        assert_eq!(rest.len(), 1);
        assert_eq!(rest[0].role, Role::User);
    }

    #[test]
    fn history_to_anthropic_messages_skips_system() {
        let history = vec![
            HistoryItem {
                role: Role::System,
                content: MessageContent::Text("System prompt".into()),
                ..Default::default()
            },
            HistoryItem {
                role: Role::User,
                content: MessageContent::Text("User msg".into()),
                ..Default::default()
            },
        ];
        let msgs = history_to_anthropic_messages(&history, false, 0);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0]["role"], "user");
    }

    #[test]
    fn cache_control_applied_to_last_n() {
        let history: Vec<HistoryItem> = (0..4)
            .map(|i| HistoryItem {
                role: if i % 2 == 0 { Role::User } else { Role::Assistant },
                content: MessageContent::Text(format!("msg {}", i)),
                ..Default::default()
            })
            .collect();

        // 2 user messages, cache_last_n=1 → only last user msg gets cache
        let msgs = history_to_anthropic_messages(&history, true, 1);
        // The last user message (idx=2 in items after stripping system) gets cache
        // idx 0 = user, idx 1 = assistant, idx 2 = user (last) → has cache_control
        let last_user_content = &msgs[2]["content"];
        let arr = last_user_content.as_array()
            .expect("last user message content should be an array");
        let last_block = arr.last().expect("content array should be non-empty");
        assert!(last_block.get("cache_control").is_some(), "last user msg should have cache_control");

        // first user message should NOT have cache
        let first_user_content = &msgs[0]["content"];
        let arr = first_user_content.as_array()
            .expect("first user message content should be an array");
        let last_block = arr.last().expect("content array should be non-empty");
        assert!(last_block.get("cache_control").is_none(), "first user msg should not have cache_control");
    }

    #[test]
    fn tool_role_becomes_tool_result_block() {
        let history = vec![HistoryItem {
            role: Role::Tool,
            content: MessageContent::Text("command output".into()),
            tool_call_ids: Some(vec!["tc_1".into()]),
            ..Default::default()
        }];
        let msgs = history_to_anthropic_messages(&history, false, 0);
        assert_eq!(msgs[0]["role"], "user");
        let content = &msgs[0]["content"];
        assert!(content.is_array());
        assert_eq!(content[0]["type"], "tool_result");
        assert_eq!(content[0]["tool_use_id"], "tc_1");
    }
}
