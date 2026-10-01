//! D6 child and D7 turn fixtures, all synthetic.
use super::*;
use crate::common::now;

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
/// A record of `kind` at `second` with raw extra JSON fields.
fn record(kind: &str, second: u64, fields: &str) -> String {
    format!(
        "{{\"type\":\"{kind}\",\"sessionId\":\"{ID}\",\"uuid\":\"{kind}-{second}\",\"timestamp\":\"{}\"{}{fields}}}",
        stamp(second),
        if fields.is_empty() { "" } else { "," }
    )
}
/// A user record whose content is the JSON string of `text`.
fn user(second: u64, text: &str, fields: &str) -> String {
    let content = serde_json::to_string(text).unwrap();
    let message = format!("\"message\":{{\"role\":\"user\",\"content\":{content}}}");
    if fields.is_empty() {
        record("user", second, &message)
    } else {
        record("user", second, &format!("{fields},{message}"))
    }
}
fn tool(second: u64, result: &str) -> String {
    record(
        "user",
        second,
        &format!(
            "\"toolUseResult\":{result},\"message\":{{\"role\":\"user\",\"content\":[{{\"type\":\"tool_result\",\"content\":\"done\"}}]}}"
        ),
    )
}
fn launch(second: u64, agent: &str) -> String {
    tool(
        second,
        &format!("{{\"status\":\"async_launched\",\"agentId\":\"{agent}\"}}"),
    )
}
fn notice(task: &str, status: &str) -> String {
    format!(
        "<task-notification>\n<task-id>{task}</task-id>\n<status>{status}</status>\n<summary>synthetic</summary>\n</task-notification>"
    )
}
/// A task notification as a user record with `origin.kind`.
fn notified(second: u64, task: &str, status: &str) -> String {
    user(
        second,
        &notice(task, status),
        "\"origin\":{\"kind\":\"task-notification\"}",
    )
}
/// A `queued_command` attachment carrying `prompt` in `mode`.
fn queued(second: u64, mode: &str, prompt: &str) -> String {
    let prompt = serde_json::to_string(prompt).unwrap();
    record(
        "attachment",
        second,
        &format!(
            "\"attachment\":{{\"type\":\"queued_command\",\"commandMode\":\"{mode}\",\"prompt\":{prompt}}}"
        ),
    )
}
fn assistant(second: u64, message: &str, stop: &str) -> String {
    record(
        "assistant",
        second,
        &format!(
            "\"message\":{{\"id\":\"{message}\",\"model\":\"claude-fixture-1\",\"stop_reason\":{stop},\"usage\":{{\"input_tokens\":1,\"output_tokens\":1,\"cache_read_input_tokens\":0,\"cache_creation_input_tokens\":0}},\"content\":[]}}"
        ),
    )
}
fn run(lines: &[String]) -> Row {
    let mut row = Row::new([1, 2], 0, now());
    for line in lines {
        let value: Value = serde_json::from_str(line).unwrap();
        row.apply(&Record::from_value(&value, ID, now()).unwrap());
    }
    row
}
fn key(agent: &str) -> String {
    common::sha256(agent.as_bytes())
}
fn status(row: &Row, agent: &str) -> Option<String> {
    row.children.get(&key(agent)).cloned()
}

#[test]
fn children_async_and_sync_launches_and_launch_without_agent_id() {
    let row = run(&[
        launch(10, "agent-a"),
        tool(
            11,
            "{\"agentId\":\"agent-b\",\"totalDurationMs\":900,\"totalTokens\":5}",
        ),
        tool(12, "{\"status\":\"async_launched\"}"),
        tool(13, "{\"status\":\"async_launched\",\"agentId\":null}"),
        // Neither a launch nor a synchronous result: not a child record.
        tool(14, "{\"agentId\":\"agent-c\"}"),
        tool(15, "\"plain string result\""),
    ]);
    assert!(row.valid);
    assert_eq!(row.children.len(), 2);
    assert_eq!(status(&row, "agent-a").as_deref(), Some("running"));
    assert_eq!(status(&row, "agent-b").as_deref(), Some("completed"));
    assert_eq!(row.seq, micros(11));
    // Keys are hashed; no raw id is kept.
    assert!(row.children.keys().all(|key| common::hex_id(key, 64)));
    // A repeat launch of a known child returns it to running.
    let again = run(&[
        launch(10, "agent-a"),
        notified(20, "agent-a", "completed"),
        launch(30, "agent-a"),
    ]);
    assert_eq!(status(&again, "agent-a").as_deref(), Some("running"));
    assert_eq!(again.children.len(), 1);
}

#[test]
fn children_resume_known_child_and_ignore_unknown_child() {
    let resume = |agent: &str, success: &str| {
        tool(
            40,
            &format!("{{\"resumedAgentId\":\"{agent}\",\"success\":{success}}}"),
        )
    };
    let base = [launch(10, "agent-a"), notified(20, "agent-a", "completed")];
    let resumed = run(&[base[0].clone(), base[1].clone(), resume("agent-a", "true")]);
    assert_eq!(status(&resumed, "agent-a").as_deref(), Some("running"));
    assert_eq!(resumed.seq, micros(40));
    for (agent, success) in [
        ("agent-a", "false"),
        ("agent-a", "\"true\""),
        ("agent-z", "true"),
    ] {
        let row = run(&[base[0].clone(), base[1].clone(), resume(agent, success)]);
        assert!(row.valid, "{agent} {success}");
        assert_eq!(row.children.len(), 1);
        assert_eq!(status(&row, "agent-a").as_deref(), Some("completed"));
        assert_eq!(row.seq, micros(20));
    }
}

#[test]
fn children_notifications_through_origin_command_mode_and_without_origin() {
    let done = |line: String| {
        let row = run(&[launch(10, "agent-a"), line]);
        assert!(row.valid);
        status(&row, "agent-a")
    };
    // Through `origin.kind` on a user record.
    assert_eq!(
        done(notified(20, "agent-a", "completed")).as_deref(),
        Some("completed")
    );
    // Through `commandMode` on a queued command, which carries no origin.
    let attached = queued(20, "task-notification", &notice("agent-a", "completed"));
    assert!(!attached.contains("origin"));
    assert_eq!(done(attached).as_deref(), Some("completed"));
    // The same tag block in a user record without origin, or in a queued
    // prompt, is not a notification.
    assert_eq!(
        done(user(20, &notice("agent-a", "completed"), "")).as_deref(),
        Some("running")
    );
    let prompt = queued(20, "prompt", &notice("agent-a", "completed"));
    assert_eq!(done(prompt).as_deref(), Some("running"));
    // Notifications for shell tasks, workflows or teammates are ignored.
    let other = run(&[
        launch(10, "agent-a"),
        notified(20, "shell-task-1", "failed"),
    ]);
    assert!(other.valid);
    assert_eq!(other.children.len(), 1);
    assert_eq!(other.seq, micros(10));
    // A notification whose task id cannot be read may name a known child.
    for text in [
        "<task-notification><status>completed</status></task-notification>".to_owned(),
        notice("bad id", "completed"),
        format!("lead {}", notice("agent-a", "completed")),
        format!(
            "<task-notification><summary>{}</summary><task-id>agent-a</task-id>",
            "x".repeat(text::LIMIT)
        ),
    ] {
        let row = run(&[
            launch(10, "agent-a"),
            user(20, &text, "\"origin\":{\"kind\":\"task-notification\"}"),
        ]);
        assert!(!row.valid, "{text:.80}");
    }
}

#[test]
fn children_status_mapping_partitions_every_status() {
    for (raw, expected) in [
        ("completed", "completed"),
        ("failed", "errored"),
        ("killed", "interrupted"),
        ("blocked", "unknown"),
        ("paused", "unknown"),
        ("", "unknown"),
    ] {
        let row = run(&[launch(10, "agent-a"), notified(20, "agent-a", raw)]);
        assert_eq!(status(&row, "agent-a").as_deref(), Some(expected), "{raw}");
        assert_eq!(row.seq, micros(20));
    }
    // A notification without a status tag leaves the outcome unknown.
    let row = run(&[
        launch(10, "agent-a"),
        user(
            20,
            "<task-notification><task-id>agent-a</task-id></task-notification>",
            "\"origin\":{\"kind\":\"task-notification\"}",
        ),
    ]);
    assert_eq!(status(&row, "agent-a").as_deref(), Some("unknown"));
}

#[test]
fn children_cap_and_malformed_ids_invalidate() {
    let mut lines: Vec<String> = (0..128)
        .map(|n| launch(10, &format!("agent-{n}")))
        .collect();
    let full = run(&lines);
    assert!(full.valid);
    assert_eq!(full.children.len(), 128);
    // Updating a known child at the cap is fine; a new one is not.
    lines.push(notified(20, "agent-7", "completed"));
    assert!(run(&lines).valid);
    lines.push(launch(30, "agent-128"));
    let over = run(&lines);
    assert!(!over.valid);
    assert_eq!(over.children.len(), 128);
    for result in [
        "{\"status\":\"async_launched\",\"agentId\":\"bad id\"}",
        "{\"status\":\"async_launched\",\"agentId\":7}",
        "{\"agentId\":\"\",\"totalDurationMs\":5}",
        "{\"resumedAgentId\":\"bad/id\",\"success\":true}",
        "{\"resumedAgentId\":[],\"success\":false}",
    ] {
        assert!(!run(&[tool(10, result)]).valid, "{result}");
    }
    // A malformed id outside a recognised child shape is not a child record.
    assert!(run(&[tool(10, "{\"agentId\":\"bad id\"}")]).valid);
    // An accepted child record needs a validated timestamp.
    let undated = launch(10, "agent-a").replace(&stamp(10), "not a time");
    assert!(!run(&[undated]).valid);
}

#[test]
fn children_status_seq_is_the_largest_accepted_stamp_else_coverage() {
    let none = run(&[
        user(50, "hello", ""),
        assistant(60, "msg_a", "\"end_turn\""),
    ]);
    assert_eq!((none.seq, none.status_seq()), (0, micros(60)));
    // File order is not time order: the largest accepted stamp wins.
    let row = run(&[
        launch(30, "agent-a"),
        launch(20, "agent-b"),
        notified(25, "agent-b", "completed"),
        // Ignored records never move the stamp.
        notified(90, "shell-task", "completed"),
        tool(95, "{\"resumedAgentId\":\"agent-z\",\"success\":true}"),
    ]);
    assert_eq!(row.seq, micros(30));
    assert_eq!(row.status_seq(), micros(30));
    assert_eq!(row.claude.coverage_seq, micros(95));
}

#[test]
fn children_ignore_forked_and_foreign_records() {
    let forked = launch(10, "agent-a").replace("\"type\"", "\"forkedFrom\":{\"x\":1},\"type\"");
    let row = run(&[forked]);
    assert!(row.valid && row.children.is_empty());
    let foreign = launch(10, "agent-a").replace(ID, "fixture-session-b");
    let row = run(&[foreign]);
    assert!(!row.valid && row.children.is_empty());
}
