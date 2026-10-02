//! Fixture helpers shared by the process acceptance tests.
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
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
