//! Bounded Pi presentation reporting. Codex metrics are collector-owned.
use crate::{Result, common, telemetry};
use serde_json::{Value, json};
use std::fs::OpenOptions;
use std::os::fd::AsRawFd;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::time::{Duration, Instant};

fn valid_pane(pane: &str) -> bool {
    !pane.is_empty()
        && pane.len() <= 80
        && pane
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b":_-".contains(&c))
}

pub fn report(
    root: &Path,
    state: &Path,
    harness: &str,
    pane: &str,
    seq: u64,
    mut raw: Value,
) -> Result<bool> {
    if harness != "pi" || !valid_pane(pane) || seq > 9_007_199_254_740_991 || !raw.is_object() {
        return Ok(false);
    }
    let _owner = common::owner_guard(&root.join(".herdr-observatory-install"))?;
    let config: Value = serde_json::from_slice(&common::read_owned(
        &root.join(".config.json"),
        1_048_576,
        true,
    )?)
    .map_err(|_| "Invalid configuration".to_string())?;
    let hosts: Vec<_> = config["hosts"]
        .as_array()
        .ok_or("Missing hosts")?
        .iter()
        .filter(|host| host["transport"].as_str().unwrap_or("local") == "local")
        .collect();
    if hosts.len() != 1 {
        return Ok(false);
    }
    let socket = Path::new(
        hosts[0]["socket_path"]
            .as_str()
            .ok_or("Missing local socket")?,
    );
    let session = raw["session_path"].as_str().map(str::to_owned);
    raw["seq"] = json!(seq);
    if let Some(tool) = raw["tool"].as_str() {
        let allowed = [
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
        ];
        if !allowed.contains(&tool) {
            raw["tool"] = json!(if tool.starts_with("mcp") {
                "mcp-tool"
            } else {
                "custom-tool"
            });
        }
    }
    let Some(event) = telemetry::telemetry_view(&raw) else {
        return Ok(false);
    };
    // State creation is guarded by the installation lease, including late hooks.
    use std::os::unix::fs::DirBuilderExt;
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(state)
        .map_err(|_| "State unavailable")?;
    common::open_directory(state)?;
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(state.join("hook.lock"))
        .map_err(|_| "Hook lock unavailable")?;
    use std::os::unix::fs::MetadataExt;
    let info = lock.metadata().map_err(|_| "Hook lock unavailable")?;
    if !info.is_file() || info.uid() != unsafe { libc::getuid() } || info.mode() & 0o077 != 0 {
        return Err("Unsafe hook lock".into());
    }
    let deadline = Instant::now() + Duration::from_millis(400);
    while unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        if Instant::now() >= deadline {
            return Ok(false);
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let _unlock = common::Unlock(&lock);
    let response = common::rpc(
        socket,
        "pane.get",
        json!({"pane_id":pane}),
        Duration::from_millis(400),
        1_048_576,
    )?;
    let agent = &response["pane"];
    if agent["agent"].as_str() != Some("pi")
        || agent["agent_session"]["kind"].as_str() != Some("path")
        || session.as_deref() != agent["agent_session"]["value"].as_str()
        || session.is_none()
    {
        return Ok(false);
    }
    let Some(binding) = telemetry::session_binding(agent) else {
        return Ok(false);
    };
    if agent["tokens"]["obs_seq"]
        .as_str()
        .and_then(|s| s.parse::<u64>().ok())
        .is_some_and(|old| old >= seq)
    {
        return Ok(false);
    }
    let params = metadata(&event, pane, seq, &binding);
    common::rpc(
        socket,
        "pane.report_metadata",
        params,
        Duration::from_millis(400),
        1_048_576,
    )?;
    Ok(true)
}

fn metadata(event: &Value, pane: &str, seq: u64, binding: &str) -> Value {
    let mut tokens = serde_json::Map::new();
    for key in [
        "v",
        "bind",
        "seq",
        "event",
        "phase",
        "tool",
        "model",
        "result",
        "usage_source",
    ] {
        let value = match key {
            "v" => json!("2"),
            "bind" => json!(binding),
            _ => event
                .get(key)
                .filter(|v| !v.is_null())
                .map(|v| match v {
                    Value::String(s) => json!(s),
                    _ => json!(v.to_string()),
                })
                .unwrap_or(Value::Null),
        };
        tokens.insert(format!("obs_{key}"), value);
    }
    for (index, fields) in telemetry::GROUPS.iter().enumerate() {
        let value = fields
            .iter()
            .map(|key| {
                event
                    .get(*key)
                    .filter(|v| !v.is_null())
                    .map(Value::to_string)
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>()
            .join(",");
        tokens.insert(format!("obs_n{index}"), json!(value));
    }
    // Pi does not expose a general child lifecycle. Explicit nulls revoke old fields.
    for key in ["children", "completion", "outcomes"] {
        tokens.insert(format!("obs_{key}"), Value::Null);
    }
    let phase = event["phase"].as_str().unwrap_or("unknown");
    let mut label = format!("pi · {phase}");
    if let Some(tool) = event["tool"].as_str() {
        label.push_str(&format!(" · {tool}"));
    }
    let label: String = label.chars().take(80).collect();
    json!({"pane_id":pane,"source":"user:observatory","agent":"pi","seq":seq,"tokens":tokens,"display_agent":label})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn report_wire_is_atomic_and_bounded() {
        let event = json!({"seq":123,"phase":"working","event":"turn","total_input":0,"input":9_007_199_254_740_991u64});
        let value = metadata(&event, "pane", 123, "binding");
        let tokens = value["tokens"].as_object().unwrap();
        assert_eq!(tokens.len(), 16);
        assert!(
            tokens
                .values()
                .all(|v| v.is_null() || v.as_str().is_some_and(|s| s.len() <= 80))
        );
        assert!(
            tokens
                .values()
                .any(|v| v.as_str().is_some_and(|s| s.contains('0')))
        );
        assert_eq!(tokens["obs_children"], Value::Null);
    }
    #[test]
    fn unsupported_harness_and_invalid_identity_do_not_touch_files() {
        assert!(
            !report(
                Path::new("/absent"),
                Path::new("/absent"),
                "codex",
                "pane",
                1,
                json!({})
            )
            .unwrap()
        );
        assert!(!valid_pane("wrong/path"));
        assert!(valid_pane("p:1_ab-c"));
    }
    /// The hook lock is released before it is closed, so a zero-wait lock
    /// taken straight after `report` never finds it held by a child that a
    /// sibling thread spawned in between.
    #[test]
    fn hook_lock_is_free_at_once_after_report_while_siblings_spawn() {
        use std::sync::atomic::{AtomicBool, Ordering};
        let base = std::env::temp_dir().join(format!(
            "anton-report-lock-{}-{}",
            std::process::id(),
            common::now().to_bits()
        ));
        std::fs::create_dir(&base).unwrap();
        common::atomic_owned_write(
            &base.join(".herdr-observatory-install"),
            b"herdr.observatory\n",
        )
        .unwrap();
        // No listener: the lock is taken, then `pane.get` fails at once.
        let socket = base.join("absent.sock");
        common::atomic_owned_write(
            &base.join(".config.json"),
            &serde_json::to_vec(&json!({"hosts":[{"id":"fixture","socket_path":socket}]})).unwrap(),
        )
        .unwrap();
        let stop = std::sync::Arc::new(AtomicBool::new(false));
        let storm: Vec<_> = (0..2)
            .map(|_| {
                let stop = stop.clone();
                std::thread::spawn(move || {
                    while !stop.load(Ordering::Relaxed) {
                        let _ = std::process::Command::new("true").status();
                    }
                })
            })
            .collect();
        let state = base.join("state");
        let raw = json!({"session_path":"/synthetic/session","event":"turn","phase":"working","usage_seq":1,"usage_source":"pi-extension"});
        let mut held = 0;
        for seq in 1..=300 {
            assert!(report(&base, &state, "pi", "p:1", seq, raw.clone()).is_err());
            let lock = OpenOptions::new()
                .read(true)
                .write(true)
                .open(state.join("hook.lock"))
                .unwrap();
            if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
                let _unlock = common::Unlock(&lock);
            } else {
                held += 1;
            }
        }
        stop.store(true, Ordering::Relaxed);
        for join in storm {
            join.join().unwrap();
        }
        std::fs::remove_dir_all(base).unwrap();
        assert_eq!(held, 0);
    }
    #[test]
    fn pi_report_is_bound_read_only_then_one_metadata_write() {
        use std::io::{BufRead, BufReader, Write};
        use std::os::unix::net::UnixListener;
        let base = std::env::temp_dir().join(format!(
            "anton-report-{}-{}-{}",
            std::process::id(),
            common::now().to_bits(),
            {
                static SEQUENCE: std::sync::atomic::AtomicUsize =
                    std::sync::atomic::AtomicUsize::new(0);
                SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            }
        ));
        std::fs::create_dir(&base).unwrap();
        let path = base.join("herdr.sock");
        let listener = UnixListener::bind(&path).unwrap();
        listener.set_nonblocking(true).unwrap();
        common::atomic_owned_write(
            &base.join(".herdr-observatory-install"),
            b"herdr.observatory\n",
        )
        .unwrap();
        common::atomic_owned_write(
            &base.join(".config.json"),
            &serde_json::to_vec(&json!({"hosts":[{"id":"fixture","socket_path":path}]})).unwrap(),
        )
        .unwrap();
        let server = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(2);
            let mut methods = Vec::new();
            while methods.len() < 2 && Instant::now() < deadline {
                let Ok((mut socket, _)) = listener.accept() else {
                    std::thread::sleep(Duration::from_millis(5));
                    continue;
                };
                socket
                    .set_read_timeout(Some(Duration::from_millis(500)))
                    .unwrap();
                let mut line = String::new();
                BufReader::new(&socket).read_line(&mut line).unwrap();
                let request: Value = serde_json::from_str(&line).unwrap();
                let method = request["method"].as_str().unwrap().to_owned();
                let response = if method == "pane.get" {
                    json!({"pane":{"agent":"pi","agent_session":{"agent":"pi","source":"herdr:pi","kind":"path","value":"/synthetic/session"},"tokens":{}}})
                } else {
                    assert_eq!(method, "pane.report_metadata");
                    assert_eq!(request["params"]["tokens"]["obs_tool"], "mcp-tool");
                    assert!(!request["params"].to_string().contains("PRIVATE"));
                    assert_eq!(request["params"]["tokens"].as_object().unwrap().len(), 16);
                    json!({})
                };
                methods.push(method);
                writeln!(socket, "{}", json!({"id":request["id"],"result":response})).unwrap();
            }
            methods
        });
        let raw = json!({"session_path":"/synthetic/session","event":"tool-start","phase":"tool","tool":"mcp_PRIVATE","input":0,"usage_seq":1,"usage_source":"pi-extension","content":"PRIVATE"});
        assert!(report(&base, &base.join("state"), "pi", "p:1", 1, raw).unwrap());
        assert_eq!(
            server.join().unwrap(),
            vec!["pane.get", "pane.report_metadata"]
        );
        std::fs::remove_dir_all(base).unwrap();
    }
}
