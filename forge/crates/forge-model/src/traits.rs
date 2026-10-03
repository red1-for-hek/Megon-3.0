use async_trait::async_trait;
use forge_types::{ForgeError, History, ModelOutput};

#[derive(Debug, Clone, Default)]
pub struct InstanceStats {
    pub total_input_tokens: u32,
    pub total_output_tokens: u32,
    pub total_cost: f64,
    pub api_calls: u32,
}

impl InstanceStats {
    pub fn add_tokens(&mut self, input: u32, output: u32, cost: f64) {
        self.total_input_tokens += input;
        self.total_output_tokens += output;
        self.total_cost += cost;
        self.api_calls += 1;
    }

    pub fn check_instance_cost_limit(&self, limit: f64) -> Result<(), ForgeError> {
        if limit > 0.0 && self.total_cost >= limit {
            Err(ForgeError::InstanceCostLimitExceeded)
        } else {
            Ok(())
        }
    }

    pub fn check_call_limit(&self, limit: u32) -> Result<(), ForgeError> {
        if limit > 0 && self.api_calls >= limit {
            Err(ForgeError::InstanceCallLimitExceeded)
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct GlobalStats {
    pub total_cost: f64,
}

impl GlobalStats {
    pub fn add_cost(&mut self, cost: f64) {
        self.total_cost += cost;
    }

    pub fn check_total_cost_limit(&self, limit: f64) -> Result<(), ForgeError> {
        if limit > 0.0 && self.total_cost >= limit {
            Err(ForgeError::TotalCostLimitExceeded)
        } else {
            Ok(())
        }
    }
}

#[async_trait]
pub trait AbstractModel: Send + Sync {
    async fn query(&self, history: &History) -> Result<ModelOutput, ForgeError>;
    fn stats(&self) -> InstanceStats;
    fn reset_stats(&self);
}

pub type SharedModel = std::sync::Arc<dyn AbstractModel>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instance_stats_accumulates() {
        let mut stats = InstanceStats::default();
        stats.add_tokens(100, 50, 0.01);
        stats.add_tokens(200, 100, 0.02);
        assert_eq!(stats.total_input_tokens, 300);
        assert_eq!(stats.total_output_tokens, 150);
        assert!((stats.total_cost - 0.03).abs() < 1e-9);
        assert_eq!(stats.api_calls, 2);
    }

    #[test]
    fn cost_limit_exceeded() {
        let mut stats = InstanceStats::default();
        stats.add_tokens(0, 0, 5.0);
        assert!(stats.check_instance_cost_limit(3.0).is_err());
        assert!(stats.check_instance_cost_limit(10.0).is_ok());
    }

    #[test]
    fn call_limit_exceeded() {
        let mut stats = InstanceStats::default();
        for _ in 0..5 {
            stats.add_tokens(1, 1, 0.001);
        }
        assert!(stats.check_call_limit(3).is_err());
        assert!(stats.check_call_limit(10).is_ok());
    }

    #[test]
    fn global_stats_total_cost_limit() {
        let mut gs = GlobalStats::default();
        gs.add_cost(10.0);
        assert!(gs.check_total_cost_limit(5.0).is_err());
        assert!(gs.check_total_cost_limit(0.0).is_ok()); // 0 = disabled
        assert!(gs.check_total_cost_limit(20.0).is_ok());
    }
}
