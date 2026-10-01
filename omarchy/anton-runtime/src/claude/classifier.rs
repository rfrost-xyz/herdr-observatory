//! Bounded incremental classifier for Claude Code records over 64 KiB. It
//! extracts only the consumed paths in `NODES` and converts each through the
//! same `Record::set` as the parsed path. Persisted state holds grammar state,
//! schema key names and converted fields; value bytes are buffered only in
//! memory and a capture cut by a pass boundary is lost, never saved.
use super::{ITEM, ITERATIONS_NODE, Iterations, MESSAGE, NODES, Record, USAGE};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// No consumed node: the value is skipped.
const NONE: u8 = u8::MAX;
/// Raw bytes kept for one key name or one value capture.
const CAP: usize = 1024;
const DEPTH: usize = 128;
const WORDS: [&[u8]; 3] = [b"true", b"false", b"null"];

/// How a classified record is applied to replay state.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Malformed JSON, or the record type itself could not be established.
    Invalid,
    /// Well formed, but a consumed field was lost or duplicated; carries the
    /// record kind for the D3 coverage table.
    Unclassified(u8),
    Record(Box<Record>),
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Classifier {
    /// Phase of each open container: 0 object start, 1 key after a comma,
    /// 2 colon, 3 object value, 4 after an object value, 5 array start,
    /// 6 array value after a comma, 7 after an array value.
    stack: Vec<u8>,
    /// Consumed node of each open container, or `NONE`.
    nodes: Vec<u8>,
    /// Node of the current key's value in each open object, or `NONE`.
    pending: Vec<u8>,
    /// 0 structural, 1 string, 2 number, 3 literal.
    lex: u8,
    number: u8,
    literal: u8,
    index: u8,
    key: u8,
    escape: u8,
    hex: u8,
    utf: u8,
    lo: u8,
    hi: u8,
    started: u8,
    done: u8,
    /// Raw bytes of a key being read in a consumed object: schema names only.
    name: Vec<u8>,
    /// Consumed node of the value being read, or `NONE`.
    capture: u8,
    /// Nodes already seen in this record (iteration fields per element).
    seen: u64,
    /// Nodes whose value was cut by a pass boundary, overlong or duplicated.
    lost: u64,
    message_object: bool,
    record: Record,
    iterations: Iterations,
    /// Raw bytes of the value being captured, never persisted.
    #[serde(skip)]
    value: Vec<u8>,
}
impl Default for Classifier {
    fn default() -> Self {
        Self {
            stack: vec![],
            nodes: vec![],
            pending: vec![],
            lex: 0,
            number: 0,
            literal: 0,
            index: 0,
            key: 0,
            escape: 0,
            hex: 0,
            utf: 0,
            lo: 128,
            hi: 191,
            started: 0,
            done: 0,
            name: vec![],
            capture: NONE,
            seen: 0,
            lost: 0,
            message_object: false,
            record: Record::default(),
            iterations: Iterations::default(),
            value: vec![],
        }
    }
}
impl Classifier {
    fn push(&mut self, phase: u8, node: u8) -> Result<(), ()> {
        if self.stack.len() >= DEPTH {
            return Err(());
        }
        self.stack.push(phase);
        self.nodes.push(node);
        self.pending.push(NONE);
        Ok(())
    }
    fn pop(&mut self) {
        self.stack.pop();
        self.nodes.pop();
        self.pending.pop();
    }
    /// Node of a value starting in the innermost container.
    fn target(&self) -> u8 {
        match (self.stack.last(), self.nodes.last(), self.pending.last()) {
            (Some(3), _, Some(node)) => *node,
            (Some(5 | 6), Some(&ITERATIONS_NODE), _) => ITEM,
            _ => NONE,
        }
    }
    /// A value of the innermost container is complete. Each completed element
    /// of `iterations` folds into the D4 selection.
    fn value_done(&mut self) {
        let Some(last) = self.stack.len().checked_sub(1) else {
            self.done = 1;
            return;
        };
        self.pending[last] = NONE;
        if self.stack[last] == 3 {
            self.stack[last] = 4;
        } else if [5, 6].contains(&self.stack[last]) {
            self.stack[last] = 7;
            if self.nodes[last] == ITERATIONS_NODE {
                self.iterations.push();
                self.seen &= !ITEM_FIELDS;
            }
        }
    }
    /// A consumed scalar value is complete.
    fn scalar(&mut self, node: u8, value: &Value, id: &str, time: f64) {
        match node {
            ITERATIONS_NODE => self.iterations.shape = if value.is_null() { 0 } else { 2 },
            20..=24 => self.iterations.set(node, value),
            MESSAGE | USAGE | ITEM | NONE => {}
            _ => self.record.set(node, value, id, time),
        }
    }
    /// A container value starts at `node`; returns the node of its frame.
    fn open(&mut self, node: u8, object: bool, id: &str, time: f64) -> u8 {
        match (node, object) {
            (MESSAGE, true) => {
                self.message_object = true;
                MESSAGE
            }
            (USAGE, true) | (ITEM, true) => node,
            (ITERATIONS_NODE, false) => {
                self.iterations.shape = 1;
                ITERATIONS_NODE
            }
            (ITERATIONS_NODE, true) => {
                self.iterations.shape = 2;
                NONE
            }
            (MESSAGE | USAGE | ITEM | NONE, _) => NONE,
            _ => {
                let stand_in = if object { json!({}) } else { json!([]) };
                self.scalar(node, &stand_in, id, time);
                NONE
            }
        }
    }
    /// A key of a consumed object is complete: select its node, and lose a
    /// duplicated consumed key rather than guess which value counts.
    fn key_done(&mut self) {
        let Some(last) = self.stack.len().checked_sub(1) else {
            return;
        };
        let parent = self.nodes[last];
        if parent == NONE {
            self.pending[last] = NONE;
            return;
        }
        let mut raw = Vec::with_capacity(self.name.len() + 2);
        raw.push(b'"');
        raw.extend_from_slice(&self.name);
        raw.push(b'"');
        self.name.clear();
        let name: Option<String> = serde_json::from_slice(&raw).ok();
        let node = NODES
            .iter()
            .position(|(owner, key)| *owner == parent && name.as_deref() == Some(*key))
            .map_or(NONE, |node| node as u8);
        if node != NONE {
            if self.seen & (1 << node) != 0 {
                self.lost |= 1 << node;
                self.pending[last] = NONE;
                return;
            }
            self.seen |= 1 << node;
        }
        self.pending[last] = node;
    }
    /// The captured string or number value is complete.
    fn capture_done(&mut self, id: &str, time: f64) {
        let node = std::mem::replace(&mut self.capture, NONE);
        if node == NONE {
            return;
        }
        let raw = std::mem::take(&mut self.value);
        if raw.len() > CAP {
            self.lost |= 1 << node;
            return;
        }
        let parsed = if self.lex == 1 {
            let mut quoted = Vec::with_capacity(raw.len() + 2);
            quoted.push(b'"');
            quoted.extend_from_slice(&raw);
            quoted.push(b'"');
            serde_json::from_slice::<Value>(&quoted)
        } else {
            serde_json::from_slice::<Value>(&raw)
        };
        match parsed {
            Ok(value) => self.scalar(node, &value, id, time),
            Err(_) => self.lost |= 1 << node,
        }
    }
    fn record_byte(&mut self, ch: u8) {
        if self.key != 0 {
            if self.nodes.last().is_some_and(|node| *node != NONE) && self.name.len() <= CAP {
                self.name.push(ch);
            }
        } else if self.capture != NONE && self.value.len() <= CAP {
            self.value.push(ch);
        }
    }
}
/// Seen bits of the fields of one `iterations` element.
const ITEM_FIELDS: u64 = 0b1_1111 << 20;
// `seen` and `lost` hold one bit per node.
const _: () = assert!(NODES.len() < 64);
impl Classifier {
    /// Feeds the next bytes of one record; false means malformed JSON or the
    /// nesting bound. The caller bounds work by feeding at most one line chunk
    /// between deadline checks.
    pub fn feed(&mut self, data: &[u8], id: &str, time: f64) -> bool {
        self.bytes(data, id, time).is_ok()
    }
    fn bytes(&mut self, data: &[u8], id: &str, time: f64) -> Result<(), ()> {
        let mut index = 0;
        while index < data.len() {
            let ch = data[index];
            match self.lex {
                1 => self.string_byte(ch, id, time)?,
                3 => self.literal_byte(ch, id, time)?,
                2 => {
                    if !self.number_byte(ch, id, time)? {
                        continue;
                    }
                }
                _ => self.structural(ch, id, time)?,
            }
            index += 1;
        }
        Ok(())
    }
    fn string_byte(&mut self, ch: u8, id: &str, time: f64) -> Result<(), ()> {
        if self.utf != 0 {
            if !(self.lo..=self.hi).contains(&ch) {
                return Err(());
            }
            self.utf -= 1;
            self.lo = 128;
            self.hi = 191;
        } else if self.escape == 2 {
            if !ch.is_ascii_hexdigit() {
                return Err(());
            }
            self.hex += 1;
            if self.hex == 4 {
                self.hex = 0;
                self.escape = 0;
            }
        } else if self.escape != 0 {
            if ch == b'u' {
                self.escape = 2;
                self.hex = 0;
            } else if b"\"\\/bfnrt".contains(&ch) {
                self.escape = 0;
            } else {
                return Err(());
            }
        } else if ch == b'\\' {
            self.escape = 1;
        } else if ch == b'"' {
            let last = self.stack.len().checked_sub(1).ok_or(())?;
            if self.key != 0 {
                self.key_done();
                self.stack[last] = 2;
            } else {
                self.capture_done(id, time);
                self.value_done();
            }
            self.lex = 0;
            self.key = 0;
            return Ok(());
        } else if ch < 32 {
            return Err(());
        } else if ch >= 128 {
            self.utf = match ch {
                194..=223 => 1,
                224..=239 => 2,
                240..=244 => 3,
                _ => return Err(()),
            };
            self.lo = match ch {
                224 => 160,
                240 => 144,
                _ => 128,
            };
            self.hi = match ch {
                237 => 159,
                244 => 143,
                _ => 191,
            };
        }
        self.record_byte(ch);
        Ok(())
    }
    fn literal_byte(&mut self, ch: u8, id: &str, time: f64) -> Result<(), ()> {
        let word = WORDS.get(self.literal as usize).ok_or(())?;
        if word.get(self.index as usize) != Some(&ch) {
            return Err(());
        }
        self.index += 1;
        if self.index as usize == word.len() {
            let value = [json!(true), json!(false), Value::Null][self.literal as usize].clone();
            let node = std::mem::replace(&mut self.capture, NONE);
            self.scalar(node, &value, id, time);
            self.lex = 0;
            self.value_done();
        }
        Ok(())
    }
    /// Returns false when `ch` ends the number and must be read again.
    fn number_byte(&mut self, ch: u8, id: &str, time: f64) -> Result<bool, ()> {
        let next = match (self.number, ch) {
            (0, b'0') => Some(1),
            (0, b'1'..=b'9') => Some(2),
            (stage @ (2 | 4 | 7), b'0'..=b'9') => Some(stage),
            (1 | 2, b'.') => Some(3),
            (3, b'0'..=b'9') => Some(4),
            (1 | 2 | 4, b'e' | b'E') => Some(5),
            (5, b'+' | b'-') => Some(6),
            (5 | 6, b'0'..=b'9') => Some(7),
            _ => None,
        };
        if let Some(stage) = next {
            self.number = stage;
            self.record_byte(ch);
            return Ok(true);
        }
        if ![1, 2, 4, 7].contains(&self.number) {
            return Err(());
        }
        self.capture_done(id, time);
        self.lex = 0;
        self.value_done();
        Ok(false)
    }
}
impl Classifier {
    fn structural(&mut self, ch: u8, id: &str, time: f64) -> Result<(), ()> {
        if b" \r\n\t".contains(&ch) {
            return Ok(());
        }
        if self.done != 0 {
            return Err(());
        }
        let Some(last) = self.stack.len().checked_sub(1) else {
            if self.started != 0 || ch != b'{' {
                return Err(());
            }
            self.started = 1;
            return self.push(0, 0);
        };
        let phase = self.stack[last];
        match phase {
            0 | 1 => {
                if phase == 0 && ch == b'}' {
                    self.pop();
                    self.value_done();
                } else if ch == b'"' {
                    self.lex = 1;
                    self.key = 1;
                    self.name.clear();
                } else {
                    return Err(());
                }
            }
            2 => {
                if ch != b':' {
                    return Err(());
                }
                self.stack[last] = 3;
            }
            4 | 7 => {
                if ch == if phase == 4 { b'}' } else { b']' } {
                    self.pop();
                    self.value_done();
                } else if ch == b',' {
                    self.stack[last] = if phase == 4 { 1 } else { 6 };
                } else {
                    return Err(());
                }
            }
            3 | 5 | 6 => {
                if phase == 5 && ch == b']' {
                    self.pop();
                    self.value_done();
                    return Ok(());
                }
                let node = self.target();
                match ch {
                    b'"' => {
                        self.lex = 1;
                        self.key = 0;
                        self.capture = node;
                        self.value.clear();
                    }
                    b'{' | b'[' => {
                        let child = self.open(node, ch == b'{', id, time);
                        self.push(if ch == b'{' { 0 } else { 5 }, child)?;
                    }
                    b't' | b'f' | b'n' => {
                        self.lex = 3;
                        self.literal = match ch {
                            b't' => 0,
                            b'f' => 1,
                            _ => 2,
                        };
                        self.index = 1;
                        self.capture = node;
                    }
                    b'-' | b'0'..=b'9' => {
                        self.lex = 2;
                        self.number = match ch {
                            b'-' => 0,
                            b'0' => 1,
                            _ => 2,
                        };
                        self.capture = node;
                        self.value.clear();
                        self.record_byte(ch);
                    }
                    _ => return Err(()),
                }
            }
            _ => return Err(()),
        }
        Ok(())
    }
    /// Ends a pass inside the record. A string or number capture cannot be
    /// persisted, so its field is lost; the record then classifies as unknown.
    pub fn suspend(&mut self) {
        if [1, 2].contains(&self.lex) && self.capture != NONE {
            self.lost |= 1 << self.capture;
            self.capture = NONE;
        }
        self.value.clear();
    }
    /// Classifies the complete record once its terminating newline is fed.
    pub fn finish(mut self) -> Outcome {
        if self.done == 0 || !self.stack.is_empty() || self.lex != 0 || self.lost & 0b10 != 0 {
            return Outcome::Invalid;
        }
        self.record.finish(self.message_object, &self.iterations);
        if self.record.kind != super::KIND_ASSISTANT {
            // Only record-level fields apply to other types.
            self.lost &= NODES
                .iter()
                .enumerate()
                .filter(|(node, (parent, _))| *parent == 0 && *node != MESSAGE as usize)
                .fold(0, |bits, (node, _)| bits | 1 << node);
        }
        // A mismatched identity fails the session whatever else was lost; a
        // forked record feeds nothing either way.
        let decided = self.record.identity == super::IDENTITY_MISMATCH || self.record.forked;
        if self.lost != 0 && !decided {
            return Outcome::Unclassified(self.record.kind);
        }
        Outcome::Record(Box::new(self.record))
    }
}
impl Iterations {
    fn validate(&self) -> bool {
        let safe = |v: &u64| *v <= super::SAFE;
        self.shape <= 2
            && self.kind <= 4
            && self
                .candidate
                .is_none_or(|(kind, counters)| kind <= 4 && counters.iter().flatten().all(safe))
            && self.counters.iter().flatten().all(safe)
    }
}
impl Classifier {
    /// Revalidates restored state so that feeding it can never panic or carry
    /// a capture across a checkpoint.
    pub fn validate(&self) -> bool {
        let count = NODES.len() as u8;
        let all = (1u64 << count) - 1;
        let containers = [0, MESSAGE, USAGE, ITERATIONS_NODE, ITEM, NONE];
        let depth = self.stack.len();
        let shape = depth <= DEPTH
            && self.nodes.len() == depth
            && self.pending.len() == depth
            && self.stack.iter().all(|phase| *phase <= 7)
            && self.nodes.iter().all(|node| containers.contains(node))
            && self.nodes.first().is_none_or(|node| *node == 0)
            && self
                .pending
                .iter()
                .all(|node| *node < count || *node == NONE);
        let lexical = self.lex <= 3
            && self.number <= 7
            && self.literal <= 2
            && self.index <= 5
            && self.key <= 1
            && self.escape <= 2
            && self.hex <= 3
            && self.utf <= 3
            && (128..=191).contains(&self.lo)
            && (128..=191).contains(&self.hi)
            && self.started <= 1
            && self.done <= 1
            && self.name.len() <= CAP + 1
            // Only a literal keeps its node across a pass: it holds no bytes.
            && (self.capture == NONE || self.lex == 3 && self.capture < count)
            && self.value.is_empty()
            && (self.started == 1 || depth == 0 && self.done == 0 && self.lex == 0)
            && (self.done == 0 || depth == 0 && self.lex == 0)
            && (self.lex == 0 || depth > 0)
            && (self.lex == 1 || self.escape == 0 && self.hex == 0 && self.utf == 0)
            && (self.lex == 1 || self.key == 0)
            && (self.lex != 3 || (self.index as usize) < WORDS[self.literal as usize].len());
        let record = &self.record;
        let fields = record.kind <= 5
            && record.subtype <= 4
            && record.identity <= 2
            && record.stop <= super::STOP_OTHER
            && record.stamp.is_none_or(|stamp| stamp <= super::SAFE)
            && record
                .uuid
                .as_deref()
                .is_none_or(|key| crate::common::hex_id(key, 24))
            && record
                .message
                .as_deref()
                .is_none_or(|key| crate::common::hex_id(key, 64))
            && record
                .model
                .as_deref()
                .is_none_or(crate::telemetry::safe_model)
            && record
                .usage
                .iter()
                .flatten()
                .all(|value| *value <= super::SAFE)
            && record.selected.is_none()
            && !record.compaction_iteration
            && !record.bad;
        shape
            && lexical
            && fields
            && self.seen & !all == 0
            && self.lost & !all == 0
            && self.iterations.validate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::now;

    const ID: &str = "fixture-session-a";
    const STAMP: &str = "2026-01-01T00:00:01.250Z";

    fn usage(extra: &str) -> String {
        format!(
            "{{\"input_tokens\":10,\"output_tokens\":5,\"cache_read_input_tokens\":100,\"cache_creation_input_tokens\":20{extra}}}"
        )
    }
    fn assistant(head: &str, message: &str) -> String {
        format!(
            "{{{head}\"type\":\"assistant\",\"sessionId\":\"{ID}\",\"uuid\":\"a-1\",\"timestamp\":\"{STAMP}\",\"message\":{{\"id\":\"msg_a\",\"model\":\"claude-fixture-1\",\"stop_reason\":\"end_turn\",{message}\"usage\":{}}}}}",
            usage("")
        )
    }
    /// Synthetic records covering every consumed node and value shape.
    fn lines() -> Vec<String> {
        let pad = "y".repeat(3000);
        let iterations = "\"iterations\":[{\"type\":\"message\",\"input_tokens\":1,\"output_tokens\":2,\"cache_read_input_tokens\":3,\"cache_creation_input_tokens\":4},{\"type\":\"advisor_message\",\"input_tokens\":9,\"output_tokens\":9,\"cache_read_input_tokens\":9,\"cache_creation_input_tokens\":9},7,null,{\"type\":\"compaction\"}]";
        vec![
            assistant("", ""),
            assistant(
                &format!("\"pad\":\"{pad}\\n\\u00e9\\\"\",\"isMeta\":false,"),
                &format!(
                    "\"content\":[{{\"type\":\"text\",\"text\":\"{pad} \u{e9}\u{1f600}\"}},[1.5e3,-2,true,null]],"
                ),
            ),
            assistant("", "").replace(&usage(""), &usage(&format!(",{iterations}"))),
            assistant("", "").replace(&usage(""), &usage(",\"iterations\":{}")),
            assistant("", "").replace(&usage(""), &usage(",\"iterations\":\"x\"")),
            assistant("", "").replace(&usage(""), &usage(",\"iterations\":null")),
            assistant("", "").replace("\"end_turn\"", "null"),
            assistant("", "").replace("\"cache_read_input_tokens\":100,", ""),
            assistant("", "").replace("\"input_tokens\":10", "\"input_tokens\":1e1"),
            assistant("", "").replace("claude-fixture-1", "<synthetic>"),
            assistant("\"forkedFrom\":{\"sessionId\":\"x\"},", ""),
            assistant("", "").replace(ID, "fixture-session-b"),
            assistant("", "").replace(&format!("\"sessionId\":\"{ID}\","), ""),
            format!(
                "{{\"type\":\"assistant\",\"sessionId\":\"{ID}\",\"timestamp\":\"{STAMP}\",\"message\":[{{\"id\":\"msg_a\"}}]}}"
            ),
            format!(
                "{{\"type\":\"user\",\"sessionId\":\"{ID}\",\"uuid\":\"u-1\",\"timestamp\":\"{STAMP}\",\"isMeta\":true,\"isCompactSummary\":true,\"message\":{{\"id\":\"ignored\",\"role\":\"user\",\"content\":\"{pad}\"}}}}"
            ),
            format!(
                "{{\"type\":\"attachment\",\"sessionId\":\"{ID}\",\"timestamp\":\"{STAMP}\",\"attachment\":{{\"type\":\"prompt_snapshot\",\"content\":\"{pad}\"}}}}"
            ),
            format!(
                "{{ \"type\" : \"system\" , \"subtype\" : \"compact_boundary\" , \"sessionId\" : \"{ID}\" , \"timestamp\" : \"{STAMP}\" }}"
            ),
            format!(
                "{{\"timestamp\":\"not a time\",\"subtype\":\"microcompact_boundary\",\"type\":\"system\",\"sessionId\":\"{ID}\"}}"
            ),
            format!(
                "{{\"type\":\"queue-operation\",\"sessionId\":\"{ID}\",\"timestamp\":\"{STAMP}\"}}"
            ),
            format!("{{\"type\":7,\"sessionId\":5,\"uuid\":[],\"timestamp\":{{}}}}"),
            "{}".to_owned(),
        ]
    }
    fn expected(line: &str) -> Outcome {
        let value: Value = serde_json::from_str(line).unwrap();
        Outcome::Record(Box::new(Record::from_value(&value, ID, now()).unwrap()))
    }
    fn classify(chunks: &[&[u8]], round_trip: bool) -> Outcome {
        let mut classifier = Classifier::default();
        for chunk in chunks {
            if !classifier.feed(chunk, ID, now()) {
                return Outcome::Invalid;
            }
            if round_trip {
                classifier.suspend();
                assert!(classifier.validate());
                classifier =
                    serde_json::from_value(serde_json::to_value(&classifier).unwrap()).unwrap();
            }
        }
        assert!(classifier.feed(b"\n", ID, now()));
        classifier.finish()
    }

    #[test]
    fn classifier_matches_parsed_path_for_every_chunking() {
        for line in lines() {
            let bytes = line.as_bytes();
            for size in [1, 3, 64, 1000, bytes.len()] {
                let chunks: Vec<&[u8]> = bytes.chunks(size).collect();
                assert_eq!(
                    classify(&chunks, false),
                    expected(&line),
                    "{size}: {line:.120}"
                );
            }
        }
    }
}
