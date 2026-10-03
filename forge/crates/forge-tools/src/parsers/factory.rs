use forge_types::ForgeError;

use super::action_only::ActionOnlyParser;
use super::function_calling::FunctionCallingParser;
use super::thought_action::ThoughtActionParser;
use super::types::Parser;
use super::xml::XmlParser;

/// Returns a boxed [`Parser`] for the given parser name.
///
/// Supported names:
/// | Name                    | Parser               |
/// |-------------------------|----------------------|
/// | `"thought_action"`      | [`ThoughtActionParser`] |
/// | `"action_only"`         | [`ActionOnlyParser`]  |
/// | `"action_only_lm_sys"`  | [`ActionOnlyParser`]  |
/// | `"xml"` / `"xml_thought_action"` | [`XmlParser`] |
/// | `"function_calling"`    | [`FunctionCallingParser`] |
///
/// Unknown names return `ForgeError::Config`.
pub fn get_parser(parser_name: &str) -> Result<Box<dyn Parser>, ForgeError> {
    match parser_name {
        "thought_action" => Ok(Box::new(ThoughtActionParser::default())),
        "action_only" | "action_only_lm_sys" => Ok(Box::new(ActionOnlyParser)),
        "xml" | "xml_thought_action" => Ok(Box::new(XmlParser::default())),
        "function_calling" => Ok(Box::new(FunctionCallingParser)),
        _ => Err(ForgeError::Config(format!("unknown parser: {}", parser_name))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thought_action_parser_is_known() {
        assert!(get_parser("thought_action").is_ok());
    }

    #[test]
    fn action_only_parser_is_known() {
        assert!(get_parser("action_only").is_ok());
    }

    #[test]
    fn action_only_lm_sys_parser_is_known() {
        assert!(get_parser("action_only_lm_sys").is_ok());
    }

    #[test]
    fn xml_parser_is_known() {
        assert!(get_parser("xml").is_ok());
    }

    #[test]
    fn xml_thought_action_parser_is_known() {
        assert!(get_parser("xml_thought_action").is_ok());
    }

    #[test]
    fn function_calling_parser_is_known() {
        assert!(get_parser("function_calling").is_ok());
    }

    #[test]
    fn unknown_parser_returns_config_error() {
        let result = get_parser("nonexistent_parser");
        assert!(result.is_err());
        if let Err(ForgeError::Config(msg)) = result {
            assert!(msg.contains("nonexistent_parser"));
        } else {
            panic!("expected ForgeError::Config");
        }
    }

    #[test]
    fn thought_action_parser_round_trip() {
        let parser = get_parser("thought_action").unwrap();
        let input = "My thought\n```\nls -l\n```";
        let out = parser.parse(input).unwrap();
        // thought is now trimmed
        assert_eq!(out.thought, "My thought");
        assert_eq!(out.action, "ls -l\n");
    }

    #[test]
    fn action_only_parser_round_trip() {
        let parser = get_parser("action_only").unwrap();
        let input = "ls -l";
        let out = parser.parse(input).unwrap();
        assert_eq!(out.thought, "");
        assert_eq!(out.action, "ls -l");
    }

    #[test]
    fn xml_parser_round_trip() {
        let parser = get_parser("xml").unwrap();
        let input = "My thought\n<command>\nls -l\n</command>";
        let out = parser.parse(input).unwrap();
        assert_eq!(out.thought, "My thought");
        assert_eq!(out.action, "ls -l");
    }
}
