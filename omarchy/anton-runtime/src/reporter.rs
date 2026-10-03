//! Bounded Pi presentation reporting and the Claude Code context window
//! report. Codex metrics are collector-owned.
use crate::{Result, common, hooks_install, telemetry};
use serde_json::{Value, json};
use std::ffi::OsStr;
use std::fs::OpenOptions;
use std::os::fd::AsRawFd;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
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
    let Some(socket) = local_socket(root)? else {
        return Ok(false);
    };
    let socket = socket.as_path();
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
    let Some(lock) = hook_lock(state)? else {
        return Ok(false);
    };
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
    if newer_or_equal(agent, seq) {
        return Ok(false);
    }
    let params = metadata(&event, pane, seq, &binding, "pi");
    common::rpc(
        socket,
        "pane.report_metadata",
        params,
        Duration::from_millis(400),
        1_048_576,
    )?;
    Ok(true)
}

/// The socket of the single local host in `.config.json`; `None` when
/// there is not exactly one.
fn local_socket(root: &Path) -> Result<Option<PathBuf>> {
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
        return Ok(None);
    }
    Ok(Some(PathBuf::from(
        hosts[0]["socket_path"]
            .as_str()
            .ok_or("Missing local socket")?,
    )))
}

/// Exit status of `--report claude` for invalid arguments (design D3).
pub const CLAUDE_INVALID: i32 = 2;
/// Exit status of `--report claude` when the report does not apply.
pub const CLAUDE_NOT_APPLICABLE: i32 = 3;
/// The Claude Code mod's report: a pane, a sequence, a session id and the
/// context window, parsed from four verbatim values.
#[derive(Debug, PartialEq)]
pub struct ClaudeReport {
    pane: String,
    seq: u64,
    session: String,
    window: u64,
}
/// A bounded unsigned decimal of ASCII digits only: no sign, space or
/// exponent.
fn digits(value: &str, max: usize) -> Option<u64> {
    if value.is_empty() || value.len() > max || !value.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    value.parse().ok()
}
impl ClaudeReport {
    /// Validates the values after `--report claude` at `time` (epoch
    /// seconds), before any file or socket access.
    pub fn parse(values: &[String], time: f64) -> Option<Self> {
        let [pane, seq, session, window] = values else {
            return None;
        };
        let seq = digits(seq, 16).filter(|v| *v <= 9_007_199_254_740_991)?;
        let window = digits(window, 9).filter(|v| (1..=100_000_000).contains(v))?;
        if !valid_pane(pane) || !common::safe_id(session, 128) || seq as f64 / 1e6 > time {
            return None;
        }
        Some(Self {
            pane: pane.clone(),
            seq,
            session: session.clone(),
            window,
        })
    }
}
/// The home directory of the current user from the password database.
fn passwd_home() -> Option<PathBuf> {
    use std::os::unix::ffi::OsStrExt;
    let mut buffer = vec![0 as libc::c_char; 16_384];
    // SAFETY: a zeroed `passwd` is a valid out-parameter for getpwuid_r.
    let mut entry: libc::passwd = unsafe { std::mem::zeroed() };
    let mut result = std::ptr::null_mut();
    // SAFETY: every pointer is valid for the call and the length is exact.
    let status = unsafe {
        libc::getpwuid_r(
            libc::getuid(),
            &mut entry,
            buffer.as_mut_ptr(),
            buffer.len(),
            &mut result,
        )
    };
    if status != 0 || result.is_null() || entry.pw_dir.is_null() {
        return None;
    }
    // SAFETY: getpwuid_r succeeded, so `pw_dir` is a C string in `buffer`.
    let directory = unsafe { std::ffi::CStr::from_ptr(entry.pw_dir) };
    Some(PathBuf::from(std::ffi::OsStr::from_bytes(
        directory.to_bytes(),
    )))
}
/// Absolute home and state directories for `--report claude` (design D3),
/// which inherits Claude Code's environment and working directory. Home is
/// an absolute `home` (`HOME`), else `passwd`. State is an explicit
/// `--state`, else an absolute `xdg` (`XDG_STATE_HOME`), else
/// `<home>/.local/state`, then `leaf`. A relative explicit state, or no
/// absolute home, gives `None`, so nothing resolves against the cwd.
pub fn claude_paths(
    state: Option<&Path>,
    home: Option<&OsStr>,
    xdg: Option<&OsStr>,
    passwd: impl FnOnce() -> Option<PathBuf>,
    leaf: &str,
) -> Option<(PathBuf, PathBuf)> {
    let home = match home.map(PathBuf::from).filter(|v| v.is_absolute()) {
        Some(home) => home,
        None => passwd().filter(|v| v.is_absolute())?,
    };
    let state = match state {
        Some(state) if state.is_absolute() => state.to_owned(),
        Some(_) => return None,
        None => xdg
            .map(PathBuf::from)
            .filter(|v| v.is_absolute())
            .unwrap_or_else(|| home.join(".local/state"))
            .join(leaf),
    };
    Some((home, state))
}
/// `--report claude <pane> <seq> <session-id> <window>` (design D3): exit
/// status 0 when the pane's metadata holds this bound window, 2 for invalid
/// arguments, 3 when the report does not apply and 1 for any other error.
/// Stdin is never read.
pub fn claude(root: &Path, state: Option<&Path>, values: &[String], leaf: &str) -> i32 {
    let Some(report) = ClaudeReport::parse(values, common::now()) else {
        return CLAUDE_INVALID;
    };
    if !root.is_absolute() {
        return CLAUDE_NOT_APPLICABLE;
    }
    let Some((home, state)) = claude_paths(
        state,
        std::env::var_os("HOME").as_deref(),
        std::env::var_os("XDG_STATE_HOME").as_deref(),
        passwd_home,
        leaf,
    ) else {
        return CLAUDE_NOT_APPLICABLE;
    };
    match report_claude(root, &state, &home, &report) {
        Ok(true) => 0,
        Ok(false) => CLAUDE_NOT_APPLICABLE,
        Err(error) => {
            eprintln!("{error}");
            1
        }
    }
}

/// The guarded report (design D3 guards 1 to 7). `Ok(false)` means the
/// report does not apply; `Ok(true)` means the pane's metadata holds this
/// bound window, written now or already there.
fn report_claude(root: &Path, state: &Path, home: &Path, report: &ClaudeReport) -> Result<bool> {
    let _owner = common::owner_guard(&root.join(".herdr-observatory-install"))?;
    if !hooks_install::claude_mod_recorded(root, home) {
        return Ok(false);
    }
    let Some(socket) = local_socket(root)? else {
        return Ok(false);
    };
    // As the collector does, but against the resolved home; a relative path
    // would resolve against the Claude session's working directory.
    let socket = common::expand_home_in(&socket.to_string_lossy(), home);
    if !socket.is_absolute() {
        return Ok(false);
    }
    let Some(lock) = hook_lock(state)? else {
        return Ok(false);
    };
    let _unlock = common::Unlock(&lock);
    let response = common::rpc(
        &socket,
        "pane.get",
        json!({"pane_id":report.pane}),
        Duration::from_millis(400),
        1_048_576,
    )?;
    let agent = &response["pane"];
    let session = &agent["agent_session"];
    if agent["agent"] != "claude"
        || session["agent"] != "claude"
        || session["source"] != "herdr:claude"
        || session["kind"] != "id"
        || session["value"] != report.session.as_str()
    {
        return Ok(false);
    }
    let Some(binding) = telemetry::session_binding(agent) else {
        return Ok(false);
    };
    if newer_or_equal(agent, report.seq) {
        return Ok(false);
    }
    // No change: the bound metadata already holds exactly this window and
    // no other number. `telemetry_from_agent` checks `obs_bind`.
    if telemetry::telemetry_from_agent(agent).is_some_and(|view| {
        view.as_object().is_some_and(|fields| {
            fields.iter().all(|(key, value)| match key.as_str() {
                "window" => common::number(value) == Some(report.window),
                "seq" => true,
                _ => !value.is_number(),
            })
        })
    }) {
        return Ok(true);
    }
    let raw = json!({"seq":report.seq,"event":"session","phase":"ready","window":report.window});
    let event = telemetry::telemetry_view(&raw).ok_or("Invalid report sequence")?;
    common::rpc(
        &socket,
        "pane.report_metadata",
        metadata(&event, &report.pane, report.seq, &binding, "claude"),
        Duration::from_millis(400),
        1_048_576,
    )?;
    Ok(true)
}

/// Whether the pane's metadata already has an `obs_seq` at or after `seq`.
fn newer_or_equal(agent: &Value, seq: u64) -> bool {
    agent["tokens"]["obs_seq"]
        .as_str()
        .and_then(|s| s.parse::<u64>().ok())
        .is_some_and(|old| old >= seq)
}

/// Creates the private state directory and takes `hook.lock` within 400 ms.
/// `None` means the lock stayed busy.
fn hook_lock(state: &Path) -> Result<Option<std::fs::File>> {
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
            return Ok(None);
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    Ok(Some(lock))
}

/// The v2 metadata write for `agent`. Pi adds its `display_agent` label;
/// Claude Code's pane label is left to Herdr.
fn metadata(event: &Value, pane: &str, seq: u64, binding: &str, agent: &str) -> Value {
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
    let mut params =
        json!({"pane_id":pane,"source":"user:observatory","agent":agent,"seq":seq,"tokens":tokens});
    if agent == "pi" {
        let phase = event["phase"].as_str().unwrap_or("unknown");
        let mut label = format!("pi · {phase}");
        if let Some(tool) = event["tool"].as_str() {
            label.push_str(&format!(" · {tool}"));
        }
        let label: String = label.chars().take(80).collect();
        params["display_agent"] = json!(label);
    }
    params
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn report_wire_is_atomic_and_bounded() {
        let event = json!({"seq":123,"phase":"working","event":"turn","total_input":0,"input":9_007_199_254_740_991u64});
        let value = metadata(&event, "pane", 123, "binding", "pi");
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
    /// Regression guard: the Pi wire, label included, is byte-identical to
    /// the output before `metadata` gained an agent.
    #[test]
    fn pi_metadata_bytes_are_unchanged() {
        let raw = json!({"seq":1_700_000_000_000_000u64,"event":"tool-start","phase":"tool","tool":"bash","model":"fixture-model","input":12,"context":4000,"window":128000,"usage_seq":1_700_000_000_000_000u64,"usage_source":"pi-extension"});
        let event = telemetry::telemetry_view_at(&raw, 1_800_000_000.0).unwrap();
        let value = metadata(&event, "p:1", 1_700_000_000_000_000, "binding", "pi");
        assert_eq!(value.to_string(), PI_WIRE);
    }
    const PI_WIRE: &str = r#"{"agent":"pi","display_agent":"pi · tool · bash","pane_id":"p:1","seq":1700000000000000,"source":"user:observatory","tokens":{"obs_bind":"binding","obs_children":null,"obs_completion":null,"obs_event":"tool-start","obs_model":"fixture-model","obs_n0":"12,,,","obs_n1":"4000,128000,1700000000000000,","obs_n2":",,,","obs_n3":",","obs_outcomes":null,"obs_phase":"tool","obs_result":null,"obs_seq":"1700000000000000","obs_tool":"bash","obs_usage_source":"pi-extension","obs_v":"2"}}"#;
    /// D3: the Claude wire sets `agent` to `claude`, adds no label and
    /// fills only the window slot of the four numeric groups.
    #[test]
    fn claude_metadata_has_no_label_and_only_the_window() {
        let raw = json!({"seq":1_700_000_000_000_000u64,"event":"session","phase":"ready","window":200_000});
        let event = telemetry::telemetry_view_at(&raw, 1_800_000_000.0).unwrap();
        let value = metadata(&event, "p:1", 1_700_000_000_000_000, "binding", "claude");
        assert_eq!(value.to_string(), CLAUDE_WIRE);
    }
    const CLAUDE_WIRE: &str = r#"{"agent":"claude","pane_id":"p:1","seq":1700000000000000,"source":"user:observatory","tokens":{"obs_bind":"binding","obs_children":null,"obs_completion":null,"obs_event":"session","obs_model":null,"obs_n0":",,,","obs_n1":",200000,,","obs_n2":",,,","obs_n3":",","obs_outcomes":null,"obs_phase":"ready","obs_result":null,"obs_seq":"1700000000000000","obs_tool":null,"obs_usage_source":null,"obs_v":"2"}}"#;
    /// D3: the four values, in order, with no sign, exponent or extra value.
    #[test]
    fn claude_arguments_are_bounded_digits_and_safe_ids() {
        let values = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let now = 1_800_000_000.0;
        let parsed =
            ClaudeReport::parse(&values(&["p:1", "1700000000000000", "id-1", "200000"]), now);
        assert_eq!(
            parsed,
            Some(ClaudeReport {
                pane: "p:1".into(),
                seq: 1_700_000_000_000_000,
                session: "id-1".into(),
                window: 200_000
            })
        );
        assert!(ClaudeReport::parse(&values(&["p:1", "1", "id-1", "100000000"]), now).is_some());
        for bad in [
            &["p:1", "1", "id-1"][..],
            &["p:1", "1", "id-1", "5", "6"],
            &["p:1", "1", "id-1", "100000001"],
            &["p:1", "1", "id-1", "0"],
            &["p:1", "1", "id-1", "+5"],
            &["p:1", "+1", "id-1", "5"],
            &["p:1", "9007199254740992", "id-1", "5"],
            &["p:1", "1900000000000000", "id-1", "5"],
            &["p:1", "1", "id.jsonl", "5"],
            &["p/1", "1", "id-1", "5"],
        ] {
            assert_eq!(ClaudeReport::parse(&values(bad), now), None, "{bad:?}");
        }
    }
    /// D3: home is an absolute `HOME`, else the password database; state is
    /// an absolute explicit `--state`, else an absolute `XDG_STATE_HOME`,
    /// else under home. Nothing relative is ever returned.
    #[test]
    fn claude_paths_are_absolute_without_the_cwd() {
        let os = |v: &'static str| Some(OsStr::new(v));
        let passwd = || Some(PathBuf::from("/passwd/home"));
        let leaf = "herdr.observatory";
        let paths = |state: Option<&str>, home, xdg| {
            claude_paths(state.map(Path::new), home, xdg, passwd, leaf).map(|(h, s)| {
                (
                    h.to_string_lossy().into_owned(),
                    s.to_string_lossy().into_owned(),
                )
            })
        };
        let pair = |h: &str, s: &str| Some((h.to_owned(), s.to_owned()));
        assert_eq!(
            paths(None, os("/home/a"), None),
            pair("/home/a", "/home/a/.local/state/herdr.observatory")
        );
        assert_eq!(
            paths(None, None, None),
            pair(
                "/passwd/home",
                "/passwd/home/.local/state/herdr.observatory"
            )
        );
        assert_eq!(
            paths(None, os("relative"), None),
            pair(
                "/passwd/home",
                "/passwd/home/.local/state/herdr.observatory"
            )
        );
        assert_eq!(
            paths(None, os(""), None),
            pair(
                "/passwd/home",
                "/passwd/home/.local/state/herdr.observatory"
            )
        );
        assert_eq!(
            paths(None, os("/home/a"), os("relative/state")),
            pair("/home/a", "/home/a/.local/state/herdr.observatory")
        );
        assert_eq!(
            paths(None, os("/home/a"), os("/xdg")),
            pair("/home/a", "/xdg/herdr.observatory")
        );
        assert_eq!(
            paths(Some("/explicit"), os("/home/a"), os("/xdg")),
            pair("/home/a", "/explicit")
        );
        assert_eq!(paths(Some("explicit"), os("/home/a"), os("/xdg")), None);
        assert_eq!(
            claude_paths(
                None,
                os("relative"),
                None,
                || Some(PathBuf::from("rel")),
                leaf
            ),
            None
        );
        assert_eq!(claude_paths(None, None, None, || None, leaf), None);
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
