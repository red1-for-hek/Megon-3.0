pub const SUBMISSION: &str = "<<SWE_AGENT_SUBMISSION>>";
pub const RETRY_WITH_OUTPUT: &str = "###SWE-AGENT-RETRY-WITH-OUTPUT###";
pub const RETRY_WITHOUT_OUTPUT: &str = "###SWE-AGENT-RETRY-WITHOUT-OUTPUT###";
pub const EXIT_FORFEIT: &str = "###SWE-AGENT-EXIT-FORFEIT###";

pub fn contains_submission(output: &str) -> bool {
    output.contains(SUBMISSION)
}
pub fn contains_retry_with_output(output: &str) -> bool {
    output.contains(RETRY_WITH_OUTPUT)
}
pub fn contains_retry_without_output(output: &str) -> bool {
    output.contains(RETRY_WITHOUT_OUTPUT)
}
pub fn contains_forfeit(output: &str) -> bool {
    output.contains(EXIT_FORFEIT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_submission_token() {
        assert_eq!(SUBMISSION, "<<SWE_AGENT_SUBMISSION>>");
    }

    #[test]
    fn exact_retry_tokens() {
        assert_eq!(RETRY_WITH_OUTPUT, "###SWE-AGENT-RETRY-WITH-OUTPUT###");
        assert_eq!(RETRY_WITHOUT_OUTPUT, "###SWE-AGENT-RETRY-WITHOUT-OUTPUT###");
        assert_eq!(EXIT_FORFEIT, "###SWE-AGENT-EXIT-FORFEIT###");
    }

    #[test]
    fn detection_functions() {
        assert!(contains_submission("output\n<<SWE_AGENT_SUBMISSION>>\nmore"));
        assert!(!contains_submission("no token"));
        assert!(contains_retry_with_output("###SWE-AGENT-RETRY-WITH-OUTPUT###"));
        assert!(contains_forfeit("###SWE-AGENT-EXIT-FORFEIT###"));
        assert!(!contains_forfeit("normal output"));
    }
}
