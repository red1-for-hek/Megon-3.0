//! Parser layer: convert model response strings into structured
//! `(thought, action)` pairs, matching the TypeScript SWEagent parser system.

pub mod action_only;
pub mod factory;
pub mod function_calling;
pub mod thought_action;
pub mod types;
pub mod xml;

// Flatten the most-used public items to the crate surface.
pub use action_only::ActionOnlyParser;
pub use factory::get_parser;
pub use function_calling::FunctionCallingParser;
pub use thought_action::ThoughtActionParser;
pub use types::{ParsedOutput, Parser};
pub use xml::XmlParser;
