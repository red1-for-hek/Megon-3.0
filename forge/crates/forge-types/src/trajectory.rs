use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::history::History;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TrajectoryStep {
    pub action: String,
    pub observation: String,
    pub response: String,
    pub state: HashMap<String, String>,
    pub thought: String,
    #[serde(alias = "executionTime")]
    pub execution_time: f64,
    #[serde(default)]
    pub query: Vec<serde_json::Value>,
    #[serde(default)]
    pub extra_info: HashMap<String, serde_json::Value>,
}

pub type Trajectory = Vec<TrajectoryStep>;

/// AgentInfo — supports both camelCase (TS) and snake_case on read, writes camelCase
#[derive(Debug, Clone, Default)]
pub struct AgentInfo {
    pub exit_status: Option<String>,
    pub submission: Option<String>,
    pub model_stats: HashMap<String, serde_json::Value>,
    pub extra: HashMap<String, serde_json::Value>,
}

impl Serialize for AgentInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(None)?;
        if let Some(ref val) = self.exit_status {
            map.serialize_entry("exitStatus", val)?;
        }
        if let Some(ref val) = self.submission {
            map.serialize_entry("submission", val)?;
        }
        if !self.model_stats.is_empty() {
            map.serialize_entry("modelStats", &self.model_stats)?;
        }
        for (k, v) in &self.extra {
            map.serialize_entry(k, v)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for AgentInfo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{MapAccess, Visitor};
        use std::fmt;

        struct AgentInfoVisitor;

        impl<'de> Visitor<'de> for AgentInfoVisitor {
            type Value = AgentInfo;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct AgentInfo")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut exit_status = None;
                let mut submission = None;
                let mut model_stats = HashMap::new();
                let mut extra = HashMap::new();

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "exitStatus" | "exit_status" => {
                            exit_status = map.next_value()?;
                        }
                        "submission" => {
                            submission = map.next_value()?;
                        }
                        "modelStats" | "model_stats" => {
                            model_stats = map.next_value()?;
                        }
                        _ => {
                            let value = map.next_value()?;
                            extra.insert(key, value);
                        }
                    }
                }

                Ok(AgentInfo {
                    exit_status,
                    submission,
                    model_stats,
                    extra,
                })
            }
        }

        deserializer.deserialize_map(AgentInfoVisitor)
    }
}

/// The on-disk .traj file format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajFile {
    pub trajectory: Trajectory,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub history: Option<History>,
    pub info: AgentInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replay_config: Option<String>,
    #[serde(default = "default_environment")]
    pub environment: String,
}

fn default_environment() -> String {
    "docker".into()
}

/// The predictions.jsonl line format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionEntry {
    pub instance_id: String,
    pub model_patch: String,
    pub model_name_or_path: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trajectory_step_round_trip() {
        let step = TrajectoryStep {
            action: "ls -l".into(),
            observation: "file.py".into(),
            response: "thinking...".into(),
            thought: "let me check".into(),
            execution_time: 0.5,
            ..Default::default()
        };
        let json = serde_json::to_string(&step).unwrap();
        let back: TrajectoryStep = serde_json::from_str(&json).unwrap();
        assert_eq!(back.action, "ls -l");
        assert!((back.execution_time - 0.5).abs() < 1e-9);
    }

    #[test]
    fn agent_info_camel_case_read() {
        // TS writes exitStatus (camelCase)
        let json = r#"{"exitStatus": "submitted", "modelStats": {"totalCost": 0.5}}"#;
        let info: AgentInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.exit_status.as_deref(), Some("submitted"));
    }

    #[test]
    fn agent_info_snake_case_read() {
        // also accept snake_case
        let json = r#"{"exit_status": "exit_cost", "model_stats": {}}"#;
        let info: AgentInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.exit_status.as_deref(), Some("exit_cost"));
    }

    #[test]
    fn trajectory_step_camel_case_execution_time() {
        let json = r#"{
            "action": "ls",
            "observation": "file.txt",
            "response": "done",
            "state": {},
            "thought": "check files",
            "executionTime": 1.5,
            "query": [],
            "extraInfo": {}
        }"#;
        let step: TrajectoryStep = serde_json::from_str(json).unwrap();
        assert_eq!(step.execution_time, 1.5);
    }

    #[test]
    fn prediction_entry_serializes() {
        let entry = PredictionEntry {
            instance_id: "repo__issue-1".into(),
            model_patch: "diff --git ...".into(),
            model_name_or_path: "claude-3-5-sonnet".into(),
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("instance_id"));
        assert!(json.contains("model_patch"));
        assert!(json.contains("model_name_or_path"));
    }
}
