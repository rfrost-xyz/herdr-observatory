//! User navigation with fixture executables. No actual desktop, SSH or Herdr
//! control is performed; stale routes must fail before even inspecting windows.
use anton_runtime::{common, navigation};
use serde_json::{Value, json};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static FIXTURE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "anton-navigation-{}-{}-{}",
            std::process::id(),
            common::now().to_bits(),
            FIXTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::create_dir(root.join("bin")).unwrap();
        let fixture = Self(root);
        fixture.script("herdr", "cat \"$ANTON_TEST_FIXTURE/profiles.json\"");
        fixture.script(
            "hyprctl",
            "echo window-read >> \"$ANTON_TEST_FIXTURE/calls\"; echo '[]'",
        );
        fixture.script("ssh","printf '%s\\n' \"$@\" > \"$ANTON_TEST_FIXTURE/ssh-args\"; cat > \"$ANTON_TEST_FIXTURE/focus.json\"");
        fixture.script("omarchy", "exit 0");
        fixture
    }
    fn script(&self, name: &str, body: &str) {
        let path = self.0.join("bin").join(name);
        fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    fn click(&self, profiles: &Value, binding: &Value) -> std::process::Output {
        fs::write(self.0.join("profiles.json"), profiles.to_string()).unwrap();
        Command::new(env!("CARGO_BIN_EXE_anton-runtime"))
            .args([
                "--root",
                self.0.to_str().unwrap(),
                "--open-thread",
                "legacy-host",
                "legacy-host:w1:p2",
                &binding.to_string(),
            ])
            .env("HOME", &self.0)
            .env("XDG_STATE_HOME", self.0.join("state"))
            .env(
                "PATH",
                format!("{}:/usr/bin:/bin", self.0.join("bin").display()),
            )
            .env("ANTON_TEST_FIXTURE", &self.0)
            .output()
            .unwrap()
    }
    fn configured_click(
        &self,
        config: &Value,
        host: &str,
        binding: Option<&Value>,
    ) -> std::process::Output {
        fs::write(self.0.join(".config.json"), config.to_string()).unwrap();
        self.script(
            "herdr",
            "if [ \"$1\" = machine ]; then cat \"$ANTON_TEST_FIXTURE/profiles.json\"; else printf '%s\\n' \"$@\" > \"$ANTON_TEST_FIXTURE/local-focus\"; fi",
        );
        let mut command = Command::new(env!("CARGO_BIN_EXE_anton-runtime"));
        command.args([
            "--root",
            self.0.to_str().unwrap(),
            "--open-thread",
            host,
            &format!("{host}:w1:p2"),
        ]);
        if let Some(binding) = binding {
            command.arg(binding.to_string());
        }
        command
            .env("HOME", &self.0)
            .env("XDG_STATE_HOME", self.0.join("state"))
            .env(
                "PATH",
                format!("{}:/usr/bin:/bin", self.0.join("bin").display()),
            )
            .env("ANTON_TEST_FIXTURE", &self.0)
            .output()
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn unchanged_profile_focuses_exact_observed_session_despite_duplicate_labels() {
    let f = Fixture::new();
    let profile = json!({"id":"saved","label":"Shared label","target":"user@fixture","session":"work; quoted value","enabled":true});
    let binding = navigation::profile_binding(&profile).unwrap();
    let other = json!({"id":"another","label":"Shared label","target":"user@other","session":"default","enabled":true});
    let output = f.click(&json!([profile, other]), &binding);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let focus: Value = serde_json::from_slice(&fs::read(f.0.join("focus.json")).unwrap()).unwrap();
    assert_eq!(
        focus,
        json!({"pane":"w1:p2","session":"work; quoted value"})
    );
    let args = fs::read_to_string(f.0.join("ssh-args")).unwrap();
    assert!(args.lines().any(|line| line == "user@fixture"));
    assert!(!args.contains("quoted value"));
}

#[test]
fn changed_removed_disabled_or_ambiguous_profile_never_reaches_control_commands() {
    let f = Fixture::new();
    let profile = json!({"id":"saved","label":"legacy-host","target":"user@fixture","session":"work","enabled":true});
    let binding = navigation::profile_binding(&profile).unwrap();
    let mut cases = vec![json!([]), json!([profile, profile])];
    for (key, value) in [
        ("target", json!("user@other")),
        ("session", json!("replacement")),
        ("enabled", json!(false)),
        ("id", json!("new-profile")),
    ] {
        let mut changed = profile.clone();
        changed[key] = value;
        cases.push(json!([changed]));
    }
    for profiles in cases {
        let output = f.click(&profiles, &binding);
        assert!(!output.status.success());
        assert!(!f.0.join("calls").exists());
        assert!(!f.0.join("focus.json").exists());
        assert!(!String::from_utf8_lossy(&output.stderr).contains("user@"));
    }
    let output = f.click(
        &json!([profile]),
        &json!({"profile_id":"saved","route_key":"malformed"}),
    );
    assert!(!output.status.success());
    assert!(!f.0.join("calls").exists());
}

#[test]
fn configured_local_sources_focus_exact_session_with_custom_ids() {
    for session in ["default", "synthetic-other", "work; quoted value"] {
        let f = Fixture::new();
        let host = json!({"id":"custom-local","transport":"local","session":session});
        let binding = navigation::host_binding(&host).unwrap();
        assert!(!binding.to_string().contains(session));
        let output = f.configured_click(&json!({"hosts":[host]}), "custom-local", Some(&binding));
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            fs::read_to_string(f.0.join("local-focus")).unwrap(),
            format!("--session\n{session}\nagent\nfocus\nw1:p2\n")
        );
        assert!(!f.0.join("focus.json").exists());
    }
}

#[test]
fn configured_ssh_host_never_uses_hostname_as_local_authority() {
    let f = Fixture::new();
    let output = Command::new("hostname").output().unwrap();
    let name = String::from_utf8(output.stdout).unwrap().trim().to_owned();
    let host =
        json!({"id":name,"transport":"ssh","target":"user@fixture","session":"remote-other"});
    let binding = navigation::host_binding(&host).unwrap();
    fs::write(
        f.0.join("profiles.json"),
        json!([{"id":"saved","target":"user@fixture","session":"remote-other","enabled":true}])
            .to_string(),
    )
    .unwrap();
    let output = f.configured_click(&json!({"hosts":[host]}), &name, Some(&binding));
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!f.0.join("local-focus").exists());
    let focus: Value = serde_json::from_slice(&fs::read(f.0.join("focus.json")).unwrap()).unwrap();
    assert_eq!(focus, json!({"pane":"w1:p2","session":"remote-other"}));
}

#[test]
fn configured_stale_missing_socket_and_malformed_routes_never_control() {
    let host = json!({"id":"custom-local","transport":"local","session":"work"});
    let binding = navigation::host_binding(&host).unwrap();
    let mut cases = vec![
        (json!({"hosts":[host]}), None),
        (json!({"hosts":[{"id":"other"}]}), Some(binding.clone())),
        (
            json!({"hosts":[host]}),
            Some(json!({"host_id":"custom-local","route_key":"bad"})),
        ),
    ];
    for (key, value) in [
        ("session", json!("replacement")),
        ("herdr", json!("/other/herdr")),
        ("transport", json!("ssh")),
    ] {
        let mut changed = host.clone();
        changed[key] = value;
        changed["target"] = json!("user@other");
        cases.push((json!({"hosts":[changed]}), Some(binding.clone())));
    }
    let socket = json!({"id":"custom-local","socket_path":"/synthetic/herdr.sock"});
    cases.push((
        json!({"hosts":[socket]}),
        Some(navigation::host_binding(&socket).unwrap()),
    ));
    for (config, binding) in cases {
        let f = Fixture::new();
        let output = f.configured_click(&config, "custom-local", binding.as_ref());
        assert!(!output.status.success());
        for file in ["calls", "local-focus", "focus.json"] {
            assert!(!f.0.join(file).exists(), "{file}");
        }
    }
}

#[test]
fn configured_ssh_requires_unique_current_saved_route() {
    let host = json!({"id":"remote","transport":"ssh","target":"user@fixture","session":"work"});
    let binding = navigation::host_binding(&host).unwrap();
    let profile = json!({"id":"saved","target":"user@fixture","session":"work","enabled":true});
    for profiles in [
        json!([]),
        json!([profile, profile]),
        json!([{"id":"saved","target":"user@other","session":"work","enabled":true}]),
    ] {
        let f = Fixture::new();
        fs::write(f.0.join("profiles.json"), profiles.to_string()).unwrap();
        let output = f.configured_click(&json!({"hosts":[host]}), "remote", Some(&binding));
        assert!(!output.status.success());
        for file in ["calls", "local-focus", "focus.json"] {
            assert!(!f.0.join(file).exists());
        }
    }
}
