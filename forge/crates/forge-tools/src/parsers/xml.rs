use std::sync::OnceLock;

use forge_types::{ForgeError, ModelOutput};

use super::types::{ParsedOutput, Parser};

static COMMAND_RE: OnceLock<regex::Regex> = OnceLock::new();
fn command_re() -> &'static regex::Regex {
    COMMAND_RE.get_or_init(|| regex::Regex::new(r"(?si)<command>(.*?)</command>").unwrap())
}

static ACTION_RE: OnceLock<regex::Regex> = OnceLock::new();
fn action_re() -> &'static regex::Regex {
    ACTION_RE.get_or_init(|| regex::Regex::new(r"(?si)<action>(.*?)</action>").unwrap())
}

static THOUGHT_RE: OnceLock<regex::Regex> = OnceLock::new();
fn thought_re() -> &'static regex::Regex {
    THOUGHT_RE.get_or_init(|| regex::Regex::new(r"(?si)<thought>(.*?)</thought>").unwrap())
}

/// Parses XML-style `<command>` tags from a model response.
///
/// Mirrors `XMLThoughtActionParser.parse` in `src/tools/parsing.ts`:
/// - Finds all `<command>…</command>` blocks.
/// - If none found and non-strict → `[message, ""]`.
/// - If none found and strict → `FormatError`.
/// - If multiple found and strict → `FormatError`.
/// - Otherwise: content before the first `<command>` tag is the thought;
///   trimmed content of the first tag is the action.
///
/// Also supports `<action>…</action>` and `<thought>…</thought>` tags as
/// defined in `src/agent/tools/parsing.ts` (`XMLThoughtActionParser` there).
pub struct XmlParser {
    pub strict: bool,
}

impl Default for XmlParser {
    fn default() -> Self {
        Self { strict: false }
    }
}

impl Parser for XmlParser {
    fn parse_model_output(&self, output: &ModelOutput) -> Result<ParsedOutput, ForgeError> {
        parse_xml(&output.message, self.strict)
    }
}

#[cfg(test)]
fn parse_xml_strict(model_response: &str) -> Result<ParsedOutput, ForgeError> {
    parse_xml(model_response, true)
}

fn parse_xml(message: &str, strict: bool) -> Result<ParsedOutput, ForgeError> {
    // Primary: look for <command>...</command>
    let command_matches: Vec<_> = command_re().find_iter(message).collect();

    if command_matches.is_empty() {
        // Fallback: look for <action>...</action> (agent/tools/parsing.ts variant)
        if let Some(action_cap) = action_re().captures(message) {
            let action = action_cap.get(1).map(|m| m.as_str().trim().to_string()).unwrap_or_default();

            // Optional <thought>...</thought>
            let thought = thought_re()
                .captures(message)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_else(|| {
                    // Everything before the <action> tag
                    let action_start = action_re().find(message).map(|m| m.start()).unwrap_or(0);
                    message[..action_start].trim().to_string()
                });

            return Ok(ParsedOutput { thought, action });
        }

        if strict {
            return Err(ForgeError::FormatError(
                "No <command> tag found in response".to_string(),
            ));
        }

        return Ok(ParsedOutput {
            thought: message.to_string(),
            action: String::new(),
        });
    }

    if command_matches.len() > 1 && strict {
        return Err(ForgeError::FormatError(
            "Multiple <command> tags found in response".to_string(),
        ));
    }

    let first_match = &command_matches[0];
    let thought = message[..first_match.start()].trim().to_string();

    let caps = command_re()
        .captures(message)
        .ok_or_else(|| ForgeError::FormatError("No <command> tag found in response".to_string()))?;
    let action = caps.get(1).map(|m| m.as_str().trim().to_string()).unwrap_or_default();

    Ok(ParsedOutput { thought, action })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_command_tag() {
        let parser = XmlParser::default();
        let input = "Let's look at the files in the current directory.\n<command>\nls -l\n</command>";
        let out = parser.parse(input).unwrap();
        assert_eq!(out.thought, "Let's look at the files in the current directory.");
        assert_eq!(out.action, "ls -l");
    }

    #[test]
    fn no_command_tag_non_strict() {
        let parser = XmlParser::default();
        let input = "No command tags";
        let out = parser.parse(input).unwrap();
        assert_eq!(out.thought, "No command tags");
        assert_eq!(out.action, "");
    }

    #[test]
    fn no_command_tag_strict_returns_error() {
        let result = parse_xml_strict("No command tags");
        assert!(matches!(result, Err(ForgeError::FormatError(_))));
    }

    #[test]
    fn parses_action_tag() {
        let parser = XmlParser::default();
        let input = "<thought>My thought</thought><action>ls -l</action>";
        let out = parser.parse(input).unwrap();
        assert_eq!(out.thought, "My thought");
        assert_eq!(out.action, "ls -l");
    }

    #[test]
    fn multiple_command_tags_strict_returns_error() {
        let result = parse_xml_strict("<command>ls</command><command>cd</command>");
        assert!(matches!(result, Err(ForgeError::FormatError(_))));
    }

    /// Fix 6: action tag present but no thought tag — pre-action text becomes thought.
    #[test]
    fn xml_action_without_thought_tag() {
        let parser = XmlParser::default();
        let input = "Some pre-action text.\n<action>\necho hello\n</action>";
        let out = parser.parse(input).unwrap();
        assert_eq!(out.action.trim(), "echo hello");
        // thought comes from pre-action text
        assert_eq!(out.thought, "Some pre-action text.");
    }

    #[test]
    fn strict_field_controls_behavior() {
        let strict_parser = XmlParser { strict: true };
        let result = strict_parser.parse("No tags here");
        assert!(matches!(result, Err(ForgeError::FormatError(_))));

        let lenient_parser = XmlParser { strict: false };
        let result = lenient_parser.parse("No tags here").unwrap();
        assert_eq!(result.action, "");
    }

    #[test]
    fn parse_model_output_uses_message_field() {
        let parser = XmlParser::default();
        let output = ModelOutput {
            message: "My thought.\n<command>\nls -l\n</command>".to_string(),
            tool_calls: None,
            thinking_blocks: None,
            input_tokens: None,
            output_tokens: None,
            cost: None,
        };
        let result = parser.parse_model_output(&output).unwrap();
        assert_eq!(result.thought, "My thought.");
        assert_eq!(result.action, "ls -l");
    }
}
