//! Bounded analysis of the leading text of a user record or queued command.
//! Only the leading wrapper tag, the exact interrupt marker and a task
//! notification's `task-id` and `status` tags are derived; no text is kept.
use crate::common::{hex_id, safe_id, sha256};
use serde::{Deserialize, Serialize};

/// Units of text examined: one per character, ASCII as itself and any other
/// character as `WIDE`, so a parsed string and a streamed one agree.
pub const LIMIT: usize = 4096;
pub const WIDE: u8 = 0x80;
/// Leading tags with a D7 role; `LEAD_NONE` is text without a leading tag.
pub const WRAPPERS: &[&str] = &[
    "command-name",
    "local-command-stdout",
    "local-command-stderr",
    "bash-input",
    "bash-stdout",
    "bash-stderr",
    "task-notification",
];
pub const LEAD_NONE: u8 = 0;
pub const LEAD_COMMAND: u8 = 1;
/// `local-command-stdout` and `local-command-stderr`.
pub const LOCAL_OUTPUT: [u8; 2] = [2, 3];
pub const LEAD_NOTIFICATION: u8 = 7;
pub const LEAD_OTHER: u8 = WRAPPERS.len() as u8 + 1;
/// D6 notification statuses; 0 is absent and `STATUS_OTHER` unrecognised.
pub const STATUSES: &[&str] = &["completed", "failed", "killed", "blocked"];
pub const STATUS_OTHER: u8 = STATUSES.len() as u8 + 1;
const MARKERS: [&str; 2] = [
    "[Request interrupted by user]",
    "[Request interrupted by user for tool use]",
];
/// Longest tag name the grammar reads.
const NAME: usize = 32;

/// The derived facts of one text value.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Text {
    pub lead: u8,
    pub interrupt: bool,
    /// sha256 of a valid `task-id`, the only id read from a notification.
    #[serde(deserialize_with = "Option::deserialize")]
    pub task: Option<String>,
    pub status: u8,
}

/// The units of `text`, as the classifier produces them from raw JSON.
pub fn units(text: &str) -> impl Iterator<Item = u8> + '_ {
    text.chars()
        .map(|ch| if ch.is_ascii() { ch as u8 } else { WIDE })
}

fn position(names: &[&str], value: &[u8]) -> Option<u8> {
    names
        .iter()
        .position(|name| name.as_bytes() == value)
        .map(|index| index as u8 + 1)
}

/// A `<name>` tag starting at `at`: its name and the index after `>`.
fn tag(units: &[u8], at: usize) -> Option<(&[u8], usize)> {
    if units.get(at) != Some(&b'<') {
        return None;
    }
    let start = at + 1;
    let length = units[start..]
        .iter()
        .take(NAME + 1)
        .position(|ch| !(ch.is_ascii_lowercase() || ch.is_ascii_digit() || b"-_".contains(ch)))?;
    let end = start + length;
    (length > 0 && units.get(end) == Some(&b'>')).then(|| (&units[start..end], end + 1))
}

impl Text {
    /// Analyses a whole text value; only its first `LIMIT` units are read.
    pub fn of(text: &str) -> Self {
        Self::analyse(&units(text).take(LIMIT).collect::<Vec<_>>())
    }
    /// Analyses at most `LIMIT` leading units.
    pub fn analyse(units: &[u8]) -> Self {
        let mut text = Self {
            interrupt: MARKERS.iter().any(|marker| units == marker.as_bytes()),
            ..Self::default()
        };
        let Some((name, after)) = tag(units, 0) else {
            return text;
        };
        text.lead = position(WRAPPERS, name).unwrap_or(LEAD_OTHER);
        if text.lead == LEAD_NOTIFICATION {
            text.notification(units, after);
            // Anything but whitespace after the first closing tag, such as
            // a second block, is unreadable, as text before the block is
            // (D6). Past `LIMIT` units nothing is seen.
            let close = b"</task-notification>";
            if let Some(end) = units[after..]
                .windows(close.len())
                .position(|window| window == close)
                && units[after + end + close.len()..]
                    .iter()
                    .any(|ch| !b" \t\r\n".contains(ch))
            {
                text.task = None;
            }
        }
        text
    }
    /// The bounded tag grammar: whitespace-separated `<name>value</name>`
    /// elements. Only `task-id` and `status` values are read; a duplicated
    /// `task-id` loses the id, and any other shape stops the scan.
    fn notification(&mut self, units: &[u8], mut at: usize) {
        let mut task = None;
        loop {
            while units.get(at).is_some_and(|ch| b" \t\r\n".contains(ch)) {
                at += 1;
            }
            let Some((name, start)) = tag(units, at) else {
                break;
            };
            let mut close = Vec::with_capacity(name.len() + 3);
            close.extend_from_slice(b"</");
            close.extend_from_slice(name);
            close.push(b'>');
            let Some(length) = units[start..]
                .windows(close.len())
                .position(|window| window == close)
            else {
                break;
            };
            let value = &units[start..start + length];
            match name {
                b"task-id" if task.is_some() => {
                    task = Some(None);
                    break;
                }
                b"task-id" => {
                    task = Some(
                        std::str::from_utf8(value)
                            .ok()
                            .filter(|id| safe_id(id, 128))
                            .map(|id| sha256(id.as_bytes())),
                    )
                }
                b"status" if self.status == 0 => {
                    self.status = position(STATUSES, value).unwrap_or(STATUS_OTHER)
                }
                _ => {}
            }
            if task.is_some() && self.status != 0 {
                break;
            }
            at = start + length + close.len();
        }
        self.task = task.flatten();
    }
    pub fn validate(&self) -> bool {
        self.lead <= LEAD_OTHER
            && self.status <= STATUS_OTHER
            && self.task.as_deref().is_none_or(|key| hex_id(key, 64))
            && (self.lead == LEAD_NOTIFICATION || self.task.is_none() && self.status == 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notification(body: &str) -> Text {
        Text::of(&format!(
            "<task-notification>\n{body}\n</task-notification>"
        ))
    }
    fn key(id: &str) -> Option<String> {
        Some(sha256(id.as_bytes()))
    }

    #[test]
    fn text_lead_tags_and_interrupt_markers() {
        for (index, name) in WRAPPERS.iter().enumerate() {
            let text = Text::of(&format!("<{name}>x</{name}>"));
            assert_eq!(text.lead, index as u8 + 1, "{name}");
            assert!(text.validate());
        }
        for (source, lead) in [
            ("plain prompt", LEAD_NONE),
            ("", LEAD_NONE),
            ("<command-message>x</command-message>", LEAD_OTHER),
            ("<local-command-caveat>x", LEAD_OTHER),
            (" <command-name>", LEAD_NONE),
            ("<Command-Name>", LEAD_NONE),
            ("<command-name", LEAD_NONE),
            ("<>", LEAD_NONE),
            (&format!("<{}>", "a".repeat(33)), LEAD_NONE),
            (&format!("<{}>", "a".repeat(32)), LEAD_OTHER),
            ("<b\u{e9}>", LEAD_NONE),
        ] {
            assert_eq!(Text::of(source).lead, lead, "{source}");
        }
        for marker in MARKERS {
            let text = Text::of(marker);
            assert!(text.interrupt && text.lead == LEAD_NONE);
            assert!(!Text::of(&format!("{marker} ")).interrupt);
            assert!(!Text::of(&format!(" {marker}")).interrupt);
        }
        assert!(!Text::of("[Request interrupted by user").interrupt);
    }

    #[test]
    fn text_notification_reads_only_task_id_and_status() {
        let both = notification("<task-id>agent-1</task-id>\n<status>completed</status>");
        assert_eq!((both.task.clone(), both.status), (key("agent-1"), 1));
        assert!(both.validate() && both.lead == LEAD_NOTIFICATION);
        let reversed = notification(
            "<summary>a <b>bold</b> </b claim</summary>\r\n\t<status>killed</status><output-file>/x</output-file><task-id>agent_2</task-id>",
        );
        assert_eq!((reversed.task, reversed.status), (key("agent_2"), 3));
        for (status, index) in [
            ("completed", 1),
            ("failed", 2),
            ("killed", 3),
            ("blocked", 4),
            ("paused", STATUS_OTHER),
            ("", STATUS_OTHER),
        ] {
            let text = notification(&format!("<task-id>a</task-id><status>{status}</status>"));
            assert_eq!(text.status, index, "{status}");
        }
        for (body, task, status) in [
            ("<status>failed</status>", None, 2),
            ("<task-id>a b</task-id><status>failed</status>", None, 2),
            ("<task-id>\u{e9}</task-id><status>failed</status>", None, 2),
            ("<task-id></task-id>", None, 0),
            (
                "<task-id>a</task-id><task-id>b</task-id><status>failed</status>",
                None,
                0,
            ),
            (
                "<task-id>a</task-id> stray <status>failed</status>",
                key("a"),
                0,
            ),
            ("<task-id>a</task-id><status>failed", key("a"), 0),
            (
                "<status>completed</status><status>failed</status><task-id>a</task-id>",
                key("a"),
                1,
            ),
        ] {
            let text = notification(body);
            assert_eq!((text.task.clone(), text.status), (task, status), "{body}");
            assert!(text.validate());
        }
        // Tags outside a notification are never read.
        let plain = Text::of("<task-id>a</task-id><status>failed</status>");
        assert!(plain.task.is_none() && plain.status == 0 && plain.lead == LEAD_OTHER);
    }

    #[test]
    fn text_notification_with_content_after_its_closing_tag_is_unreadable() {
        let block = |task: &str| {
            format!(
                "<task-notification>\n<task-id>{task}</task-id>\n<status>completed</status>\n</task-notification>"
            )
        };
        for trailer in [
            block("agent-b"),
            "x".into(),
            "<status>failed</status>".into(),
        ] {
            let text = Text::of(&format!("{}\n{trailer}", block("agent-a")));
            assert!(
                text.validate() && text.lead == LEAD_NOTIFICATION,
                "{trailer}"
            );
            assert_eq!(text.task, None, "{trailer}");
        }
        let text = Text::of(&format!("{} \r\n\t", block("agent-a")));
        assert_eq!(text.task, key("agent-a"));
        // Only the first `LIMIT` units are read: a closing tag past them
        // shows nothing after it.
        let pad = "x".repeat(LIMIT);
        let far = format!(
            "<task-notification><task-id>a</task-id><summary>{pad}</summary></task-notification>{}",
            block("agent-b")
        );
        assert_eq!(Text::of(&far).task, key("a"));
    }

    #[test]
    fn text_scan_is_bounded_to_its_prefix() {
        let pad = "x".repeat(LIMIT - 64);
        let near = notification(&format!("<summary>{pad}</summary><task-id>a</task-id>"));
        assert_eq!(near.task, key("a"));
        let far = notification(&format!(
            "<summary>{pad}xxxxxxxxxx</summary><task-id>a</task-id>"
        ));
        assert_eq!(far.task, None);
        // Each non-ASCII character is one unit, whatever its UTF-8 length.
        let wide = "\u{1f600}".repeat(LIMIT - 64);
        let text = notification(&format!("<summary>{wide}</summary><task-id>a</task-id>"));
        assert_eq!(text.task, key("a"));
        assert_eq!(
            units("a\u{e9}\u{1f600}").collect::<Vec<_>>(),
            [b'a', WIDE, WIDE]
        );
        let mut bad = near.clone();
        bad.lead = LEAD_OTHER;
        assert!(!bad.validate());
        bad = Text {
            task: Some("raw".into()),
            ..near
        };
        assert!(!bad.validate());
    }
}
