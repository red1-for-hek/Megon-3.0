/// Persistent bash session running inside a Docker container via `docker exec`.
///
/// ## Sentinel pattern
///
/// After each user command we write a sentinel echo to bash's stdin:
///
/// ```text
/// <cmd>
/// echo '__FORGE_SENTINEL__' $?
/// ```
///
/// We then read stdout until we see a line that matches `^__FORGE_SENTINEL__\d+$`.
/// The digits give us the exit code of the command and we know the command is
/// done.
///
/// ## Why not `docker exec` per command?
///
/// Creating a new exec per command is expensive (~100 ms per call) and does
/// not preserve shell state (current directory, exports, etc.).  A persistent
/// session keeps state across calls exactly like a real interactive terminal.
use bollard::exec::{CreateExecOptions, StartExecOptions, StartExecResults};
use bollard::Docker;
use forge_types::ForgeError;
use futures_util::StreamExt;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};

/// The sentinel marker written after each command.  We include a fixed string
/// that is astronomically unlikely to appear in normal command output.
const SENTINEL: &str = "__FORGE_SENTINEL__";

/// Output produced by a single command execution.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i64,
}

/// A persistent bash session inside a Docker container.
pub struct BashSession {
    /// Retained for potential future use (e.g., inspecting the exec).
    #[allow(dead_code)]
    docker: Docker,
    /// Retained for potential future use (e.g., re-attaching on failure).
    #[allow(dead_code)]
    container_id: String,
    /// Channel through which we send data to bash's stdin.
    stdin_tx: mpsc::UnboundedSender<Vec<u8>>,
    /// Channel from which we receive demultiplexed stdout lines.
    stdout_rx: mpsc::UnboundedReceiver<String>,
    /// Channel from which we receive demultiplexed stderr lines.
    stderr_rx: mpsc::UnboundedReceiver<String>,
}

impl BashSession {
    /// Create a new persistent bash session inside `container_id`.
    ///
    /// This starts a `docker exec` running `/bin/bash -s` (read commands from
    /// stdin, non-interactive so no PS1 noise) and spawns background tasks to
    /// pump stdin/stdout/stderr through unbounded mpsc channels.
    pub async fn new(docker: Docker, container_id: &str) -> Result<Self, ForgeError> {
        // Create the exec object.
        let exec_id = docker
            .create_exec(
                container_id,
                CreateExecOptions {
                    cmd: Some(vec!["/bin/bash", "-s"]),
                    attach_stdin: Some(true),
                    attach_stdout: Some(true),
                    attach_stderr: Some(true),
                    tty: Some(false),
                    ..Default::default()
                },
            )
            .await
            .map_err(|e| ForgeError::Docker(format!("create_exec: {}", e)))?
            .id;

        // Start the exec.
        let start_result = docker
            .start_exec(
                &exec_id,
                Some(StartExecOptions {
                    detach: false,
                    ..Default::default()
                }),
            )
            .await
            .map_err(|e| ForgeError::Docker(format!("start_exec: {}", e)))?;

        match start_result {
            StartExecResults::Attached { mut output, input } => {
                // Channels
                let (stdin_tx, mut stdin_rx) = mpsc::unbounded_channel::<Vec<u8>>();
                let (stdout_tx, stdout_rx) = mpsc::unbounded_channel::<String>();
                let (stderr_tx, stderr_rx) = mpsc::unbounded_channel::<String>();

                // Pump stdin: forward bytes from the channel into the exec input stream.
                tokio::spawn(async move {
                    let mut writer = input;
                    while let Some(bytes) = stdin_rx.recv().await {
                        if writer.write_all(&bytes).await.is_err() {
                            break;
                        }
                    }
                });

                // Pump stdout/stderr: read the multiplexed output stream and
                // dispatch to the appropriate channel.
                tokio::spawn(async move {
                    let mut stdout_buf = String::new();
                    let mut stderr_buf = String::new();

                    while let Some(frame) = output.next().await {
                        match frame {
                            Ok(bollard::container::LogOutput::StdOut { message }) => {
                                let text = String::from_utf8_lossy(&message);
                                stdout_buf.push_str(&text);
                                // Flush complete lines, stripping any trailing \r so
                                // patches and other output never carry carriage-returns
                                // into the captured stdout string.
                                while let Some(pos) = stdout_buf.find('\n') {
                                    let line = stdout_buf[..pos]
                                        .trim_end_matches('\r')
                                        .to_string();
                                    stdout_buf = stdout_buf[pos + 1..].to_string();
                                    let _ = stdout_tx.send(line);
                                }
                            }
                            Ok(bollard::container::LogOutput::StdErr { message }) => {
                                let text = String::from_utf8_lossy(&message);
                                stderr_buf.push_str(&text);
                                while let Some(pos) = stderr_buf.find('\n') {
                                    let line = stderr_buf[..pos]
                                        .trim_end_matches('\r')
                                        .to_string();
                                    stderr_buf = stderr_buf[pos + 1..].to_string();
                                    let _ = stderr_tx.send(line);
                                }
                            }
                            Ok(bollard::container::LogOutput::Console { message }) => {
                                // TTY=false shouldn't produce console output, but handle
                                // it gracefully by treating it as stdout.
                                let text = String::from_utf8_lossy(&message);
                                stdout_buf.push_str(&text);
                                while let Some(pos) = stdout_buf.find('\n') {
                                    let line = stdout_buf[..pos]
                                        .trim_end_matches('\r')
                                        .to_string();
                                    stdout_buf = stdout_buf[pos + 1..].to_string();
                                    let _ = stdout_tx.send(line);
                                }
                            }
                            _ => {}
                        }
                    }

                    // Flush any remaining partial lines when the stream ends.
                    if !stdout_buf.is_empty() {
                        let _ = stdout_tx.send(stdout_buf.trim_end_matches('\r').to_string());
                    }
                    if !stderr_buf.is_empty() {
                        let _ = stderr_tx.send(stderr_buf.trim_end_matches('\r').to_string());
                    }
                });

                Ok(Self {
                    docker,
                    container_id: container_id.to_string(),
                    stdin_tx,
                    stdout_rx,
                    stderr_rx,
                })
            }
            StartExecResults::Detached => Err(ForgeError::Docker(
                "exec started in detached mode — expected attached".to_string(),
            )),
        }
    }

    /// Execute `cmd` in the persistent bash session.
    ///
    /// Returns a [`CommandOutput`] containing stdout, stderr, and the exit code.
    /// Times out after `timeout_secs` seconds, returning [`ForgeError::CommandTimeout`].
    pub async fn run_command(
        &mut self,
        cmd: &str,
        timeout_secs: u64,
    ) -> Result<CommandOutput, ForgeError> {
        // Write the command followed by the sentinel echo.
        let payload = format!("{}\necho '{}' $?\n", cmd, SENTINEL);
        self.stdin_tx
            .send(payload.into_bytes())
            .map_err(|_| ForgeError::Environment("bash session stdin closed".to_string()))?;

        // Collect output until we see the sentinel.
        let result = timeout(
            Duration::from_secs(timeout_secs),
            self.collect_until_sentinel(),
        )
        .await;

        match result {
            Ok(inner) => inner,
            Err(_) => Err(ForgeError::CommandTimeout),
        }
    }

    /// Write raw bytes to bash's stdin without waiting for a sentinel.
    ///
    /// Useful for setup operations (e.g., sourcing rc files) where we drive
    /// the shell manually and poll with a sentinel separately.
    pub fn write_raw(&mut self, data: &[u8]) -> Result<(), ForgeError> {
        self.stdin_tx
            .send(data.to_vec())
            .map_err(|_| ForgeError::Environment("bash session stdin closed".to_string()))
    }

    // ------------------------------------------------------------------
    // Internal helpers
    // ------------------------------------------------------------------

    /// Read stdout/stderr lines from the pumping tasks until we spot the
    /// sentinel.  Returns immediately when found.
    async fn collect_until_sentinel(&mut self) -> Result<CommandOutput, ForgeError> {
        let mut stdout_lines: Vec<String> = Vec::new();
        let mut stderr_lines: Vec<String> = Vec::new();
        let exit_code: i64;

        // Both stdout (sentinel detection) and stderr branches are polled.
        // tokio::select! uses pseudo-random selection; under heavy stderr load,
        // the stdout branch could be delayed but never starved permanently —
        // tokio guarantees fairness across wakeups. This is acceptable for
        // typical SWE-agent command outputs.
        loop {
            tokio::select! {
                // Drain stderr eagerly so it doesn't block the pumper.
                Some(line) = self.stderr_rx.recv() => {
                    stderr_lines.push(line);
                }
                Some(line) = self.stdout_rx.recv() => {
                    if let Some(code) = parse_sentinel(&line) {
                        exit_code = code;
                        break;
                    }
                    stdout_lines.push(line);
                }
                else => {
                    // Both channels closed — bash exited unexpectedly.
                    return Err(ForgeError::Environment(
                        "bash session ended before sentinel was received".to_string(),
                    ));
                }
            }
        }

        // Drain any remaining stderr that arrived while we were stopping.
        while let Ok(line) = self.stderr_rx.try_recv() {
            stderr_lines.push(line);
        }

        Ok(CommandOutput {
            stdout: stdout_lines.join("\n"),
            stderr: stderr_lines.join("\n"),
            exit_code,
        })
    }
}

/// If `line` matches `__FORGE_SENTINEL__ <exit_code>`, return the exit code.
fn parse_sentinel(line: &str) -> Option<i64> {
    let trimmed = line.trim();
    if let Some(rest) = trimmed.strip_prefix(SENTINEL) {
        let code_str = rest.trim();
        if let Ok(code) = code_str.parse::<i64>() {
            return Some(code);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sentinel_constant_value() {
        assert_eq!(SENTINEL, "__FORGE_SENTINEL__");
    }

    #[test]
    fn command_output_serializes() {
        let out = CommandOutput {
            stdout: "hello".to_string(),
            stderr: "".to_string(),
            exit_code: 0,
        };
        let json = serde_json::to_string(&out).expect("serialize");
        let back: CommandOutput = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.stdout, "hello");
        assert_eq!(back.exit_code, 0);
    }

    #[test]
    fn parse_sentinel_success() {
        assert_eq!(parse_sentinel("__FORGE_SENTINEL__ 0"), Some(0));
        assert_eq!(parse_sentinel("__FORGE_SENTINEL__ 1"), Some(1));
        assert_eq!(parse_sentinel("__FORGE_SENTINEL__ 127"), Some(127));
        assert_eq!(parse_sentinel("  __FORGE_SENTINEL__ 42  "), Some(42));
    }

    #[test]
    fn parse_sentinel_no_match() {
        assert_eq!(parse_sentinel("hello world"), None);
        assert_eq!(parse_sentinel("__FORGE_SENTINEL__abc"), None);
        assert_eq!(parse_sentinel(""), None);
        assert_eq!(parse_sentinel("SENTINEL 0"), None);
    }

    #[test]
    fn parse_sentinel_with_non_numeric() {
        assert_eq!(parse_sentinel("__FORGE_SENTINEL__ abc"), None);
    }

    #[tokio::test]
    #[ignore = "requires Docker daemon"]
    async fn bash_session_echo() {
        let docker = Docker::connect_with_local_defaults().expect("docker");

        // Start a simple alpine container that stays alive.
        use bollard::container::{Config, CreateContainerOptions, StartContainerOptions};
        let create_resp = docker
            .create_container(
                Some(CreateContainerOptions::<String> {
                    name: "forge-env-test-bash".to_string(),
                    platform: None,
                }),
                Config {
                    image: Some("alpine:latest"),
                    cmd: Some(vec!["tail", "-f", "/dev/null"]),
                    tty: Some(false),
                    open_stdin: Some(true),
                    ..Default::default()
                },
            )
            .await
            .expect("create");

        docker
            .start_container(&create_resp.id, None::<StartContainerOptions<String>>)
            .await
            .expect("start");

        let mut session = BashSession::new(docker.clone(), &create_resp.id)
            .await
            .expect("session");

        let out = session.run_command("echo hello", 10).await.expect("run");
        assert_eq!(out.stdout.trim(), "hello");
        assert_eq!(out.exit_code, 0);

        let out2 = session
            .run_command("exit_code_test() { return 42; }; exit_code_test", 10)
            .await
            .expect("run2");
        assert_eq!(out2.exit_code, 42);

        // Cleanup.
        use bollard::container::RemoveContainerOptions;
        let _ = docker.stop_container(&create_resp.id, None).await;
        docker
            .remove_container(
                &create_resp.id,
                Some(RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await
            .expect("remove");
    }

    #[tokio::test]
    #[ignore = "requires Docker daemon"]
    async fn bash_session_empty_output() {
        let docker = Docker::connect_with_local_defaults().expect("docker");

        use bollard::container::{Config, CreateContainerOptions, StartContainerOptions};
        let create_resp = docker
            .create_container(
                Some(CreateContainerOptions::<String> {
                    name: "forge-env-test-empty".to_string(),
                    platform: None,
                }),
                Config {
                    image: Some("alpine:latest"),
                    cmd: Some(vec!["tail", "-f", "/dev/null"]),
                    tty: Some(false),
                    open_stdin: Some(true),
                    ..Default::default()
                },
            )
            .await
            .expect("create");

        docker
            .start_container(&create_resp.id, None::<StartContainerOptions<String>>)
            .await
            .expect("start");

        let mut session = BashSession::new(docker.clone(), &create_resp.id)
            .await
            .expect("session");

        // A command that produces no output.
        let out = session.run_command("true", 10).await.expect("run");
        assert_eq!(out.stdout, "");
        assert_eq!(out.exit_code, 0);

        use bollard::container::RemoveContainerOptions;
        let _ = docker.stop_container(&create_resp.id, None).await;
        docker
            .remove_container(
                &create_resp.id,
                Some(RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await
            .expect("remove");
    }
}
