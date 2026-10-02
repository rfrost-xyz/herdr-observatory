//! Bounded incremental classifier for Claude Code records over 64 KiB. It
//! extracts only the consumed paths in `NODES` and converts each through the
//! same `Record::set` as the parsed path. Persisted state holds grammar state,
//! prefixes of schema key names and converted fields; value bytes are
//! buffered only in memory and a capture cut by a pass boundary is lost,
//! never saved.
use super::text::{LIMIT, Text, WIDE};
use super::{
    ATTACHMENT, BLOCK, Blocks, COMPACT_SUMMARY, CONTENT, FORKED, FRAMES, INTERRUPTED, ITEM,
    ITERATIONS_NODE, Iterations, MESSAGE, NODES, ORIGIN, PROMPT, Record, TEXT_NODES, TOOL, USAGE,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// No consumed node: the value is skipped.
const NONE: u8 = u8::MAX;
/// Raw bytes kept for one value capture.
const CAP: usize = 1024;
/// Raw bytes kept for one key name: one more than the longest consumed key
/// written entirely as `\uXXXX` escapes, so a longer name cannot match.
const NAME: usize = {
    let mut longest = 0;
    let mut index = 0;
    while index < NODES.len() {
        if NODES[index].1.len() > longest {
            longest = NODES[index].1.len();
        }
        index += 1;
    }
    6 * longest + 1
};
const DEPTH: usize = 128;
/// Containers counted, unread, past `DEPTH` (D3).
const DEEP: u32 = 1 << 20;
/// Nodes whose non-null, non-false value only signals presence, so a string
/// of any length is recorded at its start and never buffered.
const PRESENCE: [u8; 5] = [FORKED, COMPACT_SUMMARY, ORIGIN, TOOL, INTERRUPTED];
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
    /// Raw bytes of a key being read in a consumed object, kept only while
    /// they can still spell a consumed key at that parent (`viable`).
    name: Vec<u8>,
    /// The key being read can no longer match: its bytes were dropped.
    miss: bool,
    /// Consumed node of the value being read, or `NONE`.
    capture: u8,
    /// Nodes already seen in this record (iteration fields per element).
    seen: u64,
    /// Nodes whose value was cut by a pass boundary, overlong or duplicated.
    lost: u64,
    /// Open containers past `DEPTH`, which hold no consumed node: only their
    /// count and their strings are checked, never their bracket kinds or
    /// separators.
    deep: u32,
    /// The record nested past `DEPTH`, so it is unclassified (D3).
    overflow: bool,
    message_object: bool,
    record: Record,
    iterations: Iterations,
    blocks: Blocks,
    /// Raw bytes of the value being captured, never persisted.
    #[serde(skip)]
    value: Vec<u8>,
    /// Text units of the text value being analysed, never persisted.
    #[serde(skip)]
    scan: Vec<u8>,
    /// The `\uXXXX` escape being decoded in a text value.
    #[serde(skip)]
    code: u16,
    /// The last unit of this string was a high surrogate escape, so a low
    /// one next completes its pair. A pass boundary loses the text anyway.
    #[serde(skip)]
    high: bool,
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
            miss: false,
            capture: NONE,
            seen: 0,
            lost: 0,
            deep: 0,
            overflow: false,
            message_object: false,
            record: Record::default(),
            iterations: Iterations::default(),
            blocks: Blocks::default(),
            value: vec![],
            scan: vec![],
            code: 0,
            high: false,
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
            (Some(5 | 6), Some(&(CONTENT | PROMPT)), _) => BLOCK,
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
            } else if [CONTENT, PROMPT].contains(&self.nodes[last]) {
                self.blocks.push(self.nodes[last]);
                self.seen &= !BLOCK_FIELDS;
            }
        }
    }
    /// A consumed scalar value is complete.
    fn scalar(&mut self, node: u8, value: &Value, id: &str, time: f64) {
        match node {
            ITERATIONS_NODE => self.iterations.shape = if value.is_null() { 0 } else { 2 },
            20..=24 => self.iterations.set(node, value),
            PROMPT | CONTENT | 41 | super::BLOCK_TEXT => self.blocks.set(node, value),
            NONE => {}
            _ if FRAMES.contains(&node) => {}
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
            (USAGE | ITEM | ATTACHMENT | BLOCK, true) | (CONTENT | PROMPT, false) => node,
            (ORIGIN | TOOL, true) => {
                self.scalar(node, &json!({}), id, time);
                node
            }
            (ITERATIONS_NODE, false) => {
                self.iterations.shape = 1;
                ITERATIONS_NODE
            }
            (ITERATIONS_NODE, true) => {
                self.iterations.shape = 2;
                NONE
            }
            (NONE, _) => NONE,
            _ if FRAMES.contains(&node) => NONE,
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
        if std::mem::take(&mut self.miss) {
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
            .position(|(owner, key)| {
                !key.is_empty() && *owner == parent && name.as_deref() == Some(*key)
            })
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
        if TEXT_NODES.contains(&node) {
            // Only a string carries text; a number at a text node is ignored.
            if self.lex == 1 {
                self.text_done(node);
            }
            self.scan.clear();
            self.value.clear();
            return;
        }
        let raw = std::mem::take(&mut self.value);
        if raw.len() > CAP {
            // A `sessionId` this long, or a number, can never equal a safe
            // id, so it is a mismatch, as the parsed path reads it.
            if node == 3 {
                self.record.identity = super::IDENTITY_MISMATCH;
            } else {
                self.lost |= 1 << node;
            }
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
            let parent = self.nodes.last().copied().unwrap_or(NONE);
            if parent != NONE && !self.miss {
                self.name.push(ch);
                if !viable(parent, &self.name) {
                    // No raw bytes of a key outside the schema are kept.
                    self.name.clear();
                    self.miss = true;
                }
            }
        } else if self.capture != NONE
            && !TEXT_NODES.contains(&self.capture)
            && self.value.len() <= CAP
        {
            self.value.push(ch);
        }
    }
}
/// Whether `raw`, the bytes of a key read so far, can still spell a consumed
/// key at `parent`, so that only schema prefixes are ever persisted.
fn viable(parent: u8, raw: &[u8]) -> bool {
    NODES
        .iter()
        .any(|(owner, key)| *owner == parent && !key.is_empty() && spells(key.as_bytes(), raw))
}
/// Plain bytes and complete `\uXXXX` escapes must decode to a prefix of
/// `key`; a trailing partial escape must agree with its next character.
fn spells(key: &[u8], raw: &[u8]) -> bool {
    let (mut at, mut index) = (0, 0);
    while index < raw.len() {
        let Some(&want) = key.get(at) else {
            return false;
        };
        if raw[index] != b'\\' {
            if raw[index] != want {
                return false;
            }
            index += 1;
        } else {
            let escape = format!("u{want:04x}");
            let rest = &raw[index + 1..];
            let length = rest.len().min(5);
            if rest.first().is_some_and(|c| *c != b'u')
                || !rest[..length].eq_ignore_ascii_case(&escape.as_bytes()[..length])
            {
                return false;
            }
            index += 1 + length;
        }
        at += 1;
    }
    true
}
/// Seen bits of the fields of one `iterations` element.
const ITEM_FIELDS: u64 = 0b1_1111 << 20;
/// Seen bits of the fields of one content or prompt block.
const BLOCK_FIELDS: u64 = 0b11 << 41;
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
        let mut unit = None;
        let mut high = false;
        if self.utf != 0 {
            if !(self.lo..=self.hi).contains(&ch) {
                return Err(());
            }
            self.utf -= 1;
            self.lo = 128;
            self.hi = 191;
        } else if self.escape == 2 {
            let digit = (ch as char).to_digit(16).ok_or(())?;
            self.code = self.code << 4 | digit as u16;
            self.hex += 1;
            if self.hex == 4 {
                self.hex = 0;
                self.escape = 0;
                // A pair is one character and one `WIDE` unit, as parsed; an
                // unpaired surrogate is one replacement unit of its own.
                let paired = std::mem::take(&mut self.high);
                unit = match self.code {
                    0..=0x7f => Some(self.code as u8),
                    0xdc00..=0xdfff if paired => None,
                    _ => Some(WIDE),
                };
                high = (0xd800..=0xdbff).contains(&self.code);
            }
        } else if self.escape != 0 {
            if ch == b'u' {
                self.escape = 2;
                self.hex = 0;
                self.code = 0;
            } else if let Some(at) = b"\"\\/bfnrt".iter().position(|c| *c == ch) {
                self.escape = 0;
                unit = Some(b"\"\\/\x08\x0c\n\r\t"[at]);
            } else {
                return Err(());
            }
        } else if ch == b'\\' {
            self.escape = 1;
        } else if ch == b'"' && self.deep > 0 {
            self.lex = 0;
            return Ok(());
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
            self.high = false;
            return Ok(());
        } else if ch < 32 {
            return Err(());
        } else if ch >= 128 {
            unit = Some(WIDE);
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
        } else {
            unit = Some(ch);
        }
        if unit.is_some() {
            self.high = high;
        }
        if let Some(unit) = unit.filter(|_| self.key == 0) {
            self.unit(unit);
        }
        self.record_byte(ch);
        Ok(())
    }
    /// One text unit of a text value. Once the prefix is complete it is
    /// analysed and the rest of the value is skipped unread.
    fn unit(&mut self, unit: u8) {
        if !TEXT_NODES.contains(&self.capture) {
            return;
        }
        if self.scan.len() < LIMIT {
            self.scan.push(unit);
            return;
        }
        let node = std::mem::replace(&mut self.capture, NONE);
        self.text_done(node);
    }
    fn text_done(&mut self, node: u8) {
        let text = Text::analyse(&self.scan);
        self.scan.clear();
        self.blocks.text_value(node, text);
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
        if self.deep > 0 {
            return self.deep_byte(ch);
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
                    self.miss = false;
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
                // A frame's own scalar carries nothing, so it is never captured.
                let capture = if FRAMES.contains(&node) { NONE } else { node };
                match ch {
                    b'"' => {
                        self.lex = 1;
                        self.key = 0;
                        self.capture = capture;
                        if PRESENCE.contains(&capture) {
                            self.scalar(capture, &json!(""), id, time);
                            self.capture = NONE;
                        }
                        self.value.clear();
                        self.scan.clear();
                    }
                    b'{' | b'[' => {
                        let child = self.open(node, ch == b'{', id, time);
                        // A container past the bound holds no consumed node,
                        // so it is only counted (D3).
                        if self.stack.len() >= DEPTH && child == NONE {
                            self.deep = 1;
                            self.overflow = true;
                            return Ok(());
                        }
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
                        self.capture = capture;
                    }
                    b'-' | b'0'..=b'9' => {
                        self.lex = 2;
                        self.number = match ch {
                            b'-' => 0,
                            b'0' => 1,
                            _ => 2,
                        };
                        self.capture = capture;
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
    /// One structural byte inside the containers past `DEPTH`: strings are
    /// read in full, other bytes must belong to JSON's token alphabet, and
    /// the bracket that closes the outermost of them completes a value of
    /// the container at `DEPTH`.
    fn deep_byte(&mut self, ch: u8) -> Result<(), ()> {
        match ch {
            b'"' => {
                self.lex = 1;
                self.key = 0;
            }
            b'{' | b'[' if self.deep < DEEP => self.deep += 1,
            b'}' | b']' => {
                self.deep -= 1;
                if self.deep == 0 {
                    self.value_done();
                }
            }
            b',' | b':' | b'-' | b'+' | b'.' | b'0'..=b'9' => {}
            _ if b"truefalsn".contains(&ch) || ch == b'E' => {}
            _ => return Err(()),
        }
        Ok(())
    }
    /// Ends a pass inside the record. A string or number capture cannot be
    /// persisted, so its field is lost; the record then classifies as unknown.
    /// A `sessionId` already over `CAP` is decided: as in `capture_done`, it
    /// can never equal a safe id, so it is a mismatch rather than lost.
    pub fn suspend(&mut self) {
        if [1, 2].contains(&self.lex) && self.capture != NONE {
            if self.capture == 3 && self.value.len() > CAP {
                self.record.identity = super::IDENTITY_MISMATCH;
            } else {
                self.lost |= 1 << self.capture;
            }
            self.capture = NONE;
        }
        self.value.clear();
        self.scan.clear();
        self.high = false;
    }
    /// The record nested past `DEPTH`, so part of it was only counted.
    pub fn overflowed(&self) -> bool {
        self.overflow
    }
    /// The record's `sessionId` reading so far. It is meaningful only when
    /// `finish` does not return `Outcome::Invalid`.
    pub fn identity(&self) -> u8 {
        self.record.identity
    }
    /// Classifies the complete record once its terminating newline is fed.
    /// A lost `type` or `sessionId` (one cut by a pass boundary) is invalid
    /// for every kind: the record may name another session, so its identity
    /// is never taken as absent.
    pub fn finish(mut self) -> Outcome {
        let identity = 1 << 1 | 1 << 3;
        if self.done == 0 || !self.stack.is_empty() || self.lex != 0 || self.lost & identity != 0 {
            return Outcome::Invalid;
        }
        self.record
            .finish(self.message_object, &self.iterations, &self.blocks);
        // Only the fields this record type consumes count when lost.
        self.lost &= super::consumed(self.record.kind);
        // A mismatched identity fails the session whatever else was lost; a
        // forked record feeds nothing either way.
        let decided = self.record.identity == super::IDENTITY_MISMATCH || self.record.forked;
        // A record nested past the bound was not read in full.
        if (self.lost != 0 || self.overflow) && !decided {
            return Outcome::Unclassified(self.record.kind);
        }
        Outcome::Record(Box::new(self.record))
    }
}
impl Iterations {
    fn validate(&self) -> bool {
        let safe = |v: &u64| *v <= super::SAFE;
        let kinds = super::ITERATIONS.len() as u8;
        self.shape <= 2
            && self.kind <= kinds
            && self
                .candidate
                .is_none_or(|(kind, counters)| kind <= kinds && counters.iter().flatten().all(safe))
            && self.counters.iter().flatten().all(safe)
    }
}
impl Classifier {
    /// Revalidates restored state so that feeding it can never panic or carry
    /// a capture across a checkpoint.
    pub fn validate(&self) -> bool {
        let count = NODES.len() as u8;
        let all = (1u64 << count) - 1;
        let containers = [
            0,
            MESSAGE,
            USAGE,
            ITERATIONS_NODE,
            ITEM,
            ORIGIN,
            TOOL,
            ATTACHMENT,
            CONTENT,
            PROMPT,
            BLOCK,
            NONE,
        ];
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
            && self.name.len() <= NAME
            && (self.key == 1 || self.name.is_empty() && !self.miss)
            && (!self.miss || self.name.is_empty())
            && (self.name.is_empty()
                || self.nodes.last().is_some_and(|parent| viable(*parent, &self.name)))
            // Only a literal keeps its node across a pass: it holds no bytes.
            && (self.capture == NONE || self.lex == 3 && self.capture < count)
            && self.value.is_empty()
            && (self.started == 1 || depth == 0 && self.done == 0 && self.lex == 0)
            && (self.done == 0 || depth == 0 && self.lex == 0)
            && (self.lex == 0 || depth > 0)
            && (self.lex == 1 || self.escape == 0 && self.hex == 0 && self.utf == 0)
            && (self.lex == 1 || self.key == 0)
            && (self.lex != 3 || (self.index as usize) < WORDS[self.literal as usize].len())
            && self.deep <= DEEP
            && (self.deep == 0 || self.overflow)
            // Inside the region past the bound only a string is ever open.
            && (self.deep == 0
                || depth == DEPTH
                    && self.lex <= 1
                    && self.key == 0
                    && self.capture == NONE
                    && self.name.is_empty()
                    && !self.miss
                    && [3, 5, 6].contains(&self.stack[DEPTH - 1]));
        let record = &self.record;
        let fields = record.kind as usize <= super::KINDS.len()
            && record.subtype as usize <= super::SUBTYPES.len()
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
                .is_none_or(|key| crate::common::hex_id(key, super::GROUP))
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
            && !record.bad
            && record.origin <= super::ORIGIN_OTHER
            && record.mode as usize <= super::MODES.len()
            && record.operation as usize <= super::OPERATIONS.len()
            && record.elapsed.is_none_or(|value| value <= super::SAFE)
            && [&record.agent, &record.resumed]
                .iter()
                .all(|key| key.as_deref().is_none_or(|key| crate::common::hex_id(key, 64)))
            // Text facts are derived only when the record finishes.
            && record.text.is_none()
            && !record.interrupt;
        shape
            && lexical
            && fields
            && self.seen & !all == 0
            && self.lost & !all == 0
            && self.iterations.validate()
            && self.blocks.validate()
            && self.scan.is_empty()
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
        let mut lines = vec![
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
            format!(
                "{{\"type\":\"attachment\",\"operation\":\"dequeue\",\"sessionId\":\"{ID}\",\"timestamp\":\"{STAMP}\"}}"
            ),
            format!("{{\"type\":7,\"sessionId\":5,\"uuid\":[],\"timestamp\":{{}}}}"),
            "{}".to_owned(),
            // An empty key names no consumed field, even past `CAP`.
            assistant(&format!("\"\":\"{pad}\","), ""),
        ];
        for operation in [
            "\"enqueue\"",
            "\"dequeue\"",
            "\"remove\"",
            "\"popAll\"",
            "7",
            "null",
            "{\"op\":\"dequeue\"}",
        ] {
            for key in ["operation", "oper\\u0061tion"] {
                lines.push(format!(
                    "{{\"type\":\"queue-operation\",\"{key}\":{operation},\"sessionId\":\"{ID}\",\"timestamp\":\"{STAMP}\",\"content\":\"{pad}\"}}"
                ));
            }
        }
        lines
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

    fn user(fields: &str) -> String {
        format!(
            "{{\"type\":\"user\",\"sessionId\":\"{ID}\",\"uuid\":\"u-2\",\"timestamp\":\"{STAMP}\",{fields}}}"
        )
    }
    fn content(content: &str) -> String {
        user(&format!(
            "\"message\":{{\"role\":\"user\",\"content\":{content}}}"
        ))
    }
    fn queued(mode: &str, prompt: &str) -> String {
        format!(
            "{{\"type\":\"attachment\",\"sessionId\":\"{ID}\",\"timestamp\":\"{STAMP}\",\"attachment\":{{\"type\":\"queued_command\",\"commandMode\":{mode},\"prompt\":{prompt}}}}}"
        )
    }
    /// Synthetic D6 and D7 records covering every child and turn field.
    fn turn_lines() -> Vec<String> {
        let notice = "\"<task-notification>\\n<task-id>agent-1</task-id>\\n<status>failed</status>\\n<summary>x</summary>\\n</task-notification>\"";
        let mut lines = vec![];
        for origin in [
            "null",
            "{}",
            "{\"kind\":\"human\"}",
            "{\"kind\":\"task-notification\"}",
            "{\"kind\":\"peer\"}",
            "{\"kind\":\"coordinator\"}",
            "{\"kind\":\"other\"}",
            "{\"kind\":7}",
            "\"human\"",
        ] {
            lines.push(
                content(&format!("\"plain\",\"origin\":{origin}"))
                    .replace("\"message\"", &format!("\"origin\":{origin},\"message\"")),
            );
        }
        lines.retain(|line| serde_json::from_str::<Value>(line).is_ok());
        for result in [
            "{\"status\":\"async_launched\",\"agentId\":\"agent-1\",\"prompt\":\"p\"}",
            "{\"status\":\"async_launched\"}",
            "{\"status\":\"async_launched\",\"agentId\":\"bad id\"}",
            "{\"status\":\"async_launched\",\"agentId\":7}",
            "{\"status\":\"async_launched\",\"agentId\":null}",
            "{\"agentId\":\"agent-2\",\"totalDurationMs\":1200,\"totalTokens\":5}",
            "{\"agentId\":\"agent-2\",\"totalDurationMs\":\"x\"}",
            "{\"resumedAgentId\":\"agent-1\",\"success\":true}",
            "{\"resumedAgentId\":\"agent-1\",\"success\":\"true\"}",
            "{\"resumedAgentId\":[],\"success\":false}",
            "\"Error: plain string result\"",
            "null",
        ] {
            lines.push(user(&format!(
                "\"toolUseResult\":{result},\"message\":{{\"role\":\"user\",\"content\":[{{\"type\":\"tool_result\",\"content\":[{{\"type\":\"text\",\"text\":\"<command-name>x\"}}]}}]}}"
            )));
        }
        let twice = format!("{}\\n{}", &notice[..notice.len() - 1], &notice[1..]);
        let spaced = format!("{} \\n\"", &notice[..notice.len() - 1]);
        for text in [
            notice,
            &twice,
            &spaced,
            "\"<command-name>/x</command-name>\"",
            "\"<local-command-stdout>x</local-command-stdout>\"",
            "\"<bash-input>ls</bash-input>\"",
            "\"<other-tag>x\"",
            "\"[Request interrupted by user]\"",
            "\"[Request interrupted by user for tool use]\"",
            "\"\\u005bRequest interrupted by user\\u005d\"",
            "\"\\u003ccommand-name\\u003e\"",
            "\"\\ud83d\\ude00<command-name>\"",
            "\"\u{e9}\u{1f600} wide\"",
            "\"tab\\t\\\"quote\\\\ \\/\"",
            "7",
            "null",
        ] {
            lines.push(content(text));
            lines.push(content(&format!(
                "[{{\"type\":\"image\",\"text\":\"<command-name>\"}},{{\"type\":\"tool_result\"}},{{\"text\":{text},\"type\":\"text\"}},{{\"type\":\"text\",\"text\":\"[Request interrupted by user]\"}},\"loose\",[{{\"type\":\"text\",\"text\":\"<bash-input>\"}}]]"
            )));
            lines.push(queued("\"task-notification\"", text));
            lines.push(queued(
                "\"prompt\"",
                &format!("[{{\"type\":\"text\",\"text\":{text}}}]"),
            ));
        }
        lines.push(content("[]"));
        lines.push(content("{\"type\":\"text\",\"text\":\"<command-name>\"}"));
        lines.push(queued("\"other\"", notice));
        lines.push(queued("null", "null"));
        lines.push(
            queued("\"task-notification\"", notice).replace("queued_command", "edited_text_file"),
        );
        lines.push(content(notice).replace("\"type\":\"user\"", "\"type\":\"system\""));
        lines.push(user("\"interruptedMessageId\":\"msg_a\",\"isMeta\":true"));
        lines.push(user("\"interruptedMessageId\":null"));
        lines.push(
            assistant(
                "\"isAbortedMidStream\":true,\"interruptedMessageId\":\"msg_a\",",
                "",
            )
            .replace("\"end_turn\"", "null"),
        );
        lines.push(assistant("\"isAbortedMidStream\":\"yes\",", ""));
        lines.push(assistant("\"origin\":{\"kind\":\"human\"},\"toolUseResult\":{\"agentId\":\"agent-1\",\"totalDurationMs\":1},", ""));
        lines.push(
            format!(
                "{{\"type\":\"system\",\"subtype\":\"stop_hook_summary\",\"sessionId\":\"{ID}\",\"timestamp\":\"{STAMP}\",\"isAbortedMidStream\":true}}"
            ),
        );
        for (subtype, fields) in [
            ("turn_duration", "\"durationMs\":4250,\"messageCount\":3"),
            (
                "turn_duration",
                "\"durationMs\":0,\"pendingBackgroundAgentCount\":0,\"pendingWorkflowCount\":0",
            ),
            (
                "turn_duration",
                "\"durationMs\":12.5,\"pendingBackgroundAgentCount\":2",
            ),
            (
                "turn_duration",
                "\"durationMs\":-3,\"pendingWorkflowCount\":1",
            ),
            (
                "turn_duration",
                "\"durationMs\":\"9\",\"pendingWorkflowCount\":null",
            ),
            (
                "turn_duration",
                "\"durationMs\":{},\"pendingBackgroundAgentCount\":[]",
            ),
            ("turn_duration", "\"durationMs\":9007199254740992"),
            ("turn_duration", "\"pendingBackgroundAgentCount\":\"0\""),
            (
                "compact_boundary",
                "\"durationMs\":5,\"pendingWorkflowCount\":1",
            ),
        ] {
            lines.push(format!(
                "{{\"type\":\"system\",\"subtype\":\"{subtype}\",\"sessionId\":\"{ID}\",\"timestamp\":\"{STAMP}\",{fields}}}"
            ));
        }
        lines
    }

    #[test]
    fn classifier_matches_parsed_path_for_child_and_turn_fields() {
        let lines = turn_lines();
        assert!(lines.len() > 70);
        for line in &lines {
            let bytes = line.as_bytes();
            for size in [1, 2, 7, 64, bytes.len()] {
                let chunks: Vec<&[u8]> = bytes.chunks(size).collect();
                assert_eq!(
                    classify(&chunks, false),
                    expected(line),
                    "{size}: {line:.160}"
                );
            }
        }
    }

    #[test]
    fn classifier_text_cut_by_a_pass_boundary_is_never_read_as_a_prefix() {
        let marker = "[Request interrupted by user]";
        let echo = content(&format!("\"xx{marker}\""));
        let tagged =
            content("[{\"type\":\"text\",\"text\":\"xx<command-name>/x</command-name>\"}]");
        let prompt = queued("\"prompt\"", "\"xx<bash-input>ls</bash-input>\"");
        for (line, kind) in [(&echo, 2), (&tagged, 2), (&prompt, 3)] {
            // Cut after "xx": the suffix alone would read as a marker or tag.
            let parts = split(line, "xx", 2);
            assert_eq!(
                classify(&parts, true),
                Outcome::Unclassified(kind),
                "{line}"
            );
            assert_eq!(classify(&parts, false), expected(line));
        }
        // Once `LIMIT` units are read the prefix is complete and survives.
        let long = content(&format!("\"<bash-stdout>{}\"", "y".repeat(LIMIT + 100)));
        let parts = split(&long, "yyyy", LIMIT);
        let Outcome::Record(record) = classify(&parts, true) else {
            panic!("a complete prefix must survive the boundary");
        };
        assert_eq!(Outcome::Record(record.clone()), expected(&long));
        assert_eq!(record.text.map(|text| text.lead), Some(5));
        // Text of an assistant record is never consumed, so a cut is harmless.
        let lines = lines();
        let parts = split(&lines[1], "yyyy", 2);
        assert_eq!(classify(&parts, true), expected(&lines[1]));
    }

    /// Splits `line` just after the first `marker` plus `into` bytes.
    fn split<'a>(line: &'a str, marker: &str, into: usize) -> [&'a [u8]; 2] {
        let at = line.find(marker).unwrap() + into;
        let (head, tail) = line.as_bytes().split_at(at);
        [head, tail]
    }

    #[test]
    fn classifier_state_survives_pass_boundaries_except_inside_captures() {
        let lines = lines();
        let padded = &lines[1];
        let survive = [
            ("yyyy", 100),
            ("\\u00e9", 3),
            ("\u{1f600}", 2),
            ("\"timestamp\"", 5),
            ("\"isMeta\":false", 11),
            ("\"stop_reason\"", 14),
            ("\"content\"", 0),
        ];
        for (marker, into) in survive {
            let parts = split(padded, marker, into);
            assert_eq!(classify(&parts, true), expected(padded), "{marker}");
        }
        let cut = [
            ("\"input_tokens\":10", 16),
            ("\"timestamp\":\"", 15),
            ("\"msg_a\"", 3),
            ("\"claude-fixture-1\"", 4),
        ];
        for (marker, into) in cut {
            let parts = split(padded, marker, into);
            assert_eq!(classify(&parts, true), Outcome::Unclassified(1), "{marker}");
        }
        let user = &lines[14];
        let parts = split(user, "\"u-1\"", 2);
        assert_eq!(classify(&parts, true), Outcome::Unclassified(2));
        // Fields under `message` of another record type are never consumed.
        let parts = split(user, "\"ignored\"", 2);
        assert_eq!(classify(&parts, true), expected(user));
    }

    #[test]
    fn classifier_cut_session_id_is_invalid_and_an_overlong_one_mismatches() {
        let lines = lines();
        let progress = format!(
            "{{\"type\":\"progress\",\"sessionId\":\"{ID}\",\"timestamp\":\"{STAMP}\",\"data\":{{}}}}"
        );
        let queue = format!(
            "{{\"type\":\"queue-operation\",\"operation\":\"dequeue\",\"sessionId\":\"{ID}\",\"timestamp\":\"{STAMP}\"}}"
        );
        let user = user("\"message\":{\"role\":\"user\",\"content\":\"hello\"}");
        // Plain, forked (which feeds nothing once identity is known), and
        // record kinds outside the coverage table.
        for line in [&lines[0], &lines[10], &progress, &queue, &user] {
            // A pass boundary inside the value is invalid.
            let parts = split(line, ID, 3);
            assert_eq!(classify(&parts, true), Outcome::Invalid, "{line:.120}");
            // A value over `CAP` names another session, as the parsed path
            // reads it.
            let long = line.replace(ID, &"s".repeat(CAP + 100));
            let mismatch = expected(&long);
            assert!(
                matches!(&mismatch, Outcome::Record(r) if r.identity == super::super::IDENTITY_MISMATCH)
            );
            assert_eq!(classify(&[long.as_bytes()], false), mismatch);
            // A pass boundary after `CAP` bytes of it has already decided the
            // mismatch; one inside `CAP` has not, so that record is invalid.
            let marker = "\"sessionId\":\"";
            let parts = split(&long, marker, marker.len() + CAP + 50);
            assert_eq!(classify(&parts, true), mismatch, "{line:.120}");
            let parts = split(&long, marker, marker.len() + 500);
            assert_eq!(classify(&parts, true), Outcome::Invalid, "{line:.120}");
            // An intact identity still classifies as the parsed path does.
            assert_eq!(classify(&[line.as_bytes()], false), expected(line));
        }
    }

    #[test]
    fn classifier_reads_an_unpaired_surrogate_as_one_character() {
        let tag = "<local-command-stdout>x</local-command-stdout>";
        let text = |lead: &str| content(&format!("\"{lead}{tag}\""));
        // serde rejects the unpaired forms; each reads as a replacement
        // character, so the output tag after it is never the leading tag.
        for (lead, parsed) in [
            ("\\udc00", "\\ufffd"),
            ("\\ud83d", "\\ufffd"),
            ("\\ude00\\ud83d", "\\ufffd\\ufffd"),
            ("\\ud83dx\\ude00", "\\ufffdx\\ufffd"),
            ("\\ud83d\\ud83d\\ude00", "\\ufffd\\ud83d\\ude00"),
            ("\\ud83d\\ude00", "\\ud83d\\ude00"),
        ] {
            let line = text(lead);
            assert_eq!(whole(line.as_bytes()), expected(&text(parsed)), "{lead}");
            // Up to the tag, one unit per parsed character.
            let mut classifier = Classifier::default();
            assert!(classifier.feed(&line.as_bytes()[..line.find(tag).unwrap()], ID, now()));
            let chars: String = serde_json::from_str(&format!("\"{parsed}\"")).unwrap();
            assert_eq!(
                classifier.scan,
                crate::claude::text::units(&chars).collect::<Vec<_>>()
            );
            // A pair split between chunks of one pass is still one character.
            let at = line.rfind("\\u").unwrap();
            for at in [at, at + 3] {
                let (head, tail) = line.as_bytes().split_at(at);
                let outcome = classify(&[head, tail], false);
                assert_eq!(outcome, expected(&text(parsed)), "{lead}");
            }
        }
        assert_ne!(expected(&text("")), expected(&text("\\ufffd")));
        // A high surrogate ending one string pairs with nothing in the next.
        let after = |end: &str, lead: &str| {
            user(&format!(
                "\"x\":\"{end}\",\"message\":{{\"role\":\"user\",\"content\":\"{lead}{tag}\"}}"
            ))
        };
        let line = after("\\ud83d", "\\udc00");
        assert_eq!(
            whole(line.as_bytes()),
            expected(&after("\\ufffd", "\\ufffd"))
        );
    }

    /// The persisted key bytes after feeding `head` and suspending.
    fn persisted_name(head: &[u8]) -> Vec<u8> {
        let mut classifier = Classifier::default();
        assert!(classifier.feed(head, ID, now()));
        classifier.suspend();
        assert!(classifier.validate());
        let state = serde_json::to_value(&classifier).unwrap();
        serde_json::from_value(state["name"].clone()).unwrap()
    }

    #[test]
    fn classifier_never_persists_key_bytes_outside_the_schema() {
        let line = user(
            "\"toolUseResult\":{\"stat_secret_value\":1,\"st\\u0061tus\":\"async_launched\",\"agentId\":\"agent-a\"},\"message\":{\"role\":\"user\",\"content\":\"x\"}",
        );
        // A schema prefix may persist; the divergent rest never does.
        assert_eq!(persisted_name(split(&line, "stat_", 4)[0]), b"stat");
        for into in [5, 10, 18] {
            let name = persisted_name(split(&line, "stat_", into)[0]);
            assert!(name.is_empty(), "{into}: {name:?}");
            let parts = split(&line, "stat_", into);
            assert_eq!(classify(&parts, true), expected(&line), "{into}");
        }
        // An escaped schema key cut inside its escape still matches.
        for into in [3, 5, 7, 8] {
            let parts = split(&line, "st\\u0061tus", into);
            assert!(!persisted_name(parts[0]).is_empty(), "{into}");
            assert_eq!(classify(&parts, true), expected(&line), "{into}");
        }
        let Outcome::Record(record) = expected(&line) else {
            panic!("record expected");
        };
        assert!(record.launch && record.agent.is_some());
        // A non-schema key under a consumed parent never persists either.
        let private = line.replace("agentId", "private");
        assert!(persisted_name(split(&private, "private", 4)[0]).is_empty());
    }

    fn whole(line: &[u8]) -> Outcome {
        classify(&[line], false)
    }

    /// `{"a":` then `depth` nested arrays around `inner`, then `}`: `depth + 1`
    /// containers in all.
    fn nested(depth: usize, inner: &str) -> String {
        format!(
            "{{\"a\":{}{inner}{}}}",
            "[".repeat(depth),
            "]".repeat(depth)
        )
    }
    /// A user record whose tool result nests past `DEPTH` keeps its `type`: it
    /// is unclassified, so only the D3 coverage table applies, at every
    /// chunking and across pass boundaries. Inside the region past the bound
    /// strings and the token alphabet are still checked, and the bracket
    /// count must balance; bracket kinds and separators there are not.
    #[test]
    fn classifier_reads_nesting_beyond_its_bound_as_unclassified() {
        let deep = |inner: &str| {
            format!(
                "{{\"type\":\"user\",\"sessionId\":\"{ID}\",\"uuid\":\"u-1\",\"timestamp\":\"{STAMP}\",\"toolUseResult\":{{\"data\":{}{inner}{}}},\"message\":{{\"role\":\"user\",\"content\":[{{\"type\":\"tool_result\",\"content\":\"done\"}}]}}}}",
                "[{\"k\":".repeat(DEPTH),
                "}]".repeat(DEPTH)
            )
        };
        let line = deep("[\"s\\u00e9\\\"]\",-1.5e3,true,null,{\"x\":false}]");
        let bytes = line.as_bytes();
        // Pass boundaries fall only inside the nested value, so no consumed
        // capture is cut; inside it they cut strings and escapes anywhere.
        let from = line.find("\"data\":").unwrap() + 7;
        let to = line.find("},\"message\"").unwrap();
        for size in [1, 3, 64, bytes.len()] {
            let chunks: Vec<&[u8]> = bytes.chunks(size).collect();
            let unclassified = Outcome::Unclassified(super::super::KIND_USER);
            assert_eq!(classify(&chunks, false), unclassified, "{size}");
            let mut cut = vec![&bytes[..from]];
            cut.extend(bytes[from..to].chunks(size));
            cut.push(&bytes[to..]);
            assert_eq!(classify(&cut, true), unclassified, "{size}");
        }
        assert_eq!(
            whole(nested(DEPTH, "1").as_bytes()),
            Outcome::Unclassified(super::super::KIND_OTHER)
        );
        for inner in ["\"\\x\"", "\"\x01\"", "\"\\u12\"", "@", "[1"] {
            assert_eq!(whole(deep(inner).as_bytes()), Outcome::Invalid, "{inner}");
        }
        let mut broken = deep("\"x\"").into_bytes();
        let at = broken.iter().rposition(|c| *c == b'x').unwrap();
        broken[at] = 0xff;
        assert_eq!(whole(&broken), Outcome::Invalid);
        // An unterminated region, and a bracket closing past the region.
        assert_eq!(
            whole(format!("{{\"a\":{}", "[".repeat(DEPTH + 4)).as_bytes()),
            Outcome::Invalid
        );
        assert_eq!(
            whole(format!("{}]", nested(DEPTH, "1")).as_bytes()),
            Outcome::Invalid
        );
    }

    #[test]
    fn classifier_rejects_malformed_json_and_reads_nesting_within_its_bound() {
        let user = format!("{{\"type\":\"user\",\"sessionId\":\"{ID}\"");
        let malformed: Vec<Vec<u8>> = vec![
            format!("{user}}} x").into(),
            user.clone().into(),
            b"[1]".to_vec(),
            b"{} {}".to_vec(),
            b"{\"a\":\"\\x\"}".to_vec(),
            b"{\"a\":\"\\u12\"}".to_vec(),
            b"{\"a\":\"\x01\"}".to_vec(),
            b"{\"a\":\"\xff\"}".to_vec(),
            b"{\"a\":\"\xc0\x80\"}".to_vec(),
            b"{\"a\":\"\xed\xa0\x80\"}".to_vec(),
            b"{\"a\":\"\xe2\x82\"}".to_vec(),
            b"{\"a\":01}".to_vec(),
            b"{\"a\":-}".to_vec(),
            b"{\"a\":1.}".to_vec(),
            b"{\"a\":tru}".to_vec(),
            b"{\"a\" 1}".to_vec(),
            b"{\"a\":1,}".to_vec(),
            b"{\"a\":[1,]}".to_vec(),
            b"{,}".to_vec(),
            b"{\"type\":\"user\",\"type\":\"user\"}".to_vec(),
        ];
        for line in malformed {
            assert_eq!(
                whole(&line),
                Outcome::Invalid,
                "{}",
                String::from_utf8_lossy(&line)
            );
        }
        assert!(matches!(
            whole(nested(DEPTH - 1, "1").as_bytes()),
            Outcome::Record(_)
        ));
    }

    #[test]
    fn classifier_loses_duplicated_or_overlong_consumed_fields() {
        let lines = lines();
        let base = &lines[0];
        let long = "z".repeat(CAP + 1);
        let unclassified = [
            base.replacen("\"uuid\"", "\"timestamp\":\"x\",\"uuid\"", 1),
            base.replacen("\"model\"", "\"id\":\"msg_b\",\"model\"", 1),
            base.replacen(
                "\"input_tokens\"",
                "\"output_tokens\":1,\"input_tokens\"",
                1,
            ),
            base.replacen("msg_a", &long, 1),
            base.replacen(":10,", &format!(":1{},", "0".repeat(CAP)), 1),
        ];
        for line in &unclassified {
            assert_eq!(
                whole(line.as_bytes()),
                Outcome::Unclassified(1),
                "{line:.160}"
            );
        }
        let system = &lines[16];
        let duplicated = system.replacen("\"type\"", "\"subtype\":\"x\",\"type\"", 1);
        assert_eq!(whole(duplicated.as_bytes()), Outcome::Unclassified(4));
        // Long values and repeated keys outside consumed paths are skipped.
        let skipped = base.replacen(
            "\"uuid\"",
            &format!("\"pad\":\"{long}\",\"pad\":[\"{long}\"],\"{long}\":1,\"uuid\""),
            1,
        );
        assert_eq!(whole(skipped.as_bytes()), expected(base));
        // A mismatched session or a fork decides the record whatever was lost.
        let mismatch = lines[11].replacen("msg_a", &long, 1);
        let Outcome::Record(record) = whole(mismatch.as_bytes()) else {
            panic!("mismatch must stay a record");
        };
        assert_eq!(record.identity, super::super::IDENTITY_MISMATCH);
        let fork = lines[10].replacen("msg_a", &long, 1);
        assert!(matches!(whole(fork.as_bytes()), Outcome::Record(record) if record.forked));
    }

    #[test]
    fn classifier_reads_presence_only_fields_of_any_string_length() {
        let long =
            serde_json::to_string(&format!("Error: {}\n\u{e9}", "q".repeat(2 * CAP))).unwrap();
        let message = "\"message\":{\"role\":\"user\",\"content\":\"x\"}";
        for key in [
            "toolUseResult",
            "interruptedMessageId",
            "isCompactSummary",
            "forkedFrom",
            "origin",
        ] {
            let line = user(&format!("\"{key}\":{long},{message}"));
            let expected = expected(&line);
            assert!(matches!(&expected, Outcome::Record(_)), "{key}");
            assert_eq!(whole(line.as_bytes()), expected, "{key}");
            // A pass boundary inside the value loses nothing either.
            for into in [3, CAP, CAP + 9] {
                let parts = split(&line, "Error: ", into);
                assert_eq!(classify(&parts, true), expected, "{key} {into}");
            }
        }
        for (node, key) in [
            (FORKED, "forkedFrom"),
            (COMPACT_SUMMARY, "isCompactSummary"),
            (ORIGIN, "origin"),
            (TOOL, "toolUseResult"),
            (INTERRUPTED, "interruptedMessageId"),
        ] {
            assert_eq!(NODES[node as usize], (0, key));
        }
        // Literals keep their exact meaning.
        for value in ["false", "null", "true"] {
            let line = user(&format!("\"isCompactSummary\":{value},{message}"));
            assert_eq!(whole(line.as_bytes()), expected(&line), "{value}");
        }
    }

    #[test]
    fn classifier_validate_rejects_tampered_state() {
        let mut classifier = Classifier::default();
        assert!(classifier.feed(b"{\"type\":\"user\",\"isMeta\":tr", ID, now()));
        classifier.suspend();
        let state = serde_json::to_value(&classifier).unwrap();
        let restored: Classifier = serde_json::from_value(state.clone()).unwrap();
        assert!(restored.validate());
        let tampered = [
            ("stack", json!([3, 3])),
            ("nodes", json!([200])),
            ("pending", json!([99])),
            ("lex", json!(4)),
            ("capture", json!(200)),
            ("lex", json!(1)),
            ("index", json!(4)),
            ("seen", json!(u64::MAX)),
            ("name", json!(vec![b'a'; NAME + 1])),
            ("name", json!(b"type")),
            ("miss", json!(true)),
            // A counted region needs the bound reached and its flag.
            ("deep", json!(1)),
        ];
        for (field, value) in tampered {
            let mut state = state.clone();
            state[field] = value;
            let restored: Classifier = serde_json::from_value(state).unwrap();
            assert!(!restored.validate(), "{field}");
        }
        // Inside the counted region past `DEPTH`, cut inside an escape.
        let mut classifier = Classifier::default();
        let head = format!("{{\"a\":{}\"\\u00", "[".repeat(DEPTH + 2));
        assert!(classifier.feed(head.as_bytes(), ID, now()));
        classifier.suspend();
        let state = serde_json::to_value(&classifier).unwrap();
        assert_eq!(state["deep"], json!(3));
        let restored: Classifier = serde_json::from_value(state.clone()).unwrap();
        assert!(restored.validate());
        for (field, value) in [
            ("deep", json!(DEEP + 1)),
            ("overflow", json!(false)),
            ("lex", json!(2)),
            ("capture", json!(1)),
        ] {
            let mut state = state.clone();
            state[field] = value;
            let restored: Classifier = serde_json::from_value(state).unwrap();
            assert!(!restored.validate(), "{field}");
        }
        // Inside a key, only a viable schema prefix is valid.
        let mut classifier = Classifier::default();
        assert!(classifier.feed(b"{\"ty", ID, now()));
        let state = serde_json::to_value(&classifier).unwrap();
        assert!(
            serde_json::from_value::<Classifier>(state.clone())
                .unwrap()
                .validate()
        );
        let mut tampered = state.clone();
        tampered["name"] = json!(b"tz");
        assert!(
            !serde_json::from_value::<Classifier>(tampered)
                .unwrap()
                .validate()
        );
        let mut missing = state.clone();
        missing.as_object_mut().unwrap().remove("record");
        assert!(serde_json::from_value::<Classifier>(missing).is_err());
    }
}
