use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

use forge_env::environment::SweEnvironment;
use forge_model::traits::SharedModel;
use forge_tools::parsers::get_parser;
use forge_types::error::{ExitStatus, ForgeError};
use forge_types::history::{HistoryItem, MessageContent, MessageType, Role};
use forge_types::trajectory::{AgentInfo, TrajFile, TrajectoryStep};
use forge_types::{
    History, Trajectory,
    contains_forfeit, contains_retry_with_output, contains_retry_without_output,
    contains_submission,
};

use crate::history_processors::BoxedProcessor;
use crate::problem_statement::{AnyProblemStatement, ProblemStatement};

// ---------------------------------------------------------------------------
// Pure helpers (testable without Docker)
// ---------------------------------------------------------------------------

/// Simple `{key}` template substitution.
pub fn render_template(template: &str, vars: &HashMap<&str, &str>) -> String {
    let mut result = template.to_string();
    for (key, value) in vars {
        let placeholder = format!("{{{}}}", key);
        result = result.replace(&placeholder, value);
    }
    result
}

/// Truncate an observation that is too long, inserting an elision note.
///
/// Uses character count (not byte count) so multi-byte UTF-8 sequences are
/// never split mid-codepoint.
pub fn truncate_observation(observation: String, max_length: usize) -> String {
    let char_count = observation.chars().count();
    if char_count <= max_length {
        return observation;
    }
    let elided = char_count - max_length;
    // Find the byte boundary for the max_length-th character.
    let byte_end = observation
        .char_indices()
        .nth(max_length)
        .map(|(i, _)| i)
        .unwrap_or(observation.len());
    format!(
        "{}<response clipped><NOTE>Observations should not exceed {} characters. {} characters were elided.</NOTE>",
        &observation[..byte_end],
        max_length,
        elided
    )
}

/// Check whether an action is blocked by the blocklist.
///
/// - `blocklist`: substrings that may not appear anywhere in the action.
/// - `blocklist_standalone`: strings that may not be the *entire* action (trimmed).
pub fn is_blocked(action: &str, blocklist: &[String], blocklist_standalone: &[String]) -> bool {
    for entry in blocklist {
        if action.contains(entry.as_str()) {
            return true;
        }
    }
    let trimmed = action.trim();
    for entry in blocklist_standalone {
        if trimmed == entry.as_str() {
            return true;
        }
    }
    false
}

// ---------------------------------------------------------------------------
// AgentConfig
// ---------------------------------------------------------------------------

pub struct AgentConfig {
    pub max_steps: u32,
    pub max_requeries: u32,
    pub max_observation_length: usize,
    pub system_template: String,
    pub instance_template: String,
    pub next_step_template: String,
    pub parser_type: String,
    pub submit_command: String,
    pub execution_timeout_secs: u64,
    pub blocklist: Vec<String>,
    pub blocklist_standalone: Vec<String>,
    pub history_processors: Vec<BoxedProcessor>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_steps: 100,
            max_requeries: 3,
            max_observation_length: 100_000,
            system_template: concat!(
                "You are an expert software engineer with access to a bash shell inside a Docker container.\n",
                "The repository you are working on has been cloned to {repo}.\n",
                "\n",
                "## FORMAT — non-negotiable\n",
                "Every single response MUST follow this structure:\n",
                "1. A short reasoning section (plain text) explaining what you are about to do.\n",
                "2. Exactly ONE ```bash``` code block containing the command to run.\n",
                "No response is valid without a ```bash``` block. No exceptions.\n",
                "\n",
                "## RULES\n",
                "- One bash block per response. Never two.\n",
                "- Put reasoning BEFORE the block, never inside it.\n",
                "- Keep commands focused: one logical action per step.\n",
                "- Never guess file contents — read them first with cat or head.\n",
                "- When the task is fully complete, submit with:\n",
                "  ```bash\n",
                "  submit\n",
                "  ```\n",
                "\n",
                "## WORKFLOW\n",
                "1. Explore the repo structure (ls, find, cat key files).\n",
                "2. Understand existing conventions before writing new code.\n",
                "3. Implement the minimum change that satisfies the task.\n",
                "4. Verify your change is correct (read the file back, run tests if available).\n",
                "5. Submit.\n",
            ).to_string(),
            instance_template: concat!(
                "TASK:\n",
                "{problem_statement}\n",
                "\n",
                "Repository: {repo}\n",
                "Start by exploring the repo structure, then implement the solution and run `submit`.\n",
            ).to_string(),
            next_step_template: "Observation: {observation}".to_string(),
            parser_type: "thought_action".to_string(),
            submit_command: "submit".to_string(),
            execution_timeout_secs: 30,
            blocklist: vec![],
            blocklist_standalone: vec![],
            history_processors: vec![],
        }
    }
}

// ---------------------------------------------------------------------------
// AgentRunResult
// ---------------------------------------------------------------------------

pub struct AgentRunResult {
    pub info: AgentInfo,
    pub trajectory: Trajectory,
    pub history: History,
}

// ---------------------------------------------------------------------------
// DefaultAgent
// ---------------------------------------------------------------------------

pub struct DefaultAgent {
    pub config: AgentConfig,
    pub model: SharedModel,
    pub history: History,
    pub trajectory: Trajectory,
    pub info: AgentInfo,
}

impl DefaultAgent {
    pub fn new(config: AgentConfig, model: SharedModel) -> Self {
        Self {
            config,
            model,
            history: Vec::new(),
            trajectory: Vec::new(),
            info: AgentInfo::default(),
        }
    }

    /// Apply all configured history processors in sequence.
    fn apply_processors(&self) -> History {
        self.config
            .history_processors
            .iter()
            .fold(self.history.clone(), |h, p| p.process(&h))
    }

    /// Run the full agent loop.
    pub async fn run(
        &mut self,
        env: &mut SweEnvironment,
        problem: &AnyProblemStatement,
        output_dir: Option<&Path>,
    ) -> Result<AgentRunResult, ForgeError> {
        // Reset model stats.
        self.model.reset_stats();

        // Get problem statement.
        let problem_text = problem.get_problem_statement().await?;

        // Gather template variables.
        let repo_path = env.config.repo_path.clone();
        let mut vars: HashMap<&str, &str> = HashMap::new();
        vars.insert("problem_statement", &problem_text);
        vars.insert("repo", &repo_path);

        // Build system message.
        let system_content = render_template(&self.config.system_template, &vars);
        self.history.push(HistoryItem {
            role: Role::System,
            content: MessageContent::Text(system_content),
            message_type: Some(MessageType::System),
            ..Default::default()
        });

        // Build instance template message.
        let instance_content = render_template(&self.config.instance_template, &vars);
        self.history.push(HistoryItem {
            role: Role::User,
            content: MessageContent::Text(instance_content),
            message_type: Some(MessageType::Observation),
            ..Default::default()
        });

        // Create the parser.
        let parser = get_parser(&self.config.parser_type)?;

        let mut requery_count: u32 = 0;
        let mut exit_status = ExitStatus::StepLimitReached;
        let mut submission: Option<String> = None;

        'steps: for _step in 0..self.config.max_steps {
            let step_start = Instant::now();

            // Apply history processors to get the query history.
            let processed = self.apply_processors();

            // Query the model.
            let model_output = self.model.query(&processed).await?;

            // Parse the output.
            let parsed = match parser.parse_model_output(&model_output) {
                Ok(p) => {
                    requery_count = 0;
                    p
                }
                Err(e) => {
                    // Parse failure: add feedback and retry.
                    requery_count += 1;
                    if requery_count > self.config.max_requeries {
                        exit_status = ExitStatus::Error;
                        break 'steps;
                    }
                    let error_msg = format!(
                        "Your response could not be parsed. Error: {}. Please try again.",
                        e
                    );
                    self.history.push(HistoryItem {
                        role: Role::User,
                        content: MessageContent::Text(error_msg),
                        message_type: Some(MessageType::Observation),
                        ..Default::default()
                    });
                    continue 'steps;
                }
            };

            let action = parsed.action.trim().to_string();
            let thought = parsed.thought.clone();

            // --- Special token checks ---

            // RETRY_WITHOUT_OUTPUT
            if contains_retry_without_output(&action) {
                self.history.push(HistoryItem {
                    role: Role::Assistant,
                    content: MessageContent::Text(model_output.message.clone()),
                    thought: Some(thought.clone()),
                    action: Some(action.clone()),
                    ..Default::default()
                });
                self.history.push(HistoryItem {
                    role: Role::User,
                    content: MessageContent::Text(String::new()),
                    message_type: Some(MessageType::Observation),
                    ..Default::default()
                });
                continue 'steps;
            }

            // RETRY_WITH_OUTPUT
            if contains_retry_with_output(&action) {
                let output = env
                    .execute_with_timeout(&action, self.config.execution_timeout_secs)
                    .await;
                let observation = match output {
                    Ok(cmd_out) => {
                        if cmd_out.exit_code == 0 {
                            cmd_out.stdout
                        } else {
                            format!("Error (exit code {}): {}", cmd_out.exit_code, cmd_out.stderr)
                        }
                    }
                    Err(e) => format!("Error running command: {}", e),
                };
                let observation = truncate_observation(observation, self.config.max_observation_length);
                self.history.push(HistoryItem {
                    role: Role::Assistant,
                    content: MessageContent::Text(model_output.message.clone()),
                    thought: Some(thought.clone()),
                    action: Some(action.clone()),
                    ..Default::default()
                });
                self.history.push(HistoryItem {
                    role: Role::User,
                    content: MessageContent::Text(observation),
                    message_type: Some(MessageType::Observation),
                    ..Default::default()
                });
                continue 'steps;
            }

            // EXIT_FORFEIT
            if contains_forfeit(&action) {
                exit_status = ExitStatus::Forfeited;
                break 'steps;
            }

            // --- Blocklist check ---
            if is_blocked(&action, &self.config.blocklist, &self.config.blocklist_standalone) {
                let block_msg = format!(
                    "Action blocked: '{}'. This command is not allowed. Please try a different approach.",
                    action
                );
                self.history.push(HistoryItem {
                    role: Role::Assistant,
                    content: MessageContent::Text(model_output.message.clone()),
                    thought: Some(thought.clone()),
                    action: Some(action.clone()),
                    ..Default::default()
                });
                self.history.push(HistoryItem {
                    role: Role::User,
                    content: MessageContent::Text(block_msg),
                    message_type: Some(MessageType::Observation),
                    ..Default::default()
                });
                continue 'steps;
            }

            // --- Submission check ---
            let is_submit = contains_submission(&action)
                || action.trim() == self.config.submit_command.as_str();

            if is_submit {
                // Run the action and collect the submission.
                let output = env
                    .execute_with_timeout(&action, self.config.execution_timeout_secs)
                    .await;
                let observation = match output {
                    Ok(cmd_out) => {
                        if cmd_out.exit_code == 0 {
                            cmd_out.stdout.clone()
                        } else {
                            format!("Error (exit code {}): {}", cmd_out.exit_code, cmd_out.stderr)
                        }
                    }
                    Err(e) => format!("Error running submit: {}", e),
                };

                submission = Some(observation.clone());
                exit_status = ExitStatus::Submitted;

                // Add to history.
                self.history.push(HistoryItem {
                    role: Role::Assistant,
                    content: MessageContent::Text(model_output.message.clone()),
                    thought: Some(thought.clone()),
                    action: Some(action.clone()),
                    ..Default::default()
                });
                self.history.push(HistoryItem {
                    role: Role::User,
                    content: MessageContent::Text(observation.clone()),
                    message_type: Some(MessageType::Observation),
                    ..Default::default()
                });

                // Record trajectory step.
                let execution_time = step_start.elapsed().as_secs_f64();
                self.trajectory.push(TrajectoryStep {
                    thought: thought.clone(),
                    action: action.clone(),
                    observation,
                    response: model_output.message.clone(),
                    execution_time,
                    state: HashMap::new(),
                    query: vec![],
                    extra_info: HashMap::new(),
                });

                // Save trajectory.
                self.save_trajectory(problem.id(), output_dir).await?;

                break 'steps;
            }

            // --- Normal command execution ---
            let output = env
                .execute_with_timeout(&action, self.config.execution_timeout_secs)
                .await;
            let observation = match output {
                Ok(cmd_out) => {
                    if cmd_out.exit_code == 0 {
                        cmd_out.stdout
                    } else {
                        format!("Error (exit code {}): {}", cmd_out.exit_code, cmd_out.stderr)
                    }
                }
                Err(e) => format!("Error running command: {}", e),
            };
            let observation = truncate_observation(observation, self.config.max_observation_length);

            let execution_time = step_start.elapsed().as_secs_f64();

            // Add assistant message.
            self.history.push(HistoryItem {
                role: Role::Assistant,
                content: MessageContent::Text(model_output.message.clone()),
                thought: Some(thought.clone()),
                action: Some(action.clone()),
                ..Default::default()
            });

            // Build next-step observation.
            let mut obs_vars: HashMap<&str, &str> = HashMap::new();
            obs_vars.insert("observation", &observation);
            let next_step_content = render_template(&self.config.next_step_template, &obs_vars);

            // Add observation message.
            self.history.push(HistoryItem {
                role: Role::User,
                content: MessageContent::Text(next_step_content),
                message_type: Some(MessageType::Observation),
                ..Default::default()
            });

            // Record trajectory step.
            self.trajectory.push(TrajectoryStep {
                thought: thought.clone(),
                action: action.clone(),
                observation: observation.clone(),
                response: model_output.message.clone(),
                execution_time,
                state: HashMap::new(),
                query: vec![],
                extra_info: HashMap::new(),
            });

            // Save trajectory.
            self.save_trajectory(problem.id(), output_dir).await?;
        }

        // Collect model stats.
        let stats = self.model.stats();
        let mut model_stats = HashMap::new();
        model_stats.insert(
            "totalCost".to_string(),
            serde_json::json!(stats.total_cost),
        );
        model_stats.insert(
            "totalInputTokens".to_string(),
            serde_json::json!(stats.total_input_tokens),
        );
        model_stats.insert(
            "totalOutputTokens".to_string(),
            serde_json::json!(stats.total_output_tokens),
        );
        model_stats.insert(
            "apiCalls".to_string(),
            serde_json::json!(stats.api_calls),
        );

        self.info.exit_status = Some(exit_status.as_str().to_string());
        self.info.submission = submission;
        self.info.model_stats = model_stats;

        Ok(AgentRunResult {
            info: self.info.clone(),
            trajectory: self.trajectory.clone(),
            history: self.history.clone(),
        })
    }

    /// Save the trajectory to `{output_dir}/{problem_id}.traj`.
    async fn save_trajectory(&self, problem_id: &str, output_dir: Option<&Path>) -> Result<(), ForgeError> {
        let Some(dir) = output_dir else {
            return Ok(());
        };

        let path = dir.join(format!("{}.traj", problem_id));
        let traj_file = TrajFile {
            trajectory: self.trajectory.clone(),
            info: self.info.clone(),
            history: Some(self.history.clone()),
            replay_config: None,
            environment: "docker".to_string(),
        };

        let json = serde_json::to_string_pretty(&traj_file)?;
        tokio::fs::write(path, json).await?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    // --- Pure function tests (no Docker needed) ---

    #[test]
    fn render_template_basic() {
        let mut vars = HashMap::new();
        vars.insert("name", "world");
        vars.insert("repo", "/repo");
        let result = render_template("Hello {name}, repo is {repo}.", &vars);
        assert_eq!(result, "Hello world, repo is /repo.");
    }

    #[test]
    fn render_template_missing_key_stays() {
        let vars: HashMap<&str, &str> = HashMap::new();
        let result = render_template("Hello {missing}!", &vars);
        assert_eq!(result, "Hello {missing}!");
    }

    #[test]
    fn render_template_multiple_occurrences() {
        let mut vars = HashMap::new();
        vars.insert("x", "42");
        let result = render_template("{x} and {x}", &vars);
        assert_eq!(result, "42 and 42");
    }

    #[test]
    fn truncate_observation_short_unchanged() {
        let obs = "short".to_string();
        let result = truncate_observation(obs.clone(), 1000);
        assert_eq!(result, obs);
    }

    #[test]
    fn truncate_observation_long_is_clipped() {
        let obs = "a".repeat(200);
        let result = truncate_observation(obs, 100);
        assert!(result.starts_with(&"a".repeat(100)));
        assert!(result.contains("<response clipped>"));
        assert!(result.contains("100 characters"));
        assert!(result.contains("100 characters were elided"));
    }

    #[test]
    fn truncate_observation_exact_length_unchanged() {
        let obs = "x".repeat(50);
        let result = truncate_observation(obs.clone(), 50);
        assert_eq!(result, obs);
    }

    #[test]
    fn is_blocked_substring_match() {
        let blocklist = vec!["rm -rf /".to_string()];
        let standalone: Vec<String> = vec![];
        assert!(is_blocked("sudo rm -rf / --no-preserve-root", &blocklist, &standalone));
        assert!(!is_blocked("ls -la", &blocklist, &standalone));
    }

    #[test]
    fn is_blocked_standalone_match() {
        let blocklist: Vec<String> = vec![];
        let standalone = vec!["exit".to_string()];
        assert!(is_blocked("exit", &blocklist, &standalone));
        assert!(is_blocked("  exit  ", &blocklist, &standalone));
        assert!(!is_blocked("exit 1", &blocklist, &standalone));
    }

    #[test]
    fn is_blocked_no_match() {
        let blocklist = vec!["forbidden".to_string()];
        let standalone = vec!["exit".to_string()];
        assert!(!is_blocked("ls -la", &blocklist, &standalone));
    }

    #[test]
    fn default_agent_constructs() {
        use forge_model::replay::ReplayModel;

        let model: SharedModel = Arc::new(ReplayModel::from_responses(vec![]));
        let config = AgentConfig::default();
        let agent = DefaultAgent::new(config, model);

        assert_eq!(agent.history.len(), 0);
        assert_eq!(agent.trajectory.len(), 0);
        assert!(agent.config.max_steps == 100);
        assert_eq!(agent.config.submit_command, "submit");
    }

    #[test]
    fn agent_config_defaults() {
        let cfg = AgentConfig::default();
        assert_eq!(cfg.max_steps, 100);
        assert_eq!(cfg.max_requeries, 3);
        assert_eq!(cfg.max_observation_length, 100_000);
        assert_eq!(cfg.execution_timeout_secs, 30);
        assert_eq!(cfg.submit_command, "submit");
        assert_eq!(cfg.parser_type, "thought_action");
        assert!(cfg.blocklist.is_empty());
        assert!(cfg.blocklist_standalone.is_empty());
        assert!(cfg.history_processors.is_empty());
    }
}
