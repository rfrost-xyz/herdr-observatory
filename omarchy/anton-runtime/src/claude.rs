//! Exact-session Claude Code transcript binding. A Herdr session id names one
//! `<root>/<entry>/<id>.jsonl` file, verified by its header and record identity.
//! Native paths and ids stay in process memory.
use crate::{
    Result,
    common::{self, hex_id, safe_id},
    native::{Fingerprint, line, timestamp_us},
    telemetry,
    turns::Turns,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fs::{File, Metadata};
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::Instant;

mod classifier;
pub use classifier::{Classifier, Outcome};

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

const SAFE: u64 = 9_007_199_254_740_991;
const RING: usize = 32;

/// Usage counters in source order: input, output, cache read, cache creation.
pub type Counters = [u64; 4];

fn sum(counters: &[u64]) -> Option<u64> {
    counters
        .iter()
        .try_fold(0u64, |total, value| total.checked_add(*value))
        .filter(|total| *total <= SAFE)
}

/// The D4 usage object of one response and its allowlisted model.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Response {
    pub usage: Counters,
    #[serde(deserialize_with = "Option::deserialize")]
    pub model: Option<String>,
}
impl Response {
    fn validate(&self) -> bool {
        sum(&self.usage).is_some() && self.model.as_deref().is_none_or(telemetry::safe_model)
    }
}

/// The open D3 group: hashed `message.id`, its counted contribution, the stop
/// state of its latest line and, when that stop is non-null, its response.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Group {
    pub id: String,
    pub usage: Counters,
    pub stop: u8,
    #[serde(deserialize_with = "Option::deserialize")]
    pub response: Option<Response>,
    /// Set when coverage failed while the group was open: it cannot restore
    /// `last_valid`, because it is not a complete group after the failure.
    pub tainted: bool,
}

/// Stop states: `null`, `end_turn`, `tool_use` and any other string.
pub const STOP_NULL: u8 = 1;
pub const STOP_END_TURN: u8 = 2;
pub const STOP_TOOL_USE: u8 = 3;
pub const STOP_OTHER: u8 = 4;

/// D8: every Claude parser field beyond the shared row-level cursor fields.
/// Every field is required, including each `Option`, so a block written by
/// another version never resumes with defaulted state.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ClaudeCursor {
    /// Sums of closed counted groups. The open group's contribution is separate.
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_creation: u64,
    pub totals_valid: bool,
    #[serde(deserialize_with = "Option::deserialize")]
    pub last: Option<Response>,
    pub last_valid: bool,
    pub usage_seq: u64,
    pub coverage_seq: u64,
    #[serde(deserialize_with = "Option::deserialize")]
    pub open: Option<Group>,
    pub closed: Vec<String>,
    pub compactions: u64,
    pub compaction_iteration: bool,
    #[serde(deserialize_with = "Option::deserialize")]
    pub pending_start: Option<(String, u64)>,
    pub abort_adjacent: bool,
    pub queued_since_start: bool,
    /// The oversized-record classifier while a line over `LINE` is being read.
    #[serde(deserialize_with = "Option::deserialize")]
    pub classifier: Option<Classifier>,
}
impl Default for ClaudeCursor {
    fn default() -> Self {
        Self {
            input: 0,
            output: 0,
            cache_read: 0,
            cache_creation: 0,
            totals_valid: true,
            last: None,
            last_valid: true,
            usage_seq: 0,
            coverage_seq: 0,
            open: None,
            closed: Vec::new(),
            compactions: 0,
            compaction_iteration: false,
            pending_start: None,
            abort_adjacent: false,
            queued_since_start: false,
            classifier: None,
        }
    }
}
impl ClaudeCursor {
    /// Cumulative counters including the open group's contribution.
    pub fn totals(&self) -> Option<Counters> {
        let open = self.open.as_ref().map(|g| g.usage).unwrap_or_default();
        let mut result = [0; 4];
        for (index, value) in [
            self.input,
            self.output,
            self.cache_read,
            self.cache_creation,
        ]
        .into_iter()
        .enumerate()
        {
            result[index] = sum(&[value, open[index]])?;
        }
        sum(&[result[0], result[2], result[3]])?;
        Some(result)
    }
    /// Revalidates every bound on reuse; `time` is the caller's Unix time.
    pub fn validate(&self, time: f64) -> bool {
        let horizon = ((time + 1.0) * 1e6).clamp(0.0, SAFE as f64) as u64;
        let mut ids = std::collections::BTreeSet::new();
        self.totals().is_some()
            && self.last.as_ref().is_none_or(Response::validate)
            && (self.last_valid || self.last.is_none())
            && self.usage_seq <= self.coverage_seq
            && self.coverage_seq <= horizon
            && self.open.as_ref().is_none_or(|group| {
                hex_id(&group.id, 64)
                    && (STOP_NULL..=STOP_OTHER).contains(&group.stop)
                    && (group.stop != STOP_NULL || group.response.is_none())
                    && (!group.tainted || group.response.is_none())
                    && group.response.as_ref().is_none_or(Response::validate)
                    && !self.closed.contains(&group.id)
            })
            && self.closed.len() <= RING
            && self
                .closed
                .iter()
                .all(|id| hex_id(id, 64) && ids.insert(id.as_str()))
            && self.compactions <= SAFE
            && self
                .pending_start
                .as_ref()
                .is_none_or(|(key, second)| hex_id(key, 24) && (1..=SAFE).contains(second))
            && self.classifier.as_ref().is_none_or(Classifier::validate)
    }
}

pub const KIND_OTHER: u8 = 0;
pub const KIND_ASSISTANT: u8 = 1;
pub const KIND_USER: u8 = 2;
pub const KIND_ATTACHMENT: u8 = 3;
pub const KIND_SYSTEM: u8 = 4;
pub const KIND_QUEUE: u8 = 5;
pub const SUBTYPE_COMPACT_BOUNDARY: u8 = 1;
pub const SUBTYPE_MICROCOMPACT_BOUNDARY: u8 = 2;
pub const SUBTYPE_TURN_DURATION: u8 = 3;
pub const SUBTYPE_STOP_HOOK_SUMMARY: u8 = 4;
const KINDS: &[&str] = &[
    "assistant",
    "user",
    "attachment",
    "system",
    "queue-operation",
];
const SUBTYPES: &[&str] = &[
    "compact_boundary",
    "microcompact_boundary",
    "turn_duration",
    "stop_hook_summary",
];

/// The fields of one record that replay consumes, extracted identically from a
/// parsed line or by the oversized-line classifier. Strings are hashed or
/// allowlisted; no content leaves the parser.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub kind: u8,
    pub subtype: u8,
    pub identity: u8,
    pub forked: bool,
    pub compact_summary: bool,
    pub meta: bool,
    /// Validated timestamp in microseconds, at most now plus 1 s.
    #[serde(deserialize_with = "Option::deserialize")]
    pub stamp: Option<u64>,
    /// The D7 turn key of `uuid`.
    #[serde(deserialize_with = "Option::deserialize")]
    pub uuid: Option<String>,
    /// sha256 of `message.id`.
    #[serde(deserialize_with = "Option::deserialize")]
    pub message: Option<String>,
    pub synthetic: bool,
    #[serde(deserialize_with = "Option::deserialize")]
    pub model: Option<String>,
    pub stop: u8,
    pub usage: [Option<u64>; 4],
    /// The D4 iteration, when Claude Code's rule selects one over top level.
    #[serde(deserialize_with = "Option::deserialize")]
    pub selected: Option<Counters>,
    pub compaction_iteration: bool,
    /// A consumed field is present with a value replay cannot represent.
    pub bad: bool,
}
pub const IDENTITY_ABSENT: u8 = 0;
pub const IDENTITY_MATCH: u8 = 1;
pub const IDENTITY_MISMATCH: u8 = 2;

fn index(names: &[&str], value: Option<&str>) -> u8 {
    value
        .and_then(|value| names.iter().position(|name| *name == value))
        .map_or(0, |position| position as u8 + 1)
}
fn horizon(time: f64) -> u64 {
    ((time + 1.0) * 1e6).clamp(0.0, SAFE as f64) as u64
}
pub fn turn_key(id: &str, uuid: &str) -> String {
    common::sha256(format!("anton-turn-v1:{id}:{uuid}").as_bytes())[..24].to_owned()
}
/// D4: the last iteration that is neither `advisor_message` nor `compaction`,
/// used only when it is `message` or `fallback_message` with four numeric
/// counters summing above zero.
fn select(kind: u8, counters: [Option<u64>; 4]) -> Option<Counters> {
    let counters = [counters[0]?, counters[1]?, counters[2]?, counters[3]?];
    ([1, 2].contains(&kind) && sum(&counters).is_some_and(|total| total > 0)).then_some(counters)
}
const ITERATIONS: &[&str] = &[
    "message",
    "fallback_message",
    "advisor_message",
    "compaction",
];

/// The consumed paths as (parent node, key); node 0 is the record and node
/// `ITEM` is each element of `message.usage.iterations`. The parsed path and
/// the oversized-line classifier both convert through `Record::set`.
pub(crate) const NODES: &[(u8, &str)] = &[
    (0, ""),
    (0, "type"),
    (0, "subtype"),
    (0, "sessionId"),
    (0, "uuid"),
    (0, "timestamp"),
    (0, "isMeta"),
    (0, "forkedFrom"),
    (0, "isCompactSummary"),
    (0, "message"),
    (MESSAGE, "id"),
    (MESSAGE, "model"),
    (MESSAGE, "stop_reason"),
    (MESSAGE, "usage"),
    (USAGE, "input_tokens"),
    (USAGE, "output_tokens"),
    (USAGE, "cache_read_input_tokens"),
    (USAGE, "cache_creation_input_tokens"),
    (USAGE, "iterations"),
    (ITERATIONS_NODE, ""),
    (ITEM, "type"),
    (ITEM, "input_tokens"),
    (ITEM, "output_tokens"),
    (ITEM, "cache_read_input_tokens"),
    (ITEM, "cache_creation_input_tokens"),
];
pub(crate) const MESSAGE: u8 = 9;
pub(crate) const USAGE: u8 = 13;
pub(crate) const ITERATIONS_NODE: u8 = 18;
pub(crate) const ITEM: u8 = 19;

/// The running D4 iteration fold: the compaction flag and the latest element
/// that is neither advisor nor compaction, as its kind and counters.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Iterations {
    /// 0 absent or null, 1 an array, 2 any other value.
    pub shape: u8,
    pub compaction: bool,
    #[serde(deserialize_with = "Option::deserialize")]
    pub candidate: Option<(u8, [Option<u64>; 4])>,
    /// The element being read.
    pub kind: u8,
    pub counters: [Option<u64>; 4],
}
impl Iterations {
    /// Sets one field of the element being read.
    pub(crate) fn set(&mut self, node: u8, value: &Value) {
        match node {
            20 => self.kind = index(ITERATIONS, value.as_str()),
            21..=24 => self.counters[(node - 21) as usize] = common::number(value),
            _ => {}
        }
    }
    /// Folds the element being read into the running selection.
    pub(crate) fn push(&mut self) {
        if self.kind == 4 {
            self.compaction = true;
        }
        if ![3, 4].contains(&self.kind) {
            self.candidate = Some((self.kind, self.counters));
        }
        self.kind = 0;
        self.counters = [None; 4];
    }
}

impl Record {
    /// Sets one consumed scalar field from its JSON value. A container value
    /// arrives as an empty object or array of the same kind.
    pub(crate) fn set(&mut self, node: u8, value: &Value, id: &str, time: f64) {
        let wrap = |key: &str| json!({ key: value });
        match node {
            1 => self.kind = index(KINDS, value.as_str()),
            2 => self.subtype = index(SUBTYPES, value.as_str()),
            3 => {
                self.identity = match session_identity(&wrap("sessionId"), id) {
                    Identity::Absent => IDENTITY_ABSENT,
                    Identity::Match => IDENTITY_MATCH,
                    Identity::Mismatch => IDENTITY_MISMATCH,
                }
            }
            4 => {
                self.uuid = value
                    .as_str()
                    .filter(|uuid| safe_id(uuid, 128))
                    .map(|uuid| turn_key(id, uuid))
            }
            5 => {
                self.stamp = value
                    .as_str()
                    .and_then(timestamp_us)
                    .filter(|stamp| *stamp <= horizon(time))
            }
            6 => self.meta = *value == true,
            7 => self.forked = forked(&wrap("forkedFrom")),
            8 => self.compact_summary = !value.is_null() && *value != false,
            10 => {
                self.message = value
                    .as_str()
                    .filter(|id| safe_id(id, 128))
                    .map(|id| common::sha256(id.as_bytes()))
            }
            11 => {
                self.synthetic = value == "<synthetic>";
                self.model = value
                    .as_str()
                    .filter(|model| telemetry::safe_model(model))
                    .map(str::to_owned);
            }
            12 => {
                self.stop = match value {
                    Value::Null => STOP_NULL,
                    Value::String(stop) if stop == "end_turn" => STOP_END_TURN,
                    Value::String(stop) if stop == "tool_use" => STOP_TOOL_USE,
                    Value::String(_) => STOP_OTHER,
                    _ => 0,
                }
            }
            14..=17 => self.usage[(node - 14) as usize] = common::number(value),
            _ => {}
        }
    }
    /// Completes a record once every field is set: assistant-only fields are
    /// cleared on other types and the D4 selection and `bad` flag are derived.
    pub(crate) fn finish(&mut self, message_object: bool, iterations: &Iterations) {
        if self.kind != KIND_ASSISTANT {
            *self = Self {
                kind: self.kind,
                subtype: self.subtype,
                identity: self.identity,
                forked: self.forked,
                compact_summary: self.compact_summary,
                meta: self.meta,
                stamp: self.stamp,
                uuid: self.uuid.take(),
                ..Self::default()
            };
            return;
        }
        self.compaction_iteration = iterations.compaction;
        self.selected = iterations
            .candidate
            .and_then(|(kind, counters)| select(kind, counters));
        self.bad = !message_object
            || self.message.is_none()
            || self.stop == 0
            || self.stamp.is_none()
            || iterations.shape == 2;
    }
    /// Extracts a parsed line; `None` when the line is not a JSON object.
    pub fn from_value(value: &Value, id: &str, time: f64) -> Option<Self> {
        value.as_object()?;
        let mut record = Self::default();
        let mut iterations = Iterations::default();
        for (node, (parent, key)) in NODES.iter().enumerate() {
            let parent = match *parent {
                0 => value,
                MESSAGE => &value["message"],
                USAGE => &value["message"]["usage"],
                _ => continue,
            };
            if let Some(field) = parent.as_object().and_then(|object| object.get(*key)) {
                record.set(node as u8, field, id, time);
            }
        }
        match value.pointer("/message/usage/iterations") {
            None | Some(Value::Null) => {}
            Some(Value::Array(list)) => {
                iterations.shape = 1;
                for item in list {
                    for (node, (parent, key)) in NODES.iter().enumerate() {
                        if *parent == ITEM
                            && let Some(field) =
                                item.as_object().and_then(|object| object.get(*key))
                        {
                            iterations.set(node as u8, field);
                        }
                    }
                    iterations.push();
                }
            }
            Some(_) => iterations.shape = 2,
        }
        record.finish(value["message"].is_object(), &iterations);
        Some(record)
    }
}

/// Row-level replay state. Field names and meanings match `native::Cursor` so
/// the native dispatch maps between them, except that `seq` starts at 0 (D6).
#[derive(Clone, Debug)]
pub struct Row {
    pub file: [u64; 2],
    pub offset: u64,
    pub children: BTreeMap<String, String>,
    pub seq: u64,
    pub valid: bool,
    pub compactions_valid: bool,
    pub caught_up: bool,
    pub skipping: bool,
    pub turns: Turns,
    pub fingerprint: Option<Fingerprint>,
    pub at: f64,
    pub claude: ClaudeCursor,
}
impl Row {
    pub fn new(file: [u64; 2], offset: u64, time: f64) -> Self {
        Self {
            file,
            offset,
            children: BTreeMap::new(),
            seq: 0,
            valid: true,
            compactions_valid: true,
            caught_up: false,
            skipping: false,
            turns: Turns::default(),
            fingerprint: None,
            at: time,
            claude: ClaudeCursor::default(),
        }
    }
    /// Last-response values stay unknown until the next complete counted group.
    fn fail_last(&mut self) {
        let block = &mut self.claude;
        block.last_valid = false;
        block.last = None;
        if let Some(group) = &mut block.open {
            group.tainted = true;
            group.response = None;
        }
    }
    fn fail_totals(&mut self) {
        self.claude.totals_valid = false;
        self.fail_last();
    }
    /// An unparseable line, or a record naming another session.
    pub fn invalid(&mut self) {
        self.valid = false;
        self.compactions_valid = false;
        self.turns.unknown();
        self.fail_totals();
    }
    /// D3 coverage table for a relevant record that cannot be classified.
    fn unclassified(&mut self, kind: u8) {
        match kind {
            KIND_ASSISTANT => self.fail_totals(),
            KIND_SYSTEM => self.compactions_valid = false,
            KIND_USER | KIND_ATTACHMENT => self.valid = false,
            _ => return,
        }
        self.turns.unknown();
    }
    fn close_group(&mut self) {
        let Some(group) = self.claude.open.take() else {
            return;
        };
        // `assistant` keeps sums plus the open contribution within 2^53.
        let block = &mut self.claude;
        block.input += group.usage[0];
        block.output += group.usage[1];
        block.cache_read += group.usage[2];
        block.cache_creation += group.usage[3];
        if let Some(response) = group.response {
            block.last = Some(response);
            block.last_valid = true;
        }
        block.closed.push(group.id);
        if block.closed.len() > RING {
            block.closed.remove(0);
        }
    }
    /// Applies one record's extracted fields to the replay state.
    pub fn apply(&mut self, record: &Record) {
        if let Some(stamp) = record.stamp {
            self.claude.coverage_seq = self.claude.coverage_seq.max(stamp);
        }
        let relevant =
            [KIND_ASSISTANT, KIND_USER, KIND_ATTACHMENT, KIND_SYSTEM].contains(&record.kind);
        match record.identity {
            IDENTITY_MISMATCH => return self.invalid(),
            IDENTITY_ABSENT => return self.unclassified(record.kind),
            _ => {}
        }
        if record.forked || record.kind == KIND_ASSISTANT && record.synthetic {
            return;
        }
        if record.bad && relevant {
            return self.unclassified(record.kind);
        }
        if record.kind == KIND_SYSTEM && record.subtype == SUBTYPE_COMPACT_BOUNDARY {
            self.claude.compactions = (self.claude.compactions + 1).min(SAFE);
        }
        if record.kind == KIND_ASSISTANT {
            self.assistant(record);
        }
    }
    /// Feeds one chunk of a line over `LINE` to the classifier. Malformed JSON
    /// fails closed at once and the rest of the line is skipped unread.
    fn oversized(&mut self, bytes: &[u8], terminated: bool, id: &str, time: f64) {
        let fresh = !self.skipping;
        self.skipping = !terminated;
        let mut classifier = match self.claude.classifier.take() {
            Some(classifier) => classifier,
            None if fresh => Classifier::default(),
            None => return,
        };
        if !classifier.feed(bytes, id, time) {
            return self.invalid();
        }
        if !terminated {
            self.claude.classifier = Some(classifier);
            return;
        }
        match classifier.finish() {
            Outcome::Invalid => self.invalid(),
            Outcome::Unclassified(kind) => self.unclassified(kind),
            Outcome::Record(record) => self.apply(&record),
        }
    }
    /// D3 grouping: a different `message.id` closes the open group, a later line
    /// of the open group replaces its contribution, and a closed id reopening
    /// makes totals unknown.
    fn assistant(&mut self, record: &Record) {
        let (Some(id), Some(stamp)) = (record.message.clone(), record.stamp) else {
            return self.unclassified(KIND_ASSISTANT);
        };
        if self.claude.open.as_ref().is_none_or(|group| group.id != id) {
            self.close_group();
            if self.claude.closed.contains(&id) {
                return self.fail_totals();
            }
            self.claude.open = Some(Group {
                id,
                usage: [0; 4],
                stop: STOP_NULL,
                response: None,
                tainted: false,
            });
        }
        let counters = [
            record.usage[0],
            record.usage[1],
            record.usage[2],
            record.usage[3],
        ];
        let complete = counters.iter().all(Option::is_some);
        let usage = counters.map(|v| v.unwrap_or(0));
        let block = &mut self.claude;
        block.usage_seq = block.usage_seq.max(stamp);
        block.compaction_iteration |= record.compaction_iteration;
        let group = block.open.as_mut().unwrap();
        group.usage = usage;
        group.stop = record.stop;
        group.response =
            (complete && record.stop != STOP_NULL && !group.tainted).then(|| Response {
                usage: record.selected.unwrap_or(usage),
                model: record.model.clone(),
            });
        if !complete || block.totals().is_none() {
            // Totals stay unknown; a zero contribution keeps the sums bounded.
            if let Some(group) = &mut block.open {
                group.usage = [0; 4];
            }
            self.fail_totals();
        }
    }
}

impl Row {
    /// D3 to D5 projection into the telemetry object. `window` and
    /// `context_percent` are omitted rather than null, so a merge never replaces
    /// metadata values (D4). Publishing it only when caught up is the caller's.
    pub fn usage(&self) -> Value {
        let block = &self.claude;
        let known = block.usage_seq > 0;
        let totals = block.totals().filter(|_| known && block.totals_valid);
        let response = block
            .open
            .as_ref()
            .and_then(|group| group.response.clone())
            .or_else(|| block.last.clone().filter(|_| block.last_valid))
            .filter(|_| known);
        let context = response
            .as_ref()
            .and_then(|r| sum(&[r.usage[0], r.usage[2], r.usage[3]]));
        let compactions = (known && self.compactions_valid && !block.compaction_iteration)
            .then_some(block.compactions);
        let total = |index: usize| totals.map(|t| t[index]);
        json!({
            "total_input": totals.and_then(|t| sum(&[t[0], t[2], t[3]])),
            "total_cache_read": total(2),
            "total_cache_write": total(3),
            "total_uncached_input": total(0),
            "total_output": total(1),
            "input": context,
            "output_tokens": response.as_ref().map(|r| r.usage[1]),
            "cache_read": response.as_ref().map(|r| r.usage[2]),
            "cache_write": response.as_ref().map(|r| r.usage[3]),
            "context": context,
            "model": response.and_then(|r| r.model),
            "usage_seq": known.then_some(block.usage_seq),
            "usage_source": known.then_some("claude-transcript"),
            "compactions": compactions,
        })
    }
    /// Whether a checkpointed row may resume this file: same dev/inode, header
    /// and tail hashes, an offset within the file, and every bound revalidated.
    fn resumable(
        &self,
        stream: &mut BufReader<File>,
        head: &[u8],
        info: &Metadata,
        time: f64,
    ) -> bool {
        let mtime = mtime_us(info);
        let matches = self.file == [info.dev(), info.ino()]
            && (self.skipping || self.claude.classifier.is_none())
            && head.len() as u64 <= self.offset
            && self.offset <= info.len()
            && self.turns.validate()
            && self.claude.validate(time)
            && self.fingerprint.as_ref().is_some_and(|f| {
                f.header == common::sha256(head)
                    && self.offset <= f.size
                    && f.size <= info.len()
                    && (f.size != info.len() || f.mtime_us == mtime)
            });
        matches
            && tail(stream, self.offset)
                .is_ok_and(|hash| self.fingerprint.as_ref().is_some_and(|f| f.tail == hash))
    }
}

const TAIL: usize = 524288;

fn mtime_us(info: &Metadata) -> u64 {
    (info.mtime().max(0) as u64) * 1_000_000 + (info.mtime_nsec().max(0) as u64) / 1000
}
/// sha256 of the up to 1 KiB that end at `offset`.
fn tail(stream: &mut BufReader<File>, offset: u64) -> Result<String> {
    stream
        .seek(SeekFrom::Start(offset.saturating_sub(1024)))
        .map_err(|_| "Claude session seek failed")?;
    let mut bytes = vec![0; offset.min(1024) as usize];
    stream
        .read_exact(&mut bytes)
        .map_err(|_| "Claude session read failed")?;
    Ok(common::sha256(&bytes))
}
fn position(stream: &mut BufReader<File>) -> Result<u64> {
    Ok(stream
        .stream_position()
        .map_err(|_| "Claude session seek failed")?)
}

/// One bounded replay pass over a bound session file. It resumes `previous`
/// only when the file is provably the same and has only grown; otherwise it
/// starts after the header. A pass reads at most `TAIL` bytes and stops at
/// `deadline`; a partial last line waits for the next pass.
pub fn replay(
    root: &Path,
    path: &Path,
    id: &str,
    previous: Option<Row>,
    time: f64,
    deadline: Instant,
) -> Result<Row> {
    let (mut stream, head) = open_session(root, path, id)?;
    let info = stream
        .get_ref()
        .metadata()
        .map_err(|_| "Claude session stat failed")?;
    let mut row = match previous {
        Some(row) if row.resumable(&mut stream, &head, &info, time) => row,
        _ => Row::new([info.dev(), info.ino()], head.len() as u64, time),
    };
    stream
        .seek(SeekFrom::Start(row.offset))
        .map_err(|_| "Claude session seek failed")?;
    let end = info.len().min(row.offset + TAIL as u64);
    while position(&mut stream)? < end && Instant::now() < deadline {
        let offset = position(&mut stream)?;
        let bytes = line(&mut stream, (LINE + 1).min((end - offset) as usize))
            .map_err(|_| "Claude session read failed")?;
        let terminated = bytes.last() == Some(&b'\n');
        if row.skipping || bytes.len() > LINE {
            row.oversized(&bytes, terminated, id, time);
            row.offset = position(&mut stream)?;
            continue;
        }
        if !terminated {
            break;
        }
        row.offset = position(&mut stream)?;
        match serde_json::from_slice::<Value>(&bytes)
            .ok()
            .and_then(|value| Record::from_value(&value, id, time))
        {
            Some(record) => row.apply(&record),
            None => row.invalid(),
        }
    }
    if let Some(classifier) = &mut row.claude.classifier {
        classifier.suspend();
    }
    row.caught_up = row.offset == info.len() && !row.skipping;
    row.at = time;
    row.fingerprint = Some(Fingerprint {
        size: info.len(),
        mtime_us: mtime_us(&info),
        header: common::sha256(&head),
        tail: tail(&mut stream, row.offset)?,
    });
    Ok(row)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::now;
    use std::time::Duration;
    pub(super) struct Fixture {
        root: PathBuf,
        pub(super) projects: PathBuf,
    }
    impl Fixture {
        pub(super) fn new() -> Self {
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
        pub(super) fn file(&self, entry: &str, name: &str, text: &str) -> PathBuf {
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
#[cfg(test)]
mod replay_tests {
    use super::*;
    use crate::common::now;
    use std::time::Duration;
    const ID: &str = "fixture-session-a";
    const BASE: u64 = 1_767_225_600;

    fn stamp(second: u64) -> String {
        format!(
            "2026-01-01T{:02}:{:02}:{:02}.250Z",
            second / 3600,
            second / 60 % 60,
            second % 60
        )
    }
    fn micros(second: u64) -> u64 {
        (BASE + second) * 1_000_000 + 250_000
    }
    /// A synthetic assistant line; `stop` is raw JSON (`null` or a string).
    fn assistant(message: &str, second: u64, stop: &str, usage: [u64; 4]) -> String {
        assistant_with(message, second, stop, &counters(usage), "")
    }
    fn counters(usage: [u64; 4]) -> String {
        format!(
            "\"input_tokens\":{},\"output_tokens\":{},\"cache_read_input_tokens\":{},\"cache_creation_input_tokens\":{}",
            usage[0], usage[1], usage[2], usage[3]
        )
    }
    fn assistant_with(message: &str, second: u64, stop: &str, usage: &str, extra: &str) -> String {
        format!(
            "{{\"type\":\"assistant\",\"sessionId\":\"{ID}\",\"uuid\":\"a-{message}-{second}\",\"timestamp\":\"{}\",{extra}\"message\":{{\"id\":\"{message}\",\"model\":\"claude-fixture-1\",\"stop_reason\":{stop},\"usage\":{{{usage}}},\"content\":[]}}}}",
            stamp(second)
        )
    }
    fn user(second: u64) -> String {
        format!(
            "{{\"type\":\"user\",\"sessionId\":\"{ID}\",\"uuid\":\"u-{second}\",\"timestamp\":\"{}\",\"message\":{{\"role\":\"user\",\"content\":\"synthetic\"}}}}",
            stamp(second)
        )
    }
    fn attachment(second: u64) -> String {
        format!(
            "{{\"type\":\"attachment\",\"sessionId\":\"{ID}\",\"timestamp\":\"{}\",\"attachment\":{{\"type\":\"fixture\"}}}}",
            stamp(second)
        )
    }
    fn system(subtype: &str, second: u64) -> String {
        format!(
            "{{\"type\":\"system\",\"subtype\":\"{subtype}\",\"sessionId\":\"{ID}\",\"timestamp\":\"{}\"}}",
            stamp(second)
        )
    }
    fn run(lines: &[String]) -> Row {
        let mut row = Row::new([1, 2], 0, now());
        for line in lines {
            match serde_json::from_str::<Value>(line)
                .ok()
                .and_then(|value| Record::from_value(&value, ID, now()))
            {
                Some(record) => row.apply(&record),
                None => row.invalid(),
            }
        }
        row
    }
    fn get(row: &Row, key: &str) -> Value {
        row.usage()[key].clone()
    }
    fn totals(row: &Row) -> [Value; 5] {
        let usage = row.usage();
        [
            "total_input",
            "total_output",
            "total_cache_read",
            "total_cache_write",
            "total_uncached_input",
        ]
        .map(|key| usage[key].clone())
    }

    #[test]
    fn usage_identical_split_and_streaming_partial_groups_count_once() {
        let split = run(&[
            assistant("msg_a", 1, "null", [10, 5, 100, 20]),
            assistant("msg_a", 2, "null", [10, 5, 100, 20]),
            assistant("msg_a", 3, "\"end_turn\"", [10, 5, 100, 20]),
        ]);
        assert_eq!(
            totals(&split),
            [json!(130), json!(5), json!(100), json!(20), json!(10)]
        );
        let partial = run(&[
            assistant("msg_a", 1, "null", [10, 1, 100, 20]),
            assistant("msg_a", 2, "null", [10, 3, 100, 20]),
            assistant("msg_a", 3, "\"tool_use\"", [10, 9, 100, 20]),
            assistant("msg_b", 4, "\"end_turn\"", [2, 4, 130, 0]),
        ]);
        assert_eq!(
            totals(&partial),
            [json!(262), json!(13), json!(230), json!(20), json!(12)]
        );
        for (key, value) in [
            ("input", json!(132)),
            ("context", json!(132)),
            ("output_tokens", json!(4)),
            ("cache_read", json!(130)),
            ("cache_write", json!(0)),
            ("model", json!("claude-fixture-1")),
            ("usage_seq", json!(micros(4))),
            ("usage_source", json!("claude-transcript")),
            ("compactions", json!(0)),
        ] {
            assert_eq!(get(&partial, key), value, "{key}");
        }
        let usage = partial.usage();
        assert!(usage.get("window").is_none() && usage.get("context_percent").is_none());
    }
    #[test]
    fn usage_groups_survive_interleaved_user_attachment_and_system_records() {
        let row = run(&[
            assistant("msg_a", 1, "null", [1, 2, 3, 4]),
            user(2),
            attachment(3),
            system("turn_duration", 4),
            "{\"type\":\"queue-operation\",\"operation\":\"enqueue\"}".to_owned(),
            "{\"type\":\"file-history-snapshot\",\"messageId\":\"m\"}".to_owned(),
            assistant("msg_a", 5, "\"end_turn\"", [1, 2, 3, 4]),
        ]);
        assert_eq!(
            totals(&row),
            [json!(8), json!(2), json!(3), json!(4), json!(1)]
        );
        assert_eq!(get(&row, "context"), json!(8));
        assert_eq!(row.claude.closed.len(), 0);
        assert!(row.valid && row.compactions_valid && row.turns.valid);
    }
    #[test]
    fn usage_reopened_closed_group_makes_totals_unknown_until_next_group() {
        let row = run(&[
            assistant("msg_a", 1, "\"end_turn\"", [1, 1, 1, 1]),
            assistant("msg_b", 2, "\"end_turn\"", [2, 2, 2, 2]),
            assistant("msg_a", 3, "\"end_turn\"", [1, 1, 1, 1]),
        ]);
        assert_eq!(
            totals(&row),
            [
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null
            ]
        );
        assert_eq!(get(&row, "context"), Value::Null);
        assert_eq!(get(&row, "model"), Value::Null);
        let mut lines = vec![
            assistant("msg_a", 1, "\"end_turn\"", [1, 1, 1, 1]),
            assistant("msg_b", 2, "\"end_turn\"", [2, 2, 2, 2]),
            assistant("msg_a", 3, "\"end_turn\"", [1, 1, 1, 1]),
            assistant("msg_c", 4, "\"end_turn\"", [3, 5, 7, 9]),
        ];
        let row = run(&lines);
        assert_eq!(get(&row, "total_input"), Value::Null);
        assert_eq!(get(&row, "context"), json!(19));
        assert_eq!(get(&row, "output_tokens"), json!(5));
        // The ring holds the last 32 closed ids; an older id is beyond it.
        lines.truncate(1);
        for index in 0..33 {
            lines.push(assistant(
                &format!("msg_n{index}"),
                10 + index,
                "\"end_turn\"",
                [1, 0, 0, 0],
            ));
        }
        lines.push(assistant("msg_a", 50, "\"end_turn\"", [1, 0, 0, 0]));
        let row = run(&lines);
        assert_eq!(row.claude.closed.len(), 32);
        assert_eq!(get(&row, "total_uncached_input"), json!(35));
    }

    fn iterations(list: &[(&str, [u64; 4])]) -> String {
        let items: Vec<String> = list
            .iter()
            .map(|(kind, usage)| format!("{{\"type\":\"{kind}\",{}}}", counters(*usage)))
            .collect();
        format!(",\"iterations\":[{}]", items.join(","))
    }
    #[test]
    fn usage_advisor_iterations_select_context_and_top_level_feeds_totals() {
        let top = counters([5, 30, 200, 10]);
        let advisor = iterations(&[
            ("message", [2, 10, 100, 5]),
            ("advisor_message", [50, 50, 50, 50]),
            ("message", [3, 20, 100, 5]),
        ]);
        let row = run(&[assistant_with(
            "msg_a",
            1,
            "\"end_turn\"",
            &(top.clone() + &advisor),
            "",
        )]);
        assert_eq!(
            totals(&row),
            [json!(215), json!(30), json!(200), json!(10), json!(5)]
        );
        assert_eq!(get(&row, "context"), json!(108));
        assert_eq!(get(&row, "input"), json!(108));
        assert_eq!(get(&row, "output_tokens"), json!(20));
        for (list, context) in [
            (iterations(&[("fallback_message", [1, 1, 1, 1])]), 3),
            (iterations(&[("message", [0, 0, 0, 0])]), 215),
            (
                iterations(&[("message", [1, 1, 1, 1]), ("tool_round", [9, 9, 9, 9])]),
                215,
            ),
            (
                iterations(&[("message", [1, 1, 1, 1]), ("advisor_message", [9, 9, 9, 9])]),
                3,
            ),
            (
                ",\"iterations\":[{\"type\":\"message\",\"input_tokens\":1}]".to_owned(),
                215,
            ),
            (",\"iterations\":[7]".to_owned(), 215),
            (",\"iterations\":[]".to_owned(), 215),
            (",\"iterations\":null".to_owned(), 215),
        ] {
            let row = run(&[assistant_with(
                "msg_a",
                1,
                "\"end_turn\"",
                &(top.clone() + &list),
                "",
            )]);
            assert_eq!(get(&row, "context"), json!(context), "{list}");
            assert_eq!(get(&row, "total_input"), json!(215), "{list}");
        }
        let row = run(&[assistant_with(
            "msg_a",
            1,
            "\"end_turn\"",
            &(top + ",\"iterations\":{}"),
            "",
        )]);
        assert_eq!(get(&row, "total_input"), Value::Null);
    }
    #[test]
    fn usage_compaction_iteration_makes_compactions_unknown() {
        let top = counters([5, 30, 200, 10]);
        let list = iterations(&[("compaction", [1, 1, 1, 1]), ("message", [2, 2, 2, 2])]);
        let row = run(&[
            system("compact_boundary", 1),
            assistant_with("msg_a", 2, "\"end_turn\"", &(top + &list), ""),
        ]);
        assert_eq!(get(&row, "compactions"), Value::Null);
        assert_eq!(get(&row, "context"), json!(6));
        assert_eq!(get(&row, "total_input"), json!(215));
    }
    #[test]
    fn usage_synthetic_records_are_neutral() {
        let synthetic = |message: &str, second: u64| {
            assistant(message, second, "\"stop_sequence\"", [0, 0, 0, 0])
                .replace("claude-fixture-1", "<synthetic>")
        };
        let row = run(&[
            assistant("msg_a", 1, "null", [1, 2, 3, 4]),
            synthetic("msg_s", 2),
            assistant("msg_a", 3, "\"end_turn\"", [1, 2, 3, 4]),
            synthetic("msg_a", 4),
        ]);
        assert_eq!(
            totals(&row),
            [json!(8), json!(2), json!(3), json!(4), json!(1)]
        );
        assert_eq!(get(&row, "model"), json!("claude-fixture-1"));
        assert_eq!(get(&row, "usage_seq"), json!(micros(3)));
        assert!(row.claude.closed.is_empty());
    }
    #[test]
    fn usage_aborted_group_counts_but_is_never_last_response() {
        let row = run(&[
            assistant("msg_a", 1, "\"end_turn\"", [1, 2, 3, 4]),
            assistant("msg_b", 2, "null", [10, 20, 30, 40]),
        ]);
        assert_eq!(
            totals(&row),
            [json!(88), json!(22), json!(33), json!(44), json!(11)]
        );
        assert_eq!(get(&row, "context"), json!(8));
        assert_eq!(get(&row, "output_tokens"), json!(2));
        assert_eq!(get(&row, "usage_seq"), json!(micros(2)));
        let row = run(&[
            assistant("msg_a", 1, "\"end_turn\"", [1, 2, 3, 4]),
            assistant("msg_b", 2, "null", [10, 20, 30, 40]),
            assistant("msg_c", 3, "\"tool_use\"", [5, 0, 0, 0]),
        ]);
        assert_eq!(get(&row, "context"), json!(5));
        let unsafe_model = run(&[assistant("msg_a", 1, "\"end_turn\"", [1, 2, 3, 4])
            .replace("claude-fixture-1", "../model")]);
        assert_eq!(get(&unsafe_model, "model"), Value::Null);
        assert_eq!(get(&unsafe_model, "context"), json!(8));
    }

    #[test]
    fn usage_missing_counter_or_unparseable_line_is_unknown_until_next_group() {
        let missing = "\"input_tokens\":1,\"output_tokens\":2,\"cache_read_input_tokens\":3";
        let float = "\"input_tokens\":1.0,\"output_tokens\":2,\"cache_read_input_tokens\":3,\"cache_creation_input_tokens\":4";
        for broken in [
            assistant_with("msg_b", 2, "\"end_turn\"", missing, ""),
            assistant_with("msg_b", 2, "\"end_turn\"", float, ""),
            "{\"type\":\"assistant\",\"sessionId\":\"fixture".to_owned(),
            "[1,2]".to_owned(),
        ] {
            let mut lines = vec![
                assistant("msg_a", 1, "\"end_turn\"", [1, 2, 3, 4]),
                broken.clone(),
            ];
            let row = run(&lines);
            assert_eq!(get(&row, "total_input"), Value::Null, "{broken}");
            assert_eq!(get(&row, "context"), Value::Null, "{broken}");
            assert_eq!(get(&row, "model"), Value::Null, "{broken}");
            lines.push(assistant("msg_c", 3, "\"end_turn\"", [3, 3, 3, 3]));
            let row = run(&lines);
            assert_eq!(get(&row, "total_output"), Value::Null, "{broken}");
            assert_eq!(get(&row, "context"), json!(9), "{broken}");
            assert_eq!(get(&row, "model"), json!("claude-fixture-1"), "{broken}");
        }
        let row = run(&["not json".to_owned(), system("compact_boundary", 1)]);
        assert!(!row.valid && !row.compactions_valid && !row.turns.valid);
        assert_eq!(get(&row, "compactions"), Value::Null);
        // A failure inside an open group taints it: its later lines cannot
        // restore the last response, only a group opened afterwards can.
        let row = run(&[
            assistant("msg_a", 1, "null", [1, 2, 3, 4]),
            "not json".to_owned(),
            assistant("msg_a", 2, "\"end_turn\"", [1, 2, 3, 4]),
        ]);
        assert_eq!(get(&row, "context"), Value::Null);
    }
    #[test]
    fn identity_mismatch_fork_and_absent_session_feed_no_metric() {
        let good = assistant("msg_a", 1, "\"end_turn\"", [1, 2, 3, 4]);
        let other = good
            .replace(ID, "fixture-session-b")
            .replace("msg_a", "msg_b");
        let row = run(&[good.clone(), other]);
        assert_eq!(
            totals(&row),
            [
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null
            ]
        );
        assert!(!row.valid && !row.compactions_valid && !row.turns.valid);
        let fork = assistant_with(
            "msg_f",
            2,
            "\"end_turn\"",
            &counters([50, 50, 50, 50]),
            "\"forkedFrom\":{\"sessionId\":\"fixture-session-b\",\"messageUuid\":\"x\"},",
        );
        let compact =
            system("compact_boundary", 3).replace("\"system\",", "\"system\",\"forkedFrom\":{},");
        let snapshot =
            "{\"type\":\"file-history-snapshot\",\"messageId\":\"m\",\"snapshot\":{}}".to_owned();
        let row = run(&[good.clone(), fork, compact, snapshot]);
        assert_eq!(
            totals(&row),
            [json!(8), json!(2), json!(3), json!(4), json!(1)]
        );
        assert_eq!(get(&row, "compactions"), json!(0));
        assert_eq!(get(&row, "usage_seq"), json!(micros(1)));
        assert!(row.valid && row.compactions_valid && row.turns.valid);
        let null_fork = good.replace(
            "\"type\":\"assistant\",",
            "\"type\":\"assistant\",\"forkedFrom\":null,",
        );
        assert_eq!(get(&run(&[null_fork]), "total_output"), json!(2));
        let anonymous = good.replace(&format!("\"sessionId\":\"{ID}\","), "");
        let row = run(&[anonymous]);
        assert_eq!(get(&row, "total_input"), Value::Null);
        assert!(row.valid && row.compactions_valid && !row.turns.valid);
    }
    #[test]
    fn compactions_count_boundaries_only() {
        let summary = format!(
            "{{\"type\":\"user\",\"sessionId\":\"{ID}\",\"isCompactSummary\":true,\"timestamp\":\"{}\",\"message\":{{\"content\":\"synthetic\"}}}}",
            stamp(3)
        );
        let row = run(&[
            system("compact_boundary", 1),
            system("microcompact_boundary", 2),
            summary,
            system("compact_boundary", 4),
            assistant("msg_a", 5, "\"end_turn\"", [1, 2, 3, 4]),
        ]);
        assert_eq!(get(&row, "compactions"), json!(2));
        let row = run(&[system("compact_boundary", 1)]);
        assert_eq!(row.claude.compactions, 1);
        assert_eq!(get(&row, "compactions"), Value::Null);
    }
    #[test]
    fn usage_seq_is_the_largest_counted_timestamp_and_coverage_covers_all() {
        let row = run(&[
            assistant("msg_a", 9, "null", [1, 0, 0, 0]),
            assistant("msg_a", 5, "\"end_turn\"", [1, 0, 0, 0]),
            assistant("msg_b", 7, "\"end_turn\"", [1, 0, 0, 0]),
            user(30),
        ]);
        assert_eq!(get(&row, "usage_seq"), json!(micros(9)));
        assert_eq!(row.claude.coverage_seq, micros(30));
        for line in [
            assistant("msg_c", 0, "\"end_turn\"", [1, 0, 0, 0])
                .replace(&stamp(0), "2026-01-01T00:00:00"),
            assistant("msg_c", 0, "\"end_turn\"", [1, 0, 0, 0])
                .replace(&stamp(0), "2999-01-01T00:00:00Z"),
            assistant_with("msg_c", 0, "7", &counters([1, 0, 0, 0]), ""),
            assistant("msg_c", 0, "\"end_turn\"", [1, 0, 0, 0])
                .replace("\"stop_reason\":\"end_turn\",", ""),
            assistant("msg-c", 0, "\"end_turn\"", [1, 0, 0, 0]).replace("msg-c", "bad id"),
        ] {
            let row = run(&[
                assistant("msg_a", 1, "\"end_turn\"", [1, 0, 0, 0]),
                line.clone(),
            ]);
            assert_eq!(get(&row, "total_input"), Value::Null, "{line}");
            assert_eq!(get(&row, "usage_seq"), json!(micros(1)), "{line}");
        }
        let empty = run(&[user(1)]);
        assert!(
            empty
                .usage()
                .as_object()
                .unwrap()
                .values()
                .all(Value::is_null)
        );
    }

    fn header() -> String {
        format!("{{\"type\":\"mode\",\"mode\":\"normal\",\"sessionId\":\"{ID}\"}}\n")
    }
    fn padded_user(second: u64, pad: usize) -> String {
        user(second).replace("synthetic", &"x".repeat(pad))
    }
    /// Replays to completion with the block round-tripped through serde between
    /// passes, as a checkpoint would; returns the row and the pass count.
    fn passes(fixture: &tests::Fixture, path: &Path, mut row: Option<Row>) -> (Row, usize) {
        for count in 1..64 {
            let deadline = Instant::now() + Duration::from_secs(5);
            let mut next = replay(&fixture.projects, path, ID, row, now(), deadline).unwrap();
            next.claude =
                serde_json::from_value(serde_json::to_value(&next.claude).unwrap()).unwrap();
            if next.caught_up {
                return (next, count);
            }
            row = Some(next);
        }
        panic!("replay never caught up");
    }
    fn body(lines: &[String]) -> String {
        lines.iter().map(|line| format!("{line}\n")).collect()
    }
    #[test]
    fn replay_resumes_byte_cursor_across_tail_bounded_passes() {
        let fixture = tests::Fixture::new();
        let mut lines = Vec::new();
        for index in 0..700 {
            lines.push(padded_user(index * 3, 1000));
            lines.push(assistant(
                &format!("msg_{index}"),
                index * 3 + 1,
                "null",
                [1, 2, 3, 4],
            ));
            lines.push(assistant(
                &format!("msg_{index}"),
                index * 3 + 2,
                "\"end_turn\"",
                [1, 2, 3, 4],
            ));
        }
        let text = header() + &body(&lines);
        assert!(text.len() > 2 * TAIL);
        let path = fixture.file("slug", &format!("{ID}.jsonl"), &text);
        let (row, count) = passes(&fixture, &path, None);
        assert!(count >= 3, "{count}");
        assert_eq!(row.offset, text.len() as u64);
        assert_eq!(row.usage(), run(&lines).usage());
        assert_eq!(get(&row, "total_input"), json!(700 * 8));
        // Growth resumes from the checkpoint without recounting.
        let more = assistant("msg_z", 9000, "\"end_turn\"", [5, 5, 5, 5]);
        std::fs::write(&path, text.clone() + &more + "\n").unwrap();
        let (grown, count) = passes(&fixture, &path, Some(row));
        assert_eq!(count, 1);
        assert_eq!(get(&grown, "total_input"), json!(700 * 8 + 15));
    }
    #[test]
    fn replay_waits_for_partial_lines_and_restarts_on_replacement() {
        let fixture = tests::Fixture::new();
        let first = assistant("msg_a", 1, "\"end_turn\"", [1, 2, 3, 4]);
        let partial = assistant("msg_b", 2, "\"end_turn\"", [10, 0, 0, 0]);
        let text = header() + &first + "\n" + &partial;
        let path = fixture.file("slug", &format!("{ID}.jsonl"), &text);
        let deadline = Instant::now() + Duration::from_secs(5);
        let row = replay(&fixture.projects, &path, ID, None, now(), deadline).unwrap();
        assert!(!row.caught_up && row.valid);
        assert_eq!(row.offset, (header().len() + first.len() + 1) as u64);
        assert_eq!(get(&row, "total_input"), json!(8));
        std::fs::write(&path, text.clone() + "\n").unwrap();
        let (row, _) = passes(&fixture, &path, Some(row));
        assert_eq!(get(&row, "total_input"), json!(18));
        // A same-size rewrite with different content is a replacement.
        let rewritten = text
            .replace("msg_a", "msg_q")
            .replace(&counters([1, 2, 3, 4]), &counters([2, 2, 3, 4]))
            + "\n";
        assert_eq!(rewritten.len(), text.len() + 1);
        std::fs::write(&path, &rewritten).unwrap();
        let (row, _) = passes(&fixture, &path, Some(row));
        assert_eq!(get(&row, "total_input"), json!(19));
        // A tampered block is never resumed.
        let mut tampered = row.clone();
        tampered.claude.closed = vec!["not-a-hash".to_owned()];
        let (row, _) = passes(&fixture, &path, Some(tampered));
        assert_eq!(get(&row, "total_input"), json!(19));
        // A header naming another session fails before any replay.
        std::fs::write(
            &path,
            header().replace(ID, "fixture-session-b") + &first + "\n",
        )
        .unwrap();
        assert!(replay(&fixture.projects, &path, ID, Some(row), now(), deadline).is_err());
    }
    #[test]
    fn replay_deadline_and_oversized_lines_fail_closed() {
        let fixture = tests::Fixture::new();
        let first = assistant("msg_a", 1, "\"end_turn\"", [1, 2, 3, 4]);
        let path = fixture.file("slug", &format!("{ID}.jsonl"), &(header() + &first + "\n"));
        let row = replay(&fixture.projects, &path, ID, None, now(), Instant::now()).unwrap();
        assert!(!row.caught_up);
        assert_eq!(row.offset, header().len() as u64);
        assert_eq!(get(&row, "usage_seq"), Value::Null);
        // An oversized well-formed record is classified, not dropped.
        let big = padded_user(2, LINE + 10);
        let text = header() + &first + "\n" + &big + "\n";
        std::fs::write(&path, &text).unwrap();
        let (row, _) = passes(&fixture, &path, Some(row));
        assert!(row.valid && !row.skipping && row.claude.classifier.is_none());
        assert_eq!(get(&row, "total_input"), json!(8));
        // Malformed JSON in an oversized line fails closed and is skipped.
        let broken = big.replacen("\"uuid\"", "\"uuid\" \"", 1);
        let later = assistant("msg_b", 3, "\"end_turn\"", [1, 0, 0, 0]);
        std::fs::write(&path, text + &broken + "\n" + &later + "\n").unwrap();
        let (row, _) = passes(&fixture, &path, Some(row));
        assert!(!row.valid && !row.skipping && row.claude.classifier.is_none());
        assert_eq!(get(&row, "total_input"), Value::Null);
        assert_eq!(get(&row, "context"), json!(1));
    }

    /// `line` with a leading filler key of `pad` bytes.
    fn lead_pad(line: &str, pad: usize) -> String {
        line.replacen('{', &format!("{{\"pad\":\"{}\",", "p".repeat(pad)), 1)
    }
    fn content_pad(line: &str, pad: usize) -> String {
        line.replacen(
            "\"content\":[]",
            &format!("\"content\":[{{\"text\":\"{}\"}}]", "c".repeat(pad)),
            1,
        )
    }
    #[test]
    fn oversized_records_crossing_tail_are_classified_across_passes() {
        let fixture = tests::Fixture::new();
        let selected = iterations(&[("message", [1, 1, 1, 1]), ("advisor_message", [9, 9, 9, 9])]);
        let lines = vec![
            assistant("msg_a", 1, "\"end_turn\"", [1, 2, 3, 4]),
            lead_pad(&padded_user(2, LINE), TAIL),
            content_pad(
                &assistant_with(
                    "msg_b",
                    3,
                    "\"end_turn\"",
                    &(counters([10, 20, 30, 40]) + &selected),
                    "",
                ),
                TAIL + LINE,
            ),
            lead_pad(&attachment(4), 2 * LINE),
            lead_pad(&system("compact_boundary", 5), LINE),
            lead_pad(&system("microcompact_boundary", 6), LINE),
        ];
        let text = header() + &body(&lines);
        let path = fixture.file("slug", &format!("{ID}.jsonl"), &text);
        let (row, count) = passes(&fixture, &path, None);
        assert!(count >= 3, "{count}");
        assert!(row.valid && row.compactions_valid && row.turns.valid);
        assert!(!row.skipping && row.claude.classifier.is_none());
        assert_eq!(row.usage(), run(&lines).usage());
        assert_eq!(get(&row, "total_input"), json!(8 + 80));
        assert_eq!(get(&row, "context"), json!(3));
        assert_eq!(get(&row, "output_tokens"), json!(1));
        assert_eq!(get(&row, "compactions"), json!(1));
        assert_eq!(get(&row, "usage_seq"), json!(micros(3)));
    }

    #[test]
    fn oversized_unclassifiable_records_follow_the_coverage_table() {
        let fixture = tests::Fixture::new();
        let good = assistant("msg_a", 1, "\"end_turn\"", [1, 2, 3, 4]);
        let next = assistant("msg_c", 9, "\"end_turn\"", [1, 0, 0, 0]);
        // A pass boundary inside the timestamp of an oversized assistant line.
        let late = assistant("msg_b", 2, "\"end_turn\"", [5, 5, 5, 5]);
        let at = lead_pad(&late, 0).find(&stamp(2)).unwrap();
        let cut = lead_pad(&late, TAIL - good.len() - 1 - at - 5);
        let text = header() + &body(&[good.clone(), cut, next.clone()]);
        assert_eq!(text.find(&stamp(2)).unwrap() + 5, header().len() + TAIL);
        let path = fixture.file("slug", &format!("{ID}.jsonl"), &text);
        let (row, count) = passes(&fixture, &path, None);
        assert!(count >= 2);
        assert!(row.valid && row.compactions_valid && !row.turns.valid);
        assert_eq!(totals(&row), [(); 5].map(|_| Value::Null));
        // The next complete group restores last-response values only.
        assert_eq!(get(&row, "context"), json!(1));
        assert_eq!(get(&row, "compactions"), json!(0));
        let duplicated = |line: &str, key: &str| {
            lead_pad(line, LINE).replacen(
                &format!("\"{key}\""),
                &format!("\"{key}\":\"x\",\"{key}\""),
                1,
            )
        };
        let cases = [
            (
                duplicated(&system("compact_boundary", 2), "subtype"),
                [true, false],
            ),
            (duplicated(&user(2), "uuid"), [false, true]),
            (duplicated(&attachment(2), "timestamp"), [false, true]),
        ];
        for (line, [valid, compactions]) in cases {
            let lines = [good.clone(), line, next.clone()];
            std::fs::write(&path, header() + &body(&lines)).unwrap();
            let (row, _) = passes(&fixture, &path, None);
            assert_eq!([row.valid, row.compactions_valid], [valid, compactions]);
            assert!(!row.turns.valid);
            assert_eq!(get(&row, "total_input"), json!(9));
            assert_eq!(get(&row, "compactions") != Value::Null, compactions);
        }
    }

    #[test]
    fn block_requires_every_field_and_a_consistent_classifier() {
        let fixture = tests::Fixture::new();
        let big = lead_pad(&assistant("msg_a", 1, "\"end_turn\"", [1, 2, 3, 4]), TAIL);
        let path = fixture.file("slug", &format!("{ID}.jsonl"), &(header() + &big + "\n"));
        let deadline = Instant::now() + Duration::from_secs(5);
        let row = replay(&fixture.projects, &path, ID, None, now(), deadline).unwrap();
        assert!(row.skipping && row.claude.classifier.is_some() && !row.caught_up);
        let block = serde_json::to_value(&row.claude).unwrap();
        for field in block.as_object().unwrap().keys() {
            let mut missing = block.clone();
            missing.as_object_mut().unwrap().remove(field);
            assert!(
                serde_json::from_value::<ClaudeCursor>(missing).is_err(),
                "{field}"
            );
        }
        let (mut stream, head) = open_session(&fixture.projects, &path, ID).unwrap();
        let info = stream.get_ref().metadata().unwrap();
        assert!(row.resumable(&mut stream, &head, &info, now()));
        let mut stray = row.clone();
        stray.skipping = false;
        assert!(!stray.resumable(&mut stream, &head, &info, now()));
        let (row, _) = passes(&fixture, &path, Some(row));
        assert_eq!(get(&row, "total_input"), json!(8));
    }
}
