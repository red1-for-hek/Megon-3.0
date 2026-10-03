# Forge Phase 2 — Model Adapters Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement `forge-model` — the model trait and all LLM adapters (Anthropic, OpenAI-compat, Human, Replay, InstantSubmit) with cost tracking and stats.

**Architecture:** `AbstractModel` async trait. Each adapter is a separate file. No coupling to ElizaOS — these adapters are called directly by the agent loop.

**Tech Stack:** forge-types, reqwest 0.12 (json + stream features), tokio, async-trait, serde_json

**Prerequisite:** Phase 1 complete. `cargo test -p forge-types -p forge-tools` passes.

---

## File Map

```
forge/crates/forge-model/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── traits.rs          # AbstractModel trait, InstanceStats, GlobalStats
    ├── anthropic.rs       # Anthropic Claude API adapter
    ├── openai_compat.rs   # Generic OpenAI-compatible adapter
    ├── human.rs           # Interactive stdin/stdout model
    ├── replay.rs          # Replay model from .traj file
    ├── instant_submit.rs  # Always returns empty submit (for testing)
    └── pricing.rs         # Cost per token per model
```

---

## Task 1: Add forge-model to Workspace

**Files:**
- Modify: `forge/Cargo.toml`
- Create: `forge/crates/forge-model/Cargo.toml`

- [ ] **Step 1: Add to workspace**

```toml
# forge/Cargo.toml — add to [workspace] members
"crates/forge-model",
```

Add to `[workspace.dependencies]`:
```toml
reqwest = { version = "0.12", features = ["json", "stream"] }
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"
```

- [ ] **Step 2: Write Cargo.toml**

```toml
# forge/crates/forge-model/Cargo.toml
[package]
name = "forge-model"
version = "0.1.0"
edition = "2021"

[dependencies]
forge-types = { path = "../forge-types" }
serde = { workspace = true }
serde_json = { workspace = true }
reqwest = { workspace = true }
tokio = { workspace = true }
async-trait = { workspace = true }
```

- [ ] **Step 3: Create empty lib.rs and verify**

```bash
mkdir -p forge/crates/forge-model/src
echo "pub mod traits;" > forge/crates/forge-model/src/lib.rs
echo "" > forge/crates/forge-model/src/traits.rs
cd forge && cargo check -p forge-model 2>&1
```

Expected: `Finished` with no errors.

---

## Task 2: AbstractModel Trait + Stats

**Files:**
- Modify: `forge/crates/forge-model/src/traits.rs`

- [ ] **Step 1: Write tests**

```rust
// forge/crates/forge-model/src/traits.rs (bottom)
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instance_stats_accumulates() {
        let mut stats = InstanceStats::default();
        stats.add_tokens(100, 50, 0.01);
        stats.add_tokens(200, 100, 0.02);
        assert_eq!(stats.total_input_tokens, 300);
        assert_eq!(stats.total_output_tokens, 150);
        assert!((stats.total_cost - 0.03).abs() < 1e-9);
        assert_eq!(stats.api_calls, 2);
    }

    #[test]
    fn cost_limit_exceeded() {
        let mut stats = InstanceStats::default();
        stats.add_tokens(0, 0, 5.0);
        assert!(stats.check_instance_cost_limit(3.0).is_err());
        assert!(stats.check_instance_cost_limit(10.0).is_ok());
    }

    #[test]
    fn call_limit_exceeded() {
        let mut stats = InstanceStats::default();
        for _ in 0..5 { stats.add_tokens(1, 1, 0.001); }
        assert!(stats.check_call_limit(3).is_err());
        assert!(stats.check_call_limit(10).is_ok());
    }
}
```

- [ ] **Step 2: Run — expect compile failure**

```bash
cd forge && cargo test -p forge-model 2>&1 | head -10
```

- [ ] **Step 3: Implement traits.rs**

```rust
// forge/crates/forge-model/src/traits.rs
use async_trait::async_trait;
use forge_types::{ForgeError, History, ModelOutput};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Default)]
pub struct InstanceStats {
    pub total_input_tokens: u32,
    pub total_output_tokens: u32,
    pub total_cost: f64,
    pub api_calls: u32,
}

impl InstanceStats {
    pub fn add_tokens(&mut self, input: u32, output: u32, cost: f64) {
        self.total_input_tokens += input;
        self.total_output_tokens += output;
        self.total_cost += cost;
        self.api_calls += 1;
    }

    pub fn check_instance_cost_limit(&self, limit: f64) -> Result<(), ForgeError> {
        if limit > 0.0 && self.total_cost >= limit {
            Err(ForgeError::InstanceCostLimitExceeded)
        } else {
            Ok(())
        }
    }

    pub fn check_call_limit(&self, limit: u32) -> Result<(), ForgeError> {
        if limit > 0 && self.api_calls >= limit {
            Err(ForgeError::InstanceCallLimitExceeded)
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct GlobalStats {
    pub total_cost: f64,
}

impl GlobalStats {
    pub fn check_total_cost_limit(&self, limit: f64) -> Result<(), ForgeError> {
        if limit > 0.0 && self.total_cost >= limit {
            Err(ForgeError::TotalCostLimitExceeded)
        } else {
            Ok(())
        }
    }
}

#[async_trait]
pub trait AbstractModel: Send + Sync {
    async fn query(&self, history: &History) -> Result<ModelOutput, ForgeError>;
    fn stats(&self) -> InstanceStats;
    fn reset_stats(&self);
}

pub type SharedModel = Arc<dyn AbstractModel>;
```

- [ ] **Step 4: Run tests**

```bash
cd forge && cargo test -p forge-model traits 2>&1
```

Expected: 3 tests pass.

---

## Task 3: Pricing Table

**Files:**
- Create: `forge/crates/forge-model/src/pricing.rs`

- [ ] **Step 1: Write test**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_model_returns_cost() {
        let (inp, out) = cost_per_million("claude-sonnet-4-6");
        assert!(inp > 0.0);
        assert!(out > 0.0);
    }

    #[test]
    fn unknown_model_returns_zero() {
        let (inp, out) = cost_per_million("unknown-model-xyz");
        assert_eq!(inp, 0.0);
        assert_eq!(out, 0.0);
    }

    #[test]
    fn calculate_cost_correct() {
        // claude-sonnet-4-6: $3/$15 per million tokens
        let cost = calculate_cost("claude-sonnet-4-6", 1_000_000, 1_000_000);
        assert!((cost - 18.0).abs() < 0.01);
    }
}
```

- [ ] **Step 2: Implement pricing.rs**

```rust
// forge/crates/forge-model/src/pricing.rs

/// Returns (input_cost_per_million, output_cost_per_million) in USD
pub fn cost_per_million(model: &str) -> (f64, f64) {
    let model = model.to_lowercase();
    let model = model.as_str();
    match model {
        m if m.contains("claude-opus-4") => (15.0, 75.0),
        m if m.contains("claude-sonnet-4") => (3.0, 15.0),
        m if m.contains("claude-haiku-4") => (0.8, 4.0),
        m if m.contains("gpt-4o") && m.contains("mini") => (0.15, 0.6),
        m if m.contains("gpt-4o") => (2.5, 10.0),
        m if m.contains("gpt-4-turbo") => (10.0, 30.0),
        m if m.contains("gpt-3.5") => (0.5, 1.5),
        m if m.contains("o1-mini") => (3.0, 12.0),
        m if m.contains("o1") => (15.0, 60.0),
        m if m.contains("gemini-1.5-pro") => (1.25, 5.0),
        m if m.contains("gemini-1.5-flash") => (0.075, 0.3),
        m if m.contains("mistral-large") => (2.0, 6.0),
        m if m.contains("mistral-small") => (0.1, 0.3),
        _ => (0.0, 0.0),
    }
}

pub fn calculate_cost(model: &str, input_tokens: u32, output_tokens: u32) -> f64 {
    let (inp_rate, out_rate) = cost_per_million(model);
    (input_tokens as f64 / 1_000_000.0) * inp_rate
        + (output_tokens as f64 / 1_000_000.0) * out_rate
}
```

- [ ] **Step 3: Export and test**

```rust
// forge/crates/forge-model/src/lib.rs
pub mod pricing;
pub mod traits;
pub use traits::{AbstractModel, GlobalStats, InstanceStats, SharedModel};
pub use pricing::{calculate_cost, cost_per_million};
```

```bash
cd forge && cargo test -p forge-model pricing 2>&1
```

Expected: 3 tests pass.

---

## Task 4: OpenAI-Compatible Adapter

**Files:**
- Create: `forge/crates/forge-model/src/openai_compat.rs`

- [ ] **Step 1: Write tests**

```rust
// forge/crates/forge-model/src/openai_compat.rs (bottom)
#[cfg(test)]
mod tests {
    use super::*;
    use forge_types::{HistoryItem, Role, MessageContent};

    #[test]
    fn config_builder_defaults() {
        let cfg = OpenAICompatConfig::new(
            "https://api.openai.com/v1",
            "sk-test",
            "gpt-4o",
        );
        assert_eq!(cfg.temperature, 1.0);
        assert_eq!(cfg.base_url, "https://api.openai.com/v1");
    }

    #[test]
    fn history_to_messages_converts_roles() {
        let history = vec![
            HistoryItem {
                role: Role::System,
                content: MessageContent::Text("sys".into()),
                ..Default::default()
            },
            HistoryItem {
                role: Role::User,
                content: MessageContent::Text("hi".into()),
                ..Default::default()
            },
        ];
        let messages = history_to_messages(&history);
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(messages[1]["role"], "user");
        assert_eq!(messages[1]["content"], "hi");
    }
}
```

- [ ] **Step 2: Run — expect compile failure**

```bash
cd forge && cargo test -p forge-model openai_compat 2>&1 | head -10
```

- [ ] **Step 3: Implement openai_compat.rs**

```rust
// forge/crates/forge-model/src/openai_compat.rs
use async_trait::async_trait;
use forge_types::{ForgeError, History, HistoryItem, MessageContent, ModelOutput, Role, ToolCall, ToolFunction};
use serde_json::{json, Value};
use std::sync::Mutex;
use crate::{calculate_cost, AbstractModel, InstanceStats};

#[derive(Debug, Clone)]
pub struct OpenAICompatConfig {
    pub base_url: String,
    pub api_key: String,
    pub model_name: String,
    pub temperature: f32,
    pub max_tokens: Option<u32>,
    pub extra_headers: std::collections::HashMap<String, String>,
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

pub fn history_to_messages(history: &History) -> Vec<Value> {
    history.iter().map(|item| {
        let role = match item.role {
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::System => "system",
        };
        let content = match &item.content {
            MessageContent::Text(t) => json!(t),
            MessageContent::Blocks(blocks) => {
                let parts: Vec<Value> = blocks.iter().map(|b| {
                    match b {
                        forge_types::ContentBlock::Text { text } =>
                            json!({"type": "text", "text": text}),
                        forge_types::ContentBlock::Image { image_url } =>
                            json!({"type": "image_url", "image_url": {"url": image_url.url}}),
                        _ => json!({"type": "text", "text": ""}),
                    }
                }).collect();
                json!(parts)
            }
        };
        json!({"role": role, "content": content})
    }).collect()
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
        if let Some(max_tok) = self.config.max_tokens {
            body["max_tokens"] = json!(max_tok);
        }

        let mut req = self.client
            .post(format!("{}/chat/completions", self.config.base_url))
            .bearer_auth(&self.config.api_key)
            .json(&body);

        for (k, v) in &self.config.extra_headers {
            req = req.header(k, v);
        }

        let resp = req.send().await
            .map_err(|e| ForgeError::Http(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(ForgeError::Http(format!("HTTP {}: {}", status, body)));
        }

        let json: Value = resp.json().await
            .map_err(|e| ForgeError::Http(e.to_string()))?;

        let message = json["choices"][0]["message"]["content"]
            .as_str().unwrap_or("").to_string();

        let tool_calls: Option<Vec<ToolCall>> = json["choices"][0]["message"]["tool_calls"]
            .as_array()
            .map(|arr| {
                arr.iter().filter_map(|tc| {
                    let name = tc["function"]["name"].as_str()?.to_string();
                    let args: Value = serde_json::from_str(
                        tc["function"]["arguments"].as_str().unwrap_or("{}")
                    ).unwrap_or(json!({}));
                    Some(ToolCall {
                        id: tc["id"].as_str().map(|s| s.to_string()),
                        function: ToolFunction { name, arguments: args },
                    })
                }).collect()
            });

        let input_tokens = json["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as u32;
        let output_tokens = json["usage"]["completion_tokens"].as_u64().unwrap_or(0) as u32;
        let cost = calculate_cost(&self.config.model_name, input_tokens, output_tokens);

        let mut stats = self.stats.lock().unwrap();
        stats.add_tokens(input_tokens, output_tokens, cost);

        Ok(ModelOutput {
            message,
            tool_calls: tool_calls.filter(|v| !v.is_empty()),
            thinking_blocks: None,
            input_tokens,
            output_tokens,
            cost,
        })
    }

    fn stats(&self) -> InstanceStats {
        self.stats.lock().unwrap().clone()
    }

    fn reset_stats(&self) {
        *self.stats.lock().unwrap() = InstanceStats::default();
    }
}
```

- [ ] **Step 4: Run tests**

```bash
cd forge && cargo test -p forge-model openai_compat 2>&1
```

Expected: 2 tests pass (no network calls in tests).

---

## Task 5: Anthropic Adapter

**Files:**
- Create: `forge/crates/forge-model/src/anthropic.rs`

- [ ] **Step 1: Write tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use forge_types::{HistoryItem, Role, MessageContent};

    #[test]
    fn config_builder() {
        let cfg = AnthropicConfig::new("sk-ant-test", "claude-sonnet-4-6");
        assert_eq!(cfg.max_tokens, 16384);
        assert!(!cfg.extended_thinking);
    }

    #[test]
    fn history_to_anthropic_messages_skips_system() {
        let history = vec![
            HistoryItem { role: Role::System, content: MessageContent::Text("sys".into()), ..Default::default() },
            HistoryItem { role: Role::User, content: MessageContent::Text("hello".into()), ..Default::default() },
        ];
        let (sys, msgs) = split_system_message(&history);
        assert!(sys.is_some());
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0]["role"], "user");
    }

    #[test]
    fn cache_control_applied_to_last_n() {
        let history: Vec<HistoryItem> = (0..5).map(|i| HistoryItem {
            role: if i % 2 == 0 { Role::User } else { Role::Assistant },
            content: MessageContent::Text(format!("msg {}", i)),
            ..Default::default()
        }).collect();
        let msgs = history_to_anthropic_messages(&history, true, 2);
        // last 2 non-system messages should have cache_control
        let cached: Vec<_> = msgs.iter()
            .filter(|m| m.get("cache_control").is_some())
            .collect();
        assert_eq!(cached.len(), 2);
    }
}
```

- [ ] **Step 2: Implement anthropic.rs**

```rust
// forge/crates/forge-model/src/anthropic.rs
use async_trait::async_trait;
use forge_types::{
    ContentBlock, ForgeError, History, HistoryItem, MessageContent,
    ModelOutput, Role, ThinkingBlock, ToolCall, ToolFunction,
};
use serde_json::{json, Value};
use std::sync::Mutex;
use crate::{calculate_cost, AbstractModel, InstanceStats};

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";

#[derive(Debug, Clone)]
pub struct AnthropicConfig {
    pub api_key: String,
    pub model_name: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub extended_thinking: bool,
    pub thinking_budget_tokens: Option<u32>,
    pub cache_control: bool,
    pub cache_last_n_messages: usize,
}

impl AnthropicConfig {
    pub fn new(api_key: &str, model_name: &str) -> Self {
        Self {
            api_key: api_key.into(),
            model_name: model_name.into(),
            temperature: 1.0,
            max_tokens: 16384,
            extended_thinking: false,
            thinking_budget_tokens: None,
            cache_control: false,
            cache_last_n_messages: 2,
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

pub fn split_system_message(history: &History) -> (Option<String>, Vec<&HistoryItem>) {
    let system = history.iter()
        .find(|h| matches!(h.role, Role::System))
        .and_then(|h| match &h.content {
            MessageContent::Text(t) => Some(t.clone()),
            _ => None,
        });
    let rest: Vec<&HistoryItem> = history.iter()
        .filter(|h| !matches!(h.role, Role::System))
        .collect();
    (system, rest)
}

pub fn history_to_anthropic_messages(
    history: &History,
    apply_cache: bool,
    cache_last_n: usize,
) -> Vec<Value> {
    let (_, items) = split_system_message(history);
    let n = items.len();
    items.iter().enumerate().map(|(i, item)| {
        let role = match item.role {
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::System => "user", // shouldn't happen after split
        };
        let content = match &item.content {
            MessageContent::Text(t) => json!(t),
            MessageContent::Blocks(blocks) => {
                json!(blocks.iter().map(|b| match b {
                    ContentBlock::Text { text } => json!({"type": "text", "text": text}),
                    ContentBlock::Image { image_url } => json!({
                        "type": "image",
                        "source": {"type": "url", "url": image_url.url}
                    }),
                    ContentBlock::Thinking { thinking } => json!({
                        "type": "thinking", "thinking": thinking
                    }),
                    _ => json!({"type": "text", "text": ""}),
                }).collect::<Vec<_>>())
            }
        };
        let mut msg = json!({"role": role, "content": content});
        // Apply cache_control to last N messages
        if apply_cache && cache_last_n > 0 && i >= n.saturating_sub(cache_last_n) {
            msg["cache_control"] = json!({"type": "ephemeral"});
        }
        msg
    }).collect()
}

#[async_trait]
impl AbstractModel for AnthropicModel {
    async fn query(&self, history: &History) -> Result<ModelOutput, ForgeError> {
        let (system, _) = split_system_message(history);
        let messages = history_to_anthropic_messages(
            history,
            self.config.cache_control,
            self.config.cache_last_n_messages,
        );

        let mut body = json!({
            "model": self.config.model_name,
            "max_tokens": self.config.max_tokens,
            "messages": messages,
        });

        if let Some(sys) = system {
            body["system"] = json!(sys);
        }

        if self.config.extended_thinking {
            let budget = self.config.thinking_budget_tokens.unwrap_or(10000);
            body["thinking"] = json!({"type": "enabled", "budget_tokens": budget});
            body["temperature"] = json!(1.0); // required for extended thinking
        } else {
            body["temperature"] = json!(self.config.temperature);
        }

        let resp = self.client
            .post(ANTHROPIC_API_URL)
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ForgeError::Http(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            if status.as_u16() == 400 && text.contains("context_length_exceeded") {
                return Err(ForgeError::ContextWindowExceeded);
            }
            return Err(ForgeError::Http(format!("HTTP {}: {}", status, text)));
        }

        let json: Value = resp.json().await
            .map_err(|e| ForgeError::Http(e.to_string()))?;

        let mut message = String::new();
        let mut thinking_blocks = Vec::new();
        let mut tool_calls = Vec::new();

        if let Some(content) = json["content"].as_array() {
            for block in content {
                match block["type"].as_str() {
                    Some("text") => {
                        message = block["text"].as_str().unwrap_or("").to_string();
                    }
                    Some("thinking") => {
                        thinking_blocks.push(ThinkingBlock {
                            content: block["thinking"].as_str().unwrap_or("").to_string(),
                        });
                    }
                    Some("tool_use") => {
                        let name = block["name"].as_str().unwrap_or("").to_string();
                        let input = block["input"].clone();
                        tool_calls.push(ToolCall {
                            id: block["id"].as_str().map(|s| s.to_string()),
                            function: ToolFunction { name, arguments: input },
                        });
                    }
                    _ => {}
                }
            }
        }

        let input_tokens = json["usage"]["input_tokens"].as_u64().unwrap_or(0) as u32;
        let output_tokens = json["usage"]["output_tokens"].as_u64().unwrap_or(0) as u32;
        let cost = calculate_cost(&self.config.model_name, input_tokens, output_tokens);

        {
            let mut stats = self.stats.lock().unwrap();
            stats.add_tokens(input_tokens, output_tokens, cost);
        }

        Ok(ModelOutput {
            message,
            tool_calls: if tool_calls.is_empty() { None } else { Some(tool_calls) },
            thinking_blocks: if thinking_blocks.is_empty() { None } else { Some(thinking_blocks) },
            input_tokens,
            output_tokens,
            cost,
        })
    }

    fn stats(&self) -> InstanceStats {
        self.stats.lock().unwrap().clone()
    }

    fn reset_stats(&self) {
        *self.stats.lock().unwrap() = InstanceStats::default();
    }
}
```

- [ ] **Step 3: Run tests**

```bash
cd forge && cargo test -p forge-model anthropic 2>&1
```

Expected: 3 tests pass.

---

## Task 6: Instant Submit + Replay Models

**Files:**
- Create: `forge/crates/forge-model/src/instant_submit.rs`
- Create: `forge/crates/forge-model/src/replay.rs`
- Create: `forge/crates/forge-model/src/human.rs`

- [ ] **Step 1: Implement instant_submit.rs**

```rust
// forge/crates/forge-model/src/instant_submit.rs
use async_trait::async_trait;
use forge_types::{ForgeError, History, ModelOutput, special_tokens};
use std::sync::Mutex;
use crate::{AbstractModel, InstanceStats};

/// Always returns a submit action — used in tests and dry runs
pub struct InstantSubmitModel {
    stats: Mutex<InstanceStats>,
}

impl InstantSubmitModel {
    pub fn new() -> Self {
        Self { stats: Mutex::new(InstanceStats::default()) }
    }
}

#[async_trait]
impl AbstractModel for InstantSubmitModel {
    async fn query(&self, _history: &History) -> Result<ModelOutput, ForgeError> {
        self.stats.lock().unwrap().add_tokens(0, 0, 0.0);
        Ok(ModelOutput {
            message: format!("submit\n{}", special_tokens::SUBMISSION),
            ..Default::default()
        })
    }
    fn stats(&self) -> InstanceStats { self.stats.lock().unwrap().clone() }
    fn reset_stats(&self) { *self.stats.lock().unwrap() = InstanceStats::default(); }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn returns_submission_token() {
        let model = InstantSubmitModel::new();
        let out = model.query(&[]).await.unwrap();
        assert!(out.message.contains(forge_types::special_tokens::SUBMISSION));
    }

    #[tokio::test]
    async fn increments_api_calls() {
        let model = InstantSubmitModel::new();
        model.query(&[]).await.unwrap();
        model.query(&[]).await.unwrap();
        assert_eq!(model.stats().api_calls, 2);
    }
}
```

- [ ] **Step 2: Implement replay.rs**

```rust
// forge/crates/forge-model/src/replay.rs
use async_trait::async_trait;
use forge_types::{ForgeError, History, ModelOutput, TrajFile};
use std::sync::Mutex;
use crate::{AbstractModel, InstanceStats};

/// Replays model outputs from a saved trajectory file
pub struct ReplayModel {
    responses: Vec<String>,
    cursor: Mutex<usize>,
    stats: Mutex<InstanceStats>,
}

impl ReplayModel {
    pub fn from_traj(traj: &TrajFile) -> Self {
        let responses: Vec<String> = traj.trajectory.iter()
            .map(|step| step.response.clone())
            .collect();
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
        let mut cursor = self.cursor.lock().unwrap();
        if *cursor >= self.responses.len() {
            return Err(ForgeError::Model("Replay exhausted".into()));
        }
        let message = self.responses[*cursor].clone();
        *cursor += 1;
        self.stats.lock().unwrap().add_tokens(0, 0, 0.0);
        Ok(ModelOutput { message, ..Default::default() })
    }
    fn stats(&self) -> InstanceStats { self.stats.lock().unwrap().clone() }
    fn reset_stats(&self) { *self.stats.lock().unwrap() = InstanceStats::default(); }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_types::{AgentInfo, TrajFile, TrajectoryStep};

    fn traj_with(responses: &[&str]) -> TrajFile {
        TrajFile {
            trajectory: responses.iter().map(|r| TrajectoryStep {
                response: r.to_string(), ..Default::default()
            }).collect(),
            history: vec![],
            info: AgentInfo::default(),
            replay_config: None,
            environment: "docker".into(),
        }
    }

    #[tokio::test]
    async fn replays_in_order() {
        let model = ReplayModel::from_traj(&traj_with(&["first", "second"]));
        let r1 = model.query(&[]).await.unwrap();
        let r2 = model.query(&[]).await.unwrap();
        assert_eq!(r1.message, "first");
        assert_eq!(r2.message, "second");
    }

    #[tokio::test]
    async fn errors_when_exhausted() {
        let model = ReplayModel::from_traj(&traj_with(&["only"]));
        model.query(&[]).await.unwrap();
        assert!(model.query(&[]).await.is_err());
    }
}
```

- [ ] **Step 3: Implement human.rs stub** (interactive — not unit tested)

```rust
// forge/crates/forge-model/src/human.rs
use async_trait::async_trait;
use forge_types::{ForgeError, History, ModelOutput};
use std::sync::Mutex;
use crate::{AbstractModel, InstanceStats};

/// Interactive model that reads actions from stdin
pub struct HumanModel {
    stats: Mutex<InstanceStats>,
}

impl HumanModel {
    pub fn new() -> Self {
        Self { stats: Mutex::new(InstanceStats::default()) }
    }
}

#[async_trait]
impl AbstractModel for HumanModel {
    async fn query(&self, history: &History) -> Result<ModelOutput, ForgeError> {
        if let Some(last) = history.last() {
            match &last.content {
                forge_types::MessageContent::Text(t) => eprintln!("\n[Observation]\n{}\n", t),
                _ => {}
            }
        }
        eprint!"> ");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)
            .map_err(|e| ForgeError::Io(e))?;
        self.stats.lock().unwrap().add_tokens(0, 0, 0.0);
        Ok(ModelOutput { message: input.trim().to_string(), ..Default::default() })
    }
    fn stats(&self) -> InstanceStats { self.stats.lock().unwrap().clone() }
    fn reset_stats(&self) { *self.stats.lock().unwrap() = InstanceStats::default(); }
}
```

- [ ] **Step 4: Export all from lib.rs**

```rust
// forge/crates/forge-model/src/lib.rs
pub mod anthropic;
pub mod human;
pub mod instant_submit;
pub mod openai_compat;
pub mod pricing;
pub mod replay;
pub mod traits;

pub use traits::{AbstractModel, GlobalStats, InstanceStats, SharedModel};
pub use pricing::{calculate_cost, cost_per_million};
pub use anthropic::{AnthropicConfig, AnthropicModel};
pub use openai_compat::{OpenAICompatConfig, OpenAICompatModel};
pub use instant_submit::InstantSubmitModel;
pub use replay::ReplayModel;
pub use human::HumanModel;
```

- [ ] **Step 5: Run full forge-model test suite**

```bash
cd forge && cargo test -p forge-model 2>&1
```

Expected: all tests pass (instant_submit + replay + stats + pricing + anthropic + openai_compat).

---

## Task 7: Phase 2 Verification

- [ ] **Step 1: Full build check across all crates so far**

```bash
cd forge && cargo build -p forge-types -p forge-tools -p forge-model 2>&1
```

Expected: `Finished` with no errors.

- [ ] **Step 2: Run all tests**

```bash
cd forge && cargo test -p forge-types -p forge-tools -p forge-model 2>&1
```

Expected: all tests pass, zero failures.

- [ ] **Step 3: Verify dependency isolation (forge-model must not import forge-tools)**

```bash
grep -r "forge-tools" forge/crates/forge-model/ 2>/dev/null
```

Expected: no output — they are siblings, not dependencies of each other.

---

**Phase 2 complete.** All model adapters implemented and tested. Move to Phase 3 (`forge-env` — Docker environment).
