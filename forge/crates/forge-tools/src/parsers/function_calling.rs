use forge_types::{ForgeError, FormatErrorCode, ModelOutput, ToolFunction};
use indexmap::IndexMap;
use serde_json::Value;

use super::types::{ParsedOutput, Parser};

/// Parses function/tool-call responses from a model.
///
/// Mirrors `FunctionCallingParser.parse` in `src/tools/parsing.ts`:
/// - Requires exactly one tool call in `ModelOutput.tool_calls`.
/// - Missing tool calls → `FunctionCallingFormatError { code: Missing }`.
/// - Multiple tool calls → `FunctionCallingFormatError { code: Multiple }`.
/// - Unknown command name → `FunctionCallingFormatError { code: InvalidCommand }`.
/// - Invalid JSON arguments → `FunctionCallingFormatError { code: InvalidJson }`.
/// - Missing required argument → `FunctionCallingFormatError { code: MissingArg }`.
/// - On success returns `[message, action_string]`.
///
/// The `Parser` trait implementation uses `parse_model_output`.  The
/// string-only `parse` convenience method (from the trait default) treats the
/// entire string as the action with an empty thought — matching the TS
/// fallback path when no tool calls are available.
pub struct FunctionCallingParser;

impl Parser for FunctionCallingParser {
    fn parse_model_output(&self, output: &ModelOutput) -> Result<ParsedOutput, ForgeError> {
        self.parse_model_output_with_commands(output, &[])
    }
}

impl FunctionCallingParser {
    /// Parse a full [`ModelOutput`] that may contain tool calls, validating
    /// against the provided list of allowed command names.
    ///
    /// `commands` is a slice of allowed command names.  When non-empty, the
    /// function name in the tool call must match one of them; otherwise
    /// `FunctionCallingFormatError { InvalidCommand }` is returned.
    pub fn parse_model_output_with_commands(
        &self,
        model_output: &ModelOutput,
        commands: &[&str],
    ) -> Result<ParsedOutput, ForgeError> {
        let tool_calls = model_output.tool_calls.as_deref().unwrap_or(&[]);

        if tool_calls.is_empty() {
            return Err(ForgeError::FunctionCallingFormatError {
                code: FormatErrorCode::Missing,
                message: "No tool calls found".to_string(),
            });
        }

        if tool_calls.len() > 1 {
            return Err(ForgeError::FunctionCallingFormatError {
                code: FormatErrorCode::Multiple,
                message: "Multiple tool calls found".to_string(),
            });
        }

        let tool_call = &tool_calls[0];
        let action = build_action_string(&tool_call.function, commands)?;
        let thought = model_output.message.clone();

        Ok(ParsedOutput { thought, action })
    }
}

fn build_action_string(func: &ToolFunction, commands: &[&str]) -> Result<String, ForgeError> {
    let function_name = &func.name;

    // Validate against allowed commands when provided
    if !commands.is_empty() && !commands.contains(&function_name.as_str()) {
        return Err(ForgeError::FunctionCallingFormatError {
            code: FormatErrorCode::InvalidCommand,
            message: format!("Unknown command: {}", function_name),
        });
    }

    // Parse arguments from the JSON value stored in ToolFunction.arguments
    // Use IndexMap to preserve insertion order (matches TS argument order behavior)
    let args_value = &func.arguments;
    let parsed_args: IndexMap<String, Value> = match args_value {
        Value::Object(map) => map.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        Value::String(s) => {
            // Arguments may have been serialised as a JSON string
            match serde_json::from_str::<Value>(s) {
                Ok(Value::Object(map)) => map.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
                Ok(_) => {
                    return Err(ForgeError::FunctionCallingFormatError {
                        code: FormatErrorCode::InvalidJson,
                        message: "Arguments JSON is not an object".to_string(),
                    })
                }
                Err(_) => {
                    return Err(ForgeError::FunctionCallingFormatError {
                        code: FormatErrorCode::InvalidJson,
                        message: "Invalid JSON in arguments".to_string(),
                    })
                }
            }
        }
        Value::Null => IndexMap::new(),
        _ => {
            return Err(ForgeError::FunctionCallingFormatError {
                code: FormatErrorCode::InvalidJson,
                message: "Arguments must be a JSON object".to_string(),
            })
        }
    };

    // Build the action string: "function_name arg1 arg2 ..."
    // Iterate in IndexMap order (preserves declaration order from JSON)
    let mut action = function_name.clone();
    for (key, value) in &parsed_args {
        // Check for missing required args: null values with non-null key indicate MissingArg
        if matches!(value, Value::Null) {
            return Err(ForgeError::FunctionCallingFormatError {
                code: FormatErrorCode::MissingArg,
                message: format!("Required argument '{}' is missing", key),
            });
        }
        let formatted = format_arg_value(value);
        if !formatted.is_empty() {
            action.push(' ');
            action.push_str(&formatted);
        }
    }

    Ok(action)
}

fn format_arg_value(value: &Value) -> String {
    match value {
        Value::String(s) => {
            // Quote if it contains whitespace or shell-special characters
            if s.chars().any(|c| c.is_whitespace() || matches!(c, '"' | '\'' | '`' | '$')) {
                format!("\"{}\"", s.replace('"', "\\\""))
            } else {
                s.clone()
            }
        }
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_types::{ToolCall, ToolFunction};

    fn make_model_output(message: &str, tool_calls: Option<Vec<ToolCall>>) -> ModelOutput {
        ModelOutput {
            message: message.to_string(),
            tool_calls,
            thinking_blocks: None,
            input_tokens: None,
            output_tokens: None,
            cost: None,
        }
    }

    fn make_tool_call(name: &str, args: Value) -> ToolCall {
        ToolCall {
            id: None,
            tool_type: None,
            function: ToolFunction {
                name: name.to_string(),
                arguments: args,
            },
        }
    }

    #[test]
    fn parses_simple_function_call() {
        let parser = FunctionCallingParser;
        let output = make_model_output(
            "Let's list the files",
            Some(vec![make_tool_call("ls", Value::Object(Default::default()))]),
        );
        let result = parser.parse_model_output_with_commands(&output, &["ls"]).unwrap();
        assert_eq!(result.thought, "Let's list the files");
        assert_eq!(result.action, "ls");
    }

    #[test]
    fn missing_tool_calls_returns_error() {
        let parser = FunctionCallingParser;
        let output = make_model_output("No tool calls", None);
        let err = parser.parse_model_output_with_commands(&output, &["ls"]).unwrap_err();
        assert!(matches!(
            err,
            ForgeError::FunctionCallingFormatError { code: FormatErrorCode::Missing, .. }
        ));
    }

    #[test]
    fn multiple_tool_calls_returns_error() {
        let parser = FunctionCallingParser;
        let output = make_model_output(
            "Multiple calls",
            Some(vec![
                make_tool_call("ls", Value::Object(Default::default())),
                make_tool_call("cd", Value::Object(Default::default())),
            ]),
        );
        let err = parser.parse_model_output_with_commands(&output, &["ls", "cd"]).unwrap_err();
        assert!(matches!(
            err,
            ForgeError::FunctionCallingFormatError { code: FormatErrorCode::Multiple, .. }
        ));
    }

    #[test]
    fn unknown_command_returns_error() {
        let parser = FunctionCallingParser;
        let output = make_model_output(
            "Invalid command",
            Some(vec![make_tool_call("invalid", Value::Object(Default::default()))]),
        );
        let err = parser.parse_model_output_with_commands(&output, &["ls"]).unwrap_err();
        assert!(matches!(
            err,
            ForgeError::FunctionCallingFormatError { code: FormatErrorCode::InvalidCommand, .. }
        ));
    }

    #[test]
    fn invalid_json_string_arguments_returns_error() {
        let parser = FunctionCallingParser;
        let output = make_model_output(
            "Invalid JSON",
            Some(vec![make_tool_call("ls", Value::String("invalid json".to_string()))]),
        );
        let err = parser.parse_model_output_with_commands(&output, &["ls"]).unwrap_err();
        assert!(matches!(
            err,
            ForgeError::FunctionCallingFormatError { code: FormatErrorCode::InvalidJson, .. }
        ));
    }

    #[test]
    fn string_based_parser_returns_error_no_tool_calls() {
        // The string-only parse delegates to parse_model_output which sees no tool_calls
        let parser = FunctionCallingParser;
        let err = parser.parse("some response").unwrap_err();
        assert!(matches!(
            err,
            ForgeError::FunctionCallingFormatError { code: FormatErrorCode::Missing, .. }
        ));
    }

    #[test]
    fn missing_required_arg_returns_missing_arg_error() {
        let parser = FunctionCallingParser;
        let mut args = serde_json::Map::new();
        args.insert("required_param".to_string(), Value::Null);
        let output = make_model_output(
            "Command with missing arg",
            Some(vec![make_tool_call("ls", Value::Object(args))]),
        );
        let err = parser.parse_model_output_with_commands(&output, &["ls"]).unwrap_err();
        assert!(matches!(
            err,
            ForgeError::FunctionCallingFormatError { code: FormatErrorCode::MissingArg, .. }
        ));
    }

    #[test]
    fn args_are_collected_into_index_map() {
        let parser = FunctionCallingParser;
        // Use a JSON object with args — all values should appear in the action string
        let args_json = r#"{"aaa_first": "value1", "zzz_last": "value2"}"#;
        let output = make_model_output(
            "Ordered args test",
            Some(vec![make_tool_call("ls", Value::String(args_json.to_string()))]),
        );
        let result = parser.parse_model_output_with_commands(&output, &["ls"]).unwrap();
        // Both values must be present in the action
        assert!(result.action.contains("value1"), "first arg value missing: {}", result.action);
        assert!(result.action.contains("value2"), "second arg value missing: {}", result.action);
        // aaa_first < zzz_last alphabetically, so with any ordered map aaa_first is first
        let first_pos = result.action.find("value1").unwrap();
        let last_pos = result.action.find("value2").unwrap();
        assert!(first_pos < last_pos, "aaa_first should appear before zzz_last: {}", result.action);
    }
}
