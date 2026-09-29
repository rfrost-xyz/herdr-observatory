//! Browser-owned Notion observations. No token, cookie or HTTP access here.
use crate::{Result, common};
use serde_json::{Value, json};
use std::io::Write;
use std::path::Path;
use std::time::{Duration, Instant};

const LIMIT: usize = 4096;
fn uuid(v: &Value) -> bool {
    v.as_str().is_some_and(|s| {
        s.len() == 36
            && s.bytes().enumerate().all(|(i, c)| {
                if [8, 13, 18, 23].contains(&i) {
                    c == b'-'
                } else {
                    c.is_ascii_digit() || (b'a'..=b'f').contains(&c)
                }
            })
    })
}
pub fn validate(config: Option<&Value>) -> Result<()> {
    let Some(c) = config else {
        return Ok(());
    };
    if c.as_object().is_none_or(|o| {
        o.len() != 4
            || o.keys().any(|k| {
                !["user_id", "workspace_id", "extension_id", "label"].contains(&k.as_str())
            })
    }) || !uuid(&c["user_id"])
        || !uuid(&c["workspace_id"])
        || c["extension_id"]
            .as_str()
            .is_none_or(|s| s.len() != 32 || !s.bytes().all(|b| (b'a'..=b'p').contains(&b)))
        || c["label"].as_str().is_none_or(|s| {
            s.is_empty()
                || s.len() > 40
                || !s
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b" _.-".contains(&b))
        })
    {
        return Err("Invalid Notion account mapping".into());
    }
    Ok(())
}
fn key(c: &Value) -> String {
    common::sha256(
        format!(
            "anton-notion-v1:{}:{}",
            c["user_id"].as_str().unwrap_or(""),
            c["workspace_id"].as_str().unwrap_or("")
        )
        .as_bytes(),
    )
}
fn finite(v: &Value) -> Option<f64> {
    v.as_f64().filter(|v| v.is_finite())
}
fn normalise(c: &Value, v: &Value, now: f64) -> Result<Value> {
    if v.as_object().is_none_or(|o| {
        o.len() != 8
            || o.keys().any(|k| {
                ![
                    "version",
                    "user_id",
                    "workspace_id",
                    "sampled_at",
                    "available",
                    "used",
                    "limit",
                    "resets_at",
                ]
                .contains(&k.as_str())
            })
    }) || v["version"] != 1
        || v["user_id"] != c["user_id"]
        || v["workspace_id"] != c["workspace_id"]
    {
        return Err("Notion observation identity mismatch".into());
    }
    let at = finite(&v["sampled_at"])
        .filter(|at| *at <= now + 1.0 && now - *at <= 600.0)
        .ok_or("Invalid Notion source time")?;
    let available = v["available"]
        .as_bool()
        .ok_or("Invalid Notion availability")?;
    let mut out = json!({"account_key":key(c),"sampled_at":at,"available":available,"used_percent":null,"resets_at":null});
    if available {
        let used = finite(&v["used"])
            .filter(|v| *v >= 0.0 && *v <= 9e15)
            .ok_or("Invalid Notion usage")?;
        let limit = finite(&v["limit"])
            .filter(|v| *v > 0.0 && *v <= 9e15)
            .ok_or("Invalid Notion limit")?;
        let ratio = used / limit * 100.0;
        if !ratio.is_finite() || ratio > 1e6 {
            return Err("Invalid Notion ratio".into());
        }
        let reset = v["resets_at"]
            .as_u64()
            .filter(|v| *v as f64 > now && *v as f64 <= now + 32.0 * 86400.0)
            .ok_or("Invalid Notion reset")?;
        out["used_percent"] = json!(ratio);
        out["resets_at"] = json!(reset);
    } else if !v["used"].is_null() || !v["limit"].is_null() || !v["resets_at"].is_null() {
        return Err("Unavailable Notion observation contains balances".into());
    }
    Ok(out)
}
pub fn snapshot(config: &Value, state: &Path, now: f64) -> Option<Value> {
    let c = config.get("notion")?;
    validate(Some(c)).ok()?;
    let cached = common::read_owned(&state.join("notion.json"), LIMIT, true)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok());
    let row = cached.filter(|v| {
        v["account_key"] == key(c)
            && finite(&v["sampled_at"]).is_some_and(|at| at <= now + 1.0 && now - at <= 600.0)
            && v["available"] == true
            && finite(&v["used_percent"]).is_some_and(|v| (0.0..=1e6).contains(&v))
            && v["resets_at"]
                .as_u64()
                .is_some_and(|v| v as f64 > now && v as f64 <= now + 32.0 * 86400.0)
    });
    Some(
        json!({"provider":"notion","provider_label":"Notion","account_id":"notion-monthly","label":c["label"],
        "available":row.is_some(),"window_seconds":0,
        "monthly_used_percent":row.as_ref().map(|v| &v["used_percent"]),
        "monthly_resets_at":row.as_ref().map(|v| &v["resets_at"]),
        "sampled_at":row.as_ref().map(|v| &v["sampled_at"]),
        "plan":null,"weekly_remaining":null,"weekly_resets_at":null,"reset_count":null,"reset_expires_at":null,
        "lifetime_tokens":null,"peak_daily_tokens":null,"daily_usage":null}),
    )
}
fn read_frame() -> Result<Value> {
    let until = Instant::now() + Duration::from_secs(3);
    let mut bytes = Vec::new();
    loop {
        if Instant::now() >= until {
            return Err("Notion bridge input deadline".into());
        }
        let mut fd = libc::pollfd {
            fd: libc::STDIN_FILENO,
            events: libc::POLLIN,
            revents: 0,
        };
        if unsafe { libc::poll(&mut fd, 1, 50) } <= 0 {
            continue;
        }
        let mut chunk = [0u8; LIMIT + 4];
        let n = unsafe { libc::read(libc::STDIN_FILENO, chunk.as_mut_ptr().cast(), chunk.len()) };
        if n <= 0 {
            return Err("Incomplete Notion frame".into());
        }
        bytes.extend_from_slice(&chunk[..n as usize]);
        if bytes.len() >= 4 {
            let len = u32::from_ne_bytes(bytes[..4].try_into().unwrap()) as usize;
            if len == 0 || len > LIMIT || bytes.len() > len + 4 {
                return Err("Invalid Notion frame size".into());
            }
            if bytes.len() == len + 4 {
                return serde_json::from_slice(&bytes[4..])
                    .map_err(|_| "Invalid Notion frame".into());
            }
        }
    }
}
pub fn bridge(root: &Path, state: &Path, origin: &str, config: &Value) -> Result<()> {
    let _guard = common::owner_guard(&root.join(".herdr-observatory-install"))?;
    let c = config.get("notion").ok_or("Notion is not configured")?;
    validate(Some(c))?;
    if origin
        != format!(
            "chrome-extension://{}/",
            c["extension_id"].as_str().unwrap()
        )
    {
        return Err("Unconfigured Notion extension".into());
    }
    let input = read_frame()?;
    let output = if input == json!({"version":1,"operation":"configuration"}) {
        json!({"version":1,"user_id":c["user_id"],"workspace_id":c["workspace_id"]})
    } else {
        let row = normalise(c, &input, common::now())?;
        let lock = common::open_owned(&state.join("notion.lock"), true, true)?;
        use std::os::fd::AsRawFd;
        if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } < 0 {
            return Err("Notion bridge busy".into());
        }
        if let Ok(bytes) = common::read_owned(&state.join("notion.json"), LIMIT, true) {
            if let Ok(old) = serde_json::from_slice::<Value>(&bytes) {
                if old["account_key"] == row["account_key"]
                    && finite(&old["sampled_at"])
                        .is_some_and(|at| at >= row["sampled_at"].as_f64().unwrap())
                {
                    return Err("Older Notion observation".into());
                }
            }
        }
        common::atomic_owned_write(
            &state.join("notion.json"),
            &serde_json::to_vec(&row).map_err(|_| "Invalid Notion output")?,
        )?;
        json!({"ok":true})
    };
    let bytes = serde_json::to_vec(&output).map_err(|_| "Invalid Notion reply")?;
    let mut out = std::io::stdout().lock();
    out.write_all(&(bytes.len() as u32).to_ne_bytes())
        .and_then(|_| out.write_all(&bytes))
        .map_err(|_| "Notion reply unavailable".into())
}
/// Register only Chromium's user-local native host. The extension is loaded by
/// the user in the intended profile; this never edits browser preferences.
pub fn register(root: &Path, config: &Value, config_home: &Path) -> Result<()> {
    use std::os::unix::fs::DirBuilderExt;
    let _guard = common::owner_guard(&root.join(".herdr-observatory-install"))?;
    let c = config.get("notion").ok_or("Notion is not configured")?;
    validate(Some(c))?;
    common::open_directory(&config_home.join("chromium"))?;
    let directory = config_home.join("chromium/NativeMessagingHosts");
    if !directory.exists() {
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&directory)
            .map_err(|_| "Cannot create native host directory")?;
    }
    common::open_directory(&directory)?;
    let path = directory.join("world.herdr.notion_allowance.json");
    let content = json!({"name":"world.herdr.notion_allowance","description":"Anton Notion allowance receiver","path":root.join("anton-runtime"),"type":"stdio","allowed_origins":[format!("chrome-extension://{}/",c["extension_id"].as_str().unwrap())]});
    let receipt_path = root.join(".notion-bridge.json");
    let receipt = json!({"path":path,"manifest":content});
    if path.symlink_metadata().is_ok() || receipt_path.symlink_metadata().is_ok() {
        let old: Value = serde_json::from_slice(&common::read_owned(&receipt_path, LIMIT, true)?)
            .map_err(|_| "Invalid Notion receipt")?;
        if old != receipt {
            return Err("Notion host registration conflict".into());
        }
        if path.symlink_metadata().is_ok() {
            let manifest: Value = serde_json::from_slice(&common::read_owned(&path, LIMIT, true)?)
                .map_err(|_| "Invalid native host")?;
            if manifest != content {
                return Err("Native host was modified".into());
            }
        }
    }
    // Receipt first permits retry after an interrupted registration.
    common::atomic_owned_write(
        &receipt_path,
        &serde_json::to_vec(&receipt).map_err(|_| "Invalid receipt")?,
    )?;
    common::atomic_owned_write(
        &path,
        &serde_json::to_vec(&content).map_err(|_| "Invalid host manifest")?,
    )
}
pub fn unregister(root: &Path, config_home: &Path) -> Result<()> {
    let receipt_path = root.join(".notion-bridge.json");
    if receipt_path.symlink_metadata().is_err() {
        return Ok(());
    }
    let _guard = common::owner_guard(&root.join(".herdr-observatory-install"))?;
    let receipt: Value = serde_json::from_slice(&common::read_owned(&receipt_path, LIMIT, true)?)
        .map_err(|_| "Invalid Notion receipt")?;
    let path = config_home.join("chromium/NativeMessagingHosts/world.herdr.notion_allowance.json");
    if receipt["path"] != json!(path)
        || receipt["manifest"]["path"] != json!(root.join("anton-runtime"))
    {
        return Err("Notion receipt path mismatch".into());
    }
    if path.symlink_metadata().is_ok() {
        let manifest: Value = serde_json::from_slice(&common::read_owned(&path, LIMIT, true)?)
            .map_err(|_| "Invalid native host")?;
        if manifest != receipt["manifest"] {
            return Err("Native host was modified; preserving it".into());
        }
        std::fs::remove_file(path).map_err(|_| "Cannot remove Notion native host")?;
    }
    std::fs::remove_file(receipt_path).map_err(|_| "Cannot remove Notion receipt".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn config() -> Value {
        json!({"user_id":"11111111-1111-1111-1111-111111111111","workspace_id":"22222222-2222-2222-2222-222222222222","extension_id":"a".repeat(32),"label":"Work"})
    }
    fn observation() -> Value {
        let c = config();
        json!({"version":1,"user_id":c["user_id"],"workspace_id":c["workspace_id"],"sampled_at":1000,"available":true,"used":20,"limit":200,"resets_at":2000})
    }
    #[test]
    fn ratio_zero_overage_and_unknown_are_distinct() {
        let c = config();
        let mut v = observation();
        assert_eq!(normalise(&c, &v, 1000.).unwrap()["used_percent"], 10.);
        v["used"] = json!(0);
        assert_eq!(normalise(&c, &v, 1000.).unwrap()["used_percent"], 0.);
        v["used"] = json!(250);
        assert_eq!(normalise(&c, &v, 1000.).unwrap()["used_percent"], 125.);
        v["used"] = Value::Null;
        assert!(normalise(&c, &v, 1000.).is_err());
    }
    #[test]
    fn reject_wrong_identity_expiry_and_malformed() {
        for (k, val) in [
            ("user_id", json!("other")),
            ("limit", json!(0)),
            ("used", json!(-1)),
            ("sampled_at", json!(399)),
            ("sampled_at", json!(1002)),
            ("resets_at", json!(1000)),
            ("resets_at", json!(9_000_000)),
        ] {
            let mut v = observation();
            v[k] = val;
            assert!(normalise(&config(), &v, 1000.).is_err(), "{k}");
        }
        let mut v = observation();
        v["cookie"] = json!("secret");
        assert!(normalise(&config(), &v, 1000.).is_err());
    }
    #[test]
    fn unavailable_cannot_carry_balances() {
        let mut v = observation();
        v["available"] = json!(false);
        assert!(normalise(&config(), &v, 1000.).is_err());
        for k in ["used", "limit", "resets_at"] {
            v[k] = Value::Null;
        }
        assert_eq!(normalise(&config(), &v, 1000.).unwrap()["available"], false);
    }
    #[test]
    fn persisted_snapshot_expires_and_rebinds_without_inventing_zero() {
        let state = std::env::temp_dir().join(format!(
            "notion-snapshot-{}-{}",
            std::process::id(),
            common::now()
        ));
        std::fs::create_dir(&state).unwrap();
        let c = config();
        let config = json!({"notion":c});
        assert_eq!(
            snapshot(&config, &state, 1000.).unwrap()["available"],
            false
        );
        let row = normalise(&c, &observation(), 1000.).unwrap();
        common::atomic_owned_write(
            &state.join("notion.json"),
            &serde_json::to_vec(&row).unwrap(),
        )
        .unwrap();
        let fresh = snapshot(&config, &state, 1000.).unwrap();
        assert_eq!(fresh["monthly_used_percent"], 10.);
        assert_eq!(fresh["sampled_at"], 1000.);
        for now in [998., 1601., 2000.] {
            let expired = snapshot(&config, &state, now).unwrap();
            assert_eq!(expired["available"], false);
            assert!(expired["monthly_used_percent"].is_null());
        }
        let mut replacement = config.clone();
        replacement["notion"]["workspace_id"] = json!("33333333-3333-3333-3333-333333333333");
        assert_eq!(
            snapshot(&replacement, &state, 1000.).unwrap()["available"],
            false
        );
        std::fs::remove_dir_all(state).unwrap();
    }
    #[test]
    fn mapping_is_strict() {
        assert!(validate(Some(&config())).is_ok());
        let mut c = config();
        c["extension_id"] = json!("../bad");
        assert!(validate(Some(&c)).is_err());
        assert!(validate(Some(&Value::Null)).is_err());
    }
}
