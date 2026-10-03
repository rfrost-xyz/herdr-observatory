//! Explicit configuration migration and receipt-bound peer removal.
use crate::{Result, common, config, hooks_install, native};
use serde_json::{Value, json};
use std::path::Path;
use std::time::Duration;

const PEER_FILES: &[&str] = &[
    "anton-runtime",
    "uninstall.sh",
    ".config.json",
    ".herdr-observatory-install",
    ".peer-receipt.json",
    ".hooks-receipt.json",
    ".hooks-before-native.json",
];

pub fn migrate(mut value: Value) -> Result<Value> {
    let obj = value.as_object_mut().ok_or("Invalid configuration")?;
    obj.remove("publish");
    obj.remove("music");
    if let Some(hosts) = obj.get_mut("hosts").and_then(Value::as_array_mut) {
        for host in hosts {
            if let Some(host) = host.as_object_mut() {
                for name in [
                    "gpu_path",
                    "gpu_api",
                    "gpu_state_port",
                    "disk_path",
                    "metrics_path",
                    "metrics_api",
                ] {
                    host.remove(name);
                }
            }
        }
    }
    if let Some(allowances) = obj.get_mut("allowances").and_then(Value::as_object_mut) {
        allowances.remove("publish");
    }
    if let Some(sources) = obj
        .get_mut("allowances")
        .and_then(Value::as_object_mut)
        .and_then(|a| a.get_mut("sources"))
        .and_then(Value::as_array_mut)
    {
        for source in sources {
            if let Some(source) = source.as_object_mut() {
                source.remove("container");
            }
        }
    }
    config::validate(value)
}

pub fn record_peer(root: &Path) -> Result<()> {
    let home = std::env::var_os("HOME").ok_or("Home unavailable")?;
    if root != Path::new(&home).join(".local/share/herdr.observatory-peer") {
        return Err("Unexpected peer directory".into());
    }
    let _owner = common::owner_guard(&root.join(".herdr-observatory-install"))?;
    config::validate(
        serde_json::from_slice(&common::read_owned(
            &root.join(".config.json"),
            1_048_576,
            true,
        )?)
        .map_err(|_| "Invalid peer configuration")?,
    )?;
    for entry in std::fs::read_dir(root).map_err(|_| "Peer directory unavailable")? {
        let entry = entry.map_err(|_| "Peer entry unavailable")?;
        if !entry
            .file_type()
            .map_err(|_| "Peer entry unavailable")?
            .is_file()
            || !PEER_FILES.iter().any(|name| entry.file_name() == *name)
        {
            return Err("Unknown peer file".into());
        }
    }
    common::atomic_owned_write(
        &root.join(".peer-receipt.json"),
        &serde_json::to_vec(
            &json!({"version":1,"role":"herdr.observatory-peer","files":PEER_FILES}),
        )
        .map_err(|_| "Invalid receipt")?,
    )
}

pub fn remove_peer(root: &Path, state: &Path, home: &Path) -> Result<()> {
    if root != home.join(".local/share/herdr.observatory-peer")
        || state != home.join(".local/state/herdr.observatory-peer")
    {
        return Err("Unexpected peer directory".into());
    }
    let receipt: Value = serde_json::from_slice(&common::read_owned(
        &root.join(".peer-receipt.json"),
        4096,
        true,
    )?)
    .map_err(|_| "Invalid peer receipt")?;
    if receipt != json!({"version":1,"role":"herdr.observatory-peer","files":PEER_FILES}) {
        return Err("Unknown peer receipt".into());
    }
    for entry in std::fs::read_dir(root).map_err(|_| "Peer directory unavailable")? {
        let entry = entry.map_err(|_| "Peer entry unavailable")?;
        if !entry
            .file_type()
            .map_err(|_| "Peer entry unavailable")?
            .is_file()
            || !PEER_FILES.iter().any(|name| entry.file_name() == *name)
        {
            return Err("Unknown peer file; preserving installation".into());
        }
    }
    hooks_install::uninstall(root, home)?;
    native::retire(state, &root.join(".herdr-observatory-install"))?;
    for name in [
        "allowances.json",
        "allowances.json.lock",
        "allowances-refresh.lock",
        "hook.lock",
        "sessions.json",
    ] {
        let path = state.join(name);
        if path.exists() {
            common::read_owned(&path, 1_048_576, false)?;
            std::fs::remove_file(path).map_err(|_| "Peer state removal failed")?;
        }
    }
    if state.exists() {
        match std::fs::remove_dir(state) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::DirectoryNotEmpty => {}
            Err(_) => return Err("Peer state removal failed".into()),
        }
    }
    // A tiny installed shell tail can finish after interruption even when the
    // native executable was already removed. It accepts only a retired marker.
    common::run_bounded(
        &[
            "bash".into(),
            root.join("uninstall.sh").to_string_lossy().into_owned(),
        ],
        &[],
        Duration::from_secs(5),
        4096,
        None,
    )?;
    Ok(())
}

fn valid_target(target: &str) -> bool {
    !target.is_empty()
        && target.len() <= 160
        && !target.starts_with('-')
        && target
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_.@:-".contains(&c))
}

pub fn remove_peers(root: &Path) -> Result<()> {
    let path = root.join(".peers.json");
    if !path.exists() {
        return Ok(());
    }
    let receipt: Value = serde_json::from_slice(&common::read_owned(&path, 8192, true)?)
        .map_err(|_| "Invalid peer receipt")?;
    if receipt["version"] != 1 {
        return Err("Invalid peer receipt version".into());
    }
    let targets = receipt["targets"]
        .as_array()
        .ok_or("Invalid peer targets")?;
    if targets
        .iter()
        .filter_map(Value::as_str)
        .collect::<std::collections::BTreeSet<_>>()
        .len()
        != targets.len()
        || targets.len() > 16
        || targets
            .iter()
            .any(|v| !v.as_str().is_some_and(valid_target))
    {
        return Err("Invalid peer target".into());
    }
    let mut remaining = targets.clone();
    for target in targets {
        let args = vec![
            "ssh".into(),
            "-T".into(),
            "-o".into(),
            "BatchMode=yes".into(),
            "-o".into(),
            "ConnectTimeout=4".into(),
            "--".into(),
            target.as_str().unwrap().into(),
            "if [ -L \"$HOME/.local/share/herdr.observatory-peer\" ]; then exit 1; fi; if [ ! -e \"$HOME/.local/share/herdr.observatory-peer\" ]; then exit 0; fi; exec \"$HOME/.local/share/herdr.observatory-peer/uninstall.sh\""
                .into(),
        ];
        if common::run_bounded(&args, &[], Duration::from_secs(12), 4096, None).is_err() {
            return Err("Peer removal failed; installation and receipt retained".into());
        }
        remaining.retain(|v| v != target);
        common::atomic_owned_write(
            &path,
            &serde_json::to_vec(&json!({"version":1,"targets":remaining}))
                .map_err(|_| "Invalid peer receipt")?,
        )?;
    }
    std::fs::remove_file(path).map_err(|_| "Peer receipt removal failed")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn target_is_data_not_shell() {
        assert!(valid_target("user@ws-255"));
        for bad in [
            "-oProxyCommand=x",
            "host;touch x",
            "host\nother",
            "$(id)",
            "",
        ] {
            assert!(!valid_target(bad));
        }
    }
    #[test]
    fn duplicate_peer_targets_fail_without_losing_receipt() {
        let root = std::env::temp_dir().join(format!(
            "anton-peer-receipt-{}-{}-{}",
            std::process::id(),
            common::now().to_bits(),
            {
                static SEQUENCE: std::sync::atomic::AtomicUsize =
                    std::sync::atomic::AtomicUsize::new(0);
                SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            }
        ));
        std::fs::create_dir(&root).unwrap();
        let bytes =
            serde_json::to_vec(&json!({"version":1,"targets":["fixture","fixture"]})).unwrap();
        common::atomic_owned_write(&root.join(".peers.json"), &bytes).unwrap();
        assert_eq!(remove_peers(&root).unwrap_err(), "Invalid peer target");
        assert_eq!(std::fs::read(root.join(".peers.json")).unwrap(), bytes);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn legacy_forwarding_migration_preserves_private_mapping_and_targets() {
        let key = "a".repeat(64);
        let old = json!({"hosts":[{"id":"local","socket_path":"/fixture/herdr.sock","disk_path":"/fixture/disk"},{"id":"remote","transport":"ssh","target":"user@host","gpu_api":"http://127.0.0.1:8789","gpu_state_port":8789}],"publish":{"host_id":"local","target":"user@host"},"music":{"socket_path":"/fixture/music"},"allowances":{"accounts":{key:"Personal"},"sources":[{"target":"user@host","container":"old-dashboard"}],"publish":true}});
        let migrated = migrate(old.clone()).unwrap();
        assert_eq!(
            migrated["allowances"]["accounts"],
            old["allowances"]["accounts"]
        );
        assert_eq!(
            migrated["allowances"]["sources"],
            json!([{"target":"user@host"}])
        );
        assert!(migrated.get("publish").is_none() && migrated.get("music").is_none());
        assert!(migrated["allowances"].get("publish").is_none());
        assert!(migrated["hosts"][0].get("disk_path").is_none());
        assert!(migrated["hosts"][1].get("gpu_state_port").is_none());
        assert_eq!(migrated["hosts"][1]["target"], old["hosts"][1]["target"]);
    }
}
