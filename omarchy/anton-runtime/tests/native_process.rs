//! Synthetic process acceptance. Every HOME, socket, executable and SSH target is
//! fixture-owned. No interpreter, live account, remote machine or GPU is used.
mod support;

use anton_runtime::common;
use serde_json::{Value, json};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
    mpsc,
};
use std::thread;
use std::time::{Duration, Instant};

const BIN: &str = env!("CARGO_BIN_EXE_anton-runtime");
static FIXTURE_SEQUENCE: AtomicU64 = AtomicU64::new(0);
fn write(path: &Path, bytes: impl AsRef<[u8]>, mode: u32) {
    if mode & 0o111 != 0 {
        return support::write_executable(path, bytes.as_ref(), mode);
    }
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}
/// Accept instant and, once written, reply instant of one fixture Herdr RPC.
type Served = (Instant, Option<Instant>);
struct Fixture {
    dir: PathBuf,
    root: PathBuf,
    peer: PathBuf,
    state: PathBuf,
    raw: Arc<Mutex<Value>>,
    /// Accept and reply instants of every local Herdr RPC served.
    served: Arc<Mutex<Vec<Served>>>,
    /// Milliseconds the fixture socket waits before replying.
    delay: Arc<AtomicU64>,
    stop: Arc<AtomicBool>,
    server: Option<thread::JoinHandle<()>>,
}
impl Fixture {
    fn new() -> Self {
        // Fixtures of runs before the prefix changed are swept too.
        support::remove_stale_fixtures("anton-native-");
        support::remove_stale_fixtures("anton-native-exec-");
        let dir = support::fixture_dir(
            "anton-process-native-",
            FIXTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed),
        );
        fs::create_dir(&dir).unwrap();
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)).unwrap();
        let root = dir.join("plugin");
        let peer = dir.join("herdr.observatory-peer");
        let state = dir.join("state");
        for path in [&root, &peer, &state, &dir.join("bin"), &dir.join("home")] {
            fs::create_dir(path).unwrap();
        }
        for path in [&root, &peer] {
            write(
                &path.join(".herdr-observatory-install"),
                b"herdr.observatory\n",
                0o600,
            );
        }
        let socket = dir.join("herdr.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        listener.set_nonblocking(true).unwrap();
        let raw = Arc::new(Mutex::new(
            json!({"protocol":1,"version":"fixture","agents":[],"workspaces":[]}),
        ));
        let stop = Arc::new(AtomicBool::new(false));
        let served = Arc::new(Mutex::new(Vec::new()));
        let delay = Arc::new(AtomicU64::new(0));
        let state_raw = raw.clone();
        let stop_server = stop.clone();
        let server_served = served.clone();
        let server_delay = delay.clone();
        let server = thread::spawn(move || {
            while !stop_server.load(Ordering::Relaxed) {
                if let Ok((mut stream, _)) = listener.accept() {
                    let index = {
                        let mut served = server_served.lock().unwrap();
                        served.push((Instant::now(), None));
                        served.len() - 1
                    };
                    stream
                        .set_read_timeout(Some(Duration::from_secs(1)))
                        .unwrap();
                    let mut line = String::new();
                    if BufReader::new(stream.try_clone().unwrap())
                        .read_line(&mut line)
                        .is_err()
                    {
                        continue;
                    }
                    let Ok(request) = serde_json::from_str::<Value>(&line) else {
                        continue;
                    };
                    assert_eq!(request["method"], "session.snapshot");
                    let value = state_raw.lock().unwrap().clone();
                    if value == "disconnect" {
                        continue;
                    }
                    let response =
                        json!({"jsonrpc":"2.0","id":request["id"],"result":{"snapshot":value}});
                    thread::sleep(Duration::from_millis(server_delay.load(Ordering::Relaxed)));
                    let _ = writeln!(stream, "{response}");
                    server_served.lock().unwrap()[index].1 = Some(Instant::now());
                } else {
                    thread::sleep(Duration::from_millis(5));
                }
            }
        });
        let local = json!({"id":"local","socket_path":socket});
        write(&root.join(".config.json"),serde_json::to_vec(&json!({"hosts":[local,{"id":"remote","transport":"ssh","target":"fixture"}],"interval":2})).unwrap(),0o600);
        write(
            &peer.join(".config.json"),
            serde_json::to_vec(&json!({"hosts":[{"id":"remote","socket_path":socket}]})).unwrap(),
            0o600,
        );
        write(&dir.join("bin/ssh"),b"#!/bin/sh\nfor arg do last=$arg; done\ncase $last in\n *--allowances-probe) mode=--allowances-probe;;\n *--identity-probe) mode=--identity-probe;;\n *--probe*) mode=--probe;;\n *) exit 99;;\nesac\nexec \"$ANTON_TEST_BINARY\" --root \"$ANTON_TEST_PEER\" --state \"$ANTON_TEST_PEER_STATE\" \"$mode\"\n",0o755);
        Self {
            dir,
            root,
            peer,
            state,
            raw,
            served,
            delay,
            stop,
            server: Some(server),
        }
    }
    fn command(&self) -> Command {
        let mut command = Command::new(BIN);
        command
            .args([
                "--root",
                self.root.to_str().unwrap(),
                "--state",
                self.state.to_str().unwrap(),
            ])
            .env("HOME", self.dir.join("home"))
            .env("CODEX_HOME", self.dir.join("home/.codex"))
            .env("CLAUDE_CONFIG_DIR", self.dir.join("home/.claude"))
            .env("XDG_STATE_HOME", self.dir.join("xdg-state"))
            .env(
                "PATH",
                format!("{}:/usr/bin:/bin", self.dir.join("bin").display()),
            )
            .env("ANTON_TEST_BINARY", BIN)
            .env("ANTON_TEST_PEER", &self.peer)
            .env("ANTON_TEST_PEER_STATE", self.dir.join("peer-state"));
        command
    }
    fn run(&self, args: &[&str], input: &Value) -> std::process::Output {
        let mut child = self
            .command()
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.to_string().as_bytes())
            .unwrap();
        child.wait_with_output().unwrap()
    }
    fn agents(&self) {
        let binding = common::sha256(b"pi:path:/synthetic/session");
        *self.raw.lock().unwrap() = json!({"protocol":1,"version":"fixture","workspaces":[{"workspace_id":"w1","label":"Synthetic","worktree":{"checkout_path":"/synthetic/branch"}}],"agents":[{"pane_id":"w1:p1","workspace_id":"w1","cwd":"/synthetic/branch","agent":"pi","agent_status":"working","agent_session":{"agent":"pi","source":"herdr:pi","kind":"path","value":"/synthetic/session"},"tokens":{"obs_v":"2","obs_bind":binding,"obs_seq":"1700000000000000","obs_event":"output","obs_phase":"output","obs_n0":"12345,678,12000,0","obs_n1":"4000,128000,1700000000000000,90000","obs_n2":"800,88000,0,2000","obs_n3":"2,3","obs_usage_source":"pi-extension"}}]});
    }
    /// One Claude pane bound by id, and its synthetic transcript `text`.
    fn claude(&self, text: &str) {
        fs::create_dir_all(self.dir.join("home/.claude/projects/entry-a")).unwrap();
        write(&self.transcript("entry-a"), text, 0o600);
        *self.raw.lock().unwrap() = json!({"protocol":1,"version":"fixture","workspaces":[{"workspace_id":"w1","label":"Synthetic","worktree":{"checkout_path":"/synthetic/branch"}}],"agents":[{"pane_id":"w1:p1","workspace_id":"w1","cwd":"/synthetic/branch","agent":"claude","agent_status":"working","agent_session":{"agent":"claude","source":"herdr:claude","kind":"id","value":CLAUDE_ID}}]});
    }
    fn transcript(&self, entry: &str) -> PathBuf {
        self.dir
            .join(format!("home/.claude/projects/{entry}/{CLAUDE_ID}.jsonl"))
    }
    /// One direct peer probe with `cursors`, as the SSH command runs it.
    fn probe(&self, cursors: &Value) -> Value {
        let mut child = Command::new(BIN);
        child
            .args([
                "--root",
                self.peer.to_str().unwrap(),
                "--state",
                self.dir.join("peer-state").to_str().unwrap(),
                "--probe",
            ])
            .env("HOME", self.dir.join("home"))
            .env("CODEX_HOME", self.dir.join("home/.codex"))
            .env("CLAUDE_CONFIG_DIR", self.dir.join("home/.claude"));
        let input = json!({"version":1,"host_id":"remote","cursors":cursors});
        let mut child = child
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.to_string().as_bytes())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success());
        let response: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(response["ok"], true);
        response
    }
    fn codex(&self) {
        let reset = (common::now() + 86400.0) as u64;
        let script = format!(
            "#!/bin/sh\nwhile IFS= read -r line; do\n case $line in\n *'\"method\":\"initialize\"'*) printf '%s\\n' '{{\"id\":1,\"result\":{{}}}}';;\n *'account/rateLimits/read'*) printf '%s\\n' '{{\"id\":2,\"result\":{{\"accountId\":\"synthetic-account\",\"rateLimits\":{{\"primary\":{{\"windowDurationMins\":10080,\"usedPercent\":25,\"resetsAt\":{reset}}}}}}}}}';;\n *'account/usage/read'*) printf '%s\\n' '{{\"id\":3,\"result\":{{}}}}';;\n esac\ndone\n"
        );
        write(&self.dir.join("bin/codex"), script, 0o755);
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(server) = self.server.take() {
            server.join().unwrap();
        }
        let _ = fs::remove_dir_all(&self.dir);
    }
}
struct Stream {
    child: Child,
    rx: mpsc::Receiver<Value>,
}
impl Stream {
    fn new(f: &Fixture) -> Self {
        let mut child = f
            .command()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let stdout = child.stdout.take().unwrap();
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else {
                    break;
                };
                let value = serde_json::from_str(&line).expect("valid snapshot");
                if tx.send(value).is_err() {
                    break;
                }
            }
        });
        Self { child, rx }
    }
    fn until(&self, predicate: impl Fn(&Value) -> bool) -> Value {
        let end = Instant::now() + Duration::from_secs(18);
        loop {
            let snapshot = self
                .rx
                .recv_timeout(end.saturating_duration_since(Instant::now()))
                .expect("snapshot deadline");
            if predicate(&snapshot) {
                return snapshot;
            }
        }
    }
    fn write(&mut self, bytes: &[u8]) {
        let input = self.child.stdin.as_mut().unwrap();
        input.write_all(bytes).unwrap();
        input.flush().unwrap();
    }
    fn close(&mut self) {
        drop(self.child.stdin.take());
        let end = Instant::now() + Duration::from_secs(2);
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                assert!(status.success());
                break;
            }
            assert!(Instant::now() < end, "collector failed to stop");
            thread::sleep(Duration::from_millis(10));
        }
    }
}
impl Drop for Stream {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn native_peer_matches_local_metrics_and_empty_failure_are_distinct() {
    let f = Fixture::new();
    f.agents();
    let mut stream = Stream::new(&f);
    let first = stream.until(|v| {
        v["hosts"]
            .as_array()
            .unwrap()
            .iter()
            .all(|h| h["online"] == true)
    });
    let hosts = first["hosts"].as_array().unwrap();
    assert_eq!(
        hosts[0]["agents"][0]["technical"]["telemetry"],
        hosts[1]["agents"][0]["technical"]["telemetry"]
    );
    assert_eq!(
        hosts[0]["agents"][0]["technical"]["telemetry"]["input"],
        12345
    );
    assert!(first.get("music").is_none());
    assert!(first.get("publish").is_none());
    *f.raw.lock().unwrap() = json!({"agents":[],"workspaces":[]});
    stream.until(|v| {
        v["hosts"]
            .as_array()
            .unwrap()
            .iter()
            .all(|h| h["online"] == true && h["agents"] == json!([]))
    });
    *f.raw.lock().unwrap() = json!("disconnect");
    stream.until(|v| {
        v["hosts"]
            .as_array()
            .unwrap()
            .iter()
            .all(|h| h["connection_state"] == "unreachable" && h["agents"] == json!([]))
    });
    stream.close();
}

#[test]
fn slow_remote_does_not_block_local_and_owner_eof_kills_descendants() {
    let f = Fixture::new();
    f.agents();
    write(&f.dir.join("bin/ssh"),b"#!/bin/sh\necho $$ > \"$ANTON_TEST_PEER/ssh.pid\"\nsleep 60 &\necho $! > \"$ANTON_TEST_PEER/sleep.pid\"\nwait\n",0o755);
    let mut stream = Stream::new(&f);
    let snapshot = stream.until(|v| v["hosts"][0]["online"] == true);
    assert_eq!(snapshot["hosts"][1]["connection_state"], "connecting");
    let end = Instant::now() + Duration::from_secs(1);
    while !f.peer.join("sleep.pid").exists() {
        assert!(Instant::now() < end);
        thread::sleep(Duration::from_millis(10));
    }
    stream.close();
    for name in ["ssh.pid", "sleep.pid"] {
        let pid = fs::read_to_string(f.peer.join(name)).unwrap();
        let status = fs::read_to_string(format!("/proc/{}/stat", pid.trim()));
        assert!(
            status.is_err() || status.unwrap().split_whitespace().nth(2) == Some("Z"),
            "descendant still running"
        );
    }
}

#[test]
fn standalone_peer_allowance_does_not_require_threads_and_projects_to_stream() {
    let f = Fixture::new();
    f.codex();
    let unmapped = f.run(&["--allowances-probe"], &json!({}));
    assert!(unmapped.status.success());
    assert_eq!(
        serde_json::from_slice::<Value>(&unmapped.stdout).unwrap(),
        json!([])
    );
    let key = anton_runtime::allowances::account_key("synthetic-account").unwrap();
    for root in [&f.root, &f.peer] {
        let mut cfg: Value =
            serde_json::from_slice(&fs::read(root.join(".config.json")).unwrap()).unwrap();
        cfg["allowances"] = json!({"accounts":{key.clone():{"id":"synthetic","label":"Synthetic","category":"Personal"}}});
        write(
            &root.join(".config.json"),
            serde_json::to_vec(&cfg).unwrap(),
            0o600,
        );
    }
    let out = f.run(&["--allowances-probe"], &json!({}));
    assert!(out.status.success());
    let rows: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(rows[0]["weekly_remaining"], 75);
    // From this point only the peer can read the synthetic account. This proves
    // an active local thread or successful local account is not a prerequisite.
    let codex = fs::read_to_string(f.dir.join("bin/codex"))
        .unwrap()
        .replace(
            "#!/bin/sh\n",
            "#!/bin/sh\n[ \"${ANTON_TEST_REMOTE:-}\" = 1 ] || exit 1\n",
        );
    write(&f.dir.join("bin/codex"), codex, 0o755);
    let ssh = fs::read_to_string(f.dir.join("bin/ssh"))
        .unwrap()
        .replace("#!/bin/sh\n", "#!/bin/sh\nexport ANTON_TEST_REMOTE=1\n");
    write(&f.dir.join("bin/ssh"), ssh, 0o755);
    let mut config: Value =
        serde_json::from_slice(&fs::read(f.root.join(".config.json")).unwrap()).unwrap();
    config["allowances"] = json!({"accounts":{key:{"id":"synthetic","label":"Synthetic","category":"Personal"}},"sources":[{"target":"fixture"}]});
    write(
        &f.root.join(".config.json"),
        serde_json::to_vec(&config).unwrap(),
        0o600,
    );
    let mut stream = Stream::new(&f);
    let snapshot = stream.until(|v| used(v, 0) == Some(25.0));
    assert_eq!(snapshot["allowances"][0]["status"], "available");
    assert_eq!(snapshot["allowances"][0]["account_id"], "synthetic");
    assert!(!snapshot.to_string().contains("synthetic-account"));
    assert!(
        snapshot["hosts"]
            .as_array()
            .unwrap()
            .iter()
            .all(|h| h["agents"] == json!([]))
    );
    stream.close();
}

#[test]
fn oversized_peer_is_rejected_and_retired_owner_cannot_restart() {
    let f = Fixture::new();
    f.agents();
    write(
        &f.dir.join("bin/ssh"),
        b"#!/bin/sh\nhead -c 5000000 /dev/zero\n",
        0o755,
    );
    let mut stream = Stream::new(&f);
    let result = stream.until(|v| {
        v["hosts"][0]["online"] == true && v["hosts"][1]["connection_state"] == "unreachable"
    });
    assert_eq!(result["hosts"][1]["agents"], json!([]));
    stream.close();
    let retire = f.run(&["--retire-checkpoints"], &json!({}));
    assert!(
        retire.status.success(),
        "{}",
        String::from_utf8_lossy(&retire.stderr)
    );
    let rejected = f.run(&[], &json!({}));
    assert!(!rejected.status.success());
    let report = f.run(
        &["--report", "pi", "w1:p1", "1700000000000000"],
        &json!({"event":"output","phase":"output"}),
    );
    assert!(!report.status.success());
}

#[test]
fn sigterm_stops_hanging_peer_and_restart_has_fresh_state() {
    let f = Fixture::new();
    f.agents();
    write(
        &f.dir.join("bin/ssh"),
        b"#!/bin/sh\necho $$ > \"$ANTON_TEST_PEER/ssh.pid\"\nexec sleep 60\n",
        0o755,
    );
    let mut stream = Stream::new(&f);
    stream.until(|v| v["hosts"][0]["online"] == true);
    unsafe {
        libc::kill(stream.child.id() as i32, libc::SIGTERM);
    }
    let end = Instant::now() + Duration::from_secs(3);
    loop {
        if stream.child.try_wait().unwrap().is_some() {
            break;
        }
        assert!(Instant::now() < end, "SIGTERM deadline");
        thread::sleep(Duration::from_millis(10));
    }
    let pid = fs::read_to_string(f.peer.join("ssh.pid")).unwrap();
    assert!(!Path::new(&format!("/proc/{}", pid.trim())).exists());
    *f.raw.lock().unwrap() = json!({"agents":[],"workspaces":[]});
    let mut restarted = Stream::new(&f);
    let sample = restarted.until(|v| v["hosts"][0]["online"] == true);
    assert_eq!(sample["hosts"][0]["agents"], json!([]));
    restarted.close();
}

#[test]
fn native_install_uninstall_preserves_unknown_files_and_retries_retirement() {
    let f = Fixture::new();
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("herdr.observatory");
    let stage = f.dir.join("source");
    fs::create_dir(&stage).unwrap();
    for entry in fs::read_dir(&source).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            fs::copy(entry.path(), stage.join(entry.file_name())).unwrap();
        }
    }
    // Packaging is exercised with this exact Cargo-built binary. Compilation is
    // separately gated; replacing only the build wrapper avoids a second build.
    write(
        &stage.join("build-native.sh"),
        b"#!/bin/sh\ninstall -m755 \"$ANTON_TEST_BINARY\" \"$1\"\n",
        0o755,
    );
    for name in ["omarchy", "omarchy-plugin-validate"] {
        write(&f.dir.join("bin").join(name), b"#!/bin/sh\nexit 0\n", 0o755);
    }
    write(&f.dir.join("bin/omarchy-shell"),b"#!/bin/sh\ncase $2 in\n listPlugins) printf '%s\\n' '[{\"id\":\"herdr.observatory\",\"enabled\":true}]';;\n setPluginEnabled) echo ok;;\n *) :;;\nesac\n",0o755);
    let config_home = f.dir.join("config");
    let run = |script: &Path| {
        let mut command = Command::new("bash");
        command
            .arg(script)
            .env("HOME", f.dir.join("home"))
            .env("XDG_CONFIG_HOME", &config_home)
            .env("XDG_STATE_HOME", f.dir.join("xdg-state"))
            .env(
                "PATH",
                format!("{}:/usr/bin:/bin", f.dir.join("bin").display()),
            )
            .env("ANTON_CONFIG", f.root.join(".config.json"))
            .env("ANTON_TEST_BINARY", BIN);
        command.output().unwrap()
    };
    let installed = run(&stage.join("install.sh"));
    assert!(
        installed.status.success(),
        "{}",
        String::from_utf8_lossy(&installed.stderr)
    );
    let target = config_home.join("omarchy/plugins/herdr.observatory");
    assert!(target.join("anton-runtime").is_file());
    for old in [
        "runtime.py",
        "runtime.zip",
        "native-adapter.py",
        "refresh-allowances.py",
    ] {
        assert!(!target.join(old).exists());
    }
    let extension = f.dir.join("home/.pi/agent/extensions/observatory.ts");
    assert!(extension.is_file());
    write(&target.join("unrelated.txt"), b"retain me", 0o600);
    let refused = run(&target.join("uninstall.sh"));
    assert!(!refused.status.success());
    assert!(extension.is_file());
    assert!(target.join("anton-runtime").is_file());
    fs::remove_file(target.join("unrelated.txt")).unwrap();
    fs::rename(target.join("anton-runtime"), f.dir.join("saved-runtime")).unwrap();
    std::os::unix::fs::symlink(BIN, target.join("anton-runtime")).unwrap();
    assert!(!run(&target.join("uninstall.sh")).status.success());
    assert!(extension.is_file());
    fs::remove_file(target.join("anton-runtime")).unwrap();
    fs::rename(f.dir.join("saved-runtime"), target.join("anton-runtime")).unwrap();
    // Simulate a prior successful retirement followed by interrupted deletion.
    let retired = Command::new(BIN)
        .args([
            "--root",
            target.to_str().unwrap(),
            "--state",
            f.state.to_str().unwrap(),
            "--retire-checkpoints",
        ])
        .output()
        .unwrap();
    assert!(retired.status.success());
    let removed = run(&target.join("uninstall.sh"));
    assert!(
        removed.status.success(),
        "{}",
        String::from_utf8_lossy(&removed.stderr)
    );
    assert!(!target.exists());
    assert!(!extension.exists());
    // A missing-runtime retired retry must not traverse a replacement state
    // symlink, and must still clean recognised state after the conflict clears.
    fs::create_dir(&target).unwrap();
    write(
        &target.join("uninstall.sh"),
        fs::read(stage.join("uninstall.sh")).unwrap(),
        0o755,
    );
    write(
        &target.join(".herdr-observatory-install"),
        b"herdr.observatory:retired\n",
        0o600,
    );
    let state = f.dir.join("xdg-state/herdr.observatory");
    fs::create_dir_all(state.parent().unwrap()).unwrap();
    let unrelated = f.dir.join("unrelated-state");
    fs::create_dir(&unrelated).unwrap();
    write(&unrelated.join("privacy.ini"), b"retain", 0o600);
    std::os::unix::fs::symlink(&unrelated, &state).unwrap();
    assert!(!run(&target.join("uninstall.sh")).status.success());
    assert_eq!(fs::read(unrelated.join("privacy.ini")).unwrap(), b"retain");
    fs::remove_file(&state).unwrap();
    fs::create_dir(&state).unwrap();
    write(&state.join("allowances.json"), b"{}", 0o600);
    assert!(run(&target.join("uninstall.sh")).status.success());
    assert!(!target.exists());
    assert!(!state.exists());
}

#[test]
fn codex_child_completion_survives_restart_and_checkpoint_has_no_paths() {
    let f = Fixture::new();
    let sessions = f.dir.join("home/.codex/sessions");
    fs::create_dir_all(&sessions).unwrap();
    let file = sessions.join("rollout-fixture-session.jsonl");
    let stamp = ((common::now() - 2.0) * 1000.0) as u64;
    let records = [
        json!({"type":"session_meta","payload":{"id":"fixture-session"}}),
        json!({"type":"event_msg","payload":{"type":"sub_agent_activity","agent_path":"/root/worker","kind":"started","occurred_at_ms":stamp}}),
    ];
    write(
        &file,
        records.iter().map(|v| format!("{v}\n")).collect::<String>(),
        0o600,
    );
    *f.raw.lock().unwrap() = json!({"protocol":1,"version":"fixture","workspaces":[{"workspace_id":"w1","label":"Synthetic"}],"agents":[{"pane_id":"w1:p1","workspace_id":"w1","agent":"codex","agent_status":"working","agent_session":{"agent":"codex","source":"herdr:codex","kind":"id","value":"fixture-session"}}]});
    let mut stream = Stream::new(&f);
    let first = stream.until(|v| {
        v["hosts"]
            .as_array()
            .unwrap()
            .iter()
            .all(|h| h["agents"][0]["technical"]["telemetry"]["subagent_total"] == 1)
    });
    assert_eq!(
        first["hosts"][0]["agents"][0]["technical"]["telemetry"]["subagent_done"],
        0
    );
    stream.close();
    let saved = fs::read_to_string(f.state.join("replay-checkpoints.json")).unwrap();
    assert!(!saved.contains("fixture-session"));
    assert!(!saved.contains("/root/worker"));
    assert!(!saved.contains(f.dir.to_str().unwrap()));
    let mut append = fs::OpenOptions::new().append(true).open(&file).unwrap();
    writeln!(append,"{}",json!({"type":"event_msg","payload":{"type":"sub_agent_activity","agent_path":"/root/worker","kind":"completed","occurred_at_ms":stamp+1000}})).unwrap();
    drop(append);
    let mut restarted = Stream::new(&f);
    restarted.until(|v| {
        v["hosts"]
            .as_array()
            .unwrap()
            .iter()
            .all(|h| h["agents"][0]["technical"]["telemetry"]["subagent_done"] == 1)
    });
    restarted.close();
}

#[test]
fn peer_uninstaller_refuses_unknown_payload_and_finishes_retired_retry() {
    let f = Fixture::new();
    let home = f.dir.join("home");
    let root = home.join(".local/share/herdr.observatory-peer");
    let state = home.join(".local/state/herdr.observatory-peer");
    fs::create_dir_all(&root).unwrap();
    fs::copy(BIN, root.join("anton-runtime")).unwrap();
    write(
        &root.join(".herdr-observatory-install"),
        b"herdr.observatory\n",
        0o600,
    );
    write(
        &root.join(".config.json"),
        b"{\"hosts\":[{\"id\":\"fixture\"}]}",
        0o600,
    );
    let script = fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("herdr.observatory/uninstall-peer.sh"),
    )
    .unwrap();
    write(&root.join("uninstall.sh"), &script, 0o755);
    let cli = |flag: &str| {
        f.command()
            .args([
                "--root",
                root.to_str().unwrap(),
                "--state",
                state.to_str().unwrap(),
                flag,
            ])
            .output()
            .unwrap()
    };
    assert!(cli("--record-peer").status.success());
    fs::rename(root.join("anton-runtime"), f.dir.join("saved-peer-runtime")).unwrap();
    std::os::unix::fs::symlink(BIN, root.join("anton-runtime")).unwrap();
    let refused_link = Command::new("bash")
        .arg(root.join("uninstall.sh"))
        .env("HOME", &home)
        .output()
        .unwrap();
    assert!(!refused_link.status.success());
    assert!(root.join(".peer-receipt.json").exists());
    fs::remove_file(root.join("anton-runtime")).unwrap();
    fs::rename(f.dir.join("saved-peer-runtime"), root.join("anton-runtime")).unwrap();
    write(&root.join("unrelated"), b"retain", 0o600);
    assert!(!cli("--uninstall-peer").status.success());
    assert!(root.join(".peer-receipt.json").exists());
    fs::remove_file(root.join("unrelated")).unwrap();
    let removed = cli("--uninstall-peer");
    assert!(
        removed.status.success(),
        "{}",
        String::from_utf8_lossy(&removed.stderr)
    );
    assert!(!root.exists());
    // Interrupted deletion can resume with just the retired marker and shell.
    fs::create_dir(&root).unwrap();
    write(
        &root.join(".herdr-observatory-install"),
        b"herdr.observatory:retired\n",
        0o600,
    );
    write(&root.join("uninstall.sh"), script, 0o755);
    fs::create_dir_all(&state).unwrap();
    write(&state.join("allowances.json"), b"{}", 0o600);
    write(&state.join("replay-checkpoints.lock"), b"", 0o600);
    write(
        &state.join(".replay-checkpoints-0123456789abcdef"),
        b"{}",
        0o600,
    );
    write(&state.join("unrelated"), b"retain", 0o600);
    let retry = Command::new("bash")
        .arg(root.join("uninstall.sh"))
        .env("HOME", home)
        .env("PATH", "/usr/bin:/bin")
        .output()
        .unwrap();
    assert!(
        retry.status.success(),
        "{}",
        String::from_utf8_lossy(&retry.stderr)
    );
    assert!(!root.exists());
    assert!(!state.join("allowances.json").exists());
    assert!(!state.join("replay-checkpoints.lock").exists());
    assert!(!state.join(".replay-checkpoints-0123456789abcdef").exists());
    assert_eq!(fs::read(state.join("unrelated")).unwrap(), b"retain");
}

/// Used percentage of the single pacing window of one popover allowance row.
fn used(snapshot: &Value, index: usize) -> Option<f64> {
    let row = &snapshot["allowances"][index];
    let windows = row["windows"].as_array()?;
    let pacing: Vec<_> = windows.iter().filter(|w| w["pacing"] == true).collect();
    (row["status"] == "available" && pacing.len() == 1)
        .then(|| pacing[0]["used_percent"].as_f64())
        .flatten()
}
fn keys(value: &Value) -> Vec<&str> {
    value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect()
}
/// A theme object exactly as peers installed before theme removal still send it.
fn legacy_theme() -> Value {
    json!({"name":"Tokyo Night · fallback","colours":{"background":"#1a1b26","foreground":"#c0caf5","accent":"#7aa2f7","green":"#9ece6a","red":"#f7768e","yellow":"#e0af68","muted":"#565f89","lighter_background":"#24283b"}})
}
/// Replaces SSH with a counting stub that answers like an unchanged old peer.
fn legacy_peer(f: &Fixture) {
    let sample = json!({"version":1,"host_id":"remote","session":"default","ok":true,"result":{"agents":[],"theme":legacy_theme(),"sampled_at":common::now(),"error":null,"protocol":1,"version":"fixture","cursors":{}}});
    write(&f.dir.join("remote-sample.json"), sample.to_string(), 0o600);
    write(&f.dir.join("bin/ssh"),b"#!/bin/sh\nfor arg do last=$arg; done\nbase=\"$ANTON_TEST_PEER/..\"\nprintf '%s\\n' ssh >> \"$base/ssh-calls\"\ncase $last in\n *--allowances-probe*) cat > /dev/null; printf '[]\\n';;\n *--probe*) cat > /dev/null; cat \"$base/remote-sample.json\";;\n *) exit 91;;\nesac\n",0o755);
}
fn lines(path: &Path) -> usize {
    fs::read_to_string(path).map_or(0, |v| v.lines().count())
}
fn local_sampled_at(snapshot: &Value) -> f64 {
    snapshot["hosts"][0]["sampled_at"].as_f64().unwrap_or(0.0)
}

#[test]
fn snapshot_has_exact_contract_keys_and_probe_theme_is_null() {
    let f = Fixture::new();
    f.agents();
    // Retired theme keys still load and are otherwise ignored.
    let mut cfg: Value =
        serde_json::from_slice(&fs::read(f.root.join(".config.json")).unwrap()).unwrap();
    cfg["theme_host"] = json!("remote");
    cfg["hosts"][0]["theme_path"] = json!(f.dir.join("missing-theme"));
    write(&f.root.join(".config.json"), cfg.to_string(), 0o600);
    let mut stream = Stream::new(&f);
    let snapshot = stream.until(|v| {
        v["hosts"]
            .as_array()
            .unwrap()
            .iter()
            .all(|h| h["online"] == true)
    });
    stream.close();
    assert_eq!(
        keys(&snapshot),
        [
            "allowances",
            "at",
            "fleet_discovery",
            "heartbeat_seconds",
            "hosts",
            "interval"
        ]
    );
    assert_eq!(snapshot["heartbeat_seconds"], 4);
    let required = [
        "agents",
        "connection_state",
        "error",
        "id",
        "label",
        "online",
        "sampled_at",
    ];
    for host in snapshot["hosts"].as_array().unwrap() {
        let keys = keys(host);
        assert!(required.iter().all(|key| keys.contains(key)), "{keys:?}");
        assert!(
            keys.iter()
                .all(|key| required.contains(key)
                    || ["navigation", "protocol", "version"].contains(key)),
            "{keys:?}"
        );
    }
    let probe = Command::new(BIN)
        .args([
            "--root",
            f.peer.to_str().unwrap(),
            "--state",
            f.dir.join("peer-state").to_str().unwrap(),
            "--probe",
        ])
        .env("HOME", f.dir.join("home"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    probe
        .stdin
        .as_ref()
        .unwrap()
        .write_all(
            json!({"version":1,"host_id":"remote","cursors":{}})
                .to_string()
                .as_bytes(),
        )
        .unwrap();
    let output = probe.wait_with_output().unwrap();
    assert!(output.status.success());
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(response["ok"], true);
    assert_eq!(response["result"].get("theme"), Some(&Value::Null));
    assert_eq!(response["result"]["agents"][0]["host"], "remote");
}

#[test]
fn unchanged_peer_result_with_theme_object_reports_connected() {
    let f = Fixture::new();
    legacy_peer(&f);
    let mut stream = Stream::new(&f);
    let snapshot = stream.until(|v| v["hosts"][1]["connection_state"] == "connected");
    assert_eq!(snapshot["hosts"][1]["online"], true);
    assert!(snapshot.get("theme").is_none());
    stream.close();
}

#[test]
fn owner_refresh_after_emission_yields_fresh_local_sample_promptly() {
    let f = Fixture::new();
    f.agents();
    let mut stream = Stream::new(&f);
    stream.until(|v| {
        v["hosts"]
            .as_array()
            .unwrap()
            .iter()
            .all(|h| h["online"] == true)
    });
    // Request straight after an emission, so the four-second heartbeat cannot
    // coincidentally satisfy the deadline.
    stream.rx.recv_timeout(Duration::from_secs(6)).unwrap();
    let requested = common::now();
    let began = Instant::now();
    stream.write(b"refresh\n");
    stream.until(|v| local_sampled_at(v) >= requested);
    assert!(
        began.elapsed() < Duration::from_millis(1500),
        "refresh took {:?}",
        began.elapsed()
    );
    stream.close();
}

#[test]
fn owner_refresh_during_inflight_local_sample_samples_again_at_once() {
    let f = Fixture::new();
    f.agents();
    write(
        &f.root.join(".config.json"),
        json!({"hosts":[{"id":"local","socket_path":f.dir.join("herdr.sock")}]}).to_string(),
        0o600,
    );
    let mut stream = Stream::new(&f);
    stream.until(|v| v["hosts"][0]["online"] == true);
    f.delay.store(1200, Ordering::Relaxed);
    let end = Instant::now() + Duration::from_secs(6);
    let inflight = loop {
        let served = f.served.lock().unwrap();
        if let Some(index) = served.iter().rposition(|(_, reply)| reply.is_none()) {
            if index > 0 && served[index].0.elapsed() < Duration::from_millis(300) {
                break index;
            }
        }
        drop(served);
        assert!(Instant::now() < end, "no in-flight local sample");
        thread::sleep(Duration::from_millis(5));
    };
    let requested = common::now();
    stream.write(b"refresh\n");
    stream.until(|v| local_sampled_at(v) >= requested);
    let served = f.served.lock().unwrap().clone();
    let replied = served[inflight].1.expect("in-flight sample completed");
    let next = served[inflight + 1].0;
    assert!(
        next.duration_since(replied) < Duration::from_millis(500),
        "resample waited {:?} after the in-flight reply",
        next.duration_since(replied)
    );
    stream.close();
}

#[test]
fn owner_refresh_recomputes_allowances_from_local_cache_at_once() {
    let f = Fixture::new();
    write(&f.dir.join("bin/codex"), b"#!/bin/sh\nexit 1\n", 0o755);
    let key = "a".repeat(64);
    write(&f.root.join(".config.json"),json!({"hosts":[{"id":"local","socket_path":f.dir.join("herdr.sock")}],"allowances":{"accounts":{key.clone():"Personal"}}}).to_string(),0o600);
    let cache = |remaining: u64| {
        let row = json!([{"account_key":key,"plan":"pro","weekly_remaining":remaining,"weekly_resets_at":(common::now()+86400.0) as u64,"sampled_at":common::now()}]);
        let temporary = f.state.join("allowances.json.fixture");
        write(&temporary, row.to_string(), 0o600);
        fs::rename(&temporary, f.state.join("allowances.json")).unwrap();
    };
    cache(50);
    let mut stream = Stream::new(&f);
    // Seeing a changed row means a periodic recompute has only just run, so
    // the next periodic one is about two seconds away.
    stream.until(|v| used(v, 0) == Some(50.0));
    cache(75);
    let began = Instant::now();
    stream.write(b"refresh\n");
    stream.until(|v| used(v, 0) == Some(25.0));
    assert!(
        began.elapsed() < Duration::from_millis(600),
        "allowance recompute took {:?}",
        began.elapsed()
    );
    stream.close();
}

#[test]
fn owner_refresh_burst_is_coalesced_and_never_wakes_ssh_or_codex() {
    let f = Fixture::new();
    f.agents();
    f.codex();
    let codex = fs::read_to_string(f.dir.join("bin/codex"))
        .unwrap()
        .replace(
            "#!/bin/sh\n",
            "#!/bin/sh\nprintf '%s\\n' codex >> \"$ANTON_TEST_PEER/../codex-calls\"\n",
        );
    write(&f.dir.join("bin/codex"), codex, 0o755);
    legacy_peer(&f);
    let key = anton_runtime::allowances::account_key("synthetic-account").unwrap();
    write(&f.root.join(".config.json"),json!({"interval":60,"hosts":[{"id":"local","socket_path":f.dir.join("herdr.sock")},{"id":"remote","transport":"ssh","target":"fixture"}],"allowances":{"accounts":{key:{"id":"synthetic","label":"Synthetic","category":"Personal"}},"sources":[{"target":"fixture"}]}}).to_string(),0o600);
    let mut stream = Stream::new(&f);
    stream.until(|v| {
        v["hosts"]
            .as_array()
            .unwrap()
            .iter()
            .all(|h| h["online"] == true)
            && used(v, 0) == Some(25.0)
    });
    let end = Instant::now() + Duration::from_secs(6);
    while lines(&f.dir.join("ssh-calls")) < 2 {
        assert!(Instant::now() < end, "startup ssh reads incomplete");
        thread::sleep(Duration::from_millis(10));
    }
    thread::sleep(Duration::from_millis(300));
    let ssh = lines(&f.dir.join("ssh-calls"));
    let codex = lines(&f.dir.join("codex-calls"));
    assert!(codex >= 1, "account worker must exist for this test");
    let began = Instant::now();
    for _ in 0..20 {
        stream.write(b"refresh\n");
        thread::sleep(Duration::from_millis(10));
    }
    thread::sleep(Duration::from_secs(3).saturating_sub(began.elapsed()));
    let window = began + Duration::from_secs(3);
    let reads = f
        .served
        .lock()
        .unwrap()
        .iter()
        .filter(|(at, _)| *at >= began && *at < window)
        .count();
    assert!((2..=4).contains(&reads), "{reads} local Herdr reads in 3 s");
    assert_eq!(lines(&f.dir.join("ssh-calls")), ssh, "refresh woke SSH");
    assert_eq!(
        lines(&f.dir.join("codex-calls")),
        codex,
        "refresh ran Codex"
    );
    stream.close();
}

#[test]
fn malformed_owner_input_then_eof_stops_and_reaps_descendants() {
    let f = Fixture::new();
    f.agents();
    write(&f.dir.join("bin/ssh"),b"#!/bin/sh\necho $$ > \"$ANTON_TEST_PEER/ssh.pid\"\nsleep 60 &\necho $! > \"$ANTON_TEST_PEER/sleep.pid\"\nwait\n",0o755);
    let mut stream = Stream::new(&f);
    stream.until(|v| v["hosts"][0]["online"] == true);
    let end = Instant::now() + Duration::from_secs(2);
    while !f.peer.join("sleep.pid").exists() {
        assert!(Instant::now() < end);
        thread::sleep(Duration::from_millis(10));
    }
    stream.write(&vec![b'x'; 256 * 1024]);
    stream.write(b"\n\xff\xfe\n\r\nunknown\nREFRESH\n\0refresh\n");
    // Ignored input must not stall publishing: a snapshot still arrives within
    // one heartbeat plus one loop wait (4 s + 1 s) and 1 s of margin.
    let malformed = Instant::now();
    stream.rx.recv_timeout(Duration::from_secs(6)).unwrap();
    assert!(malformed.elapsed() < Duration::from_secs(6));
    // The reader recovered after the oversized line: a valid refresh, sent
    // straight after an emission, still answers promptly with a local sample
    // stamped after the request.
    stream.rx.recv_timeout(Duration::from_secs(6)).unwrap();
    let requested = common::now();
    let began = Instant::now();
    stream.write(b"refresh\n");
    stream.until(|v| local_sampled_at(v) >= requested);
    assert!(
        began.elapsed() < Duration::from_millis(1500),
        "refresh after malformed input took {:?}",
        began.elapsed()
    );
    stream.write(&[b'r'; 100]);
    stream.close();
    for name in ["ssh.pid", "sleep.pid"] {
        let pid = fs::read_to_string(f.peer.join(name)).unwrap();
        let status = fs::read_to_string(format!("/proc/{}/stat", pid.trim()));
        assert!(
            status.is_err() || status.unwrap().split_whitespace().nth(2) == Some("Z"),
            "descendant still running"
        );
    }
}

#[test]
fn peer_receipt_retains_partial_progress_and_dangling_link_conflicts() {
    let f = Fixture::new();
    write(
        &f.root.join(".peers.json"),
        b"{\"version\":1,\"targets\":[\"first\",\"second\"]}",
        0o600,
    );
    write(&f.dir.join("bin/ssh"), b"#!/bin/sh\nfor arg do case $arg in first) exit 0;; second) exit 1;; esac; done\nexit 99\n", 0o755);
    assert!(
        !f.command()
            .arg("--remove-peers")
            .output()
            .unwrap()
            .status
            .success()
    );
    let receipt: Value =
        serde_json::from_slice(&fs::read(f.root.join(".peers.json")).unwrap()).unwrap();
    assert_eq!(receipt, json!({"version":1,"targets":["second"]}));
    // Execute the actual fixed remote command in the fixture HOME.
    write(
        &f.dir.join("bin/ssh"),
        b"#!/bin/sh\nfor arg do last=$arg; done\nexec /bin/sh -c \"$last\"\n",
        0o755,
    );
    let peer = f.dir.join("home/.local/share/herdr.observatory-peer");
    fs::create_dir_all(peer.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(f.dir.join("missing-peer"), &peer).unwrap();
    assert!(
        !f.command()
            .arg("--remove-peers")
            .output()
            .unwrap()
            .status
            .success()
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&fs::read(f.root.join(".peers.json")).unwrap()).unwrap(),
        receipt
    );
    assert!(
        fs::symlink_metadata(&peer)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    fs::remove_file(peer).unwrap();
    assert!(
        f.command()
            .arg("--remove-peers")
            .output()
            .unwrap()
            .status
            .success()
    );
    assert!(!f.root.join(".peers.json").exists());
}

fn fleet_fixture(f: &Fixture, inventory: Value) {
    write(&f.dir.join("inventory.json"), inventory.to_string(), 0o600);
    write(&f.dir.join("bin/herdr"),b"#!/bin/sh\ncase $1 in machine) cat \"$ANTON_TEST_PEER/../inventory.json\";; *) exit 90;; esac\n",0o755);
    write(&f.root.join(".config.json"),json!({"fleet_discovery":true,"interval":2,"hosts":[{"id":"local","socket_path":f.dir.join("herdr.sock")},{"id":"legacy","profile_id":"saved","transport":"ssh","target":"retired"}],"allowances":{"accounts":{"a".repeat(64):"Personal"},"sources":[{"profile_id":"saved","target":"retired"}]}}).to_string(),0o600);
    let row = json!({"account_key":"a".repeat(64),"plan":"pro","weekly_remaining":75,"weekly_resets_at":(common::now()+86400.0) as u64,"reset_count":2,"reset_expires_at":(common::now()+86400.0) as u64,"sampled_at":common::now()});
    write(
        &f.dir.join("remote-allowances.json"),
        json!([row]).to_string(),
        0o600,
    );
    let sample = json!({"version":1,"host_id":"legacy","session":"default","ok":true,"result":{"agents":[],"theme":legacy_theme(),"sampled_at":common::now(),"error":null,"protocol":1,"version":"fixture","cursors":{}}});
    write(&f.dir.join("remote-sample.json"), sample.to_string(), 0o600);
    write(&f.dir.join("bin/ssh"),b"#!/bin/sh\nfor arg do last=$arg; done\nbase=\"$ANTON_TEST_PEER/..\"\ncase $last in\n *--allowances-probe*) printf '%s\\n' allowance >> \"$base/calls\"; cat \"$base/remote-allowances.json\";;\n *--probe*) cat > \"$base/last-probe\"; cat \"$base/remote-sample.json\";;\n *) exit 91;;\nesac\n",0o755);
}
fn saved_profile(target: &str, label: &str) -> Value {
    json!({"id":"saved","label":label,"target":target,"session":"default","enabled":true})
}
#[test]
fn fleet_rename_preserves_account_worker_then_removal_retires_sources_and_restart() {
    let f = Fixture::new();
    fleet_fixture(&f, json!([saved_profile("fixture", "Initial")]));
    let mut stream = Stream::new(&f);
    let first = stream.until(|v| {
        v["fleet_discovery"]["state"] == "available"
            && v["hosts"]
                .as_array()
                .unwrap()
                .iter()
                .any(|h| h["id"] == "legacy" && h["online"] == true)
            && used(v, 0) == Some(25.0)
    });
    assert_eq!(first["allowances"][0]["status"], "available");
    let binding = first["hosts"][1]["navigation"].clone();
    let request: Value =
        serde_json::from_slice(&fs::read(f.dir.join("last-probe")).unwrap()).unwrap();
    assert_eq!(request["host_id"], "legacy");
    assert_eq!(request["session"], "default");
    write(
        &f.dir.join("inventory.json"),
        json!([saved_profile("fixture", "Renamed")]).to_string(),
        0o600,
    );
    let renamed = stream.until(|v| v["hosts"][1]["label"] == "Renamed");
    assert_eq!(renamed["hosts"][1]["navigation"], binding);
    assert_eq!(
        fs::read_to_string(f.dir.join("calls"))
            .unwrap()
            .lines()
            .count(),
        1,
        "rename must not restart allowance cadence"
    );
    write(&f.dir.join("inventory.json"), b"[]", 0o600);
    let removed = stream.until(|v| {
        v["hosts"].as_array().unwrap().len() == 1 && v["allowances"][0]["status"] == "unavailable"
    });
    assert_eq!(removed["allowances"][0]["windows"], json!([]));
    assert_eq!(removed["fleet_discovery"]["state"], "available");
    stream.close();
    let mut restarted = Stream::new(&f);
    let snapshot = restarted.until(|v| v["fleet_discovery"]["state"] == "available");
    assert_eq!(snapshot["hosts"].as_array().unwrap().len(), 1);
    assert_eq!(
        fs::read_to_string(f.dir.join("calls"))
            .unwrap()
            .lines()
            .count(),
        1,
        "removed binding must never use retired static target after restart"
    );
    restarted.close();
}
#[test]
fn discovery_failure_retains_inventory_and_missing_peer_is_distinct() {
    let f = Fixture::new();
    fleet_fixture(&f, json!([saved_profile("fixture", "Saved")]));
    write(&f.dir.join("bin/ssh"),b"#!/bin/sh\nfor arg do last=$arg; done\ncase $last in *--probe*) exec /bin/sh -c \"$last\";; *) printf '%s\\n' '[]';; esac\n",0o755);
    let mut stream = Stream::new(&f);
    let first = stream.until(|v| v["hosts"][1]["connection_state"] == "setup_needed");
    assert_eq!(first["hosts"][1]["agents"], json!([]));
    write(
        &f.dir.join("inventory.json"),
        vec![b'x'; anton_runtime::fleet::LIMIT + 1],
        0o600,
    );
    let failed = stream.until(|v| v["fleet_discovery"]["state"] == "unavailable");
    assert_eq!(failed["hosts"][1]["id"], "legacy");
    assert_eq!(failed["hosts"][1]["connection_state"], "setup_needed");
    stream.close();
}
#[test]
fn discovery_owner_eof_cancels_machine_list_process() {
    let f = Fixture::new();
    fleet_fixture(&f, json!([]));
    write(
        &f.dir.join("bin/herdr"),
        b"#!/bin/sh\nprintf '%s' $$ > \"$ANTON_TEST_PEER/../discovery-pid\"\nsleep 30 &\nwait\n",
        0o755,
    );
    let mut stream = Stream::new(&f);
    stream.until(|v| v["hosts"][0]["online"] == true);
    let end = Instant::now() + Duration::from_secs(1);
    while !f.dir.join("discovery-pid").exists() {
        assert!(Instant::now() < end);
        thread::sleep(Duration::from_millis(10));
    }
    let pid = fs::read_to_string(f.dir.join("discovery-pid")).unwrap();
    stream.close();
    assert!(!Path::new("/proc").join(pid).exists());
}

#[test]
fn fleet_addition_and_route_replacement_use_current_target_and_session() {
    let f = Fixture::new();
    fleet_fixture(&f, json!([]));
    let mut stream = Stream::new(&f);
    let initial = stream.until(|v| v["fleet_discovery"]["state"] == "available");
    assert_eq!(initial["hosts"].as_array().unwrap().len(), 1);
    write(
        &f.dir.join("inventory.json"),
        json!([saved_profile("first", "Added")]).to_string(),
        0o600,
    );
    let added = stream.until(|v| v["hosts"][1]["online"] == true);
    let old_binding = added["hosts"][1]["navigation"].clone();
    let mut changed = saved_profile("second", "Added");
    changed["session"] = json!("new-session");
    // Capture the transport target as well as the JSON selector at the peer boundary.
    write(&f.dir.join("bin/ssh"),b"#!/bin/sh\nfor arg do last=$arg; done\nbase=\"$ANTON_TEST_PEER/..\"\ncase $last in *--allowances-probe*) printf '%s\\n' allowance >> \"$base/calls\"; cat \"$base/remote-allowances.json\";; *--probe*) printf '%s\\n' \"$@\" > \"$base/last-ssh\"; cat > \"$base/last-probe\"; cat \"$base/remote-sample.json\";; *) exit 91;; esac\n",0o755);
    let mut response: Value =
        serde_json::from_slice(&fs::read(f.dir.join("remote-sample.json")).unwrap()).unwrap();
    response["session"] = json!("new-session");
    write(
        &f.dir.join("remote-sample.json"),
        response.to_string(),
        0o600,
    );
    write(
        &f.dir.join("inventory.json"),
        json!([changed]).to_string(),
        0o600,
    );
    let rerouted = stream
        .until(|v| v["hosts"][1]["online"] == true && v["hosts"][1]["navigation"] != old_binding);
    assert_eq!(rerouted["hosts"][1]["id"], "legacy");
    let request: Value =
        serde_json::from_slice(&fs::read(f.dir.join("last-probe")).unwrap()).unwrap();
    assert_eq!(request["session"], "new-session");
    assert_eq!(request["cursors"], json!({}));
    assert!(
        fs::read_to_string(f.dir.join("last-ssh"))
            .unwrap()
            .lines()
            .any(|s| s == "second")
    );
    stream.close();
}
#[test]
fn peer_probe_uses_validated_requested_session_instead_of_configured_socket() {
    let f = Fixture::new();
    write(&f.dir.join("peer-snapshot.json"),json!({"result":{"snapshot":{"protocol":1,"version":"session-fixture","agents":[],"workspaces":[]}}}).to_string(),0o600);
    write(&f.dir.join("bin/herdr"),b"#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$ANTON_TEST_PEER/../peer-selector\"\ncat \"$ANTON_TEST_PEER/../peer-snapshot.json\"\n",0o755);
    write(&f.peer.join(".config.json"),json!({"hosts":[{"id":"remote","socket_path":f.dir.join("herdr.sock"),"herdr":f.dir.join("bin/herdr")}]}).to_string(),0o600);
    let output = f.run(
        &["--root", f.peer.to_str().unwrap(), "--probe"],
        &json!({"version":1,"host_id":"remote","session":"chosen-session","cursors":{}}),
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["result"]["version"], "session-fixture");
    assert_eq!(
        fs::read_to_string(f.dir.join("peer-selector")).unwrap(),
        "--session\nchosen-session\napi\nsnapshot\n"
    );
    for session in [json!("-invalid"), json!("line\nbreak"), Value::Null] {
        assert!(
            !f.run(
                &["--root", f.peer.to_str().unwrap(), "--probe"],
                &json!({"version":1,"host_id":"remote","session":session,"cursors":{}})
            )
            .status
            .success()
        );
    }
}

#[test]
fn profile_removal_cancels_inflight_host_and_allowance_process_groups() {
    let f = Fixture::new();
    fleet_fixture(&f, json!([saved_profile("fixture", "Slow")]));
    write(&f.dir.join("bin/ssh"),b"#!/bin/sh\nfor arg do last=$arg; done\nbase=\"$ANTON_TEST_PEER/..\"\ncase $last in *--allowances-probe*) kind=account;; *--probe*) kind=host;; *) exit 91;; esac\nprintf '%s' $$ > \"$base/$kind-pid\"\nsleep 30 &\nprintf '%s' $! > \"$base/$kind-child\"\nwait\n",0o755);
    let mut stream = Stream::new(&f);
    stream
        .until(|v| v["fleet_discovery"]["state"] == "available" && v["hosts"][0]["online"] == true);
    let end = Instant::now() + Duration::from_secs(2);
    while !["host-child", "account-child"]
        .iter()
        .all(|name| f.dir.join(name).exists())
    {
        assert!(Instant::now() < end);
        thread::sleep(Duration::from_millis(10));
    }
    let pids: Vec<_> = ["host-pid", "host-child", "account-pid", "account-child"]
        .into_iter()
        .map(|name| fs::read_to_string(f.dir.join(name)).unwrap())
        .collect();
    write(&f.dir.join("inventory.json"), b"[]", 0o600);
    let removed = stream.until(|v| v["hosts"].as_array().unwrap().len() == 1);
    assert_eq!(removed["allowances"][0]["status"], "unavailable");
    assert_eq!(removed["allowances"][0]["windows"], json!([]));
    for pid in pids {
        let path = Path::new("/proc").join(pid);
        assert!(
            !path.exists()
                || fs::read_to_string(path.join("stat")).is_ok_and(|v| v.contains(") Z ")),
            "retired owned process still active"
        );
    }
    stream.close();
}

#[test]
fn fleet_identity_refresh_keeps_mapped_accounts_from_first_and_fifth_targets() {
    let f = Fixture::new();
    let profiles: Vec<_> = (1..=5).map(|n| json!({"id":format!("profile-{n}"),"target":format!("peer-{n}"),"label":format!("Machine {n}"),"session":"default","enabled":true})).collect();
    fleet_fixture(&f, json!(profiles));
    write(&f.root.join(".config.json"), json!({"fleet_discovery":true,"hosts":[{"id":"local","socket_path":f.dir.join("herdr.sock")}],"allowances":{"accounts":{"a".repeat(64):"Personal","b".repeat(64):"Work"}}}).to_string(), 0o600);
    write(&f.dir.join("bin/codex"), b"#!/bin/sh\nexit 1\n", 0o755);
    write(
        &f.dir.join("first-identity.json"),
        json!({"account_key":"a".repeat(64),"email":"first@example.invalid"}).to_string(),
        0o600,
    );
    write(
        &f.dir.join("fifth-identity.json"),
        json!({"account_key":"b".repeat(64),"email":"fifth@example.invalid"}).to_string(),
        0o600,
    );
    write(
        &f.dir.join("unmapped-identity.json"),
        json!({"account_key":"c".repeat(64),"email":"unmapped@example.invalid"}).to_string(),
        0o600,
    );
    write(&f.dir.join("bin/ssh"), b"#!/bin/sh\nfor arg do case $arg in peer-[1-5]) target=$arg;; esac; done\nbase=\"$ANTON_TEST_PEER/..\"\nprintf '%s\\n' \"$target\" >> \"$base/identity-targets\"\ncase $target in peer-1|peer-3) cat \"$base/first-identity.json\";; peer-5) cat \"$base/fifth-identity.json\";; *) cat \"$base/unmapped-identity.json\";; esac\n", 0o755);
    write(&f.root.join(".accounts.json"), json!({"Personal":"previous-first@example.invalid","Work":"previous-fifth@example.invalid"}).to_string(), 0o600);
    let output = f.command().arg("--refresh-identities").output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let identities: Value =
        serde_json::from_slice(&fs::read(f.root.join(".accounts.json")).unwrap()).unwrap();
    assert_eq!(
        identities,
        json!({"Personal":"first@example.invalid","Work":"fifth@example.invalid"})
    );
    assert_eq!(
        fs::read_to_string(f.dir.join("identity-targets")).unwrap(),
        "peer-1\npeer-2\npeer-3\npeer-4\npeer-5\n"
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "Saved 2 verified account labels locally.\n"
    );
}

#[test]
fn retired_hook_repair_cli_and_local_peer_uninstall_preserve_conflicts() {
    use anton_runtime::hooks_install::RETIRED_CODEX_SHIM;
    for peer in [false, true] {
        let f = Fixture::new();
        let home = f.dir.join("home");
        let root = if peer {
            home.join(".local/share/herdr.observatory-peer")
        } else {
            f.root.clone()
        };
        let state = if peer {
            home.join(".local/state/herdr.observatory-peer")
        } else {
            f.state.clone()
        };
        common::ensure_private_directory(&root).unwrap();
        write(
            &root.join(".herdr-observatory-install"),
            b"herdr.observatory\n",
            0o600,
        );
        write(
            &root.join(".config.json"),
            b"{\"hosts\":[{\"id\":\"fixture\"}]}",
            0o600,
        );
        // Copied by a child process for the reason `support::write_executable` gives.
        assert!(
            Command::new("cp")
                .arg("--")
                .arg(BIN)
                .arg(root.join("anton-runtime"))
                .status()
                .unwrap()
                .success()
        );
        fs::set_permissions(
            root.join("anton-runtime"),
            fs::Permissions::from_mode(0o700),
        )
        .unwrap();
        if peer {
            let script = fs::read(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .parent()
                    .unwrap()
                    .join("herdr.observatory/uninstall-peer.sh"),
            )
            .unwrap();
            write(&root.join("uninstall.sh"), script, 0o755);
        }
        let cli = |mode: &str| {
            f.command()
                .args([
                    "--root",
                    root.to_str().unwrap(),
                    "--state",
                    state.to_str().unwrap(),
                    mode,
                ])
                .output()
                .unwrap()
        };
        if peer {
            assert!(cli("--record-peer").status.success());
        }
        assert!(cli("--install-hooks").status.success());
        let shell = home.join(".local/share/herdr-observatory/hooks/codex.sh");
        write(&root.join(".hooks-before-native.json"),json!({"hooks":{"Stop":[{"hooks":[{"type":"command","command":format!("sh {}",shell.display())}]}]}}).to_string(),0o600);
        let extension = home.join(".pi/agent/extensions/observatory.ts");
        let pi = fs::read(&extension).unwrap();
        let repaired = cli("--repair-retired-hooks");
        assert!(
            repaired.status.success(),
            "{}",
            String::from_utf8_lossy(&repaired.stderr)
        );
        assert!(cli("--repair-retired-hooks").status.success());
        assert_eq!(fs::read(&shell).unwrap(), RETIRED_CODEX_SHIM);
        assert_eq!(fs::read(&extension).unwrap(), pi);
        write(&shell, b"user-owned change", 0o600);
        let uninstall = if peer {
            "--uninstall-peer"
        } else {
            "--uninstall-hooks"
        };
        assert!(!cli(uninstall).status.success());
        assert_eq!(fs::read(&extension).unwrap(), pi);
        assert!(root.join(".hooks-receipt.json").exists());
        write(&shell, RETIRED_CODEX_SHIM, 0o600);
        let unrelated = shell.with_file_name("unrelated.txt");
        write(&unrelated, b"keep", 0o600);
        let removed = cli(uninstall);
        assert!(
            removed.status.success(),
            "{}",
            String::from_utf8_lossy(&removed.stderr)
        );
        assert!(!shell.exists());
        assert!(!extension.exists());
        assert_eq!(fs::read(unrelated).unwrap(), b"keep");
    }
}

/// The synthetic cc5f982-shape allowance fixture with every source time moved
/// from its recorded `now` to the real clock.
fn legacy_allowances() -> Value {
    let mut fixture: Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/native-allowances-legacy.json"
    ))
    .unwrap();
    let shift = common::now() - fixture["now"].as_f64().unwrap();
    let rebase = |row: &mut Value| {
        row["sampled_at"] = json!(row["sampled_at"].as_f64().unwrap() + shift);
        for field in ["weekly_resets_at", "reset_expires_at"] {
            if let Some(at) = row[field].as_u64() {
                row[field] = json!((at as f64 + shift) as u64);
            }
        }
    };
    for row in fixture["cache"].as_array_mut().unwrap() {
        rebase(row);
    }
    for output in fixture["peer_outputs"].as_array_mut().unwrap() {
        for row in output.as_array_mut().unwrap() {
            rebase(row);
        }
    }
    fixture
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
/// Codex that counts each start and then fails, so no row can come from it.
fn failing_codex(f: &Fixture) {
    write(
        &f.dir.join("bin/codex"),
        b"#!/bin/sh\nprintf '%s\\n' codex >> \"$ANTON_TEST_PEER/../codex-calls\"\nexit 1\n",
        0o755,
    );
}

#[test]
fn unchanged_peer_allowance_row_streams_as_weekly_pacing_window() {
    let f = Fixture::new();
    failing_codex(&f);
    let legacy = legacy_allowances();
    // An installed peer is not updated: it keeps answering with the legacy
    // row, token activity and extra fields.
    write(
        &f.dir.join("remote-allowances.json"),
        legacy["peer_outputs"][0].to_string(),
        0o600,
    );
    write(&f.dir.join("bin/ssh"),b"#!/bin/sh\nfor arg do last=$arg; done\nbase=\"$ANTON_TEST_PEER/..\"\ncase $last in\n *--allowances-probe*) cat > /dev/null; cat \"$base/remote-allowances.json\";;\n *) exit 91;;\nesac\n",0o755);
    write(&f.root.join(".config.json"),json!({"hosts":[{"id":"local","socket_path":f.dir.join("herdr.sock")}],"allowances":{"accounts":legacy["accounts"],"sources":[{"target":"fixture"}]}}).to_string(),0o600);
    let mut stream = Stream::new(&f);
    let snapshot = stream.until(|v| used(v, 0) == Some(25.0));
    stream.close();
    let peer = &legacy["peer_outputs"][0][0];
    let row = &snapshot["allowances"][0];
    assert_eq!(keys(row), ALLOWANCE_KEYS);
    assert_eq!(row["provider"], "codex");
    assert_eq!(row["provider_label"], "Codex");
    assert_eq!(row["account_id"], "synthetic-a");
    assert_eq!(row["status"], "available");
    assert_eq!(row["status_text"], Value::Null);
    assert_eq!(row["plan"], "plus");
    assert_eq!(row["sampled_at"], peer["sampled_at"]);
    assert_eq!(row["reset_count"], 2);
    assert_eq!(
        row["windows"],
        json!([{"kind":"weekly","label":"Weekly","used_percent":25.0,
            "resets_at":peer["weekly_resets_at"],"duration_s":604800,"pacing":true}])
    );
    let text = snapshot["allowances"].to_string();
    for private in [
        "example.invalid",
        "theme",
        "future_field",
        "lifetime_tokens",
        "peak_daily_tokens",
        "daily_usage",
        "weekly_remaining",
        "123456",
    ] {
        assert!(!text.contains(private), "{private}");
    }
}

#[test]
fn legacy_allowance_cache_is_available_at_startup_when_codex_fails() {
    let f = Fixture::new();
    failing_codex(&f);
    let legacy = legacy_allowances();
    write(
        &f.state.join("allowances.json"),
        legacy["cache"].to_string(),
        0o600,
    );
    write(&f.root.join(".config.json"),json!({"hosts":[{"id":"local","socket_path":f.dir.join("herdr.sock")}],"allowances":{"accounts":legacy["accounts"]}}).to_string(),0o600);
    let mut stream = Stream::new(&f);
    let snapshot = stream.until(|v| v["allowances"].as_array().is_some_and(|a| a.len() == 3));
    stream.close();
    let cache = &legacy["cache"][0];
    let rows = snapshot["allowances"].as_array().unwrap();
    for row in rows {
        assert_eq!(keys(row), ALLOWANCE_KEYS);
    }
    assert_eq!(rows[0]["status"], "available");
    assert_eq!(rows[0]["sampled_at"], cache["sampled_at"]);
    assert_eq!(rows[0]["reset_count"], 1);
    assert_eq!(rows[0]["reset_expires_at"], cache["reset_expires_at"]);
    assert_eq!(
        rows[0]["windows"],
        json!([{"kind":"weekly","label":"Weekly","used_percent":40.0,
            "resets_at":cache["weekly_resets_at"],"duration_s":604800,"pacing":true}])
    );
    // The stale cached row is unavailable; the past reset keeps its count.
    assert_eq!(rows[1]["status"], "unavailable");
    assert_eq!(rows[1]["windows"], json!([]));
    assert_eq!(rows[2]["status"], "available");
    assert_eq!(rows[2]["reset_count"], 0);
    assert_eq!(rows[2]["windows"][0]["used_percent"], Value::Null);
    // The startup account refresh may start Codex, but every start fails and
    // cannot write the cache, so the first row came from the legacy cache.
    let cached: Value =
        serde_json::from_slice(&fs::read(f.state.join("allowances.json")).unwrap()).unwrap();
    assert_eq!(cached, legacy["cache"]);
    assert!(lines(&f.dir.join("codex-calls")) <= 1);
    assert!(!snapshot["allowances"].to_string().contains("daily_usage"));
}

const CLAUDE_ID: &str = "fixture-claude-session";
fn claude_record(kind: &str, second: u64, fields: &str) -> String {
    format!(
        "{{\"type\":\"{kind}\",\"sessionId\":\"{CLAUDE_ID}\",\"uuid\":\"{kind}-{second}\",\"timestamp\":\"2026-01-01T00:{:02}:{:02}.250Z\",{fields}}}",
        second / 60,
        second % 60
    )
}
fn claude_assistant(second: u64, message: &str, stop: &str, usage: [u64; 4]) -> String {
    claude_record(
        "assistant",
        second,
        &format!(
            "\"message\":{{\"id\":\"{message}\",\"model\":\"claude-fixture-1\",\"stop_reason\":{stop},\"usage\":{{\"input_tokens\":{},\"output_tokens\":{},\"cache_read_input_tokens\":{},\"cache_creation_input_tokens\":{}}},\"content\":[]}}",
            usage[0], usage[1], usage[2], usage[3]
        ),
    )
}
/// One finished turn with two counted groups: 25 output tokens in total and a
/// last response of 1110 context tokens.
fn claude_transcript() -> String {
    let lines = [
        format!("{{\"type\":\"permission-mode\",\"sessionId\":\"{CLAUDE_ID}\"}}"),
        claude_record(
            "user",
            10,
            "\"message\":{\"role\":\"user\",\"content\":\"synthetic\"}",
        ),
        claude_assistant(11, "msg-1", "\"tool_use\"", [100, 20, 1000, 50]),
        claude_assistant(12, "msg-2", "\"end_turn\"", [10, 5, 1100, 0]),
        claude_record(
            "system",
            13,
            "\"subtype\":\"turn_duration\",\"durationMs\":3000",
        ),
    ];
    lines.map(|line| line + "\n").concat()
}
fn telemetry(snapshot: &Value, host: usize) -> &Value {
    &snapshot["hosts"][host]["agents"][0]["technical"]["telemetry"]
}
/// Whether `host` is online with its pane, so a null telemetry is a value.
fn pane(snapshot: &Value, host: usize) -> bool {
    snapshot["hosts"][host]["online"] == true && snapshot["hosts"][host]["agents"][0].is_object()
}
/// A Claude probe omits the window keys (D4), while the snapshot's
/// telemetry model writes every key, so they appear there as null.
fn snapshot_form(telemetry: &Value) -> Value {
    let mut value = telemetry.clone();
    for key in ["window", "context_percent"] {
        assert!(value.get(key).is_none(), "{key} in {telemetry}");
        value[key] = Value::Null;
    }
    value
}
/// The retained numeric subset as the local re-emits it: children and
/// compactions are null.
fn retained_subset(telemetry: &Value) -> Value {
    let mut value = telemetry.clone();
    for (key, field) in value.as_object_mut().unwrap() {
        if key.starts_with("subagent_") || key == "compactions" {
            *field = Value::Null;
        }
    }
    value
}

/// Review round 2 (D4): `--probe` collects a Claude pane carrying a bound
/// reporter window exactly as one without it. A caught-up sample keeps the
/// replay's own stamp and has no window, and without a transcript the
/// telemetry stays null instead of the window-only metadata fallback.
#[test]
fn claude_peer_probe_ignores_a_bound_reporter_window() {
    let f = Fixture::new();
    f.claude(&claude_transcript());
    let plain = f.probe(&json!({}));
    let mut raw = f.raw.lock().unwrap().clone();
    let pane = &mut raw["agents"][0];
    let binding = anton_runtime::telemetry::session_binding(pane).unwrap();
    let seq = ((common::now() as u64 - 5) * 1_000_000).to_string();
    pane["tokens"] = json!({"obs_v":"2","obs_bind":binding,"obs_seq":seq,
        "obs_event":"session","obs_phase":"ready","obs_tool":null,"obs_model":null,
        "obs_result":null,"obs_usage_source":null,"obs_n0":",,,","obs_n1":",200000,,",
        "obs_n2":",,,","obs_n3":",","obs_children":null,"obs_completion":null,"obs_outcomes":null});
    *f.raw.lock().unwrap() = raw;
    let bound = f.probe(&json!({}));
    let telemetry = |probe: &Value| probe["result"]["agents"][0]["technical"]["telemetry"].clone();
    assert_eq!(telemetry(&bound), telemetry(&plain));
    assert_eq!(telemetry(&bound)["total_input"], 2260);
    assert!(telemetry(&bound).get("window").is_none());
    fs::remove_file(f.transcript("entry-a")).unwrap();
    let missing = f.probe(&json!({}));
    assert!(missing["result"]["agents"][0].is_object());
    assert!(telemetry(&missing).is_null(), "{missing}");
}

#[test]
fn claude_peer_probe_publishes_transcript_telemetry_and_old_cursor_replays_fresh() {
    let f = Fixture::new();
    f.claude(&claude_transcript());
    let first = f.probe(&json!({}));
    let technical = &first["result"]["agents"][0]["technical"];
    let sample = &technical["telemetry"];
    assert_eq!(sample["usage_source"], "claude-transcript");
    assert_eq!(
        [
            &sample["total_input"],
            &sample["total_output"],
            &sample["context"],
            &sample["output_tokens"],
        ],
        [&json!(2260), &json!(25), &json!(1110), &json!(5)]
    );
    assert!(sample["context_percent"].is_null() && sample["window"].is_null());
    assert_eq!(technical["turn_timing"]["complete"], true);
    let cursors = first["result"]["cursors"].clone();
    let (key, row) = cursors.as_object().unwrap().iter().next().unwrap();
    assert_eq!(cursors.as_object().unwrap().len(), 1);
    assert!(row["claude"].is_object() && row["caught_up"] == true);
    assert!(!cursors.to_string().contains(CLAUDE_ID));
    // An old local re-serialises the row without its block: the peer replays
    // the transcript from the header instead of resuming zeroed sums.
    let mut old = cursors.clone();
    old[key].as_object_mut().unwrap().remove("claude");
    let fresh = f.probe(&old);
    assert_eq!(
        fresh["result"]["agents"][0]["technical"]["telemetry"],
        *sample
    );
    let replayed = &fresh["result"]["cursors"][key];
    assert_eq!(
        [&replayed["claude"], &replayed["offset"], &replayed["turns"]],
        [&row["claude"], &row["offset"], &row["turns"]]
    );
    // With the block, the row resumes: a marked sum survives.
    let mut marked = cursors.clone();
    marked[key]["claude"]["output"] = json!(row["claude"]["output"].as_u64().unwrap() + 1000);
    let warm = f.probe(&marked);
    assert_eq!(
        warm["result"]["agents"][0]["technical"]["telemetry"]["total_output"],
        1025
    );
}

#[test]
fn claude_peer_sample_is_retained_for_incomplete_replay_and_dropped_on_ambiguity() {
    let f = Fixture::new();
    f.claude(&claude_transcript());
    let mut stream = Stream::new(&f);
    let first = stream.until(|v| (0..2).all(|host| telemetry(v, host)["total_output"] == 25));
    assert_eq!(telemetry(&first, 0), telemetry(&first, 1));
    let caught = telemetry(&first, 1).clone();
    assert_eq!(caught["subagent_total"], 0);
    // A partial trailing line: each fresh peer follower resumes the returned
    // row without catching up and publishes nothing, so the local re-emits
    // the retained subset, matching the local follower's own retention.
    let path = f.transcript("entry-a");
    let partial = format!(
        "{}\n",
        claude_assistant(20, "msg-3", "\"end_turn\"", [7, 7, 7, 7])
    );
    let append = |text: &str| {
        let mut file = fs::OpenOptions::new().append(true).open(&path).unwrap();
        file.write_all(text.as_bytes()).unwrap();
    };
    append(&partial[..40]);
    let incomplete = stream.until(|v| {
        (0..2).all(|host| {
            telemetry(v, host)["subagent_total"].is_null()
                && telemetry(v, host)["total_output"] == 25
        })
    });
    assert_eq!(*telemetry(&incomplete, 1), retained_subset(&caught));
    assert_eq!(telemetry(&incomplete, 0), telemetry(&incomplete, 1));
    append(&partial[40..]);
    stream.until(|v| telemetry(v, 1)["total_output"] == 32);
    // A second match makes the binding ambiguous: the peer publishes an
    // all-null sample at the row's source time, which replaces the retained one.
    fs::create_dir_all(f.dir.join("home/.claude/projects/entry-b")).unwrap();
    fs::copy(&path, f.transcript("entry-b")).unwrap();
    let ambiguous = stream.until(|v| {
        pane(v, 1) && telemetry(v, 1)["seq"].is_u64() && telemetry(v, 1)["usage_seq"].is_null()
    });
    let unknown = telemetry(&ambiguous, 1);
    assert_eq!(unknown["seq"], 1_767_225_620_250_000u64);
    for (key, value) in unknown.as_object().unwrap() {
        assert!(
            ["seq", "event", "phase"].contains(&key.as_str()) || value.is_null(),
            "{key}"
        );
    }
    // Back to one match with an incomplete replay: nothing is re-emitted.
    fs::remove_file(f.transcript("entry-b")).unwrap();
    append("{\"type\":");
    stream.until(|v| pane(v, 1) && telemetry(v, 1).is_null());
    stream.close();
}

/// Whether `sample` is the all-null Claude sample a lost or failed binding
/// publishes: only `seq`, `event` and `phase` are set.
fn all_null(sample: &Value) -> bool {
    sample["seq"].is_u64()
        && sample.as_object().unwrap().iter().all(|(key, value)| {
            ["seq", "event", "phase"].contains(&key.as_str()) || value.is_null()
        })
}

/// A record naming another session, applied in a pass that does not catch
/// up, is an identity failure on both hosts: the fresh peer follower
/// publishes the all-null sample, so the local never re-emits the old one.
#[test]
fn claude_foreign_record_in_an_incomplete_peer_pass_publishes_all_null() {
    let f = Fixture::new();
    f.claude(&claude_transcript());
    let mut stream = Stream::new(&f);
    stream.until(|v| (0..2).all(|host| telemetry(v, host)["total_output"] == 25));
    let path = f.transcript("entry-a");
    let mut file = fs::OpenOptions::new().append(true).open(&path).unwrap();
    let foreign = claude_record(
        "user",
        30,
        "\"message\":{\"role\":\"user\",\"content\":\"synthetic\"}",
    )
    .replace(CLAUDE_ID, "fixture-other-session");
    file.write_all(format!("{foreign}\n{{\"type\":").as_bytes())
        .unwrap();
    drop(file);
    stream.until(|v| (0..2).all(|host| pane(v, host) && all_null(telemetry(v, host))));
    for _ in 0..3 {
        let snapshot = stream.until(|v| pane(v, 1));
        for host in 0..2 {
            let sample = telemetry(&snapshot, host);
            assert!(all_null(sample), "{host}: {sample}");
        }
    }
    stream.close();
}

#[test]
fn claude_peer_retention_follows_invalid_totals_unknown_samples_and_missing_rows() {
    let f = Fixture::new();
    f.claude(&claude_transcript());
    let probe = f.probe(&json!({}));
    let caught = snapshot_form(&probe["result"]["agents"][0]["technical"]["telemetry"]);
    let rows = probe["result"]["cursors"].clone();
    // Edited copies of a real probe response, served by a stub peer. Each one
    // carries a distinct title, so a snapshot shows when it was accepted.
    legacy_peer(&f);
    let serve = |title: &str, telemetry: &Value, cursors: &Value| {
        let mut response = probe.clone();
        let result = &mut response["result"];
        result["sampled_at"] = json!(common::now());
        result["agents"][0]["title"] = json!(title);
        result["agents"][0]["technical"]["telemetry"] = telemetry.clone();
        result["cursors"] = cursors.clone();
        // Replace atomically: the stub may read the file at any moment.
        let staged = f.dir.join("remote-sample.json.tmp");
        write(&staged, response.to_string(), 0o600);
        fs::rename(&staged, f.dir.join("remote-sample.json")).unwrap();
    };
    let mut stream = Stream::new(&f);
    let step = |title: &str, telemetry: &Value, cursors: &Value| {
        serve(title, telemetry, cursors);
        let snapshot = stream.until(|v| v["hosts"][1]["agents"][0]["title"] == title);
        self::telemetry(&snapshot, 1).clone()
    };
    let incomplete = Value::Null;
    assert_eq!(step("caught-up", &caught, &rows), caught);
    assert_eq!(
        step("incomplete", &incomplete, &rows),
        retained_subset(&caught)
    );
    // A caught-up sample with invalid totals replaces the retained one.
    let mut invalid = caught.clone();
    for key in [
        "total_input",
        "total_output",
        "total_cache_read",
        "total_cache_write",
        "total_uncached_input",
    ] {
        invalid[key] = Value::Null;
    }
    assert_eq!(step("invalid-totals", &invalid, &rows), invalid);
    assert_eq!(
        step("incomplete-invalid", &incomplete, &rows),
        retained_subset(&invalid)
    );
    // A caught-up sample where everything is unknown drops it.
    assert_eq!(step("caught-up-again", &caught, &rows), caught);
    let mut unknown = caught.clone();
    for (key, value) in unknown.as_object_mut().unwrap() {
        if !["seq", "event", "phase"].contains(&key.as_str()) && !key.starts_with("subagent_") {
            *value = Value::Null;
        }
    }
    assert_eq!(step("unknown", &unknown, &rows), unknown);
    assert!(step("incomplete-unknown", &incomplete, &rows).is_null());
    // No cursor row: dropped, and a returning row does not revive it.
    assert_eq!(step("caught-up-last", &caught, &rows), caught);
    assert!(step("no-row", &incomplete, &json!({})).is_null());
    assert!(step("row-returns", &incomplete, &rows).is_null());
    stream.close();
}

#[test]
fn claude_peer_sample_is_not_reemitted_after_a_request_without_its_row() {
    let f = Fixture::new();
    f.claude(&claude_transcript());
    let probe = f.probe(&json!({}));
    let caught = snapshot_form(&probe["result"]["agents"][0]["technical"]["telemetry"]);
    // A partial trailing line: a peer replaying from the header does not catch
    // up, publishes nothing and returns a fresh row.
    let mut file = fs::OpenOptions::new()
        .append(true)
        .open(f.transcript("entry-a"))
        .unwrap();
    file.write_all(b"{\"type\":").unwrap();
    let restarted = f.probe(&json!({}));
    assert!(restarted["result"]["agents"][0]["technical"]["telemetry"].is_null());
    let rows = restarted["result"]["cursors"].as_object().unwrap();
    assert!(rows.len() == 1 && rows.values().all(|v| v["caught_up"] == false));
    // SSH answers from a stub while `stub-peer` exists, then from the real peer.
    // The stub's caught-up sample carries one Claude row, so it is stored, but
    // under another session's key, as a stale request after pane churn
    // carries: the next request has a Claude row, yet none for this pane.
    let mut stored = probe.clone();
    stored["result"]["sampled_at"] = json!(common::now());
    stored["result"]["agents"][0]["title"] = json!("stored");
    let row = probe["result"]["cursors"]
        .as_object()
        .unwrap()
        .values()
        .next()
        .unwrap()
        .clone();
    stored["result"]["cursors"] = json!({ common::sha256(b"another-session"): row });
    write(&f.dir.join("remote-sample.json"), stored.to_string(), 0o600);
    let flag = f.dir.join("stub-peer");
    write(&flag, "", 0o600);
    write(&f.dir.join("bin/ssh"),b"#!/bin/sh\nfor arg do last=$arg; done\nbase=\"$ANTON_TEST_PEER/..\"\nif [ -e \"$base/stub-peer\" ]; then\n case $last in\n  *--allowances-probe*) cat > /dev/null; printf '[]\\n';;\n  *--probe*) cat > /dev/null; cat \"$base/remote-sample.json\";;\n  *) exit 91;;\n esac\n exit\nfi\ncase $last in\n *--allowances-probe) mode=--allowances-probe;;\n *--identity-probe) mode=--identity-probe;;\n *--probe*) mode=--probe;;\n *) exit 99;;\nesac\nexec \"$ANTON_TEST_BINARY\" --root \"$ANTON_TEST_PEER\" --state \"$ANTON_TEST_PEER_STATE\" \"$mode\"\n",0o755);
    let mut stream = Stream::new(&f);
    let stubbed = stream.until(|v| v["hosts"][1]["agents"][0]["title"] == "stored");
    assert_eq!(*telemetry(&stubbed, 1), caught);
    fs::remove_file(&flag).unwrap();
    // The real peer replays from the header without catching up; nothing
    // proves the file is the one the retained sample measured, so it is not
    // re-emitted.
    let real = stream.until(|v| pane(v, 1) && v["hosts"][1]["agents"][0]["title"] != "stored");
    assert!(
        telemetry(&real, 1).is_null(),
        "re-emitted: {}",
        telemetry(&real, 1)
    );
    stream.close();
}

#[test]
fn executable_fixtures_run_while_sibling_threads_spawn() {
    let dir = support::fixture_dir(
        "anton-process-native-exec-",
        FIXTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed),
    );
    fs::create_dir(&dir).unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let storm: Vec<_> = (0..2)
        .map(|_| {
            let stop = stop.clone();
            thread::spawn(move || {
                while !stop.load(Ordering::Relaxed) {
                    let _ = Command::new("true").status();
                }
            })
        })
        .collect();
    let mut failures = 0;
    for index in 0..150 {
        let script = dir.join(format!("fixture-{index}"));
        write(&script, b"#!/bin/sh\nexit 0\n", 0o755);
        failures += usize::from(Command::new(&script).status().is_err());
    }
    stop.store(true, Ordering::Relaxed);
    for join in storm {
        join.join().unwrap();
    }
    let _ = fs::remove_dir_all(&dir);
    assert_eq!(failures, 0);
}

static REPORTER_SEQUENCE: AtomicU64 = AtomicU64::new(0);
/// One recorded Herdr RPC: its method and params.
type Call = (String, Value);
/// A synthetic plugin root, home and Herdr socket for `--report claude`
/// (design D3). The socket answers `pane.get` from `panes` and applies each
/// `pane.report_metadata` to the pane's tokens, as Herdr does.
struct Reporter {
    dir: PathBuf,
    root: PathBuf,
    home: PathBuf,
    state: PathBuf,
    socket: PathBuf,
    panes: Arc<Mutex<Value>>,
    calls: Arc<Mutex<Vec<Call>>>,
    stop: Arc<AtomicBool>,
    server: Option<thread::JoinHandle<()>>,
}
/// A 64-character hex hash for a synthetic receipt.
fn hex(seed: &str) -> String {
    common::sha256(seed.as_bytes())
}
impl Reporter {
    fn new() -> Self {
        let dir = support::fixture_dir(
            "anton-process-reporter-",
            REPORTER_SEQUENCE.fetch_add(1, Ordering::Relaxed),
        );
        fs::create_dir(&dir).unwrap();
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)).unwrap();
        let root = dir.join("plugin");
        let home = dir.join("home");
        for path in [&root, &home, &dir.join("cwd")] {
            fs::create_dir(path).unwrap();
        }
        write(
            &root.join(".herdr-observatory-install"),
            b"herdr.observatory\n",
            0o600,
        );
        let socket = dir.join("herdr.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        listener.set_nonblocking(true).unwrap();
        let panes = Arc::new(Mutex::new(json!({})));
        let calls = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let (server_panes, server_calls, server_stop) =
            (panes.clone(), calls.clone(), stop.clone());
        let server = thread::spawn(move || {
            while !server_stop.load(Ordering::Relaxed) {
                let Ok((mut stream, _)) = listener.accept() else {
                    thread::sleep(Duration::from_millis(5));
                    continue;
                };
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(1)))
                    .unwrap();
                let mut line = String::new();
                if BufReader::new(stream.try_clone().unwrap())
                    .read_line(&mut line)
                    .is_err()
                {
                    continue;
                }
                let request: Value = serde_json::from_str(&line).unwrap_or(Value::Null);
                let method = request["method"].as_str().unwrap_or("").to_owned();
                let params = request["params"].clone();
                server_calls
                    .lock()
                    .unwrap()
                    .push((method.clone(), params.clone()));
                let pane = params["pane_id"].as_str().unwrap_or("").to_owned();
                let mut panes = server_panes.lock().unwrap();
                // A pane marked `fixture_reject` fails its metadata write;
                // `fixture_delay_ms` delays its reply, holding the reporter
                // inside `hook.lock`.
                let delay = match method.as_str() {
                    "pane.report_metadata" => panes
                        .get(&pane)
                        .and_then(|entry| entry["fixture_delay_ms"].as_u64())
                        .unwrap_or(0),
                    _ => 0,
                };
                let result = match method.as_str() {
                    "pane.get" => {
                        Some(json!({"pane":panes.get(&pane).cloned().unwrap_or(json!({}))}))
                    }
                    "pane.report_metadata" => match panes.get_mut(&pane) {
                        Some(entry) if entry["fixture_reject"] == true => None,
                        Some(entry) => {
                            entry["tokens"] = params["tokens"].clone();
                            Some(json!({}))
                        }
                        None => Some(json!({})),
                    },
                    _ => Some(json!({})),
                };
                drop(panes);
                thread::sleep(Duration::from_millis(delay));
                let reply = match result {
                    Some(result) => json!({"jsonrpc":"2.0","id":request["id"],"result":result}),
                    None => {
                        json!({"jsonrpc":"2.0","id":request["id"],"error":{"code":-32000,"message":"fixture"}})
                    }
                };
                let _ = writeln!(stream, "{reply}");
            }
        });
        let state = dir.join("state");
        let fixture = Self {
            dir,
            root,
            home,
            state,
            socket,
            panes,
            calls,
            stop,
            server: Some(server),
        };
        fixture.config(1);
        fixture.receipt(Some(fixture.mod_entry(false)));
        fixture.pane("w1:p1", claude_pane(CLAUDE_ID));
        fixture
    }
    /// `count` local hosts, each on the fixture socket.
    fn config(&self, count: usize) {
        let hosts: Vec<_> = (0..count)
            .map(|i| json!({"id":format!("local-{i}"),"socket_path":self.socket}))
            .chain([json!({"id":"remote","transport":"ssh","target":"fixture"})])
            .collect();
        write(
            &self.root.join(".config.json"),
            serde_json::to_vec(&json!({"hosts":hosts,"interval":2})).unwrap(),
            0o600,
        );
    }
    /// The D5 `claude_mod` receipt entry, mid-refresh when `prior` is set.
    fn mod_entry(&self, prior: bool) -> Value {
        let root = self.home.join(".claude/skills/anton-observatory");
        let files: Vec<_> = [
            ".claude-plugin/plugin.json",
            "hooks/hooks.json",
            "hooks/register.js",
        ]
        .iter()
        .map(|name| {
            let mut file = json!({"path":root.join(name),"sha256":hex(name)});
            if prior {
                file["prior_sha256"] = json!(hex(&format!("prior-{name}")));
            }
            file
        })
        .collect();
        json!({"version":1,"root":root,"directories":[root,root.join(".claude-plugin"),root.join("hooks")],"files":files})
    }
    /// The hook receipt, with `entry` as `claude_mod` when given.
    fn receipt(&self, entry: Option<Value>) {
        let mut receipt = json!({"version":1,"runtime":self.root.join("anton-runtime"),
            "extension":self.home.join(".pi/agent/extensions/observatory.ts"),"sha256":hex("extension")});
        if let Some(entry) = entry {
            receipt["claude_mod"] = entry;
        }
        write(
            &self.root.join(".hooks-receipt.json"),
            serde_json::to_vec(&receipt).unwrap(),
            0o600,
        );
    }
    fn pane(&self, id: &str, pane: Value) {
        self.panes.lock().unwrap()[id] = pane;
    }
    fn methods(&self) -> Vec<String> {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .map(|(m, _)| m.clone())
            .collect()
    }
    fn writes(&self) -> Vec<Value> {
        let calls = self.calls.lock().unwrap();
        calls
            .iter()
            .filter(|(m, _)| m == "pane.report_metadata")
            .map(|(_, p)| p.clone())
            .collect()
    }
    /// `anton-runtime` with only an absolute temporary `HOME`, explicit
    /// `--root` and `--state` before `--report`, and a private empty cwd.
    fn command(&self, report: &[&str]) -> Command {
        let mut command = Command::new(BIN);
        command
            .env_clear()
            .env("HOME", &self.home)
            .current_dir(self.dir.join("cwd"))
            .args([
                "--root",
                self.root.to_str().unwrap(),
                "--state",
                self.state.to_str().unwrap(),
                "--report",
                "claude",
            ])
            .args(report)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command
    }
    /// Runs one report and returns its exit status, with no output for
    /// statuses 0, 2 and 3.
    fn report(&self, report: &[&str]) -> Option<i32> {
        let output = self.command(report).output().unwrap();
        let code = output.status.code();
        if matches!(code, Some(0 | 2 | 3)) {
            assert!(output.stdout.is_empty(), "{report:?}");
            assert!(output.stderr.is_empty(), "{report:?}");
        }
        code
    }
}
impl Drop for Reporter {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(server) = self.server.take() {
            server.join().unwrap();
        }
        let _ = fs::remove_dir_all(&self.dir);
    }
}
/// A Herdr pane bound to Claude session `id`, with no metadata yet.
fn claude_pane(id: &str) -> Value {
    json!({"pane_id":"w1:p1","agent":"claude","agent_session":{"agent":"claude","source":"herdr:claude","kind":"id","value":id},"tokens":{}})
}
/// A report sequence `ago` seconds in the past, in epoch microseconds.
fn report_seq(ago: u64) -> String {
    ((common::now() as u64 - ago) * 1_000_000).to_string()
}

/// D3: invalid arguments, including every count other than 4, 7 or 10 and
/// every malformed rate-limit tail, exit 2 before any file or socket access;
/// valid seven- and ten-value runs then report the window (exit 0).
#[test]
fn claude_report_rejects_invalid_arguments_without_socket_access() {
    let f = Reporter::new();
    let seq = report_seq(10);
    let future = ((common::now() as u64 + 3600) * 1_000_000).to_string();
    let long = "a".repeat(129);
    let mut cases: Vec<Vec<&str>> = vec![
        vec![],
        vec!["w1:p1", &seq, CLAUDE_ID],
        vec!["w1:p1", &seq, CLAUDE_ID, "200000", "extra"],
        vec!["w1:p1", &seq, CLAUDE_ID, "200000", "--state"],
        vec!["w1/p1", &seq, CLAUDE_ID, "200000"],
        vec!["", &seq, CLAUDE_ID, "200000"],
        vec!["w1:p1", "seq", CLAUDE_ID, "200000"],
        vec!["w1:p1", "+1700000000000000", CLAUDE_ID, "200000"],
        vec!["w1:p1", "9007199254740992", CLAUDE_ID, "200000"],
        vec!["w1:p1", &future, CLAUDE_ID, "200000"],
        vec!["w1:p1", &seq, ".", "200000"],
        vec!["w1:p1", &seq, "a/b", "200000"],
        vec!["w1:p1", &seq, &long, "200000"],
        vec!["w1:p1", &seq, "fixture.jsonl", "200000"],
        vec!["w1:p1", &seq, "", "200000"],
    ];
    for window in ["0", "-1", "+5", "1e6", "100000001", "\u{0663}", "", " 5"] {
        cases.push(vec!["w1:p1", &seq, CLAUDE_ID, window]);
    }
    // Rate-limit tails: counts 5, 6, 8, 9 and 11, unknown and repeated
    // kinds, bad used values and resets in the past, at `seq`, beyond the
    // bound or with 12 digits.
    let whole = seq.parse::<u64>().unwrap() / 1_000_000;
    let reset = (whole + 60).to_string();
    let at = whole.to_string();
    let past = (whole - 1).to_string();
    let beyond_five = (whole + 21_601).to_string();
    let beyond_seven = (whole + 608_401).to_string();
    let twelve = format!("00{reset}");
    let head = ["w1:p1", seq.as_str(), CLAUDE_ID, "200000"];
    let five = ["five_hour", "12.5", reset.as_str()];
    let seven = ["seven_day", "40", reset.as_str()];
    let mut tails: Vec<Vec<&str>> = vec![
        vec!["five_hour"],
        vec!["five_hour", "12.5"],
        [&five[..], &["seven_day"]].concat(),
        [&five[..], &["seven_day", "40"]].concat(),
        [&five[..], &seven, &["five_hour"]].concat(),
        [&five[..], &five].concat(),
        vec!["spend_limit", "12.5", &reset],
        vec!["five_hour", "12.5", &past],
        vec!["five_hour", "12.5", &at],
        vec!["five_hour", "12.5", &beyond_five],
        vec!["seven_day", "40", &beyond_seven],
        vec!["five_hour", "12.5", &twelve],
    ];
    for used in ["-1", "100.1", "1.25", "01", "5.0", "1e1", "+5"] {
        tails.push(vec!["five_hour", used, &reset]);
    }
    let tail_cases: Vec<Vec<&str>> = tails.iter().map(|t| [&head[..], t].concat()).collect();
    for case in cases.iter().chain(&tail_cases) {
        assert_eq!(f.report(case), Some(2), "{case:?}");
    }
    assert!(f.methods().is_empty());
    assert!(!f.state.exists());
    assert_eq!(fs::read_dir(f.dir.join("cwd")).unwrap().count(), 0);
    // The accepted neighbours: seven and ten valid values report the window.
    assert_eq!(f.report(&[&head[..], &five].concat()), Some(0));
    assert_eq!(f.methods(), ["pane.get", "pane.report_metadata"]);
    assert_claude_wire(&f.writes()[0], &seq, 200_000);
    let later = report_seq(5);
    let head = ["w1:p1", later.as_str(), CLAUDE_ID, "300000"];
    assert_eq!(f.report(&[&head[..], &seven, &five].concat()), Some(0));
    assert_eq!(f.writes().len(), 2);
    assert_claude_wire(&f.writes()[1], &later, 300_000);
}

/// D3: the four values after `claude` are taken verbatim, so an option-like
/// pane or session id is a value, never `--state`.
#[test]
fn claude_report_takes_option_like_values_verbatim() {
    let f = Reporter::new();
    let seq = report_seq(10);
    assert_eq!(f.report(&["w1:p1", &seq, "--state", "200000"]), Some(3));
    assert_eq!(f.methods(), ["pane.get"]);
    assert_eq!(f.calls.lock().unwrap()[0].1["pane_id"], "w1:p1");
    f.calls.lock().unwrap().clear();
    assert_eq!(f.report(&["--state", &seq, CLAUDE_ID, "200000"]), Some(3));
    assert_eq!(f.methods(), ["pane.get"]);
    assert_eq!(f.calls.lock().unwrap()[0].1["pane_id"], "--state");
    // The explicit state directory was used, never one named by a value.
    assert!(f.state.join("hook.lock").is_file());
    assert_eq!(fs::read_dir(f.dir.join("cwd")).unwrap().count(), 0);
    assert!(!f.root.join("200000").exists());
}

/// The wire of one Claude report (design D3).
fn assert_claude_wire(params: &Value, seq: &str, window: u64) {
    let object = params.as_object().unwrap();
    let mut keys: Vec<_> = object.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(keys, ["agent", "pane_id", "seq", "source", "tokens"]);
    assert_eq!(params["agent"], "claude");
    assert_eq!(params["source"], "user:observatory");
    assert_eq!(params["pane_id"], "w1:p1");
    assert_eq!(params["seq"].to_string(), seq);
    let binding = common::sha256(format!("claude:id:{CLAUDE_ID}").as_bytes());
    let window = format!(",{window},,");
    let expected = json!({"obs_v":"2","obs_bind":binding,"obs_seq":seq,
        "obs_event":"session","obs_phase":"ready","obs_tool":null,"obs_model":null,
        "obs_result":null,"obs_usage_source":null,
        "obs_n0":",,,","obs_n1":window,"obs_n2":",,,","obs_n3":",",
        "obs_children":null,"obs_completion":null,"obs_outcomes":null});
    assert_eq!(params["tokens"], expected);
    assert_eq!(params["tokens"].as_object().unwrap().len(), 16);
}

/// D3: one `pane.get`, then one metadata write carrying only the window; a
/// repeat with the same window reads only; a new window writes again; an
/// older sequence is refused after the read.
#[test]
fn claude_report_writes_the_bound_window_once_and_repeats_read_only() {
    let f = Reporter::new();
    let seq = report_seq(30);
    assert_eq!(f.report(&["w1:p1", &seq, CLAUDE_ID, "200000"]), Some(0));
    assert_eq!(f.methods(), ["pane.get", "pane.report_metadata"]);
    assert_claude_wire(&f.writes()[0], &seq, 200_000);
    let text = f.writes()[0].to_string();
    for absent in [
        "display_agent",
        "usage_seq",
        "account",
        "rate",
        "model\":\"",
    ] {
        assert!(!text.contains(absent), "{absent} in {text}");
    }
    // The same window at a later sequence: read, no write, success.
    let repeat = report_seq(20);
    assert_eq!(f.report(&["w1:p1", &repeat, CLAUDE_ID, "200000"]), Some(0));
    assert_eq!(
        f.methods(),
        ["pane.get", "pane.report_metadata", "pane.get"]
    );
    // A changed window writes again.
    let changed = report_seq(10);
    assert_eq!(
        f.report(&["w1:p1", &changed, CLAUDE_ID, "1000000"]),
        Some(0)
    );
    assert_eq!(f.writes().len(), 2);
    assert_claude_wire(&f.writes()[1], &changed, 1_000_000);
    // A sequence not newer than the pane's `obs_seq` is refused.
    for old in [&changed, &seq] {
        f.calls.lock().unwrap().clear();
        assert_eq!(f.report(&["w1:p1", old, CLAUDE_ID, "200000"]), Some(3));
        assert_eq!(f.methods(), ["pane.get"]);
    }
}

/// D3: stdin is never read, so an open pipe cannot stall the report. The
/// full wire is asserted, so the case fails where `--report claude` is absent.
#[test]
fn claude_report_with_open_stdin_completes_the_write() {
    let f = Reporter::new();
    let seq = report_seq(10);
    let started = Instant::now();
    let mut child = f
        .command(&["w1:p1", &seq, CLAUDE_ID, "200000"])
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    let held = child.stdin.take();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if started.elapsed() > Duration::from_millis(1500) {
            let _ = child.kill();
            panic!("report blocked on open stdin");
        }
        thread::sleep(Duration::from_millis(10));
    };
    drop(held);
    assert_eq!(status.code(), Some(0));
    assert_eq!(f.methods(), ["pane.get", "pane.report_metadata"]);
    assert_claude_wire(&f.writes()[0], &seq, 200_000);
}

/// D3: an `env -i` run with only an absolute `HOME` succeeds, and an
/// explicit relative `--state` exits 3 with no file access in the cwd.
#[test]
fn claude_report_needs_only_home_and_refuses_relative_state() {
    let f = Reporter::new();
    let seq = report_seq(10);
    // `command` clears the environment and sets only `HOME`.
    assert_eq!(f.report(&["w1:p1", &seq, CLAUDE_ID, "200000"]), Some(0));
    assert_eq!(f.writes().len(), 1);
    f.calls.lock().unwrap().clear();
    let output = Command::new(BIN)
        .env_clear()
        .env("HOME", &f.home)
        .current_dir(f.dir.join("cwd"))
        .args([
            "--root",
            f.root.to_str().unwrap(),
            "--state",
            "relative/state",
        ])
        .args([
            "--report",
            "claude",
            "w1:p1",
            &report_seq(5),
            CLAUDE_ID,
            "300000",
        ])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty() && output.stderr.is_empty());
    assert!(f.methods().is_empty());
    assert_eq!(fs::read_dir(f.dir.join("cwd")).unwrap().count(), 0);
}

/// D3, as the mod runs it: the installed runtime with only an absolute
/// `HOME`, no `--root` or `--state` and an unrelated cwd derives its root
/// from its own path and its state from the home, and reports.
#[test]
fn claude_report_as_the_mod_runs_it_derives_root_and_state() {
    let f = Reporter::new();
    f.installable();
    f.install_step("--install-hooks");
    f.install_step("--install-claude-mod");
    f.calls.lock().unwrap().clear();
    let output = Command::new(f.root.join("anton-runtime"))
        .env_clear()
        .env("HOME", &f.home)
        .current_dir(f.dir.join("cwd"))
        .args(["--report", "claude", "w1:p1", &report_seq(10), CLAUDE_ID])
        .arg("200000")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(f.methods(), ["pane.get", "pane.report_metadata"]);
    let state = f.home.join(".local/state/herdr.observatory");
    assert!(state.join("hook.lock").is_file());
    assert!(!f.state.exists());
    assert_eq!(fs::read_dir(f.dir.join("cwd")).unwrap().count(), 0);
}

/// D3 (review round 2): the reporter expands a `~/` socket path against
/// its home, as the collector does, and refuses a relative one (exit 3, no
/// RPC), because it runs in the Claude session's working directory. A local
/// host without `socket_path` is not applicable either (review round 5).
#[test]
fn claude_report_expands_a_home_socket_and_refuses_a_relative_one() {
    let f = Reporter::new();
    let socket_config = |path: &str| {
        write(
            &f.root.join(".config.json"),
            serde_json::to_vec(&json!({"hosts":[{"id":"local","socket_path":path}]})).unwrap(),
            0o600,
        );
    };
    std::os::unix::fs::symlink(&f.socket, f.home.join("herdr.sock")).unwrap();
    socket_config("~/herdr.sock");
    assert_eq!(
        f.report(&["w1:p1", &report_seq(20), CLAUDE_ID, "200000"]),
        Some(0)
    );
    assert_eq!(f.methods(), ["pane.get", "pane.report_metadata"]);
    // A relative path, run from a directory that holds a socket of that name.
    socket_config("herdr.sock");
    f.calls.lock().unwrap().clear();
    let output = f
        .command(&["w1:p1", &report_seq(10), CLAUDE_ID, "300000"])
        .current_dir(&f.dir)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty() && output.stderr.is_empty());
    assert!(f.methods().is_empty());
    // Review round 5: a local host without `socket_path` (a `session` host,
    // or neither key, so Herdr's CLI default) has no socket to report to.
    // The Claude reporter exits 3 without output or RPC; Pi still exits 1.
    for host in [
        json!({"id":"local","session":"main"}),
        json!({"id":"local"}),
    ] {
        write(
            &f.root.join(".config.json"),
            serde_json::to_vec(&json!({"hosts":[host]})).unwrap(),
            0o600,
        );
        f.calls.lock().unwrap().clear();
        let output = f
            .command(&["w1:p1", &report_seq(30), CLAUDE_ID, "200000"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(3), "{host}");
        assert!(
            output.stdout.is_empty() && output.stderr.is_empty(),
            "{host}"
        );
        let mut pi = Command::new(BIN)
            .env_clear()
            .env("HOME", &f.home)
            .current_dir(f.dir.join("cwd"))
            .args(["--report", "pi", "w1:p1", &report_seq(30)])
            .args(["--root", f.root.to_str().unwrap()])
            .args(["--state", f.state.to_str().unwrap()])
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        pi.stdin
            .take()
            .unwrap()
            .write_all(br#"{"event":"turn","phase":"working"}"#)
            .unwrap();
        let pi = pi.wait_with_output().unwrap();
        assert_eq!(pi.status.code(), Some(1), "{host}");
        assert_eq!(
            String::from_utf8_lossy(&pi.stderr).trim(),
            "Missing local socket",
            "{host}"
        );
        assert!(f.methods().is_empty(), "{host}");
    }
}

/// D3: a pane that is not this exact Claude session, an older `obs_seq`,
/// no mod receipt, two local hosts or a busy hook lock exit 3 without a
/// write; a mod receipt mid-refresh is accepted.
#[test]
fn claude_report_refuses_unbound_panes_and_missing_ownership() {
    let f = Reporter::new();
    let seq = report_seq(10);
    let mut cases = Vec::new();
    let mut pane = claude_pane(CLAUDE_ID);
    pane["agent"] = json!("codex");
    cases.push(pane);
    let mut pane = claude_pane(CLAUDE_ID);
    pane["agent_session"]["agent"] = json!("pi");
    cases.push(pane);
    let mut pane = claude_pane(CLAUDE_ID);
    pane["agent_session"]["source"] = json!("user:claude");
    cases.push(pane);
    let mut pane = claude_pane(CLAUDE_ID);
    pane["agent_session"]["kind"] = json!("path");
    cases.push(pane);
    cases.push(claude_pane("fixture-other-session"));
    let mut pane = claude_pane(CLAUDE_ID);
    pane["tokens"] = json!({"obs_seq":report_seq(0)});
    cases.push(pane);
    for pane in cases {
        f.pane("w1:p1", pane.clone());
        f.calls.lock().unwrap().clear();
        assert_eq!(
            f.report(&["w1:p1", &seq, CLAUDE_ID, "200000"]),
            Some(3),
            "{pane}"
        );
        assert_eq!(f.methods(), ["pane.get"], "{pane}");
    }
    f.pane("w1:p1", claude_pane(CLAUDE_ID));
    // No receipt, a receipt without `claude_mod` and a malformed entry.
    let mut broken = f.mod_entry(false);
    broken["files"][0]["sha256"] = json!("not-a-hash");
    for entry in [None, Some(json!(null)), Some(broken)] {
        f.receipt(entry);
        f.calls.lock().unwrap().clear();
        assert_eq!(f.report(&["w1:p1", &seq, CLAUDE_ID, "200000"]), Some(3));
        assert!(f.methods().is_empty());
    }
    fs::remove_file(f.root.join(".hooks-receipt.json")).unwrap();
    assert_eq!(f.report(&["w1:p1", &seq, CLAUDE_ID, "200000"]), Some(3));
    assert!(f.methods().is_empty());
    // Two local hosts.
    f.receipt(Some(f.mod_entry(false)));
    f.config(2);
    assert_eq!(f.report(&["w1:p1", &seq, CLAUDE_ID, "200000"]), Some(3));
    assert!(f.methods().is_empty());
    f.config(1);
    // A busy hook lock, held past the reporter's 400 ms wait.
    assert!(f.state.is_dir());
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(f.state.join("hook.lock"))
        .unwrap();
    use std::os::fd::AsRawFd;
    use std::os::unix::fs::OpenOptionsExt;
    assert_eq!(unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX) }, 0);
    assert_eq!(f.report(&["w1:p1", &seq, CLAUDE_ID, "200000"]), Some(3));
    assert!(f.methods().is_empty());
    drop(lock);
    // A receipt entry still carrying `prior_sha256` from a refresh.
    f.receipt(Some(f.mod_entry(true)));
    assert_eq!(f.report(&["w1:p1", &seq, CLAUDE_ID, "200000"]), Some(0));
    assert_eq!(f.methods(), ["pane.get", "pane.report_metadata"]);
}

/// Claude account fixtures (design D4 to D6): a synthetic `.claude.json`
/// with `PRIVATE` markers in unlisted keys, and reports carrying a tail.
const CLAUDE_UUID: &str = "00000000-0000-4000-8000-000000000001";
const CLAUDE_EMAIL: &str = "fixture@example.invalid";
fn claude_account_key(uuid: &str) -> String {
    common::sha256(format!("observatory-claude-account-v1:{uuid}").as_bytes())
}
fn claude_session_key(id: &str) -> String {
    common::sha256(format!("observatory-claude-session-v1:{id}").as_bytes())
}
/// A synthetic `.claude.json` naming `uuid`, with `extra` top-level keys.
fn provider_body(uuid: &str, extra: &str) -> String {
    format!(
        r#"{{"projects":{{"/PRIVATE/path":{{"history":["PRIVATE"]}}}},"oauthAccount":{{"accountUuid":"{uuid}","emailAddress":"{CLAUDE_EMAIL}","organizationName":"PRIVATE"}},"cachedUsageUtilization":{{"accountUuid":"{uuid}","fetchedAtMs":1,"utilization":{{"spend":"PRIVATE"}}}}{extra}}}"#
    )
}
/// A Claude report at `seq` with `windows` as (kind, used, seconds after
/// the whole second of `seq`).
fn tail_report(
    pane: &str,
    session: &str,
    seq: u64,
    window: u64,
    windows: &[(&str, &str, u64)],
) -> Vec<String> {
    let whole = seq / 1_000_000;
    let mut values = vec![
        pane.to_owned(),
        seq.to_string(),
        session.to_owned(),
        window.to_string(),
    ];
    for (kind, used, after) in windows {
        values.extend([
            kind.to_string(),
            used.to_string(),
            (whole + after).to_string(),
        ]);
    }
    values
}
/// Environment pairs for `Reporter::account_report`.
fn vars(pairs: &[(&str, &str)]) -> Vec<(std::ffi::OsString, std::ffi::OsString)> {
    pairs
        .iter()
        .map(|(name, value)| (name.into(), value.into()))
        .collect()
}
/// Asserts that no fixture secret appears in `bytes`.
fn assert_no_secret(bytes: &[u8]) {
    let text = String::from_utf8_lossy(bytes);
    for secret in ["PRIVATE", CLAUDE_UUID, CLAUDE_EMAIL] {
        assert!(!text.contains(secret), "{secret} in {text}");
    }
}
impl Reporter {
    fn provider(&self, body: &str) {
        write(&self.home.join(".claude.json"), body, 0o600);
    }
    fn settings(&self, body: &str) {
        fs::create_dir_all(self.home.join(".claude")).unwrap();
        write(&self.home.join(".claude/settings.json"), body, 0o644);
    }
    fn account_path(&self) -> PathBuf {
        self.state.join("claude-allowances.json")
    }
    fn account_state(&self) -> Option<Value> {
        fs::read(self.account_path())
            .ok()
            .map(|bytes| serde_json::from_slice(&bytes).unwrap())
    }
    /// Runs one report with `env` added to the fixture environment. Exit 0
    /// has no output; no output and no state file ever holds a fixture
    /// secret.
    fn account_report(
        &self,
        report: &[String],
        env: &[(std::ffi::OsString, std::ffi::OsString)],
    ) -> Option<i32> {
        let report: Vec<&str> = report.iter().map(String::as_str).collect();
        let mut command = self.command(&report);
        command.envs(env.iter().map(|(name, value)| (name, value)));
        let output = command.output().unwrap();
        let code = output.status.code();
        if code == Some(0) {
            assert!(output.stdout.is_empty(), "{report:?}");
            assert!(output.stderr.is_empty(), "{report:?}");
        }
        assert_no_secret(&output.stdout);
        assert_no_secret(&output.stderr);
        if let Ok(entries) = fs::read_dir(&self.state) {
            for entry in entries {
                let path = entry.unwrap().path();
                if fs::symlink_metadata(&path).unwrap().is_file() {
                    assert_no_secret(&fs::read(&path).unwrap());
                }
            }
        }
        code
    }
}

/// D4 to D6: a ten-value report writes the bound window with the change 3
/// wire and records both windows for the profile's account; a later tail
/// with the same window still records, a window absent from it keeps its
/// value and stamp, and a four-value run leaves the account state alone.
#[test]
fn claude_report_records_fresh_windows_for_the_profile_account() {
    let f = Reporter::new();
    f.provider(&provider_body(CLAUDE_UUID, ""));
    let first = (common::now() as u64 - 100) * 1_000_000;
    let report = tail_report(
        "w1:p1",
        CLAUDE_ID,
        first,
        200_000,
        &[("five_hour", "12.5", 60), ("seven_day", "40", 3600)],
    );
    assert_eq!(f.account_report(&report, &[]), Some(0));
    assert_eq!(f.methods(), ["pane.get", "pane.report_metadata"]);
    assert_claude_wire(&f.writes()[0], &first.to_string(), 200_000);
    let text = f.writes()[0].to_string();
    for absent in ["account", "rate", "five_hour", "seven_day", "12.5"] {
        assert!(!text.contains(absent), "{absent} in {text}");
    }
    let key = claude_account_key(CLAUDE_UUID);
    let session = claude_session_key(CLAUDE_ID);
    let whole = first / 1_000_000;
    let at = first as f64 / 1e6;
    assert_eq!(
        f.account_state().unwrap(),
        json!({"version":1,"accounts":[{"account_key":key,"windows":{
            "five_hour":{"used_percent":12.5,"resets_at":whole + 60,"sampled_at":at},
            "seven_day":{"used_percent":40,"resets_at":whole + 3600,"sampled_at":at}}}],
            "sessions":[{"session":session,"account_key":key,"at":at}]})
    );
    let bytes = fs::read(f.account_path()).unwrap();
    assert!(String::from_utf8_lossy(&bytes).contains(r#""used_percent":40,"#));
    use std::os::unix::fs::MetadataExt;
    assert_eq!(
        fs::metadata(f.account_path()).unwrap().mode() & 0o777,
        0o600
    );
    // The same window later: no metadata write, the account state still
    // records; `five_hour`, absent from the tail, keeps its value and stamp.
    let second = first + 10_000_000;
    let report = tail_report(
        "w1:p1",
        CLAUDE_ID,
        second,
        200_000,
        &[("seven_day", "41.5", 3600)],
    );
    assert_eq!(f.account_report(&report, &[]), Some(0));
    assert_eq!(
        f.methods(),
        ["pane.get", "pane.report_metadata", "pane.get"]
    );
    let state = f.account_state().unwrap();
    let later = second as f64 / 1e6;
    assert_eq!(
        state["accounts"][0]["windows"]["five_hour"],
        json!({"used_percent":12.5,"resets_at":whole + 60,"sampled_at":at})
    );
    assert_eq!(
        state["accounts"][0]["windows"]["seven_day"],
        json!({"used_percent":41.5,"resets_at":second / 1_000_000 + 3600,"sampled_at":later})
    );
    assert_eq!(
        state["sessions"],
        json!([{"session":session,"account_key":key,"at":later}])
    );
    // Four values: the window report only; the account state is unchanged.
    let before = fs::read(f.account_path()).unwrap();
    let third = (second + 10_000_000).to_string();
    assert_eq!(f.report(&["w1:p1", &third, CLAUDE_ID, "300000"]), Some(0));
    assert_claude_wire(&f.writes()[1], &third, 300_000);
    assert_eq!(fs::read(f.account_path()).unwrap(), before);
}

/// D4: every refusal keeps the window report and its exit status and
/// writes no account state, deciding on names, never values; the accepted
/// neighbours, each differing only in the refused property, write it.
#[test]
fn claude_report_refusals_keep_the_window_report_and_write_no_account_state() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    let f = Reporter::new();
    let good = provider_body(CLAUDE_UUID, "");
    f.provider(&good);
    let mut seq = (common::now() as u64 - 300) * 1_000_000;
    let mut window = 200_000;
    // One report with a tail; the window changes every second run, so the
    // account step follows both a metadata write and the no-change return.
    // True when account state was written.
    let mut run = |env: &[(std::ffi::OsString, std::ffi::OsString)]| -> bool {
        seq += 1_000_000;
        if (seq / 1_000_000) % 2 == 0 {
            window = if window == 200_000 { 300_000 } else { 200_000 };
        }
        let report = tail_report(
            "w1:p1",
            CLAUDE_ID,
            seq,
            window,
            &[("five_hour", "12.5", 60), ("seven_day", "40", 3600)],
        );
        assert_eq!(f.account_report(&report, env), Some(0), "{env:?}");
        assert_eq!(
            f.panes.lock().unwrap()["w1:p1"]["tokens"]["obs_n1"],
            format!(",{window},,")
        );
        let written = f.account_path().exists();
        let _ = fs::remove_file(f.account_path());
        written
    };
    let home_claude = f.home.join(".claude").to_string_lossy().into_owned();
    let elsewhere = f.dir.join("PRIVATE-config").to_string_lossy().into_owned();
    for env in [
        vars(&[("ANTHROPIC_BASE_URL", "https://PRIVATE.example.invalid/v1")]),
        vars(&[("ANTHROPIC_BASE_URL", "")]),
        vars(&[("ANTHROPIC_API_KEY", "PRIVATE")]),
        vars(&[("ANTHROPIC_AUTH_TOKEN", "PRIVATE")]),
        vars(&[("ANTHROPIC_CUSTOM_HEADERS", "PRIVATE")]),
        vars(&[
            ("CLAUDE_CODE_MESSAGING_TOKEN", "PRIVATE"),
            ("CLAUDE_CODE_SESSION_TOKEN", "PRIVATE"),
        ]),
        vars(&[("CLAUDE_CODE_OAUTH_TOKEN_FILE_DESCRIPTOR", "3")]),
        vars(&[("CLAUDE_CODE_HOST_PLATFORM", "PRIVATE")]),
        vars(&[("CLAUDE_CODE_USE_BEDROCK", "1")]),
        vars(&[("CCR_OAUTH_TOKEN_FILE", "PRIVATE")]),
        vars(&[("CLAUDE_CODE_CUSTOM_OAUTH_URL", "PRIVATE")]),
        vec![(OsString::from_vec(b"FIXTURE_\xff".to_vec()), "1".into())],
        vars(&[("CLAUDE_CONFIG_DIR", "")]),
        vars(&[("CLAUDE_CONFIG_DIR", home_claude.as_str())]),
        vars(&[("CLAUDE_CONFIG_DIR", elsewhere.as_str())]),
    ] {
        assert!(!run(&env), "{env:?}");
    }
    for env in [
        vars(&[]),
        vars(&[("CLAUDE_CODE_MESSAGING_TOKEN", "PRIVATE")]),
        vars(&[("ANTHROPIC_BASE_URL_X", "PRIVATE")]),
        vars(&[("ANTHROPIC_MODEL", "PRIVATE")]),
        vec![(
            OsString::from("FIXTURE_NAME"),
            OsString::from_vec(b"\xff".to_vec()),
        )],
    ] {
        assert!(run(&env), "{env:?}");
    }
    // Legacy configuration as a file and as a dangling link.
    fs::create_dir_all(f.home.join(".claude")).unwrap();
    let legacy = f.home.join(".claude/.config.json");
    write(&legacy, "{}", 0o600);
    assert!(!run(&[]));
    fs::remove_file(&legacy).unwrap();
    std::os::unix::fs::symlink(f.dir.join("absent"), &legacy).unwrap();
    assert!(!run(&[]));
    fs::remove_file(&legacy).unwrap();
    assert!(run(&[]));
    // `apiKeyHelper` with any value, and unsafe or malformed settings.
    for body in [
        r#"{"apiKeyHelper":"/PRIVATE/helper"}"#,
        r#"{"apiKeyHelper":null}"#,
        r#"{"apiKeyHelper":{"PRIVATE":[1]}}"#,
        "{PRIVATE",
        "[]",
    ] {
        f.settings(body);
        assert!(!run(&[]), "{body}");
    }
    f.settings(&format!(r#"{{"pad":"{}"}}"#, "x".repeat(1_048_576)));
    assert!(!run(&[]));
    let settings = f.home.join(".claude/settings.json");
    fs::remove_file(&settings).unwrap();
    write(&f.dir.join("real-settings.json"), "{}", 0o644);
    std::os::unix::fs::symlink(f.dir.join("real-settings.json"), &settings).unwrap();
    assert!(!run(&[]));
    fs::remove_file(&settings).unwrap();
    f.settings(r#"{"env":{"PRIVATE":"PRIVATE"},"model":"PRIVATE"}"#);
    assert!(run(&[]));
    fs::remove_file(&settings).unwrap();
    // Provider state: `primaryApiKey` with any value, no valid account,
    // mistyped account id, a loose mode, a link, over 4 MiB, missing.
    for body in [
        provider_body(CLAUDE_UUID, r#","primaryApiKey":"PRIVATE""#),
        provider_body(CLAUDE_UUID, r#","primaryApiKey":null"#),
        r#"{"oauthAccount":{"emailAddress":"PRIVATE"}}"#.to_owned(),
        r#"{"oauthAccount":{"accountUuid":""}}"#.to_owned(),
        r#"{"oauthAccount":{"accountUuid":["PRIVATE"]}}"#.to_owned(),
        "PRIVATE".to_owned(),
        provider_body(
            CLAUDE_UUID,
            &format!(r#","pad":"{}""#, "x".repeat(4 * 1_048_576)),
        ),
    ] {
        f.provider(&body);
        assert!(!run(&[]));
    }
    let provider = f.home.join(".claude.json");
    write(&provider, &good, 0o644);
    assert!(!run(&[]));
    fs::remove_file(&provider).unwrap();
    write(&f.dir.join("real-provider.json"), &good, 0o600);
    std::os::unix::fs::symlink(f.dir.join("real-provider.json"), &provider).unwrap();
    assert!(!run(&[]));
    fs::remove_file(&provider).unwrap();
    assert!(!run(&[]));
    // A mistyped field outside the reporter's extraction is never read.
    f.provider(&provider_body(
        CLAUDE_UUID,
        r#","oauthAccount2":1,"cachedUsageUtilization":{"accountUuid":7,"fetchedAtMs":"PRIVATE"}"#,
    ));
    assert!(run(&[]));
    f.provider(&good);
    assert!(run(&[]));
}

/// D6: newest stamp wins across two panes bound to two sessions on one
/// account, also when the older report arrives second; a session that finds
/// another account is refused for good, while other sessions attribute.
#[test]
fn claude_report_account_state_keeps_newest_stamps_and_refuses_switched_sessions() {
    let f = Reporter::new();
    f.provider(&provider_body(CLAUDE_UUID, ""));
    let (id_b, id_c) = ("fixture-session-b", "fixture-session-c");
    for (pane, id) in [("w1:p2", id_b), ("w1:p3", id_c)] {
        let mut value = claude_pane(id);
        value["pane_id"] = json!(pane);
        f.pane(pane, value);
    }
    let key = claude_account_key(CLAUDE_UUID);
    let (a, b) = (claude_session_key(CLAUDE_ID), claude_session_key(id_b));
    let base = (common::now() as u64 - 100) * 1_000_000;
    let (older, newer) = (base, base + 5_000_000);
    let at = |seq: u64| seq as f64 / 1e6;
    let window = |used: u64, seq: u64, after: u64| json!({"used_percent":used,"resets_at":seq / 1_000_000 + after,"sampled_at":at(seq)});
    let report = tail_report("w1:p2", id_b, newer, 200_000, &[("five_hour", "20", 60)]);
    assert_eq!(f.account_report(&report, &[]), Some(0));
    let report = tail_report(
        "w1:p1",
        CLAUDE_ID,
        older,
        200_000,
        &[("five_hour", "10", 60), ("seven_day", "5", 3600)],
    );
    assert_eq!(f.account_report(&report, &[]), Some(0));
    let state = f.account_state().unwrap();
    assert_eq!(
        state["accounts"],
        json!([{"account_key":key,"windows":{"five_hour":window(20, newer, 60),"seven_day":window(5, older, 3600)}}])
    );
    assert_eq!(
        state["sessions"],
        json!([{"session":b,"account_key":key,"at":at(newer)},{"session":a,"account_key":key,"at":at(older)}])
    );
    let accounts = state["accounts"].clone();
    // The profile now names another account: session `a` is refused.
    let other = "00000000-0000-4000-8000-000000000002";
    f.provider(&provider_body(other, ""));
    let switch = base + 10_000_000;
    let report = tail_report(
        "w1:p1",
        CLAUDE_ID,
        switch,
        200_000,
        &[("five_hour", "30", 60)],
    );
    assert_eq!(f.account_report(&report, &[]), Some(0));
    let state = f.account_state().unwrap();
    assert_eq!(state["accounts"], accounts);
    assert_eq!(
        state["sessions"][1],
        json!({"session":a,"account_key":null,"at":at(switch)})
    );
    // Back on the first account, session `a` stays refused.
    f.provider(&provider_body(CLAUDE_UUID, ""));
    let back = base + 15_000_000;
    let report = tail_report(
        "w1:p1",
        CLAUDE_ID,
        back,
        200_000,
        &[("five_hour", "40", 60)],
    );
    assert_eq!(f.account_report(&report, &[]), Some(0));
    let state = f.account_state().unwrap();
    assert_eq!(state["accounts"], accounts);
    assert_eq!(
        state["sessions"][1],
        json!({"session":a,"account_key":null,"at":at(back)})
    );
    // Session `b` still attributes to the first account.
    let later = base + 20_000_000;
    let report = tail_report("w1:p2", id_b, later, 200_000, &[("five_hour", "50", 60)]);
    assert_eq!(f.account_report(&report, &[]), Some(0));
    let state = f.account_state().unwrap();
    assert_eq!(
        state["accounts"][0]["windows"]["five_hour"],
        window(50, later, 60)
    );
    // A new session under the other account attributes normally.
    f.provider(&provider_body(other, ""));
    let last = base + 25_000_000;
    let report = tail_report("w1:p3", id_c, last, 200_000, &[("seven_day", "60", 3600)]);
    assert_eq!(f.account_report(&report, &[]), Some(0));
    let state = f.account_state().unwrap();
    assert_eq!(
        state["accounts"][1],
        json!({"account_key":claude_account_key(other),"windows":{"seven_day":window(60, last, 3600)}})
    );
    assert_eq!(state["sessions"].as_array().unwrap().len(), 3);
}

/// D6: at most four accounts (the least recently stamped evicted) and 32
/// sessions (expired ones dropped, then attributed ones oldest first,
/// refused ones last); a malformed, oversized, other-version or unknown-key
/// owned file is replaced; a link or a loose mode refuses the write.
#[test]
fn claude_report_account_state_bounds_and_replacement() {
    let f = Reporter::new();
    f.provider(&provider_body(CLAUDE_UUID, ""));
    fs::create_dir_all(&f.state).unwrap();
    fs::set_permissions(&f.state, fs::Permissions::from_mode(0o700)).unwrap();
    let now = common::now() as u64;
    let key = claude_account_key(CLAUDE_UUID);
    let session = claude_session_key(CLAUDE_ID);
    let mut seq = (now - 200) * 1_000_000;
    let mut report = || {
        seq += 1_000_000;
        let report = tail_report(
            "w1:p1",
            CLAUDE_ID,
            seq,
            200_000,
            &[("five_hour", "12.5", 60)],
        );
        assert_eq!(f.account_report(&report, &[]), Some(0));
        seq
    };
    let stamp = |t: u64| json!({"used_percent":1,"resets_at":now + 600,"sampled_at":t as f64});
    let accounts: Vec<Value> = (0..4u64)
        .map(|i| json!({"account_key":hex(&format!("account-{i}")),"windows":{"seven_day":stamp(now - 400 + i * 10)}}))
        .collect();
    let refused: Vec<Value> = (0..4u64)
        .map(|i| json!({"session":hex(&format!("refused-{i}")),"account_key":null,"at":(now - 3000 + i) as f64}))
        .collect();
    let attributed: Vec<Value> = (0..28u64)
        .map(|i| json!({"session":hex(&format!("attributed-{i}")),"account_key":hex("account-1"),"at":(now - 2000 + i) as f64}))
        .collect();
    let sessions = [refused.clone(), attributed.clone()].concat();
    let seeded = json!({"version":1,"accounts":accounts,"sessions":sessions});
    write(&f.account_path(), seeded.to_string(), 0o600);
    let first = report();
    let state = f.account_state().unwrap();
    let keys: Vec<_> = state["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["account_key"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(
        keys,
        [
            hex("account-1"),
            hex("account-2"),
            hex("account-3"),
            key.clone()
        ]
    );
    let mut expected = [refused.clone(), attributed[1..].to_vec()].concat();
    expected.push(json!({"session":session,"account_key":key,"at":first as f64 / 1e6}));
    assert_eq!(state["sessions"], json!(expected));
    // A session older than 24 hours is dropped on the next report.
    let expired =
        json!({"session":hex("expired"),"account_key":hex("account-1"),"at":(now - 90_000) as f64});
    let recent = json!({"session":hex("recent"),"account_key":null,"at":(now - 1000) as f64});
    write(
        &f.account_path(),
        json!({"version":1,"accounts":[],"sessions":[expired, recent]}).to_string(),
        0o600,
    );
    let second = report();
    assert_eq!(
        f.account_state().unwrap()["sessions"],
        json!([recent, {"session":session,"account_key":key,"at":second as f64 / 1e6}])
    );
    // Owned private files that are malformed, oversized, of another
    // version or with an unknown key are replaced.
    for body in [
        "PRIVATE{".to_owned(),
        format!(r#"{{"pad":"{}"}}"#, "x".repeat(16_384)),
        r#"{"version":2,"accounts":[],"sessions":[]}"#.to_owned(),
        r#"{"version":1,"accounts":[],"sessions":[],"extra":"PRIVATE"}"#.to_owned(),
    ] {
        write(&f.account_path(), &body, 0o600);
        let seq = report();
        let state = f.account_state().unwrap();
        assert_eq!(state["accounts"][0]["account_key"], key);
        assert_eq!(
            state["sessions"],
            json!([{"session":session,"account_key":key,"at":seq as f64 / 1e6}])
        );
    }
    // A link or a loose mode refuses the write and is left as it was.
    fs::remove_file(f.account_path()).unwrap();
    write(&f.dir.join("state-target.json"), "{}", 0o600);
    std::os::unix::fs::symlink(f.dir.join("state-target.json"), f.account_path()).unwrap();
    report();
    assert!(fs::symlink_metadata(f.account_path()).unwrap().is_symlink());
    assert_eq!(fs::read(f.dir.join("state-target.json")).unwrap(), b"{}");
    fs::remove_file(f.account_path()).unwrap();
    let loose = json!({"version":1,"accounts":[],"sessions":[]}).to_string();
    write(&f.account_path(), &loose, 0o644);
    report();
    assert_eq!(fs::read(f.account_path()).unwrap(), loose.as_bytes());
}

/// D4: when the metadata write fails the reporter exits 1 and writes no
/// account state; the same report once the write succeeds records it.
#[test]
fn claude_report_failed_metadata_write_writes_no_account_state() {
    let f = Reporter::new();
    f.provider(&provider_body(CLAUDE_UUID, ""));
    let mut pane = claude_pane(CLAUDE_ID);
    pane["fixture_reject"] = json!(true);
    f.pane("w1:p1", pane);
    let seq = (common::now() as u64 - 10) * 1_000_000;
    let report = tail_report(
        "w1:p1",
        CLAUDE_ID,
        seq,
        200_000,
        &[("five_hour", "12.5", 60)],
    );
    assert_eq!(f.account_report(&report, &[]), Some(1));
    assert_eq!(f.methods(), ["pane.get", "pane.report_metadata"]);
    assert!(!f.account_path().exists());
    f.pane("w1:p1", claude_pane(CLAUDE_ID));
    assert_eq!(f.account_report(&report, &[]), Some(0));
    assert_eq!(
        f.account_state().unwrap()["accounts"][0]["account_key"],
        claude_account_key(CLAUDE_UUID)
    );
}

/// D4: a ten-value report reading a provider state file at its 4 MiB
/// bound holds `hook.lock` briefly enough that a Pi report on the same
/// state directory, started while the Claude run holds the lock (its
/// metadata reply is delayed by 100 ms, then the account step runs),
/// completes within its 400 ms lock wait.
#[test]
fn claude_report_with_large_provider_state_lets_a_pi_report_through() {
    let f = Reporter::new();
    let small = provider_body(CLAUDE_UUID, r#","pad":"""#);
    let pad = "x".repeat(4 * 1_048_576 - small.len());
    f.provider(&provider_body(CLAUDE_UUID, &format!(r#","pad":"{pad}""#)));
    assert_eq!(
        fs::metadata(f.home.join(".claude.json")).unwrap().len(),
        4 * 1_048_576
    );
    let mut pane = claude_pane(CLAUDE_ID);
    pane["fixture_delay_ms"] = json!(100);
    f.pane("w1:p1", pane);
    f.pane(
        "w1:p2",
        json!({"pane_id":"w1:p2","agent":"pi","agent_session":{"agent":"pi","source":"herdr:pi","kind":"path","value":"/synthetic/session"},"tokens":{}}),
    );
    for round in 0..3u64 {
        let seq = (common::now() as u64 - 30 + round * 5) * 1_000_000;
        let report = tail_report(
            "w1:p1",
            CLAUDE_ID,
            seq,
            200_000 + round,
            &[("five_hour", "12.5", 60), ("seven_day", "40", 3600)],
        );
        let report: Vec<&str> = report.iter().map(String::as_str).collect();
        f.calls.lock().unwrap().clear();
        let mut claude = f.command(&report).spawn().unwrap();
        // Wait until the Claude run is inside `hook.lock`: its metadata
        // write has reached the socket and its reply is being delayed.
        let deadline = Instant::now() + Duration::from_secs(2);
        while !f.methods().contains(&"pane.report_metadata".to_owned()) {
            assert!(Instant::now() < deadline, "{round}");
            thread::sleep(Duration::from_millis(1));
        }
        assert!(claude.try_wait().unwrap().is_none(), "{round}");
        let mut pi = Command::new(BIN)
            .env_clear()
            .env("HOME", &f.home)
            .current_dir(f.dir.join("cwd"))
            .args(["--report", "pi", "w1:p2", &seq.to_string()])
            .args([
                "--root",
                f.root.to_str().unwrap(),
                "--state",
                f.state.to_str().unwrap(),
            ])
            .stdin(Stdio::piped())
            .spawn()
            .unwrap();
        let event = json!({"session_path":"/synthetic/session","event":"turn","phase":"working"});
        pi.stdin
            .take()
            .unwrap()
            .write_all(event.to_string().as_bytes())
            .unwrap();
        assert!(claude.wait().unwrap().success(), "{round}");
        assert!(pi.wait().unwrap().success(), "{round}");
        // Pi read its pane only after the Claude run released the lock.
        let calls = f.calls.lock().unwrap().clone();
        let order: Vec<_> = calls
            .iter()
            .map(|(method, params)| (method.as_str(), params["pane_id"].as_str().unwrap_or("")))
            .collect();
        assert_eq!(
            order,
            [
                ("pane.get", "w1:p1"),
                ("pane.report_metadata", "w1:p1"),
                ("pane.get", "w1:p2"),
                ("pane.report_metadata", "w1:p2")
            ],
            "{round}"
        );
        let state = f.account_state().unwrap();
        assert_eq!(
            state["accounts"][0]["windows"]["five_hour"]["sampled_at"],
            json!(seq as f64 / 1e6)
        );
    }
}

/// Regression guard: `--report pi` still parses options after its values
/// and reads its event from stdin.
#[test]
fn pi_report_option_parsing_is_unchanged() {
    let f = Reporter::new();
    f.pane(
        "w1:p1",
        json!({"agent":"pi","agent_session":{"agent":"pi","source":"herdr:pi","kind":"path","value":"/synthetic/session"},"tokens":{}}),
    );
    let mut child = Command::new(BIN)
        .env_clear()
        .env("HOME", &f.home)
        .current_dir(f.dir.join("cwd"))
        .args(["--report", "pi", "w1:p1", &report_seq(10)])
        .args([
            "--root",
            f.root.to_str().unwrap(),
            "--state",
            f.state.to_str().unwrap(),
        ])
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    let event = json!({"session_path":"/synthetic/session","event":"turn","phase":"working"});
    child
        .stdin
        .take()
        .unwrap()
        .write_all(event.to_string().as_bytes())
        .unwrap();
    assert!(child.wait().unwrap().success());
    assert_eq!(f.methods(), ["pane.get", "pane.report_metadata"]);
    let write = &f.writes()[0];
    assert_eq!(write["agent"], "pi");
    assert_eq!(write["display_agent"], "pi · working");
}

/// Installer CLI fixtures for the Claude Code mod (design D5, D7). Each run
/// has `env_clear()`, an absolute temporary `HOME`, a `PATH` holding only the
/// fixture `bin` directory (so the host's `chezmoi` and `mise` never run) and
/// a private working directory.
impl Reporter {
    /// A real runtime in the plugin root, `~/.claude` and an empty `bin`, with
    /// the synthetic receipt removed so the installers write their own.
    fn installable(&self) {
        fs::remove_file(self.root.join(".hooks-receipt.json")).unwrap();
        // Copied by a child process for the reason `support::write_executable` gives.
        assert!(
            Command::new("cp")
                .arg("--")
                .arg(BIN)
                .arg(self.root.join("anton-runtime"))
                .status()
                .unwrap()
                .success()
        );
        fs::set_permissions(
            self.root.join("anton-runtime"),
            fs::Permissions::from_mode(0o700),
        )
        .unwrap();
        for path in [self.home.join(".claude"), self.dir.join("bin")] {
            fs::create_dir(&path).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        }
    }
    fn installer(&self, mode: &str) -> Command {
        let mut command = Command::new(BIN);
        command
            .env_clear()
            .env("HOME", &self.home)
            .env("PATH", self.dir.join("bin"))
            // Keeps the host's `/etc/mise` out of the mise declaration read.
            .env("MISE_SYSTEM_CONFIG_DIR", self.dir.join("etc-mise"))
            .current_dir(self.dir.join("cwd"))
            .args([
                "--root",
                self.root.to_str().unwrap(),
                "--state",
                self.state.to_str().unwrap(),
                mode,
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command
    }
    /// Runs one installer command and asserts it succeeded.
    fn install_step(&self, mode: &str) {
        let output = self.installer(mode).output().unwrap();
        assert!(
            output.status.success(),
            "{mode}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    fn mod_root(&self) -> PathBuf {
        self.home.join(".claude/skills/anton-observatory")
    }
}
/// Every entry under and including `dir`, keyed by path: kind, bytes (regular
/// files only), inode and mode. Directories are recorded, so a comparison sees
/// one created or removed.
fn file_tree(dir: &Path) -> std::collections::BTreeMap<PathBuf, (&'static str, Vec<u8>, u64, u32)> {
    use std::os::unix::fs::MetadataExt;
    let mut entries = std::collections::BTreeMap::new();
    let mut pending = vec![dir.to_owned()];
    while let Some(path) = pending.pop() {
        let info = fs::symlink_metadata(&path).unwrap();
        let kind = if info.is_dir() {
            for entry in fs::read_dir(&path).unwrap() {
                pending.push(entry.unwrap().path());
            }
            "directory"
        } else if info.is_file() {
            "file"
        } else if info.file_type().is_symlink() {
            "symlink"
        } else {
            "other"
        };
        let bytes = if kind == "file" {
            fs::read(&path).unwrap()
        } else {
            Vec::new()
        };
        entries.insert(path, (kind, bytes, info.ino(), info.mode()));
    }
    entries
}

/// D3 guard 2 and D5 agree: a mod installed through the CLI is accepted by
/// `--report claude`, and after `--uninstall-claude-mod` it is refused.
#[test]
fn claude_mod_installed_by_the_cli_is_accepted_by_the_reporter() {
    let f = Reporter::new();
    f.installable();
    f.install_step("--install-hooks");
    f.install_step("--install-claude-mod");
    let register = fs::read_to_string(f.mod_root().join("hooks/register.js")).unwrap();
    assert!(register.contains(&format!(
        "const nativeRuntime = {};",
        json!(f.root.join("anton-runtime"))
    )));
    for name in [".claude-plugin/plugin.json", "hooks/hooks.json"] {
        assert!(f.mod_root().join(name).is_file(), "{name}");
    }
    assert_eq!(
        f.report(&["w1:p1", &report_seq(30), CLAUDE_ID, "200000"]),
        Some(0)
    );
    assert_eq!(f.methods(), ["pane.get", "pane.report_metadata"]);
    let before = file_tree(&f.dir);
    f.install_step("--install-claude-mod");
    assert_eq!(
        file_tree(&f.dir),
        before,
        "an identical reinstall writes nothing"
    );
    f.install_step("--uninstall-claude-mod");
    assert!(!f.home.join(".claude/skills").exists());
    assert!(f.home.join(".pi/agent/extensions/observatory.ts").exists());
    f.calls.lock().unwrap().clear();
    assert_eq!(
        f.report(&["w1:p1", &report_seq(20), CLAUDE_ID, "200000"]),
        Some(3)
    );
    assert!(f.methods().is_empty(), "no RPC without a mod receipt");
    f.install_step("--install-claude-mod");
    f.install_step("--uninstall-hooks");
    assert!(!f.home.join(".claude/skills").exists());
    assert!(!f.root.join(".hooks-receipt.json").exists());
}

/// D5 removal step 2 (review round 6): every kept recorded directory is
/// reported on stderr, including a recorded `~/.claude/skills` that now
/// holds another skill, and the removal still succeeds.
#[test]
fn claude_mod_removal_reports_a_kept_skills_directory() {
    let f = Reporter::new();
    f.installable();
    f.install_step("--install-hooks");
    for mode in ["--uninstall-claude-mod", "--uninstall-hooks"] {
        f.install_step("--install-claude-mod");
        let skills = f.home.join(".claude/skills");
        let other = skills.join("other/SKILL.md");
        fs::create_dir_all(other.parent().unwrap()).unwrap();
        fs::write(&other, "another skill").unwrap();
        let output = f.installer(mode).output().unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "{mode}: {stderr}");
        assert!(
            stderr.contains(&format!("Kept {}:", skills.display())),
            "{mode}: {stderr}"
        );
        assert_eq!(fs::read_to_string(&other).unwrap(), "another skill");
        assert!(!f.mod_root().exists(), "{mode}");
        fs::remove_dir_all(&skills).unwrap();
        if mode == "--uninstall-hooks" {
            assert!(!f.root.join(".hooks-receipt.json").exists());
        }
    }
}

/// D1: the CLI reads `CLAUDE_CONFIG_DIR` from its own environment, and a
/// value naming another configuration refuses the mod and changes nothing.
#[test]
fn claude_mod_install_refuses_another_claude_config_dir_through_the_cli() {
    let f = Reporter::new();
    f.installable();
    f.install_step("--install-hooks");
    let before = file_tree(&f.dir);
    let output = f
        .installer("--install-claude-mod")
        .env("CLAUDE_CONFIG_DIR", f.dir.join("other-claude"))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("CLAUDE_CONFIG_DIR"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(file_tree(&f.dir), before);
    assert!(!f.home.join(".claude/skills").exists());
    // Naming `~/.claude` itself is accepted.
    let output = f
        .installer("--install-claude-mod")
        .env("CLAUDE_CONFIG_DIR", f.home.join(".claude"))
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(f.mod_root().join("hooks/register.js").is_file());
}

/// D5: every mise run (`dotfiles paths --json`, then `config get -f <file>`
/// for each config file it could load) runs from the home directory with a
/// null stdin, whatever the installer's own working directory holds and
/// whatever stdin the installer itself has (here an open pipe). A sibling
/// declaration does not refuse.
#[test]
fn claude_mod_install_runs_mise_from_the_home_with_null_stdin() {
    let f = Reporter::new();
    f.installable();
    f.install_step("--install-hooks");
    write(&f.dir.join("cwd/mise.toml"), b"[tools]\n", 0o600);
    let config = f.home.join(".config/mise/config.toml");
    fs::create_dir_all(config.parent().unwrap()).unwrap();
    write(
        &config,
        b"[dotfiles.\"~/.claude/skills/other\"]\nmode = \"copy\"\n",
        0o600,
    );
    let log = f.dir.join("mise.log");
    let home = f.home.to_str().unwrap();
    support::write_executable(
        &f.dir.join("bin/mise"),
        format!(
            "#!/bin/sh\n{{ printf '%s ' \"$@\"; printf '| %s | ' \"$(pwd)\"; if [ /proc/$$/fd/0 -ef /dev/null ]; then echo null; else echo other; fi; }} >> '{}'\ncase \"$3\" in\ndotfiles) printf '%s' '{{\"entries\":[{{\"path\":\"~/.claude/skills/other\"}}]}}' ;;\nconfig) case \"$6\" in '{home}'/*) while IFS= read -r line; do printf '%s\\n' \"$line\"; done < \"$6\" ;; esac ;;\n*) exit 1 ;;\nesac\n",
            log.display()
        )
        .as_bytes(),
        0o700,
    );
    // The installer's own stdin is a pipe held open here, so only the
    // explicit null stdin of the mise run can make the fake print `null`.
    let mut installer = f
        .installer("--install-claude-mod")
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    let held = installer.stdin.take();
    let output = installer.wait_with_output().unwrap();
    drop(held);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let calls = fs::read_to_string(&log).unwrap();
    let calls: Vec<&str> = calls.lines().collect();
    assert_eq!(
        calls[0],
        format!("-C {home} dotfiles paths --json | {home} | null")
    );
    for call in &calls[1..] {
        assert!(
            call.starts_with(&format!("-C {home} config get -f /")),
            "{call}"
        );
        assert!(call.ends_with(&format!(" | {home} | null")), "{call}");
    }
    // Of the fixture's own files, only the home's config file is read; the
    // other calls are the host's directories above the fixture, if any.
    let fixture = f.dir.to_str().unwrap();
    let ours: Vec<&str> = calls[1..]
        .iter()
        .filter(|call| call.contains(&format!(" -f {fixture}/")))
        .copied()
        .collect();
    assert_eq!(
        ours,
        [format!(
            "-C {home} config get -f {} | {home} | null",
            config.display()
        )]
    );
    assert!(f.mod_root().join("hooks/register.js").is_file());
}

/// D5: a copy-mode `[dotfiles]` declaration of `~/.claude/skills` refuses
/// through the CLI although history lists nothing, and nothing is written.
#[test]
fn claude_mod_install_refuses_a_copy_mode_mise_declaration() {
    let f = Reporter::new();
    f.installable();
    f.install_step("--install-hooks");
    let config = f.home.join(".config/mise/config.toml");
    fs::create_dir_all(config.parent().unwrap()).unwrap();
    write(
        &config,
        b"[dotfiles.\"~/.claude/skills\"]\nmode = \"copy\"\n",
        0o600,
    );
    let home = f.home.to_str().unwrap();
    support::write_executable(
        &f.dir.join("bin/mise"),
        format!(
            "#!/bin/sh\ncase \"$3\" in\ndotfiles) printf '%s' '{{\"entries\":[],\"incomplete\":[]}}' ;;\nconfig) case \"$6\" in '{home}'/*) while IFS= read -r line; do printf '%s\\n' \"$line\"; done < \"$6\" ;; esac ;;\n*) exit 1 ;;\nesac\n"
        )
        .as_bytes(),
        0o700,
    );
    let receipt = fs::read(f.root.join(".hooks-receipt.json")).unwrap();
    let output = f.installer("--install-claude-mod").output().unwrap();
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("managed by mise"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!f.home.join(".claude/skills").exists());
    assert_eq!(
        fs::read(f.root.join(".hooks-receipt.json")).unwrap(),
        receipt
    );
}

/// D5: while the plugin root is locked, every receipt writer refuses as busy
/// within its bound and leaves every file unchanged.
#[test]
fn hook_receipt_writers_refuse_as_busy_through_the_cli() {
    use std::os::fd::AsRawFd;
    let f = Reporter::new();
    f.installable();
    f.install_step("--install-hooks");
    f.install_step("--install-claude-mod");
    let before = file_tree(&f.dir);
    let lock = fs::File::open(&f.root).unwrap();
    assert_eq!(
        unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
        0
    );
    let started = Instant::now();
    let writers: Vec<_> = [
        "--install-hooks",
        "--install-claude-mod",
        "--uninstall-claude-mod",
        "--uninstall-hooks",
        "--repair-retired-hooks",
    ]
    .into_iter()
    .map(|mode| (mode, f.installer(mode).spawn().unwrap()))
    .collect();
    for (mode, writer) in writers {
        let output = writer.wait_with_output().unwrap();
        assert_eq!(output.status.code(), Some(1), "{mode}");
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            "Hook receipt busy; retry\n",
            "{mode}"
        );
    }
    let waited = started.elapsed();
    assert!(
        waited >= Duration::from_secs(3) && waited < Duration::from_secs(6),
        "{waited:?}"
    );
    assert_eq!(file_tree(&f.dir), before);
    drop(lock);
    f.install_step("--uninstall-claude-mod");
    assert!(!f.mod_root().exists());
}
