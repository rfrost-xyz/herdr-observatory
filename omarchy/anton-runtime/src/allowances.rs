//! Account-bound read-only Codex quotas. Account identifiers never enter display rows.
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

pub fn snapshot(config: &Value, state: &Path, remote: &[Value]) -> Vec<Value> {
    let Some(cfg) = configuration(config) else {
        return vec![];
    };
    let Some(accounts) = cfg["accounts"].as_object() else {
        return vec![];
    };
    let now = common::now();
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
    accounts.iter().filter_map(|(key,value)|{let account=mapping(value).ok()?;let row=rows.get(key);let mut public=json!({"label":account["label"],"account_id":account["id"],"provider":"codex","provider_label":"Codex","window_seconds":604800,"available":row.is_some()});for field in ["plan","weekly_remaining","weekly_resets_at","reset_count","reset_expires_at","sampled_at","lifetime_tokens","peak_daily_tokens","daily_usage"]{public[field]=row.map(|r|r[field].clone()).unwrap_or(Value::Null);}Some(public)}).collect()
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
            if let Ok(Value::Array(values)) = serde_json::from_slice::<Value>(&data) {
                if values.len() <= 4 {
                    rows.extend(values.iter().filter_map(|v| sanitise(v, common::now())));
                }
            }
        }
    }
    rows
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
                "anton-allowance-{}-{}",
                std::process::id(),
                common::now().to_bits()
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
        assert_eq!(view[0]["weekly_remaining"], 0);
        assert_eq!(view[0]["reset_count"], 0);
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
        fs::write(&binary, script).unwrap();
        fs::set_permissions(&binary, fs::Permissions::from_mode(0o700)).unwrap();
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
        fs::write(
            &binary,
            format!(
                "#!/bin/sh\nprintf '%s' $$ > '{}'\nhead -c 300000 /dev/zero\nsleep 60\n",
                pid.display()
            ),
        )
        .unwrap();
        fs::set_permissions(&binary, fs::Permissions::from_mode(0o700)).unwrap();
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
}
