//! Exact-session transcript following. Persistent cursors contain opaque hashes,
//! typed counters and grammar state only. Native paths stay in process memory.
use crate::{
    Result,
    claude::{self, ClaudeCursor},
    common::{self, hex_id, now, number, safe_id, sha256},
    envelope::Envelope,
    telemetry,
    turns::Turns,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::os::fd::AsRawFd;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
pub const LIMIT: usize = 262144;
const LINE: usize = 65536;
const TAIL: usize = 524288;
#[cfg(test)]
thread_local! { static TEST_ROOT:std::cell::RefCell<Option<PathBuf>>=const {std::cell::RefCell::new(None)}; }
pub fn session_root() -> PathBuf {
    #[cfg(test)]
    if let Some(root) = TEST_ROOT.with(|value| value.borrow().clone()) {
        return root;
    }
    std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| common::expand_home("~/.codex"))
        .join("sessions")
}
fn open_session(path: &Path) -> Result<File> {
    let root = session_root();
    if !path.starts_with(&root) || path == root || path.extension().is_none_or(|s| s != "jsonl") {
        return Err("Invalid session path".into());
    }
    common::open_owned(path, false, false)
}
pub(crate) fn line<R: BufRead>(stream: &mut R, limit: usize) -> std::io::Result<Vec<u8>> {
    let mut result = Vec::new();
    while result.len() < limit {
        let chunk = stream.fill_buf()?;
        if chunk.is_empty() {
            break;
        }
        let count = chunk
            .iter()
            .position(|c| *c == b'\n')
            .map(|n| n + 1)
            .unwrap_or(chunk.len())
            .min(limit - result.len());
        result.extend_from_slice(&chunk[..count]);
        stream.consume(count);
        if result.last() == Some(&b'\n') {
            break;
        }
    }
    Ok(result)
}
fn header(stream: &mut BufReader<File>, session: &str) -> Result<Vec<u8>> {
    let bytes = line(stream, LINE + 1).map_err(|_| "Cannot read session header")?;
    let value: Value = serde_json::from_slice(&bytes).map_err(|_| "Invalid session header")?;
    if bytes.len() > LINE
        || bytes.last() != Some(&b'\n')
        || value["type"] != "session_meta"
        || value["payload"]["id"] != session
    {
        return Err("Session identity mismatch".into());
    }
    Ok(bytes)
}
fn verified(path: &Path, session: &str) -> bool {
    open_session(path)
        .ok()
        .is_some_and(|file| header(&mut BufReader::new(file), session).is_ok())
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Fingerprint {
    pub(crate) size: u64,
    pub(crate) mtime_us: u64,
    pub(crate) header: String,
    pub(crate) tail: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Cursor {
    file: [u64; 2],
    offset: u64,
    children: BTreeMap<String, String>,
    seq: u64,
    valid: bool,
    compactions_valid: bool,
    #[serde(default)]
    compaction_markers: u64,
    #[serde(default)]
    compaction_summaries: u64,
    #[serde(default)]
    caught_up: bool,
    #[serde(default)]
    skipping: bool,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    envelope: Option<Envelope>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    turns: Option<Turns>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    fingerprint: Option<Fingerprint>,
    at: f64,
    /// D8: Claude parser state. Absent on Codex rows, so they serialise
    /// unchanged; required on Claude rows.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    claude: Option<ClaudeCursor>,
}
impl Cursor {
    fn validate(&self, time: f64) -> bool {
        let max = 9_007_199_254_740_991;
        if !(-1.0..86400.0).contains(&(time - self.at))
            || [
                self.file[0],
                self.file[1],
                self.offset,
                self.seq,
                self.compaction_markers,
                self.compaction_summaries,
            ]
            .iter()
            .any(|v| *v > max)
            || self.children.len() > 128
            || self.children.iter().any(|(key, status)| {
                // D6: only Claude rows produce the `unknown` status.
                let unknown = self.claude.is_some() && status == "unknown";
                !hex_id(key, 64)
                    || !unknown
                        && !["running", "completed", "interrupted", "errored"]
                            .contains(&status.as_str())
            })
            || self.turns.as_ref().is_some_and(|turns| !turns.validate())
            || self.fingerprint.as_ref().is_some_and(|f| {
                f.size > max || f.mtime_us > max || !hex_id(&f.header, 64) || !hex_id(&f.tail, 64)
            })
            || self.claude.as_ref().is_some_and(|block| {
                !block.validate(time)
                    || self.turns.is_none()
                    || self.envelope.is_some()
                    || self.compaction_markers != 0
                    || self.compaction_summaries != 0
                    || !self.skipping && block.classifier.is_some()
            })
        {
            return false;
        }
        true
    }
    fn from_row(row: claude::Row) -> Self {
        Self {
            file: row.file,
            offset: row.offset,
            children: row.children,
            seq: row.seq,
            valid: row.valid,
            compactions_valid: row.compactions_valid,
            compaction_markers: 0,
            compaction_summaries: 0,
            caught_up: row.caught_up,
            skipping: row.skipping,
            envelope: None,
            turns: Some(row.turns),
            fingerprint: row.fingerprint,
            at: row.at,
            claude: Some(row.claude),
        }
    }
    /// A Claude-keyed row without its block is never resumed (D8).
    fn into_row(self) -> Option<claude::Row> {
        Some(claude::Row {
            file: self.file,
            offset: self.offset,
            children: self.children,
            seq: self.seq,
            valid: self.valid,
            compactions_valid: self.compactions_valid,
            caught_up: self.caught_up,
            skipping: self.skipping,
            turns: self.turns?,
            fingerprint: self.fingerprint,
            at: self.at,
            claude: self.claude?,
        })
    }
    /// Whether this is a Claude row. A validated Claude row always has its block.
    pub fn is_claude(&self) -> bool {
        self.claude.is_some()
    }
    /// D8: a Claude row over the byte bound sheds its finished intervals, all
    /// but the last turn's, before it is dropped. Accumulated turn coverage
    /// becomes unknown for the rest of the binding; the byte cursor,
    /// fingerprint, block, children and current and last turn stay, so replay
    /// keeps its position. Returns whether anything was shed.
    fn shrink(&mut self) -> bool {
        let (Some(_), Some(turns)) = (&self.claude, &mut self.turns) else {
            return false;
        };
        let last: BTreeMap<_, _> = turns
            .last
            .as_ref()
            .and_then(|key| turns.finished.get_key_value(key))
            .map(|(key, interval)| (key.clone(), interval.clone()))
            .into_iter()
            .collect();
        if turns.finished.len() <= last.len() {
            return false;
        }
        turns.valid = false;
        turns.total = last.values().map(|(start, end, _)| end - start).sum();
        turns.finished = last;
        true
    }
    fn invalid(&mut self) {
        self.valid = false;
        self.compactions_valid = false;
        if let Some(turns) = &mut self.turns {
            turns.unknown();
        }
    }
    fn finish_envelope(&mut self) {
        let envelope = self.envelope.take();
        let kind = envelope.as_ref().map(Envelope::kind).unwrap_or(0);
        let payload = envelope.as_ref().map(|v| v.payload_type).unwrap_or(0);
        let item = envelope.as_ref().map(|v| v.item_type).unwrap_or(0);
        let compactions = [1, 2, 3, 4].contains(&kind) || kind == 5 && [8, 9].contains(&payload);
        let roster = [1, 2, 3, 4].contains(&kind)
            || kind == 5 && (payload == 9 || payload == 8 && [11, 12].contains(&item));
        if kind == 4 {
            self.compaction_summaries += 1;
        } else if kind == 5 && payload == 9 {
            self.compaction_markers += 1;
        }
        if !compactions {
            self.compactions_valid = false;
            if let Some(turns) = &mut self.turns {
                turns.unknown();
            }
        }
        if !roster {
            self.valid = false;
        }
    }
}
/// Validates peer or checkpoint cursor rows within 32 rows and `LIMIT` bytes.
/// Codex rows are charged against the byte budget first, in key order, and
/// Claude rows after them, so Claude rows never displace Codex rows. A Claude
/// row that would exceed the budget is shrunk before it is dropped.
pub fn validate_cursors(raw: &Value) -> BTreeMap<String, Cursor> {
    let mut result = BTreeMap::new();
    let Some(rows) = raw.as_object().filter(|v| v.len() <= 32) else {
        return result;
    };
    let time = now();
    let mut valid = vec![];
    for (key, row) in rows {
        if !hex_id(key, 64) {
            continue;
        }
        if let Ok(mut cursor) = serde_json::from_value::<Cursor>(row.clone()) {
            if !cursor.validate(time) {
                continue;
            }
            if cursor.skipping {
                cursor.caught_up = false;
                if cursor.envelope.as_ref().is_some_and(|v| !v.valid()) {
                    cursor.envelope = None;
                }
            } else {
                cursor.envelope = None;
            }
            valid.push((key, cursor));
        }
    }
    // A stable sort keeps key order within each kind.
    valid.sort_by_key(|(_, cursor)| cursor.is_claude());
    let cost = |cursor: &Cursor, key: &str| {
        serde_json::to_vec(cursor)
            .map(|v| v.len())
            .unwrap_or(LIMIT + 1)
            + key.len()
            + 8
    };
    let mut size = 0;
    for (key, mut cursor) in valid {
        let mut bytes = cost(&cursor, key);
        if size + bytes > LIMIT && cursor.shrink() {
            bytes = cost(&cursor, key);
        }
        size += bytes;
        if size > LIMIT {
            break;
        }
        result.insert(key.clone(), cursor);
    }
    result
}
fn replay(
    path: &Path,
    session: &str,
    previous: Option<Cursor>,
    time: f64,
    deadline: Instant,
) -> Result<Cursor> {
    let mut stream = BufReader::new(open_session(path)?);
    let head = header(&mut stream, session)?;
    let info = stream
        .get_ref()
        .metadata()
        .map_err(|_| "Session stat failed")?;
    let file = [info.dev(), info.ino()];
    let mtime = (info.mtime().max(0) as u64) * 1_000_000 + (info.mtime_nsec().max(0) as u64) / 1000;
    let mut valid = previous.as_ref().is_some_and(|v| {
        v.file == file
            && head.len() as u64 <= v.offset
            && v.offset <= info.len()
            && v.turns.as_ref().is_some_and(Turns::validate)
            && v.fingerprint.as_ref().is_some_and(|f| {
                f.header == sha256(&head)
                    && v.offset <= f.size
                    && f.size <= info.len()
                    && (f.size != info.len() || f.mtime_us == mtime)
            })
    });
    if valid {
        let previous = previous.as_ref().unwrap();
        stream
            .seek(SeekFrom::Start(previous.offset.saturating_sub(1024)))
            .map_err(|_| "Seek failed")?;
        let mut tail = vec![0; previous.offset.min(1024) as usize];
        stream
            .read_exact(&mut tail)
            .map_err(|_| "Tail read failed")?;
        valid = previous.fingerprint.as_ref().unwrap().tail == sha256(&tail);
    }
    let mut state = if valid {
        previous.unwrap()
    } else {
        Cursor {
            file,
            offset: head.len() as u64,
            children: BTreeMap::new(),
            seq: (time * 1e6) as u64,
            valid: true,
            compactions_valid: true,
            compaction_markers: 0,
            compaction_summaries: 0,
            caught_up: false,
            skipping: false,
            envelope: None,
            turns: Some(Turns::default()),
            fingerprint: None,
            at: time,
            claude: None,
        }
    };
    stream
        .seek(SeekFrom::Start(state.offset))
        .map_err(|_| "Session seek failed")?;
    let end = info.len().min(state.offset + TAIL as u64);
    while stream.stream_position().map_err(|_| "Seek failed")? < end && Instant::now() < deadline {
        let offset = stream.stream_position().map_err(|_| "Seek failed")?;
        let bytes = line(&mut stream, (LINE + 1).min((end - offset) as usize))
            .map_err(|_| "Session read failed")?;
        if state.skipping {
            if let Some(mut envelope) = state.envelope.clone() {
                if envelope.feed(&bytes, deadline).is_err() {
                    if Instant::now() >= deadline {
                        break;
                    }
                    state.envelope = None;
                } else {
                    state.envelope = Some(envelope);
                }
            }
            state.offset = stream.stream_position().map_err(|_| "Seek failed")?;
            state.skipping = bytes.last() != Some(&b'\n');
            if !state.skipping {
                state.finish_envelope();
            }
            continue;
        }
        if bytes.len() > LINE || bytes.last() != Some(&b'\n') {
            if bytes.len() <= LINE {
                break;
            }
            let mut envelope = Envelope::default();
            state.envelope = if envelope.feed(&bytes, deadline).is_ok() {
                Some(envelope)
            } else {
                if Instant::now() >= deadline {
                    break;
                }
                None
            };
            state.offset = stream.stream_position().map_err(|_| "Seek failed")?;
            state.skipping = bytes.last() != Some(&b'\n');
            if !state.skipping {
                state.finish_envelope();
            }
            continue;
        }
        state.offset = stream.stream_position().map_err(|_| "Seek failed")?;
        let Ok(record) = serde_json::from_slice::<Value>(&bytes) else {
            state.invalid();
            continue;
        };
        let payload = &record["payload"];
        if record["type"] == "event_msg" && payload.is_object() {
            state
                .turns
                .as_mut()
                .unwrap()
                .observe(payload, session, time);
        }
        if record["type"] == "compacted" {
            state.compaction_summaries += 1;
        }
        if record["type"] == "event_msg" && payload["type"] == "context_compacted" {
            state.compaction_markers += 1;
        }
        if record["type"] != "event_msg" || !payload.is_object() {
            continue;
        }
        let (item, stamp) = if payload["type"] == "item_completed" {
            let item = &payload["item"];
            if item["type"] != "SubAgentActivity" {
                continue;
            }
            if payload["thread_id"] != session {
                state.valid = false;
                continue;
            }
            (item, payload["completed_at_ms"].as_u64())
        } else if payload["type"] == "sub_agent_activity" {
            (payload, payload["occurred_at_ms"].as_u64())
        } else {
            continue;
        };
        let name = item["agent_path"].as_str().unwrap_or("");
        let kind = item["kind"].as_str().unwrap_or("");
        let stamp = stamp.and_then(|v| v.checked_mul(1000));
        if !name.starts_with("/root/")
            || name.len() > 512
            || name[6..].split('/').any(|s| !safe_id(s, 512))
            || ![
                "started",
                "interacted",
                "interrupted",
                "completed",
                "errored",
                "failed",
            ]
            .contains(&kind)
            || stamp.is_none_or(|v| v == 0 || v as f64 > time * 1e6)
        {
            state.valid = false;
            continue;
        }
        let key = sha256(name.as_bytes());
        if !state.children.contains_key(&key) && (kind != "started" || state.children.len() >= 128)
        {
            state.valid = false;
            continue;
        }
        state.children.insert(
            key,
            match kind {
                "started" | "interacted" => "running",
                "completed" => "completed",
                "interrupted" => "interrupted",
                _ => "errored",
            }
            .into(),
        );
        state.seq = stamp.unwrap();
    }
    state.caught_up = state.offset == info.len() && !state.skipping;
    state.at = time;
    stream
        .seek(SeekFrom::Start(state.offset.saturating_sub(1024)))
        .map_err(|_| "Session seek failed")?;
    let mut tail = vec![0; state.offset.min(1024) as usize];
    stream
        .read_exact(&mut tail)
        .map_err(|_| "Session read failed")?;
    state.fingerprint = Some(Fingerprint {
        size: info.len(),
        mtime_us: mtime,
        header: sha256(&head),
        tail: sha256(&tail),
    });
    Ok(state)
}

pub(crate) fn timestamp_us(stamp: &str) -> Option<u64> {
    // RFC3339 source timestamps only. Offset/fraction conversion is exact to the
    // microsecond; timezone-less timestamps do not become fresh measurements.
    if stamp.len() < 20 || !stamp.is_ascii() {
        return None;
    }
    let n = |range: std::ops::Range<usize>| stamp.get(range)?.parse::<i64>().ok();
    if &stamp[4..5] != "-"
        || &stamp[7..8] != "-"
        || !["T", "t", " "].contains(&&stamp[10..11])
        || &stamp[13..14] != ":"
        || &stamp[16..17] != ":"
    {
        return None;
    }
    let (mut year, month, day, hour, minute, second) = (
        n(0..4)?,
        n(5..7)?,
        n(8..10)?,
        n(11..13)?,
        n(14..16)?,
        n(17..19)?,
    );
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    if !(1..=12).contains(&month)
        || day < 1
        || day > days[(month - 1) as usize]
        || hour > 23
        || minute > 59
        || second > 59
    {
        return None;
    }
    let mut position = 19;
    let mut micros = 0u64;
    if stamp.as_bytes().get(position) == Some(&b'.') {
        position += 1;
        let start = position;
        while stamp
            .as_bytes()
            .get(position)
            .is_some_and(u8::is_ascii_digit)
        {
            position += 1;
        }
        if position == start {
            return None;
        }
        let fraction = &stamp[start..position];
        for i in 0..6 {
            micros = micros * 10
                + fraction
                    .as_bytes()
                    .get(i)
                    .map(|c| (c - b'0') as u64)
                    .unwrap_or(0);
        }
    }
    let zone = &stamp[position..];
    let offset = if zone == "Z" || zone == "z" {
        0
    } else {
        if zone.len() != 6 || !matches!(zone.as_bytes()[0], b'+' | b'-') || &zone[3..4] != ":" {
            return None;
        }
        let h = zone[1..3].parse::<i64>().ok()?;
        let m = zone[4..6].parse::<i64>().ok()?;
        if h > 23 || m > 59 {
            return None;
        }
        (h * 3600 + m * 60) * if zone.starts_with('-') { -1 } else { 1 }
    };
    year -= i64::from(month <= 2);
    let era = year.div_euclid(400);
    let yoe = year - era * 400;
    let mp = month + if month > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    let seconds = days * 86400 + hour * 3600 + minute * 60 + second - offset;
    if seconds < 0 {
        return None;
    }
    Some(seconds as u64 * 1_000_000 + micros)
}
pub fn read_usage(path: &Path, session: &str, time: f64) -> Value {
    fn inner(path: &Path, session: &str, time: f64) -> Result<Value> {
        let mut stream = BufReader::new(open_session(path)?);
        let head = header(&mut stream, session)?;
        let size = stream
            .get_ref()
            .metadata()
            .map_err(|_| "Session stat failed")?
            .len();
        let start = (head.len() as u64).max(size.saturating_sub(TAIL as u64));
        stream
            .seek(SeekFrom::Start(start))
            .map_err(|_| "Session seek failed")?;
        let mut tail = Vec::new();
        stream
            .take(TAIL as u64)
            .read_to_end(&mut tail)
            .map_err(|_| "Session read failed")?;
        let mut lines: Vec<&[u8]> = tail.split_inclusive(|v| *v == b'\n').collect();
        if start > head.len() as u64 && !lines.is_empty() {
            lines.remove(0);
        }
        for line in lines.iter().rev() {
            if line.len() > LINE || line.last() != Some(&b'\n') {
                continue;
            }
            let Ok(event) = serde_json::from_slice::<Value>(line) else {
                continue;
            };
            let payload = &event["payload"];
            if event["type"] != "event_msg" || payload["type"] != "token_count" {
                continue;
            }
            let info = &payload["info"];
            if !info.is_object() {
                continue;
            }
            let usage = &info["last_token_usage"];
            if !usage.is_object() {
                return Ok(json!({}));
            }
            let Some(stamp) = event["timestamp"].as_str().and_then(timestamp_us) else {
                continue;
            };
            if stamp as f64 > time * 1e6 {
                return Ok(json!({}));
            }
            let mut result = json!({});
            for (key, source) in [
                ("input", "input_tokens"),
                ("output_tokens", "output_tokens"),
                ("cache_read", "cached_input_tokens"),
                ("cache_write", "cache_write_input_tokens"),
                ("context", "total_tokens"),
            ] {
                result[key] = json!(number(&usage[source]));
            }
            let totals = &info["total_token_usage"];
            for (key, source) in [
                ("total_input", "input_tokens"),
                ("total_output", "output_tokens"),
                ("total_cache_read", "cached_input_tokens"),
                ("total_cache_write", "cache_write_input_tokens"),
            ] {
                result[key] = json!(number(&totals[source]));
            }
            result["total_uncached_input"] = json!(
                number(&result["total_input"])
                    .zip(number(&result["total_cache_read"]))
                    .and_then(|(i, c)| i.checked_sub(c))
            );
            result["compactions"] = Value::Null;
            if start == head.len() as u64 {
                let parsed: Option<Vec<Value>> = lines
                    .iter()
                    .map(|line| {
                        if line.len() <= LINE && line.last() == Some(&b'\n') {
                            serde_json::from_slice::<Value>(line)
                                .ok()
                                .filter(Value::is_object)
                        } else {
                            None
                        }
                    })
                    .collect();
                if let Some(parsed) = parsed {
                    let markers = parsed
                        .iter()
                        .filter(|v| {
                            v["type"] == "event_msg" && v["payload"]["type"] == "context_compacted"
                        })
                        .count();
                    let summaries = parsed.iter().filter(|v| v["type"] == "compacted").count();
                    result["compactions"] = json!(markers.max(summaries));
                }
            }
            result["window"] = json!(number(&info["model_context_window"]));
            result["context_percent"] = Value::Null;
            if let (Some(context), Some(window)) =
                (number(&result["context"]), number(&result["window"]))
            {
                result["context_percent"] = json!(if window <= 12000 {
                    100
                } else {
                    let remaining = (window - 12000).saturating_sub(context.saturating_sub(12000));
                    100 - (remaining as f64 / (window - 12000) as f64 * 100.0)
                        .clamp(0.0, 100.0)
                        .round() as u64
                });
                if window == 0 || context > window {
                    for key in ["context", "window", "context_percent"] {
                        result[key] = Value::Null;
                    }
                }
            }
            if result.as_object().unwrap().values().all(Value::is_null) {
                return Ok(json!({}));
            }
            result["usage_seq"] = json!(stamp);
            result["usage_source"] = json!("codex-rollout");
            return Ok(result);
        }
        Ok(json!({}))
    }
    inner(path, session, time).unwrap_or_else(|_| json!({}))
}

type FileSignature = (u64, u64, u64, i64, i64, i64, i64);
/// A Claude session binding (D1) and its last published numeric sample (D3).
struct Binding {
    at: Instant,
    /// Rediscover on the next pass: a truncated scan or a growing candidate.
    rescan: bool,
    path: Option<PathBuf>,
    /// The usage subset and `seq` of the last caught-up sample, re-emitted only
    /// for an incomplete replay that resumed this bound, identity-checked file.
    retained: Option<(Value, u64)>,
}
#[derive(Default)]
pub struct NativeTelemetry {
    discovery: BTreeMap<String, (Instant, Option<PathBuf>)>,
    usage: BTreeMap<String, (FileSignature, Value)>,
    claude: BTreeMap<String, Binding>,
}
fn turn_timing(turns: &Turns, time: f64) -> Value {
    json!({"active":if turns.current_known{Some(turns.active.is_some())}else{None},"started_at_s":if turns.current_known{turns.start}else{None},"observed_at_s":time,"last_duration_s":turns.last_duration,"last_outcome":turns.last_outcome,"total_finished_duration_s":if turns.valid&&turns.supported{Some(turns.total)}else{None},"complete":turns.valid&&turns.supported})
}
/// Merges a Claude replay object into the agent's metadata telemetry as Codex
/// usage is merged, adds children from a caught-up `row`, then stamps it with
/// the largest of the metadata, usage, child and replay `seq` times (D6, D8).
/// Returns whether a sample was published: none is when no time is usable.
fn publish_claude(
    agent: &mut Value,
    usage: &Value,
    row: Option<&claude::Row>,
    seq: u64,
    time: f64,
) -> bool {
    let previous = telemetry::telemetry_from_agent(agent).unwrap_or_else(|| json!({}));
    let mut value = previous.clone();
    if number(&previous["usage_seq"]).is_none()
        || number(&usage["usage_seq"]) >= number(&previous["usage_seq"])
    {
        if let Some(usage) = usage.as_object() {
            value.as_object_mut().unwrap().extend(usage.clone());
        }
    }
    if let Some(row) = row.filter(|v| v.valid) {
        let count = |status: &str| row.children.values().filter(|v| *v == status).count();
        let done = count("completed");
        value["subagent_total"] = json!(row.children.len());
        value["subagent_done"] = json!(done);
        value["subagent_status_seq"] = json!(row.status_seq());
        for (key, count) in [
            ("subagent_running", count("running")),
            ("subagent_completed", done),
            ("subagent_interrupted", count("interrupted")),
            ("subagent_failed", count("errored")),
            ("subagent_unknown", count("unknown")),
        ] {
            value[key] = json!(count);
        }
    } else {
        for key in ["subagent_total", "subagent_done", "subagent_status_seq"]
            .iter()
            .chain(telemetry::OUTCOMES)
        {
            value[*key] = Value::Null;
        }
    }
    if let Some(stamp) = [
        number(&value["seq"]),
        number(&value["usage_seq"]),
        number(&value["subagent_status_seq"]),
        Some(seq),
    ]
    .into_iter()
    .flatten()
    .filter(|v| *v > 0 && *v as f64 <= time * 1e6)
    .max()
    {
        value["seq"] = json!(stamp);
        if value.get("event").is_none() {
            value["event"] = json!("session");
            value["phase"] = json!("ready");
        }
        let mut view = telemetry::telemetry_view_at(&value, time).unwrap_or(Value::Null);
        // D4: a Claude pane has no window source, so the window and its
        // percentage are omitted rather than null; a metadata value stays.
        if let Some(view) = view.as_object_mut() {
            for key in ["window", "context_percent"] {
                if view.get(key).is_some_and(Value::is_null) {
                    view.remove(key);
                }
            }
        }
        agent["_native_telemetry"] = view;
        return true;
    }
    false
}
/// The source times of a Claude cursor row: `coverage_seq`, `usage_seq` and
/// the child `seq`.
fn incoming_stamps(cursors: &BTreeMap<String, Cursor>, key: &str) -> Option<[u64; 3]> {
    cursors.get(key).and_then(|v| {
        let block = v.claude.as_ref()?;
        Some([block.coverage_seq, block.usage_seq, v.seq])
    })
}
/// A peer follower is fresh on every probe, so a bind or verification
/// failure, a deadline skip, or a restart that publishes nothing, for a
/// session with a cursor row publishes an all-null sample at that row's
/// latest usable original source time. It replaces the copy the local
/// retains for the peer (D3). Without a cursor row the local re-emits
/// nothing. Returns false only when the row has a source time but none is
/// usable yet: the caller then withholds the row, so the local cannot
/// re-emit.
fn unknown_claude(agent: &mut Value, stamps: Option<[u64; 3]>, time: f64) -> bool {
    let Some(stamps) = stamps else {
        return true;
    };
    let usage = claude::Row::new([0, 0], 0, time).usage();
    match stamps
        .into_iter()
        .filter(|v| *v > 0 && *v as f64 <= time * 1e6)
        .max()
    {
        Some(seq) => publish_claude(agent, &usage, None, seq, time),
        None => stamps.iter().all(|v| *v == 0),
    }
}
impl NativeTelemetry {
    fn discover(session: &str, deadline: Instant) -> Option<PathBuf> {
        let mut stack = vec![(session_root(), 0)];
        let mut checked = 0;
        while let Some((directory, depth)) = stack.pop() {
            if checked >= 8192 || Instant::now() >= deadline {
                break;
            }
            let Ok(entries) = std::fs::read_dir(directory) else {
                continue;
            };
            let mut directories = vec![];
            for entry in entries.flatten() {
                checked += 1;
                if checked >= 8192 || Instant::now() >= deadline {
                    break;
                }
                let Ok(kind) = entry.file_type() else {
                    continue;
                };
                if kind.is_dir() && depth < 3 {
                    directories.push((entry.path(), depth + 1));
                } else if kind.is_file()
                    && entry
                        .file_name()
                        .to_string_lossy()
                        .ends_with(&format!("-{session}.jsonl"))
                    && verified(&entry.path(), session)
                {
                    return Some(entry.path());
                }
            }
            directories.sort();
            stack.extend(directories);
        }
        None
    }
    /// D1: positive and negative bindings are both rediscovered every 60 s. A
    /// truncated scan or a growing predecessor candidate is not cached. Any
    /// result other than the same bound path drops the retained sample. A scan
    /// truncated because the shared `deadline` passed returns `None` and
    /// changes nothing: the pane is skipped by the deadline, not unbound.
    fn bind(
        &mut self,
        root: &Path,
        session: &str,
        key: &str,
        deadline: Instant,
    ) -> Option<Option<PathBuf>> {
        if let Some(binding) = self
            .claude
            .get(key)
            .filter(|v| !v.rescan && v.at.elapsed() < Duration::from_secs(60))
        {
            return Some(binding.path.clone());
        }
        let mut budget =
            claude::Budget::new(deadline.min(Instant::now() + Duration::from_millis(100)));
        let (path, rescan) = match claude::discover(root, session, &mut budget) {
            claude::Discovery::Found(path) => {
                match claude::predecessor(&path, session, &mut budget) {
                    claude::Predecessor::Clear { growing } => (Some(path), growing),
                    claude::Predecessor::Unknown => (None, false),
                    claude::Predecessor::Truncated => (None, true),
                }
            }
            claude::Discovery::Truncated => (None, true),
            claude::Discovery::None | claude::Discovery::Ambiguous => (None, false),
        };
        if path.is_none() && rescan && Instant::now() >= deadline {
            return None;
        }
        let binding = self.claude.entry(key.to_owned()).or_insert(Binding {
            at: Instant::now(),
            rescan,
            path: None,
            retained: None,
        });
        if path.is_none() || binding.path != path {
            binding.retained = None;
        }
        binding.at = Instant::now();
        binding.rescan = rescan;
        binding.path = path.clone();
        Some(path)
    }
    /// A pane skipped by the shared deadline: a current binding with its
    /// cursor row re-emits the retained sample, as an incomplete replay does.
    /// Without a current binding, which a peer's fresh follower never has,
    /// nothing verified the file, so an incoming row takes the all-null sample
    /// or is withheld, as after a bind failure; the local never re-emits.
    fn skip_claude(
        &self,
        agent: &mut Value,
        key: &str,
        cursors: &mut BTreeMap<String, Cursor>,
        time: f64,
    ) {
        let current = self
            .claude
            .get(key)
            .filter(|v| !v.rescan && v.path.is_some() && v.at.elapsed() < Duration::from_secs(60));
        let Some(binding) = current else {
            let stamps = incoming_stamps(cursors, key);
            if stamps.is_some() && !unknown_claude(agent, stamps, time) {
                cursors.remove(key);
            }
            return;
        };
        if let Some((subset, seq)) = binding
            .retained
            .as_ref()
            .filter(|_| cursors.get(key).is_some_and(Cursor::is_claude))
        {
            publish_claude(agent, subset, None, *seq, time);
        }
    }
    /// The deadline passed after binding, before any replay pass, so the file
    /// was not opened in this call. `bind` has just made the binding current,
    /// so only a sample retained from an earlier verified pass may be
    /// re-emitted. Without one, which is always the case on a peer's fresh
    /// follower, the incoming row takes the all-null sample or is withheld
    /// (D1, D3).
    fn skip_after_bind(
        &self,
        agent: &mut Value,
        key: &str,
        cursors: &mut BTreeMap<String, Cursor>,
        time: f64,
    ) {
        if let Some((subset, seq)) = self.claude.get(key).and_then(|v| v.retained.as_ref()) {
            publish_claude(agent, subset, None, *seq, time);
            return;
        }
        let stamps = incoming_stamps(cursors, key);
        if stamps.is_some() && !unknown_claude(agent, stamps, time) {
            cursors.remove(key);
        }
    }
    /// One Claude pane: bind, replay up to 16 bounded passes, then publish the
    /// caught-up sample, or re-emit the retained one for an incomplete replay.
    fn enrich_claude(
        &mut self,
        agent: &mut Value,
        cursors: &mut BTreeMap<String, Cursor>,
        active: &mut BTreeSet<String>,
        time: f64,
        deadline: Instant,
    ) {
        if telemetry::session_binding(agent).is_none() || agent["agent_session"]["kind"] != "id" {
            return;
        }
        let Some(session) = agent["agent_session"]["value"]
            .as_str()
            .filter(|v| safe_id(v, 128))
            .map(str::to_owned)
        else {
            return;
        };
        let key = sha256(format!("anton-native-session-v1:claude:{session}").as_bytes());
        active.insert(key.clone());
        if Instant::now() >= deadline {
            self.skip_claude(agent, &key, cursors, time);
            return;
        }
        let incoming = incoming_stamps(cursors, &key);
        let unknown = |agent: &mut Value| unknown_claude(agent, incoming, time);
        let root = claude::projects_root();
        let path = match self.bind(&root, &session, &key, deadline) {
            Some(Some(path)) => path,
            Some(None) => {
                if !unknown(agent) {
                    cursors.remove(&key);
                }
                return;
            }
            // The deadline passed inside discovery or the predecessor scan.
            None => {
                self.skip_claude(agent, &key, cursors, time);
                return;
            }
        };
        let incoming_row = cursors.remove(&key);
        let mut row = incoming_row.clone().and_then(Cursor::into_row);
        let (mut ran, mut restarted, mut withhold) = (false, false, false);
        for _ in 0..16 {
            if Instant::now() >= deadline {
                break;
            }
            let offset = row.as_ref().map(|v| v.offset);
            let pass = deadline.min(Instant::now() + Duration::from_millis(600));
            let (next, resumed) =
                match claude::resume(&root, &path, &session, row.take(), time, pass) {
                    Ok(value) => value,
                    Err(failure) => {
                        // An open, ownership, header or read failure drops the
                        // retained sample, but keeps the incoming row, so the host
                        // still reports a row for this pane and a peer publishes the
                        // all-null sample at its source time (D3). Progress made by
                        // an earlier pass of this call is discarded. The binding
                        // keeps its path and time: only a failure that may be
                        // transient rediscovers at the next pass, so one that holds
                        // for the path keeps the 60 s cadence (D1).
                        let binding = self.claude.get_mut(&key).unwrap();
                        binding.retained = None;
                        binding.rescan |= failure.transient;
                        if let Some(incoming_row) = incoming_row.filter(|_| unknown(agent)) {
                            cursors.insert(key, incoming_row);
                        }
                        return;
                    }
                };
            ran = true;
            restarted |= !resumed;
            let done = next.caught_up || Some(next.offset) == offset;
            row = Some(next);
            if done {
                break;
            }
        }
        let Some(row) = row else {
            return;
        };
        if ran {
            let binding = self.claude.get_mut(&key).unwrap();
            // A record naming another session is a D2 identity failure.
            let foreign = row.claude.foreign;
            if restarted || foreign {
                binding.retained = None;
            }
            if row.caught_up && !row.skipping {
                if !foreign {
                    agent["_native_turn_timing"] = turn_timing(&row.published_turns(), time);
                }
                let usage = row.usage();
                let seq = row.claude.coverage_seq;
                let children = Some(&row).filter(|_| !foreign);
                // A restart that publishes nothing, as with no timestamped
                // record yet, must still replace the local's copy (D3).
                if !publish_claude(agent, &usage, children, seq, time) && (restarted || foreign) {
                    withhold = !unknown(agent);
                }
                let mut subset = usage;
                subset.as_object_mut().unwrap().remove("compactions");
                binding.retained = Some((subset, seq));
            } else if let Some((subset, seq)) = &binding.retained {
                publish_claude(agent, subset, None, *seq, time);
            } else if restarted || foreign {
                // An identity failure replaces the copy a local retains for
                // a peer, whose fresh follower has nothing to re-emit (D3).
                withhold = !unknown(agent);
            }
        } else {
            cursors.insert(key.clone(), Cursor::from_row(row));
            self.skip_after_bind(agent, &key, cursors, time);
            return;
        }
        if !withhold {
            cursors.insert(key, Cursor::from_row(row));
        }
    }
    /// One Codex pane: discover, read usage, replay up to 16 bounded passes,
    /// then publish usage, children, compactions and turn timing.
    fn enrich_codex(
        &mut self,
        agent: &mut Value,
        cursors: &mut BTreeMap<String, Cursor>,
        active: &mut BTreeSet<String>,
        time: f64,
        deadline: Instant,
    ) {
        if agent["agent"] != "codex"
            || telemetry::session_binding(agent).is_none()
            || agent["agent_session"]["kind"] != "id"
        {
            return;
        }
        let Some(session) = agent["agent_session"]["value"]
            .as_str()
            .filter(|v| safe_id(v, 128))
            .map(str::to_owned)
        else {
            return;
        };
        let key = sha256(format!("anton-native-session-v1:{session}").as_bytes());
        active.insert(key.clone());
        if Instant::now() >= deadline {
            return;
        }
        let cached = self.discovery.get(&key);
        let path = if cached
            .is_none_or(|(at, path)| path.is_none() && at.elapsed() >= Duration::from_secs(60))
        {
            let path = Self::discover(
                &session,
                deadline.min(Instant::now() + Duration::from_millis(100)),
            );
            self.discovery
                .insert(key.clone(), (Instant::now(), path.clone()));
            path
        } else {
            cached.and_then(|(_, path)| path.clone())
        };
        let Some(path) = path else {
            return;
        };
        let usage = if let Ok(info) = open_session(&path)
            .and_then(|file| file.metadata().map_err(|_| "Session stat failed".into()))
        {
            let signature = (
                info.dev(),
                info.ino(),
                info.len(),
                info.mtime(),
                info.mtime_nsec(),
                info.ctime(),
                info.ctime_nsec(),
            );
            let result = if let Some((old, value)) =
                self.usage.get(&key).filter(|(old, _)| *old == signature)
            {
                let _ = old;
                value.clone()
            } else {
                read_usage(&path, &session, time)
            };
            self.usage.insert(key.clone(), (signature, result.clone()));
            result
        } else {
            self.usage.remove(&key);
            self.discovery.remove(&key);
            json!({})
        };
        let mut native = cursors.remove(&key).filter(|v| v.claude.is_none());
        for _ in 0..16 {
            if Instant::now() >= deadline {
                break;
            }
            let offset = native.as_ref().map(|v| v.offset);
            native = replay(
                &path,
                &session,
                native,
                time,
                deadline.min(Instant::now() + Duration::from_millis(600)),
            )
            .ok();
            if native
                .as_ref()
                .is_none_or(|v| v.caught_up || Some(v.offset) == offset)
            {
                break;
            }
        }
        if let Some(state) = &native {
            if state.caught_up && !state.skipping {
                if let Some(turns) = &state.turns {
                    agent["_native_turn_timing"] = turn_timing(turns, time);
                }
            }
        }
        let previous = telemetry::telemetry_from_agent(agent).unwrap_or_else(|| json!({}));
        let mut value = previous.clone();
        if usage.as_object().is_some_and(|v| !v.is_empty())
            && (number(&previous["usage_seq"]).is_none()
                || number(&usage["usage_seq"]) >= number(&previous["usage_seq"]))
        {
            value
                .as_object_mut()
                .unwrap()
                .extend(usage.as_object().unwrap().clone());
        }
        if let Some(state) = native
            .as_ref()
            .filter(|v| v.valid && v.caught_up && !v.skipping)
        {
            let total = state.children.len();
            let count = |status: &str| {
                state
                    .children
                    .values()
                    .filter(|v| v.as_str() == status)
                    .count()
            };
            let done = count("completed");
            value["subagent_total"] = json!(total);
            value["subagent_done"] = json!(done);
            value["subagent_status_seq"] = json!(state.seq);
            for (key, count) in [
                ("subagent_running", count("running")),
                ("subagent_completed", done),
                ("subagent_interrupted", count("interrupted")),
                ("subagent_failed", count("errored")),
                ("subagent_unknown", 0),
            ] {
                value[key] = json!(count);
            }
        } else {
            for key in ["subagent_total", "subagent_done", "subagent_status_seq"]
                .iter()
                .chain(telemetry::OUTCOMES)
            {
                value[*key] = Value::Null;
            }
        }
        if let Some(state) = native
            .as_ref()
            .filter(|v| v.caught_up && v.compactions_valid && number(&value["usage_seq"]).is_some())
        {
            value["compactions"] = json!(state.compaction_markers.max(state.compaction_summaries));
        }
        if let Some(stamp) = ["seq", "usage_seq", "subagent_status_seq"]
            .iter()
            .filter_map(|key| number(&value[*key]))
            .filter(|v| *v > 0 && *v as f64 <= time * 1e6)
            .max()
        {
            value["seq"] = json!(stamp);
            if value.get("event").is_none() {
                value["event"] = json!("session");
                value["phase"] = json!("ready");
            }
            agent["_native_telemetry"] =
                telemetry::telemetry_view_at(&value, time).unwrap_or(Value::Null);
        }
        if let Some(state) = native {
            cursors.insert(key, state);
        }
    }
    pub fn enrich(&mut self, agents: &mut [Value], raw_cursors: &Value) -> Value {
        self.enrich_until(
            agents,
            raw_cursors,
            Instant::now() + Duration::from_millis(750),
        )
    }
    fn enrich_until(
        &mut self,
        agents: &mut [Value],
        raw_cursors: &Value,
        deadline: Instant,
    ) -> Value {
        let time = now();
        let mut cursors = validate_cursors(raw_cursors);
        let mut active = BTreeSet::new();
        // Codex panes go first, so Claude replay, which an old local makes
        // restart from the header on every peer probe, cannot use up the
        // shared deadline before them.
        for agent in agents.iter_mut().take(32) {
            if agent["agent"] != "claude" {
                self.enrich_codex(agent, &mut cursors, &mut active, time, deadline);
            }
        }
        for agent in agents.iter_mut().take(32) {
            if agent["agent"] == "claude" {
                self.enrich_claude(agent, &mut cursors, &mut active, time, deadline);
            }
        }
        cursors.retain(|key, _| active.contains(key));
        self.discovery.retain(|key, _| active.contains(key));
        self.usage.retain(|key, _| active.contains(key));
        self.claude.retain(|key, _| active.contains(key));
        let result = serde_json::to_value(cursors).unwrap_or_else(|_| json!({}));
        serde_json::to_value(validate_cursors(&result)).unwrap_or_else(|_| json!({}))
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct CheckpointRow {
    host: String,
    session: String,
    cursor: Cursor,
}
#[derive(Serialize, Deserialize)]
struct CheckpointFile {
    version: u32,
    records: Vec<CheckpointRow>,
}
/// Each holder releases the checkpoint lock explicitly (`common::Unlock`),
/// because a process spawned by another thread shares the lock's open file
/// description until it calls exec. The wait covers genuine contention.
const LOCK_WAIT: Duration = Duration::from_millis(250);
pub struct Checkpoints {
    state: PathBuf,
    owner: PathBuf,
    /// The lock file identity taken at startup, or why it could not be
    /// taken; `persist` reports that cause rather than a changed owner.
    lease: Result<(u64, u64)>,
    rows: Vec<CheckpointRow>,
    written: Value,
    loaded: bool,
    last_write: Instant,
}
fn lock(file: &File, exclusive: bool, wait: Duration) -> Result<()> {
    let end = Instant::now() + wait;
    loop {
        if unsafe {
            libc::flock(
                file.as_raw_fd(),
                (if exclusive {
                    libc::LOCK_EX
                } else {
                    libc::LOCK_SH
                }) | libc::LOCK_NB,
            )
        } == 0
        {
            return Ok(());
        }
        if Instant::now() >= end {
            return Err("Checkpoint ownership busy".into());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
impl Checkpoints {
    pub fn new(state: &Path, owner: &Path) -> Result<Self> {
        let _owner = common::owner_guard(owner)?;
        let directory = common::open_directory(state)?;
        if directory
            .metadata()
            .map_err(|_| "Cannot inspect state")?
            .uid()
            != unsafe { libc::getuid() }
        {
            return Err("Invalid state owner".into());
        }
        let lease = common::open_owned(&state.join("replay-checkpoints.lock"), true, true)
            .and_then(|file| {
                lock(&file, true, LOCK_WAIT)?;
                let _unlock = common::Unlock(&file);
                let info = file
                    .metadata()
                    .map_err(|_| "Cannot inspect checkpoint lease")?;
                if info.len() != 0 {
                    return Err("Invalid checkpoint lease".into());
                }
                Ok((info.dev(), info.ino()))
            });
        let mut cache = Self {
            state: state.to_owned(),
            owner: owner.to_owned(),
            lease,
            rows: vec![],
            written: json!([]),
            loaded: false,
            last_write: Instant::now()
                .checked_sub(Duration::from_secs(3600))
                .unwrap(),
        };
        if let Ok(bytes) = common::read_owned(&state.join("replay-checkpoints.json"), LIMIT, true) {
            // Rows are decoded one by one, so a row this build cannot read,
            // such as another build's Claude block, never discards the others.
            if let Ok(value) = serde_json::from_slice::<Value>(&bytes) {
                let records = value["records"].as_array().filter(|v| v.len() <= 32);
                if let (Some(1), Some(records)) = (value["version"].as_u64(), records) {
                    cache.loaded = true;
                    cache.rows = records
                        .iter()
                        .filter_map(|row| serde_json::from_value(row.clone()).ok())
                        .collect();
                    cache.written = cache.signature();
                    cache.rows.retain(|row| {
                        hex_id(&row.host, 64)
                            && hex_id(&row.session, 64)
                            && row.cursor.validate(now())
                            && row.cursor.turns.is_some()
                            && row.cursor.fingerprint.is_some()
                    });
                }
            }
        }
        Ok(cache)
    }
    pub fn for_host(&self, host: &str) -> Value {
        let host = sha256(format!("anton-checkpoint-host-v1:{host}").as_bytes());
        let values: BTreeMap<_, _> = self
            .rows
            .iter()
            .filter(|row| row.host == host)
            .map(|row| (row.session.clone(), row.cursor.clone()))
            .collect();
        serde_json::to_value(values).unwrap_or_else(|_| json!({}))
    }
    fn signature(&self) -> Value {
        let mut value = serde_json::to_value(&self.rows).unwrap_or_else(|_| json!([]));
        for row in value.as_array_mut().unwrap() {
            row["cursor"].as_object_mut().unwrap().remove("at");
        }
        value
    }
    /// Remove expired and no-longer-configured records once at startup. An absent
    /// cache stays absent until collection produces a useful checkpoint.
    pub fn reconcile(&mut self, cursors: &BTreeMap<String, Value>) -> Result<()> {
        if !self.loaded {
            return Ok(());
        }
        self.replace_rows(cursors);
        if self.signature() != self.written {
            self.persist(true)?;
        }
        Ok(())
    }
    pub fn update(&mut self, cursors: &BTreeMap<String, Value>, force: bool) -> Result<()> {
        self.replace_rows(cursors);
        self.persist(force)
    }
    fn replace_rows(&mut self, cursors: &BTreeMap<String, Value>) {
        self.rows.clear();
        for (host, values) in cursors {
            let host = sha256(format!("anton-checkpoint-host-v1:{host}").as_bytes());
            for (session, cursor) in validate_cursors(values) {
                if cursor.turns.is_some() && cursor.fingerprint.is_some() {
                    self.rows.push(CheckpointRow {
                        host: host.clone(),
                        session,
                        cursor,
                    });
                }
            }
        }
        self.rows
            .sort_by(|a, b| (&a.host, &a.session).cmp(&(&b.host, &b.session)));
        while self.rows.len() > 32
            || serde_json::to_vec(&CheckpointFile {
                version: 1,
                records: self.rows.clone(),
            })
            .map(|v| v.len())
            .unwrap_or(LIMIT + 1)
                > LIMIT
        {
            // The oldest Claude row goes first; Codex rows only once none is left.
            let Some(index) = self
                .rows
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    (!a.cursor.is_claude())
                        .cmp(&!b.cursor.is_claude())
                        .then(a.cursor.at.total_cmp(&b.cursor.at))
                })
                .map(|(index, _)| index)
            else {
                break;
            };
            self.rows.remove(index);
        }
    }
    fn persist(&mut self, force: bool) -> Result<()> {
        let signature = self.signature();
        if !force
            && (self.last_write.elapsed() < Duration::from_secs(10)
                || signature == self.written
                    && self.last_write.elapsed() < Duration::from_secs(3600))
        {
            return Ok(());
        }
        let lease = self.lease.clone()?;
        let _owner = common::owner_guard(&self.owner)?;
        let file = common::open_owned(&self.state.join("replay-checkpoints.lock"), true, false)?;
        lock(&file, true, LOCK_WAIT)?;
        let _unlock = common::Unlock(&file);
        let info = file.metadata().map_err(|_| "Invalid checkpoint lease")?;
        let current = std::fs::symlink_metadata(self.state.join("replay-checkpoints.lock"))
            .map_err(|_| "Checkpoint lease removed")?;
        if info.len() != 0
            || lease != (info.dev(), info.ino())
            || info.dev() != current.dev()
            || info.ino() != current.ino()
        {
            return Err("Checkpoint ownership changed".into());
        }
        let bytes = serde_json::to_vec(&CheckpointFile {
            version: 1,
            records: self.rows.clone(),
        })
        .map_err(|_| "Invalid checkpoint state")?;
        common::atomic_checkpoint_write(&self.state.join("replay-checkpoints.json"), &bytes)?;
        self.written = signature;
        self.loaded = true;
        self.last_write = Instant::now();
        Ok(())
    }
}
pub fn retire(state: &Path, owner: &Path) -> Result<()> {
    use std::io::Write;
    let mut marker = common::open_owned(owner, true, false)?;
    lock(&marker, true, Duration::from_secs(3))?;
    let mut bytes = Vec::new();
    (&mut marker)
        .take(81)
        .read_to_end(&mut bytes)
        .map_err(|_| "Cannot read owner")?;
    if ![
        b"herdr.observatory\n".as_slice(),
        b"herdr.observatory",
        b"herdr.observatory:retired\n",
        b"herdr.observatory:retired",
    ]
    .contains(&bytes.as_slice())
    {
        return Err("Invalid plugin owner".into());
    }
    let current = std::fs::symlink_metadata(owner).map_err(|_| "Owner marker removed")?;
    let opened = marker.metadata().map_err(|_| "Invalid owner")?;
    if current.dev() != opened.dev() || current.ino() != opened.ino() {
        return Err("Owner changed".into());
    }
    let mut owned = Vec::new();
    let mut lease = None;
    if state.exists() {
        let directory = common::open_directory(state)?;
        if directory.metadata().map_err(|_| "Invalid state")?.uid() != unsafe { libc::getuid() } {
            return Err("Invalid state owner".into());
        }
        if state.join("replay-checkpoints.lock").exists() {
            let file = common::open_owned(&state.join("replay-checkpoints.lock"), true, false)?;
            lock(&file, true, Duration::from_secs(3))?;
            if file
                .metadata()
                .map_err(|_| "Invalid checkpoint lease")?
                .len()
                != 0
            {
                return Err("Invalid checkpoint lease".into());
            }
            lease = Some(file);
        }
        for entry in std::fs::read_dir(state)
            .map_err(|_| "Cannot read checkpoint state")?
            .flatten()
        {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name == "replay-checkpoints.json"
                || name
                    .strip_prefix(".replay-checkpoints-")
                    .is_some_and(|suffix| hex_id(suffix, 16))
            {
                common::open_owned(&entry.path(), false, false)?;
                owned.push(entry.path());
            }
        }
    }
    for path in owned {
        std::fs::remove_file(path).map_err(|_| "Cannot remove checkpoint")?;
    }
    marker
        .seek(SeekFrom::Start(0))
        .map_err(|_| "Cannot retire owner")?;
    marker
        .write_all(b"herdr.observatory:retired\n")
        .map_err(|_| "Cannot retire owner")?;
    marker.set_len(26).map_err(|_| "Cannot retire owner")?;
    marker.sync_all().map_err(|_| "Cannot sync retirement")?;
    if lease.is_some() {
        std::fs::remove_file(state.join("replay-checkpoints.lock"))
            .map_err(|_| "Cannot remove checkpoint lease")?;
    }
    Ok(())
}

/// Opens checkpoints as a test expects them: with the lease taken, so a
/// construction-time conflict fails here under its real cause.
#[cfg(test)]
fn leased(state: &Path, owner: &Path) -> Checkpoints {
    let cache = Checkpoints::new(state, owner).unwrap();
    if let Err(error) = &cache.lease {
        panic!("checkpoint lease: {error}");
    }
    cache
}
#[cfg(test)]
mod claude_tests;
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    struct Fixture {
        root: PathBuf,
        file: PathBuf,
    }
    impl Fixture {
        fn new() -> Self {
            static SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let root = std::env::temp_dir().join(format!(
                "anton-native-{}-{}-{}",
                std::process::id(),
                now().to_bits(),
                SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            ));
            std::fs::create_dir(&root).unwrap();
            let sessions = root.join("sessions");
            std::fs::create_dir(&sessions).unwrap();
            TEST_ROOT.with(|value| *value.borrow_mut() = Some(sessions.clone()));
            Self {
                root,
                file: sessions.join("fixture-session.jsonl"),
            }
        }
        fn write(&self, lines: &[Value], session: &str) {
            let mut file = File::create(&self.file).unwrap();
            writeln!(
                file,
                "{}",
                json!({"type":"session_meta","payload":{"id":session}})
            )
            .unwrap();
            for row in lines {
                let text = if let Some(value) = row.get("json") {
                    serde_json::to_string(value).unwrap()
                } else {
                    row["text"].as_str().unwrap().replace(
                        "@PAYLOAD@",
                        &"x".repeat(row["repeat"].as_u64().unwrap_or(0) as usize),
                    )
                };
                file.write_all(text.as_bytes()).unwrap();
                if row.get("newline").and_then(Value::as_bool).unwrap_or(true) {
                    file.write_all(b"\n").unwrap();
                }
            }
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            TEST_ROOT.with(|value| *value.borrow_mut() = None);
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }
    fn projected(cursor: &Cursor) -> Value {
        let value = serde_json::to_value(cursor).unwrap();
        let mut result = json!({});
        for key in [
            "children",
            "seq",
            "valid",
            "compactions_valid",
            "compaction_markers",
            "compaction_summaries",
            "caught_up",
            "skipping",
            "turns",
        ] {
            result[key] = value[key].clone();
        }
        result
    }
    #[test]
    fn static_python_oracle_covers_usage_rosters_turns_and_oversized_records() {
        let fixture = Fixture::new();
        let data: Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/native-transcripts-parity.json"
        ))
        .unwrap();
        let time = data["now"].as_f64().unwrap();
        let session = data["session"].as_str().unwrap();
        let cases = data["cases"].as_array().unwrap();
        assert!(cases.len() >= 40);
        for case in cases {
            let name = case["name"].as_str().unwrap();
            fixture.write(case["lines"].as_array().unwrap(), session);
            let usage = read_usage(&fixture.file, session, time);
            assert_eq!(usage, case["expected_usage"], "usage: {name}");
            let mut cursor = None;
            for _ in 0..16 {
                let offset = cursor.as_ref().map(|v: &Cursor| v.offset);
                cursor = Some(
                    replay(
                        &fixture.file,
                        session,
                        cursor,
                        time,
                        Instant::now() + Duration::from_secs(2),
                    )
                    .unwrap(),
                );
                if cursor
                    .as_ref()
                    .is_some_and(|v| v.caught_up || Some(v.offset) == offset)
                {
                    break;
                }
            }
            assert_eq!(
                projected(cursor.as_ref().unwrap()),
                case["expected_cursor"],
                "cursor: {name}"
            );
        }
    }
    #[test]
    fn warm_cursor_append_replacement_and_same_size_rewrite_are_verified() {
        let fixture = Fixture::new();
        let time = now();
        let record = |kind: &str| json!({"json":{"type":"event_msg","payload":{"type":"sub_agent_activity","agent_path":"/root/worker","kind":kind,"occurred_at_ms":((time-1.0)*1000.0)as u64}}});
        fixture.write(&[record("started")], "fixture-session");
        let first = replay(
            &fixture.file,
            "fixture-session",
            None,
            time,
            Instant::now() + Duration::from_secs(2),
        )
        .unwrap();
        let warm = replay(
            &fixture.file,
            "fixture-session",
            Some(first.clone()),
            time,
            Instant::now() + Duration::from_secs(2),
        )
        .unwrap();
        assert_eq!(projected(&first), projected(&warm));
        let mut append = std::fs::OpenOptions::new()
            .append(true)
            .open(&fixture.file)
            .unwrap();
        writeln!(append, "{}", record("completed")["json"]).unwrap();
        drop(append);
        let done = replay(
            &fixture.file,
            "fixture-session",
            Some(warm),
            time,
            Instant::now() + Duration::from_secs(2),
        )
        .unwrap();
        assert_eq!(done.children.values().next().unwrap(), "completed");
        fixture.write(&[record("errored")], "fixture-session");
        let replaced = replay(
            &fixture.file,
            "fixture-session",
            Some(done),
            time,
            Instant::now() + Duration::from_secs(2),
        )
        .unwrap();
        assert!(!replaced.valid);
        assert!(replaced.children.is_empty());
        fixture.write(&[record("started")], "another-session");
        assert!(
            replay(
                &fixture.file,
                "fixture-session",
                Some(first),
                time,
                Instant::now() + Duration::from_secs(2)
            )
            .is_err()
        );
    }
    #[test]
    fn no_follow_header_partial_and_deadline_bounds() {
        let fixture = Fixture::new();
        fixture.write(&[], "fixture-session");
        let alias = fixture.file.with_file_name("alias.jsonl");
        std::os::unix::fs::symlink(&fixture.file, &alias).unwrap();
        assert!(open_session(&alias).is_err());
        assert!(
            open_session(
                &fixture
                    .root
                    .join("sessions/../sessions/fixture-session.jsonl")
            )
            .is_err()
        );
        let mut append = std::fs::OpenOptions::new()
            .append(true)
            .open(&fixture.file)
            .unwrap();
        append.write_all(b"{\"type\":\"event_msg\"").unwrap();
        drop(append);
        let partial = replay(
            &fixture.file,
            "fixture-session",
            None,
            now(),
            Instant::now() + Duration::from_secs(2),
        )
        .unwrap();
        assert!(!partial.caught_up);
        let offset = partial.offset;
        let deadline = replay(
            &fixture.file,
            "fixture-session",
            Some(partial),
            now(),
            Instant::now(),
        )
        .unwrap();
        assert_eq!(deadline.offset, offset);
    }
    #[test]
    fn checkpoint_roundtrip_clock_throttle_expiry_and_retirement() {
        let fixture = Fixture::new();
        fixture.write(&[], "fixture-session");
        let owner = fixture.root.join(".herdr-observatory-install");
        std::fs::write(&owner, b"herdr.observatory\n").unwrap();
        let state = fixture.root.join("state");
        std::fs::create_dir(&state).unwrap();
        let cursor = replay(
            &fixture.file,
            "fixture-session",
            None,
            now(),
            Instant::now() + Duration::from_secs(2),
        )
        .unwrap();
        let session = sha256(b"session");
        let mut values = BTreeMap::from([("test".to_owned(), json!({session.clone():cursor}))]);
        let mut cache = leased(&state, &owner);
        cache.update(&values, true).unwrap();
        let saved = std::fs::read(state.join("replay-checkpoints.json")).unwrap();
        assert!(!String::from_utf8_lossy(&saved).contains(fixture.file.to_str().unwrap()));
        let loaded = leased(&state, &owner);
        assert_eq!(loaded.for_host("test"), values["test"]);
        values.get_mut("test").unwrap()[&session]["at"] = json!(now());
        cache.update(&values, false).unwrap();
        assert_eq!(
            std::fs::read(state.join("replay-checkpoints.json")).unwrap(),
            saved
        );
        retire(&state, &owner).unwrap();
        assert!(cache.update(&values, true).is_err());
        assert!(!state.join("replay-checkpoints.json").exists());
        assert!(Checkpoints::new(&state, &owner).is_err());
        values.get_mut("test").unwrap()[&session]["at"] = json!(now() - 86401.0);
        assert!(validate_cursors(&values["test"]).is_empty());
    }
    #[test]
    fn checkpoint_rows_load_individually_so_one_unreadable_row_keeps_the_rest() {
        let fixture = Fixture::new();
        fixture.write(&[], "fixture-session");
        let owner = fixture.root.join(".herdr-observatory-install");
        std::fs::write(&owner, b"herdr.observatory\n").unwrap();
        let state = fixture.root.join("state");
        std::fs::create_dir(&state).unwrap();
        let path = state.join("replay-checkpoints.json");
        let cursor = replay(
            &fixture.file,
            "fixture-session",
            None,
            now(),
            Instant::now() + Duration::from_secs(2),
        )
        .unwrap();
        let session = sha256(b"session");
        let values = BTreeMap::from([("kept".to_owned(), json!({session.clone():cursor}))]);
        let mut cache = leased(&state, &owner);
        cache.update(&values, true).unwrap();
        let mut file: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        let mut unreadable = file["records"][0].clone();
        unreadable["session"] = json!(sha256(b"other"));
        unreadable["cursor"]["claude"] = json!({"field_from_another_build": 1});
        file["records"].as_array_mut().unwrap().push(unreadable);
        common::atomic_checkpoint_write(&path, &serde_json::to_vec(&file).unwrap()).unwrap();
        let loaded = leased(&state, &owner);
        let hydrated = loaded.for_host("kept");
        assert!(hydrated.get(&session).is_some());
        assert!(hydrated.get(sha256(b"other")).is_none());
    }
    #[test]
    fn checkpoint_startup_reconciles_expired_and_removed_hosts_without_empty_creation() {
        let fixture = Fixture::new();
        fixture.write(&[], "fixture-session");
        let owner = fixture.root.join(".herdr-observatory-install");
        std::fs::write(&owner, b"herdr.observatory\n").unwrap();
        let state = fixture.root.join("state");
        std::fs::create_dir(&state).unwrap();
        let path = state.join("replay-checkpoints.json");
        let empty = BTreeMap::from([("kept".to_owned(), json!({}))]);
        let mut cache = leased(&state, &owner);
        cache.reconcile(&empty).unwrap();
        assert!(!path.exists(), "startup must not create an empty cache");
        let cursor = replay(
            &fixture.file,
            "fixture-session",
            None,
            now(),
            Instant::now() + Duration::from_secs(2),
        )
        .unwrap();
        let session = sha256(b"session");
        let values = BTreeMap::from([
            ("kept".to_owned(), json!({session.clone():cursor})),
            ("removed".to_owned(), json!({session.clone():cursor})),
        ]);
        cache.update(&values, true).unwrap();
        let mut loaded = leased(&state, &owner);
        let configured = BTreeMap::from([("kept".to_owned(), loaded.for_host("kept"))]);
        loaded.reconcile(&configured).unwrap();
        let kept = std::fs::read(&path).unwrap();
        let saved: CheckpointFile = serde_json::from_slice(&kept).unwrap();
        assert_eq!(saved.records.len(), 1);
        assert_eq!(
            saved.records[0].host,
            sha256(b"anton-checkpoint-host-v1:kept")
        );
        loaded.reconcile(&configured).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), kept);
        // An expired-only file must be reconciled even though hydration is empty.
        let mut expired: Value = serde_json::from_slice(&kept).unwrap();
        expired["records"][0]["cursor"]["at"] = json!(now() - 86401.0);
        common::atomic_checkpoint_write(&path, &serde_json::to_vec(&expired).unwrap()).unwrap();
        let mut loaded = leased(&state, &owner);
        assert_eq!(loaded.for_host("kept"), json!({}));
        loaded.reconcile(&empty).unwrap();
        let saved: CheckpointFile = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert!(saved.records.is_empty());
        // A removed-host-only file also needs clearing with no live cursors.
        cache
            .update(
                &BTreeMap::from([("removed".to_owned(), values["removed"].clone())]),
                true,
            )
            .unwrap();
        let mut loaded = leased(&state, &owner);
        loaded.reconcile(&empty).unwrap();
        let saved: CheckpointFile = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert!(saved.records.is_empty());
    }
    /// A sibling thread spawning processes until dropped. Each child holds a
    /// copy of every open file description until it calls exec, including
    /// the checkpoint lock's.
    struct Spawner(
        std::sync::Arc<std::sync::atomic::AtomicBool>,
        Option<std::thread::JoinHandle<()>>,
    );
    impl Spawner {
        fn new() -> Self {
            let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let flag = stop.clone();
            Self(
                stop,
                Some(std::thread::spawn(move || {
                    while !flag.load(std::sync::atomic::Ordering::Relaxed) {
                        let _ = std::process::Command::new("true").status();
                    }
                })),
            )
        }
    }
    impl Drop for Spawner {
        fn drop(&mut self) {
            self.0.store(true, std::sync::atomic::Ordering::Relaxed);
            if let Some(join) = self.1.take() {
                let _ = join.join();
            }
        }
    }
    /// Both checkpoint lock sites release the lock before closing it, so a
    /// zero-wait lock taken straight after `new` or `persist` never finds it
    /// held by a child that a sibling thread spawned in between.
    #[test]
    fn checkpoint_lock_is_free_at_once_after_each_holder_while_siblings_spawn() {
        let fixture = Fixture::new();
        let owner = fixture.root.join(".herdr-observatory-install");
        std::fs::write(&owner, b"herdr.observatory\n").unwrap();
        let state = fixture.root.join("state");
        std::fs::create_dir(&state).unwrap();
        let path = state.join("replay-checkpoints.lock");
        let _spawner = Spawner::new();
        let empty = BTreeMap::new();
        let mut held = 0;
        let free = |held: &mut usize| {
            let file = common::open_owned(&path, true, false).unwrap();
            if lock(&file, true, Duration::ZERO).is_ok() {
                let _unlock = common::Unlock(&file);
            } else {
                *held += 1;
            }
        };
        for _ in 0..300 {
            let mut cache = leased(&state, &owner);
            free(&mut held);
            cache.update(&empty, true).unwrap();
            free(&mut held);
        }
        assert_eq!(held, 0);
    }
    /// A lease that `new` could not take is reported by `persist` under its
    /// real cause, not as a changed owner.
    #[test]
    fn checkpoint_lease_failure_at_startup_is_reported_by_persist() {
        let fixture = Fixture::new();
        let owner = fixture.root.join(".herdr-observatory-install");
        std::fs::write(&owner, b"herdr.observatory\n").unwrap();
        let state = fixture.root.join("state");
        std::fs::create_dir(&state).unwrap();
        let holder =
            common::open_owned(&state.join("replay-checkpoints.lock"), true, true).unwrap();
        lock(&holder, true, Duration::ZERO).unwrap();
        let unlock = common::Unlock(&holder);
        let mut cache = Checkpoints::new(&state, &owner).unwrap();
        drop(unlock);
        drop(holder);
        assert_eq!(cache.lease, Err("Checkpoint ownership busy".to_owned()));
        assert_eq!(
            cache.update(&BTreeMap::new(), true),
            Err("Checkpoint ownership busy".to_owned())
        );
        assert!(!state.join("replay-checkpoints.json").exists());
        assert!(
            leased(&state, &owner)
                .update(&BTreeMap::new(), true)
                .is_ok()
        );
    }
    /// Each lease and write releases the lock before closing it, so repeated
    /// cycles never find it held by a child a sibling thread spawned.
    #[test]
    fn checkpoint_cycles_release_the_lock_while_siblings_spawn() {
        let fixture = Fixture::new();
        let owner = fixture.root.join(".herdr-observatory-install");
        std::fs::write(&owner, b"herdr.observatory\n").unwrap();
        let state = fixture.root.join("state");
        std::fs::create_dir(&state).unwrap();
        let _spawner = Spawner::new();
        let empty = BTreeMap::new();
        let mut failures = 0;
        for _ in 0..300 {
            let result = Checkpoints::new(&state, &owner).and_then(|mut cache| {
                cache.update(&empty, true)?;
                cache.reconcile(&empty)
            });
            failures += usize::from(result.is_err());
        }
        assert_eq!(failures, 0);
    }
    /// Genuine contention: another holder keeps the checkpoint lock for about
    /// 100 ms, then releases it. The lease and the write each wait it out
    /// within `LOCK_WAIT`; with no wait, `new` would record a busy lease.
    #[test]
    fn checkpoint_lease_and_write_wait_out_a_brief_holder() {
        let fixture = Fixture::new();
        let owner = fixture.root.join(".herdr-observatory-install");
        std::fs::write(&owner, b"herdr.observatory\n").unwrap();
        let state = fixture.root.join("state");
        std::fs::create_dir(&state).unwrap();
        let path = state.join("replay-checkpoints.lock");
        // Holds the lock on its own open file description, so it conflicts
        // with this thread's, until about 100 ms after it is taken.
        let hold = || {
            let (ready, taken) = std::sync::mpsc::channel();
            let shared = path.clone();
            let holder = std::thread::spawn(move || {
                let file = common::open_owned(&shared, true, true).unwrap();
                lock(&file, true, Duration::ZERO).unwrap();
                let _unlock = common::Unlock(&file);
                ready.send(()).unwrap();
                std::thread::sleep(Duration::from_millis(100));
            });
            taken.recv().unwrap();
            let probe = common::open_owned(&path, true, false).unwrap();
            let _unlock = common::Unlock(&probe);
            assert!(lock(&probe, true, Duration::ZERO).is_err());
            holder
        };
        let holder = hold();
        let started = Instant::now();
        let mut cache = leased(&state, &owner);
        assert!(started.elapsed() >= Duration::from_millis(50));
        holder.join().unwrap();
        let holder = hold();
        assert!(cache.update(&BTreeMap::new(), true).is_ok());
        holder.join().unwrap();
        assert!(state.join("replay-checkpoints.json").exists());
    }
    /// Clears the per-run cursor fields so a golden comparison is stable.
    fn normalised(cursors: &Value) -> Value {
        let mut value = cursors.clone();
        for cursor in value.as_object_mut().unwrap().values_mut() {
            cursor["at"] = json!(0);
            cursor["file"] = json!([0, 0]);
            if cursor["fingerprint"].is_object() {
                cursor["fingerprint"]["mtime_us"] = json!(0);
            }
        }
        value
    }
    fn codex_agent(session: &str) -> Value {
        json!({"agent":"codex","agent_session":{"agent":"codex","source":"herdr:codex","kind":"id","value":session}})
    }
    /// Codex-only enrichment, cursors and checkpoint rows as produced before
    /// Claude dispatch existed (`a2b3363`). Any change here changes Codex output.
    #[test]
    fn codex_only_enrich_cursor_and_checkpoint_are_unchanged() {
        let fixture = Fixture::new();
        let child = |kind: &str, ms: u64| json!({"json":{"type":"event_msg","payload":{"type":"sub_agent_activity","agent_path":"/root/worker","kind":kind,"occurred_at_ms":ms}}});
        let usage = json!({"json":{"type":"event_msg","timestamp":"2026-01-01T00:00:30+00:00","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":12,"output_tokens":3,"cached_input_tokens":0,"cache_write_input_tokens":4,"total_tokens":185000},"total_token_usage":{"input_tokens":21700000,"output_tokens":4200,"cached_input_tokens":20000000,"cache_write_input_tokens":0},"model_context_window":258400}}}});
        fixture.write(
            &[
                json!({"json":{"type":"event_msg","payload":{"type":"task_started","turn_id":"turn-a","started_at":1767225601}}}),
                child("started", 1_767_225_602_000),
                child("interrupted", 1_767_225_603_000),
                json!({"json":{"type":"compacted","payload":{}}}),
                json!({"json":{"type":"event_msg","payload":{"type":"context_compacted"}}}),
                usage,
                json!({"json":{"type":"event_msg","payload":{"type":"task_complete","turn_id":"turn-a","started_at":1767225601,"completed_at":1767225620}}}),
                child("interacted", 1_767_225_621_000),
                child("completed", 1_767_225_625_000),
            ],
            "session",
        );
        let mut follower = NativeTelemetry::default();
        let mut agents = vec![codex_agent("session"), codex_agent("absent")];
        let cursors = follower.enrich(&mut agents, &json!({}));
        let mut timing = agents[0]["_native_turn_timing"].clone();
        timing["observed_at_s"] = json!(0);
        assert_eq!(
            json!({"telemetry":agents[0]["_native_telemetry"],"timing":timing,"absent":agents[1],"cursors":normalised(&cursors)}),
            json!({
                "absent": codex_agent("absent"),
                "cursors": {"384ed787d76c0bbd72bc0b86f6a2ea605455237463b18f5ef656c40cd66dbe63": {
                    "at": 0, "caught_up": true,
                    "children": {"e75c94502a7fbb74b08bc4ffc5219f31d5d16266272e870171adc0310a3e01f7": "completed"},
                    "compaction_markers": 1, "compaction_summaries": 1, "compactions_valid": true,
                    "file": [0, 0],
                    "fingerprint": {"header": "b35d3bda40a5e3ad26bf99af603b7a02d858f8ca2b6a2a1f96fc1879d77f627c", "mtime_us": 0, "size": 1317, "tail": "850b6772e93a8ef47b0c76a3da0a96bebdeb489af8bdda25e0ece15537eaa6a8"},
                    "offset": 1317, "seq": 1_767_225_625_000_000_u64, "skipping": false,
                    "turns": {"active": null, "current_known": true, "finished": {"4ea1a6a42fdcde0801691c1a": [1767225601, 1767225620, "completed"]}, "last": "4ea1a6a42fdcde0801691c1a", "last_duration": 19, "last_end": 1767225620, "last_outcome": "completed", "start": null, "supported": true, "total": 19, "valid": true},
                    "valid": true
                }},
                "telemetry": {
                    "cache_read": 0, "cache_write": 4, "compactions": 1, "context": 185000, "context_percent": 70,
                    "event": "session", "input": 12, "model": null, "output_tokens": 3, "phase": "ready", "result": null,
                    "seq": 1_767_225_630_000_000_u64, "subagent_completed": 1, "subagent_done": 1, "subagent_failed": 0,
                    "subagent_interrupted": 0, "subagent_running": 0, "subagent_seq": null, "subagent_starts": null,
                    "subagent_status_seq": 1_767_225_625_000_000_u64, "subagent_stops": null, "subagent_total": 1,
                    "subagent_unknown": 0, "tool": null, "total_cache_read": 20000000, "total_cache_write": 0,
                    "total_input": 21700000, "total_output": 4200, "total_uncached_input": 1700000,
                    "usage_seq": 1_767_225_630_000_000_u64, "usage_source": "codex-rollout", "window": 258400
                },
                "timing": {"active": false, "complete": true, "last_duration_s": 19, "last_outcome": "completed", "observed_at_s": 0, "started_at_s": null, "total_finished_duration_s": 19}
            })
        );
        let owner = fixture.root.join(".herdr-observatory-install");
        std::fs::write(&owner, b"herdr.observatory\n").unwrap();
        let state = fixture.root.join("state");
        std::fs::create_dir(&state).unwrap();
        let mut cache = leased(&state, &owner);
        cache
            .update(
                &BTreeMap::from([("test".to_owned(), cursors.clone())]),
                true,
            )
            .unwrap();
        let saved = std::fs::read_to_string(state.join("replay-checkpoints.json")).unwrap();
        assert!(!saved.contains("claude"));
        let file: CheckpointFile = serde_json::from_str(&saved).unwrap();
        let (key, row) = cursors.as_object().unwrap().iter().next().unwrap();
        assert_eq!(file.records.len(), 1);
        assert_eq!(&file.records[0].session, key);
        assert_eq!(&serde_json::to_value(&file.records[0].cursor).unwrap(), row);
        let mut cursor = file.records[0].cursor.clone();
        cursor.at = 0.0;
        cursor.file = [0, 0];
        cursor.fingerprint.as_mut().unwrap().mtime_us = 0;
        assert_eq!(
            serde_json::to_string(&cursor).unwrap(),
            concat!(
                r#"{"file":[0,0],"offset":1317,"children":{"e75c94502a7fbb74b08bc4ffc5219f31d5d16266272e870171adc0310a3e01f7":"completed"},"seq":1767225625000000,"valid":true,"compactions_valid":true,"compaction_markers":1,"compaction_summaries":1,"caught_up":true,"skipping":false,"turns":{"valid":true,"supported":tru"#,
                r#"e,"current_known":true,"active":null,"start":null,"last":"4ea1a6a42fdcde0801691c1a","last_duration":19,"last_outcome":"completed","last_end":1767225620,"total":19,"finished":{"4ea1a6a42fdcde0801691c1a":[1767225601,1767225620,"completed"]}},"fingerprint":{"size":1317,"mtime_us":0,"header":"b35d3bda40a5e3ad26bf99af603b7a02d858f8ca2b6a2a1f96fc1879d77f627c","tail":"850b6772e93a8ef47b0c76a3da0a96bebdeb489af8bdda25e0ece15537eaa6a8"},"at":0.0}"#
            )
        );
        let again = follower.enrich(&mut agents, &cursors);
        assert_eq!(normalised(&again), normalised(&cursors));
    }
}
