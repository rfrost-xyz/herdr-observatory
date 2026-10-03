//! Bounded operating-system primitives shared by the native plugin commands.
use crate::Result;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::ffi::CString;
use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::MetadataExt;
use std::os::unix::net::UnixStream;
use std::os::unix::process::CommandExt;
use std::path::{Component, Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub fn now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}
pub fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn expand_home(value: &str) -> PathBuf {
    expand_home_in(
        value,
        Path::new(&std::env::var_os("HOME").unwrap_or_default()),
    )
}
/// `expand_home` against an explicit `home`: `~` and a leading `~/`.
pub fn expand_home_in(value: &str, home: &Path) -> PathBuf {
    if value == "~" {
        return home.to_owned();
    }
    if let Some(relative) = value.strip_prefix("~/") {
        return home.join(relative);
    }
    PathBuf::from(value)
}
pub fn safe_id(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
}
pub fn hex_id(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
pub fn number(value: &Value) -> Option<u64> {
    value.as_u64().filter(|v| *v <= 9_007_199_254_740_991)
}

/// Walk every directory with openat/O_NOFOLLOW. Root-owned ancestors are normal;
/// callers requiring a private directory also verify the final owner.
pub fn open_directory(path: &Path) -> Result<File> {
    if !path.is_absolute() {
        return Err("Absolute owned path required".into());
    }
    let mut directory = File::open("/").map_err(|_| "Cannot open root")?;
    for component in path.components() {
        let Component::Normal(name) = component else {
            if component == Component::RootDir {
                continue;
            }
            return Err("Invalid owned path".into());
        };
        let name = CString::new(name.as_encoded_bytes()).map_err(|_| "Invalid path")?;
        let fd = unsafe {
            libc::openat(
                directory.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err("Cannot open owned directory".into());
        }
        directory = unsafe { File::from_raw_fd(fd) };
    }
    Ok(directory)
}
pub fn open_owned(path: &Path, write: bool, create: bool) -> Result<File> {
    let parent = open_directory(path.parent().ok_or("Missing parent")?)?;
    let name = CString::new(
        path.file_name()
            .ok_or("Missing filename")?
            .as_encoded_bytes(),
    )
    .map_err(|_| "Invalid path")?;
    let flags = if write { libc::O_RDWR } else { libc::O_RDONLY };
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            flags
                | libc::O_NOFOLLOW
                | libc::O_NONBLOCK
                | libc::O_CLOEXEC
                | if create { libc::O_CREAT } else { 0 },
            0o600,
        )
    };
    if fd < 0 {
        return Err("Cannot open owned file".into());
    }
    let file = unsafe { File::from_raw_fd(fd) };
    let info = file.metadata().map_err(|_| "Cannot inspect owned file")?;
    if !info.is_file() || info.uid() != unsafe { libc::getuid() } {
        return Err("Unsafe file owner or type".into());
    }
    Ok(file)
}
pub fn read_owned(path: &Path, limit: usize, private: bool) -> Result<Vec<u8>> {
    let file = open_owned(path, false, false)?;
    let info = file.metadata().map_err(|_| "Cannot inspect file")?;
    if info.len() > limit as u64 || (private && info.mode() & 0o077 != 0) {
        return Err("Invalid private file size or permissions".into());
    }
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Cannot read file")?;
    if bytes.len() > limit {
        return Err("File exceeds limit".into());
    }
    Ok(bytes)
}
pub fn atomic_owned_write(path: &Path, bytes: &[u8]) -> Result<()> {
    atomic_write(path, bytes, false)
}
pub fn atomic_checkpoint_write(path: &Path, bytes: &[u8]) -> Result<()> {
    atomic_write(path, bytes, true)
}
fn atomic_write(path: &Path, bytes: &[u8], checkpoint: bool) -> Result<()> {
    let parent = open_directory(path.parent().ok_or("Missing parent")?)?;
    if parent
        .metadata()
        .map_err(|_| "Cannot inspect directory")?
        .uid()
        != unsafe { libc::getuid() }
    {
        return Err("Invalid directory owner".into());
    }
    let name = CString::new(
        path.file_name()
            .ok_or("Missing filename")?
            .as_encoded_bytes(),
    )
    .map_err(|_| "Invalid path")?;
    let mut info = std::mem::MaybeUninit::<libc::stat>::uninit();
    let exists = unsafe {
        libc::fstatat(
            parent.as_raw_fd(),
            name.as_ptr(),
            info.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if exists == 0 {
        let info = unsafe { info.assume_init() };
        if info.st_mode & libc::S_IFMT != libc::S_IFREG || info.st_uid != unsafe { libc::getuid() }
        {
            return Err("Unsafe destination".into());
        }
    } else if std::io::Error::last_os_error().raw_os_error() != Some(libc::ENOENT) {
        return Err("Cannot inspect destination".into());
    }
    let temporary = CString::new(if checkpoint {
        format!(
            ".replay-checkpoints-{:016x}",
            now().to_bits() ^ std::process::id() as u64
        )
    } else {
        format!(".anton-write-{}-{}", std::process::id(), now().to_bits())
    })
    .unwrap();
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            temporary.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            0o600,
        )
    };
    if fd < 0 {
        return Err("Cannot create private temporary file".into());
    }
    let mut file = unsafe { File::from_raw_fd(fd) };
    let result = (|| {
        file.write_all(bytes)
            .map_err(|_| "Cannot write private state")?;
        file.sync_all().map_err(|_| "Cannot sync private state")?;
        if unsafe {
            libc::renameat(
                parent.as_raw_fd(),
                temporary.as_ptr(),
                parent.as_raw_fd(),
                name.as_ptr(),
            )
        } < 0
        {
            return Err("Cannot replace private state".into());
        }
        Ok(())
    })();
    unsafe {
        libc::unlinkat(parent.as_raw_fd(), temporary.as_ptr(), 0);
    }
    result
}
pub struct OwnerGuard {
    _file: File,
}
/// Releases a `flock` taken on the file when dropped. A process spawned by
/// another thread holds the lock's open file description until it calls
/// exec, so closing the descriptor alone can leave the lock held. Declare
/// the guard after the file so it drops first.
pub struct Unlock<'a>(pub &'a File);
impl Drop for Unlock<'_> {
    fn drop(&mut self) {
        unsafe {
            libc::flock(self.0.as_raw_fd(), libc::LOCK_UN);
        }
    }
}
pub fn owner_guard(path: &Path) -> Result<OwnerGuard> {
    let mut file = open_owned(path, false, false)?;
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_SH | libc::LOCK_NB) } < 0 {
        return Err("Plugin ownership busy".into());
    }
    let mut bytes = Vec::new();
    (&mut file)
        .take(81)
        .read_to_end(&mut bytes)
        .map_err(|_| "Cannot read owner marker")?;
    if bytes != b"herdr.observatory\n" && bytes != b"herdr.observatory" {
        return Err("Plugin ownership unavailable".into());
    }
    let current = std::fs::symlink_metadata(path).map_err(|_| "Owner marker removed")?;
    let opened = file.metadata().map_err(|_| "Owner marker invalid")?;
    if current.dev() != opened.dev() || current.ino() != opened.ino() {
        return Err("Owner marker replaced".into());
    }
    Ok(OwnerGuard { _file: file })
}
pub fn spawn_group(argv: &[String]) -> Result<Child> {
    spawn_group_in(argv, Stdio::piped(), None)
}
fn spawn_group_in(argv: &[String], stdin: Stdio, cwd: Option<&Path>) -> Result<Child> {
    let (program, args) = argv.split_first().ok_or("Missing executable")?;
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(stdin)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .process_group(0);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    command.spawn().map_err(|_| "Command unavailable".into())
}
pub fn terminate_group(child: &mut Child) {
    unsafe {
        libc::kill(-(child.id() as i32), libc::SIGKILL);
    }
    let _ = child.kill();
    let _ = child.wait();
}
pub struct ProcessOutput {
    pub status: i32,
    pub stdout: Vec<u8>,
}
pub fn run_process(
    argv: &[String],
    input: &[u8],
    timeout: Duration,
    limit: usize,
    cancel: Option<&AtomicBool>,
) -> Result<ProcessOutput> {
    run_process_in(argv, Some(input), None, timeout, limit, cancel)
}
/// `run_process` with an optional working directory. `None` input gives the
/// child a null stdin instead of a pipe.
pub fn run_process_in(
    argv: &[String],
    input: Option<&[u8]>,
    cwd: Option<&Path>,
    timeout: Duration,
    limit: usize,
    cancel: Option<&AtomicBool>,
) -> Result<ProcessOutput> {
    let piped = input.is_some();
    let input = input.unwrap_or_default();
    let mut child = spawn_group_in(
        argv,
        if piped { Stdio::piped() } else { Stdio::null() },
        cwd,
    )?;
    let stdin = if piped {
        Some(child.stdin.take().ok_or("Missing stdin")?)
    } else {
        None
    };
    let mut stdout = child.stdout.take().ok_or("Missing stdout")?;
    for fd in stdin
        .iter()
        .map(AsRawFd::as_raw_fd)
        .chain([stdout.as_raw_fd()])
    {
        unsafe {
            let flags = libc::fcntl(fd, libc::F_GETFL);
            libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
        }
    }
    let deadline = Instant::now() + timeout;
    let mut offset = 0;
    let mut output = Vec::new();
    let mut closed = false;
    let mut writer = stdin;
    let mut chunk = [0u8; 16384];
    let result = (|| {
        loop {
            if Instant::now() >= deadline || cancel.is_some_and(|v| v.load(Ordering::Relaxed)) {
                return Err("Command deadline or cancellation".into());
            }
            if let Some(stream) = &mut writer {
                if offset < input.len() {
                    match stream.write(&input[offset..]) {
                        Ok(0) => return Err("Command input closed".into()),
                        Ok(count) => offset += count,
                        Err(error)
                            if matches!(
                                error.kind(),
                                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
                            ) => {}
                        Err(_) => return Err("Command input failed".into()),
                    }
                }
                if offset == input.len() {
                    writer = None;
                }
            }
            loop {
                match stdout.read(&mut chunk) {
                    Ok(0) => {
                        closed = true;
                        break;
                    }
                    Ok(count) => {
                        output.extend_from_slice(&chunk[..count]);
                        if output.len() > limit {
                            return Err("Command output exceeds limit".into());
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
                    Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(_) => return Err("Cannot read command".into()),
                }
            }
            if let Some(status) = child.try_wait().map_err(|_| "Cannot inspect command")? {
                if closed {
                    return Ok(ProcessOutput {
                        status: status.code().unwrap_or(-1),
                        stdout: output,
                    });
                }
            }
            thread::sleep(Duration::from_millis(10));
        }
    })();
    terminate_group(&mut child);
    result
}
pub fn run_bounded(
    argv: &[String],
    input: &[u8],
    timeout: Duration,
    limit: usize,
    cancel: Option<&AtomicBool>,
) -> Result<Vec<u8>> {
    let result = run_process(argv, input, timeout, limit, cancel)?;
    if result.status != 0 {
        return Err("Command failed".into());
    }
    Ok(result.stdout)
}

pub fn rpc(
    path: &Path,
    method: &str,
    params: Value,
    timeout: Duration,
    limit: usize,
) -> Result<Value> {
    rpc_with_cancel(path, method, params, timeout, limit, None)
}
pub fn rpc_with_cancel(
    path: &Path,
    method: &str,
    params: Value,
    timeout: Duration,
    limit: usize,
    cancel: Option<&AtomicBool>,
) -> Result<Value> {
    let bytes = path.as_os_str().as_encoded_bytes();
    let mut address: libc::sockaddr_un = unsafe { std::mem::zeroed() };
    if bytes.len() >= address.sun_path.len() {
        return Err("Socket path too long".into());
    }
    address.sun_family = libc::AF_UNIX as _;
    for (destination, source) in address.sun_path.iter_mut().zip(bytes) {
        *destination = *source as _;
    }
    let fd = unsafe {
        libc::socket(
            libc::AF_UNIX,
            libc::SOCK_STREAM | libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK,
            0,
        )
    };
    if fd < 0 {
        return Err("Socket unavailable".into());
    }
    let mut stream = unsafe { UnixStream::from_raw_fd(fd) };
    let deadline = Instant::now() + timeout;
    if unsafe {
        libc::connect(
            fd,
            (&address as *const libc::sockaddr_un).cast(),
            std::mem::size_of_val(&address) as _,
        )
    } < 0
    {
        let errno = std::io::Error::last_os_error().raw_os_error();
        if errno != Some(libc::EINPROGRESS) && errno != Some(libc::EAGAIN) {
            return Err("Herdr socket unavailable".into());
        }
    }
    let identifier = "anton-readonly";
    let mut request = serde_json::to_vec(&json!({"id":identifier,"method":method,"params":params}))
        .map_err(|_| "Invalid request")?;
    request.push(b'\n');
    let mut offset = 0;
    let mut buffer = Vec::new();
    let mut chunk = [0; 16384];
    while Instant::now() < deadline {
        if cancel.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
            return Err("Herdr RPC cancelled".into());
        }
        if offset < request.len() {
            match stream.write(&request[offset..]) {
                Ok(0) => return Err("Herdr socket closed".into()),
                Ok(count) => offset += count,
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
                    ) => {}
                Err(_) => return Err("Herdr socket write failed".into()),
            }
        }
        match stream.read(&mut chunk) {
            Ok(0) => return Err("Incomplete Herdr response".into()),
            Ok(count) => {
                buffer.extend_from_slice(&chunk[..count]);
                if buffer.len() > limit {
                    return Err("Herdr response exceeds limit".into());
                }
                if let Some(end) = buffer.iter().position(|v| *v == b'\n') {
                    let result: Value =
                        serde_json::from_slice(&buffer[..end]).map_err(|_| "Invalid Herdr JSON")?;
                    if result["id"] != identifier {
                        return Err("Herdr response mismatch".into());
                    }
                    if result.get("error").is_some() {
                        return Err("Herdr RPC rejected".into());
                    }
                    return result
                        .get("result")
                        .cloned()
                        .ok_or("Missing Herdr result".into());
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
                ) => {}
            Err(_) => return Err("Herdr socket read failed".into()),
        }
        let mut poll = libc::pollfd {
            fd,
            events: libc::POLLIN
                | if offset < request.len() {
                    libc::POLLOUT
                } else {
                    0
                },
            revents: 0,
        };
        unsafe {
            libc::poll(&mut poll, 1, 20);
        }
    }
    Err("Herdr RPC deadline".into())
}

pub fn ensure_private_directory(path: &Path) -> Result<()> {
    if !path.is_absolute() {
        return Err("Absolute state path required".into());
    }
    let mut directory = File::open("/").map_err(|_| "Cannot open root")?;
    for component in path.components() {
        let Component::Normal(name) = component else {
            if component == Component::RootDir {
                continue;
            }
            return Err("Invalid state path".into());
        };
        let name = CString::new(name.as_encoded_bytes()).map_err(|_| "Invalid state path")?;
        let mut fd = unsafe {
            libc::openat(
                directory.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if fd < 0 && std::io::Error::last_os_error().raw_os_error() == Some(libc::ENOENT) {
            if unsafe { libc::mkdirat(directory.as_raw_fd(), name.as_ptr(), 0o700) } < 0
                && std::io::Error::last_os_error().raw_os_error() != Some(libc::EEXIST)
            {
                return Err("Cannot create private state directory".into());
            }
            fd = unsafe {
                libc::openat(
                    directory.as_raw_fd(),
                    name.as_ptr(),
                    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                )
            };
        }
        if fd < 0 {
            return Err("Unsafe state directory".into());
        }
        directory = unsafe { File::from_raw_fd(fd) };
    }
    if directory
        .metadata()
        .map_err(|_| "Cannot inspect state directory")?
        .uid()
        != unsafe { libc::getuid() }
    {
        return Err("State directory ownership mismatch".into());
    }
    Ok(())
}
/// A unique fixture directory name under the temporary directory, after
/// removing stale siblings with `prefix`. Fixtures clean up on drop, which a
/// killed test process never reaches. A sibling is stale only when the
/// current user owns it, its embedded process id is dead here and it has not
/// been modified for an hour: a run in another pid namespace sharing the
/// temporary directory has ids that look dead here, so the age is what keeps
/// its live fixtures. A sweep by hand should match the exact prefix and age.
#[cfg(test)]
pub(crate) fn fixture_dir(prefix: &str, sequence: u64) -> PathBuf {
    let temp = std::env::temp_dir();
    if let Ok(entries) = std::fs::read_dir(&temp) {
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
            // A modification time in the future or unreadable is recent.
            let owned = std::fs::symlink_metadata(entry.path()).is_ok_and(|info| {
                info.is_dir()
                    && info.uid() == unsafe { libc::geteuid() }
                    && info
                        .modified()
                        .ok()
                        .and_then(|at| at.elapsed().ok())
                        .is_some_and(|age| age.as_secs() >= 3600)
            });
            // SAFETY: signal 0 only checks whether the process exists.
            let dead = unsafe { libc::kill(pid, 0) } == -1
                && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH);
            if owned && dead {
                let _ = std::fs::remove_dir_all(entry.path());
            }
        }
    }
    temp.join(format!(
        "{prefix}{}-{}-{sequence}",
        std::process::id(),
        now().to_bits()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A sibling whose process id is dead here may belong to a live run in
    /// another pid namespace, so only one also untouched for an hour is
    /// swept (review round 12).
    #[test]
    fn fixture_sweep_removes_only_dead_pid_dirs_untouched_for_an_hour() {
        let prefix = format!("anton-unit-sweep-{}-", std::process::id());
        let max: i64 = std::fs::read_to_string("/proc/sys/kernel/pid_max")
            .ok()
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(4_194_304);
        let decoy = std::env::temp_dir().join(format!("{prefix}{}-0-0", max + 1));
        std::fs::create_dir(&decoy).unwrap();
        fixture_dir(&prefix, 0);
        let kept = decoy.is_dir();
        File::open(&decoy)
            .unwrap()
            .set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(3601))
            .unwrap();
        fixture_dir(&prefix, 1);
        let removed = !decoy.exists();
        let _ = std::fs::remove_dir_all(&decoy);
        assert!(kept, "a recent dead-pid sibling is kept");
        assert!(removed, "an hour-old dead-pid sibling is removed");
    }
}
