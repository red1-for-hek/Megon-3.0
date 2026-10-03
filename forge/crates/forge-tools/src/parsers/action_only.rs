use forge_types::{ForgeError, ModelOutput};

use super::types::{ParsedOutput, Parser};

/// Returns the entire model response as the action; thought is always empty.
///
/// Mirrors `ActionOnlyParser.parse` in `src/tools/parsing.ts`:
/// ```ts
/// parse(modelResponse) { return ['', modelResponse.message.trim()]; }
/// ```
pub struct ActionOnlyParser;

impl Parser for ActionOnlyParser {
    fn parse_model_output(&self, output: &ModelOutput) -> Result<ParsedOutput, ForgeError> {
        Ok(ParsedOutput {
            thought: String::new(),
            // Trim the response (Fix 4)
            action: output.message.trim().to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_output(msg: &str) -> ModelOutput {
        ModelOutput {
            message: msg.to_string(),
            tool_calls: None,
            thinking_blocks: None,
            input_tokens: None,
            output_tokens: None,
            cost: None,
        }
    }

    #[test]
    fn whole_message_becomes_action() {
        let parser = ActionOnlyParser;
        let out = parser.parse("ls -l").unwrap();
        assert_eq!(out.thought, "");
        assert_eq!(out.action, "ls -l");
    }

    #[test]
    fn empty_message() {
        let parser = ActionOnlyParser;
        let out = parser.parse("").unwrap();
        assert_eq!(out.thought, "");
        assert_eq!(out.action, "");
    }

    #[test]
    fn trims_whitespace() {
        let parser = ActionOnlyParser;
        let out = parser.parse("  ls -l  \n").unwrap();
        assert_eq!(out.thought, "");
        assert_eq!(out.action, "ls -l");
    }

    #[test]
    fn parse_model_output_trims() {
        let parser = ActionOnlyParser;
        let out = parser.parse_model_output(&make_output("  echo hi  \n")).unwrap();
        assert_eq!(out.thought, "");
        assert_eq!(out.action, "echo hi");
    }
}
