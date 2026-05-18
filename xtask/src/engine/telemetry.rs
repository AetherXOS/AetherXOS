use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct BuildMetrics {
    pub total_duration_ms: u64,
    pub task_durations_ms: HashMap<String, u64>,
    pub artifact_sizes: HashMap<String, u64>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl BuildMetrics {
    pub fn new() -> Self {
        Self {
            timestamp: chrono::Utc::now(),
            ..Default::default()
        }
    }

    pub fn report(&self) {
        use crate::utils::logging;
        logging::info(
            "TELEMETRY",
            "Build Performance Report",
            &[(
                "total",
                &format!("{:.2}s", self.total_duration_ms as f32 / 1000.0),
            )],
        );

        for (task, dur_ms) in &self.task_durations_ms {
            logging::info(
                "METRIC",
                task,
                &[("duration", &format!("{:.2}s", *dur_ms as f32 / 1000.0))],
            );
        }
    }
}
