//! Exact-session Claude Code transcript binding. A Herdr session id names one
//! `<root>/<entry>/<id>.jsonl` file, verified by its header and record identity.
//! Native paths and ids stay in process memory.
use crate::{
    Result,
    common::{self, safe_id},
    native::line,
};
use serde_json::Value;
use std::fs::File;
use std::io::BufReader;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::Instant;

const LINE: usize = 65536;
const ENTRIES: usize = 8192;
const SUCCESSOR_BYTES: usize = 262144;
const SUCCESSOR_RECORDS: usize = 512;

pub fn projects_root() -> PathBuf {
    std::env::var_os("CLAUDE_CONFIG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| common::expand_home("~/.claude"))
        .join("projects")
}

/// Shared entry and time budget for one discovery, including the predecessor scan.
pub struct Budget {
    entries: usize,
    deadline: Instant,
}
impl Budget {
    pub fn new(deadline: Instant) -> Self {
        Self::with_entries(ENTRIES, deadline)
    }
    pub fn with_entries(entries: usize, deadline: Instant) -> Self {
        Self { entries, deadline }
    }
    fn take(&mut self) -> bool {
        if self.entries == 0 || Instant::now() >= self.deadline {
            return false;
        }
        self.entries -= 1;
        true
    }
    fn expired(&self) -> bool {
        Instant::now() >= self.deadline
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Discovery {
    Found(PathBuf),
    None,
    Ambiguous,
    Truncated,
}

/// D1: look for `<root>/<entry>/<id>.jsonl` at depth exactly two. Any filesystem
/// object with that name counts as a match, so a symlinked candidate still makes
/// the binding ambiguous rather than letting another file stand in for it.
pub fn discover(root: &Path, id: &str, budget: &mut Budget) -> Discovery {
    if !safe_id(id, 128) || common::open_directory(root).is_err() {
        return Discovery::None;
    }
    let Ok(entries) = std::fs::read_dir(root) else {
        return Discovery::None;
    };
    let name = format!("{id}.jsonl");
    let mut found = None;
    for entry in entries {
        if !budget.take() {
            return Discovery::Truncated;
        }
        let Ok(entry) = entry else {
            return Discovery::Truncated;
        };
        let candidate = entry.path().join(&name);
        match std::fs::symlink_metadata(&candidate) {
            Ok(_) if found.is_some() => return Discovery::Ambiguous,
            Ok(_) => found = Some(candidate),
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
                ) => {}
            Err(_) => return Discovery::Truncated,
        }
    }
    found.map_or(Discovery::None, Discovery::Found)
}

#[derive(Debug, PartialEq, Eq)]
pub enum Predecessor {
    /// No successor names the bound id. `growing` is set when a candidate ended
    /// within the bound with no `session_id`; that result must not be cached.
    Clear { growing: bool },
    /// A successor names the bound id, or the scan could not decide.
    Unknown,
}

/// D1 predecessor check after `/clear`: every `.jsonl` file in the bound file's
/// directory that is at least as new as it is scanned to its first record that
/// carries `session_id`, within 256 KiB and 512 records each.
pub fn predecessor(path: &Path, id: &str, budget: &mut Budget) -> Predecessor {
    let (Some(directory), Some(own)) = (path.parent(), path.file_name()) else {
        return Predecessor::Unknown;
    };
    let Ok(bound) = common::open_owned(path, false, false).and_then(|file| {
        file.metadata()
            .map_err(|_| "Cannot inspect Claude session".into())
    }) else {
        return Predecessor::Unknown;
    };
    let since = (bound.mtime(), bound.mtime_nsec());
    if common::open_directory(directory).is_err() {
        return Predecessor::Unknown;
    }
    let Ok(entries) = std::fs::read_dir(directory) else {
        return Predecessor::Unknown;
    };
    let mut growing = false;
    for entry in entries {
        if !budget.take() {
            return Predecessor::Unknown;
        }
        let Ok(entry) = entry else {
            return Predecessor::Unknown;
        };
        let name = entry.file_name();
        if name == own || !name.to_string_lossy().ends_with(".jsonl") {
            continue;
        }
        let Ok(info) = entry.metadata() else {
            return Predecessor::Unknown;
        };
        if info.is_dir() {
            continue;
        }
        if !info.is_file() {
            return Predecessor::Unknown;
        }
        if (info.mtime(), info.mtime_nsec()) < since {
            continue;
        }
        match first_session_id(&entry.path(), id, budget) {
            Some(Successor::Ended) => growing = true,
            Some(Successor::Other) => {}
            None => return Predecessor::Unknown,
        }
    }
    Predecessor::Clear { growing }
}

enum Successor {
    /// Case 1: end of file within the bound with no `session_id`.
    Ended,
    /// Case 4: the first `session_id` names another session.
    Other,
}

/// Cases 2 (bound exhausted) and 3 (first `session_id` is the bound id) and any
/// unreadable or unparseable record return `None`.
fn first_session_id(path: &Path, id: &str, budget: &Budget) -> Option<Successor> {
    let mut stream = BufReader::new(common::open_owned(path, false, false).ok()?);
    let mut consumed = 0;
    for _ in 0..SUCCESSOR_RECORDS {
        let remaining = SUCCESSOR_BYTES - consumed;
        if remaining == 0 || budget.expired() {
            return None;
        }
        let bytes = line(&mut stream, remaining).ok()?;
        consumed += bytes.len();
        if bytes.last() != Some(&b'\n') {
            return (bytes.len() < remaining).then_some(Successor::Ended);
        }
        let record: Value = serde_json::from_slice(&bytes).ok()?;
        match record.get("session_id") {
            None => {}
            Some(Value::String(value)) if value != id => return Some(Successor::Other),
            Some(_) => return None,
        }
    }
    None
}

/// D2: the path must be exactly `<root>/<entry>/<id>.jsonl`, opened without
/// following any symlink, owned by this user, with a verified header line.
pub fn open_session(root: &Path, path: &Path, id: &str) -> Result<(BufReader<File>, Vec<u8>)> {
    let confined = safe_id(id, 128)
        && path.file_name() == Some(format!("{id}.jsonl").as_ref())
        && path
            .parent()
            .and_then(Path::parent)
            .is_some_and(|parent| parent == root)
        && path
            .strip_prefix(root)
            .is_ok_and(|rest| rest.components().count() == 2)
        && path.components().all(|part| {
            matches!(
                part,
                std::path::Component::RootDir | std::path::Component::Normal(_)
            )
        });
    if !confined {
        return Err("Invalid Claude session path".into());
    }
    let mut stream = BufReader::new(common::open_owned(path, false, false)?);
    let bytes = header(&mut stream, id)?;
    Ok((stream, bytes))
}

/// The first line: at most 64 KiB, newline-terminated, `sessionId == id`, any type.
fn header(stream: &mut BufReader<File>, id: &str) -> Result<Vec<u8>> {
    let bytes = line(stream, LINE + 1).map_err(|_| "Cannot read Claude session header")?;
    if bytes.len() > LINE || bytes.last() != Some(&b'\n') {
        return Err("Invalid Claude session header".into());
    }
    let value: Value =
        serde_json::from_slice(&bytes).map_err(|_| "Invalid Claude session header")?;
    if session_identity(&value, id) != Identity::Match {
        return Err("Claude session identity mismatch".into());
    }
    Ok(bytes)
}

#[derive(Debug, PartialEq, Eq)]
pub enum Identity {
    Match,
    /// No `sessionId` (for example `file-history-snapshot`): ignored for identity.
    Absent,
    /// A different or non-string `sessionId`: the session's telemetry is unknown.
    Mismatch,
}

pub fn session_identity(record: &Value, id: &str) -> Identity {
    match record.get("sessionId") {
        None => Identity::Absent,
        Some(Value::String(value)) if value == id => Identity::Match,
        Some(_) => Identity::Mismatch,
    }
}

/// Fork and branch copies carry `forkedFrom`: inherited history that feeds no
/// metric. Only its presence is read, never the nested id.
pub fn forked(record: &Value) -> bool {
    record
        .get("forkedFrom")
        .is_some_and(|value| !value.is_null())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::now;
    use std::time::Duration;
    struct Fixture {
        root: PathBuf,
        projects: PathBuf,
    }
    impl Fixture {
        fn new() -> Self {
            static SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let root = std::env::temp_dir().join(format!(
                "anton-claude-{}-{}-{}",
                std::process::id(),
                now().to_bits(),
                SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            ));
            let projects = root.join("projects");
            std::fs::create_dir_all(&projects).unwrap();
            Self { root, projects }
        }
        fn file(&self, entry: &str, name: &str, text: &str) -> PathBuf {
            let directory = self.projects.join(entry);
            std::fs::create_dir_all(&directory).unwrap();
            let path = directory.join(name);
            std::fs::write(&path, text).unwrap();
            path
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }
    fn budget() -> Budget {
        Budget::new(Instant::now() + Duration::from_secs(5))
    }
    const ID: &str = "fixture-session-a";

    #[test]
    fn discovery_requires_exactly_one_depth_two_match() {
        let fixture = Fixture::new();
        let root = &fixture.projects;
        assert_eq!(discover(root, ID, &mut budget()), Discovery::None);
        fixture.file("slug-deep/nested", &format!("{ID}.jsonl"), "{}\n");
        fixture.file(&format!("slug-x/{ID}/subagents"), "agent-x.jsonl", "{}\n");
        fixture.file("slug-x", "other-session.jsonl", "{}\n");
        std::fs::write(root.join(format!("{ID}.jsonl")), "{}\n").unwrap();
        assert_eq!(discover(root, ID, &mut budget()), Discovery::None);
        let path = fixture.file("slug-a", &format!("{ID}.jsonl"), "{}\n");
        assert_eq!(discover(root, ID, &mut budget()), Discovery::Found(path));
        fixture.file("slug-b", &format!("{ID}.jsonl"), "{}\n");
        assert_eq!(discover(root, ID, &mut budget()), Discovery::Ambiguous);
        assert_eq!(discover(root, "../slug-a", &mut budget()), Discovery::None);
        assert_eq!(discover(root, "", &mut budget()), Discovery::None);
    }
    #[test]
    fn discovery_truncated_by_entry_or_time_budget_is_unknown() {
        let fixture = Fixture::new();
        let root = &fixture.projects;
        for entry in ["slug-a", "slug-b", "slug-c"] {
            std::fs::create_dir(root.join(entry)).unwrap();
        }
        fixture.file("slug-b", &format!("{ID}.jsonl"), "{}\n");
        let far = Instant::now() + Duration::from_secs(5);
        assert_eq!(
            discover(root, ID, &mut Budget::with_entries(2, far)),
            Discovery::Truncated
        );
        assert!(matches!(
            discover(root, ID, &mut Budget::with_entries(3, far)),
            Discovery::Found(_)
        ));
        assert_eq!(
            discover(root, ID, &mut Budget::new(Instant::now())),
            Discovery::Truncated
        );
    }
    #[test]
    fn discovery_unsearchable_entry_is_never_a_unique_match() {
        use std::os::unix::fs::PermissionsExt;
        let fixture = Fixture::new();
        fixture.file("slug-a", &format!("{ID}.jsonl"), "{}\n");
        fixture.file("slug-b", &format!("{ID}.jsonl"), "{}\n");
        let hidden = fixture.projects.join("slug-b");
        std::fs::set_permissions(&hidden, std::fs::Permissions::from_mode(0o000)).unwrap();
        let result = discover(&fixture.projects, ID, &mut budget());
        std::fs::set_permissions(&hidden, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(!matches!(result, Discovery::Found(_)), "{result:?}");
    }
    #[test]
    fn discovery_rejects_symlinked_root_and_counts_symlinked_candidates() {
        let fixture = Fixture::new();
        let path = fixture.file("slug-a", &format!("{ID}.jsonl"), "{}\n");
        let alias = fixture.root.join("alias");
        std::os::unix::fs::symlink(&fixture.projects, &alias).unwrap();
        assert_eq!(discover(&alias, ID, &mut budget()), Discovery::None);
        std::fs::create_dir(fixture.projects.join("slug-b")).unwrap();
        std::os::unix::fs::symlink(&path, fixture.projects.join(format!("slug-b/{ID}.jsonl")))
            .unwrap();
        assert_eq!(
            discover(&fixture.projects, ID, &mut budget()),
            Discovery::Ambiguous
        );
        let mut alias_path = alias.join("slug-a");
        alias_path.push(format!("{ID}.jsonl"));
        std::fs::remove_file(fixture.projects.join(format!("slug-b/{ID}.jsonl"))).unwrap();
        std::fs::write(
            &path,
            format!("{{\"type\":\"mode\",\"sessionId\":\"{ID}\"}}\n"),
        )
        .unwrap();
        assert!(open_session(&fixture.projects, &path, ID).is_ok());
        assert!(open_session(&alias, &alias_path, ID).is_err());
    }
    fn header_line(id: &str) -> String {
        format!("{{\"type\":\"mode\",\"mode\":\"normal\",\"sessionId\":\"{id}\"}}\n")
    }
    #[test]
    fn identity_header_accepts_any_first_record_type_and_rejects_mismatch() {
        let fixture = Fixture::new();
        let root = &fixture.projects;
        let name = format!("{ID}.jsonl");
        let path = fixture.file("slug-a", &name, &header_line(ID));
        let (_, header) = open_session(root, &path, ID).unwrap();
        assert_eq!(header, header_line(ID).into_bytes());
        let other = format!("{{\"type\":\"user\",\"sessionId\":\"{ID}\"}}\n{{}}\n");
        std::fs::write(&path, &other).unwrap();
        assert!(open_session(root, &path, ID).is_ok());
        for text in [
            header_line("fixture-session-b"),
            header_line(ID).trim_end().to_owned(),
            "{\"type\":\"mode\"}\n".to_owned(),
            "{\"type\":\"mode\",\"sessionId\":7}\n".to_owned(),
            "not json\n".to_owned(),
            String::new(),
            format!(
                "{{\"type\":\"mode\",\"sessionId\":\"{ID}\",\"pad\":\"{}\"}}\n",
                "x".repeat(LINE)
            ),
        ] {
            std::fs::write(&path, &text).unwrap();
            assert!(open_session(root, &path, ID).is_err(), "{text:.40}");
        }
        std::fs::write(&path, header_line(ID)).unwrap();
        for bad in [
            root.join(&name),
            root.join("slug-a/nested").join(&name),
            root.join("slug-a/../slug-a").join(&name),
            root.join("slug-a/other.jsonl"),
        ] {
            assert!(open_session(root, &bad, ID).is_err(), "{}", bad.display());
        }
        assert!(open_session(root, &path, "fixture-session-b").is_err());
    }
    /// A synthetic successor: `records` filler records without `session_id`, one
    /// oversized attachment, then records carrying the given `session_id`.
    fn successor(records: usize, oversized: usize, first: &str, then: &str) -> String {
        let mut text = String::new();
        for index in 0..records {
            text.push_str(&format!(
                "{{\"type\":\"user\",\"sessionId\":\"fixture-session-c\",\"uuid\":\"u{index}\"}}\n"
            ));
        }
        text.push_str(&format!(
            "{{\"type\":\"attachment\",\"sessionId\":\"fixture-session-c\",\"pad\":\"{}\"}}\n",
            "x".repeat(oversized)
        ));
        for value in [first, then] {
            text.push_str(&format!(
                "{{\"type\":\"attachment\",\"sessionId\":\"fixture-session-c\",\"session_id\":\"{value}\"}}\n"
            ));
        }
        text
    }
    fn aged(path: &Path, seconds: u64) {
        let file = std::fs::OpenOptions::new().write(true).open(path).unwrap();
        file.set_modified(std::time::SystemTime::now() - Duration::from_secs(seconds))
            .unwrap();
    }
    #[test]
    fn predecessor_scan_covers_four_cases_beyond_sixteen_records_and_64_kib() {
        let fixture = Fixture::new();
        let bound = fixture.file("slug-a", &format!("{ID}.jsonl"), &header_line(ID));
        aged(&bound, 60);
        assert_eq!(
            predecessor(&bound, ID, &mut budget()),
            Predecessor::Clear { growing: false }
        );
        let next = fixture.file("slug-a", "fixture-session-c.jsonl", "");
        let cases = [
            (
                successor(17, 70000, ID, "fixture-session-d"),
                Predecessor::Unknown,
            ),
            (
                successor(17, 70000, "fixture-session-d", ID),
                Predecessor::Clear { growing: false },
            ),
            (successor(17, 300000, ID, ID), Predecessor::Unknown),
            (
                "{\"type\":\"user\"}\n".repeat(SUCCESSOR_RECORDS + 1),
                Predecessor::Unknown,
            ),
            (
                "{\"type\":\"user\"}\n".repeat(20),
                Predecessor::Clear { growing: true },
            ),
            (
                format!(
                    "{}{{\"type\":\"user\",\"session_id\":\"{ID}",
                    "{}\n".repeat(20)
                ),
                Predecessor::Clear { growing: true },
            ),
            ("{}\nnot json\n".to_owned(), Predecessor::Unknown),
            ("{\"session_id\":7}\n".to_owned(), Predecessor::Unknown),
            (String::new(), Predecessor::Clear { growing: true }),
        ];
        for (text, expected) in cases {
            std::fs::write(&next, &text).unwrap();
            assert_eq!(
                predecessor(&bound, ID, &mut budget()),
                expected,
                "{text:.60}"
            );
        }
        let text = successor(17, 70000, ID, ID);
        let first = text.find("\"session_id\"").unwrap();
        assert!(first > LINE && text[..first].matches('\n').count() > 16);
        std::fs::write(&next, &text).unwrap();
        aged(&next, 120);
        assert_eq!(
            predecessor(&bound, ID, &mut budget()),
            Predecessor::Clear { growing: false }
        );
    }
    #[test]
    fn predecessor_scan_fails_closed_on_budget_symlinks_and_unsafe_bound() {
        let fixture = Fixture::new();
        let bound = fixture.file("slug-a", &format!("{ID}.jsonl"), &header_line(ID));
        aged(&bound, 60);
        let next = fixture.file("slug-a", "fixture-session-c.jsonl", "{}\n");
        fixture.file("slug-a", "notes.txt", "{}\n");
        std::fs::create_dir(fixture.projects.join(format!("slug-a/{ID}"))).unwrap();
        let far = Instant::now() + Duration::from_secs(5);
        assert_eq!(
            predecessor(&bound, ID, &mut budget()),
            Predecessor::Clear { growing: true }
        );
        assert_eq!(
            predecessor(&bound, ID, &mut Budget::with_entries(2, far)),
            Predecessor::Unknown
        );
        assert_eq!(
            predecessor(&bound, ID, &mut Budget::new(Instant::now())),
            Predecessor::Unknown
        );
        std::fs::remove_file(&next).unwrap();
        let elsewhere = fixture.file(
            "slug-b",
            "fixture-session-c.jsonl",
            &successor(0, 0, ID, ID),
        );
        std::os::unix::fs::symlink(&elsewhere, &next).unwrap();
        assert_eq!(predecessor(&bound, ID, &mut budget()), Predecessor::Unknown);
        let alias = fixture.root.join("alias");
        std::os::unix::fs::symlink(&fixture.projects, &alias).unwrap();
        assert_eq!(
            predecessor(&alias.join(format!("slug-a/{ID}.jsonl")), ID, &mut budget()),
            Predecessor::Unknown
        );
    }
    #[test]
    fn record_identity_and_fork_helpers() {
        let parse = |text: &str| serde_json::from_str::<Value>(text).unwrap();
        let same = parse(&format!(
            "{{\"type\":\"assistant\",\"sessionId\":\"{ID}\"}}"
        ));
        assert_eq!(session_identity(&same, ID), Identity::Match);
        for text in [
            "{\"type\":\"file-history-snapshot\",\"messageId\":\"m1\"}",
            "{\"type\":\"file-history-delta\"}",
        ] {
            assert_eq!(session_identity(&parse(text), ID), Identity::Absent);
        }
        for text in [
            "{\"type\":\"user\",\"sessionId\":\"fixture-session-b\"}",
            "{\"type\":\"user\",\"sessionId\":null}",
            "{\"type\":\"user\",\"sessionId\":1}",
        ] {
            assert_eq!(session_identity(&parse(text), ID), Identity::Mismatch);
        }
        let snake = parse(&format!(
            "{{\"type\":\"user\",\"sessionId\":\"{ID}\",\"session_id\":\"fixture-session-b\"}}"
        ));
        assert_eq!(session_identity(&snake, ID), Identity::Match);
        assert!(!forked(&same));
        assert!(!forked(&parse("{\"forkedFrom\":null}")));
        assert!(forked(&parse(
            "{\"sessionId\":\"x\",\"forkedFrom\":{\"sessionId\":\"y\",\"messageUuid\":\"z\"}}"
        )));
    }
}
