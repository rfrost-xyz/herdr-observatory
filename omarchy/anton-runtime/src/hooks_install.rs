//! Explicit, repeatable Pi integration and removal of proven-owned legacy hooks.
use crate::{Result, common};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const RETIRED_CODEX_SHIM: &[u8] = b"#!/bin/sh\n# herdr-observatory retired Codex hook v1\ncat >/dev/null 2>/dev/null || :\nexit 0\n";
pub const RETIRED_CODEX_SHA256: &str =
    "0b6dabc77c3845f29cc93defaff8b38b8c79319254e1d7e321997cdeec5a5b73";
const LEGACY: &str = "herdr-observatory adapter v1";
const MARKER: &str = "herdr-observatory native adapter v2";
const EVENTS: &[&str] = &[
    "SessionStart",
    "UserPromptSubmit",
    "PreToolUse",
    "PostToolUse",
    "PreCompact",
    "PostCompact",
    "Stop",
    "Interrupt",
    "SessionEnd",
    "SubagentStart",
    "SubagentStop",
];
const LIMIT: usize = 1_048_576;

fn regular(path: &Path) -> Result<Option<Vec<u8>>> {
    let mut parent = Some(path);
    while let Some(part) = parent {
        if std::fs::symlink_metadata(part).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err("Refusing integration symlink".into());
        }
        parent = part.parent();
    }
    match std::fs::symlink_metadata(path) {
        Ok(_) => common::read_owned(path, LIMIT, false).map(Some),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err("Integration unavailable".into()),
    }
}
fn managed(path: &Path) -> Result<()> {
    if std::env::var_os("PATH")
        .is_some_and(|p| std::env::split_paths(&p).any(|p| p.join("chezmoi").is_file()))
    {
        let args = vec![
            "chezmoi".into(),
            "source-path".into(),
            path.to_string_lossy().into_owned(),
        ];
        if common::run_bounded(&args, &[], Duration::from_secs(2), 4096, None).is_ok() {
            return Err("Edit managed integration through chezmoi".into());
        }
    }
    Ok(())
}
fn marked(bytes: &[u8], marker: &str) -> bool {
    std::str::from_utf8(bytes).is_ok_and(|s| s.lines().take(2).any(|s| s.contains(marker)))
}
fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}
fn legacy_commands(shell: &Path) -> Vec<String> {
    let value = shell.to_string_lossy();
    vec![format!("sh {value}"), format!("sh {}", shell_quote(&value))]
}
fn strip_callbacks(mut config: Value, commands: &[String]) -> Result<Value> {
    let root = config.as_object_mut().ok_or("Invalid hook configuration")?;
    let Some(hooks) = root.get_mut("hooks") else {
        return Ok(config);
    };
    let hooks = hooks.as_object_mut().ok_or("Invalid hook configuration")?;
    for event in EVENTS {
        let Some(entries) = hooks.get_mut(*event) else {
            continue;
        };
        let entries = entries.as_array_mut().ok_or("Invalid hook entries")?;
        for entry in entries.iter_mut() {
            let calls = entry
                .get_mut("hooks")
                .and_then(Value::as_array_mut)
                .ok_or("Invalid hook entry")?;
            calls.retain(|call| {
                !(call["type"] == "command"
                    && call["command"]
                        .as_str()
                        .is_some_and(|c| commands.iter().any(|known| known == c)))
            });
        }
        entries.retain(|entry| entry["hooks"].as_array().is_some_and(|a| !a.is_empty()));
        if entries.is_empty() {
            hooks.remove(*event);
        }
    }
    Ok(config)
}
fn paths(home: &Path) -> (PathBuf, PathBuf, PathBuf) {
    (
        home.join(".local/share/herdr-observatory/hooks"),
        home.join(".pi/agent/extensions/observatory.ts"),
        home.join(".codex/hooks.json"),
    )
}

fn runtime_owned(root: &Path) -> Result<PathBuf> {
    use std::os::unix::fs::MetadataExt;
    let runtime = root.join("anton-runtime");
    let file = common::open_owned(&runtime, false, false)?;
    let metadata = file.metadata().map_err(|_| "Runtime missing")?;
    if metadata.mode() & 0o111 == 0 {
        return Err("Unsafe runtime".into());
    }
    Ok(runtime)
}
fn receipt_value(bytes: &[u8], runtime: &Path, extension: &Path) -> Result<Value> {
    let receipt: Value = serde_json::from_slice(bytes).map_err(|_| "Invalid hook receipt")?;
    if receipt["version"] != 1
        || receipt["runtime"] != runtime.to_string_lossy().as_ref()
        || receipt["extension"] != extension.to_string_lossy().as_ref()
    {
        return Err("Conflicting hook owner".into());
    }
    Ok(receipt)
}
/// The Claude Code mod directory under `home` (design D1).
pub(crate) fn claude_mod_root(home: &Path) -> PathBuf {
    home.join(".claude/skills/anton-observatory")
}
/// The mod files a `claude_mod` receipt entry records, relative to its root.
pub(crate) const CLAUDE_MOD_FILES: [&str; 3] = [
    ".claude-plugin/plugin.json",
    "hooks/hooks.json",
    "hooks/register.js",
];
/// Whether `entry` has the D5 `claude_mod` shape for the mod directory
/// `root`: version 1, that root, absolute `directories` at or under it (or
/// its `skills` parent), and exactly the three mod files, each with a
/// SHA-256 and, while a refresh is in progress, a `prior_sha256`.
pub(crate) fn claude_mod_entry(entry: &Value, root: &Path) -> bool {
    let skills = root.parent();
    let directories = entry["directories"].as_array().is_some_and(|list| {
        !list.is_empty()
            && list.iter().all(|value| {
                value.as_str().map(Path::new).is_some_and(|path| {
                    path.is_absolute() && (path.starts_with(root) || Some(path) == skills)
                })
            })
    });
    let hash = |value: &Value| value.as_str().is_some_and(|v| common::hex_id(v, 64));
    let files = entry["files"].as_array().is_some_and(|list| {
        let mut paths: Vec<_> = list
            .iter()
            .filter_map(|file| file["path"].as_str())
            .collect();
        paths.sort_unstable();
        let mut expected = CLAUDE_MOD_FILES.map(|name| root.join(name));
        expected.sort_unstable();
        list.len() == CLAUDE_MOD_FILES.len()
            && paths
                .iter()
                .map(Path::new)
                .eq(expected.iter().map(PathBuf::as_path))
            && list
                .iter()
                .all(|file| hash(&file["sha256"]) && file.get("prior_sha256").is_none_or(&hash))
    });
    entry["version"] == 1
        && entry["root"].as_str().map(Path::new) == Some(root)
        && directories
        && files
}
/// Reporter guard 2 (design D3): the plugin's hook receipt, owned by this
/// runtime, records the Claude Code mod for `home`. An entry left
/// mid-refresh, with `prior_sha256` values, is accepted.
pub fn claude_mod_recorded(root: &Path, home: &Path) -> bool {
    let (_, extension, _) = paths(home);
    common::read_owned(&root.join(".hooks-receipt.json"), LIMIT, false)
        .and_then(|bytes| receipt_value(&bytes, &root.join("anton-runtime"), &extension))
        .is_ok_and(|receipt| claude_mod_entry(&receipt["claude_mod"], &claude_mod_root(home)))
}
fn native_extension(receipt: &Value, bytes: &[u8], runtime: &Path) -> Result<()> {
    let declaration = format!(
        "const nativeRuntime = {};",
        json!(runtime.to_string_lossy())
    );
    if !marked(bytes, MARKER)
        || receipt["sha256"] != common::sha256(bytes)
        || !String::from_utf8_lossy(bytes).contains(&declaration)
    {
        return Err("Pi extension changed; preserving it".into());
    }
    Ok(())
}
fn compatibility(receipt: &Value, shell: &Path) -> Result<bool> {
    let Some(value) = receipt.get("compatibility") else {
        return Ok(false);
    };
    if value.as_object().is_none_or(|v| v.len() != 2)
        || value["path"] != shell.to_string_lossy().as_ref()
        || value["sha256"] != RETIRED_CODEX_SHA256
    {
        return Err("Conflicting retired hook owner".into());
    }
    Ok(true)
}
fn compatibility_directories(home: &Path, create: bool) -> Result<()> {
    use std::os::unix::fs::MetadataExt;
    // System ancestors may belong to root. Home and every managed descendant
    // must be real directories owned by this user before anything is created.
    let owned = |path: &Path| -> Result<()> {
        let directory = common::open_directory(path)?;
        if directory
            .metadata()
            .map_err(|_| "Cannot inspect hook directory")?
            .uid()
            != unsafe { libc::getuid() }
        {
            return Err("Conflicting hook directory owner".into());
        }
        Ok(())
    };
    owned(home)?;
    let mut path = home.to_owned();
    for component in [".local", "share", "herdr-observatory", "hooks"] {
        path.push(component);
        match std::fs::symlink_metadata(&path) {
            Ok(_) => owned(&path)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                if create {
                    common::ensure_private_directory(&path)?;
                }
            }
            Err(_) => return Err("Cannot inspect hook directory".into()),
        }
    }
    Ok(())
}
fn backup_proves_callback(root: &Path, shell: &Path) -> Result<bool> {
    let Some(bytes) = regular(&root.join(".hooks-before-native.json"))? else {
        return Ok(false);
    };
    let original: Value =
        serde_json::from_slice(&bytes).map_err(|_| "Invalid former hooks backup")?;
    Ok(strip_callbacks(original.clone(), &legacy_commands(shell))? != original)
}
fn compatibility_receipt(mut receipt: Value, shell: &Path) -> Value {
    receipt["compatibility"] = json!({"path":shell,"sha256":RETIRED_CODEX_SHA256});
    receipt
}
fn create_shim_exclusive(shell: &Path) -> Result<()> {
    use std::io::Write;
    use std::os::fd::{AsRawFd, FromRawFd};
    use std::os::unix::fs::MetadataExt;
    let parent = common::open_directory(shell.parent().ok_or("Missing hook parent")?)?;
    let name = std::ffi::CString::new(
        shell
            .file_name()
            .ok_or("Missing hook filename")?
            .as_encoded_bytes(),
    )
    .map_err(|_| "Invalid hook filename")?;
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            0o600,
        )
    };
    if fd < 0 {
        return Err("Retired hook path appeared or is unavailable".into());
    }
    let mut file = unsafe { std::fs::File::from_raw_fd(fd) };
    let result = file
        .write_all(RETIRED_CODEX_SHIM)
        .and_then(|_| file.sync_all());
    if result.is_err() {
        // Roll back only the inode just created through this pinned directory.
        let mut current = std::mem::MaybeUninit::<libc::stat>::uninit();
        if let Ok(created) = file.metadata() {
            if unsafe {
                libc::fstatat(
                    parent.as_raw_fd(),
                    name.as_ptr(),
                    current.as_mut_ptr(),
                    libc::AT_SYMLINK_NOFOLLOW,
                )
            } == 0
            {
                let current = unsafe { current.assume_init() };
                if current.st_dev == created.dev() && current.st_ino == created.ino() {
                    unsafe { libc::unlinkat(parent.as_raw_fd(), name.as_ptr(), 0) };
                }
            }
        }
        return Err("Cannot write retired hook".into());
    }
    Ok(())
}
/// Commit ownership intent first. A failed receipt write creates no helper or
/// directory; a later creation failure leaves an owned missing path for retry.
fn record_and_create(
    receipt_path: &Path,
    receipt: &Value,
    home: &Path,
    shell: &Path,
    write_receipt: impl FnOnce(&Path, &[u8]) -> Result<()>,
) -> Result<()> {
    write_receipt(
        receipt_path,
        &serde_json::to_vec(receipt).map_err(|_| "Invalid hook receipt")?,
    )?;
    compatibility_directories(home, true)?;
    create_shim_exclusive(shell)
}
pub fn repair_retired(root: &Path, home: &Path) -> Result<()> {
    let _owner = common::owner_guard(&root.join(".herdr-observatory-install"))?;
    let runtime = runtime_owned(root)?;
    let (legacy, extension, _) = paths(home);
    let shell = legacy.join("codex.sh");
    let receipt_path = root.join(".hooks-receipt.json");
    let receipt = receipt_value(
        &regular(&receipt_path)?.ok_or("Native hook receipt missing")?,
        &runtime,
        &extension,
    )?;
    native_extension(
        &receipt,
        &regular(&extension)?.ok_or("Native Pi extension missing")?,
        &runtime,
    )?;
    compatibility_directories(home, false)?;
    let owned = compatibility(&receipt, &shell)?;
    if let Some(bytes) = regular(&shell)? {
        if owned && bytes == RETIRED_CODEX_SHIM {
            return Ok(());
        }
        return Err("Conflicting retired hook; preserving it".into());
    }
    if !backup_proves_callback(root, &shell)? {
        return Err("Exact former hook command proof missing".into());
    }
    managed(&shell)?;
    record_and_create(
        &receipt_path,
        &compatibility_receipt(receipt, &shell),
        home,
        &shell,
        common::atomic_owned_write,
    )
}

pub fn install(root: &Path, home: &Path, adopt_legacy: bool) -> Result<()> {
    let _owner = common::owner_guard(&root.join(".herdr-observatory-install"))?;
    let runtime = runtime_owned(root)?;
    let (legacy, extension, config) = paths(home);
    let receipt_path = root.join(".hooks-receipt.json");
    let existing_receipt = regular(&receipt_path)?;
    let prior_receipt = existing_receipt
        .as_ref()
        .map(|bytes| receipt_value(bytes, &runtime, &extension))
        .transpose()?;
    let existing_extension = regular(&extension)?;
    if existing_extension
        .as_ref()
        .is_some_and(|b| !marked(b, LEGACY) && !marked(b, MARKER))
    {
        return Err("Conflicting Pi extension".into());
    }
    if existing_extension.as_ref().is_some_and(|b| {
        marked(b, LEGACY)
            && !adopt_legacy
            && !String::from_utf8_lossy(b)
                .contains(&root.join("runtime.py").to_string_lossy().to_string())
    }) {
        return Err("Legacy Pi owner requires explicit migration".into());
    }
    if let Some(bytes) = existing_extension.as_ref().filter(|b| marked(b, MARKER)) {
        let expected = format!(
            "const nativeRuntime = {};",
            json!(runtime.to_string_lossy())
        );
        if !String::from_utf8_lossy(bytes).contains(&expected) {
            return Err("Another native installation owns Pi".into());
        }
    }
    if let Some(bytes) = existing_extension.as_ref().filter(|b| marked(b, MARKER)) {
        native_extension(
            prior_receipt
                .as_ref()
                .ok_or("Native Pi ownership receipt missing")?,
            bytes,
            &runtime,
        )?;
    }
    let helper = regular(&legacy.join("codex_usage.py"))?;
    let legacy_owned = helper.as_ref().is_some_and(|b| {
        marked(b, LEGACY)
            && (adopt_legacy
                || String::from_utf8_lossy(b)
                    .contains(&root.join("runtime.py").to_string_lossy().to_string()))
    });
    let mut remove = Vec::new();
    if legacy_owned {
        for name in ["codex_usage.py", "allowances_probe.py"] {
            let path = legacy.join(name);
            if let Some(bytes) = regular(&path)? {
                if !marked(&bytes, LEGACY) {
                    return Err("Conflicting legacy adapter".into());
                }
                remove.push(path);
            }
        }
    }
    let shell = legacy.join("codex.sh");
    compatibility_directories(home, false)?;
    let compatibility_owned = prior_receipt
        .as_ref()
        .map(|r| compatibility(r, &shell))
        .transpose()?
        .unwrap_or(false);
    let shell_bytes = regular(&shell)?;
    if let Some(bytes) = &shell_bytes {
        if !(compatibility_owned && bytes == RETIRED_CODEX_SHIM
            || legacy_owned && marked(bytes, LEGACY))
        {
            return Err("Conflicting retired hook; preserving it".into());
        }
    }
    let preserve_compatibility = legacy_owned
        || compatibility_owned
        || (prior_receipt.is_some() && backup_proves_callback(root, &shell)?);
    if preserve_compatibility {
        managed(&shell)?;
    }
    let original = regular(&config)?;
    let original_value: Value = original
        .as_ref()
        .map(|b| serde_json::from_slice(b))
        .transpose()
        .map_err(|_| "Invalid hook configuration")?
        .unwrap_or_else(|| json!({}));
    let updated = if legacy_owned {
        strip_callbacks(
            original_value.clone(),
            &legacy_commands(&legacy.join("codex.sh")),
        )?
    } else {
        original_value.clone()
    };
    for path in std::iter::once(&extension)
        .chain(remove.iter())
        .chain((updated != original_value).then_some(&config))
    {
        managed(path)?;
    }
    let payload = include_str!("../../../hooks/observatory.ts").replace(
        "const nativeRuntime = '';",
        &format!(
            "const nativeRuntime = {};",
            json!(runtime.to_string_lossy())
        ),
    );
    let mut receipt = prior_receipt.clone().unwrap_or_else(|| json!({}));
    receipt["version"] = json!(1);
    receipt["runtime"] = json!(runtime);
    receipt["extension"] = json!(extension);
    receipt["sha256"] = json!(common::sha256(payload.as_bytes()));
    if preserve_compatibility {
        receipt = compatibility_receipt(receipt, &shell);
    }

    if updated != original_value && regular(&root.join(".hooks-before-native.json"))?.is_none() {
        common::atomic_owned_write(
            &root.join(".hooks-before-native.json"),
            &serde_json::to_vec_pretty(&original_value).map_err(|_| "Invalid hook backup")?,
        )?;
    }
    use std::os::unix::fs::DirBuilderExt;
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(extension.parent().ok_or("Missing extension parent")?)
        .map_err(|_| "Cannot create extension directory")?;
    common::atomic_owned_write(&extension, payload.as_bytes())?;
    let receipt_result = common::atomic_owned_write(
        &receipt_path,
        &serde_json::to_vec(&receipt).map_err(|_| "Invalid hook receipt")?,
    );
    if let Err(error) = receipt_result {
        // Only undo the extension bytes installed by this attempt.
        if regular(&extension)?.as_deref() == Some(payload.as_bytes()) {
            if let Some(bytes) = existing_extension {
                common::atomic_owned_write(&extension, &bytes)?;
            } else {
                std::fs::remove_file(&extension).map_err(|_| "Cannot roll back Pi integration")?;
            }
        }
        return Err(error);
    }
    if preserve_compatibility && shell_bytes.as_deref() != Some(RETIRED_CODEX_SHIM) {
        compatibility_directories(home, true)?;
        if shell_bytes.is_none() {
            create_shim_exclusive(&shell)?;
        } else {
            if regular(&shell)? != shell_bytes {
                return Err("Retired hook changed during migration".into());
            }
            common::atomic_owned_write(&shell, RETIRED_CODEX_SHIM)?;
        }
    }
    if updated != original_value {
        common::atomic_owned_write(
            &config,
            &serde_json::to_vec_pretty(&updated).map_err(|_| "Invalid hook configuration")?,
        )?;
    }
    for path in remove {
        std::fs::remove_file(path).map_err(|_| "Legacy adapter removal failed")?;
    }
    Ok(())
}

pub fn uninstall(root: &Path, home: &Path) -> Result<()> {
    let receipt_path = root.join(".hooks-receipt.json");
    let Some(bytes) = regular(&receipt_path)? else {
        return Ok(());
    };
    let marker =
        regular(&root.join(".herdr-observatory-install"))?.ok_or("Hook owner marker missing")?;
    if ![
        b"herdr.observatory\n".as_slice(),
        b"herdr.observatory",
        b"herdr.observatory:retired\n",
        b"herdr.observatory:retired",
    ]
    .contains(&marker.as_slice())
    {
        return Err("Conflicting hook owner marker".into());
    }
    let runtime = root.join("anton-runtime");
    let (legacy, extension, _) = paths(home);
    let shell = legacy.join("codex.sh");
    let receipt = receipt_value(&bytes, &runtime, &extension)?;
    let extension_bytes = regular(&extension)?;
    if let Some(bytes) = &extension_bytes {
        native_extension(&receipt, bytes, &runtime)?;
        managed(&extension)?;
    }
    let compatibility_owned = compatibility(&receipt, &shell)?;
    let mut shell_present = false;
    if compatibility_owned {
        compatibility_directories(home, false)?;
        if let Some(bytes) = regular(&shell)? {
            if bytes != RETIRED_CODEX_SHIM {
                return Err("Retired hook changed; preserving integrations".into());
            }
            managed(&shell)?;
            shell_present = true;
        }
    }
    // Both integrations have been preflighted before deleting either one.
    if shell_present {
        std::fs::remove_file(&shell).map_err(|_| "Retired hook removal failed")?;
    }
    if extension_bytes.is_some() {
        std::fs::remove_file(&extension).map_err(|_| "Pi removal failed")?;
    }
    std::fs::remove_file(receipt_path).map_err(|_| "Receipt removal failed")?;
    if compatibility_owned {
        // remove_dir only removes empty directories, preserving unrelated files.
        let _ = std::fs::remove_dir(&legacy);
        if let Some(parent) = legacy.parent() {
            let _ = std::fs::remove_dir(parent);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Reporter guard 2 accepts exactly the D5 `claude_mod` shape, also
    /// mid-refresh with `prior_sha256` values.
    #[test]
    fn claude_mod_entry_has_the_receipt_shape() {
        let root = claude_mod_root(Path::new("/home/a"));
        let hash = common::sha256(b"fixture");
        let files: Vec<_> = CLAUDE_MOD_FILES
            .iter()
            .map(|name| json!({"path":root.join(name),"sha256":hash}))
            .collect();
        let entry = json!({"version":1,"root":root,"directories":["/home/a/.claude/skills",root,root.join("hooks")],"files":files});
        assert!(claude_mod_entry(&entry, &root));
        let mut prior = entry.clone();
        prior["files"][2]["prior_sha256"] = json!(hash);
        assert!(claude_mod_entry(&prior, &root));
        let mut broken = Vec::new();
        for (pointer, value) in [
            ("/version", json!(2)),
            ("/root", json!("/home/b/.claude/skills/anton-observatory")),
            ("/directories", json!([])),
            ("/directories/0", json!("/home/a/.claude")),
            ("/directories/0", json!("relative")),
            ("/files/0/sha256", json!("short")),
            ("/files/0/path", json!("/home/a/other.json")),
            ("/files", json!(files[..2])),
        ] {
            let mut value_entry = entry.clone();
            *value_entry.pointer_mut(pointer).unwrap() = value;
            broken.push(value_entry);
        }
        let mut bad_prior = prior.clone();
        bad_prior["files"][2]["prior_sha256"] = json!("short");
        broken.push(bad_prior);
        let mut extra = entry.clone();
        extra["files"]
            .as_array_mut()
            .unwrap()
            .push(files[0].clone());
        broken.push(extra);
        broken.push(json!(null));
        for entry in broken {
            assert!(!claude_mod_entry(&entry, &root), "{entry}");
        }
    }
    #[test]
    fn removes_only_exact_owned_callbacks() {
        let original = json!({"other":true,"hooks":{"Stop":[{"matcher":"*","hooks":[{"type":"command","command":"sh /owned"},{"type":"command","command":"herdr native"}]}],"Custom":[{"hooks":[{"type":"command","command":"sh /owned"}]}]}});
        let updated = strip_callbacks(original.clone(), &["sh /owned".into()]).unwrap();
        assert_eq!(
            updated["hooks"]["Stop"][0]["hooks"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(updated["hooks"]["Custom"], original["hooks"]["Custom"]);
        assert_eq!(updated["other"], true);
    }
    #[test]
    fn malformed_entries_fail_before_mutation() {
        assert!(strip_callbacks(json!({"hooks":{"Stop":[{"hooks":null}]}}), &[]).is_err());
    }
    #[test]
    fn repeat_install_migration_and_uninstall_preserve_unrelated_hooks() {
        use std::os::unix::fs::PermissionsExt;
        let home = std::env::temp_dir().join(format!(
            "anton-hooks-{}-{}-{}",
            std::process::id(),
            common::now().to_bits(),
            {
                static SEQUENCE: std::sync::atomic::AtomicUsize =
                    std::sync::atomic::AtomicUsize::new(0);
                SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            }
        ));
        let root = home.join("plugin");
        let (legacy, extension, config) = paths(&home);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&legacy).unwrap();
        std::fs::create_dir_all(config.parent().unwrap()).unwrap();
        common::atomic_owned_write(
            &root.join(".herdr-observatory-install"),
            b"herdr.observatory\n",
        )
        .unwrap();
        common::atomic_owned_write(&root.join("anton-runtime"), b"fixture").unwrap();
        std::fs::set_permissions(
            root.join("anton-runtime"),
            std::fs::Permissions::from_mode(0o700),
        )
        .unwrap();
        for name in ["codex.sh", "codex_usage.py", "allowances_probe.py"] {
            let content = format!(
                "# {LEGACY}\nNATIVE_RUNTIME = {:?}\n",
                root.join("runtime.py")
            );
            common::atomic_owned_write(&legacy.join(name), content.as_bytes()).unwrap();
        }
        let command = legacy_commands(&legacy.join("codex.sh"))[0].clone();
        let original = json!({"hooks":{"Stop":[{"hooks":[{"type":"command","command":command},{"type":"command","command":"native-herdr"}]}]},"unrelated":true});
        common::atomic_owned_write(&config, &serde_json::to_vec(&original).unwrap()).unwrap();
        install(&root, &home, false).unwrap();
        install(&root, &home, false).unwrap();
        let updated: Value = serde_json::from_slice(&std::fs::read(&config).unwrap()).unwrap();
        assert_eq!(
            updated["hooks"]["Stop"][0]["hooks"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(updated["unrelated"], true);
        assert!(!legacy.join("codex_usage.py").exists());
        assert_eq!(
            std::fs::read(legacy.join("codex.sh")).unwrap(),
            RETIRED_CODEX_SHIM
        );
        assert_eq!(common::sha256(RETIRED_CODEX_SHIM), RETIRED_CODEX_SHA256);
        assert!(extension.exists());
        let mut changed = std::fs::read(&extension).unwrap();
        changed.extend_from_slice(b"// user change\n");
        common::atomic_owned_write(&extension, &changed).unwrap();
        assert!(uninstall(&root, &home).is_err());
        assert!(install(&root, &home, false).is_err());
        assert!(extension.exists());
        let payload = include_str!("../../../hooks/observatory.ts").replace(
            "const nativeRuntime = '';",
            &format!(
                "const nativeRuntime = {};",
                json!(root.join("anton-runtime").to_string_lossy())
            ),
        );
        common::atomic_owned_write(&extension, payload.as_bytes()).unwrap();
        uninstall(&root, &home).unwrap();
        uninstall(&root, &home).unwrap();
        assert!(!extension.exists());
        assert!(!legacy.join("codex.sh").exists());
        assert!(config.exists());
        std::fs::remove_dir_all(home).unwrap();
    }
    struct NativeFixture {
        home: PathBuf,
        root: PathBuf,
    }
    impl NativeFixture {
        fn new() -> Self {
            use std::os::unix::fs::PermissionsExt;
            static SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let home = std::env::temp_dir().join(format!(
                "anton-retired-hook-{}-{}-{}",
                std::process::id(),
                common::now().to_bits(),
                SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ));
            let root = home.join("plugin");
            common::ensure_private_directory(&root).unwrap();
            common::atomic_owned_write(
                &root.join(".herdr-observatory-install"),
                b"herdr.observatory\n",
            )
            .unwrap();
            common::atomic_owned_write(&root.join("anton-runtime"), b"fixture").unwrap();
            std::fs::set_permissions(
                root.join("anton-runtime"),
                std::fs::Permissions::from_mode(0o700),
            )
            .unwrap();
            common::ensure_private_directory(&home.join(".codex")).unwrap();
            common::atomic_owned_write(&paths(&home).2, b"{\"unrelated\":true}").unwrap();
            install(&root, &home, false).unwrap();
            Self { home, root }
        }
        fn shell(&self) -> PathBuf {
            paths(&self.home).0.join("codex.sh")
        }
        fn receipt(&self) -> Value {
            serde_json::from_slice(&std::fs::read(self.root.join(".hooks-receipt.json")).unwrap())
                .unwrap()
        }
        fn backup(&self) {
            let value = json!({"hooks":{"Stop":[{"hooks":[{"type":"command","command":legacy_commands(&self.shell())[0]}]}]}});
            common::atomic_owned_write(
                &self.root.join(".hooks-before-native.json"),
                &serde_json::to_vec(&value).unwrap(),
            )
            .unwrap();
        }
    }
    impl Drop for NativeFixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.home);
        }
    }
    #[test]
    fn proof_based_repair_is_quiet_private_and_idempotent_without_touching_integrations() {
        use std::os::unix::fs::MetadataExt;
        let f = NativeFixture::new();
        f.backup();
        let (_, extension, config) = paths(&f.home);
        let before: Vec<_> = [&extension, &config]
            .into_iter()
            .map(|p| {
                (
                    std::fs::read(p).unwrap(),
                    std::fs::metadata(p).unwrap().ino(),
                )
            })
            .collect();
        let old_receipt = f.receipt();
        repair_retired(&f.root, &f.home).unwrap();
        let receipt = f.receipt();
        for key in ["version", "runtime", "extension", "sha256"] {
            assert_eq!(receipt[key], old_receipt[key]);
        }
        assert_eq!(
            receipt["compatibility"],
            json!({"path":f.shell(),"sha256":RETIRED_CODEX_SHA256})
        );
        assert_eq!(std::fs::read(f.shell()).unwrap(), RETIRED_CODEX_SHIM);
        assert_eq!(std::fs::metadata(f.shell()).unwrap().mode() & 0o777, 0o600);
        assert_eq!(
            std::fs::metadata(f.shell().parent().unwrap())
                .unwrap()
                .mode()
                & 0o777,
            0o700
        );
        let inode = std::fs::metadata(f.root.join(".hooks-receipt.json"))
            .unwrap()
            .ino();
        repair_retired(&f.root, &f.home).unwrap();
        assert_eq!(
            std::fs::metadata(f.root.join(".hooks-receipt.json"))
                .unwrap()
                .ino(),
            inode
        );
        for (p, (bytes, ino)) in [&extension, &config].into_iter().zip(before) {
            assert_eq!(std::fs::read(p).unwrap(), bytes);
            assert_eq!(std::fs::metadata(p).unwrap().ino(), ino);
        }
        for size in [0, 65536, 1048576] {
            let output = common::run_bounded(
                &["/bin/sh".into(), f.shell().to_string_lossy().into_owned()],
                &vec![b'x'; size],
                Duration::from_secs(2),
                1,
                None,
            )
            .unwrap();
            assert!(output.is_empty());
        }
        install(&f.root, &f.home, false).unwrap();
        assert_eq!(f.receipt()["compatibility"], receipt["compatibility"]);
    }
    #[test]
    fn repair_requires_proof_and_refuses_unknown_files_symlinks_or_owner() {
        for scenario in 0..8 {
            let f = NativeFixture::new();
            if scenario != 0 {
                f.backup();
            }
            let (_, extension, config) = paths(&f.home);
            match scenario {
                1 | 2 => {
                    compatibility_directories(&f.home, true).unwrap();
                    common::atomic_owned_write(
                        &f.shell(),
                        if scenario == 1 {
                            b"unknown"
                        } else {
                            RETIRED_CODEX_SHIM
                        },
                    )
                    .unwrap();
                }
                3 => {
                    compatibility_directories(&f.home, true).unwrap();
                    std::os::unix::fs::symlink(&extension, f.shell()).unwrap();
                }
                4 => {
                    std::os::unix::fs::symlink(f.root.clone(), f.home.join(".local")).unwrap();
                }
                5 => {
                    let mut r = f.receipt();
                    r["runtime"] = json!("/unknown/runtime");
                    common::atomic_owned_write(
                        &f.root.join(".hooks-receipt.json"),
                        &serde_json::to_vec(&r).unwrap(),
                    )
                    .unwrap();
                }
                6 => {
                    common::atomic_owned_write(&extension, b"changed").unwrap();
                }
                7 => {
                    common::atomic_owned_write(
                        &f.root.join(".herdr-observatory-install"),
                        b"unknown",
                    )
                    .unwrap();
                }
                _ => {}
            }
            let before = [&extension, &config, &f.root.join(".hooks-receipt.json")]
                .map(|p| std::fs::read(p).unwrap());
            assert!(
                repair_retired(&f.root, &f.home).is_err(),
                "scenario {scenario}"
            );
            let after = [&extension, &config, &f.root.join(".hooks-receipt.json")]
                .map(|p| std::fs::read(p).unwrap());
            assert_eq!(after, before);
        }
    }
    #[test]
    fn failed_receipt_write_creates_nothing_and_recorded_absent_intent_retries() {
        let f = NativeFixture::new();
        f.backup();
        let path = f.root.join(".hooks-receipt.json");
        let before = std::fs::read(&path).unwrap();
        let receipt = compatibility_receipt(f.receipt(), &f.shell());
        assert!(
            record_and_create(&path, &receipt, &f.home, &f.shell(), |_, _| Err(
                "Injected receipt failure".into()
            ))
            .is_err()
        );
        assert_eq!(std::fs::read(&path).unwrap(), before);
        assert!(!f.home.join(".local").exists());
        // Simulate interruption after durable ownership intent and before mkdir.
        common::atomic_owned_write(&path, &serde_json::to_vec(&receipt).unwrap()).unwrap();
        repair_retired(&f.root, &f.home).unwrap();
        assert_eq!(std::fs::read(f.shell()).unwrap(), RETIRED_CODEX_SHIM);
    }
    #[test]
    fn recorded_migration_intent_accepts_only_proven_legacy_preimage() {
        let f = NativeFixture::new();
        f.backup();
        compatibility_directories(&f.home, true).unwrap();
        let legacy = paths(&f.home).0;
        let previous = format!("# {LEGACY}\nRUNTIME = {:?}\n", f.root.join("runtime.py"));
        for name in ["codex.sh", "codex_usage.py", "allowances_probe.py"] {
            common::atomic_owned_write(&legacy.join(name), previous.as_bytes()).unwrap();
        }
        let receipt = compatibility_receipt(f.receipt(), &f.shell());
        common::atomic_owned_write(
            &f.root.join(".hooks-receipt.json"),
            &serde_json::to_vec(&receipt).unwrap(),
        )
        .unwrap();
        install(&f.root, &f.home, false).unwrap();
        assert_eq!(std::fs::read(f.shell()).unwrap(), RETIRED_CODEX_SHIM);
        assert!(!legacy.join("codex_usage.py").exists());
    }
    #[test]
    fn uninstall_preflights_both_integrations_and_preserves_unrelated_legacy_content() {
        let f = NativeFixture::new();
        f.backup();
        repair_retired(&f.root, &f.home).unwrap();
        let extension = paths(&f.home).1;
        let original = std::fs::read(&extension).unwrap();
        let receipt = std::fs::read(f.root.join(".hooks-receipt.json")).unwrap();
        common::atomic_owned_write(&f.shell(), b"user changed helper").unwrap();
        assert!(uninstall(&f.root, &f.home).is_err());
        assert_eq!(std::fs::read(&extension).unwrap(), original);
        assert_eq!(
            std::fs::read(f.root.join(".hooks-receipt.json")).unwrap(),
            receipt
        );
        common::atomic_owned_write(&f.shell(), RETIRED_CODEX_SHIM).unwrap();
        common::atomic_owned_write(&extension, b"user changed Pi").unwrap();
        assert!(uninstall(&f.root, &f.home).is_err());
        assert!(f.shell().exists());
        common::atomic_owned_write(&extension, &original).unwrap();
        let unrelated = f.shell().with_file_name("keep.txt");
        common::atomic_owned_write(&unrelated, b"unrelated").unwrap();
        uninstall(&f.root, &f.home).unwrap();
        uninstall(&f.root, &f.home).unwrap();
        assert!(!f.shell().exists());
        assert!(!extension.exists());
        assert_eq!(std::fs::read(unrelated).unwrap(), b"unrelated");
    }
    #[test]
    fn repair_exclusive_creation_preserves_a_file_appearing_after_preflight() {
        let f = NativeFixture::new();
        f.backup();
        let receipt = compatibility_receipt(f.receipt(), &f.shell());
        let result = record_and_create(
            &f.root.join(".hooks-receipt.json"),
            &receipt,
            &f.home,
            &f.shell(),
            |path, bytes| {
                common::atomic_owned_write(path, bytes)?;
                compatibility_directories(&f.home, true)?;
                common::atomic_owned_write(&f.shell(), b"concurrent user file")
            },
        );
        assert!(result.is_err());
        assert_eq!(std::fs::read(f.shell()).unwrap(), b"concurrent user file");
        assert!(repair_retired(&f.root, &f.home).is_err());
        assert!(paths(&f.home).1.exists());
    }
}
