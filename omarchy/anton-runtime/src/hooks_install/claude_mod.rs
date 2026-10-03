//! The Claude Code mod (design D1, D5): hash-proven install, refresh and
//! removal of the files recorded in the hook receipt's `claude_mod` entry.
use super::{
    CLAUDE_MOD_FILES, LIMIT, RECEIPT_WAIT, claude_mod_entry, claude_mod_root, native_extension,
    paths, receipt_lock, receipt_value, regular, runtime_owned,
};
use crate::{Result, common};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

const PLUGIN_JSON: &str =
    include_str!("../../../../hooks/claude/anton-observatory/.claude-plugin/plugin.json");
const HOOKS_JSON: &str =
    include_str!("../../../../hooks/claude/anton-observatory/hooks/hooks.json");
const REGISTER_JS: &str =
    include_str!("../../../../hooks/claude/anton-observatory/hooks/register.js");
const PLACEHOLDER: &str = "const nativeRuntime = '';";
const MISE_WAIT: Duration = Duration::from_secs(3);

/// The environment the mod checks consult. It is passed explicitly so that
/// fixtures never read or change the process environment (design D7).
#[derive(Clone, Debug, Default)]
pub(crate) struct ClaudeEnv {
    /// `PATH`, used to find `chezmoi` and `mise`.
    pub path: Option<OsString>,
    /// `CLAUDE_CONFIG_DIR`.
    pub config_dir: Option<OsString>,
}
impl ClaudeEnv {
    pub(crate) fn process() -> Self {
        Self {
            path: std::env::var_os("PATH"),
            config_dir: std::env::var_os("CLAUDE_CONFIG_DIR"),
        }
    }
}

/// The mod files for `runtime` with the runtime declared in `register.js`
/// exactly as the Pi extension declares it. They are listed in write order
/// (D5 step 4): the manifest last, so a session that starts mid-install
/// finds no manifest whose module is missing.
pub(super) fn payload(runtime: &Path) -> Result<Vec<(&'static str, Vec<u8>)>> {
    let declaration = format!(
        "const nativeRuntime = {};",
        json!(runtime.to_string_lossy())
    );
    if REGISTER_JS.matches(PLACEHOLDER).count() != 1 {
        return Err("Invalid Claude Code mod payload".into());
    }
    let register = REGISTER_JS.replacen(PLACEHOLDER, &declaration, 1);
    Ok(vec![
        ("hooks/hooks.json", HOOKS_JSON.as_bytes().to_vec()),
        ("hooks/register.js", register.into_bytes()),
        (
            ".claude-plugin/plugin.json",
            PLUGIN_JSON.as_bytes().to_vec(),
        ),
    ])
}

/// An executable named `name` in an absolute directory of `path`. Relative
/// entries are skipped, because the mise check runs in the home directory.
fn executable(path: Option<&OsStr>, name: &str) -> Option<PathBuf> {
    std::env::split_paths(path?)
        .filter(|dir| dir.is_absolute())
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
}
/// The chezmoi check of `managed`, with `PATH` taken from `env`.
pub(super) fn chezmoi_managed(path: &Path, env: &ClaudeEnv) -> Result<()> {
    if let Some(chezmoi) = executable(env.path.as_deref(), "chezmoi") {
        let args = vec![
            chezmoi.to_string_lossy().into_owned(),
            "source-path".into(),
            path.to_string_lossy().into_owned(),
        ];
        if common::run_bounded(&args, &[], Duration::from_secs(2), 4096, None).is_ok() {
            return Err("Edit managed integration through chezmoi".into());
        }
    }
    Ok(())
}
/// A mise dotfiles `path`, with `~` expanded against `home`. A relative path
/// is taken as relative to `home`, where the command runs. `None` (refuse)
/// for another user's `~name` or a `..` component.
fn mise_path(value: &str, home: &Path) -> Option<PathBuf> {
    let path = if value == "~" {
        home.to_owned()
    } else if let Some(rest) = value.strip_prefix("~/") {
        home.join(rest)
    } else if value.starts_with('~') {
        return None;
    } else {
        home.join(value)
    };
    (!value.is_empty() && !path.components().any(|c| c == Component::ParentDir)).then_some(path)
}
/// D5 mise check. Without `mise` on `PATH` the target is not mise-managed.
/// Otherwise every item of `entries`, `incomplete`, `invalid`, `nested` and
/// `omitted` that equals, contains or is inside `target` refuses, and any
/// failure to read the list refuses too.
fn mise_managed(target: &Path, home: &Path, env: &ClaudeEnv) -> Result<()> {
    let Some(mise) = executable(env.path.as_deref(), "mise") else {
        return Ok(());
    };
    let unknown = || String::from("Cannot confirm mise dotfiles; refusing the Claude Code mod");
    let argv = [
        mise.as_os_str(),
        OsStr::new("-C"),
        home.as_os_str(),
        OsStr::new("dotfiles"),
        OsStr::new("paths"),
        OsStr::new("--json"),
    ]
    .map(|value| value.to_string_lossy().into_owned());
    let output = common::run_process_in(&argv, None, Some(home), MISE_WAIT, LIMIT, None)
        .map_err(|_| unknown())?;
    if output.status != 0 {
        return Err(unknown());
    }
    let listing: Value = serde_json::from_slice(&output.stdout).map_err(|_| unknown())?;
    if !listing.get("entries").is_some_and(Value::is_array) {
        return Err(unknown());
    }
    for key in ["entries", "incomplete", "invalid", "nested", "omitted"] {
        let Some(items) = listing.get(key) else {
            continue;
        };
        for item in items.as_array().ok_or_else(unknown)? {
            let path = item["path"]
                .as_str()
                .and_then(|value| mise_path(value, home))
                .ok_or_else(unknown)?;
            if path.starts_with(target) || target.starts_with(&path) {
                return Err("Claude Code mod directory is managed by mise dotfiles".into());
            }
        }
    }
    Ok(())
}
/// D5 Git check: a real repository marker in any directory from `target` up
/// to and including `home`. An empty `.git` directory does not count. Git
/// follows a symlinked `.git`, so any `.git` symlink, resolvable or not,
/// counts as a marker.
fn git_managed(target: &Path, home: &Path) -> Result<()> {
    use std::io::Read;
    for directory in target.ancestors().take_while(|d| d.starts_with(home)) {
        let marker = directory.join(".git");
        let Ok(info) = std::fs::symlink_metadata(&marker) else {
            continue;
        };
        let repository = if info.file_type().is_symlink() {
            true
        } else if info.is_dir() {
            std::fs::symlink_metadata(marker.join("HEAD")).is_ok_and(|head| head.is_file())
        } else if info.is_file() {
            let mut first = Vec::new();
            std::fs::File::open(&marker)
                .and_then(|file| file.take(4096).read_to_end(&mut first))
                .map_err(|_| "Cannot inspect Git marker")?;
            first.starts_with(b"gitdir:")
        } else {
            false
        };
        if repository {
            return Err("Claude Code mod directory is inside a Git repository".into());
        }
    }
    Ok(())
}
/// The managed-configuration checks for the mod directory `target` (D5):
/// chezmoi for it and each mod file, mise dotfiles, then Git markers.
pub(crate) fn claude_managed(target: &Path, home: &Path, env: &ClaudeEnv) -> Result<()> {
    chezmoi_managed(target, env)?;
    for name in CLAUDE_MOD_FILES {
        chezmoi_managed(&target.join(name), env)?;
    }
    mise_managed(target, home, env)?;
    git_managed(target, home)
}
/// D1: `CLAUDE_CONFIG_DIR` unset or naming `~/.claude`. An empty value
/// names no directory and refuses.
fn config_dir(home: &Path, env: &ClaudeEnv) -> Result<()> {
    match &env.config_dir {
        Some(value) if Path::new(value) != home.join(".claude") => {
            Err("CLAUDE_CONFIG_DIR names another Claude Code configuration; refusing".into())
        }
        _ => Ok(()),
    }
}

fn user() -> u32 {
    unsafe { libc::getuid() }
}
/// Refuses a symlink at `path` or any of its ancestors, as `regular` does.
fn no_symlink(path: &Path) -> Result<()> {
    if path
        .ancestors()
        .any(|part| std::fs::symlink_metadata(part).is_ok_and(|m| m.file_type().is_symlink()))
    {
        return Err("Refusing integration symlink".into());
    }
    Ok(())
}
/// Whether `path` is an existing real directory owned by the user; absent is
/// `false` and anything else refuses.
fn real_directory(path: &Path) -> Result<bool> {
    no_symlink(path)?;
    match std::fs::symlink_metadata(path) {
        Ok(info) if info.is_dir() && info.uid() == user() => Ok(true),
        Ok(_) => Err(format!(
            "Conflicting Claude Code mod entry: {}; preserving it",
            path.display()
        )),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(_) => Err("Cannot inspect Claude Code mod directory".into()),
    }
}
fn sync_directory(path: &Path) -> Result<()> {
    common::open_directory(path)?
        .sync_all()
        .map_err(|_| "Cannot sync hook directory".into())
}
/// `atomic_write`'s temporary file name, `.anton-write-<pid>-<bits>`.
fn debris_name(name: &OsStr) -> bool {
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    name.to_str()
        .and_then(|name| name.strip_prefix(".anton-write-"))
        .and_then(|rest| rest.split_once('-'))
        .is_some_and(|(pid, bits)| digits(pid) && digits(bits))
}
/// Deletes installer debris (D5): regular, user-owned, non-symlink files
/// with a debris name in the recorded mod directories that receive atomic
/// writes, `.claude-plugin/` and `hooks/`, when they exist. A debris-named
/// entry anywhere else, or a symlink or another type with such a name, is
/// not Anton's: it is kept, and inside the mod directory refused as
/// unrecorded.
fn delete_debris(mod_root: &Path, directories: &[PathBuf]) -> Result<()> {
    let written = [mod_root.join(".claude-plugin"), mod_root.join("hooks")];
    for directory in directories.iter().filter(|d| written.contains(d)) {
        if !real_directory(directory)? {
            continue;
        }
        let entries =
            std::fs::read_dir(directory).map_err(|_| "Cannot inspect Claude Code mod directory")?;
        for entry in entries {
            let entry = entry.map_err(|_| "Cannot inspect Claude Code mod directory")?;
            if !debris_name(&entry.file_name()) {
                continue;
            }
            let path = entry.path();
            if std::fs::symlink_metadata(&path).is_ok_and(|m| m.is_file() && m.uid() == user()) {
                std::fs::remove_file(&path).map_err(|_| "Cannot remove installer debris")?;
            }
        }
    }
    Ok(())
}
/// A recorded `claude_mod` entry, read after `claude_mod_entry` accepted it.
pub(super) struct Recorded {
    pub directories: Vec<PathBuf>,
    /// Each recorded file with its accepted hashes (`sha256`, then any
    /// `prior_sha256`).
    pub files: BTreeMap<PathBuf, Vec<String>>,
}
impl Recorded {
    fn read(entry: &Value) -> Self {
        let text = |value: &Value| value.as_str().map(str::to_owned);
        Self {
            directories: entry["directories"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|v| v.as_str().map(PathBuf::from))
                .collect(),
            files: entry["files"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|file| {
                    let hashes = [&file["sha256"], &file["prior_sha256"]]
                        .into_iter()
                        .filter_map(text)
                        .collect();
                    Some((PathBuf::from(file["path"].as_str()?), hashes))
                })
                .collect(),
        }
    }
    fn accepts(&self, path: &Path, bytes: &[u8]) -> bool {
        let hash = common::sha256(bytes);
        self.files
            .get(path)
            .is_some_and(|hashes| hashes.contains(&hash))
    }
}
/// The D5 target-state check: the mod directory is absent, or every entry
/// under it is a recorded, user-owned directory or file and each recorded
/// file present has an accepted hash. Returns the hash of each present file.
fn inspect(mod_root: &Path, recorded: Option<&Recorded>) -> Result<BTreeMap<PathBuf, String>> {
    let mut present = BTreeMap::new();
    if !real_directory(mod_root)? {
        return Ok(present);
    }
    let Some(recorded) = recorded else {
        return Err("Conflicting Claude Code mod directory; preserving it".into());
    };
    let conflict = |path: &Path| {
        format!(
            "Conflicting Claude Code mod entry: {}; preserving it",
            path.display()
        )
    };
    let mut pending = vec![mod_root.to_owned()];
    while let Some(directory) = pending.pop() {
        if !recorded.directories.contains(&directory) || !real_directory(&directory)? {
            return Err(conflict(&directory));
        }
        let entries = std::fs::read_dir(&directory)
            .map_err(|_| "Cannot inspect Claude Code mod directory")?;
        for entry in entries {
            let path = entry
                .map_err(|_| "Cannot inspect Claude Code mod directory")?
                .path();
            let info =
                std::fs::symlink_metadata(&path).map_err(|_| "Cannot inspect Claude Code mod")?;
            if info.is_dir() {
                pending.push(path);
            } else if info.is_file() && recorded.files.contains_key(&path) {
                let bytes = regular(&path)?.ok_or_else(|| conflict(&path))?;
                if !recorded.accepts(&path, &bytes) {
                    return Err(format!(
                        "Claude Code mod file changed: {}; preserving it",
                        path.display()
                    ));
                }
                present.insert(path, common::sha256(&bytes));
            } else {
                return Err(conflict(&path));
            }
        }
    }
    Ok(present)
}

/// Builds the mod files for a runtime path, in write order.
pub(super) type Payload<'a> = &'a dyn Fn(&Path) -> Result<Vec<(&'static str, Vec<u8>)>>;
/// Writes one receipt or mod file; fixtures record the order.
pub(super) type Writer<'a> = &'a mut dyn FnMut(&Path, &[u8]) -> Result<()>;

/// `--install-claude-mod`: install or refresh the mod (design D5).
pub fn install_claude_mod(root: &Path, home: &Path) -> Result<()> {
    install_mod(
        root,
        home,
        &ClaudeEnv::process(),
        &payload,
        &mut |path, bytes| common::atomic_owned_write(path, bytes),
    )
}
/// `--uninstall-claude-mod`: remove only the mod and its receipt entry.
pub fn uninstall_claude_mod(root: &Path, home: &Path) -> Result<()> {
    uninstall_mod(root, home, &ClaudeEnv::process(), &mut |path, bytes| {
        common::atomic_owned_write(path, bytes)
    })
}

/// D5 install and refresh: preflight, then the dual-hash order. The receipt
/// first records every new hash, keeping the verified on-disk hash of each
/// file that changes as `prior_sha256`; then directories and files are
/// written, the manifest last; then the receipt keeps only the new hashes.
pub(super) fn install_mod(
    root: &Path,
    home: &Path,
    env: &ClaudeEnv,
    payload: Payload,
    write: Writer,
) -> Result<()> {
    if root
        .file_name()
        .is_some_and(|name| name == "herdr.observatory-peer")
    {
        return Err("The Claude Code mod is local only; refusing a peer installation".into());
    }
    let _owner = common::owner_guard(&root.join(".herdr-observatory-install"))?;
    let _receipt = receipt_lock(root, RECEIPT_WAIT)?;
    config_dir(home, env)?;
    let runtime = runtime_owned(root)?;
    let (_, extension, _) = paths(home);
    let receipt_path = root.join(".hooks-receipt.json");
    let mut receipt = receipt_value(
        &regular(&receipt_path)?.ok_or("Native hook receipt missing; install hooks first")?,
        &runtime,
        &extension,
    )?;
    native_extension(
        &receipt,
        &regular(&extension)?.ok_or("Native Pi extension missing")?,
        &runtime,
    )?;
    if !real_directory(&home.join(".claude")).unwrap_or(false) {
        return Err("Claude Code configuration not found".into());
    }
    let mod_root = claude_mod_root(home);
    claude_managed(&mod_root, home, env)?;
    let prior = match receipt.get("claude_mod") {
        None => None,
        Some(entry) if claude_mod_entry(entry, &mod_root) => Some(entry.clone()),
        Some(_) => return Err("Conflicting Claude Code mod owner".into()),
    };
    let recorded = prior.as_ref().map(Recorded::read);
    if let Some(recorded) = &recorded {
        delete_debris(&mod_root, &recorded.directories)?;
    }
    let present = inspect(&mod_root, recorded.as_ref())?;
    let files = payload(&runtime)?;
    let skills = mod_root
        .parent()
        .ok_or("Invalid Claude Code mod directory")?;
    let mut directories = recorded
        .map(|recorded| recorded.directories)
        .unwrap_or_default();
    let mut create = Vec::new();
    for directory in [
        skills.to_owned(),
        mod_root.clone(),
        mod_root.join(".claude-plugin"),
        mod_root.join("hooks"),
    ] {
        if !real_directory(&directory)? {
            if !directories.contains(&directory) {
                directories.push(directory.clone());
            }
            create.push(directory);
        }
    }
    let mut entry_files = Vec::new();
    let mut dual = false;
    for name in CLAUDE_MOD_FILES {
        let (_, bytes) = files
            .iter()
            .find(|(file, _)| *file == name)
            .ok_or("Invalid Claude Code mod payload")?;
        let path = mod_root.join(name);
        let hash = common::sha256(bytes);
        let mut file = json!({"path":path,"sha256":hash});
        if let Some(disk) = present.get(&path).filter(|disk| **disk != hash) {
            file["prior_sha256"] = json!(disk);
            dual = true;
        }
        entry_files.push(file);
    }
    let entry = json!({"version":1,"root":mod_root,"directories":directories,"files":entry_files});
    if !claude_mod_entry(&entry, &mod_root) {
        return Err("Invalid Claude Code mod receipt".into());
    }
    if prior.as_ref() != Some(&entry) {
        receipt["claude_mod"] = entry;
        write(
            &receipt_path,
            &serde_json::to_vec(&receipt).map_err(|_| "Invalid hook receipt")?,
        )?;
        // The receipt rename reaches disk before any mod file rename.
        sync_directory(root)?;
    }
    for directory in &create {
        use std::os::unix::fs::DirBuilderExt;
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(directory)
            .map_err(|_| "Cannot create Claude Code mod directory")?;
        if !real_directory(directory)? {
            return Err("Cannot create Claude Code mod directory".into());
        }
    }
    for (name, bytes) in &files {
        let path = mod_root.join(name);
        if present.get(&path) != Some(&common::sha256(bytes)) {
            write(&path, bytes)?;
        }
    }
    if dual {
        for directory in [mod_root.join(".claude-plugin"), mod_root.join("hooks")] {
            sync_directory(&directory)?;
        }
        for file in receipt["claude_mod"]["files"]
            .as_array_mut()
            .into_iter()
            .flatten()
        {
            if let Some(file) = file.as_object_mut() {
                file.remove("prior_sha256");
            }
        }
        write(
            &receipt_path,
            &serde_json::to_vec(&receipt).map_err(|_| "Invalid hook receipt")?,
        )?;
        sync_directory(root)?;
    }
    Ok(())
}

/// D5 removal step 1, before any Pi, shim or mod deletion: every recorded
/// directory is real, and every recorded file is absent or a regular,
/// user-owned file with an accepted hash that chezmoi does not manage.
/// `None` when the receipt records no mod.
pub(super) fn removal(home: &Path, receipt: &Value, env: &ClaudeEnv) -> Result<Option<Recorded>> {
    let Some(entry) = receipt.get("claude_mod") else {
        return Ok(None);
    };
    if !claude_mod_entry(entry, &claude_mod_root(home)) {
        return Err("Conflicting Claude Code mod owner".into());
    }
    let recorded = Recorded::read(entry);
    let changed = |path: &Path| {
        format!(
            "Claude Code mod file changed: {}; restore or remove it, then retry",
            path.display()
        )
    };
    for directory in &recorded.directories {
        real_directory(directory).map_err(|_| changed(directory))?;
    }
    for path in recorded.files.keys() {
        match regular(path) {
            Ok(None) => {}
            Ok(Some(bytes)) if recorded.accepts(path, &bytes) => chezmoi_managed(path, env)?,
            _ => return Err(changed(path)),
        }
    }
    Ok(Some(recorded))
}
/// D5 removal step 2, after the preflight: the recorded files, any debris,
/// then each recorded directory, deepest first, if it is empty. A kept mod
/// directory holds files Anton never wrote; it is reported, not an error.
pub(super) fn remove(home: &Path, recorded: &Recorded) -> Result<()> {
    for path in recorded.files.keys() {
        match std::fs::remove_file(path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err("Claude Code mod removal failed".into()),
        }
    }
    let mod_root = claude_mod_root(home);
    delete_debris(&mod_root, &recorded.directories)?;
    // Deepest first: a refresh can record a recreated parent after its children.
    let mut directories: Vec<&PathBuf> = recorded.directories.iter().collect();
    directories.sort_by_key(|directory| std::cmp::Reverse(directory.components().count()));
    for directory in directories {
        let _ = std::fs::remove_dir(directory);
        if directory.starts_with(&mod_root) && std::fs::symlink_metadata(directory).is_ok() {
            eprintln!(
                "Kept {}: it holds files the Claude Code mod installer did not write",
                directory.display()
            );
        }
    }
    Ok(())
}
pub(super) fn uninstall_mod(
    root: &Path,
    home: &Path,
    env: &ClaudeEnv,
    write: Writer,
) -> Result<()> {
    let _receipt = receipt_lock(root, RECEIPT_WAIT)?;
    let receipt_path = root.join(".hooks-receipt.json");
    let Some(bytes) = regular(&receipt_path)? else {
        return Ok(());
    };
    super::owner_marker(root)?;
    let (_, extension, _) = paths(home);
    let mut receipt = receipt_value(&bytes, &root.join("anton-runtime"), &extension)?;
    let Some(recorded) = removal(home, &receipt, env)? else {
        return Ok(());
    };
    remove(home, &recorded)?;
    receipt
        .as_object_mut()
        .ok_or("Invalid hook receipt")?
        .remove("claude_mod");
    write(
        &receipt_path,
        &serde_json::to_vec(&receipt).map_err(|_| "Invalid hook receipt")?,
    )
}
