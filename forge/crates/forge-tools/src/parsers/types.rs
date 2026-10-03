use forge_types::{ForgeError, ModelOutput};

/// The output of parsing a model response: separated thought and action.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedOutput {
    pub thought: String,
    pub action: String,
}

/// A parser that converts a model response into a [`ParsedOutput`].
///
/// The primary method is `parse_model_output`, which accepts the full
/// [`ModelOutput`] struct (giving access to `tool_calls` for
/// `FunctionCallingParser`, `thinking_blocks`, etc.).
///
/// A convenience `parse` method is also provided for callers that only have
/// a raw text string; it constructs a minimal `ModelOutput` and delegates to
/// `parse_model_output`.
pub trait Parser: Send + Sync {
    /// Parse from a full model output (preferred — gives access to tool_calls).
    fn parse_model_output(&self, output: &ModelOutput) -> Result<ParsedOutput, ForgeError>;

    /// Convenience: parse from a text string only (used by non-function-calling parsers).
    fn parse(&self, text: &str) -> Result<ParsedOutput, ForgeError> {
        self.parse_model_output(&ModelOutput {
            message: text.to_string(),
            tool_calls: None,
            thinking_blocks: None,
            input_tokens: None,
            output_tokens: None,
            cost: None,
        })
    }
}
