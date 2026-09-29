//! Incremental strict JSON envelope classifier. It retains fixed identifiers and
//! grammar state only, never payload strings, while skipping oversized records.
use serde::{Deserialize, Serialize};
use std::time::Instant;
const NAMES: &[&[u8]] = &[
    b"type",
    b"response_item",
    b"session_meta",
    b"turn_context",
    b"compacted",
    b"event_msg",
    b"payload",
    b"item",
    b"item_completed",
    b"context_compacted",
    b"SubAgentActivity",
    b"CommandExecution",
    b"McpToolCall",
];
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Envelope {
    stack: Vec<u8>,
    lex: u8,
    number: u8,
    literal: u8,
    index: u8,
    mask: u16,
    key: u8,
    capture: u8,
    escape: u8,
    hex: u8,
    utf: u8,
    lo: u8,
    hi: u8,
    started: u8,
    done: u8,
    seen: Vec<u8>,
    pending: Vec<u8>,
    #[serde(rename = "type")]
    kind: u8,
    roles: Vec<u8>,
    pub payload_type: u8,
    pub item_type: u8,
}
impl Default for Envelope {
    fn default() -> Self {
        Self {
            stack: vec![],
            lex: 0,
            number: 0,
            literal: 0,
            index: 0,
            mask: 0,
            key: 0,
            capture: 0,
            escape: 0,
            hex: 0,
            utf: 0,
            lo: 128,
            hi: 191,
            started: 0,
            done: 0,
            seen: vec![],
            pending: vec![],
            kind: 0,
            roles: vec![],
            payload_type: 0,
            item_type: 0,
        }
    }
}
impl Envelope {
    pub fn valid(&self) -> bool {
        if self.lex > 3
            || self.number > 8
            || self.literal > 2
            || self.index > 32
            || self.mask > 8191
            || self.key > 1
            || self.capture > 1
            || self.escape > 2
            || self.hex > 3
            || self.utf > 3
            || self.lo > 244
            || self.hi > 244
            || self.started > 1
            || self.done > 1
            || self.kind > 12
            || self.payload_type > 12
            || self.item_type > 12
            || self.stack.len() > 128
            || self.stack.iter().any(|v| *v > 7)
        {
            return false;
        }
        if self.roles.len() != self.stack.len()
            || self.pending.len() != self.stack.len()
            || self.seen.len() != self.stack.len()
            || self.roles.iter().chain(&self.pending).any(|v| *v > 3)
            || self.seen.iter().any(|v| *v > 7)
        {
            return false;
        }
        if self.started == 0 && (!self.stack.is_empty() || self.done != 0 || self.lex != 0)
            || self.done != 0 && (!self.stack.is_empty() || self.lex != 0)
            || self.started != 0 && self.stack.is_empty() && self.done == 0
        {
            return false;
        }
        if !self.stack.is_empty()
            && (self.roles[0] != 1
                || self.stack[0] > 4
                || self.stack[..self.stack.len() - 1]
                    .iter()
                    .any(|v| ![4, 7].contains(v)))
        {
            return false;
        }
        if self.lex != 0 {
            let Some(phase) = self.stack.last() else {
                return false;
            };
            if self.lex == 1 {
                if !(if self.key != 0 {
                    [0, 1].contains(phase)
                } else {
                    [3, 5, 6].contains(phase)
                }) || self.utf != 0 && self.escape != 0
                {
                    return false;
                }
            } else if ![3, 5, 6].contains(phase) {
                return false;
            }
            if self.lex == 3
                && (self.index < 1
                    || self.index as usize
                        >= [b"true".as_slice(), b"false", b"null"][self.literal as usize].len())
            {
                return false;
            }
        }
        self.lex == 1 || self.escape == 0 && self.utf == 0 && self.hex == 0
    }
    fn push(&mut self, phase: u8, role: u8) {
        self.stack.push(phase);
        self.roles.push(role);
        self.pending.push(0);
        self.seen.push(0);
    }
    fn pop(&mut self) {
        self.stack.pop();
        self.roles.pop();
        self.pending.pop();
        self.seen.pop();
    }
    fn character(&mut self, ch: u8) {
        for (bit, name) in NAMES.iter().enumerate() {
            if name.get(self.index as usize) != Some(&ch) {
                self.mask &= !(1 << bit);
            }
        }
        self.index = (self.index + 1).min(32);
    }
    fn value_done(&mut self) {
        if let Some(phase) = self.stack.last_mut() {
            *self.pending.last_mut().unwrap() = 0;
            if *phase == 3 {
                *phase = 4;
            } else if [5, 6].contains(phase) {
                *phase = 7;
            }
        } else {
            self.done = 1;
        }
    }
    fn string_done(&mut self) -> bool {
        let mask = NAMES
            .iter()
            .enumerate()
            .filter(|(bit, name)| self.mask & (1 << bit) != 0 && name.len() == self.index as usize)
            .fold(0u16, |mask, (bit, _)| mask | (1 << bit));
        let index = self.stack.len() - 1;
        let role = self.roles[index];
        if self.key != 0 {
            let key = if mask & 1 != 0 {
                1
            } else if mask & (1 << 6) != 0 {
                2
            } else if mask & (1 << 7) != 0 {
                3
            } else {
                0
            };
            let relevant = role == 1 && [1, 2].contains(&key)
                || role == 2 && [1, 3].contains(&key)
                || role == 3 && key == 1;
            if relevant {
                let bit = 1 << (key - 1);
                if self.seen[index] & bit != 0 {
                    return false;
                }
                self.seen[index] |= bit;
            }
            self.pending[index] = if relevant { key } else { 0 };
            self.stack[index] = 2;
        } else {
            if role != 0 && self.pending[index] == 1 {
                let value = (1..NAMES.len())
                    .find(|bit| mask & (1 << bit) != 0)
                    .unwrap_or(0) as u8;
                match role {
                    1 => self.kind = value,
                    2 => self.payload_type = value,
                    3 => self.item_type = value,
                    _ => {}
                }
            }
            self.value_done();
        }
        self.lex = 0;
        true
    }
    pub fn feed(&mut self, data: &[u8], deadline: Instant) -> Result<(), ()> {
        if !self.valid() {
            return Err(());
        }
        let mut index = 0;
        while index < data.len() {
            if index % 1024 == 0 && Instant::now() >= deadline {
                return Err(());
            }
            let ch = data[index];
            if self.lex == 1 {
                if self.utf != 0 {
                    if !(self.lo..=self.hi).contains(&ch) {
                        return Err(());
                    }
                    self.utf -= 1;
                    self.lo = 128;
                    self.hi = 191;
                } else if self.escape == 2 {
                    let digit = (ch as char).to_digit(16).ok_or(())? as u8;
                    for (bit, name) in NAMES.iter().enumerate() {
                        let expected = name
                            .get(self.index as usize)
                            .map(|v| (*v as u16 >> ((3 - self.hex) * 4)) as u8 & 15);
                        if expected != Some(digit) {
                            self.mask &= !(1 << bit);
                        }
                    }
                    self.hex += 1;
                    if self.hex == 4 {
                        self.index = (self.index + 1).min(32);
                        self.hex = 0;
                        self.escape = 0;
                    }
                } else if self.escape != 0 {
                    if ch == b'u' {
                        self.escape = 2;
                        self.hex = 0;
                    } else if b"\"\\/bfnrt".contains(&ch) {
                        self.character(if b"\"\\/".contains(&ch) { ch } else { 0 });
                        self.escape = 0;
                    } else {
                        return Err(());
                    }
                } else if ch == b'\\' {
                    self.escape = 1;
                } else if ch == b'"' {
                    if !self.string_done() {
                        return Err(());
                    }
                } else if ch < 32 {
                    return Err(());
                } else if ch < 128 {
                    self.character(ch);
                } else {
                    self.character(0);
                    self.utf = match ch {
                        194..=223 => 1,
                        224..=239 => 2,
                        240..=244 => 3,
                        _ => return Err(()),
                    };
                    self.lo = if ch == 224 {
                        160
                    } else if ch == 240 {
                        144
                    } else {
                        128
                    };
                    self.hi = if ch == 237 {
                        159
                    } else if ch == 244 {
                        143
                    } else {
                        191
                    };
                }
                index += 1;
                continue;
            }
            if self.lex == 3 {
                let word = [b"true".as_slice(), b"false", b"null"][self.literal as usize];
                if word.get(self.index as usize) != Some(&ch) {
                    return Err(());
                }
                self.index += 1;
                if self.index as usize == word.len() {
                    self.lex = 0;
                    self.value_done();
                }
                index += 1;
                continue;
            }
            if self.lex == 2 {
                let next = match (self.number, ch) {
                    (0, b'0') => Some(1),
                    (0, b'1'..=b'9') => Some(2),
                    (stage @ 2 | stage @ 4 | stage @ 7, b'0'..=b'9') => Some(stage),
                    (1 | 2, b'.') => Some(3),
                    (3, b'0'..=b'9') => Some(4),
                    (1 | 2 | 4, b'e' | b'E') => Some(5),
                    (5, b'+' | b'-') => Some(6),
                    (5 | 6, b'0'..=b'9') => Some(7),
                    _ => None,
                };
                if let Some(stage) = next {
                    self.number = stage;
                    index += 1;
                    continue;
                }
                if ![1, 2, 4, 7].contains(&self.number) {
                    return Err(());
                }
                self.lex = 0;
                self.value_done();
                continue;
            }
            if b" \r\n\t".contains(&ch) {
                index += 1;
                continue;
            }
            if self.done != 0 {
                return Err(());
            }
            if self.stack.is_empty() {
                if self.started != 0 || ch != b'{' {
                    return Err(());
                }
                self.started = 1;
                self.push(0, 1);
                index += 1;
                continue;
            }
            let last = self.stack.len() - 1;
            let phase = self.stack[last];
            match phase {
                0 | 1 => {
                    if phase == 0 && ch == b'}' {
                        self.pop();
                        self.value_done();
                    } else if ch == b'"' {
                        self.lex = 1;
                        self.key = 1;
                        self.capture = u8::from(self.roles[last] != 0);
                        self.index = 0;
                        self.mask = if self.roles[last] != 0 { 8191 } else { 0 };
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
                _ => {
                    if phase == 5 && ch == b']' {
                        self.pop();
                        self.value_done();
                    } else if ch == b'"' {
                        self.lex = 1;
                        self.key = 0;
                        self.capture = u8::from(self.roles[last] != 0 && self.pending[last] == 1);
                        self.index = 0;
                        self.mask = if self.capture != 0 { 8191 } else { 0 };
                    } else if ch == b'{' || ch == b'[' {
                        let role = self.roles[last];
                        let pending = self.pending[last];
                        let child = if ch == b'{' {
                            if role == 1 && pending == 2 {
                                2
                            } else if role == 2 && pending == 3 {
                                3
                            } else {
                                0
                            }
                        } else {
                            0
                        };
                        self.value_done();
                        self.push(if ch == b'{' { 0 } else { 5 }, child);
                        if self.stack.len() > 128 {
                            return Err(());
                        }
                    } else if b"tfn".contains(&ch) {
                        self.lex = 3;
                        self.literal = if ch == b't' {
                            0
                        } else if ch == b'f' {
                            1
                        } else {
                            2
                        };
                        self.index = 1;
                    } else if ch == b'-' || ch.is_ascii_digit() {
                        self.lex = 2;
                        self.number = if ch == b'-' {
                            0
                        } else if ch == b'0' {
                            1
                        } else {
                            2
                        };
                    } else {
                        return Err(());
                    }
                }
            }
            index += 1;
        }
        Ok(())
    }
    pub fn kind(&self) -> u8 {
        if self.valid() && self.done != 0 && self.lex == 0 && self.stack.is_empty() {
            self.kind
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    #[test]
    fn grammar_escaped_keys_utf8_and_arbitrary_chunk_boundaries() {
        let cases = [
            (
                r#"{"type":"response_item","payload":[null,true,false,-12.3e+4,{"x":"y"}]}"#,
                1,
            ),
            (r#"{"payload":{},"ty\u0070e":"response_\u0069tem"}"#, 1),
            (
                r#"{"payload":"quote\" slash\\ emoji🌌","type":"compacted"}"#,
                4,
            ),
            (
                r#"{"type":"event_msg","payload":{"type":"context_compacted"}}"#,
                5,
            ),
            (
                r#"{"payload":{"item":{"output":"text","type":"CommandExecution"},"type":"item_completed"},"type":"event_msg"}"#,
                5,
            ),
        ];
        for (data, kind) in cases {
            for chunk in [1, 3, 17, 65537] {
                let mut state = Envelope::default();
                for bytes in data.as_bytes().chunks(chunk) {
                    state
                        .feed(bytes, Instant::now() + Duration::from_secs(1))
                        .unwrap();
                    let encoded = serde_json::to_value(&state).unwrap();
                    assert!(
                        encoded
                            .as_object()
                            .unwrap()
                            .values()
                            .all(|v| v.is_number() || v.is_array())
                    );
                }
                assert_eq!(state.kind(), kind, "{data}");
            }
        }
    }
    #[test]
    fn duplicates_malformed_numbers_nested_items_and_invalid_utf8_fail_closed() {
        let cases = [
            r#"{"type":"response_item","type":"compacted"}"#,
            r#"{"type":"response_item","ty\u0070e":"compacted"}"#,
            r#"{"type":"response_item","n":01}"#,
            r#"{"type":"response_item","n":1.}"#,
            r#"{"type":"response_item","n":[1,]}"#,
            r#"{"type":"response_item",}"#,
            r#"{"type":"response_item","n":tru}"#,
            r#"{"type":"response_item","n":"\q"}"#,
            r#"{"type":"response_item"}{}"#,
            r#"{"type":"event_msg","payload":{},"payload":{"type":"item_completed"}}"#,
            r#"{"type":"event_msg","payload":{"type":"item_completed","type":"context_compacted"}}"#,
            r#"{"type":"event_msg","payload":{"type":"item_completed","item":{},"item":{}}}"#,
            r#"{"type":"event_msg","payload":{"type":"item_completed","item":{"type":"CommandExecution","type":"SubAgentActivity"}}}"#,
        ];
        for data in cases {
            assert!(
                Envelope::default()
                    .feed(data.as_bytes(), Instant::now() + Duration::from_secs(1))
                    .is_err(),
                "{data}"
            );
        }
        for bytes in [
            b"\xff".as_slice(),
            b"\xc0\xaf",
            b"\xed\xa0\x80",
            b"\xf4\x90\x80\x80",
        ] {
            let mut data = b"{\"type\":\"response_item\",\"n\":\"".to_vec();
            data.extend_from_slice(bytes);
            data.extend_from_slice(b"\"}");
            assert!(
                Envelope::default()
                    .feed(&data, Instant::now() + Duration::from_secs(1))
                    .is_err()
            );
        }
        let deep = format!(
            "{{\"type\":\"response_item\",\"n\":{}{}}}",
            "[".repeat(130),
            "]".repeat(130)
        );
        assert!(
            Envelope::default()
                .feed(deep.as_bytes(), Instant::now() + Duration::from_secs(1))
                .is_err()
        );
        let mut partial = Envelope::default();
        partial
            .feed(
                b"{\"type\":\"response_item\",\"payload\":\"open",
                Instant::now() + Duration::from_secs(1),
            )
            .unwrap();
        assert_eq!(partial.kind(), 0);
        assert!(Envelope::default().feed(b"{}", Instant::now()).is_err());
    }
    #[test]
    fn hostile_saved_lexical_combinations_never_panic() {
        let mut base = Envelope::default();
        base.feed(
            b"{\"type\":\"response_item\",\"payload\":",
            Instant::now() + Duration::from_secs(1),
        )
        .unwrap();
        for lex in 0..4 {
            for index in 0..33 {
                for phase in 0..8 {
                    let mut state = base.clone();
                    state.lex = lex;
                    state.index = index;
                    state.stack = vec![phase];
                    for bytes in [b"x", b"\"", b"]", b"0", b"}"] {
                        let _ = state
                            .clone()
                            .feed(bytes, Instant::now() + Duration::from_secs(1));
                    }
                }
            }
        }
    }
}
