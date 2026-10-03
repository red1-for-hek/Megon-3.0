use forge_types::history::{CacheControl, ContentBlock, History, HistoryItem, MessageContent, Role};
use forge_types::error::ForgeError;
use regex::Regex;

// ---------------------------------------------------------------------------
// Core trait
// ---------------------------------------------------------------------------

/// Takes a History and returns a processed History (may clone/modify items).
pub trait HistoryProcessor: Send + Sync {
    fn process(&self, history: &History) -> History;
}

pub type BoxedProcessor = Box<dyn HistoryProcessor>;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extract all text from a MessageContent (ignoring non-text blocks).
fn content_as_text(content: &MessageContent) -> String {
    match content {
        MessageContent::Text(s) => s.clone(),
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
    }
}

/// Replace text inside a MessageContent while preserving non-text blocks.
fn replace_text_in_content(content: &MessageContent, new_text: String) -> MessageContent {
    match content {
        MessageContent::Text(_) => MessageContent::Text(new_text),
        MessageContent::Blocks(blocks) => {
            // Collect non-text blocks to preserve them
            let non_text: Vec<ContentBlock> = blocks.iter()
                .filter(|b| !matches!(b, ContentBlock::Text { .. }))
                .cloned()
                .collect();

            // Build result: one merged text block + all non-text blocks
            let mut result = vec![ContentBlock::Text { text: new_text }];
            result.extend(non_text);
            MessageContent::Blocks(result)
        }
    }
}

// ---------------------------------------------------------------------------
// DefaultHistoryProcessor
// ---------------------------------------------------------------------------

/// Returns history unchanged (clone).
pub struct DefaultHistoryProcessor;

impl HistoryProcessor for DefaultHistoryProcessor {
    fn process(&self, history: &History) -> History {
        history.clone()
    }
}

// ---------------------------------------------------------------------------
// LastNObservations
// ---------------------------------------------------------------------------

/// Keep the last `n+1` observations; elide earlier ones with "Old environment output".
pub struct LastNObservations {
    pub n: usize,
}

impl Default for LastNObservations {
    fn default() -> Self {
        Self { n: 5 }
    }
}

impl HistoryProcessor for LastNObservations {
    fn process(&self, history: &History) -> History {
        if history.is_empty() {
            return history.clone();
        }

        // Detect the instance template: history[1] if it's a User message containing
        // "Instance template".
        let instance_template_idx: Option<usize> = if history.len() > 1
            && history[1].role == Role::User
            && content_as_text(&history[1].content).contains("Instance template")
        {
            Some(1)
        } else {
            None
        };

        // Collect indices of "observation" entries (User role), excluding the
        // instance template if present.
        let observation_indices: Vec<usize> = history
            .iter()
            .enumerate()
            .filter(|(i, item)| {
                item.role == Role::User
                    && instance_template_idx.map_or(true, |tmpl| *i != tmpl)
            })
            .map(|(i, _)| i)
            .collect();

        // We keep the last (n+1) observations; everything before that gets elided.
        let keep_from = if observation_indices.len() > self.n + 1 {
            let cutoff = observation_indices.len() - (self.n + 1);
            observation_indices[cutoff]
        } else {
            // Nothing to elide.
            return history.clone();
        };

        history
            .iter()
            .enumerate()
            .map(|(i, item)| {
                // Elide observations that come before the keep boundary.
                let is_observation = item.role == Role::User
                    && instance_template_idx.map_or(true, |tmpl| i != tmpl);
                if is_observation && i < keep_from {
                    HistoryItem {
                        content: MessageContent::Text("Old environment output".to_string()),
                        ..item.clone()
                    }
                } else {
                    item.clone()
                }
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// ClosedWindowHistoryProcessor
// ---------------------------------------------------------------------------

/// Keep first item (system msg) + elision marker + last `window_size` items.
pub struct ClosedWindowHistoryProcessor {
    pub window_size: usize,
}

impl Default for ClosedWindowHistoryProcessor {
    fn default() -> Self {
        Self { window_size: 10 }
    }
}

impl HistoryProcessor for ClosedWindowHistoryProcessor {
    fn process(&self, history: &History) -> History {
        if history.is_empty() || history.len() <= self.window_size {
            return history.clone();
        }

        let elided_count = history.len() - self.window_size - 1;
        let elision_item = HistoryItem {
            role: Role::User,
            content: MessageContent::Text(format!("[... {} messages elided ...]", elided_count)),
            message_type: Some(forge_types::history::MessageType::Observation),
            ..Default::default()
        };

        let mut result = Vec::with_capacity(self.window_size + 2);
        result.push(history[0].clone());
        result.push(elision_item);
        result.extend_from_slice(&history[history.len() - self.window_size..]);
        result
    }
}

// ---------------------------------------------------------------------------
// CacheControlHistoryProcessor
// ---------------------------------------------------------------------------

/// Mark the last `cache_last_n` items with `cache_control: { type: "ephemeral" }`.
pub struct CacheControlHistoryProcessor {
    pub cache_last_n: usize,
}

impl Default for CacheControlHistoryProcessor {
    fn default() -> Self {
        Self { cache_last_n: 5 }
    }
}

impl HistoryProcessor for CacheControlHistoryProcessor {
    fn process(&self, history: &History) -> History {
        if history.is_empty() {
            return history.clone();
        }

        let start = if history.len() > self.cache_last_n {
            history.len() - self.cache_last_n
        } else {
            0
        };

        history
            .iter()
            .enumerate()
            .map(|(i, item)| {
                if i >= start {
                    HistoryItem {
                        cache_control: Some(CacheControl {
                            kind: "ephemeral".to_string(),
                        }),
                        ..item.clone()
                    }
                } else {
                    item.clone()
                }
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// RemoveRegexHistoryProcessor
// ---------------------------------------------------------------------------

/// For each history item, apply regex replacements to text content.
pub struct RemoveRegexHistoryProcessor {
    pub patterns: Vec<Regex>,
}

impl Default for RemoveRegexHistoryProcessor {
    fn default() -> Self {
        Self { patterns: vec![] }
    }
}

impl RemoveRegexHistoryProcessor {
    pub fn new(patterns: Vec<Regex>) -> Self {
        Self { patterns }
    }
}

impl HistoryProcessor for RemoveRegexHistoryProcessor {
    fn process(&self, history: &History) -> History {
        if self.patterns.is_empty() {
            return history.clone();
        }

        history
            .iter()
            .map(|item| {
                let text = content_as_text(&item.content);
                let cleaned = self.patterns.iter().fold(text, |acc, re| {
                    re.replace_all(&acc, "").to_string()
                });
                HistoryItem {
                    content: replace_text_in_content(&item.content, cleaned),
                    ..item.clone()
                }
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// TagToolCallObservationsProcessor
// ---------------------------------------------------------------------------

/// For entries whose `action` field starts with one of the given function names,
/// add the specified tags (merged without duplicates).
pub struct TagToolCallObservationsProcessor {
    pub tags: Vec<String>,
    pub function_names: Vec<String>,
}

impl Default for TagToolCallObservationsProcessor {
    fn default() -> Self {
        Self {
            tags: vec![],
            function_names: vec![],
        }
    }
}

impl HistoryProcessor for TagToolCallObservationsProcessor {
    fn process(&self, history: &History) -> History {
        if self.tags.is_empty() || self.function_names.is_empty() {
            return history.clone();
        }

        history
            .iter()
            .map(|item| {
                let matches = item.action.as_ref().map_or(false, |action| {
                    self.function_names.iter().any(|fname| {
                        action == fname
                            || action.starts_with(&format!("{} ", fname))
                            || action.starts_with(&format!("{}\t", fname))
                    })
                });

                if matches {
                    let mut existing_tags = item.tags.clone().unwrap_or_default();
                    for tag in &self.tags {
                        if !existing_tags.contains(tag) {
                            existing_tags.push(tag.clone());
                        }
                    }
                    HistoryItem {
                        tags: Some(existing_tags),
                        ..item.clone()
                    }
                } else {
                    item.clone()
                }
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Chain
// ---------------------------------------------------------------------------

struct ChainedProcessor {
    processors: Vec<BoxedProcessor>,
}

impl HistoryProcessor for ChainedProcessor {
    fn process(&self, history: &History) -> History {
        self.processors
            .iter()
            .fold(history.clone(), |h, p| p.process(&h))
    }
}

/// Returns a single `BoxedProcessor` that applies each processor in sequence.
pub fn chain_processors(processors: Vec<BoxedProcessor>) -> BoxedProcessor {
    Box::new(ChainedProcessor { processors })
}

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

/// Create a processor from a JSON config object.
///
/// The config must have a `"type"` string field. Additional fields configure
/// the processor.
pub fn create_processor(config: &serde_json::Value) -> Result<BoxedProcessor, ForgeError> {
    let type_name = config["type"]
        .as_str()
        .ok_or_else(|| ForgeError::Config("history processor config missing 'type' field".to_string()))?;

    match type_name {
        "default" => Ok(Box::new(DefaultHistoryProcessor)),
        "last_n_observations" => {
            let n = config["n"].as_u64().unwrap_or(5) as usize;
            Ok(Box::new(LastNObservations { n }))
        }
        "closed_window" => {
            let window_size = config["window_size"].as_u64().unwrap_or(10) as usize;
            Ok(Box::new(ClosedWindowHistoryProcessor { window_size }))
        }
        "cache_control" => {
            let cache_last_n = config["cache_last_n"].as_u64().unwrap_or(5) as usize;
            Ok(Box::new(CacheControlHistoryProcessor { cache_last_n }))
        }
        "remove_regex" => {
            let patterns = if let Some(arr) = config["patterns"].as_array() {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| Regex::new(s).map_err(|e| ForgeError::Config(format!("invalid regex: {}", e))))
                    .collect::<Result<Vec<_>, _>>()?
            } else {
                vec![]
            };
            Ok(Box::new(RemoveRegexHistoryProcessor::new(patterns)))
        }
        "tag_tool_call_observations" => {
            let tags = if let Some(arr) = config["tags"].as_array() {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            } else {
                vec![]
            };
            let function_names = if let Some(arr) = config["function_names"].as_array() {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            } else {
                vec![]
            };
            Ok(Box::new(TagToolCallObservationsProcessor {
                tags,
                function_names,
            }))
        }
        other => Err(ForgeError::Config(format!(
            "unknown history processor type: {}",
            other
        ))),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use forge_types::history::{HistoryItem, MessageContent, Role};

    fn make_item(role: Role, text: &str) -> HistoryItem {
        HistoryItem {
            role,
            content: MessageContent::Text(text.to_string()),
            ..Default::default()
        }
    }

    fn make_user(text: &str) -> HistoryItem {
        make_item(Role::User, text)
    }

    fn make_assistant(text: &str) -> HistoryItem {
        make_item(Role::Assistant, text)
    }

    fn make_system(text: &str) -> HistoryItem {
        make_item(Role::System, text)
    }

    fn text_of(item: &HistoryItem) -> &str {
        if let MessageContent::Text(s) = &item.content {
            s
        } else {
            ""
        }
    }

    #[test]
    fn default_processor_returns_clone() {
        let history = vec![
            make_system("system"),
            make_user("hello"),
            make_assistant("world"),
        ];
        let proc = DefaultHistoryProcessor;
        let result = proc.process(&history);
        assert_eq!(result.len(), history.len());
        assert_eq!(text_of(&result[1]), "hello");
    }

    #[test]
    fn last_n_observations_elides_early() {
        // Build a history: system + 10 alternating assistant/user pairs
        let mut history = vec![make_system("system prompt")];
        for i in 0..10usize {
            history.push(make_assistant(&format!("action {}", i)));
            history.push(make_user(&format!("obs {}", i)));
        }
        // Total: 1 system + 20 items = 21 items
        // With n=2, keep last 3 observations, elide earlier ones

        let proc = LastNObservations { n: 2 };
        let result = proc.process(&history);

        // Count how many User items are "Old environment output" vs preserved.
        let elided: Vec<_> = result
            .iter()
            .filter(|it| it.role == Role::User && text_of(it) == "Old environment output")
            .collect();
        let preserved: Vec<_> = result
            .iter()
            .filter(|it| it.role == Role::User && text_of(it) != "Old environment output")
            .collect();

        // With n=2, keep n+1=3 observations. We have 10 observations, so 7 are elided.
        assert_eq!(elided.len(), 7, "expected 7 elided observations");
        assert_eq!(preserved.len(), 3, "expected 3 kept observations");
    }

    #[test]
    fn last_n_observations_no_elision_when_few() {
        let history = vec![
            make_system("system"),
            make_user("obs 1"),
            make_assistant("act 1"),
            make_user("obs 2"),
        ];
        let proc = LastNObservations { n: 5 };
        let result = proc.process(&history);
        // No elision needed
        assert_eq!(result.len(), 4);
        assert_eq!(text_of(&result[1]), "obs 1");
        assert_eq!(text_of(&result[3]), "obs 2");
    }

    #[test]
    fn closed_window_elides_middle() {
        let mut history = vec![make_system("system")];
        for i in 0..6usize {
            history.push(make_user(&format!("msg {}", i)));
        }
        // Total 7 items; window_size=3 → keep system + elision + last 3
        let proc = ClosedWindowHistoryProcessor { window_size: 3 };
        let result = proc.process(&history);

        assert_eq!(result.len(), 5); // system + elision + 3 items
        assert!(text_of(&result[1]).contains("elided"), "should contain elision message");
        // The elided count is 7 - 3 - 1 = 3
        assert!(text_of(&result[1]).contains("3"), "should mention 3 elided messages");
        assert_eq!(text_of(&result[2]), "msg 3");
        assert_eq!(text_of(&result[4]), "msg 5");
    }

    #[test]
    fn closed_window_no_elision_when_small() {
        let history = vec![
            make_system("system"),
            make_user("a"),
            make_user("b"),
        ];
        let proc = ClosedWindowHistoryProcessor { window_size: 10 };
        let result = proc.process(&history);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn cache_control_marks_last_n() {
        let history = vec![
            make_system("s"),
            make_user("u1"),
            make_user("u2"),
            make_user("u3"),
            make_user("u4"),
            make_user("u5"),
            make_user("u6"),
        ];
        let proc = CacheControlHistoryProcessor { cache_last_n: 3 };
        let result = proc.process(&history);

        // Last 3 items should have cache_control set
        assert!(result[4].cache_control.is_some());
        assert!(result[5].cache_control.is_some());
        assert!(result[6].cache_control.is_some());
        // First items should not
        assert!(result[0].cache_control.is_none());
        assert!(result[1].cache_control.is_none());

        // Verify kind is "ephemeral"
        assert_eq!(result[6].cache_control.as_ref().unwrap().kind, "ephemeral");
    }

    #[test]
    fn remove_regex_removes_matching_patterns() {
        let history = vec![
            make_user("Hello SECRET world"),
            make_user("No match here"),
        ];
        let re = Regex::new("SECRET").unwrap();
        let proc = RemoveRegexHistoryProcessor::new(vec![re]);
        let result = proc.process(&history);
        assert_eq!(text_of(&result[0]), "Hello  world");
        assert_eq!(text_of(&result[1]), "No match here");
    }

    #[test]
    fn remove_regex_empty_patterns_unchanged() {
        let history = vec![make_user("keep me")];
        let proc = RemoveRegexHistoryProcessor::default();
        let result = proc.process(&history);
        assert_eq!(text_of(&result[0]), "keep me");
    }

    #[test]
    fn tag_tool_call_observations_adds_tags() {
        let mut item = make_assistant("tool_name arg1 arg2");
        item.action = Some("tool_name arg1 arg2".to_string());

        let history = vec![item, make_user("obs")];
        let proc = TagToolCallObservationsProcessor {
            tags: vec!["tagged".to_string()],
            function_names: vec!["tool_name".to_string()],
        };
        let result = proc.process(&history);

        let tags = result[0].tags.as_ref().expect("should have tags");
        assert!(tags.contains(&"tagged".to_string()));
        // Second item (user obs) should not have tags
        assert!(result[1].tags.is_none());
    }

    #[test]
    fn tag_tool_call_observations_exact_name_match() {
        let mut item = make_assistant("other_tool");
        item.action = Some("other_tool".to_string());

        let history = vec![item];
        let proc = TagToolCallObservationsProcessor {
            tags: vec!["t".to_string()],
            function_names: vec!["other_tool".to_string()],
        };
        let result = proc.process(&history);
        let tags = result[0].tags.as_ref().expect("should have tags");
        assert!(tags.contains(&"t".to_string()));
    }

    #[test]
    fn tag_tool_call_observations_no_match() {
        let mut item = make_assistant("different_tool args");
        item.action = Some("different_tool args".to_string());

        let history = vec![item];
        let proc = TagToolCallObservationsProcessor {
            tags: vec!["t".to_string()],
            function_names: vec!["my_tool".to_string()],
        };
        let result = proc.process(&history);
        assert!(result[0].tags.is_none());
    }

    #[test]
    fn chain_processors_applies_in_sequence() {
        let history = vec![make_user("REMOVE this content")];
        let re = Regex::new("REMOVE ").unwrap();
        let p1: BoxedProcessor = Box::new(RemoveRegexHistoryProcessor::new(vec![re]));
        let p2: BoxedProcessor = Box::new(DefaultHistoryProcessor);
        let chained = chain_processors(vec![p1, p2]);
        let result = chained.process(&history);
        assert_eq!(text_of(&result[0]), "this content");
    }

    #[test]
    fn factory_creates_default() {
        let cfg = serde_json::json!({ "type": "default" });
        let proc = create_processor(&cfg).unwrap();
        let history = vec![make_user("test")];
        let result = proc.process(&history);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn factory_creates_last_n_observations() {
        let cfg = serde_json::json!({ "type": "last_n_observations", "n": 3 });
        let proc = create_processor(&cfg).unwrap();
        let history = vec![make_system("s"), make_user("o1"), make_user("o2")];
        let result = proc.process(&history);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn factory_unknown_type_errors() {
        let cfg = serde_json::json!({ "type": "nonexistent_type" });
        let result = create_processor(&cfg);
        assert!(result.is_err());
        if let Err(ForgeError::Config(msg)) = result {
            assert!(msg.contains("nonexistent_type"));
        } else {
            panic!("expected ForgeError::Config");
        }
    }
}
