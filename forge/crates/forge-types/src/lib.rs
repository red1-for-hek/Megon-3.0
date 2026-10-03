// forge-types: shared data types, errors, and contracts for the Forge SWE-agent

pub mod error;
pub mod history;
pub mod model_output;
pub mod public_workflow;
pub mod special_tokens;
pub mod step;
pub mod trajectory;

pub use error::{ExitStatus, ForgeError, FormatErrorCode};
pub use history::{
    CacheControl, ContentBlock, History, HistoryItem, ImageUrl,
    MessageContent, MessageType, Role, ThinkingBlock, ToolCall, ToolFunction,
};
pub use model_output::ModelOutput;
pub use special_tokens::{
    contains_forfeit, contains_retry_with_output,
    contains_retry_without_output, contains_submission,
    EXIT_FORFEIT, RETRY_WITH_OUTPUT, RETRY_WITHOUT_OUTPUT, SUBMISSION,
};
pub use step::StepOutput;
pub use trajectory::{AgentInfo, PredictionEntry, TrajFile, Trajectory, TrajectoryStep};
