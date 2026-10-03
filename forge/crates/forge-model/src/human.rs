use std::sync::Mutex;

use async_trait::async_trait;

use forge_types::{ForgeError, History, ModelOutput};

use crate::traits::{AbstractModel, InstanceStats};

pub struct HumanModel {
    stats: Mutex<InstanceStats>,
    prompt: String,
}

impl HumanModel {
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            stats: Mutex::new(InstanceStats::default()),
            prompt: prompt.into(),
        }
    }

    fn read_line(&self) -> Result<String, ForgeError> {
        use std::io::{BufRead, Write};

        let stderr = std::io::stderr();
        let mut lock = stderr.lock();
        write!(lock, "{}", self.prompt).map_err(ForgeError::Io)?;
        lock.flush().map_err(ForgeError::Io)?;
        drop(lock);

        let stdin = std::io::stdin();
        let mut line = String::new();
        stdin
            .lock()
            .read_line(&mut line)
            .map_err(ForgeError::Io)?;

        if line.is_empty() {
            // EOF
            return Err(ForgeError::Model("EOF on stdin".into()));
        }

        Ok(line.trim_end_matches('\n').trim_end_matches('\r').to_string())
    }
}

impl Default for HumanModel {
    fn default() -> Self {
        Self::new("> ")
    }
}

#[async_trait]
impl AbstractModel for HumanModel {
    async fn query(&self, _history: &History) -> Result<ModelOutput, ForgeError> {
        let line = self.read_line()?;

        {
            let mut stats = self.stats.lock().unwrap_or_else(|p| p.into_inner());
            stats.add_tokens(0, 0, 0.0);
        }

        Ok(ModelOutput {
            message: line,
            tool_calls: None,
            thinking_blocks: None,
            input_tokens: None,
            output_tokens: None,
            cost: Some(0.0),
        })
    }

    fn stats(&self) -> InstanceStats {
        self.stats.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }

    fn reset_stats(&self) {
        *self.stats.lock().unwrap_or_else(|p| p.into_inner()) = InstanceStats::default();
    }
}
