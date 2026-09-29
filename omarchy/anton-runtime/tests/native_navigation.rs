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
