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
fn system(subtype: &str, second: u64) -> String {
    record("system", second, &format!("\"subtype\":\"{subtype}\""))
}
/// A `queue-operation` record; only `enqueue` is written when input is typed.
fn queue(second: u64) -> String {
    operation(second, "enqueue")
}
fn operation(second: u64, operation: &str) -> String {
    record(
        "queue-operation",
        second,
        &format!("\"operation\":\"{operation}\""),
    )
}
/// The queued input is taken into the running turn.
fn dequeue(second: u64) -> String {
    operation(second, "dequeue")
}
/// Replays `lines`, checking the D8 turn invariants after every record.
fn run(lines: &[String]) -> Row {
    let mut row = Row::new([1, 2], 0, now());
    for line in lines {
        let value: Value = serde_json::from_str(line).unwrap();
        row.apply(&Record::from_value(&value, ID, now()).unwrap());
        assert!(row.turns.validate() && row.claude.validate(now()), "{line}");
        assert!(row.turn_state(), "{line}");
    }
    row
}
fn second(at: u64) -> u64 {
    BASE + at
}
/// The finished interval of the turn opened by `kind` at `at`.
fn finished(row: &Row, kind: &str, at: u64) -> Option<(u64, u64, String)> {
    let key = turn_key(ID, &format!("{kind}-{at}"));
    row.turns.finished.get(&key).cloned()
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

#[test]
fn turns_trigger_by_origin_and_by_shape() {
    let origin = |kind: &str| format!("\"origin\":{{\"kind\":\"{kind}\"}}");
    let triggers = [
        user(10, "hello", &origin("human")),
        user(
            10,
            &notice("shell-1", "completed"),
            &origin("task-notification"),
        ),
        user(
            10,
            "peer message",
            &format!("{},\"isMeta\":true", origin("peer")),
        ),
        user(10, "coordinate", &origin("coordinator")),
        // Rule 2 precedes rule 3: a recognised origin wins over tool results.
        user(
            10,
            "hello",
            &format!("{},\"toolUseResult\":{{}}", origin("human")),
        ),
        queued(10, "task-notification", &notice("shell-1", "failed")),
        // Rule 4: text without origin, a slash-command echo, any other tag.
        user(10, "plain prompt", ""),
        user(10, "<command-name>/review</command-name>", ""),
        user(10, "<command-message>review</command-message>", ""),
        user(10, &notice("shell-1", "completed"), ""),
    ];
    for trigger in triggers {
        let kind = if trigger.contains("\"attachment\"") {
            "attachment"
        } else {
            "user"
        };
        let row = run(&[
            trigger.clone(),
            assistant(12, "msg_a", "\"end_turn\""),
            system("turn_duration", 20),
        ]);
        assert!(row.turns.valid && row.turns.supported, "{trigger}");
        assert_eq!(
            finished(&row, kind, 10),
            Some((second(10), second(20), "completed".into())),
            "{trigger}"
        );
        assert_eq!(row.turns.total, 10);
    }
    // A trigger alone sets `supported`; nothing is active until confirmed.
    let pending = run(&[user(10, "hello", "")]);
    assert!(pending.turns.supported && pending.turns.active.is_none());
    assert_eq!(
        pending.claude.pending_start,
        Some((turn_key(ID, "user-10"), second(10)))
    );
}

#[test]
fn turns_ignore_metadata_tool_results_and_wrapper_output() {
    let ignored = [
        user(
            10,
            "<local-command-caveat>x</local-command-caveat>",
            "\"isMeta\":true",
        ),
        tool(10, "{\"status\":\"async_launched\"}"),
        user(10, "summary", "\"isCompactSummary\":true"),
        queued(10, "prompt", "queued human input"),
        user(10, "<local-command-stdout>ok</local-command-stdout>", ""),
        user(10, "<local-command-stderr>no</local-command-stderr>", ""),
        user(10, "<bash-input>ls</bash-input>", ""),
        user(10, "<bash-stdout>a</bash-stdout>", ""),
        user(10, "<bash-stderr>b</bash-stderr>", ""),
        system("compact_boundary", 10),
        system("microcompact_boundary", 10),
        record(
            "user",
            10,
            "\"message\":{\"role\":\"user\",\"content\":[{\"type\":\"image\"}]}",
        ),
        record("attachment", 10, "\"attachment\":{\"type\":\"fixture\"}"),
    ];
    for line in &ignored {
        let row = run(&[line.clone(), assistant(12, "msg_a", "\"end_turn\"")]);
        assert!(row.turns.valid && !row.turns.supported, "{line}");
        assert!(row.turns.active.is_none() && row.claude.pending_start.is_none());
        // Inside a turn they neither end it nor start another.
        let row = run(&[
            user(5, "hello", ""),
            assistant(6, "msg_a", "\"tool_use\""),
            line.clone(),
            assistant(12, "msg_b", "\"end_turn\""),
            system("turn_duration", 20),
        ]);
        assert!(row.turns.valid, "{line}");
        assert_eq!(row.turns.finished.len(), 1);
        assert_eq!(
            finished(&row, "user", 5),
            Some((second(5), second(20), "completed".into()))
        );
    }
    // An unrecognised origin is ambiguous and fails closed.
    let other = run(&[
        user(5, "hello", ""),
        assistant(6, "msg_a", "\"end_turn\""),
        user(10, "x", "\"origin\":{\"kind\":\"scheduler\"}"),
    ]);
    assert!(!other.turns.valid && other.turns.active.is_none());
}

#[test]
fn turns_pending_start_needs_an_assistant_record_including_synthetic() {
    // A newer trigger replaces an unconfirmed one.
    let row = run(&[
        user(10, "first", ""),
        user(12, "second", ""),
        assistant(13, "msg_a", "\"end_turn\""),
        system("turn_duration", 20),
    ]);
    assert_eq!(finished(&row, "user", 10), None);
    assert_eq!(
        finished(&row, "user", 12),
        Some((second(12), second(20), "completed".into()))
    );
    // A `<synthetic>` error record confirms the start but feeds no usage.
    let synthetic =
        assistant(13, "msg_a", "\"stop_sequence\"").replace("claude-fixture-1", "<synthetic>");
    let row = run(&[
        user(10, "hello", ""),
        synthetic,
        system("turn_duration", 20),
    ]);
    assert_eq!(
        finished(&row, "user", 10),
        Some((second(10), second(20), "completed".into()))
    );
    assert_eq!(row.usage()["usage_seq"], Value::Null);
    // Confirmation restores `current_known` and starts at the trigger's second.
    let row = run(&[
        user(10, "hello", ""),
        assistant(15, "msg_a", "\"tool_use\""),
    ]);
    assert!(row.turns.current_known);
    assert_eq!(row.turns.active, Some(turn_key(ID, "user-10")));
    assert_eq!(row.turns.start, Some(second(10)));
    assert!(row.claude.pending_start.is_none());
}

#[test]
fn turns_abort_during_pending_start_confirms_and_aborts() {
    let mid_stream =
        assistant(15, "msg_a", "null").replace("\"type\"", "\"isAbortedMidStream\":true,\"type\"");
    for abort in [
        user(15, "[Request interrupted by user]", ""),
        user(15, "[Request interrupted by user for tool use]", ""),
        record("user", 15, "\"interruptedMessageId\":\"msg_a\""),
        record("attachment", 15, "\"interruptedMessageId\":\"msg_a\""),
        mid_stream,
    ] {
        let row = run(&[user(10, "hello", ""), abort.clone()]);
        assert!(row.turns.valid, "{abort}");
        assert_eq!(
            finished(&row, "user", 10),
            Some((second(10), second(15), "aborted".into())),
            "{abort}"
        );
        // The same abort ends an active turn.
        let row = run(&[
            user(10, "hello", ""),
            assistant(11, "msg_b", "\"tool_use\""),
            abort.clone(),
        ]);
        assert_eq!(
            finished(&row, "user", 10),
            Some((second(10), second(15), "aborted".into())),
            "{abort}"
        );
        assert_eq!(row.turns.last_outcome.as_deref(), Some("aborted"));
    }
    // Abort precedes the ignore rules: a marker carried by a meta record.
    let row = run(&[
        user(10, "hello", ""),
        user(15, "[Request interrupted by user]", "\"isMeta\":true"),
    ]);
    assert_eq!(
        finished(&row, "user", 10).map(|t| t.2),
        Some("aborted".into())
    );
}

#[test]
fn turns_orphan_abort_and_orphan_turn_duration() {
    let marker = |at| user(at, "[Request interrupted by user]", "");
    // An orphan abort is ignored apart from the adjacency flag.
    let row = run(&[marker(5)]);
    assert!(row.turns.valid && row.claude.abort_adjacent);
    assert!(row.turns.finished.is_empty());
    // A `turn_duration` directly after an abort is ignored.
    for lines in [
        vec![marker(5), system("turn_duration", 6)],
        vec![
            marker(5),
            record("attachment", 6, "\"attachment\":{}"),
            system("turn_duration", 7),
        ],
        vec![
            user(1, "hello", ""),
            assistant(2, "msg_a", "\"tool_use\""),
            marker(5),
            system("turn_duration", 6),
        ],
    ] {
        let row = run(&lines);
        assert!(row.turns.valid && row.turns.supported, "{lines:?}");
        assert!(!row.claude.abort_adjacent);
    }
    // Otherwise it makes accumulated coverage unknown.
    for lines in [
        vec![system("turn_duration", 6)],
        vec![
            marker(5),
            assistant(6, "msg_a", "\"end_turn\""),
            system("turn_duration", 7),
        ],
        vec![marker(5), user(6, "hello", ""), system("turn_duration", 7)],
        vec![
            marker(5),
            system("turn_duration", 6),
            system("turn_duration", 7),
        ],
        vec![
            user(1, "hello", ""),
            assistant(2, "msg_a", "\"end_turn\""),
            system("turn_duration", 3),
            system("turn_duration", 4),
        ],
    ] {
        let row = run(&lines);
        assert!(!row.turns.valid && row.turns.supported, "{lines:?}");
    }
}

#[test]
fn turns_queued_input_joins_only_after_a_queue_operation() {
    let joined = run(&[
        user(10, "hello", ""),
        assistant(11, "msg_a", "\"tool_use\""),
        queue(12),
        dequeue(12),
        user(13, "also this", ""),
        assistant(14, "msg_b", "\"end_turn\""),
        system("turn_duration", 20),
    ]);
    assert!(joined.turns.valid);
    assert_eq!(joined.turns.finished.len(), 1);
    assert_eq!(
        finished(&joined, "user", 10),
        Some((second(10), second(20), "completed".into()))
    );
    // Without one the gap is never absorbed: coverage becomes unknown, and
    // the ambiguous trigger opens no pending start.
    let row = run(&[
        user(10, "hello", ""),
        assistant(11, "msg_a", "\"tool_use\""),
        user(13, "also this", ""),
    ]);
    assert!(!row.turns.valid && row.turns.active.is_none());
    assert!(row.claude.ambiguous && row.claude.pending_start.is_none());
    let row = run(&[
        user(10, "hello", ""),
        assistant(11, "msg_a", "\"tool_use\""),
        user(13, "also this", ""),
        assistant(14, "msg_b", "\"end_turn\""),
        system("turn_duration", 20),
    ]);
    // Its interval is never published: the start at 13 is a guess.
    assert!(!row.turns.valid && !row.turns.current_known);
    assert!(row.turns.last.is_none() && !row.claude.ambiguous);
    // A queue operation outside a turn, or before the turn started, is stale.
    for early in [
        vec![queue(5)],
        vec![dequeue(5)],
        vec![user(4, "x", ""), queue(5)],
        vec![user(4, "x", ""), dequeue(5)],
    ] {
        let mut lines = early;
        lines.extend([
            user(10, "hello", ""),
            assistant(11, "msg_a", "\"tool_use\""),
            user(13, "also this", ""),
        ]);
        assert!(!run(&lines).turns.valid);
    }
}

#[test]
fn turns_silent_end_followed_by_queued_input_is_unknown() {
    for silent in [
        assistant(15, "msg_b", "\"end_turn\""),
        system("stop_hook_summary", 15),
    ] {
        let row = run(&[
            user(10, "hello", ""),
            assistant(11, "msg_a", "\"tool_use\""),
            dequeue(12),
            silent.clone(),
            user(500, "much later", ""),
        ]);
        assert!(!row.turns.valid, "{silent}");
        assert!(row.turns.active.is_none() && row.turns.finished.is_empty());
        assert!(row.claude.ambiguous && row.claude.pending_start.is_none());
    }
    // A tool-use stop is not a silent end: the queued input still joins.
    let row = run(&[
        user(10, "hello", ""),
        assistant(11, "msg_a", "\"tool_use\""),
        dequeue(12),
        assistant(13, "msg_b", "\"tool_use\""),
        user(14, "queued", ""),
        system("turn_duration", 20),
    ]);
    assert!(row.turns.valid);
    assert_eq!(finished(&row, "user", 10).map(|t| t.1), Some(second(20)));
    // A queue operation after the silent end lets input join again.
    let row = run(&[
        user(10, "hello", ""),
        assistant(11, "msg_a", "\"end_turn\""),
        dequeue(12),
        user(13, "queued", ""),
        system("turn_duration", 20),
    ]);
    assert!(row.turns.valid && row.turns.finished.len() == 1);
}

#[test]
fn turns_floor_seconds_and_fail_closed_on_overlap_or_reversal() {
    // Both bounds floor to the second: x.250 -> x.
    let row = run(&[
        user(10, "hello", ""),
        assistant(11, "msg_a", "\"end_turn\""),
        system("turn_duration", 20),
    ]);
    assert_eq!(
        row.turns.finished.values().next().map(|t| (t.0, t.1)),
        Some((second(10), second(20)))
    );
    // Non-monotonic records inside a turn are fine.
    let row = run(&[
        user(10, "hello", ""),
        assistant(9, "msg_a", "\"tool_use\""),
        tool(8, "{}"),
        assistant(12, "msg_b", "\"end_turn\""),
        system("turn_duration", 20),
    ]);
    assert!(row.turns.valid && row.turns.total == 10);
    // A start before the previous end makes coverage unknown.
    let row = run(&[
        user(10, "hello", ""),
        assistant(11, "msg_a", "\"end_turn\""),
        system("turn_duration", 20),
        user(15, "again", ""),
        assistant(16, "msg_b", "\"end_turn\""),
        system("turn_duration", 25),
    ]);
    assert!(!row.turns.valid && row.turns.active.is_none());
    // The last valid interval stays available.
    assert_eq!(row.turns.last_end, Some(second(20)));
    // An end before its start makes coverage unknown.
    let row = run(&[
        user(10, "hello", ""),
        assistant(11, "msg_a", "\"end_turn\""),
        system("turn_duration", 5),
    ]);
    assert!(!row.turns.valid && row.turns.finished.is_empty());
    // A trigger or end without a validated timestamp fails closed.
    let undated = user(10, "hello", "").replace(&stamp(10), "x");
    assert!(!run(&[undated]).turns.valid);
    let undated = system("turn_duration", 20).replace(&stamp(20), "x");
    let row = run(&[
        user(10, "hello", ""),
        assistant(11, "msg_a", "\"end_turn\""),
        undated,
    ]);
    assert!(!row.turns.valid);
}

#[test]
fn turns_state_survives_a_block_round_trip() {
    let row = run(&[
        user(10, "hello", ""),
        assistant(11, "msg_a", "\"tool_use\""),
        dequeue(12),
    ]);
    assert!(row.claude.queued_since_start);
    let block: ClaudeCursor =
        serde_json::from_value(serde_json::to_value(&row.claude).unwrap()).unwrap();
    assert_eq!(block, row.claude);
    let pending = run(&[
        user(10, "hello", ""),
        user(11, "[Request interrupted by user]", ""),
    ]);
    assert!(pending.claude.abort_adjacent);
    let pending = run(&[user(30, "hello", "")]);
    let block = serde_json::to_value(&pending.claude).unwrap();
    assert_eq!(block["pending_start"][1], json!(second(30)));
    let mut tampered = block.clone();
    tampered["pending_start"][0] = json!("not-a-key");
    let tampered: ClaudeCursor = serde_json::from_value(tampered).unwrap();
    assert!(!tampered.validate(now()));
}

#[test]
fn turns_block_inconsistent_with_turns_is_rejected() {
    let mut queued = run(&[
        user(10, "hello", ""),
        assistant(11, "msg_a", "\"tool_use\""),
        dequeue(12),
    ]);
    assert!(queued.turn_state());
    queued.turns.unknown();
    assert!(!queued.turn_state());
    let mut pending = run(&[
        user(10, "hello", ""),
        assistant(11, "msg_a", "\"tool_use\""),
    ]);
    pending.claude.pending_start = Some((turn_key(ID, "user-20"), second(20)));
    assert!(!pending.turn_state());
    // Ambiguity holds no active turn or pending start, with coverage unknown.
    let mut ambiguous = run(&[
        user(10, "hello", ""),
        assistant(11, "msg_a", "\"tool_use\""),
    ]);
    ambiguous.claude.ambiguous = true;
    assert!(!ambiguous.turn_state());
    ambiguous.turns.unknown();
    assert!(ambiguous.turn_state());
    ambiguous.claude.pending_start = Some((turn_key(ID, "user-20"), second(20)));
    assert!(!ambiguous.turn_state());
}

#[test]
fn turns_trigger_at_unix_second_zero_is_a_missing_stamp() {
    let epoch = user(10, "hello", "").replace(&stamp(10), "1970-01-01T00:00:00.500Z");
    let mut row = Row::new([1, 2], 0, now());
    let value: Value = serde_json::from_str(&epoch).unwrap();
    let record = Record::from_value(&value, ID, now()).unwrap();
    assert_eq!(record.stamp, Some(500_000));
    row.apply(&record);
    assert_eq!(row.claude.pending_start, None);
    assert!(!row.turns.valid && row.turns.supported);
    assert!(row.claude.validate(now()) && row.turn_state());
    // The same trigger during an active turn also fails closed.
    let mut lines = vec![user(1, "hello", ""), assistant(2, "msg_a", "\"tool_use\"")];
    lines.push(epoch);
    let row = run(&lines);
    assert!(!row.turns.valid && row.claude.pending_start.is_none());
}

#[test]
fn turns_join_needs_a_dequeue_or_remove_and_each_join_consumes_it() {
    // A killed turn writes no end record: an enqueue alone never lets the
    // resumed session's prompt absorb the idle gap.
    let row = run(&[
        user(10, "hello", ""),
        assistant(11, "msg_a", "\"tool_use\""),
        queue(12),
        user(10812, "resumed later", ""),
        assistant(10813, "msg_b", "\"end_turn\""),
        system("turn_duration", 10820),
    ]);
    assert!(!row.turns.valid);
    assert!(finished(&row, "user", 10).is_none());
    // A dequeue or remove before the first assistant line still counts.
    for taken in ["dequeue", "remove"] {
        let row = run(&[
            user(1, "hello", ""),
            queue(2),
            operation(2, taken),
            assistant(3, "msg_a", "\"tool_use\""),
            user(5, "queued", ""),
            assistant(6, "msg_b", "\"end_turn\""),
            system("turn_duration", 7),
        ]);
        assert!(row.turns.valid, "{taken}");
        assert_eq!(row.turns.total, 6);
        assert_eq!(
            finished(&row, "user", 1),
            Some((second(1), second(7), "completed".into()))
        );
    }
    // A trigger joins a pending start it was dequeued into.
    let row = run(&[
        user(4, "hello", ""),
        dequeue(5),
        user(6, "queued", ""),
        assistant(7, "msg_a", "\"end_turn\""),
        system("turn_duration", 9),
    ]);
    assert!(row.turns.valid && !row.claude.queued_since_start);
    assert_eq!(finished(&row, "user", 4).map(|t| t.1), Some(second(9)));
    // Each join consumes the evidence: a second prompt needs its own.
    let row = run(&[
        user(10, "hello", ""),
        assistant(11, "msg_a", "\"tool_use\""),
        dequeue(12),
        user(13, "queued", ""),
        user(14, "queued again", ""),
    ]);
    assert!(!row.turns.valid);
    // An unknown operation is not evidence either.
    let row = run(&[
        user(10, "hello", ""),
        assistant(11, "msg_a", "\"tool_use\""),
        operation(12, "popAll"),
        user(13, "queued", ""),
    ]);
    assert!(!row.turns.valid);
}

#[test]
fn turns_after_an_unjoined_trigger_publish_nothing_until_a_proven_end() {
    let earlier = [
        user(10, "hello", ""),
        assistant(11, "msg_a", "\"end_turn\""),
        system("turn_duration", 20),
        user(31, "next", ""),
        assistant(32, "msg_b", "\"tool_use\""),
        // Input injected into the running turn without queue evidence.
        notified(33, "agent-x", "completed"),
        assistant(36, "msg_c", "\"end_turn\""),
    ];
    let row = run(&earlier);
    assert!(!row.turns.valid && !row.turns.current_known);
    assert!(row.turns.active.is_none() && row.claude.pending_start.is_none());
    let mut lines = earlier.to_vec();
    lines.push(system("turn_duration", 38));
    let row = run(&lines);
    // The truncated 33..38 interval is never current or last.
    assert!(!row.turns.current_known);
    assert_eq!(
        row.turns.last.as_deref(),
        Some(turn_key(ID, "user-10").as_str())
    );
    assert_eq!(row.turns.last_duration, Some(10));
    // A trigger after that proven end publishes again.
    lines.extend([
        user(40, "again", ""),
        assistant(41, "msg_d", "\"end_turn\""),
        system("turn_duration", 45),
    ]);
    let row = run(&lines);
    assert!(row.turns.current_known && !row.turns.valid);
    assert_eq!(
        row.turns.last.as_deref(),
        Some(turn_key(ID, "user-40").as_str())
    );
    assert_eq!(row.turns.last_duration, Some(5));
    // An abort is a proven end too; a later trigger alone is not.
    let mut lines = earlier[..6].to_vec();
    lines.extend([
        user(34, "more", ""),
        assistant(35, "msg_c", "\"tool_use\""),
        user(36, "[Request interrupted by user]", ""),
        user(40, "again", ""),
        assistant(41, "msg_d", "\"tool_use\""),
    ]);
    let row = run(&lines);
    assert!(row.turns.current_known);
    assert_eq!(
        row.turns.active.as_deref(),
        Some(turn_key(ID, "user-40").as_str())
    );
    assert_eq!(row.turns.last_duration, Some(10));
}
