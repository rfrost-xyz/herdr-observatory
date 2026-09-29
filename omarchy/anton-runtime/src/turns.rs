use crate::common::{hex_id, safe_id, sha256};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Turns {
    pub valid: bool,
    pub supported: bool,
    pub current_known: bool,
    pub active: Option<String>,
    pub start: Option<u64>,
    pub last: Option<String>,
    pub last_duration: Option<u64>,
    pub last_outcome: Option<String>,
    pub last_end: Option<u64>,
    pub total: u64,
    pub finished: BTreeMap<String, (u64, u64, String)>,
}
impl Default for Turns {
    fn default() -> Self {
        Self {
            valid: true,
            supported: false,
            current_known: false,
            active: None,
            start: None,
            last: None,
            last_duration: None,
            last_outcome: None,
            last_end: None,
            total: 0,
            finished: BTreeMap::new(),
        }
    }
}
impl Turns {
    pub fn validate(&self) -> bool {
        let safe = 9_007_199_254_740_991;
        if self
            .active
            .iter()
            .chain(&self.last)
            .any(|key| !hex_id(key, 24))
            || [
                self.start,
                self.last_duration,
                self.last_end,
                Some(self.total),
            ]
            .iter()
            .flatten()
            .any(|n| *n > safe)
            || self
                .last_outcome
                .as_deref()
                .is_some_and(|s| !["completed", "aborted"].contains(&s))
            || self.active.is_some() && (self.start.is_none() || !self.current_known)
            || self.active.is_none() && self.start.is_some()
            || self.finished.len() > 512
        {
            return false;
        }
        if self.finished.iter().any(|(key, (start, end, outcome))| {
            !hex_id(key, 24)
                || *start == 0
                || start > end
                || *end > safe
                || !["completed", "aborted"].contains(&outcome.as_str())
        }) {
            return false;
        }
        if self
            .finished
            .values()
            .try_fold(0u64, |sum, (start, end, _)| sum.checked_add(end - start))
            != Some(self.total)
            || self
                .active
                .as_ref()
                .is_some_and(|key| self.finished.contains_key(key))
        {
            return false;
        }
        if self.last.is_none()
            && (self.last_duration.is_some()
                || self.last_outcome.is_some()
                || self.last_end.is_some())
        {
            return false;
        }
        if let Some(last) = &self.last {
            if let Some((start, end, outcome)) = self.finished.get(last) {
                if self.last_duration != Some(end - start)
                    || self.last_outcome.as_ref() != Some(outcome)
                    || self.last_end != Some(*end)
                {
                    return false;
                }
            } else if self.valid {
                return false;
            }
        }
        true
    }
    pub fn unknown(&mut self) {
        self.valid = false;
        self.current_known = false;
        self.active = None;
        self.start = None;
    }
    pub fn observe(&mut self, payload: &Value, session: &str, now: f64) {
        let kind = payload["type"].as_str().unwrap_or("");
        if !["task_started", "task_complete", "turn_aborted"].contains(&kind) {
            return;
        }
        self.supported = true;
        let Some(turn) = payload["turn_id"].as_str().filter(|s| safe_id(s, 128)) else {
            self.unknown();
            return;
        };
        let Some(start) = payload["started_at"]
            .as_u64()
            .filter(|v| *v > 0 && *v as f64 <= now)
        else {
            self.unknown();
            return;
        };
        let key = sha256(format!("anton-turn-v1:{session}:{turn}").as_bytes())[..24].to_owned();
        let previous = self.finished.get(&key);
        if kind == "task_started" {
            if let Some(row) = previous {
                if row.0 != start {
                    self.unknown();
                }
                return;
            }
            if self.active.as_ref() == Some(&key) {
                if self.start != Some(start) {
                    self.unknown();
                }
                return;
            }
            if self.active.is_some() && self.start.is_some_and(|old| start < old) {
                self.valid = false;
                return;
            }
            if self.last_end.is_some_and(|end| start < end) {
                self.unknown();
                return;
            }
            if self.active.is_some() {
                self.valid = false;
            }
            self.active = Some(key);
            self.start = Some(start);
            self.current_known = true;
            return;
        }
        let Some(end) = payload["completed_at"]
            .as_u64()
            .filter(|v| *v >= start && *v as f64 <= now)
        else {
            self.unknown();
            return;
        };
        let outcome = if kind == "turn_aborted" {
            "aborted"
        } else {
            "completed"
        };
        let row = (start, end, outcome.to_owned());
        if let Some(previous) = previous {
            if previous != &row {
                self.unknown();
            }
            return;
        }
        let matched = self.active.as_ref() == Some(&key) && self.start == Some(start);
        if !matched || self.last_end.is_some_and(|last| start < last) {
            self.valid = false;
        }
        let duration = end - start;
        if self.finished.len() < 512 {
            self.finished.insert(key.clone(), row);
            if let Some(total) = self
                .total
                .checked_add(duration)
                .filter(|v| *v <= 9_007_199_254_740_991)
            {
                self.total = total;
            } else {
                self.valid = false;
            }
        } else {
            self.valid = false;
        }
        if matched {
            self.active = None;
            self.start = None;
            self.current_known = true;
        } else if self.active.is_some() && self.start.is_some_and(|start| end > start) {
            self.active = None;
            self.start = None;
            self.current_known = false;
        }
        if !matched && self.last_end.is_some_and(|last| start < last && last < end) {
            self.active = None;
            self.start = None;
            self.current_known = false;
        } else if self
            .last_end
            .is_none_or(|last| end > last && (matched || start >= last))
        {
            self.last = Some(key);
            self.last_duration = Some(duration);
            self.last_outcome = Some(outcome.to_owned());
            self.last_end = Some(end);
        }
    }
}
