//! Public numeric telemetry allowlist, independent of the harness wire format.
use crate::common::{now, number, sha256};
use serde_json::{Value, json};
pub const NUMBERS: &[&str] = &[
    "input",
    "output_tokens",
    "cache_read",
    "cache_write",
    "context",
    "window",
    "usage_seq",
    "total_input",
    "total_output",
    "total_cache_read",
    "total_cache_write",
    "total_uncached_input",
    "compactions",
    "context_percent",
];
pub const GROUPS: &[&[&str]] = &[
    &["input", "output_tokens", "cache_read", "cache_write"],
    &["context", "window", "usage_seq", "total_input"],
    &[
        "total_output",
        "total_cache_read",
        "total_cache_write",
        "total_uncached_input",
    ],
    &["compactions", "context_percent"],
];
pub const OUTCOMES: &[&str] = &[
    "subagent_running",
    "subagent_completed",
    "subagent_interrupted",
    "subagent_failed",
    "subagent_unknown",
];
pub const EVENTS: &[&str] = &[
    "session",
    "turn",
    "tool-start",
    "tool-end",
    "thinking",
    "output",
    "compact-start",
    "compact-end",
    "compact-failed",
    "idle",
    "interrupt",
    "end",
    "model",
    "subagent-start",
    "subagent-stop",
];
pub const PHASES: &[&str] = &[
    "ready",
    "working",
    "tool",
    "thinking",
    "output",
    "compacting",
    "idle",
    "interrupted",
    "ended",
];
pub fn session_binding(agent: &Value) -> Option<String> {
    let harness = agent["agent"].as_str()?;
    let session = &agent["agent_session"];
    if session["agent"] != harness || session["source"] != format!("herdr:{harness}") {
        return None;
    }
    let kind = session["kind"].as_str()?;
    let value = session["value"].as_str()?;
    if !["id", "path"].contains(&kind) || value.is_empty() {
        return None;
    }
    Some(sha256(format!("{harness}:{kind}:{value}").as_bytes()))
}
pub fn safe_model(model: &str) -> bool {
    model.len() <= 64
        && !model.contains("..")
        && !model.contains(":/")
        && model.split('/').count() <= 2
        && model.split('/').all(|part| {
            !part.is_empty()
                && part.as_bytes()[0].is_ascii_alphanumeric()
                && part
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"_.:-".contains(&c))
        })
}
pub fn telemetry_view(raw: &Value) -> Option<Value> {
    telemetry_view_at(raw, now())
}
pub fn telemetry_view_at(raw: &Value, time: f64) -> Option<Value> {
    raw.as_object()?;
    let seq = number(&raw["seq"])?;
    if seq as f64 / 1e6 > time
        || !EVENTS.contains(&raw["event"].as_str()?)
        || !PHASES.contains(&raw["phase"].as_str()?)
    {
        return None;
    }
    let mut result = json!({"seq":seq,"event":raw["event"],"phase":raw["phase"]});
    let known = [
        "Bash",
        "bash",
        "apply_patch",
        "read",
        "write",
        "edit",
        "grep",
        "find",
        "ls",
        "exec_command",
        "write_stdin",
        "exec",
        "web",
        "mcp-tool",
        "custom-tool",
    ];
    result["tool"] = raw["tool"]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(|tool| {
            json!(if known.contains(&tool) {
                tool
            } else if tool.starts_with("mcp") {
                "mcp-tool"
            } else {
                "custom-tool"
            })
        })
        .unwrap_or(Value::Null);
    result["model"] = raw["model"]
        .as_str()
        .filter(|model| safe_model(model))
        .map(|s| json!(s))
        .unwrap_or(Value::Null);
    result["result"] = raw["result"]
        .as_str()
        .filter(|s| ["finished", "error", "cancelled"].contains(s))
        .map(|s| json!(s))
        .unwrap_or(Value::Null);
    for key in NUMBERS {
        result[*key] = json!(number(&raw[*key]));
    }
    let starts = number(&raw["subagent_starts"]);
    let stops = number(&raw["subagent_stops"]);
    let child_seq = number(&raw["subagent_seq"]);
    let valid = starts.is_some_and(|v| v <= 999)
        && stops.is_some_and(|v| v <= 999)
        && (raw["subagent_seq"].is_null() || child_seq.is_some_and(|v| v <= seq))
        && (!(starts.unwrap_or(0) > 0 || stops.unwrap_or(0) > 0) || child_seq.is_some());
    result["subagent_starts"] = json!(if valid { starts } else { None });
    result["subagent_stops"] = json!(if valid { stops } else { None });
    result["subagent_seq"] = json!(if valid { child_seq } else { None });
    let total = number(&raw["subagent_total"]);
    let done = number(&raw["subagent_done"]);
    let stamp = number(&raw["subagent_status_seq"]);
    let valid = matches!((total,done,stamp),(Some(t),Some(d),Some(s))if d<=t&&t<=128&&s>0&&s<=seq);
    for key in ["subagent_total", "subagent_done", "subagent_status_seq"] {
        result[key] = if valid { raw[key].clone() } else { Value::Null };
    }
    let outcomes: Option<Vec<u64>> = OUTCOMES
        .iter()
        .map(|k| number(&raw[*k]).filter(|v| *v <= 128))
        .collect();
    let coherent = valid
        && outcomes
            .as_ref()
            .is_some_and(|v| Some(v.iter().sum::<u64>()) == total && Some(v[1]) == done);
    for key in OUTCOMES {
        result[*key] = if coherent {
            raw[*key].clone()
        } else {
            Value::Null
        };
    }
    result["usage_source"] = raw["usage_source"]
        .as_str()
        .filter(|s| ["codex-rollout", "pi-extension", "claude-transcript"].contains(s))
        .map(|s| json!(s))
        .unwrap_or(Value::Null);
    if (!raw["usage_seq"].is_null() || !raw["usage_source"].is_null())
        && number(&result["usage_seq"]).is_none_or(|v| v as f64 / 1e6 > time)
    {
        for key in NUMBERS {
            result[*key] = Value::Null;
        }
        result["usage_source"] = Value::Null;
    }
    if number(&result["context_percent"]).is_some_and(|v| v > 100) {
        result["context_percent"] = Value::Null;
    }
    if matches!((number(&result["total_input"]),number(&result["total_cache_read"])),(Some(i),Some(c))if c>i)
    {
        for key in ["total_input", "total_cache_read", "total_uncached_input"] {
            result[key] = Value::Null;
        }
    }
    if number(&result["window"]) == Some(0)
        || matches!((number(&result["context"]),number(&result["window"])),(Some(c),Some(w))if c>w)
    {
        for key in ["context", "window", "context_percent"] {
            result[key] = Value::Null;
        }
    }
    Some(result)
}
fn decimal(value: &str, max: usize) -> Option<u64> {
    if value.is_empty() || value.len() > max || !value.bytes().all(|c| c.is_ascii_digit()) {
        None
    } else {
        value.parse().ok()
    }
}
pub fn telemetry_from_agent(agent: &Value) -> Option<Value> {
    let tokens = agent["tokens"].as_object()?;
    let version = tokens.get("obs_v")?.as_str()?;
    if !["1", "2"].contains(&version)
        || tokens.get("obs_bind")?.as_str()? != session_binding(agent)?
    {
        return None;
    }
    let mut raw = json!({});
    for key in [
        "seq",
        "event",
        "phase",
        "tool",
        "model",
        "result",
        "usage_source",
    ] {
        raw[key] = tokens
            .get(&format!("obs_{key}"))
            .cloned()
            .unwrap_or(Value::Null);
    }
    raw["seq"] = json!(raw["seq"].as_str().and_then(|v| decimal(v, 16)));
    if version == "2" {
        for (index, fields) in GROUPS.iter().enumerate() {
            let packed = tokens.get(&format!("obs_n{index}"))?.as_str()?;
            if packed.len() > 67 {
                return None;
            }
            let values: Vec<_> = packed.split(',').collect();
            if values.len() != fields.len() {
                return None;
            }
            for (key, value) in fields.iter().zip(values) {
                raw[*key] = if value.is_empty() {
                    Value::Null
                } else {
                    json!(decimal(value, 16).filter(|v| *v <= 9_007_199_254_740_991)?)
                };
            }
        }
        for (source, fields, widths) in [
            (
                "obs_completion",
                &["subagent_total", "subagent_done", "subagent_status_seq"][..],
                &[3, 3, 16][..],
            ),
            ("obs_outcomes", OUTCOMES, &[3, 3, 3, 3, 3][..]),
            (
                "obs_children",
                &["subagent_starts", "subagent_stops", "subagent_seq"][..],
                &[3, 3, 16][..],
            ),
        ] {
            if let Some(value) = tokens.get(source).and_then(Value::as_str) {
                let values: Vec<_> = value.split(',').collect();
                if values.len() == fields.len()
                    && values.iter().zip(widths).enumerate().all(|(i, (v, w))| {
                        decimal(v, *w).is_some()
                            || (source == "obs_children" && i == 2 && v.is_empty())
                    })
                {
                    for (key, value) in fields.iter().zip(values) {
                        raw[*key] = json!(decimal(value, 16));
                    }
                }
            }
        }
    } else {
        for key in NUMBERS {
            raw[*key] = json!(
                tokens
                    .get(&format!("obs_{key}"))
                    .and_then(Value::as_str)
                    .and_then(|v| decimal(v, 16))
            );
        }
    }
    telemetry_view(&raw)
}
pub fn turn_timing_view(raw: &Value) -> Option<Value> {
    let time = now();
    let observed = raw["observed_at_s"].as_f64()?;
    let active = raw["active"].as_bool();
    let start = number(&raw["started_at_s"]);
    let complete = raw["complete"].as_bool()?;
    if !observed.is_finite()
        || observed <= 0.0
        || observed > time + 1.0
        || (!raw["active"].is_null() && active.is_none())
        || (!raw["started_at_s"].is_null() && start.is_none())
        || start.is_some_and(|v| v == 0 || v as f64 > observed)
        || (active == Some(true)) != start.is_some()
    {
        return None;
    }
    let mut result =
        json!({"active":active,"started_at_s":start,"observed_at_s":observed,"complete":complete});
    for key in ["last_duration_s", "total_finished_duration_s"] {
        if !raw[key].is_null() && number(&raw[key]).is_none() {
            return None;
        }
        result[key] = raw[key].clone();
    }
    if !complete {
        result["total_finished_duration_s"] = Value::Null;
    }
    if complete && result["total_finished_duration_s"].is_null() {
        return None;
    }
    if matches!((number(&result["last_duration_s"]),number(&result["total_finished_duration_s"])),(Some(a),Some(b))if a>b)
    {
        return None;
    }
    result["last_outcome"] = raw["last_outcome"]
        .as_str()
        .filter(|v| ["completed", "aborted"].contains(v))
        .map(|v| json!(v))
        .unwrap_or(Value::Null);
    if result["last_duration_s"].is_null() != result["last_outcome"].is_null() {
        return None;
    }
    let freshness = raw
        .get("freshness_seconds")
        .and_then(Value::as_f64)
        .unwrap_or(12.0);
    if !(12.0..=180.0).contains(&freshness) {
        return None;
    }
    result["freshness_seconds"] = json!(freshness);
    Some(result)
}
