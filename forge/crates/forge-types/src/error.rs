use thiserror::Error;

#[derive(Debug, Clone, PartialEq)]
pub enum FormatErrorCode {
    Missing,
    Multiple,
    IncorrectArgs,
    InvalidJson,
    InvalidCommand,
    MissingArg,
    UnexpectedArg,
}

#[derive(Debug, Error)]
pub enum ForgeError {
    #[error("format error: {0}")]
    FormatError(String),

    #[error("function calling format error ({code:?}): {message}")]
    FunctionCallingFormatError { code: FormatErrorCode, message: String },

    #[error("blocked action: {0}")]
    BlockedActionError(String),

    #[error("bash syntax error: {0}")]
    BashSyntaxError(String),

    #[error("context window exceeded")]
    ContextWindowExceeded,

    #[error("instance cost limit exceeded")]
    InstanceCostLimitExceeded,

    #[error("total cost limit exceeded")]
    TotalCostLimitExceeded,

    #[error("instance call limit exceeded")]
    InstanceCallLimitExceeded,

    #[error("command timed out")]
    CommandTimeout,

    #[error("total execution time exceeded")]
    TotalExecutionTimeExceeded,

    #[error("exit forfeit")]
    ExitForfeit,

    #[error("retry with output: {0}")]
    RetryWithOutput(String),

    #[error("retry without output")]
    RetryWithoutOutput,

    #[error("content policy violation: {0}")]
    ContentPolicyViolation(String),

    #[error("docker error: {0}")]
    Docker(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("http error: {0}")]
    Http(String),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("environment error: {0}")]
    Environment(String),

    #[error("model error: {0}")]
    Model(String),

    #[error("config error: {0}")]
    Config(String),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExitStatus {
    Submitted,
    EarlyExit,
    Forfeited,
    Blocked,
    TotalCostLimitReached,
    InstanceCostLimitReached,
    StepLimitReached,
    Error,
}

impl ExitStatus {
    /// Returns the canonical string representation used in trajectory files.
    /// Kept separate from the serde representation to provide a `&'static str`
    /// guarantee without JSON quoting overhead.
    pub fn as_str(&self) -> &'static str {
        match self {
            ExitStatus::Submitted => "submitted",
            ExitStatus::EarlyExit => "early_exit",
            ExitStatus::Forfeited => "forfeited",
            ExitStatus::Blocked => "blocked",
            ExitStatus::TotalCostLimitReached => "total_cost_limit_reached",
            ExitStatus::InstanceCostLimitReached => "instance_cost_limit_reached",
            ExitStatus::StepLimitReached => "step_limit_reached",
            ExitStatus::Error => "error",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_error_display() {
        let e = ForgeError::FormatError("bad parse".into());
        assert!(e.to_string().contains("bad parse"));
    }

    #[test]
    fn function_calling_error_code() {
        let e = ForgeError::FunctionCallingFormatError {
            code: FormatErrorCode::Missing,
            message: "no tool call".into(),
        };
        assert!(matches!(
            e,
            ForgeError::FunctionCallingFormatError { code: FormatErrorCode::Missing, .. }
        ));
    }

    #[test]
    fn exit_status_strings() {
        assert_eq!(ExitStatus::Submitted.as_str(), "submitted");
        assert_eq!(ExitStatus::EarlyExit.as_str(), "early_exit");
        assert_eq!(ExitStatus::Forfeited.as_str(), "forfeited");
        assert_eq!(ExitStatus::Blocked.as_str(), "blocked");
        assert_eq!(ExitStatus::TotalCostLimitReached.as_str(), "total_cost_limit_reached");
        assert_eq!(ExitStatus::InstanceCostLimitReached.as_str(), "instance_cost_limit_reached");
        assert_eq!(ExitStatus::StepLimitReached.as_str(), "step_limit_reached");
        assert_eq!(ExitStatus::Error.as_str(), "error");
    }
}
