//! Explicit display contract. Unknown adapter fields never reach the desktop.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Working,
    Blocked,
    Done,
    Idle,
    #[default]
    Unknown,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Agent {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub navigation: Option<serde_json::Value>,
    #[serde(default)]
    pub host: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub checkout: String,
    #[serde(default)]
    pub project: String,
    #[serde(default)]
    pub harness: String,
    pub status: Status,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub technical: Technical,
    #[serde(default)]
    pub since: f64,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Technical {
    pub revision: Option<u64>,
    pub state_change_seq: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_generation: Option<u64>,
    pub focused: Option<bool>,
    pub interactive_ready: Option<bool>,
    pub launch_pending: Option<bool>,
    pub turn_timing: Option<TurnTiming>,
    pub telemetry: Option<Telemetry>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TurnTiming {
    pub active: Option<bool>,
    pub started_at_s: Option<u64>,
    pub observed_at_s: f64,
    pub complete: bool,
    pub last_duration_s: Option<u64>,
    pub total_finished_duration_s: Option<u64>,
    pub last_outcome: Option<String>,
    pub freshness_seconds: f64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Telemetry {
    pub seq: u64,
    pub event: String,
    pub phase: String,
    pub tool: Option<String>,
    pub model: Option<String>,
    pub result: Option<String>,
    pub input: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cache_read: Option<u64>,
    pub cache_write: Option<u64>,
    pub context: Option<u64>,
    pub window: Option<u64>,
    pub usage_seq: Option<u64>,
    pub total_input: Option<u64>,
    pub total_output: Option<u64>,
    pub total_cache_read: Option<u64>,
    pub total_cache_write: Option<u64>,
    pub total_uncached_input: Option<u64>,
    pub compactions: Option<u64>,
    pub context_percent: Option<u64>,
    pub subagent_starts: Option<u64>,
    pub subagent_stops: Option<u64>,
    pub subagent_seq: Option<u64>,
    pub subagent_total: Option<u64>,
    pub subagent_done: Option<u64>,
    pub subagent_status_seq: Option<u64>,
    pub subagent_running: Option<u64>,
    pub subagent_completed: Option<u64>,
    pub subagent_interrupted: Option<u64>,
    pub subagent_failed: Option<u64>,
    pub subagent_unknown: Option<u64>,
    pub usage_source: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct DailyUsage {
    pub date: String,
    pub tokens: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct AllowanceRow {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub monthly_used_percent: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub monthly_resets_at: Option<u64>,
    pub plan: Option<String>,
    pub weekly_remaining: Option<u64>,
    pub weekly_resets_at: Option<u64>,
    pub reset_count: Option<u64>,
    pub reset_expires_at: Option<u64>,
    pub sampled_at: Option<f64>,
    pub lifetime_tokens: Option<u64>,
    pub peak_daily_tokens: Option<u64>,
    pub daily_usage: Option<Vec<DailyUsage>>,
    pub label: String,
    pub available: bool,
    pub provider: String,
    pub provider_label: String,
    pub account_id: String,
    pub window_seconds: u64,
}

impl AllowanceRow {
    /// Match the domain cache's ten-minute source TTL and independent reset
    /// deadlines even while a remote refresh is blocked or unavailable.
    pub fn expire(&mut self, now: f64) -> bool {
        let before = self.clone();
        if self
            .sampled_at
            .is_none_or(|at| now - at > 600.0 || at > now + 1.0)
            || self.monthly_resets_at.is_none_or(|at| at as f64 <= now)
        {
            if self.provider == "notion" {
                self.available = false;
            }
            self.monthly_used_percent = None;
            self.monthly_resets_at = None;
        }
        if self
            .sampled_at
            .is_some_and(|at| now + 1.0 < at || now - at > 600.0)
        {
            self.available = false;
            self.sampled_at = None;
            self.plan = None;
            self.weekly_remaining = None;
            self.weekly_resets_at = None;
            self.reset_count = None;
            self.reset_expires_at = None;
            self.lifetime_tokens = None;
            self.peak_daily_tokens = None;
            self.daily_usage = None;
        }
        if self.weekly_resets_at.is_none_or(|at| at as f64 <= now) {
            self.weekly_remaining = None;
            self.weekly_resets_at = None;
        }
        if self.reset_expires_at.is_some_and(|at| at as f64 <= now) {
            self.reset_count = None;
            self.reset_expires_at = None;
        }
        *self != before
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn private_unknown_fields_are_not_reemitted() {
        let agent: Agent = serde_json::from_value(json!({"id":"test:pane", "status":"working",
            "cwd":"/private/path", "technical":{"raw_transcript":"secret"}}))
        .unwrap();
        let value = serde_json::to_value(agent).unwrap();
        assert!(value.get("cwd").is_none());
        assert!(value["technical"].get("raw_transcript").is_none());
        assert!(value["technical"]["revision"].is_null());
    }

    #[test]
    fn invalid_state_cannot_enter_model() {
        assert!(
            serde_json::from_value::<Agent>(json!({"id":"test:pane", "status":"invented"}))
                .is_err()
        );
        assert!(serde_json::from_value::<Technical>(json!({"revision":-1})).is_err());
    }

    #[test]
    fn reset_and_source_expiry_are_independent_of_refresh() {
        let mut row: AllowanceRow = serde_json::from_value(json!({
            "weekly_remaining":70,"weekly_resets_at":1010,"reset_count":2,
            "reset_expires_at":1020,"sampled_at":1000,"label":"Synthetic",
            "available":true,"provider":"codex","provider_label":"Codex",
            "account_id":"synthetic","window_seconds":604800
        }))
        .unwrap();
        assert!(row.expire(1010.0));
        assert!(row.weekly_remaining.is_none());
        assert_eq!(row.reset_count, Some(2));
        assert!(row.available);
        assert!(row.expire(1020.0));
        assert!(row.reset_count.is_none());
        assert!(row.expire(1601.0));
        assert!(!row.available);
        assert!(row.sampled_at.is_none());
    }

    #[test]
    fn complete_telemetry_contract_roundtrips_zero_and_unknown() {
        let value = json!({
            "seq":2000000,"event":"turn","phase":"working","tool":null,
            "model":"test-model","result":null,"input":0,"output_tokens":12,
            "cache_read":0,"cache_write":null,"context":1200,"window":10000,
            "usage_seq":1000000,"total_input":2000,"total_output":40,
            "total_cache_read":1000,"total_cache_write":null,"total_uncached_input":1000,
            "compactions":0,"context_percent":12,"subagent_starts":3,
            "subagent_stops":2,"subagent_seq":1900000,"subagent_total":3,
            "subagent_done":1,"subagent_status_seq":1950000,"subagent_running":1,
            "subagent_completed":1,"subagent_interrupted":0,"subagent_failed":1,
            "subagent_unknown":0,"usage_source":"codex-rollout"
        });
        let telemetry: Telemetry = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(telemetry).unwrap(), value);
    }

    #[test]
    fn turn_timing_and_allowance_preserve_original_source_times() {
        let timing = json!({"active":true,"started_at_s":1000,"observed_at_s":1002.5,
            "complete":false,"last_duration_s":0,"total_finished_duration_s":null,
            "last_outcome":"completed","freshness_seconds":15.0});
        let parsed: TurnTiming = serde_json::from_value(timing.clone()).unwrap();
        assert_eq!(serde_json::to_value(parsed).unwrap(), timing);
        let allowance = json!({"plan":"pro","weekly_remaining":0,"weekly_resets_at":2000,
            "reset_count":0,"reset_expires_at":3000,"sampled_at":1000.5,
            "lifetime_tokens":1234,"peak_daily_tokens":1234,
            "daily_usage":[{"date":"2026-01-01","tokens":1234}],
            "label":"Synthetic","available":true,"provider":"codex",
            "provider_label":"Codex","account_id":"synthetic","window_seconds":604800});
        let mut row: AllowanceRow = serde_json::from_value(allowance.clone()).unwrap();
        assert!(!row.expire(1050.0));
        assert_eq!(serde_json::to_value(row).unwrap(), allowance);
    }
}
