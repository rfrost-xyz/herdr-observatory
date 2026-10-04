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

/// Source status of one configured account. The view never synthesises it.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AllowanceStatus {
    Available,
    Unavailable,
    AuthNeeded,
}

/// One provider-neutral allowance window. `used_percent` is 0 to 100 and a
/// past `resets_at` is already null. At most one window per available row
/// has `pacing` set: every Codex row has one, and a Claude row has one only
/// when it carries a `seven_day` window.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct AllowanceWindow {
    pub kind: String,
    pub label: String,
    pub used_percent: Option<f64>,
    pub resets_at: Option<u64>,
    pub duration_s: u64,
    pub pacing: bool,
}

/// Popover allowance row: one per configured account, every key always
/// present. A row that is not available carries no windows, sample time or
/// reset metadata.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct AllowanceRow {
    pub provider: String,
    pub provider_label: String,
    pub account_id: String,
    pub label: String,
    pub status: AllowanceStatus,
    pub status_text: Option<String>,
    pub plan: Option<String>,
    pub sampled_at: Option<f64>,
    pub reset_count: Option<u64>,
    pub reset_expires_at: Option<u64>,
    pub windows: Vec<AllowanceWindow>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

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
    fn turn_timing_preserves_original_source_times() {
        let timing = json!({"active":true,"started_at_s":1000,"observed_at_s":1002.5,
            "complete":false,"last_duration_s":0,"total_finished_duration_s":null,
            "last_outcome":"completed","freshness_seconds":15.0});
        let parsed: TurnTiming = serde_json::from_value(timing.clone()).unwrap();
        assert_eq!(serde_json::to_value(parsed).unwrap(), timing);
    }

    const ALLOWANCE_KEYS: [&str; 11] = [
        "account_id",
        "label",
        "plan",
        "provider",
        "provider_label",
        "reset_count",
        "reset_expires_at",
        "sampled_at",
        "status",
        "status_text",
        "windows",
    ];

    fn roundtrip(value: &Value) -> Value {
        let row: AllowanceRow = serde_json::from_value(value.clone()).unwrap();
        let emitted = serde_json::to_value(row).unwrap();
        let keys: Vec<_> = emitted.as_object().unwrap().keys().cloned().collect();
        assert_eq!(keys, ALLOWANCE_KEYS);
        for window in emitted["windows"].as_array().unwrap() {
            let keys: Vec<_> = window.as_object().unwrap().keys().cloned().collect();
            assert_eq!(
                keys,
                [
                    "duration_s",
                    "kind",
                    "label",
                    "pacing",
                    "resets_at",
                    "used_percent"
                ]
            );
        }
        emitted
    }

    #[test]
    fn allowance_rows_roundtrip_with_every_contract_key() {
        let available = json!({"provider":"codex","provider_label":"Codex",
            "account_id":"synthetic","label":"Synthetic","status":"available",
            "status_text":null,"plan":"pro","sampled_at":1000.5,"reset_count":0,
            "reset_expires_at":3000,"windows":[{"kind":"weekly","label":"Weekly",
            "used_percent":100.0,"resets_at":2000,"duration_s":604800,"pacing":true}]});
        assert_eq!(roundtrip(&available), available);
        let unavailable = json!({"provider":"codex","provider_label":"Codex",
            "account_id":"synthetic","label":"Synthetic","status":"unavailable",
            "status_text":null,"plan":null,"sampled_at":null,"reset_count":null,
            "reset_expires_at":null,"windows":[]});
        assert_eq!(roundtrip(&unavailable), unavailable);
        let unknown = json!({"provider":"synthetic","provider_label":"Synthetic",
            "account_id":"synthetic","label":"Synthetic","status":"available",
            "status_text":"Synthetic status","plan":null,"sampled_at":1000.5,
            "reset_count":null,"reset_expires_at":null,"windows":[{"kind":"monthly",
            "label":"Monthly","used_percent":null,"resets_at":null,
            "duration_s":2592000,"pacing":true}]});
        assert_eq!(roundtrip(&unknown), unknown);
        let auth = json!({"provider":"synthetic","provider_label":"Synthetic",
            "account_id":"synthetic","label":"Synthetic","status":"auth_needed",
            "status_text":"Sign in again","plan":null,"sampled_at":null,
            "reset_count":null,"reset_expires_at":null,"windows":[]});
        assert_eq!(roundtrip(&auth), auth);
    }

    #[test]
    fn allowance_rows_drop_unknown_and_legacy_fields_and_reject_invented_status() {
        let mut value = json!({"provider":"codex","provider_label":"Codex",
            "account_id":"synthetic","label":"Synthetic","status":"available",
            "status_text":null,"plan":null,"sampled_at":1000.0,"reset_count":null,
            "reset_expires_at":null,"windows":[{"kind":"weekly","label":"Weekly",
            "used_percent":40.0,"resets_at":2000,"duration_s":604800,"pacing":true,
            "theme":"PRIVATE"}],"email":"PRIVATE","weekly_remaining":60,
            "lifetime_tokens":1234,"daily_usage":[]});
        let emitted = roundtrip(&value);
        assert!(!emitted.to_string().contains("PRIVATE"));
        assert!(emitted.get("weekly_remaining").is_none());
        assert!(emitted.get("lifetime_tokens").is_none());
        value["status"] = json!("invented");
        assert!(serde_json::from_value::<AllowanceRow>(value).is_err());
    }
}
