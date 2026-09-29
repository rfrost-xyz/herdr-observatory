//! Explicit user navigation, with exact host/session matching and no agent input.
use crate::{Result, allowances, common};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::Read;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Route {
    pub focus: Vec<String>,
    pub launch: Vec<String>,
    pub pane: String,
    pub profile: Option<Value>,
}
fn pane(value: &str) -> bool {
    let Some((workspace, pane)) = value.split_once(':') else {
        return false;
    };
    workspace
        .strip_prefix('w')
        .is_some_and(|v| !v.is_empty() && v.bytes().all(|b| b.is_ascii_alphanumeric()))
        && pane
            .strip_prefix('p')
            .is_some_and(|v| !v.is_empty() && v.bytes().all(|b| b.is_ascii_alphanumeric()))
}
fn selector(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('-')
        && value.len() <= 1024
        && !value.chars().any(char::is_control)
}
fn thread_pane<'a>(host: &str, thread: &'a str) -> Result<&'a str> {
    let target = thread.strip_prefix(&format!("{host}:")).unwrap_or("");
    if host.is_empty()
        || host.len() > 80
        || !host.as_bytes()[0].is_ascii_alphanumeric()
        || !host
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
        || !pane(target)
    {
        return Err("This thread has no valid Herdr navigation target.".into());
    }
    Ok(target)
}
/// Opaque observation of the exact saved profile route. Labels are deliberately
/// excluded: renaming a machine must neither reroute nor invalidate its threads.
pub fn profile_binding(profile: &Value) -> Result<Value> {
    let id = profile["id"]
        .as_str()
        .filter(|v| common::safe_id(v, 40) && v.as_bytes()[0].is_ascii_alphanumeric())
        .ok_or("Invalid Herdr profile identity")?;
    let target = profile["target"]
        .as_str()
        .filter(|v| allowances::valid_target(v))
        .ok_or("Invalid Herdr profile target")?;
    let session = match profile.get("session") {
        None => "default",
        Some(value) => value
            .as_str()
            .filter(|v| selector(v))
            .ok_or("Invalid Herdr profile session")?,
    };
    let bytes = serde_json::to_vec(&json!(["anton-profile-route-v1", id, target, session]))
        .map_err(|_| "Invalid Herdr route")?;
    Ok(json!({"profile_id":id,"route_key":common::sha256(&bytes)}))
}
pub fn route_observed(
    host: &str,
    thread: &str,
    profiles: &Value,
    hostname: &str,
    observed: Option<&Value>,
) -> Result<Route> {
    let Some(observed) = observed else {
        return route(host, thread, profiles, hostname);
    };
    let target = thread_pane(host, thread)?;
    let stale = "Machine changed. Try again shortly.";
    if observed.as_object().is_none_or(|v| v.len() != 2)
        || !observed["profile_id"]
            .as_str()
            .is_some_and(|v| common::safe_id(v, 40) && v.as_bytes()[0].is_ascii_alphanumeric())
        || !observed["route_key"]
            .as_str()
            .is_some_and(|v| common::hex_id(v, 64))
    {
        return Err("Invalid observed Herdr route.".into());
    }
    let matches: Vec<_> = profiles
        .as_array()
        .ok_or("Herdr machines unavailable")?
        .iter()
        .filter(|p| p["id"] == observed["profile_id"])
        .collect();
    if matches.len() != 1
        || matches[0]["enabled"] != true
        || profile_binding(matches[0])? != *observed
    {
        return Err(stale.into());
    }
    remote_route(target, matches[0])
}
pub fn route(host: &str, thread: &str, profiles: &Value, hostname: &str) -> Result<Route> {
    let target = thread_pane(host, thread)?;
    if host == hostname || Some(host) == hostname.split('.').next() {
        let command = vec!["herdr".into(), "--session".into(), "default".into()];
        return Ok(Route {
            focus: command.clone(),
            launch: command,
            pane: target.into(),
            profile: None,
        });
    }
    let matches: Vec<_> = profiles
        .as_array()
        .into_iter()
        .flatten()
        .filter(|p| p["enabled"] == true && (p["id"] == host || p["label"] == host))
        .collect();
    if matches.len() != 1 {
        return Err("No unique enabled Herdr machine matches this thread.".into());
    }
    remote_route(target, matches[0])
}
fn remote_route(target: &str, profile: &Value) -> Result<Route> {
    let remote = profile["target"].as_str().unwrap_or("");
    let session = profile
        .get("session")
        .map(|value| value.as_str().unwrap_or(""))
        .unwrap_or("default");
    let id = profile["id"].as_str().unwrap_or("");
    if !allowances::valid_target(remote) || !selector(session) || !selector(id) {
        return Err("The saved Herdr machine has an invalid target.".into());
    }
    Ok(Route {
        focus: vec![
            "ssh".into(),
            "-T".into(),
            "-o".into(),
            "BatchMode=yes".into(),
            "-o".into(),
            "ConnectTimeout=5".into(),
            "--".into(),
            remote.into(),
        ],
        launch: vec![
            "herdr".into(),
            "--remote".into(),
            remote.into(),
            "--session".into(),
            session.into(),
        ],
        pane: target.into(),
        profile: Some(profile.clone()),
    })
}
fn option<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    for (i, value) in args.iter().enumerate() {
        if let Some(value) = value.strip_prefix(&format!("{name}=")) {
            return Some(value);
        }
        if value == name {
            return args.get(i + 1).map(String::as_str);
        }
    }
    None
}
fn small_file(path: &Path) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)
        .ok()?
        .take(65537)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() > 65536 {
        None
    } else {
        Some(bytes)
    }
}
pub fn matching_window(
    clients: &Value,
    profile: Option<&Value>,
    selected: Option<&str>,
    proc: &Path,
) -> Option<String> {
    let mut processes = BTreeMap::new();
    for entry in fs::read_dir(proc).ok()?.take(65536).flatten() {
        let Ok(pid) = entry.file_name().to_string_lossy().parse::<u32>() else {
            continue;
        };
        let path = entry.path();
        let Some(status) = small_file(&path.join("status")) else {
            continue;
        };
        let status = String::from_utf8_lossy(&status);
        let Some(parent) = status.lines().find_map(|s| {
            s.strip_prefix("PPid:")
                .and_then(|s| s.trim().parse::<u32>().ok())
        }) else {
            continue;
        };
        let Some(comm) = small_file(&path.join("comm")) else {
            continue;
        };
        let comm = String::from_utf8_lossy(&comm).trim().to_owned();
        let args = if comm == "herdr" {
            String::from_utf8_lossy(&small_file(&path.join("cmdline")).unwrap_or_default())
                .trim_end_matches('\0')
                .split('\0')
                .map(str::to_owned)
                .collect::<Vec<_>>()
        } else {
            vec![]
        };
        processes.insert(pid, (parent, comm, args));
    }
    let mut candidates = BTreeMap::new();
    for (pid, (_, comm, argv)) in &processes {
        if comm != "herdr" || argv.is_empty() {
            continue;
        }
        let args = &argv[1..];
        let mut index = 0;
        let mut interactive = true;
        while index < args.len() {
            if ["--remote", "--session", "--remote-keybindings"].contains(&args[index].as_str()) {
                if index + 1 >= args.len() {
                    interactive = false;
                    break;
                }
                index += 2;
            } else if ["--remote=", "--session=", "--remote-keybindings="]
                .iter()
                .any(|p| args[index].starts_with(p))
            {
                index += 1;
            } else {
                interactive = false;
                break;
            }
        }
        if !interactive {
            continue;
        }
        let remote = option(args, "--remote");
        let session = option(args, "--session").unwrap_or("default");
        let matches = if let Some(profile) = profile {
            (remote == profile["target"].as_str()
                && session
                    == profile
                        .get("session")
                        .and_then(Value::as_str)
                        .unwrap_or("default"))
                || (remote.is_none() && session == "default" && selected == profile["id"].as_str())
        } else {
            remote.is_none() && session == "default" && selected.is_none()
        };
        if !matches {
            continue;
        }
        let mut ancestors = BTreeSet::new();
        let mut current = *pid;
        while let Some((parent, _, _)) = processes.get(&current) {
            if !ancestors.insert(current) {
                break;
            }
            current = *parent;
        }
        let windows: Vec<_> = clients
            .as_array()
            .into_iter()
            .flatten()
            .filter(|w| {
                w["pid"]
                    .as_u64()
                    .and_then(|p| u32::try_from(p).ok())
                    .is_some_and(|p| ancestors.contains(&p))
                    && w["class"].as_str().is_some_and(|s| {
                        ["ghostty", "alacritty", "kitty", "foot", "org.omarchy.herdr"]
                            .iter()
                            .any(|name| s.to_ascii_lowercase().contains(name))
                    })
            })
            .collect();
        if windows.len() == 1 {
            let window = windows[0];
            if let Some(address) = window["address"].as_str().filter(|v| {
                v.strip_prefix("0x")
                    .is_some_and(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_hexdigit()))
            }) {
                candidates.insert(
                    address.to_owned(),
                    window["focusHistoryID"].as_u64().unwrap_or(u64::MAX),
                );
            }
        }
    }
    candidates
        .into_iter()
        .min_by_key(|(_, rank)| *rank)
        .map(|(address, _)| address)
}
fn run(command: &[String], input: &[u8]) -> Result<Vec<u8>> {
    let result = common::run_process(
        command,
        input,
        Duration::from_secs(12),
        4 * 1024 * 1024,
        None,
    )
    .map_err(|_| {
        format!(
            "{} is unavailable or did not respond.",
            command.first().map(String::as_str).unwrap_or("Herdr")
        )
    })?;
    if result.status != 0 {
        let message = if command.first().is_some_and(|v| v == "ssh") {
            match result.status {
                255 => "SSH connection failed.",
                127 => "Remote Herdr is unavailable.",
                124 => "Remote Herdr did not respond.",
                _ => "Herdr could not focus this thread.",
            }
        } else {
            "Herdr could not open the requested thread."
        };
        return Err(message.into());
    }
    Ok(result.stdout)
}
fn focus_request(route: &Route) -> Result<(Vec<String>, Vec<u8>)> {
    let mut command = route.focus.clone();
    let input = if let Some(profile) = &route.profile {
        command.push(format!("exec {} --focus", allowances::PEER));
        serde_json::to_vec(&json!({"session":profile.get("session").cloned().unwrap_or(json!("default")),"pane":route.pane})).map_err(|_|"Invalid focus request")?
    } else {
        command.extend(["agent".into(), "focus".into(), route.pane.clone()]);
        vec![]
    };
    Ok((command, input))
}
pub fn focus_agent(route: &Route) -> Result<()> {
    let (command, input) = focus_request(route)?;
    run(&command, &input)?;
    Ok(())
}
pub fn peer_focus(request: &Value) -> Result<()> {
    let session = request["session"]
        .as_str()
        .filter(|v| selector(v))
        .ok_or("Invalid Herdr session")?;
    let pane = request["pane"]
        .as_str()
        .filter(|v| pane(v))
        .ok_or("Invalid Herdr pane")?;
    let command = vec![
        allowances::executable("herdr")
            .to_string_lossy()
            .into_owned(),
        "--session".into(),
        session.into(),
        "agent".into(),
        "focus".into(),
        pane.into(),
    ];
    common::run_bounded(&command, b"", Duration::from_secs(8), 65536, None)?;
    Ok(())
}
pub fn open(host: &str, thread: &str) -> Result<()> {
    open_observed(host, thread, None)
}
pub fn open_observed(host: &str, thread: &str, observed: Option<&Value>) -> Result<()> {
    let mut name = [0u8; 256];
    if unsafe { libc::gethostname(name.as_mut_ptr().cast(), name.len()) } != 0 {
        return Err("Local hostname unavailable".into());
    }
    let end = name.iter().position(|b| *b == 0).unwrap_or(name.len());
    let hostname = String::from_utf8_lossy(&name[..end]);
    let profiles =
        if observed.is_none() && (host == hostname || Some(host) == hostname.split('.').next()) {
            json!([])
        } else {
            let raw: Value = serde_json::from_slice(&run(
                &[
                    "herdr".into(),
                    "machine".into(),
                    "list".into(),
                    "--json".into(),
                ],
                b"",
            )?)
            .map_err(|_| "Herdr machines unavailable")?;
            json!(crate::fleet::profiles(&raw)?)
        };
    let route = route_observed(host, thread, &profiles, &hostname, observed)?;
    let clients: Value = serde_json::from_slice(&run(
        &["hyprctl".into(), "clients".into(), "-j".into()],
        b"",
    )?)
    .map_err(|_| "Herdr windows unavailable")?;
    let state = std::env::var_os("XDG_STATE_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| common::expand_home("~/.local/state"));
    let selected = common::read_owned(
        &state.join("herdr/client/endpoint-selection.json"),
        65536,
        false,
    )
    .ok()
    .and_then(|v| serde_json::from_slice::<Value>(&v).ok());
    let address = matching_window(
        &clients,
        route.profile.as_ref(),
        selected
            .as_ref()
            .and_then(|v| v.get("selected_profile"))
            .filter(|v| !v.is_null())
            .map(|v| v.as_str().unwrap_or("")),
        Path::new("/proc"),
    );
    focus_agent(&route)?;
    if let Some(address) = address {
        run(
            &[
                "hyprctl".into(),
                "dispatch".into(),
                format!("hl.dsp.focus({{ window = \"address:{address}\" }})"),
            ],
            b"",
        )?;
    } else {
        Command::new("omarchy")
            .args(["launch", "terminal", "--"])
            .args(&route.launch)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0)
            .spawn()
            .map_err(|_| "Herdr terminal could not open")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_routes_preserve_session_as_data() {
        let profile = json!({"id":"saved","label":"remote","target":"user@remote","session":"work; $(touch NEVER)","enabled":true});
        let value = route("remote", "remote:w1:p2", &json!([profile]), "local").unwrap();
        assert_eq!(value.focus.last().unwrap(), "user@remote");
        assert_eq!(value.launch.last().unwrap(), "work; $(touch NEVER)");
        assert_eq!(value.pane, "w1:p2");
        let (command, input) = focus_request(&value).unwrap();
        assert_eq!(
            command.last().unwrap(),
            "exec \"$HOME/.local/share/herdr.observatory-peer/anton-runtime\" --focus"
        );
        assert!(!command.last().unwrap().contains("NEVER"));
        assert_eq!(
            serde_json::from_slice::<Value>(&input).unwrap(),
            json!({"session":"work; $(touch NEVER)","pane":"w1:p2"})
        );
        assert!(route("local", "other:w1:p2", &json!([]), "local").is_err());
        assert!(route("local", "local:--help", &json!([]), "local").is_err());
    }
    #[test]
    fn missing_or_ambiguous_profile_fails_closed() {
        let profile = json!({"id":"saved","label":"remote","target":"user@remote","enabled":true});
        assert!(route("remote", "remote:w1:p2", &json!([]), "local").is_err());
        assert!(
            route(
                "remote",
                "remote:w1:p2",
                &json!([profile, profile]),
                "local"
            )
            .is_err()
        );
    }
    #[test]
    fn observed_profile_uses_stable_identity_not_host_or_label() {
        let profile = json!({"id":"saved-profile","label":"Duplicate label","target":"user@fixture","session":"work","enabled":true});
        let binding = profile_binding(&profile).unwrap();
        assert_eq!(binding.as_object().unwrap().len(), 2);
        assert!(!binding.to_string().contains("user@fixture"));
        let duplicate_label = json!({"id":"another-profile","label":"Duplicate label","target":"user@different","session":"default","enabled":true});
        for host in ["legacy-host", "saved-profile", "local"] {
            let route = route_observed(
                host,
                &format!("{host}:w1:p2"),
                &json!([profile, duplicate_label]),
                "local",
                Some(&binding),
            )
            .unwrap();
            assert_eq!(route.focus.last().unwrap(), "user@fixture");
            assert_eq!(route.profile.as_ref().unwrap()["id"], "saved-profile");
        }
        let mut renamed = profile.clone();
        renamed["label"] = json!("New label");
        assert_eq!(profile_binding(&renamed).unwrap(), binding);
        assert!(
            route_observed(
                "legacy-host",
                "legacy-host:w1:p2",
                &json!([renamed]),
                "local",
                Some(&binding)
            )
            .is_ok()
        );
        let absent = json!({"id":"saved-profile","target":"user@fixture"});
        let explicit = json!({"id":"saved-profile","target":"user@fixture","session":"default"});
        assert_eq!(
            profile_binding(&absent).unwrap(),
            profile_binding(&explicit).unwrap()
        );
    }
    #[test]
    fn observed_profile_changes_fail_closed_without_label_fallback() {
        let profile = json!({"id":"saved","label":"remote","target":"user@fixture","session":"work","enabled":true});
        let binding = profile_binding(&profile).unwrap();
        let check = |profiles: Value, observed: &Value| {
            route_observed("remote", "remote:w1:p2", &profiles, "local", Some(observed))
        };
        assert!(check(json!([]), &binding).is_err());
        for (key, value) in [
            ("target", json!("user@other")),
            ("session", json!("other")),
            ("enabled", json!(false)),
            ("id", json!("replacement")),
        ] {
            let mut changed = profile.clone();
            changed[key] = value;
            assert!(check(json!([changed]), &binding).is_err(), "{key}");
        }
        assert!(check(json!([profile, profile]), &binding).is_err());
        let mut disabled = profile.clone();
        disabled["enabled"] = json!(false);
        assert!(check(json!([profile, disabled]), &binding).is_err());
        for observed in [
            Value::Null,
            json!({}),
            json!({"profile_id":"saved","route_key":"bad"}),
            json!({"profile_id":"saved","route_key":binding["route_key"],"target":"user@fixture"}),
        ] {
            assert!(check(json!([profile]), &observed).is_err());
        }
        assert!(
            route_observed(
                "remote",
                "other:w1:p2",
                &json!([profile]),
                "local",
                Some(&binding)
            )
            .is_err()
        );
        assert!(route_observed("local", "local:w1:p2", &json!([]), "local", None).is_ok());
    }
    #[test]
    fn recent_window_ancestry_ignores_title_and_shared_terminal_ambiguity() {
        let root = std::env::temp_dir().join(format!(
            "anton-nav-{}-{}",
            std::process::id(),
            common::now().to_bits()
        ));
        fs::create_dir(&root).unwrap();
        for (pid, parent, comm, args) in [
            (10, 1, "ghostty", vec![]),
            (11, 10, "herdr", vec!["herdr"]),
            (20, 1, "ghostty", vec![]),
            (21, 20, "herdr", vec!["herdr"]),
        ] {
            let dir = root.join(pid.to_string());
            fs::create_dir(&dir).unwrap();
            fs::write(dir.join("status"), format!("PPid:\t{parent}\n")).unwrap();
            fs::write(dir.join("comm"), comm).unwrap();
            fs::write(dir.join("cmdline"), args.join("\0")).unwrap();
        }
        let windows = json!([{"pid":10,"class":"ghostty","address":"0xaaa","focusHistoryID":3},{"pid":20,"class":"ghostty","address":"0xbbb","focusHistoryID":0}]);
        assert_eq!(
            matching_window(&windows, None, None, &root),
            Some("0xbbb".into())
        );
        assert!(matching_window(&windows, None, Some("remote"), &root).is_none());
        let shared = json!([windows[0], windows[0]]);
        assert!(matching_window(&shared, None, None, &root).is_none());
        fs::remove_dir_all(root).unwrap();
    }
}
