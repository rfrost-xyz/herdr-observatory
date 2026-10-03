use crate::{
    Result,
    common::{self, number},
    native::NativeTelemetry,
    telemetry,
};
use serde_json::{Value, json};
use std::path::{Component, Path};
use std::sync::atomic::AtomicBool;
use std::time::Duration;
pub const PEER_COMMAND: &str = "if [ -f \"$HOME/.local/share/herdr.observatory-peer/anton-runtime\" ] && [ -x \"$HOME/.local/share/herdr.observatory-peer/anton-runtime\" ]; then exec \"$HOME/.local/share/herdr.observatory-peer/anton-runtime\" --probe; else cat > /dev/null; printf '%s\\n' '{\"anton_peer\":\"missing\"}'; fi";
pub fn clean(value: &Value, fallback: &str) -> String {
    value
        .as_str()
        .unwrap_or(fallback)
        .chars()
        .filter(|c| !(*c <= '\u{1f}' || *c == '\u{7f}'))
        .take(160)
        .collect()
}
fn checkout(value: &Value) -> String {
    let Some(path) = value.as_str().filter(|v| {
        v.starts_with('/') && !Path::new(v).components().any(|c| c == Component::ParentDir)
    }) else {
        return String::new();
    };
    path.trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("")
        .chars()
        .filter(|c| !c.is_control())
        .take(80)
        .collect()
}
fn category(path: &Value, host: &Value) -> &'static str {
    let Some(path) = path.as_str().filter(|p| {
        p.starts_with('/') && !Path::new(p).components().any(|c| c == Component::ParentDir)
    }) else {
        return "personal";
    };
    for (name, key) in [("personal", "personal_roots"), ("work", "work_roots")] {
        if host[key].as_array().is_some_and(|roots| {
            roots
                .iter()
                .filter_map(Value::as_str)
                .any(|root| Path::new(path).starts_with(root))
        }) {
            return name;
        }
    }
    "personal"
}
pub fn herdr_binary(binary: &str) -> String {
    if binary == "herdr"
        && !std::env::var_os("PATH").is_some_and(|paths| {
            std::env::split_paths(&paths).any(|path| path.join("herdr").is_file())
        })
    {
        common::expand_home("~/.local/bin/herdr")
            .to_string_lossy()
            .into_owned()
    } else {
        common::expand_home(binary).to_string_lossy().into_owned()
    }
}
/// The overall deadline of the Herdr snapshot call. It also bounds how long
/// after `sampled_at` a sample's own values can be stamped.
pub const SNAPSHOT_TIMEOUT: Duration = Duration::from_secs(6);
fn snapshot(host: &Value, cancel: Option<&AtomicBool>) -> Result<Value> {
    if let Some(path) = host["socket_path"].as_str() {
        return common::rpc_with_cancel(
            &common::expand_home(path),
            "session.snapshot",
            json!({}),
            SNAPSHOT_TIMEOUT,
            4 * 1024 * 1024,
            cancel,
        )?
        .get("snapshot")
        .cloned()
        .ok_or("Missing Herdr snapshot".into());
    }
    let resolved = herdr_binary(host["herdr"].as_str().unwrap_or("herdr"));
    let mut command = vec![resolved];
    if let Some(session) = host["session"].as_str() {
        command.extend(["--session".into(), session.into()]);
    }
    command.extend(["api".into(), "snapshot".into()]);
    let bytes = common::run_bounded(&command, &[], SNAPSHOT_TIMEOUT, 4 * 1024 * 1024, cancel)?;
    let value: Value = serde_json::from_slice(&bytes).map_err(|_| "Invalid Herdr response")?;
    value
        .pointer("/result/snapshot")
        .cloned()
        .ok_or("Missing Herdr snapshot".into())
}
pub fn normalise(raw: &Value, host: &Value) -> Result<Vec<Value>> {
    let agents = raw["agents"].as_array().ok_or("Invalid agents")?;
    let workspaces = raw["workspaces"].as_array().ok_or("Invalid workspaces")?;
    let id = host["id"].as_str().ok_or("Invalid host")?;
    let mut result = Vec::new();
    for entry in agents {
        let pane = entry["pane_id"].as_str().ok_or("Invalid pane")?;
        let workspace = workspaces
            .iter()
            .find(|w| w["workspace_id"].is_string() && w["workspace_id"] == entry["workspace_id"]);
        let label = workspace
            .map(|w| clean(&w["label"], "Untitled"))
            .unwrap_or_else(|| "Untitled".into());
        let path = workspace
            .map(|w| &w["worktree"]["checkout_path"])
            .unwrap_or(&Value::Null);
        let class = category(&entry["cwd"], host);
        let leaf = if category(path, host) == class {
            checkout(path)
        } else {
            String::new()
        };
        let leaf = if leaf.is_empty() {
            checkout(&entry["cwd"])
        } else {
            leaf
        };
        let leaf = if [".", ".."].contains(&leaf.as_str()) || leaf.contains(['/', '\\']) {
            String::new()
        } else {
            leaf
        };
        let mut technical = json!({"revision":number(&entry["revision"]),"state_change_seq":number(&entry["state_change_seq"]),"focused":entry["focused"].as_bool(),"interactive_ready":entry["interactive_ready"].as_bool(),"launch_pending":entry["launch_pending"].as_bool(),"turn_timing":telemetry::turn_timing_view(&entry["_native_turn_timing"]),"telemetry":entry.get("_native_telemetry").cloned().or_else(||telemetry::telemetry_from_agent(entry))});
        if let Some(binding) = telemetry::session_binding(entry) {
            technical["session_generation"] =
                json!(u64::from_str_radix(&binding[..13], 16).unwrap());
        }
        let status = entry["agent_status"]
            .as_str()
            .filter(|s| ["working", "blocked", "done", "idle", "unknown"].contains(s))
            .unwrap_or("unknown");
        result.push(json!({"id":format!("{id}:{}",clean(&json!(pane),"")),"host":id,"category":class,"technical":technical,"checkout":leaf,"project":label,"harness":clean(&entry["agent"],"unknown"),"status":status,"title":clean(&entry["terminal_title_stripped"],"No task title reported")}));
    }
    Ok(result)
}
/// Presentation theme is no longer collected. `theme` stays in the sample as an
/// explicit `null` because pre-change local runtimes deserialise a required
/// `theme` field from peer probe results. Remove it once none can remain.
pub fn local(
    host: &Value,
    cursors: &Value,
    follower: &mut NativeTelemetry,
    cancel: Option<&AtomicBool>,
) -> Result<Value> {
    // `sampled_at` is stamped before the Herdr call, so a sample that was already
    // in flight when an owner refresh arrived can never satisfy that refresh.
    let time = common::now();
    match snapshot(host, cancel) {
        Ok(mut raw) => {
            raw["workspaces"]
                .as_array()
                .ok_or("Invalid workspace snapshot")?;
            let updated = follower.enrich(
                raw["agents"]
                    .as_array_mut()
                    .ok_or("Invalid agent snapshot")?,
                cursors,
            );
            let agents = normalise(&raw, host)?;
            Ok(
                json!({"agents":agents,"theme":null,"sampled_at":time,"error":null,"protocol":number(&raw["protocol"]),"version":clean(&raw["version"],"unknown"),"cursors":updated}),
            )
        }
        Err(_) => Ok(
            json!({"agents":[],"theme":null,"sampled_at":time,"error":"Herdr unavailable or incompatible","protocol":null,"version":"unknown","cursors":cursors}),
        ),
    }
}
pub fn remote(host: &Value, cursors: &Value, cancel: Option<&AtomicBool>) -> Result<Value> {
    let target = host["target"].as_str().ok_or("Missing target")?;
    let mut command = vec!["ssh".into()];
    if let Ok(path) = std::env::var("OBSERVATORY_SSH_CONFIG") {
        command.extend(["-F".into(), path]);
    }
    command.extend(
        [
            "-T",
            "-o",
            "BatchMode=yes",
            "-o",
            "ConnectTimeout=5",
            "-o",
            "ServerAliveInterval=5",
            "-o",
            "ServerAliveCountMax=1",
            "--",
            target,
            PEER_COMMAND,
        ]
        .map(str::to_owned),
    );
    let mut request = json!({"version":1,"host_id":host["id"],"cursors":cursors});
    if let Some(session) = host.get("session") {
        request["session"] = session.clone();
    }
    let request = serde_json::to_vec(&request).map_err(|_| "Invalid peer request")?;
    let bytes = common::run_bounded(
        &command,
        &request,
        Duration::from_secs(15),
        4 * 1024 * 1024,
        cancel,
    )?;
    let response: Value = serde_json::from_slice(&bytes).map_err(|_| "Invalid peer response")?;
    if response == json!({"anton_peer":"missing"}) {
        return Err("setup_needed".into());
    }
    if host
        .get("session")
        .is_some_and(|session| response["session"] != *session)
    {
        return Err("Peer session mismatch".into());
    }
    if response["version"] != 1 || response["host_id"] != host["id"] || response["ok"] != true {
        return Err("Peer protocol mismatch".into());
    }
    response
        .get("result")
        .cloned()
        .ok_or("Missing peer sample".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_peer_drains_large_cursor_request_before_returning_status() {
        let home = std::env::temp_dir().join(format!(
            "anton-missing-peer-{}-{}-{}",
            std::process::id(),
            common::now().to_bits(),
            {
                static SEQUENCE: std::sync::atomic::AtomicUsize =
                    std::sync::atomic::AtomicUsize::new(0);
                SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            }
        ));
        std::fs::create_dir(&home).unwrap();
        let command = vec![
            "/usr/bin/env".into(),
            format!("HOME={}", home.display()),
            "/bin/sh".into(),
            "-c".into(),
            PEER_COMMAND.into(),
        ];
        let results:Vec<_>=[64*1024,128*1024].into_iter().map(|size| {
            let request=serde_json::to_vec(&json!({"version":1,"host_id":"fixture","cursors":{"fixture":"x".repeat(size)}})).unwrap();
            common::run_bounded(&command,&request,Duration::from_secs(2),4096,None)
        }).collect();
        std::fs::remove_dir(&home).unwrap();
        for result in results {
            let response: Value = serde_json::from_slice(
                &result.expect("large request must be consumed before peer status exits"),
            )
            .unwrap();
            assert_eq!(response, json!({"anton_peer":"missing"}));
        }
    }
}
