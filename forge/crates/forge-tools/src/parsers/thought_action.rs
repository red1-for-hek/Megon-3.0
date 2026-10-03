use std::sync::OnceLock;

use forge_types::{ForgeError, ModelOutput};

use super::types::{ParsedOutput, Parser};

static CODE_BLOCK_RE: OnceLock<regex::Regex> = OnceLock::new();
fn code_block_re() -> &'static regex::Regex {
    CODE_BLOCK_RE.get_or_init(|| regex::Regex::new(r"(?s)```[^\n]*\n(.*?)```").unwrap())
}

/// Parses a model response that contains a discussion followed by a code-block
/// command, mirroring `ThoughtActionParser` in `src/tools/parsing.ts`.
///
/// Behavior (in priority order):
/// 1. If a fenced code block (``` ... ```) is present, everything before the
///    opening fence is the thought (trimmed); the block contents are the action.
/// 2. If no code block is found and `strict` is false the entire message is
///    returned as the thought with an empty action.
/// 3. If no code block is found in strict mode → `FormatError`.
///
/// This matches the TypeScript `ThoughtActionParser.parse` in
/// `src/tools/parsing.ts` which throws when strict and no block found,
/// otherwise returns `[message, ""]`.
pub struct ThoughtActionParser {
    pub strict: bool,
}

impl Default for ThoughtActionParser {
    fn default() -> Self {
        Self { strict: false }
    }
}

impl Parser for ThoughtActionParser {
    fn parse_model_output(&self, output: &ModelOutput) -> Result<ParsedOutput, ForgeError> {
        parse_thought_action(&output.message, self.strict)
    }
}

/// Strict variant used internally (returns error instead of empty action).
#[cfg(test)]
fn parse_thought_action_strict(model_response: &str) -> Result<ParsedOutput, ForgeError> {
    parse_thought_action(model_response, true)
}

fn parse_thought_action(message: &str, strict: bool) -> Result<ParsedOutput, ForgeError> {
    let re = code_block_re();
    if let Some(caps) = re.captures(message) {
        let code_block_start = caps.get(0).unwrap().start();
        let thought = message[..code_block_start].trim().to_string();
        let action = caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
        return Ok(ParsedOutput { thought, action });
    }

    if strict {
        return Err(ForgeError::FormatError(
            "No code block found in response".to_string(),
        ));
    }

    // Non-strict fallback: entire message is thought, action empty
    Ok(ParsedOutput {
        thought: message.to_string(),
        action: String::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_code_block() {
        let parser = ThoughtActionParser::default();
        let input = "Let's look at the files in the current directory.\n```\nls -l\n```";
        let out = parser.parse(input).unwrap();
        // thought is now trimmed
        assert_eq!(out.thought, "Let's look at the files in the current directory.");
        assert_eq!(out.action, "ls -l\n");
    }

    #[test]
    fn parses_bash_fenced_block() {
        let parser = ThoughtActionParser::default();
        let input = "Some thought.\n```bash\necho hello\n```";
        let out = parser.parse(input).unwrap();
        assert_eq!(out.thought, "Some thought.");
        assert_eq!(out.action, "echo hello\n");
    }

    #[test]
    fn no_code_block_non_strict_returns_whole_message_as_thought() {
        let parser = ThoughtActionParser::default();
        let input = "No code block here";
        let out = parser.parse(input).unwrap();
        assert_eq!(out.thought, "No code block here");
        assert_eq!(out.action, "");
    }

    #[test]
    fn no_code_block_strict_returns_error() {
        let result = parse_thought_action_strict("No code block");
        assert!(matches!(result, Err(ForgeError::FormatError(_))));
    }

    #[test]
    fn strict_field_controls_behavior() {
        let strict_parser = ThoughtActionParser { strict: true };
        let result = strict_parser.parse("No code block");
        assert!(matches!(result, Err(ForgeError::FormatError(_))));

        let lenient_parser = ThoughtActionParser { strict: false };
        let result = lenient_parser.parse("No code block").unwrap();
        assert_eq!(result.thought, "No code block");
        assert_eq!(result.action, "");
    }

    #[test]
    fn parse_model_output_uses_message_field() {
        let parser = ThoughtActionParser::default();
        let output = ModelOutput {
            message: "My thought.\n```\necho hi\n```".to_string(),
            tool_calls: None,
            thinking_blocks: None,
            input_tokens: None,
            output_tokens: None,
            cost: None,
        };
        let result = parser.parse_model_output(&output).unwrap();
        assert_eq!(result.thought, "My thought.");
        assert_eq!(result.action, "echo hi\n");
    }
}
