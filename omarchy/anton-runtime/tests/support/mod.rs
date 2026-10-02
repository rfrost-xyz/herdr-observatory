//! Fixture helpers shared by the process acceptance tests.
use std::fs;
use std::io::Write;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Writes an executable fixture from a child process. A process spawned by a
/// sibling test thread holds a copy of every descriptor open here until it
/// calls exec, so a script written in this process could still be open for
/// writing when it runs, and exec fails with ETXTBSY. Writing to a temporary
/// name and renaming does not help: the inode stays open.
pub fn write_executable(path: &Path, bytes: &[u8], mode: u32) {
    let mut writer = Command::new("/bin/sh")
        .args(["-c", "cat > \"$1\"", "sh"])
        .arg(path)
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = writer.stdin.take().unwrap();
    input.write_all(bytes).unwrap();
    drop(input);
    assert!(writer.wait().unwrap().success());
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}

/// Removes fixture directories named `prefix` + process id + `-...` under
/// the temporary directory whose process no longer exists. Fixtures clean up
/// on drop, which a killed test process never reaches; only directories the
/// current user owns, left by a dead process, are removed, so a concurrent
/// run is never touched and nobody needs to sweep the prefix by hand.
#[allow(dead_code)] // native_navigation has no such fixture.
pub fn remove_stale_fixtures(prefix: &str) {
    let Ok(entries) = fs::read_dir(std::env::temp_dir()) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(pid) = name
            .to_str()
            .and_then(|name| name.strip_prefix(prefix))
            .and_then(|rest| rest.split('-').next())
            .and_then(|pid| pid.parse::<libc::pid_t>().ok())
            .filter(|pid| *pid > 0 && *pid as u32 != std::process::id())
        else {
            continue;
        };
        let owned = fs::symlink_metadata(entry.path())
            .is_ok_and(|info| info.is_dir() && info.uid() == unsafe { libc::geteuid() });
        // SAFETY: signal 0 only checks whether the process exists.
        let dead = unsafe { libc::kill(pid, 0) } == -1
            && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH);
        if owned && dead {
            let _ = fs::remove_dir_all(entry.path());
        }
    }
}
/// A unique fixture directory name for `prefix`, after removing stale ones.
#[allow(dead_code)] // native_navigation has no such fixture.
pub fn fixture_dir(prefix: &str, sequence: u64) -> PathBuf {
    remove_stale_fixtures(prefix);
    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}{}-{time}-{sequence}", std::process::id()))
}
