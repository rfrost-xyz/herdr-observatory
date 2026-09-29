//! Synthetic process acceptance. Every HOME, socket, executable and SSH target is
//! fixture-owned. No interpreter, live account, remote machine or GPU is used.
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
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}
struct Fixture {
    dir: PathBuf,
    root: PathBuf,
    peer: PathBuf,
    state: PathBuf,
    raw: Arc<Mutex<Value>>,
    stop: Arc<AtomicBool>,
    server: Option<thread::JoinHandle<()>>,
}
impl Fixture {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "anton-native-{}-{}-{}",
            std::process::id(),
            common::now().to_bits(),
            FIXTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
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
        let state_raw = raw.clone();
        let stop_server = stop.clone();
        let server = thread::spawn(move || {
            while !stop_server.load(Ordering::Relaxed) {
                if let Ok((mut stream, _)) = listener.accept() {
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
                    let _ = writeln!(stream, "{response}");
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
    let snapshot = stream.until(|v| v["allowances"][0]["weekly_remaining"] == 75);
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
        ["allowances", "at", "fleet_discovery", "hosts", "interval"]
    );
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
            && v["allowances"][0]["weekly_remaining"] == 75
    });
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
        v["hosts"].as_array().unwrap().len() == 1
            && v["allowances"][0]["weekly_remaining"].is_null()
    });
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
    assert!(removed["allowances"][0]["weekly_remaining"].is_null());
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
        fs::copy(BIN, root.join("anton-runtime")).unwrap();
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
