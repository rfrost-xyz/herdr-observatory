//! Account-bound read-only Codex quotas. Account identifiers never enter display rows.
//!
//! Source rows (`summarise`, `sanitise`, the private cache and the peer probe)
//! keep their original Codex shape so older peers and caches stay readable.
//! `snapshot` is the only conversion to the provider-neutral popover row.
use crate::model::{AllowanceRow, AllowanceStatus, AllowanceWindow};
use crate::{Result, common};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

pub const LIMIT: usize = 16384;
const SAFE: u64 = 9_007_199_254_740_991;
const PLANS: &[&str] = &[
    "free",
    "go",
    "plus",
    "pro",
    "prolite",
    "team",
    "self_serve_business_prolite",
    "self_serve_business_usage_based",
    "business",
    "ent26",
    "enterprise_cbp_automation",
    "enterprise_cbp_usage_based",
    "enterprise",
    "edu",
    "edu_plus",
    "edu_pro",
    "unknown",
];
pub const PEER: &str = "\"$HOME/.local/share/herdr.observatory-peer/anton-runtime\"";

fn number(value: &Value, maximum: u64) -> Option<u64> {
    value.as_u64().filter(|n| *n <= maximum)
}
fn hash_key(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
pub fn valid_target(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('-')
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.@:-".contains(&b))
}
pub fn configuration(config: &Value) -> Option<&Value> {
    if config.get("accounts").is_some() {
        Some(config)
    } else {
        config.get("allowances").filter(|v| !v.is_null())
    }
}
pub fn account_key(value: &str) -> Option<String> {
    (!value.is_empty() && value.chars().count() <= 256)
        .then(|| common::sha256(format!("observatory-codex-account-v1:{value}").as_bytes()))
}

pub fn mapping(value: &Value) -> Result<Value> {
    if matches!(value.as_str(), Some("Personal" | "Work")) {
        return Ok(json!({"id":value,"label":value,"category":value,"window_seconds":604800}));
    }
    let object = value
        .as_object()
        .ok_or("Invalid allowance account mapping")?;
    if object
        .keys()
        .any(|k| !["id", "label", "category", "window_seconds"].contains(&k.as_str()))
    {
        return Err("Invalid allowance account mapping".into());
    }
    let id = value["id"].as_str().unwrap_or("");
    let label = value["label"].as_str().unwrap_or("");
    if id.is_empty()
        || id.len() > 40
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
        || label.is_empty()
        || label.len() > 40
        || !label.as_bytes()[0].is_ascii_alphanumeric()
        || !label
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b" _.-".contains(&b))
        || !matches!(value["category"].as_str(), Some("Personal" | "Work"))
        || value
            .get("window_seconds")
            .is_some_and(|v| v.as_u64() != Some(604800))
    {
        return Err("Invalid allowance account mapping".into());
    }
    Ok(json!({"id":id,"label":label,"category":value["category"],"window_seconds":604800}))
}

pub fn validate_config(config: Option<&Value>) -> Result<()> {
    let Some(config) = config.filter(|v| !v.is_null()) else {
        return Ok(());
    };
    let object = config
        .as_object()
        .ok_or("Invalid allowances configuration")?;
    if object
        .keys()
        .any(|k| !["accounts", "sources"].contains(&k.as_str()))
    {
        return Err("Invalid native allowances configuration".into());
    }
    let accounts = config["accounts"]
        .as_object()
        .ok_or("Invalid allowance accounts")?;
    if accounts.is_empty() || accounts.len() > 4 {
        return Err("Configure one to four allowance accounts".into());
    }
    let mut ids = BTreeSet::new();
    for (key, value) in accounts {
        let item = mapping(value)?;
        if !hash_key(key) || !ids.insert(item["id"].as_str().unwrap().to_owned()) {
            return Err("Invalid allowance account identity".into());
        }
    }
    if let Some(sources) = config.get("sources") {
        let sources = sources.as_array().ok_or("Invalid allowance sources")?;
        if sources.len() > 16 {
            return Err("Too many allowance sources".into());
        }
        for source in sources {
            let fields = source
                .as_object()
                .ok_or("Invalid native allowance source")?;
            if fields.is_empty()
                || fields
                    .keys()
                    .any(|k| !["target", "profile_id"].contains(&k.as_str()))
                || source
                    .get("target")
                    .is_some_and(|v| !v.as_str().is_some_and(valid_target))
                || source
                    .get("profile_id")
                    .is_some_and(|v| !v.as_str().is_some_and(crate::fleet::profile_id))
            {
                return Err("Invalid native allowance source".into());
            }
        }
    }
    Ok(())
}

fn date_valid(value: &str, now: f64) -> bool {
    if value.len() != 10 || !value.is_ascii() || &value[4..5] != "-" || &value[7..8] != "-" {
        return false;
    }
    let (Ok(year), Ok(month), Ok(day)) = (
        value[..4].parse::<i32>(),
        value[5..7].parse::<usize>(),
        value[8..].parse::<u32>(),
    ) else {
        return false;
    };
    if !(1..=9999).contains(&year) || !(1..=12).contains(&month) {
        return false;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    if day == 0 || day > days[month - 1] {
        return false;
    }
    let stamp = now as libc::time_t;
    let mut today = std::mem::MaybeUninit::<libc::tm>::uninit();
    if unsafe { libc::localtime_r(&stamp, today.as_mut_ptr()) }.is_null() {
        return false;
    }
    let today = unsafe { today.assume_init() };
    value
        <= format!(
            "{:04}-{:02}-{:02}",
            today.tm_year + 1900,
            today.tm_mon + 1,
            today.tm_mday
        )
        .as_str()
}

fn daily(raw: &Value, now: f64, source: bool) -> Value {
    let Some(rows) = raw
        .as_array()
        .filter(|v| v.len() <= if source { 366 } else { 30 })
    else {
        return Value::Null;
    };
    let mut result = BTreeMap::new();
    for row in rows {
        let Some(date) = row[if source { "startDate" } else { "date" }]
            .as_str()
            .filter(|s| date_valid(s, now))
        else {
            return Value::Null;
        };
        let Some(tokens) = number(&row["tokens"], SAFE) else {
            return Value::Null;
        };
        if result.insert(date.to_owned(), tokens).is_some() {
            return Value::Null;
        }
    }
    let skip = result.len().saturating_sub(30);
    Value::Array(
        result
            .into_iter()
            .skip(skip)
            .map(|(date, tokens)| json!({"date":date,"tokens":tokens}))
            .collect(),
    )
}

pub fn summarise(raw: &Value, now: f64) -> Option<Value> {
    let key = account_key(raw["accountId"].as_str()?)?;
    let bucket = raw
        .get("rateLimitsByLimitId")
        .and_then(|v| v.get("codex"))
        .unwrap_or(&raw["rateLimits"]);
    if !bucket.is_object()
        || !matches!(bucket.get("limitId"), None | Some(Value::Null))
            && bucket["limitId"] != "codex"
    {
        return None;
    }
    let weekly: Vec<_> = [&bucket["primary"], &bucket["secondary"]]
        .into_iter()
        .filter(|v| v["windowDurationMins"].as_u64() == Some(10080))
        .collect();
    let (remaining, reset) = if weekly.len() == 1 {
        match (
            number(&weekly[0]["usedPercent"], 100),
            number(&weekly[0]["resetsAt"], SAFE),
        ) {
            (Some(used), Some(reset)) if reset as f64 > now => (Some(100 - used), Some(reset)),
            _ => (None, None),
        }
    } else {
        (None, None)
    };
    let credits = &raw["rateLimitResetCredits"];
    let mut count = number(&credits["availableCount"], 10000);
    let mut expiry = None;
    if let Some(rows) = credits["credits"].as_array().filter(|v| v.len() <= 1000) {
        let available: Vec<_> = rows
            .iter()
            .filter(|v| v["status"] == "available" && v["resetType"] == "codexRateLimits")
            .collect();
        let ids: Option<BTreeSet<_>> = available
            .iter()
            .map(|v| v["id"].as_str().filter(|s| !s.is_empty()))
            .collect();
        if count == Some(available.len() as u64)
            && ids.is_some_and(|v| v.len() == available.len())
            && available
                .iter()
                .all(|v| v["expiresAt"].is_null() || number(&v["expiresAt"], SAFE).is_some())
        {
            let times: Vec<_> = available
                .iter()
                .filter_map(|v| number(&v["expiresAt"], SAFE))
                .collect();
            if times.iter().any(|v| *v as f64 <= now) {
                count = None;
            } else {
                expiry = times.into_iter().min();
            }
        }
    }
    Some(
        json!({"account_key":key,"sampled_at":now,"plan":bucket["planType"].as_str().filter(|v|PLANS.contains(v)),"weekly_remaining":remaining,"weekly_resets_at":reset,"reset_count":count,"reset_expires_at":expiry}),
    )
}

pub fn summarise_usage(raw: &Value, now: f64) -> Value {
    json!({"lifetime_tokens":number(&raw["summary"]["lifetimeTokens"],SAFE),"peak_daily_tokens":number(&raw["summary"]["peakDailyTokens"],SAFE),"daily_usage":daily(&raw["dailyUsageBuckets"],now,true)})
}
pub fn sanitise(raw: &Value, now: f64) -> Option<Value> {
    let key = raw["account_key"].as_str().filter(|v| hash_key(v))?;
    let at = raw["sampled_at"].as_f64().filter(|v| {
        v.is_finite() && *v > 0.0 && *v <= SAFE as f64 && now - *v >= -1.0 && now - *v <= 600.0
    })?;
    let mut result = json!({"account_key":key,"sampled_at":at,"plan":raw["plan"].as_str().filter(|v|PLANS.contains(v)),"daily_usage":daily(&raw["daily_usage"],now,false)});
    for (name, max) in [
        ("weekly_remaining", 100),
        ("weekly_resets_at", SAFE),
        ("reset_count", 10000),
        ("reset_expires_at", SAFE),
        ("lifetime_tokens", SAFE),
        ("peak_daily_tokens", SAFE),
    ] {
        result[name] = json!(number(&raw[name], max));
    }
    if result["weekly_resets_at"]
        .as_u64()
        .is_none_or(|v| v as f64 <= now)
    {
        result["weekly_remaining"] = Value::Null;
        result["weekly_resets_at"] = Value::Null;
    }
    if result["reset_expires_at"]
        .as_u64()
        .is_some_and(|v| v as f64 <= now)
    {
        result["reset_count"] = Value::Null;
        result["reset_expires_at"] = Value::Null;
    }
    Some(result)
}

pub fn read_cache(state: &Path, now: f64) -> Vec<Value> {
    common::read_owned(&state.join("allowances.json"), LIMIT, true)
        .ok()
        .and_then(|data| serde_json::from_slice::<Value>(&data).ok())
        .and_then(|v| v.as_array().filter(|r| r.len() <= 4).cloned())
        .unwrap_or_default()
        .iter()
        .filter_map(|v| sanitise(v, now))
        .collect()
}
pub fn receive(config: &Value, state: &Path, row: &Value, owner: Option<&Path>) -> Result<bool> {
    let _guard = owner.map(common::owner_guard).transpose()?;
    let Some(cfg) = configuration(config) else {
        return Ok(false);
    };
    validate_config(Some(cfg))?;
    let now = common::now();
    let Some(row) = sanitise(row, now) else {
        return Ok(false);
    };
    let key = row["account_key"].as_str().unwrap();
    if cfg["accounts"].get(key).is_none() {
        return Ok(false);
    }
    let directory = common::open_directory(state)?;
    if directory
        .metadata()
        .map_err(|_| "Allowance state unavailable")?
        .uid()
        != unsafe { libc::geteuid() }
    {
        return Err("Unsafe allowance state owner".into());
    }
    let lock = common::open_owned(&state.join("allowances.json.lock"), true, true)?;
    let metadata = lock
        .metadata()
        .map_err(|_| "Allowance cache lock unavailable")?;
    if !metadata.is_file()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.permissions().mode() & 0o077 != 0
    {
        return Err("Unsafe allowance cache lock".into());
    }
    if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        return Ok(false);
    }
    let mut rows = read_cache(state, now);
    if rows.iter().any(|v| {
        v["account_key"] == key
            && v["sampled_at"].as_f64().unwrap_or(0.0) >= row["sampled_at"].as_f64().unwrap()
    }) {
        return Ok(false);
    }
    rows.retain(|v| v["account_key"] != key);
    rows.push(row);
    rows.sort_by(|a, b| {
        b["sampled_at"]
            .as_f64()
            .partial_cmp(&a["sampled_at"].as_f64())
            .unwrap()
    });
    rows.truncate(4);
    common::atomic_owned_write(
        &state.join("allowances.json"),
        &serde_json::to_vec(&rows).map_err(|_| "Invalid allowance cache")?,
    )?;
    Ok(true)
}

/// Codex weekly window length. `summarise` selects the window by this
/// duration (10080 minutes), never by field or list order.
const CODEX_WEEK: u64 = 604800;

/// Bounded source-supplied status text: 1 to 80 characters with no control
/// characters, otherwise unknown. Codex supplies none.
pub fn status_text(value: &str) -> Option<String> {
    let count = value.chars().count();
    ((1..=80).contains(&count) && !value.chars().any(char::is_control)).then(|| value.to_owned())
}

/// The one conversion from a sanitised Codex source row to the popover row.
/// Without a current observation the account is unavailable and carries no
/// window, sample time, plan or reset metadata.
fn public_row(account: &Value, source: Option<&Value>) -> Option<AllowanceRow> {
    let unavailable = AllowanceRow {
        provider: "codex".into(),
        provider_label: "Codex".into(),
        account_id: account["id"].as_str()?.to_owned(),
        label: account["label"].as_str()?.to_owned(),
        status: AllowanceStatus::Unavailable,
        status_text: None,
        plan: None,
        sampled_at: None,
        reset_count: None,
        reset_expires_at: None,
        windows: vec![],
    };
    let Some(source) = source else {
        return Some(unavailable);
    };
    Some(AllowanceRow {
        status: AllowanceStatus::Available,
        plan: source["plan"].as_str().map(str::to_owned),
        sampled_at: source["sampled_at"].as_f64(),
        reset_count: source["reset_count"].as_u64(),
        reset_expires_at: source["reset_expires_at"].as_u64(),
        windows: vec![AllowanceWindow {
            kind: "weekly".into(),
            label: "Weekly".into(),
            used_percent: source["weekly_remaining"]
                .as_u64()
                .map(|remaining| 100u64.saturating_sub(remaining) as f64),
            resets_at: source["weekly_resets_at"].as_u64(),
            duration_s: CODEX_WEEK,
            pacing: true,
        }],
        ..unavailable
    })
}

pub fn snapshot(config: &Value, state: &Path, remote: &[Value]) -> Vec<AllowanceRow> {
    snapshot_at(config, state, remote, common::now())
}

fn snapshot_at(config: &Value, state: &Path, remote: &[Value], now: f64) -> Vec<AllowanceRow> {
    let Some(cfg) = configuration(config) else {
        return vec![];
    };
    let Some(accounts) = cfg["accounts"].as_object() else {
        return vec![];
    };
    let mut rows = BTreeMap::<String, Value>::new();
    for raw in read_cache(state, now).iter().chain(remote) {
        if let Some(row) = sanitise(raw, now) {
            let key = row["account_key"].as_str().unwrap().to_owned();
            if accounts.contains_key(&key)
                && rows
                    .get(&key)
                    .is_none_or(|old| old["sampled_at"].as_f64() < row["sampled_at"].as_f64())
            {
                rows.insert(key, row);
            }
        }
    }
    accounts
        .iter()
        .filter_map(|(key, value)| public_row(&mapping(value).ok()?, rows.get(key)))
        .collect()
}

pub fn executable(name: &str) -> PathBuf {
    std::env::var_os("PATH")
        .and_then(|p| {
            std::env::split_paths(&p).map(|p| p.join(name)).find(|p| {
                p.metadata()
                    .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
            })
        })
        .unwrap_or_else(|| common::expand_home(&format!("~/.local/bin/{name}")))
}
pub(crate) fn codex_rpc(
    identity: bool,
    cancel: Option<&AtomicBool>,
) -> Result<BTreeMap<u64, Value>> {
    codex_rpc_with(&executable("codex"), identity, cancel)
}

fn codex_rpc_with(
    binary: &Path,
    identity: bool,
    cancel: Option<&AtomicBool>,
) -> Result<BTreeMap<u64, Value>> {
    if cancel.is_some_and(|value| value.load(Ordering::Relaxed)) {
        return Err("Account read cancelled".into());
    }
    let args = vec![
        binary.to_string_lossy().into_owned(),
        "-c".into(),
        "analytics.enabled=false".into(),
        "-c".into(),
        "mcp_servers={}".into(),
        // This short-lived account reader needs no connector or marketplace
        // startup. These overrides affect only this child, never user config.
        "--disable".into(),
        "plugins".into(),
        "--disable".into(),
        "apps".into(),
        "app-server".into(),
        "--stdio".into(),
    ];
    let mut child = common::spawn_group(&args)?;
    let deadline = Instant::now() + Duration::from_secs(if identity { 8 } else { 5 });
    let mut results = BTreeMap::new();
    let run = (|| -> Result<()> {
        let input = child.stdin.as_mut().ok_or("Codex input unavailable")?;
        let output = child.stdout.as_mut().ok_or("Codex output unavailable")?;
        let mut send = |value: Value| -> Result<()> {
            let mut line = serde_json::to_vec(&value).map_err(|_| "Invalid account request")?;
            line.push(b'\n');
            input
                .write_all(&line)
                .map_err(|_| "Codex account request failed".into())
        };
        send(
            json!({"id":1,"method":"initialize","params":{"clientInfo":{"name":"anton_native","version":"1"},"capabilities":{"experimentalApi":true}}}),
        )?;
        let mut buffer = Vec::new();
        let mut total = 0;
        let mut requested = false;
        let mut usage_requested = false;
        while Instant::now() < deadline && !cancel.is_some_and(|v| v.load(Ordering::Relaxed)) {
            let mut descriptor = libc::pollfd {
                fd: output.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            };
            let ready = unsafe { libc::poll(&mut descriptor, 1, 100) };
            if ready < 0 {
                return Err("Codex account read unavailable".into());
            }
            if ready == 0 {
                continue;
            }
            let mut chunk = [0u8; 16384];
            let count = output
                .read(&mut chunk)
                .map_err(|_| "Codex account read unavailable")?;
            if count == 0 {
                break;
            }
            total += count;
            if total > if identity { 1048576 } else { 262144 } {
                return Err("Codex account response exceeds limits".into());
            }
            buffer.extend_from_slice(&chunk[..count]);
            while let Some(end) = buffer.iter().position(|b| *b == b'\n') {
                let line: Vec<_> = buffer.drain(..=end).collect();
                let message: Value =
                    serde_json::from_slice(&line).map_err(|_| "Invalid Codex account response")?;
                let id = message["id"].as_u64();
                if id == Some(1) && !requested {
                    if message.get("error").is_some() {
                        return Err("Codex account initialisation failed".into());
                    }
                    send(json!({"method":"initialized"}))?;
                    send(json!({"id":2,"method":"account/rateLimits/read"}))?;
                    if identity {
                        send(
                            json!({"id":3,"method":"account/read","params":{"refreshToken":false}}),
                        )?;
                    }
                    requested = true;
                } else if id == Some(2) && requested && !usage_requested {
                    results.insert(2, message.get("result").cloned().unwrap_or(Value::Null));
                    if !identity {
                        send(json!({"id":3,"method":"account/usage/read"}))?;
                    }
                    usage_requested = true;
                } else if id == Some(3) && requested {
                    results.insert(3, message.get("result").cloned().unwrap_or(Value::Null));
                }
                if results.contains_key(&2) && results.contains_key(&3) {
                    return Ok(());
                }
            }
        }
        Ok(())
    })();
    common::terminate_group(&mut child);
    if results.contains_key(&2) {
        Ok(results)
    } else {
        run?;
        Err("Codex allowance unavailable".into())
    }
}

pub fn probe(cancel: Option<&AtomicBool>) -> Result<Value> {
    let replies = codex_rpc(false, cancel)?;
    let now = common::now();
    let mut row = summarise(replies.get(&2).unwrap_or(&Value::Null), now)
        .ok_or("Codex allowance unavailable")?;
    row.as_object_mut().unwrap().extend(
        summarise_usage(replies.get(&3).unwrap_or(&Value::Null), now)
            .as_object()
            .unwrap()
            .clone(),
    );
    Ok(row)
}
pub fn remote(config: &Value, cancel: Option<&AtomicBool>) -> Vec<Value> {
    let Some(cfg) = configuration(config) else {
        return vec![];
    };
    if validate_config(Some(cfg)).is_err() {
        return vec![];
    }
    let Some(sources) = cfg["sources"].as_array() else {
        return vec![];
    };
    let mut rows = Vec::new();
    let mut seen = BTreeSet::new();
    for source in sources.iter().take(16) {
        if cancel.is_some_and(|value| value.load(Ordering::Relaxed)) {
            break;
        }
        let Some(target) = source["target"].as_str().filter(|v| valid_target(v)) else {
            continue;
        };
        if !seen.insert(target) {
            continue;
        }
        let args = vec![
            "ssh".into(),
            "-T".into(),
            "-o".into(),
            "BatchMode=yes".into(),
            "-o".into(),
            "ConnectTimeout=3".into(),
            "--".into(),
            target.into(),
            format!("exec {PEER} --allowances-probe"),
        ];
        if let Ok(data) = common::run_bounded(&args, b"{}", Duration::from_secs(12), LIMIT, cancel)
        {
            rows.extend(peer_rows(&data, common::now()));
        }
    }
    rows
}
/// Accept one peer `--allowances-probe` reply: at most four rows, each
/// sanitised, so extra peer fields never pass.
fn peer_rows(data: &[u8], now: f64) -> Vec<Value> {
    match serde_json::from_slice::<Value>(data) {
        Ok(Value::Array(values)) if values.len() <= 4 => {
            values.iter().filter_map(|v| sanitise(v, now)).collect()
        }
        _ => vec![],
    }
}
pub fn refresh(
    config: &Value,
    state: &Path,
    owner: Option<&Path>,
    cancel: Option<&AtomicBool>,
) -> Result<Value> {
    let cfg = configuration(config).ok_or("No configured allowances")?;
    validate_config(Some(cfg))?;
    let row = probe(cancel)?;
    receive(config, state, &row, owner)?;
    Ok(row)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    #[test]
    fn synthetic_python_oracle_parity() {
        let fixtures: Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/native-allowances-parity.json"
        ))
        .unwrap();
        let now = fixtures["now"].as_f64().unwrap();
        let timestamp = |value: &Value| {
            let mut value = value.clone();
            if let Some(at) = value["sampled_at"].as_f64() {
                value["sampled_at"] = json!(at);
            }
            value
        };
        for case in fixtures["summary"].as_array().unwrap() {
            assert_eq!(
                summarise(&case["raw"], now).unwrap_or(Value::Null),
                timestamp(&case["expected"]),
                "{}",
                case["name"]
            );
        }
        for case in fixtures["sanitise"].as_array().unwrap() {
            // The captured Python oracle rejected even one second of future
            // skew. Native transport now explicitly permits this bounded case.
            if case["name"] == "future" {
                assert!(case["expected"].is_null());
                assert_eq!(
                    sanitise(&case["raw"], now).unwrap()["sampled_at"].as_f64(),
                    case["raw"]["sampled_at"].as_f64()
                );
                continue;
            }
            assert_eq!(
                sanitise(&case["raw"], now).unwrap_or(Value::Null),
                timestamp(&case["expected"]),
                "{}",
                case["name"]
            );
        }
        for case in fixtures["usage"].as_array().unwrap() {
            assert_eq!(
                summarise_usage(&case["raw"], now),
                case["expected"],
                "{}",
                case["name"]
            );
        }
    }

    struct Fixture(std::path::PathBuf);
    impl Fixture {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "anton-allowance-{}-{}-{}",
                std::process::id(),
                common::now().to_bits(),
                {
                    static SEQUENCE: std::sync::atomic::AtomicUsize =
                        std::sync::atomic::AtomicUsize::new(0);
                    SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                }
            ));
            fs::create_dir(&path).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
            Self(path)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    /// Writes an executable fixture from a child process. A process spawned
    /// by a sibling test thread holds a copy of every descriptor open here
    /// until it calls exec, so a script written in this process could still
    /// be open for writing when it runs, and exec fails with ETXTBSY. Writing
    /// to a temporary name and renaming does not help: the inode stays open.
    fn write_executable(path: &Path, script: &str) {
        use std::io::Write;
        let mut writer = std::process::Command::new("/bin/sh")
            .args(["-c", "cat > \"$1\"", "sh"])
            .arg(path)
            .stdin(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let mut input = writer.stdin.take().unwrap();
        input.write_all(script.as_bytes()).unwrap();
        drop(input);
        assert!(writer.wait().unwrap().success());
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    }
    #[test]
    fn executable_fixtures_run_while_sibling_threads_spawn() {
        let fixture = Fixture::new();
        let stop = std::sync::Arc::new(AtomicBool::new(false));
        let storm: Vec<_> = (0..2)
            .map(|_| {
                let stop = stop.clone();
                std::thread::spawn(move || {
                    while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                        let _ = std::process::Command::new("true").status();
                    }
                })
            })
            .collect();
        let mut failures = 0;
        for index in 0..150 {
            let binary = fixture.0.join(format!("codex-{index}"));
            write_executable(&binary, "#!/bin/sh\nexit 0\n");
            match common::spawn_group(&[binary.display().to_string()]) {
                Ok(mut child) => drop(child.wait()),
                Err(_) => failures += 1,
            }
        }
        stop.store(true, std::sync::atomic::Ordering::Relaxed);
        for join in storm {
            join.join().unwrap();
        }
        assert_eq!(failures, 0);
    }

    #[test]
    fn cache_is_mapped_private_monotonic_and_retirement_guarded() {
        let fixture = Fixture::new();
        let key = "a".repeat(64);
        let cfg = json!({"accounts":{key.clone():"Personal"}});
        let owner = fixture.0.join("owner");
        fs::write(&owner, "herdr.observatory\n").unwrap();
        let now = common::now();
        let row = json!({"account_key":key,"sampled_at":now-1.0,"weekly_remaining":0,"weekly_resets_at":now as u64+1000,"reset_count":0});
        assert!(receive(&cfg, &fixture.0, &row, Some(&owner)).unwrap());
        assert!(!receive(&cfg, &fixture.0, &row, Some(&owner)).unwrap());
        assert_eq!(
            fs::metadata(fixture.0.join("allowances.json"))
                .unwrap()
                .mode()
                & 0o777,
            0o600
        );
        let view = snapshot(&cfg, &fixture.0, &[]);
        assert_eq!(view[0].windows[0].used_percent, Some(100.0));
        assert_eq!(view[0].reset_count, Some(0));
        assert!(
            !serde_json::to_string(&view)
                .unwrap()
                .contains(&"a".repeat(64))
        );
        fs::write(&owner, "herdr.observatory:retired\n").unwrap();
        assert!(receive(&cfg, &fixture.0, &row, Some(&owner)).is_err());
        let symlink = fixture.0.join("alias");
        std::os::unix::fs::symlink(&fixture.0, &symlink).unwrap();
        assert!(receive(&cfg, &symlink, &row, None).is_err());
    }

    #[test]
    fn allowance_clock_skew_is_bounded_without_renewing_source_or_expiry() {
        let now = 1800000000.0;
        let mut row = json!({"account_key":"a".repeat(64),"sampled_at":now+0.0472,"weekly_remaining":70,"weekly_resets_at":now as u64+100,"reset_count":2,"reset_expires_at":now as u64+50});
        for offset in [0.0472, 1.0, -600.0] {
            row["sampled_at"] = json!(now + offset);
            let safe = sanitise(&row, now).unwrap();
            assert_eq!(safe["sampled_at"], row["sampled_at"]);
            assert_eq!(safe["weekly_remaining"], 70);
        }
        for offset in [1.001, -600.001] {
            row["sampled_at"] = json!(now + offset);
            assert!(sanitise(&row, now).is_none());
        }
        row["sampled_at"] = json!(now + 0.0472);
        row["weekly_resets_at"] = json!(now as u64);
        row["reset_expires_at"] = json!(now as u64);
        let expired = sanitise(&row, now).unwrap();
        assert!(expired["weekly_remaining"].is_null());
        assert!(expired["reset_count"].is_null());
    }

    #[test]
    fn original_sample_timestamp_survives_json_cache_roundtrip() {
        // The default fast JSON float parser rounded this timestamp down by
        // one ULP, so the identical observation could rewrite the cache.
        let stamp = 1790460000.0000021_f64;
        let row = json!({"sampled_at": stamp});
        let decoded: Value = serde_json::from_slice(&serde_json::to_vec(&row).unwrap()).unwrap();
        assert_eq!(
            decoded["sampled_at"].as_f64().unwrap().to_bits(),
            stamp.to_bits()
        );
    }

    #[test]
    fn account_rpc_is_read_only_and_retains_quota_when_usage_unsupported() {
        let fixture = Fixture::new();
        let binary = fixture.0.join("codex");
        let log = fixture.0.join("requests");
        let args_log = fixture.0.join("arguments");
        let script = format!(
            r#"#!/bin/sh
printf '%s\n' "$@" > '{}'
while IFS= read -r line; do
 printf '%s\n' "$line" >> '{}'
 case "$line" in
 *'"method":"initialize"'*) printf '%s\n' '{{"id":1,"result":{{}}}}';;
 *'"method":"account/rateLimits/read"'*) printf '%s\n' '{{"id":2,"result":{{"accountId":"fixture","rateLimits":{{"primary":{{"windowDurationMins":10080,"usedPercent":20,"resetsAt":9000000000}}}}}}}}';;
 *'"method":"account/usage/read"'*) printf '%s\n' '{{"id":3,"error":{{"message":"PRIVATE ERROR"}}}}';;
 *'"method":"account/read"'*) printf '%s\n' '{{"id":3,"result":{{"account":{{"type":"chatgpt","email":"fixture@example.invalid"}}}}}}';;
 esac
done
"#,
            args_log.display(),
            log.display()
        );
        write_executable(&binary, &script);
        let result = codex_rpc_with(&binary, false, None).unwrap();
        assert_eq!(
            fs::read_to_string(&args_log)
                .unwrap()
                .lines()
                .collect::<Vec<_>>(),
            [
                "-c",
                "analytics.enabled=false",
                "-c",
                "mcp_servers={}",
                "--disable",
                "plugins",
                "--disable",
                "apps",
                "app-server",
                "--stdio"
            ]
        );
        assert_eq!(
            summarise(&result[&2], common::now()).unwrap()["weekly_remaining"],
            80
        );
        assert!(result[&3].is_null());
        let requests: Vec<Value> = fs::read_to_string(&log)
            .unwrap()
            .lines()
            .map(|v| serde_json::from_str(v).unwrap())
            .collect();
        let methods: Vec<_> = requests
            .iter()
            .map(|v| v["method"].as_str().unwrap())
            .collect();
        assert_eq!(
            methods,
            [
                "initialize",
                "initialized",
                "account/rateLimits/read",
                "account/usage/read"
            ]
        );
        fs::write(&log, "").unwrap();
        let replies = codex_rpc_with(&binary, true, None).unwrap();
        assert!(crate::identity::identity(&replies[&3], &replies[&2]).is_some());
        let requests: Vec<Value> = fs::read_to_string(log)
            .unwrap()
            .lines()
            .map(|v| serde_json::from_str(v).unwrap())
            .collect();
        assert_eq!(requests.last().unwrap()["method"], "account/read");
        assert_eq!(requests.last().unwrap()["params"]["refreshToken"], false);
    }

    #[test]
    fn account_rpc_flood_is_bounded_and_cancellation_prevents_spawn() {
        let fixture = Fixture::new();
        let binary = fixture.0.join("codex");
        let pid = fixture.0.join("pid");
        write_executable(
            &binary,
            &format!(
                "#!/bin/sh\nprintf '%s' $$ > '{}'\nhead -c 300000 /dev/zero\nsleep 60\n",
                pid.display()
            ),
        );
        let cancelled = AtomicBool::new(true);
        assert!(codex_rpc_with(&binary, false, Some(&cancelled)).is_err());
        assert!(!pid.exists());
        assert!(codex_rpc_with(&binary, false, None).is_err());
        let pid = fs::read_to_string(pid).unwrap();
        assert!(!Path::new("/proc").join(pid).exists());
    }
    #[test]
    fn weekly_reset_credit_zero_and_missing_are_distinct() {
        let now = 1000.;
        let raw = json!({"accountId":"fixture","rateLimits":{"planType":"pro","primary":{"windowDurationMins":10080,"usedPercent":100,"resetsAt":2000}},"rateLimitResetCredits":{"availableCount":0,"credits":[]}});
        let row = summarise(&raw, now).unwrap();
        assert_eq!(row["weekly_remaining"], 0);
        assert_eq!(row["reset_count"], 0);
        assert!(row["reset_expires_at"].is_null());
        assert!(sanitise(&row, 2001.).is_none());
        let mut row = row;
        row["sampled_at"] = json!(1900);
        let row = sanitise(&row, 2001.).unwrap();
        assert!(row["weekly_remaining"].is_null());
        assert_eq!(row["reset_count"], 0);
    }
    #[test]
    fn complete_credit_details_expire_but_capped_details_do_not_invent_expiry() {
        let mut raw = json!({"accountId":"fixture","rateLimits":{"secondary":{"windowDurationMins":10080,"usedPercent":20,"resetsAt":9000}},"rateLimitResetCredits":{"availableCount":2,"credits":[{"id":"one","status":"available","resetType":"codexRateLimits","expiresAt":900}]}});
        assert_eq!(summarise(&raw, 1000.).unwrap()["reset_count"], 2);
        raw["rateLimitResetCredits"]["availableCount"] = json!(1);
        assert!(summarise(&raw, 1000.).unwrap()["reset_count"].is_null());
    }
    #[test]
    fn wrong_meter_invalid_counters_dates_and_disclosure_fail_closed() {
        assert!(
            summarise(
                &json!({"accountId":"fixture","rateLimits":{"limitId":"other"}}),
                1000.
            )
            .is_none()
        );
        let row = json!({"account_key":"a".repeat(64),"sampled_at":1000,"weekly_remaining":true,"weekly_resets_at":2000,"reset_count":-1,"email":"PRIVATE"});
        let safe = sanitise(&row, 1001.).unwrap();
        assert!(safe["weekly_remaining"].is_null());
        assert!(safe["reset_count"].is_null());
        assert!(!safe.to_string().contains("PRIVATE"));
        assert!(!date_valid("2025-02-29", common::now()));
        assert!(
            daily(
                &json!([{"date":"2024-02-29","tokens":0},{"date":"2024-02-29","tokens":1}]),
                common::now(),
                false
            )
            .is_null()
        );
    }
    #[test]
    fn native_sources_are_explicit_and_without_containers() {
        assert!(
            validate_config(Some(
                &json!({"accounts":{"a".repeat(64):"Personal"},"sources":[{"target":"user@host"}]})
            ))
            .is_ok()
        );
        for source in [
            json!({"target":"-bad"}),
            json!({"target":"host","container":"legacy"}),
        ] {
            assert!(
                validate_config(Some(
                    &json!({"accounts":{"a".repeat(64):"Personal"},"sources":[source]})
                ))
                .is_err()
            );
        }
    }
    const NOW: f64 = 1800000000.0;
    const WEEKLY_KEYS: [&str; 6] = [
        "duration_s",
        "kind",
        "label",
        "pacing",
        "resets_at",
        "used_percent",
    ];
    const ROW_KEYS: [&str; 11] = [
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

    fn one(key: &str) -> Value {
        json!({"accounts":{key:{"id":"synthetic","label":"Synthetic","category":"Personal"}}})
    }
    fn source(key: &str, at: f64, remaining: Value, reset: Value) -> Value {
        json!({"account_key":key,"sampled_at":at,"plan":"pro","weekly_remaining":remaining,
            "weekly_resets_at":reset,"reset_count":1,"reset_expires_at":null})
    }
    /// Converted rows with no private cache, at a fixed clock.
    fn convert(cfg: &Value, remote: &[Value], now: f64) -> Vec<AllowanceRow> {
        let fixture = Fixture::new();
        snapshot_at(cfg, &fixture.0, remote, now)
    }
    fn keys(value: &Value) -> Vec<String> {
        value.as_object().unwrap().keys().cloned().collect()
    }
    fn assert_contract(rows: &[AllowanceRow]) {
        for row in rows {
            let value = serde_json::to_value(row).unwrap();
            assert_eq!(keys(&value), ROW_KEYS);
            for window in value["windows"].as_array().unwrap() {
                assert_eq!(keys(window), WEEKLY_KEYS);
            }
            if row.status != AllowanceStatus::Available {
                assert!(row.windows.is_empty() && row.sampled_at.is_none());
                assert!(row.plan.is_none() && row.reset_count.is_none());
                assert!(row.reset_expires_at.is_none());
            }
        }
    }
    fn weekly(used: Option<f64>, reset: Option<u64>) -> Vec<AllowanceWindow> {
        vec![AllowanceWindow {
            kind: "weekly".into(),
            label: "Weekly".into(),
            used_percent: used,
            resets_at: reset,
            duration_s: 604800,
            pacing: true,
        }]
    }

    #[test]
    fn codex_weekly_allowance_becomes_one_pacing_window() {
        let raw = json!({"accountId":"fixture","rateLimits":{"planType":"pro","primary":
            {"windowDurationMins":10080,"usedPercent":40,"resetsAt":NOW as u64+302400}},
            "rateLimitResetCredits":{"availableCount":1,"credits":[{"id":"one",
            "status":"available","resetType":"codexRateLimits","expiresAt":null}]}});
        let row = summarise(&raw, NOW - 10.25).unwrap();
        let key = row["account_key"].as_str().unwrap().to_owned();
        let rows = convert(&one(&key), &[row], NOW);
        assert_contract(&rows);
        assert_eq!(
            serde_json::to_value(&rows).unwrap(),
            json!([{"provider":"codex","provider_label":"Codex","account_id":"synthetic",
                "label":"Synthetic","status":"available","status_text":null,"plan":"pro",
                "sampled_at":NOW - 10.25,"reset_count":1,"reset_expires_at":null,
                "windows":[{"kind":"weekly","label":"Weekly","used_percent":40.0,
                "resets_at":NOW as u64+302400,"duration_s":604800,"pacing":true}]}])
        );
    }

    #[test]
    fn window_order_does_not_select_the_pacing_window() {
        let raw = json!({"accountId":"fixture","rateLimits":{
            "primary":{"windowDurationMins":300,"usedPercent":90,"resetsAt":NOW as u64+600},
            "secondary":{"windowDurationMins":10080,"usedPercent":35,"resetsAt":NOW as u64+9000}}});
        let row = summarise(&raw, NOW).unwrap();
        let key = row["account_key"].as_str().unwrap().to_owned();
        let rows = convert(&one(&key), &[row], NOW);
        assert_eq!(rows[0].windows, weekly(Some(35.0), Some(NOW as u64 + 9000)));
    }

    #[test]
    fn past_reset_and_pass_expiry_invalidate_only_their_own_fields() {
        // Cases carried over from the deleted `AllowanceRow::expire`.
        let key = "a".repeat(64);
        let cfg = one(&key);
        let raw = json!({"account_key":key,"sampled_at":1000.0,"weekly_remaining":70,
            "weekly_resets_at":1010,"reset_count":2,"reset_expires_at":1020});
        let rows = convert(&cfg, std::slice::from_ref(&raw), 1005.0);
        assert_eq!(rows[0].windows, weekly(Some(30.0), Some(1010)));
        assert_eq!(rows[0].reset_count, Some(2));
        // The reset at the current second invalidates the balance and time.
        let rows = convert(&cfg, std::slice::from_ref(&raw), 1010.0);
        assert_eq!(rows[0].status, AllowanceStatus::Available);
        assert_eq!(rows[0].windows, weekly(None, None));
        assert_eq!(rows[0].reset_count, Some(2));
        assert_eq!(rows[0].reset_expires_at, Some(1020));
        assert_eq!(rows[0].sampled_at, Some(1000.0));
        // Pass expiry nulls the count without a refill or balance change.
        let mut later = raw.clone();
        later["weekly_resets_at"] = json!(5000);
        let rows = convert(&cfg, &[later], 1020.0);
        assert_eq!(rows[0].windows, weekly(Some(30.0), Some(5000)));
        assert_eq!(rows[0].reset_count, None);
        assert_eq!(rows[0].reset_expires_at, None);
        // Source expiry makes the account unavailable.
        let rows = convert(&cfg, &[raw], 1601.0);
        assert_eq!(rows[0].status, AllowanceStatus::Unavailable);
        assert_contract(&rows);
    }

    #[test]
    fn mapped_account_without_current_observation_is_unavailable() {
        let rows = convert(&one(&"a".repeat(64)), &[], NOW);
        assert_contract(&rows);
        assert_eq!(
            serde_json::to_value(&rows).unwrap(),
            json!([{"provider":"codex","provider_label":"Codex","account_id":"synthetic",
                "label":"Synthetic","status":"unavailable","status_text":null,"plan":null,
                "sampled_at":null,"reset_count":null,"reset_expires_at":null,"windows":[]}])
        );
        // An observation for an unmapped account never produces a row.
        let rows = convert(
            &one(&"a".repeat(64)),
            &[source(
                &"b".repeat(64),
                NOW,
                json!(50),
                json!(NOW as u64 + 100),
            )],
            NOW,
        );
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].status, AllowanceStatus::Unavailable);
    }

    #[test]
    fn malformed_or_oversized_values_stay_unknown_and_private_fields_never_pass() {
        let key = "a".repeat(64);
        let reset = NOW as u64 + 1000;
        for used in [json!(101), json!(true), json!(-1), json!(40.5)] {
            let raw = json!({"accountId":"fixture","rateLimits":{"primary":
                {"windowDurationMins":10080,"usedPercent":used,"resetsAt":reset}}});
            assert!(summarise(&raw, NOW).unwrap()["weekly_remaining"].is_null());
        }
        for remaining in [json!(101), json!(true), json!("50")] {
            let mut raw = source(&key, NOW, remaining, json!(reset));
            raw["reset_count"] = json!(-1);
            raw["email"] = json!("PRIVATE");
            raw["theme"] = json!({"name":"PRIVATE"});
            let rows = convert(&one(&key), &[raw], NOW);
            assert_contract(&rows);
            // An unknown balance keeps its valid reset time.
            assert_eq!(rows[0].windows, weekly(None, Some(reset)));
            assert_eq!(rows[0].reset_count, None);
            assert!(!serde_json::to_string(&rows).unwrap().contains("PRIVATE"));
        }
        let mut raw = source(&key, NOW, json!(50), json!(reset));
        raw["reset_count"] = json!(10001);
        assert_eq!(convert(&one(&key), &[raw], NOW)[0].reset_count, None);
        // A cache of more than four rows is rejected whole.
        let fixture = Fixture::new();
        let rows: Vec<_> = (0..5)
            .map(|n| source(&format!("{n}").repeat(64), NOW, json!(50), json!(reset)))
            .collect();
        let cache = fixture.0.join("allowances.json");
        fs::write(&cache, serde_json::to_vec(&rows).unwrap()).unwrap();
        fs::set_permissions(&cache, fs::Permissions::from_mode(0o600)).unwrap();
        let cfg = json!({"accounts":{"0".repeat(64):"Personal"}});
        assert!(read_cache(&fixture.0, NOW).is_empty());
        let view = snapshot_at(&cfg, &fixture.0, &[], NOW);
        assert_eq!(view[0].status, AllowanceStatus::Unavailable);
        fs::write(&cache, serde_json::to_vec(&rows[..4]).unwrap()).unwrap();
        let view = snapshot_at(&cfg, &fixture.0, &[], NOW);
        assert_eq!(view[0].status, AllowanceStatus::Available);
    }

    #[test]
    fn several_accounts_keep_newest_observation_one_row_each_in_account_key_order() {
        let (a, b, c) = ("a".repeat(64), "b".repeat(64), "c".repeat(64));
        let cfg = json!({"accounts":{
            c.clone():{"id":"third","label":"Third","category":"Work"},
            a.clone():"Personal",
            b.clone():"Work"}});
        let reset = json!(NOW as u64 + 1000);
        let fixture = Fixture::new();
        let cache = fixture.0.join("allowances.json");
        fs::write(
            &cache,
            serde_json::to_vec(&json!([
                source(&a, NOW - 100.0, json!(10), reset.clone()),
                source(&b, NOW - 1.0, json!(20), reset.clone())
            ]))
            .unwrap(),
        )
        .unwrap();
        fs::set_permissions(&cache, fs::Permissions::from_mode(0o600)).unwrap();
        let remote = [
            source(&a, NOW - 5.0, json!(30), reset.clone()),
            source(&a, NOW - 50.0, json!(40), reset.clone()),
            source(&b, NOW - 10.0, json!(50), reset.clone()),
        ];
        let rows = snapshot_at(&cfg, &fixture.0, &remote, NOW);
        assert_contract(&rows);
        // The configuration map is ordered by account key.
        let ids: Vec<_> = rows.iter().map(|r| r.account_id.as_str()).collect();
        assert_eq!(ids, ["Personal", "Work", "third"]);
        assert_eq!(rows[0].windows[0].used_percent, Some(70.0));
        assert_eq!(rows[0].sampled_at, Some(NOW - 5.0));
        assert_eq!(rows[1].windows[0].used_percent, Some(80.0));
        assert_eq!(rows[1].sampled_at, Some(NOW - 1.0));
        assert_eq!(rows[2].status, AllowanceStatus::Unavailable);
        assert_eq!(rows[2].label, "Third");
    }

    #[test]
    fn token_activity_stays_off_the_popover_wire() {
        let key = "a".repeat(64);
        let mut raw = source(&key, NOW, json!(50), json!(NOW as u64 + 1000));
        raw["lifetime_tokens"] = json!(123456);
        raw["peak_daily_tokens"] = json!(4567);
        raw["daily_usage"] = json!([{"date":"2020-01-01","tokens":4567}]);
        assert_eq!(sanitise(&raw, NOW).unwrap()["lifetime_tokens"], 123456);
        let rows = convert(&one(&key), &[raw], NOW);
        assert_contract(&rows);
        let text = serde_json::to_string(&rows).unwrap();
        for field in [
            "lifetime_tokens",
            "peak_daily_tokens",
            "daily_usage",
            "4567",
        ] {
            assert!(!text.contains(field), "{field}");
        }
    }

    #[test]
    fn zero_values_stay_distinct_from_unknown() {
        let key = "a".repeat(64);
        let mut raw = source(&key, NOW, json!(0), json!(NOW as u64 + 1000));
        raw["reset_count"] = json!(0);
        let rows = convert(&one(&key), std::slice::from_ref(&raw), NOW);
        assert_eq!(rows[0].windows[0].used_percent, Some(100.0));
        assert_eq!(rows[0].reset_count, Some(0));
        let text = serde_json::to_string(&rows).unwrap();
        assert!(text.contains("\"used_percent\":100.0") && text.contains("\"reset_count\":0"));
        raw["weekly_remaining"] = json!(100);
        let rows = convert(&one(&key), &[raw], NOW);
        assert_eq!(rows[0].windows[0].used_percent, Some(0.0));
    }

    #[test]
    fn stale_or_future_observations_are_unavailable() {
        let key = "a".repeat(64);
        let reset = json!(NOW as u64 + 1000);
        for (offset, status) in [
            (-600.0, AllowanceStatus::Available),
            (1.0, AllowanceStatus::Available),
            (-600.001, AllowanceStatus::Unavailable),
            (1.001, AllowanceStatus::Unavailable),
        ] {
            let raw = source(&key, NOW + offset, json!(50), reset.clone());
            let rows = convert(&one(&key), &[raw], NOW);
            assert_eq!(rows[0].status, status, "{offset}");
            assert_contract(&rows);
        }
    }

    #[test]
    fn status_text_is_bounded_and_printable() {
        assert_eq!(status_text(&"a".repeat(80)), Some("a".repeat(80)));
        assert_eq!(status_text(&"é".repeat(80)), Some("é".repeat(80)));
        assert_eq!(status_text(&"a".repeat(81)), None);
        assert_eq!(status_text(""), None);
        assert_eq!(status_text("Sign in\nagain"), None);
        assert_eq!(status_text("Sign in\u{7f}"), None);
        assert_eq!(status_text("Sign in again"), Some("Sign in again".into()));
    }

    fn legacy() -> Value {
        serde_json::from_str(include_str!(
            "../../../tests/fixtures/native-allowances-legacy.json"
        ))
        .unwrap()
    }
    fn legacy_keys(fixture: &Value) -> Vec<String> {
        fixture["legacy_keys"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_owned())
            .collect()
    }

    #[test]
    fn cache_written_before_the_update_converts_fresh_and_stale_rows() {
        let legacy = legacy();
        let now = legacy["now"].as_f64().unwrap();
        let keys = legacy_keys(&legacy);
        for row in legacy["cache"].as_array().unwrap() {
            assert_eq!(super::tests::keys(row), keys);
        }
        let fixture = Fixture::new();
        let cache = fixture.0.join("allowances.json");
        fs::write(&cache, serde_json::to_vec(&legacy["cache"]).unwrap()).unwrap();
        fs::set_permissions(&cache, fs::Permissions::from_mode(0o600)).unwrap();
        let cfg = json!({"accounts":legacy["accounts"]});
        let rows = snapshot_at(&cfg, &fixture.0, &[], now);
        assert_contract(&rows);
        let reset = now as u64;
        assert_eq!(
            serde_json::to_value(&rows).unwrap(),
            json!([
                {"provider":"codex","provider_label":"Codex","account_id":"synthetic-a",
                 "label":"Synthetic A","status":"available","status_text":null,"plan":"pro",
                 "sampled_at":now - 100.5,"reset_count":1,"reset_expires_at":reset + 172800,
                 "windows":[{"kind":"weekly","label":"Weekly","used_percent":40.0,
                 "resets_at":reset + 86400,"duration_s":604800,"pacing":true}]},
                {"provider":"codex","provider_label":"Codex","account_id":"synthetic-b",
                 "label":"Synthetic B","status":"unavailable","status_text":null,"plan":null,
                 "sampled_at":null,"reset_count":null,"reset_expires_at":null,"windows":[]},
                {"provider":"codex","provider_label":"Codex","account_id":"Personal",
                 "label":"Personal","status":"available","status_text":null,"plan":null,
                 "sampled_at":now - 50.0,"reset_count":0,"reset_expires_at":null,
                 "windows":[{"kind":"weekly","label":"Weekly","used_percent":null,
                 "resets_at":null,"duration_s":604800,"pacing":true}]}
            ])
        );
    }

    #[test]
    fn unchanged_peer_row_converts_through_remote_sanitising() {
        let legacy = legacy();
        let now = legacy["now"].as_f64().unwrap();
        let cfg = json!({"accounts":legacy["accounts"]});
        let mut remote = vec![];
        for output in legacy["peer_outputs"].as_array().unwrap() {
            let data = serde_json::to_vec(output).unwrap();
            let rows = peer_rows(&data, now);
            for row in &rows {
                assert_eq!(keys(row), legacy_keys(&legacy));
            }
            remote.extend(rows);
        }
        let rows = convert(&cfg, &remote, now);
        assert_contract(&rows);
        let reset = now as u64;
        assert_eq!(rows[0].status, AllowanceStatus::Available);
        assert_eq!(rows[0].plan.as_deref(), Some("plus"));
        assert_eq!(rows[0].sampled_at, Some(now - 5.25));
        assert_eq!(rows[0].reset_count, Some(2));
        assert_eq!(rows[0].windows, weekly(Some(25.0), Some(reset + 3600)));
        assert_eq!(rows[1].windows, weekly(Some(60.0), Some(reset + 7200)));
        assert_eq!(rows[1].reset_count, Some(0));
        assert_eq!(rows[1].reset_expires_at, Some(reset + 600));
        assert_eq!(rows[2].status, AllowanceStatus::Unavailable);
        let text = serde_json::to_string(&rows).unwrap();
        for private in [
            "example.invalid",
            "theme",
            "future_field",
            "Synthetic\"",
            "123456",
        ] {
            assert!(!text.contains(private), "{private}");
        }
        // Oversized or non-array peer replies are rejected whole.
        let five = Value::Array(vec![legacy["peer_outputs"][0][0].clone(); 5]);
        assert!(peer_rows(&serde_json::to_vec(&five).unwrap(), now).is_empty());
        assert!(peer_rows(b"{}", now).is_empty());
    }

    #[test]
    fn older_local_runtime_reads_probe_and_cache_rows_unchanged() {
        let legacy = legacy();
        let expected = legacy_keys(&legacy);
        let now = common::now();
        let raw = json!({"accountId":"fixture","rateLimits":{"primary":
            {"windowDurationMins":10080,"usedPercent":40,"resetsAt":now as u64+1000}}});
        let mut row = summarise(&raw, now).unwrap();
        row.as_object_mut().unwrap().extend(
            summarise_usage(&json!({"summary":{"lifetimeTokens":10}}), now)
                .as_object()
                .unwrap()
                .clone(),
        );
        assert_eq!(keys(&row), expected);
        let fixture = Fixture::new();
        let key = row["account_key"].as_str().unwrap().to_owned();
        assert!(receive(&one(&key), &fixture.0, &row, None).unwrap());
        let cache: Value =
            serde_json::from_slice(&fs::read(fixture.0.join("allowances.json")).unwrap()).unwrap();
        assert_eq!(cache.as_array().unwrap().len(), 1);
        assert_eq!(keys(&cache[0]), expected);
        assert_eq!(cache[0]["weekly_remaining"], 60);
    }
}
