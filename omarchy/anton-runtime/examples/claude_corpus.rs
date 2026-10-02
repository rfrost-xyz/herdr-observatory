//! Counts-only corpus replay. Every `<root>/<entry>/<id>.jsonl` file is bound and
//! replayed through `claude.rs`, then only aggregate counts are printed: never an
//! id, path, model name or record content. Built only by
//! `cargo run --example claude_corpus -- [projects root]`; it is never shipped.
use anton_runtime::claude::{
    self, Budget, Classifier, Discovery, IDENTITY_MISMATCH, KIND_ASSISTANT, KIND_OTHER,
    KIND_SYSTEM, KIND_USER, ORIGIN_OTHER, Outcome, Predecessor, Record, Row, SUBTYPE_TURN_DURATION,
};
use anton_runtime::common::now;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::io::BufRead;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// The parser's single-line limit; longer lines go to the classifier.
const LINE: usize = 65536;

#[derive(Default)]
struct Counts(BTreeMap<String, u64>);
impl Counts {
    fn add(&mut self, key: impl Into<String>, value: u64) {
        *self.0.entry(key.into()).or_default() += value;
    }
    fn tick(&mut self, key: impl Into<String>) {
        self.add(key, 1);
    }
}

fn main() {
    let root = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(claude::projects_root);
    let mut counts = Counts::default();
    for path in sessions(&root, &mut counts) {
        session(&root, &path, &mut counts);
    }
    for (key, value) in &counts.0 {
        println!("{key}\t{value}");
    }
}

/// Depth-two `.jsonl` regular files, sorted so repeated runs agree.
fn sessions(root: &Path, counts: &mut Counts) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(root) else {
        counts.tick("root.unreadable");
        return found;
    };
    for entry in entries.flatten() {
        let Ok(files) = std::fs::read_dir(entry.path()) else {
            continue;
        };
        for file in files.flatten() {
            let path = file.path();
            let regular = file.file_type().is_ok_and(|kind| kind.is_file());
            if regular && path.extension().is_some_and(|ext| ext == "jsonl") {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(30)
}

fn session(root: &Path, path: &Path, counts: &mut Counts) {
    counts.tick("files");
    let Some(id) = path.file_stem().and_then(|stem| stem.to_str()) else {
        counts.tick("files.unnamed");
        return;
    };
    let mut budget = Budget::new(deadline());
    let mut bound = false;
    match claude::discover(root, id, &mut budget) {
        Discovery::Found(found) if found == path => {
            counts.tick("bind.found");
            let result = claude::predecessor(path, id, &mut budget);
            match result {
                Predecessor::Clear { growing: false } => counts.tick("bind.predecessor.clear"),
                Predecessor::Clear { growing: true } => counts.tick("bind.predecessor.growing"),
                Predecessor::Unknown => counts.tick("bind.predecessor.unknown"),
                Predecessor::Truncated => counts.tick("bind.predecessor.truncated"),
            }
            bound = matches!(result, Predecessor::Clear { .. });
            successors(path, id, &result, counts);
        }
        Discovery::Found(_) => counts.tick("bind.found_other"),
        Discovery::None => counts.tick("bind.none"),
        Discovery::Ambiguous => counts.tick("bind.ambiguous"),
        Discovery::Truncated => counts.tick("bind.truncated"),
    }
    let shadow = scan(path, id, counts);
    match replay(root, path, id) {
        Ok(row) => {
            let publishable = bound && row.caught_up && row.claude.usage_seq > 0;
            counts.tick(format!(
                "publishable.{}",
                if publishable { "yes" } else { "no" }
            ));
            outcome(&row, counts);
            agreement(&row, shadow.as_ref(), counts);
        }
        Err(error) => {
            counts.tick("publishable.no");
            counts.tick(format!("identity.error.{error}"));
        }
    }
}

/// Repeated bounded passes until the row is caught up or stops advancing.
fn replay(root: &Path, path: &Path, id: &str) -> anton_runtime::Result<Row> {
    let mut previous: Option<Row> = None;
    loop {
        let before = previous.as_ref().map(|row| row.offset);
        let (row, resumed) = claude::resume(root, path, id, previous, now(), deadline())?;
        if before.is_some() && !resumed {
            return Err("replay restarted".into());
        }
        if row.caught_up || before == Some(row.offset) {
            return Ok(row);
        }
        previous = Some(row);
    }
}

/// The replay outcome: what a caught-up pass would publish, by metric.
fn outcome(row: &Row, counts: &mut Counts) {
    counts.tick("identity.ok");
    let usage = row.usage();
    let known = |key: &str| !usage[key].is_null();
    let flags = [
        ("replay.caught_up", row.caught_up),
        ("usage.known", known("usage_seq")),
        ("usage.totals_valid", known("total_input")),
        ("usage.last_response_known", known("context")),
        ("usage.model_known", known("model")),
        ("compactions.known", known("compactions")),
        (
            "compactions.iteration_seen",
            row.claude.compaction_iteration,
        ),
        ("children.valid", row.valid),
        ("turns.coverage_valid", row.turns.valid),
        ("turns.current_known", row.turns.current_known),
    ];
    for (key, value) in flags {
        counts.tick(format!("{key}.{}", if value { "yes" } else { "no" }));
    }
    counts.add("sum.compactions", row.claude.compactions);
    counts.add("sum.children", row.children.len() as u64);
    counts.add("sum.turns_finished", row.turns.finished.len() as u64);
    counts.add("sum.turn_seconds", row.turns.total);
    counts.tick(format!(
        "turns.supported.{}",
        if row.turns.supported { "yes" } else { "no" }
    ));
}

/// An independent line scan that attributes failures to categories. It reads
/// every line, including the header, and keeps only counts.
/// Returns the shadow row unless an unclassified oversized line, which the
/// public API cannot apply, made it diverge.
fn scan(path: &Path, id: &str, counts: &mut Counts) -> Option<Row> {
    let Ok(file) = std::fs::File::open(path) else {
        counts.tick("scan.unreadable");
        return None;
    };
    let mut unclassified = false;
    let mut stream = std::io::BufReader::new(file);
    let mut bytes = Vec::new();
    let time = now();
    let mut open: Option<String> = None;
    let mut closed = BTreeSet::new();
    let mut shadow = Row::new([0, 0], 0, time);
    let mut assistant = false;
    let mut recent: Vec<String> = Vec::new();
    let mut turns_lost = false;
    let mut durations = false;
    loop {
        bytes.clear();
        match stream.read_until(b'\n', &mut bytes) {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => {
                counts.tick("scan.read_error");
                break;
            }
        }
        counts.tick("lines.total");
        if bytes.last() != Some(&b'\n') {
            counts.tick("lines.unterminated");
        }
        // The replay classifies a line over `LINE`, or one `parse_line` rejects.
        let oversized = bytes.len() > LINE;
        if oversized || claude::parse_line(&bytes).is_none() {
            let class = if oversized {
                "lines.oversized"
            } else {
                "lines.fallback"
            };
            counts.tick(class);
            let mut classifier = Classifier::default();
            let outcome = if classifier.feed(&bytes, id, time) {
                classifier.finish()
            } else {
                Outcome::Invalid
            };
            let record = match outcome {
                Outcome::Record(record) => *record,
                Outcome::Unclassified(kind) => {
                    counts.tick(format!("{class}.unclassified.{kind}"));
                    unclassified = true;
                    continue;
                }
                Outcome::Invalid => {
                    counts.tick(format!("{class}.invalid"));
                    let before = flags(&shadow);
                    shadow.invalid();
                    flips(before, &shadow, "oversized_invalid", counts);
                    continue;
                }
            };
            counts.tick(format!("{class}.record"));
            let before = flags(&shadow);
            shadow.apply(&record);
            flips(before, &shadow, &category(&record), counts);
            assistant |= record.kind == KIND_ASSISTANT && !record.synthetic;
            continue;
        }
        let Some(record) =
            claude::parse_line(&bytes).and_then(|value| Record::from_value(&value, id, time))
        else {
            counts.tick("lines.invalid_json");
            let before = flags(&shadow);
            shadow.invalid();
            flips(before, &shadow, "invalid_json", counts);
            continue;
        };
        if record.kind == KIND_OTHER {
            counts.tick("records.kind_other");
        }
        assistant |= record.kind == KIND_ASSISTANT && !record.synthetic;
        attribute(&record, counts);
        let before = flags(&shadow);
        shadow.apply(&record);
        let name = category(&record);
        flips(before, &shadow, &name, counts);
        durations |= record.kind == KIND_SYSTEM && record.subtype == SUBTYPE_TURN_DURATION;
        if before[3] && !shadow.turns.valid {
            turns_lost = true;
            counts.tick(format!("unknown_context.turns.{}", recent.join(",")));
        }
        if record.kind != KIND_OTHER {
            recent.push(name);
            if recent.len() > 4 {
                recent.remove(0);
            }
        }
        if record.kind != KIND_ASSISTANT {
            continue;
        }
        let Some(message) = record.message else {
            continue;
        };
        counts.tick("dedup.assistant_lines");
        if open.as_deref() == Some(message.as_str()) {
            counts.tick("dedup.merged_lines");
            continue;
        }
        if let Some(previous) = open.replace(message.clone()) {
            closed.insert(previous);
        }
        if closed.contains(&message) {
            counts.tick("dedup.reopened_groups");
        } else {
            counts.tick("dedup.groups");
        }
    }
    if !assistant {
        counts.tick("files.without_counted_assistant_records");
    }
    if turns_lost {
        let seen = if durations { "yes" } else { "no" };
        counts.tick(format!(
            "unknown_context.turns.file_has_turn_duration.{seen}"
        ));
    }
    (!unclassified).then_some(shadow)
}

/// Whether the line-by-line shadow replay publishes what bounded passes do.
fn agreement(row: &Row, shadow: Option<&Row>, counts: &mut Counts) {
    let Some(shadow) = shadow else {
        return counts.tick("shadow.skipped_unclassified");
    };
    let same = row.usage() == shadow.usage()
        && row.children == shadow.children
        && row.turns.finished == shadow.turns.finished
        && row.turns.valid == shadow.turns.valid;
    counts.tick(if same {
        "shadow.agrees"
    } else {
        "shadow.differs"
    });
}

/// Children, compactions, totals and turn coverage validity of a shadow row.
fn flags(row: &Row) -> [bool; 4] {
    [
        row.valid,
        row.compactions_valid,
        row.claude.totals_valid,
        row.turns.valid,
    ]
}

/// Counts the record category that first made each metric unknown.
fn flips(before: [bool; 4], row: &Row, category: &str, counts: &mut Counts) {
    let names = ["children", "compactions", "totals", "turns"];
    for ((was, now), name) in before.into_iter().zip(flags(row)).zip(names) {
        if was && !now {
            counts.tick(format!("unknown_cause.{name}.{category}"));
        }
    }
}

/// A record's kind, subtype and the flags that drive replay, without content.
fn category(record: &Record) -> String {
    let kinds = [
        "other",
        "assistant",
        "user",
        "attachment",
        "system",
        "queue",
    ];
    let mut name = format!(
        "{}/{}",
        kinds.get(record.kind as usize).unwrap_or(&"?"),
        record.subtype
    );
    if record.kind == KIND_ASSISTANT {
        name.push_str(&format!("+stop{}", record.stop));
    }
    for (flag, label) in [
        (record.identity == IDENTITY_MISMATCH, "mismatch"),
        (record.bad, "bad"),
        (record.stamp.is_none(), "no_stamp"),
        (record.meta, "meta"),
        (record.tool, "tool"),
        (record.origin == ORIGIN_OTHER, "origin_other"),
        (
            record.interrupt || record.interrupted || record.aborted,
            "interrupt",
        ),
        (record.queued, "queued"),
        (record.agent_bad || record.resumed_bad, "bad_child"),
        (record.synthetic, "synthetic"),
    ] {
        if flag {
            name.push('+');
            name.push_str(label);
        }
    }
    name
}

/// Per-record categories behind an unknown metric.
fn attribute(record: &Record, counts: &mut Counts) {
    if record.identity == IDENTITY_MISMATCH {
        counts.tick("records.identity_mismatch");
    }
    if record.forked {
        counts.tick("records.forked");
    }
    if record.bad {
        counts.tick("records.bad_field");
    }
    if record.kind == KIND_ASSISTANT {
        if record.message.is_none() {
            counts.tick("records.assistant_without_message_id");
        }
        if record.stamp.is_none() {
            counts.tick("records.assistant_without_stamp");
        }
        if record.usage.iter().any(Option::is_none) {
            counts.tick("records.assistant_incomplete_usage");
        }
    }
    if record.kind == KIND_USER && record.origin == ORIGIN_OTHER {
        counts.tick("records.user_unknown_origin");
    }
    if record.agent_bad || record.resumed_bad {
        counts.tick("records.bad_child_id");
    }
}

/// Why the D1 predecessor check decided as it did: every same-directory
/// `.jsonl` file at least as new as the bound file, by successor case.
fn successors(path: &Path, id: &str, result: &Predecessor, counts: &mut Counts) {
    let prefix = match result {
        Predecessor::Clear { growing: false } => "clear",
        Predecessor::Clear { growing: true } => "growing",
        Predecessor::Unknown => "unknown",
        Predecessor::Truncated => "truncated",
    };
    let (Some(directory), Ok(own)) = (path.parent(), std::fs::metadata(path)) else {
        return counts.tick(format!("successor.{prefix}.uninspectable"));
    };
    let Ok(entries) = std::fs::read_dir(directory) else {
        return counts.tick(format!("successor.{prefix}.uninspectable"));
    };
    for entry in entries.flatten() {
        let candidate = entry.path();
        let newer = entry.metadata().is_ok_and(|info| {
            info.is_file() && (info.mtime(), info.mtime_nsec()) >= (own.mtime(), own.mtime_nsec())
        });
        if candidate == path || !newer || candidate.extension().is_none_or(|ext| ext != "jsonl") {
            continue;
        }
        let case = successor_case(&candidate, id);
        counts.tick(format!("successor.{prefix}.{case}"));
    }
}

/// The D1 case of one candidate within 256 KiB and 512 records, and for a
/// candidate that ends with no `session_id`, whether it holds any message.
fn successor_case(path: &Path, id: &str) -> &'static str {
    let Ok(file) = std::fs::File::open(path) else {
        return "unreadable";
    };
    let mut stream = std::io::BufReader::new(file);
    let mut bytes = Vec::new();
    let (mut consumed, mut messages) = (0usize, false);
    for _ in 0..512 {
        bytes.clear();
        if stream.read_until(b'\n', &mut bytes).is_err() {
            return "unreadable";
        }
        consumed += bytes.len();
        if consumed > 262144 {
            return "bound_bytes";
        }
        if bytes.last() != Some(&b'\n') {
            return if messages {
                "ended_with_messages"
            } else {
                "ended_without_messages"
            };
        }
        let Ok(record) = serde_json::from_slice::<Value>(&bytes) else {
            return "unparseable";
        };
        messages |= ["user", "assistant"].contains(&record["type"].as_str().unwrap_or(""));
        match record.get("session_id") {
            None => {}
            Some(Value::String(value)) if value == id => return "names_bound_id",
            Some(Value::String(_)) => return "names_other_id",
            Some(_) => return "malformed_session_id",
        }
    }
    "bound_records"
}
