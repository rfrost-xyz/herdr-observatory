//! Claude Code mod installer fixtures (design D5, D7), in temporary homes.
//! Every check gets an explicit environment: `PATH` holds only a fixture
//! directory, so the host's `chezmoi` and `mise` are never consulted.
use super::claude_mod::{self, ClaudeEnv};
use super::tests::{NativeFixture, Tree, hold_receipt_lock, tree};
use super::*;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::time::Instant;

type Files = Vec<(&'static str, Vec<u8>)>;

struct Mod {
    f: NativeFixture,
    env: ClaudeEnv,
}
impl Mod {
    /// A Pi installation in a temporary home with a real `~/.claude` and an
    /// empty fixture `PATH`.
    fn new() -> Self {
        let f = NativeFixture::new();
        common::ensure_private_directory(&f.home.join(".claude")).unwrap();
        common::ensure_private_directory(&f.home.join("bin")).unwrap();
        let env = ClaudeEnv {
            path: Some(f.home.join("bin").into_os_string()),
            config_dir: None,
            mise_vars: vec![(
                "MISE_SYSTEM_CONFIG_DIR",
                f.home.join("etc-mise").into_os_string(),
            )],
        };
        Self { f, env }
    }
    fn root(&self) -> PathBuf {
        claude_mod_root(&self.f.home)
    }
    fn file(&self, name: &str) -> PathBuf {
        self.root().join(name)
    }
    fn runtime(&self) -> PathBuf {
        self.f.root.join("anton-runtime")
    }
    fn entry(&self) -> Value {
        self.f.receipt()["claude_mod"].clone()
    }
    /// Every file and directory under the home (see `tree`).
    fn snapshot(&self) -> Tree {
        tree(&self.f.home)
    }
    fn install_with(&self, files: claude_mod::Payload, write: claude_mod::Writer) -> Result<()> {
        claude_mod::install_mod(&self.f.root, &self.f.home, &self.env, files, write)
    }
    fn install_payload(&self, files: claude_mod::Payload) -> Result<()> {
        self.install_with(files, &mut |path, bytes| {
            common::atomic_owned_write(path, bytes)
        })
    }
    fn install(&self) -> Result<()> {
        self.install_payload(&claude_mod::payload)
    }
    fn uninstall_mod(&self) -> Result<()> {
        claude_mod::uninstall_mod(&self.f.root, &self.f.home, &self.env, &mut |path, bytes| {
            common::atomic_owned_write(path, bytes)
        })
    }
    fn uninstall_all(&self) -> Result<()> {
        uninstall_in(&self.f.root, &self.f.home, &self.env)
    }
    /// An executable fixture in the `PATH` directory, written by a child
    /// process so no descriptor of it is open here when it runs.
    fn tool(&self, name: &str, body: &str) {
        let path = self.f.home.join("bin").join(name);
        let mut writer = std::process::Command::new("/bin/sh")
            .args(["-c", "cat > \"$1\"", "sh"])
            .arg(&path)
            .stdin(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        use std::io::Write;
        writer
            .stdin
            .take()
            .unwrap()
            .write_all(format!("#!/bin/sh\n{body}\n").as_bytes())
            .unwrap();
        assert!(writer.wait().unwrap().success());
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    /// A fake `mise` printing `listing` for `dotfiles paths --json` and, for
    /// `config get -f <file>`, the file itself when it is in the fixture home
    /// (written in the normalised form mise prints) and nothing otherwise.
    fn mise(&self, listing: &Value) {
        let home = self.f.home.display();
        self.tool(
            "mise",
            &format!(
                "case \"$3 $4 $5\" in\n'dotfiles paths --json') printf '%s' '{listing}' ;;\n'config get -f') case \"$6\" in '{home}'/*) exec cat -- \"$6\" ;; esac ;;\n*) exit 1 ;;\nesac"
            ),
        );
    }
    /// A mise config file at `relative` in the home holding `text`.
    fn declare(&self, relative: &str, text: &str) {
        let path = self.f.home.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
}

/// The embedded payload with `tag` appended to each file selected by `mask`
/// (hooks.json, register.js, plugin.json), as a later build would ship.
fn variant(tag: &'static str, mask: [bool; 3]) -> impl Fn(&Path) -> Result<Files> {
    move |runtime| {
        let mut files = claude_mod::payload(runtime)?;
        for ((_, bytes), changed) in files.iter_mut().zip(mask) {
            if changed {
                bytes.extend_from_slice(format!("\n// {tag}\n").as_bytes());
            }
        }
        Ok(files)
    }
}
fn hash(bytes: &[u8]) -> String {
    common::sha256(bytes)
}
/// Asserts `result` refused for `reason` and nothing under the home changed.
fn refused(m: &Mod, result: Result<()>, before: &Tree, what: &str, reason: &str) {
    let error = result.expect_err(what);
    assert!(error.contains(reason), "{what}: {error}");
    assert_eq!(&m.snapshot(), before, "{what}: nothing changes");
}

#[test]
fn fresh_install_records_the_mod_and_an_identical_reinstall_writes_nothing() {
    let m = Mod::new();
    let skills = m.f.home.join(".claude/skills");
    m.install().unwrap();
    let declaration = format!(
        "const nativeRuntime = {};",
        json!(m.runtime().to_string_lossy())
    );
    let register = String::from_utf8(std::fs::read(m.file("hooks/register.js")).unwrap()).unwrap();
    assert!(register.contains(&declaration), "the runtime is declared");
    assert!(!register.contains("const nativeRuntime = '';"));
    let files = claude_mod::payload(&m.runtime()).unwrap();
    for (name, bytes) in &files {
        assert_eq!(&std::fs::read(m.file(name)).unwrap(), bytes, "{name}");
        assert_eq!(
            std::fs::metadata(m.file(name)).unwrap().mode() & 0o777,
            0o600
        );
    }
    let directories = [
        skills.clone(),
        m.root(),
        m.root().join(".claude-plugin"),
        m.root().join("hooks"),
    ];
    for directory in &directories {
        assert_eq!(std::fs::metadata(directory).unwrap().mode() & 0o777, 0o700);
    }
    let entry = m.entry();
    assert!(claude_mod_entry(&entry, &m.root()));
    assert_eq!(entry["directories"], json!(directories));
    let recorded: Vec<_> = CLAUDE_MOD_FILES
        .iter()
        .map(|name| {
            let bytes = &files.iter().find(|(file, _)| file == name).unwrap().1;
            json!({"path":m.file(name),"sha256":hash(bytes)})
        })
        .collect();
    assert_eq!(entry["files"], json!(recorded), "no prior_sha256");
    assert!(
        claude_mod_recorded(&m.f.root, &m.f.home),
        "reporter guard 2"
    );
    let manifest: Value =
        serde_json::from_slice(&std::fs::read(m.file(".claude-plugin/plugin.json")).unwrap())
            .unwrap();
    assert_eq!(manifest["version"], env!("CARGO_PKG_VERSION"));
    let before = m.snapshot();
    m.install().unwrap();
    assert_eq!(m.snapshot(), before, "no file or receipt write");
}

#[test]
fn an_existing_skills_directory_is_not_recorded_or_removed() {
    let m = Mod::new();
    let skills = m.f.home.join(".claude/skills");
    common::ensure_private_directory(&skills.join("other")).unwrap();
    m.install().unwrap();
    assert_eq!(
        m.entry()["directories"],
        json!([
            m.root(),
            m.root().join(".claude-plugin"),
            m.root().join("hooks")
        ])
    );
    m.uninstall_mod().unwrap();
    assert!(!m.root().exists());
    assert!(skills.join("other").is_dir());
    assert!(m.f.receipt().get("claude_mod").is_none());
}

/// A `skills/` the installer recreated after install is recorded after its
/// children; removal still deletes it, deepest directory first.
#[test]
fn a_skills_directory_recreated_after_install_is_removed() {
    let m = Mod::new();
    let skills = m.f.home.join(".claude/skills");
    common::ensure_private_directory(&skills).unwrap();
    m.install().unwrap();
    std::fs::remove_dir_all(&skills).unwrap();
    m.install().unwrap();
    assert_eq!(
        m.entry()["directories"],
        json!([
            m.root(),
            m.root().join(".claude-plugin"),
            m.root().join("hooks"),
            skills
        ])
    );
    m.uninstall_mod().unwrap();
    assert!(!skills.exists(), "the recorded skills directory is removed");
    assert!(m.f.home.join(".claude").is_dir());
    assert!(m.f.receipt().get("claude_mod").is_none());
}

#[test]
fn a_payload_change_rewrites_only_the_changed_files() {
    let m = Mod::new();
    m.install().unwrap();
    let before = m.snapshot();
    m.install_payload(&variant("build-b", [false, false, true]))
        .unwrap();
    let after = m.snapshot();
    for name in ["hooks/hooks.json", "hooks/register.js"] {
        assert_eq!(after[&m.file(name)], before[&m.file(name)], "{name} kept");
    }
    let manifest = m.file(".claude-plugin/plugin.json");
    assert_ne!(
        after[&manifest].inode, before[&manifest].inode,
        "manifest rewritten"
    );
    let files = variant("build-b", [false, false, true])(&m.runtime()).unwrap();
    for (index, name) in CLAUDE_MOD_FILES.iter().enumerate() {
        let bytes = &files.iter().find(|(file, _)| file == name).unwrap().1;
        assert_eq!(
            m.entry()["files"][index],
            json!({"path":m.file(name),"sha256":hash(bytes)})
        );
    }
}

#[test]
fn writes_go_receipt_first_and_the_manifest_last() {
    let m = Mod::new();
    let receipt = m.f.root.join(".hooks-receipt.json");
    let mut order = Vec::new();
    let mut record = |path: &Path, bytes: &[u8]| {
        order.push(path.to_owned());
        common::atomic_owned_write(path, bytes)
    };
    m.install_with(&claude_mod::payload, &mut record).unwrap();
    let files: Vec<_> = [
        "hooks/hooks.json",
        "hooks/register.js",
        ".claude-plugin/plugin.json",
    ]
    .iter()
    .map(|name| m.file(name))
    .collect();
    assert_eq!(order, [vec![receipt.clone()], files.clone()].concat());
    order.clear();
    let mut record = |path: &Path, bytes: &[u8]| {
        order.push(path.to_owned());
        common::atomic_owned_write(path, bytes)
    };
    m.install_with(&variant("build-b", [true, true, true]), &mut record)
        .unwrap();
    assert_eq!(
        order,
        [vec![receipt.clone()], files, vec![receipt]].concat()
    );
}

/// A debris file as `atomic_write` names it, left by a killed writer.
const DEBRIS: &str = ".anton-write-4242-4611686018427387904";

/// An interrupted A to B refresh as on-disk state (design D7): A installed,
/// the dual receipt written (each file `sha256` B and `prior_sha256` A),
/// then each file at B where `new` says so, otherwise still at A.
fn interrupted(m: &Mod, new: [bool; 3], debris: bool) {
    m.install().unwrap();
    let a = claude_mod::payload(&m.runtime()).unwrap();
    let b = variant("build-b", [true, true, true])(&m.runtime()).unwrap();
    let mut receipt = m.f.receipt();
    for (index, name) in CLAUDE_MOD_FILES.iter().enumerate() {
        let find = |files: &Files| files.iter().find(|(f, _)| f == name).unwrap().1.clone();
        receipt["claude_mod"]["files"][index] =
            json!({"path":m.file(name),"sha256":hash(&find(&b)),"prior_sha256":hash(&find(&a))});
    }
    common::atomic_owned_write(
        &m.f.root.join(".hooks-receipt.json"),
        &serde_json::to_vec(&receipt).unwrap(),
    )
    .unwrap();
    assert!(
        claude_mod_recorded(&m.f.root, &m.f.home),
        "guard 2 accepts it"
    );
    for ((name, bytes), new) in b.iter().zip(new) {
        if new {
            common::atomic_owned_write(&m.file(name), bytes).unwrap();
        }
    }
    if debris {
        common::atomic_owned_write(&m.file("hooks").join(DEBRIS), b"partial").unwrap();
    }
}
const STATES: [([bool; 3], bool, &str); 4] = [
    ([false, false, false], false, "a: all files old"),
    ([true, false, false], false, "b: files mixed"),
    ([true, true, true], false, "c: all files new"),
    ([false, true, false], true, "d: mixed with debris"),
];

#[test]
fn an_interrupted_refresh_completes_on_retry() {
    for (new, debris, state) in STATES {
        let m = Mod::new();
        interrupted(&m, new, debris);
        let directories = m.entry()["directories"].clone();
        let build = variant("build-b", [true, true, true]);
        m.install_payload(&build).unwrap();
        for (name, bytes) in build(&m.runtime()).unwrap() {
            assert_eq!(
                std::fs::read(m.file(name)).unwrap(),
                bytes,
                "{state}: {name}"
            );
        }
        let entry = m.entry();
        assert!(!entry.to_string().contains("prior_sha256"), "{state}");
        assert_eq!(entry["directories"], directories, "{state}");
        assert!(!m.file("hooks").join(DEBRIS).exists(), "{state}");
        let before = m.snapshot();
        m.install_payload(&build).unwrap();
        assert_eq!(m.snapshot(), before, "{state}: then idempotent");
    }
}

#[test]
fn an_interrupted_refresh_is_removed_by_uninstall_hooks() {
    for (new, debris, state) in STATES {
        let m = Mod::new();
        interrupted(&m, new, debris);
        m.uninstall_all().unwrap();
        assert!(!m.f.home.join(".claude/skills").exists(), "{state}");
        assert!(m.f.home.join(".claude").is_dir(), "{state}");
        assert!(!paths(&m.f.home).1.exists(), "{state}: Pi removed");
        assert!(!m.f.root.join(".hooks-receipt.json").exists(), "{state}");
    }
}

#[test]
fn a_refresh_interrupted_from_a_to_b_completes_with_build_c() {
    for (new, debris, state) in STATES {
        let m = Mod::new();
        interrupted(&m, new, debris);
        let on_disk: Vec<_> = CLAUDE_MOD_FILES
            .iter()
            .map(|name| hash(&std::fs::read(m.file(name)).unwrap()))
            .collect();
        let c = variant("build-c", [true, false, true]);
        let receipt = m.f.root.join(".hooks-receipt.json");
        let mut receipts = Vec::new();
        m.install_with(&c, &mut |path, bytes| {
            if path == receipt {
                receipts.push(serde_json::from_slice::<Value>(bytes).unwrap());
            }
            common::atomic_owned_write(path, bytes)
        })
        .unwrap();
        // The dual receipt takes each prior hash from the verified bytes on
        // disk (A or B), never from the old receipt's `prior_sha256` (A).
        assert_eq!(receipts.len(), 2, "{state}");
        let files = c(&m.runtime()).unwrap();
        for (index, name) in CLAUDE_MOD_FILES.iter().enumerate() {
            let bytes = &files.iter().find(|(f, _)| f == name).unwrap().1;
            let dual = &receipts[0]["claude_mod"]["files"][index];
            assert_eq!(dual["sha256"], hash(bytes), "{state}");
            let prior = if on_disk[index] == hash(bytes) {
                Value::Null
            } else {
                json!(on_disk[index])
            };
            assert_eq!(dual["prior_sha256"], prior, "{state}: {name}");
        }
        for (index, name) in CLAUDE_MOD_FILES.iter().enumerate() {
            let bytes = &files.iter().find(|(f, _)| f == name).unwrap().1;
            assert_eq!(&std::fs::read(m.file(name)).unwrap(), bytes, "{state}");
            assert_eq!(
                m.entry()["files"][index],
                json!({"path":m.file(name),"sha256":hash(bytes)}),
                "{state}"
            );
        }
    }
}

#[test]
fn a_retry_after_the_receipt_write_lists_each_directory_once() {
    let m = Mod::new();
    m.install().unwrap();
    let entry = m.entry();
    // As if the first install stopped after its receipt write: the receipt
    // already lists every directory, none of which exists yet.
    std::fs::remove_dir_all(m.f.home.join(".claude/skills")).unwrap();
    m.install().unwrap();
    assert_eq!(m.entry(), entry);
    m.install_payload(&variant("build-b", [true, true, true]))
        .unwrap();
    assert_eq!(m.entry()["directories"], entry["directories"]);
}

#[test]
fn a_refresh_keeps_the_directories_and_uninstall_leaves_no_mod_tree() {
    let m = Mod::new();
    m.install().unwrap();
    let directories = m.entry()["directories"].clone();
    m.install_payload(&variant("build-b", [false, true, false]))
        .unwrap();
    assert_eq!(m.entry()["directories"], directories);
    let pi = std::fs::read(paths(&m.f.home).1).unwrap();
    m.uninstall_mod().unwrap();
    assert!(!m.f.home.join(".claude/skills").exists());
    assert!(m.f.receipt().get("claude_mod").is_none());
    assert!(!claude_mod_recorded(&m.f.root, &m.f.home));
    assert_eq!(std::fs::read(paths(&m.f.home).1).unwrap(), pi);
    let before = m.snapshot();
    m.uninstall_mod().unwrap();
    assert_eq!(m.snapshot(), before, "a second removal is a no-op");
}

/// Each case prepares a home, then the install refuses and changes nothing.
#[test]
fn install_refuses_targets_it_cannot_prove_are_its_own() {
    use std::os::unix::fs::symlink;
    type Setup = fn(&mut Mod);
    let cases: [(&str, &str, Setup); 23] = [
        (
            "changed root",
            "Conflicting Claude Code mod directory",
            |m| {
                // A whole mod tree beside a receipt with no `claude_mod`, as a
                // second plugin root (another XDG_CONFIG_HOME) sees it.
                m.install().unwrap();
                let mut receipt = m.f.receipt();
                receipt.as_object_mut().unwrap().remove("claude_mod");
                common::atomic_owned_write(
                    &m.f.root.join(".hooks-receipt.json"),
                    &serde_json::to_vec(&receipt).unwrap(),
                )
                .unwrap();
            },
        ),
        (
            "unowned directory",
            "Conflicting Claude Code mod directory",
            |m| {
                common::ensure_private_directory(&m.root()).unwrap();
            },
        ),
        ("unrecorded file", "extra.js", |m| {
            m.install().unwrap();
            common::atomic_owned_write(&m.file("hooks/extra.js"), b"x").unwrap();
        }),
        ("unrecorded directory", "anton-observatory/data", |m| {
            m.install().unwrap();
            common::ensure_private_directory(&m.file("data")).unwrap();
        }),
        ("modified recorded file", "mod file changed", |m| {
            m.install().unwrap();
            common::atomic_owned_write(&m.file("hooks/register.js"), b"changed").unwrap();
        }),
        ("symlinked ~/.claude", "configuration not found", |m| {
            let real = m.f.home.join("claude-real");
            std::fs::rename(m.f.home.join(".claude"), &real).unwrap();
            symlink(&real, m.f.home.join(".claude")).unwrap();
        }),
        ("symlinked skills", "symlink", |m| {
            let real = m.f.home.join("skills-real");
            common::ensure_private_directory(&real).unwrap();
            symlink(&real, m.f.home.join(".claude/skills")).unwrap();
        }),
        ("symlinked target", "symlink", |m| {
            m.install().unwrap();
            let real = m.f.home.join("mod-real");
            std::fs::rename(m.root(), &real).unwrap();
            symlink(&real, m.root()).unwrap();
        }),
        ("symlinked recorded file", "hooks/register.js", |m| {
            m.install().unwrap();
            let real = m.f.home.join("register-real.js");
            std::fs::rename(m.file("hooks/register.js"), &real).unwrap();
            symlink(&real, m.file("hooks/register.js")).unwrap();
        }),
        ("debris-named symlink", ".anton-write-4242", |m| {
            m.install().unwrap();
            symlink(m.file("hooks/hooks.json"), m.file("hooks").join(DEBRIS)).unwrap();
        }),
        (
            "debris-named file in the mod root",
            ".anton-write-4242",
            |m| {
                // Only `.claude-plugin/` and `hooks/` receive atomic writes.
                m.install().unwrap();
                common::atomic_owned_write(&m.root().join(DEBRIS), b"x").unwrap();
            },
        ),
        ("non-debris temporary name", ".anton-write-", |m| {
            m.install().unwrap();
            for name in [".anton-write-12", ".anton-write-1-2x", ".anton-write--2"] {
                common::atomic_owned_write(&m.file("hooks").join(name), b"x").unwrap();
            }
        }),
        ("chezmoi", "chezmoi", |m| m.tool("chezmoi", "exit 0")),
        ("chezmoi-managed ~/.claude", "chezmoi", |m| {
            m.tool(
                "chezmoi",
                "case \"$2\" in */.claude) exit 0;; *) exit 1;; esac",
            );
        }),
        ("chezmoi-managed skills", "chezmoi", |m| {
            m.tool(
                "chezmoi",
                "case \"$2\" in */.claude/skills) exit 0;; *) exit 1;; esac",
            );
        }),
        ("Git directory marker", "Git repository", |m| {
            common::ensure_private_directory(&m.f.home.join(".git")).unwrap();
            common::atomic_owned_write(&m.f.home.join(".git/HEAD"), b"ref: refs/heads/main\n")
                .unwrap();
        }),
        ("Git file marker", "Git repository", |m| {
            common::atomic_owned_write(&m.f.home.join(".claude/.git"), b"gitdir: /elsewhere\n")
                .unwrap();
        }),
        ("symlinked Git directory marker", "Git repository", |m| {
            let real = m.f.home.join("dotfiles.git");
            common::ensure_private_directory(&real).unwrap();
            common::atomic_owned_write(&real.join("HEAD"), b"ref: refs/heads/main\n").unwrap();
            symlink(&real, m.f.home.join(".git")).unwrap();
        }),
        ("symlinked Git file marker", "Git repository", |m| {
            let real = m.f.home.join("gitdir-file");
            common::atomic_owned_write(&real, b"gitdir: /elsewhere\n").unwrap();
            symlink(&real, m.f.home.join(".claude/.git")).unwrap();
        }),
        ("dangling Git marker symlink", "Git repository", |m| {
            symlink(m.f.home.join("missing"), m.f.home.join(".claude/.git")).unwrap();
        }),
        ("CLAUDE_CONFIG_DIR elsewhere", "CLAUDE_CONFIG_DIR", |m| {
            m.env.config_dir = Some(m.f.home.join("other-claude").into_os_string());
        }),
        ("CLAUDE_CONFIG_DIR empty", "CLAUDE_CONFIG_DIR", |m| {
            m.env.config_dir = Some(std::ffi::OsString::new());
        }),
        (
            "no Claude Code configuration",
            "configuration not found",
            |m| {
                std::fs::remove_dir(m.f.home.join(".claude")).unwrap();
            },
        ),
    ];
    for (what, reason, setup) in cases {
        let mut m = Mod::new();
        setup(&mut m);
        let before = m.snapshot();
        refused(&m, m.install(), &before, what, reason);
    }
}

#[test]
fn install_refuses_a_target_listed_by_mise_or_an_unreadable_listing() {
    let target = "~/.claude/skills/anton-observatory";
    let mut listings = Vec::new();
    for path in [
        target,
        "~/.claude",
        "~",
        "~/.claude/skills/anton-observatory/hooks",
    ] {
        listings.push((path.to_owned(), json!({"entries":[{"path":path}]}), true));
    }
    for key in ["incomplete", "invalid", "nested", "omitted"] {
        listings.push((
            key.to_owned(),
            json!({"entries":[],key:[{"path":target}]}),
            true,
        ));
    }
    for (what, listing) in [
        ("no entries", json!({"incomplete":[]})),
        ("entries not a list", json!({"entries":{}})),
        ("item without a path", json!({"entries":[{"source":"x"}]})),
        (
            "another user's home",
            json!({"entries":[{"path":"~other/.claude"}]}),
        ),
        (
            "parent component",
            json!({"entries":[{"path":"~/x/../.claude"}]}),
        ),
    ] {
        listings.push((what.to_owned(), listing, false));
    }
    for (what, listing, covered) in listings {
        let m = Mod::new();
        m.mise(&listing);
        let before = m.snapshot();
        let reason = if covered {
            "managed by mise"
        } else {
            "Cannot confirm mise"
        };
        refused(&m, m.install(), &before, &what, reason);
    }
    for (what, body) in [
        ("failing mise", "exit 1"),
        ("unparseable output", "printf 'not json'"),
        ("hanging mise", "exec sleep 10"),
    ] {
        let m = Mod::new();
        m.tool("mise", body);
        let before = m.snapshot();
        let started = Instant::now();
        refused(&m, m.install(), &before, what, "Cannot confirm mise");
        assert!(
            started.elapsed() < Duration::from_secs(6),
            "{what} is bounded"
        );
    }
}

/// D5: a `[dotfiles]` declaration in any mode covering the target refuses,
/// although history (`dotfiles paths`) lists nothing (review round 3).
#[test]
fn install_refuses_a_target_declared_in_mise_dotfiles_in_any_mode() {
    let empty = json!({"entries":[],"exclude":[],"invalid":[],"omitted":[],"plaintext":[],"nested":[],"incomplete":[]});
    let cases: [(&str, &str, &str); 10] = [
        (
            "copy mode",
            ".config/mise/config.toml",
            "[dotfiles.\"~/.claude/skills\"]\nmode = \"copy\"\n",
        ),
        (
            "raw inline copy",
            ".config/mise/config.toml",
            "[dotfiles]\n\"~/.claude/skills\" = { mode = \"copy\" }\n",
        ),
        (
            "link mode",
            ".config/mise/config.toml",
            "[dotfiles.\"~/.claude\"]\nmode = \"link\"\n",
        ),
        (
            "template mode",
            ".config/mise/config.toml",
            "[dotfiles.\"~/.claude/skills/anton-observatory/hooks/hooks.json\"]\nmode = \"template\"\nsource = \"h.tmpl\"\n",
        ),
        (
            "track mode",
            ".config/mise/config.toml",
            "[dotfiles.\"~/.claude/skills/anton-observatory\"]\nmode = \"track\"\n",
        ),
        (
            "plain string entry",
            ".config/mise/config.toml",
            "[dotfiles]\n\"~/.claude/skills\" = \"claude/skills\"\n",
        ),
        (
            "home directory",
            ".config/mise/config.toml",
            "[dotfiles]\n\"~\" = \"home\"\n",
        ),
        (
            "conf.d fragment",
            ".config/mise/conf.d/claude.toml",
            "[dotfiles.\"~/.claude/skills\"]\nmode = \"copy\"\n",
        ),
        (
            "conf.d folder fragment",
            ".config/mise/conf.d/claude/mise.toml",
            "[dotfiles.\"~/.claude/skills\"]\nmode = \"copy\"\n",
        ),
        (
            "project file in the home",
            ".mise.local.toml",
            "[dotfiles.\"~/.claude/skills\"]\nmode = \"copy\"\n",
        ),
    ];
    for (what, file, text) in cases {
        let m = Mod::new();
        m.mise(&empty);
        m.declare(file, text);
        let before = m.snapshot();
        refused(&m, m.install(), &before, what, "managed by mise");
    }
    // Files that mise's variables move are read too.
    for (name, dir) in [
        ("MISE_CONFIG_DIR", "moved/mise"),
        ("XDG_CONFIG_HOME", "xdg"),
        ("MISE_SYSTEM_CONFIG_DIR", "etc-mise"),
    ] {
        let mut m = Mod::new();
        m.mise(&empty);
        let config = if name == "XDG_CONFIG_HOME" {
            format!("{dir}/mise/config.toml")
        } else {
            format!("{dir}/config.toml")
        };
        m.declare(
            &config,
            "[dotfiles.\"~/.claude/skills\"]\nmode = \"copy\"\n",
        );
        m.env.mise_vars.retain(|(key, _)| *key != name);
        m.env
            .mise_vars
            .push((name, m.f.home.join(dir).into_os_string()));
        let before = m.snapshot();
        refused(&m, m.install(), &before, name, "managed by mise");
    }
}

/// D5: declarations that cannot be read or understood refuse.
#[test]
fn install_refuses_unreadable_mise_declarations() {
    type Setup = fn(&Mod);
    let cases: [(&str, Setup); 5] = [
        ("config get fails", |m| {
            m.declare(".config/mise/config.toml", "[tools]\n");
            let home = m.f.home.display();
            m.tool(
                "mise",
                &format!("[ \"$3\" = dotfiles ] && printf '%s' '{}' && exit 0; case \"$6\" in '{home}'/*) exit 1 ;; esac", json!({"entries":[]})),
            );
        }),
        ("unparseable declarations", |m| {
            m.mise(&json!({"entries":[]}));
            m.declare(".config/mise/config.toml", "[dotfiles\n");
        }),
        ("array of dotfiles tables", |m| {
            m.mise(&json!({"entries":[]}));
            m.declare(".config/mise/config.toml", "[[dotfiles]]\nx = 1\n");
        }),
        ("another user's home", |m| {
            m.mise(&json!({"entries":[]}));
            m.declare(
                ".config/mise/config.toml",
                "[dotfiles]\n\"~other/.claude\" = \"x\"\n",
            );
        }),
        // Made unlistable below, after the snapshot.
        ("unlistable conf.d", |m| {
            m.mise(&json!({"entries":[]}));
            std::fs::create_dir_all(m.f.home.join(".config/mise/conf.d")).unwrap();
        }),
    ];
    for (what, setup) in cases {
        let m = Mod::new();
        setup(&m);
        let before = m.snapshot();
        let conf = m.f.home.join(".config/mise/conf.d");
        let original = std::fs::metadata(&conf).ok().map(|v| v.permissions());
        if original.is_some() {
            std::fs::set_permissions(&conf, std::fs::Permissions::from_mode(0o000)).unwrap();
        }
        let result = m.install();
        if let Some(permissions) = original {
            std::fs::set_permissions(&conf, permissions).unwrap();
        }
        refused(&m, result, &before, what, "Cannot confirm mise");
    }
}

#[test]
fn install_accepts_unrelated_markers_and_listings() {
    type Setup = fn(&Mod);
    let cases: [(&str, Setup); 6] = [
        ("empty .git directory", |m| {
            common::ensure_private_directory(&m.f.home.join(".git")).unwrap();
        }),
        (".git directory without HEAD", |m| {
            common::ensure_private_directory(&m.f.home.join(".claude/.git/objects")).unwrap();
        }),
        ("sibling mise entries", |m| {
            m.mise(&json!({"entries":[{"path":"~/.claude/skills/other"},{"path":"~/.claude/settings.json"}],"exclude":[{"path":"~/.claude"}],"plaintext":[{"path":"~/.claude"}]}));
        }),
        ("sibling mise declarations", |m| {
            m.mise(&json!({"entries":[]}));
            m.declare(
                ".config/mise/config.toml",
                "[dotfiles]\n\"~/.bashrc\" = \"bashrc\"\n\n[dotfiles.\"~/.claude/skills/other\"]\nmode = \"copy\"\n\n[dotfiles.\"~/.claude/settings.json\"]\nmode = \"copy\"\n\n[tasks.x]\nrun = \"\"\"\n[dotfiles]\n\"\"\"\n",
            );
        }),
        ("no mise on PATH", |_| {}),
        // The home is chezmoi's destination root, so it is not asked.
        ("chezmoi answering only for the home", |m| {
            let home = m.f.home.to_string_lossy().into_owned();
            m.tool(
                "chezmoi",
                &format!("[ \"$2\" = '{home}' ] && exit 0; exit 1"),
            );
        }),
    ];
    for (what, setup) in cases {
        let m = Mod::new();
        setup(&m);
        m.install().unwrap_or_else(|e| panic!("{what}: {e}"));
        assert!(claude_mod_recorded(&m.f.root, &m.f.home), "{what}");
    }
}

#[test]
fn install_refuses_a_peer_root_and_a_missing_pi_integration() {
    let m = Mod::new();
    let peer = m.f.home.join("herdr.observatory-peer");
    let error = claude_mod::install_mod(
        &peer,
        &m.f.home,
        &m.env,
        &claude_mod::payload,
        &mut |_, _| unreachable!(),
    )
    .unwrap_err();
    assert!(error.contains("local only"), "{error}");
    std::fs::remove_file(paths(&m.f.home).1).unwrap();
    let before = m.snapshot();
    refused(
        &m,
        m.install(),
        &before,
        "Pi extension missing",
        "Pi extension",
    );
    std::fs::remove_file(m.f.root.join(".hooks-receipt.json")).unwrap();
    let before = m.snapshot();
    refused(
        &m,
        m.install(),
        &before,
        "hook receipt missing",
        "receipt missing",
    );
}

#[test]
fn mod_receipt_writers_refuse_as_busy_while_the_lock_is_held() {
    let m = Mod::new();
    m.install().unwrap();
    common::atomic_owned_write(&m.file("hooks/register.js"), b"changed").unwrap();
    let before = m.snapshot();
    let held = hold_receipt_lock(&m.f.root);
    let started = Instant::now();
    let results: Vec<Result<()>> = std::thread::scope(|scope| {
        [
            scope.spawn(|| m.install()),
            scope.spawn(|| m.uninstall_mod()),
            scope.spawn(|| m.uninstall_all()),
        ]
        .into_iter()
        .map(|writer| writer.join().unwrap())
        .collect()
    });
    let waited = started.elapsed();
    for result in results {
        assert_eq!(result, Err("Hook receipt busy; retry".into()));
    }
    assert!(
        waited >= RECEIPT_WAIT && waited < RECEIPT_WAIT * 2,
        "{waited:?}"
    );
    assert_eq!(m.snapshot(), before);
    drop(held);
    assert!(m.install().is_err(), "the changed file is still refused");
}

/// Pi, the compatibility shim and the mod, all installed and recorded.
fn everything() -> Mod {
    let m = Mod::new();
    m.f.backup();
    repair_retired(&m.f.root, &m.f.home).unwrap();
    m.install().unwrap();
    assert!(m.f.shell().exists());
    m
}

#[test]
fn removal_of_a_changed_file_refuses_before_any_deletion() {
    type Setup = fn(&Mod) -> PathBuf;
    let cases: [(&str, Setup); 2] = [
        ("modified file", |m| {
            let path = m.file("hooks/register.js");
            common::atomic_owned_write(&path, b"user change").unwrap();
            path
        }),
        ("replaced by a directory", |m| {
            let path = m.file(".claude-plugin/plugin.json");
            std::fs::remove_file(&path).unwrap();
            common::ensure_private_directory(&path).unwrap();
            path
        }),
    ];
    for (what, setup) in cases {
        let m = everything();
        let path = setup(&m);
        if what == "modified file" {
            common::atomic_owned_write(&m.file("hooks").join(DEBRIS), b"partial").unwrap();
        }
        let before = m.snapshot();
        let message = format!(
            "Claude Code mod file changed: {}; restore or remove it, then retry",
            path.display()
        );
        for result in [m.uninstall_all(), m.uninstall_mod()] {
            assert_eq!(result, Err(message.clone()), "{what}");
            assert_eq!(m.snapshot(), before, "{what}: everything is kept");
        }
    }
}

/// Review round 2: debris is deleted only in `.claude-plugin/` and `hooks/`,
/// the directories that receive atomic writes. A debris-named file in a
/// recorded `skills/` or in the mod root cannot be Anton's, so install and
/// both removals keep it.
#[test]
fn debris_names_outside_the_written_directories_are_kept() {
    let skills_debris = |m: &Mod| {
        let path = m.f.home.join(".claude/skills").join(DEBRIS);
        common::atomic_owned_write(&path, b"not Anton's").unwrap();
        path
    };
    let m = Mod::new();
    m.install().unwrap();
    assert!(m.entry()["directories"][0] == json!(m.f.home.join(".claude/skills")));
    let path = skills_debris(&m);
    m.install().unwrap();
    assert!(path.is_file(), "install keeps it");
    m.uninstall_mod().unwrap();
    assert!(path.is_file(), "--uninstall-claude-mod keeps it");
    assert!(!m.root().exists());
    let m = everything();
    let path = skills_debris(&m);
    m.uninstall_all().unwrap();
    assert!(path.is_file(), "--uninstall-hooks keeps it");
    // In the mod root: removal keeps it and so the mod directory.
    let m = Mod::new();
    m.install().unwrap();
    let path = m.root().join(DEBRIS);
    common::atomic_owned_write(&path, b"not Anton's").unwrap();
    m.uninstall_mod().unwrap();
    assert!(path.is_file(), "removal keeps it");
    assert!(!m.file("hooks").exists());
}

/// D5: a symlink in any component refuses the removal before any deletion
/// and names that symlink, not the recorded path below it, so the user
/// knows which link to replace (review round 2).
#[test]
fn removal_through_a_symlink_refuses_and_names_the_link() {
    use std::os::unix::fs::symlink;
    for (what, link) in [
        ("symlinked ~/.claude", ".claude"),
        (
            "symlinked mod directory",
            ".claude/skills/anton-observatory",
        ),
        (
            "symlinked hooks directory",
            ".claude/skills/anton-observatory/hooks",
        ),
        (
            "symlinked recorded file",
            ".claude/skills/anton-observatory/hooks/register.js",
        ),
    ] {
        let m = everything();
        let link = m.f.home.join(link);
        let real = m.f.home.join("moved-real");
        std::fs::rename(&link, &real).unwrap();
        symlink(&real, &link).unwrap();
        let before = m.snapshot();
        let message = format!(
            "Refusing Claude Code mod removal through symlink {}; replace it with the real directory or file, then retry",
            link.display()
        );
        for result in [m.uninstall_all(), m.uninstall_mod()] {
            assert_eq!(result, Err(message.clone()), "{what}");
            assert_eq!(m.snapshot(), before, "{what}: everything is kept");
        }
        // Replacing the link with the real entry lets the removal proceed.
        std::fs::remove_file(&link).unwrap();
        std::fs::rename(&real, &link).unwrap();
        m.uninstall_all().unwrap_or_else(|e| panic!("{what}: {e}"));
        assert!(!m.root().exists(), "{what}");
    }
}

/// Review round 2: removal syncs every surviving recorded directory and the
/// surviving parent of each removed one, before the receipt changes.
#[test]
fn removal_syncs_the_surviving_directories() {
    for extra in [false, true] {
        let m = Mod::new();
        m.install().unwrap();
        if extra {
            common::atomic_owned_write(&m.file("hooks/extra.js"), b"x").unwrap();
        }
        let recorded = claude_mod::removal(&m.f.home, &m.f.receipt(), &m.env)
            .unwrap()
            .unwrap();
        let mut synced = Vec::new();
        claude_mod::remove_with(&m.f.home, &recorded, &mut |path| {
            synced.push(path.to_owned());
            Ok(())
        })
        .unwrap();
        synced.sort();
        let expected = if extra {
            vec![m.f.home.join(".claude/skills"), m.root(), m.file("hooks")]
        } else {
            vec![m.f.home.join(".claude")]
        };
        assert_eq!(synced, expected, "extra file: {extra}");
    }
}

#[test]
fn removal_keeps_unrecorded_files_and_reports_success() {
    let m = Mod::new();
    m.install().unwrap();
    let extra = m.file("hooks/extra.js");
    common::atomic_owned_write(&extra, b"not Anton's").unwrap();
    std::fs::remove_file(m.file(".claude-plugin/plugin.json")).unwrap();
    m.uninstall_mod().unwrap();
    assert_eq!(std::fs::read(&extra).unwrap(), b"not Anton's");
    for name in CLAUDE_MOD_FILES {
        assert!(!m.file(name).exists(), "{name}");
    }
    assert!(!m.file(".claude-plugin").exists());
    assert!(m.f.receipt().get("claude_mod").is_none());
}

#[test]
fn removal_runs_the_chezmoi_check_only() {
    let m = Mod::new();
    m.install().unwrap();
    m.mise(&json!({"entries":[{"path":"~/.claude/skills/anton-observatory"}]}));
    common::ensure_private_directory(&m.f.home.join(".git")).unwrap();
    common::atomic_owned_write(&m.f.home.join(".git/HEAD"), b"ref: refs/heads/main\n").unwrap();
    m.uninstall_mod().unwrap();
    assert!(
        !m.root().exists(),
        "mise and Git markers do not block removal"
    );
    let m = everything();
    m.tool("chezmoi", "exit 0");
    let before = m.snapshot();
    for result in [m.uninstall_mod(), m.uninstall_all()] {
        assert_eq!(
            result,
            Err("Edit managed integration through chezmoi".into())
        );
        assert_eq!(m.snapshot(), before);
    }
}

#[test]
fn uninstall_hooks_removes_pi_the_shim_and_the_mod_together() {
    let m = everything();
    let codex = paths(&m.f.home).2;
    let config = std::fs::read(&codex).unwrap();
    m.uninstall_all().unwrap();
    assert!(!paths(&m.f.home).1.exists());
    assert!(!m.f.shell().exists());
    assert!(!m.f.home.join(".claude/skills").exists());
    assert!(m.f.home.join(".claude").is_dir());
    assert!(!m.f.root.join(".hooks-receipt.json").exists());
    assert_eq!(std::fs::read(codex).unwrap(), config);
    m.uninstall_all().unwrap();
}

/// Regression guard: the Pi extension source is byte-identical to `80f6295`.
#[test]
fn pi_extension_bytes_are_unchanged() {
    assert_eq!(
        common::sha256(include_str!("../../../../hooks/observatory.ts").as_bytes()),
        "d9a998b59079d264cdfb9de61ac45ef4e55c1d827764c8759aa7fd7952f2fb76"
    );
}

/// Regression guard: `--install-hooks` and `--repair-retired-hooks` keep a
/// recorded mod entry, so the mod stays owned.
#[test]
fn pi_receipt_writers_keep_the_mod_entry() {
    let m = Mod::new();
    m.install().unwrap();
    let entry = m.entry();
    install(&m.f.root, &m.f.home, false).unwrap();
    assert_eq!(m.entry(), entry);
    m.f.backup();
    repair_retired(&m.f.root, &m.f.home).unwrap();
    assert_eq!(m.entry(), entry);
    assert!(claude_mod_recorded(&m.f.root, &m.f.home));
}
