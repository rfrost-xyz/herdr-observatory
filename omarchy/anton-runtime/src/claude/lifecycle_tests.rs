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
/// A `turn_duration` at `second` whose `durationMs` dates its turn from `from`.
fn ended(second: u64, from: u64) -> String {
    let elapsed = (second - from) * 1000;
    record(
        "system",
        second,
        &format!("\"subtype\":\"turn_duration\",\"durationMs\":{elapsed}"),
    )
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
/// Fixtures written before the `durationMs` gate (D7) leave `durationMs` out
/// of `turn_duration`: it is filled in to agree with the open start, so they
/// keep testing their own rule. Gate fixtures write it themselves or replay
/// through `run_exact`.
fn timed(row: &Row, mut value: Value) -> Value {
    let duration = value["type"] == "system" && value["subtype"] == "turn_duration";
    if duration && value.get("durationMs").is_none() {
        let stamp = Record::from_value(&value, ID, now()).and_then(|record| record.stamp);
        let elapsed = match (row.turns.start, stamp) {
            (Some(start), Some(stamp)) => (stamp / 1000).saturating_sub(start * 1000),
            _ => 0,
        };
        value["durationMs"] = elapsed.into();
    }
    value
}
/// Replays `lines`, checking the D8 turn invariants after every record.
fn run(lines: &[String]) -> Row {
    replay_lines(lines, true)
}
/// As `run`, with every line applied unchanged.
fn run_exact(lines: &[String]) -> Row {
    replay_lines(lines, false)
}
fn replay_lines(lines: &[String], fill: bool) -> Row {
    let mut row = Row::new([1, 2], 0, now());
    for line in lines {
        let mut value: Value = serde_json::from_str(line).unwrap();
        if fill {
            value = timed(&row, value);
        }
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
    // Review round 8: content after the first block, such as a second block
    // naming another child, is unreadable, as text before it is.
    let two = format!(
        "{}\n{}",
        notice("agent-b", "completed"),
        notice("agent-a", "completed")
    );
    let array = format!(
        "[{{\"type\":\"text\",\"text\":{}}},{{\"type\":\"text\",\"text\":\"more\"}}]",
        serde_json::to_string(&notice("agent-a", "completed")).unwrap()
    );
    for line in [
        user(20, &two, "\"origin\":{\"kind\":\"task-notification\"}"),
        queued(20, "task-notification", &two),
        record(
            "user",
            20,
            &format!(
                "\"origin\":{{\"kind\":\"task-notification\"}},\"message\":{{\"role\":\"user\",\"content\":{array}}}"
            ),
        ),
    ] {
        let row = run(&[launch(10, "agent-a"), launch(11, "agent-b"), line.clone()]);
        assert!(!row.valid, "{line}");
        assert_eq!(status(&row, "agent-a").as_deref(), Some("running"));
    }
    let trailing = format!("{}\n\n", notice("agent-a", "completed"));
    assert_eq!(
        done(user(
            20,
            &trailing,
            "\"origin\":{\"kind\":\"task-notification\"}"
        ))
        .as_deref(),
        Some("completed")
    );
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
        user(10, "<local-command-stdout>ok</local-command-stdout>", ""),
        user(10, "<local-command-stderr>no</local-command-stderr>", ""),
        user(10, "<bash-input>ls</bash-input>", ""),
        user(10, "<bash-stdout>a</bash-stdout>", ""),
        user(10, "<bash-stderr>b</bash-stderr>", ""),
        system("compact_boundary", 10),
        system("microcompact_boundary", 10),
        record("attachment", 10, "\"attachment\":{\"type\":\"fixture\"}"),
    ];
    for line in &ignored {
        let row = run(std::slice::from_ref(line));
        assert!(row.turns.valid && !row.turns.supported, "{line}");
        assert!(row.turns.active.is_none() && row.claude.pending_start.is_none());
        // No start opened, so an assistant record that follows confirms
        // nothing: it shows a turn whose trigger was not seen (D7).
        let row = run(&[line.clone(), assistant(12, "msg_a", "\"end_turn\"")]);
        assert!(!row.turns.valid && !row.turns.supported, "{line}");
        assert!(row.turns.active.is_none() && row.claude.ambiguous);
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

/// Rule 3 ignores queued human input on the assumption that it appears only
/// inside a turn. Outside one, or after a silent end, it may start a turn
/// whose trigger the reader never sees, so it fails closed (review round
/// 12). Inside a running turn it is still ignored.
#[test]
fn turns_queued_prompt_attachment_outside_a_turn_is_ambiguous() {
    let prompt = |at| queued(at, "prompt", "queued human input");
    let row = run(&[prompt(10)]);
    assert!(!row.turns.valid && row.claude.ambiguous);
    let row = run(&[
        user(5, "hello", ""),
        assistant(6, "msg_a", "\"tool_use\""),
        dequeue(7),
        prompt(7),
        assistant(12, "msg_b", "\"end_turn\""),
        system("turn_duration", 20),
    ]);
    assert!(row.turns.valid && !row.claude.ambiguous);
    assert_eq!(
        finished(&row, "user", 5),
        Some((second(5), second(20), "completed".into()))
    );
    let interrupt = |at| user(at, "[Request interrupted by user]", "");
    // A: the input is taken after `turn_duration`, so its prompt starts a
    // turn (13 to 15) that no trigger shows.
    let lines = [
        user(10, "hello", ""),
        assistant(11, "msg_a", "\"end_turn\""),
        queue(11),
        system("turn_duration", 12),
        dequeue(13),
        prompt(11),
    ];
    let row = run(&lines);
    assert!(row.claude.ambiguous && !row.published_turns().current_known);
    let row = run(&[lines.as_slice(), &[interrupt(15)]].concat());
    assert!(!row.turns.valid && !row.published_turns().valid);
    // B: the input is taken after an abort, then its turn runs (13 to 16).
    let lines = [
        user(10, "hello", ""),
        assistant(11, "msg_a", "\"tool_use\""),
        interrupt(12),
        dequeue(13),
        prompt(11),
        assistant(14, "msg_b", "\"tool_use\""),
    ];
    let row = run(&lines);
    assert!(row.claude.ambiguous && !row.published_turns().current_known);
    let row = run(&[lines.as_slice(), &[interrupt(16)]].concat());
    assert!(!row.turns.valid);
    assert_ne!(row.turns.last_end, Some(second(16)));
    // C: an older version writes no `turn_duration`, so the turn that ended
    // silently at 11 must not absorb the idle gap up to the abort at 105.
    let row = run(&[
        user(10, "hello", ""),
        assistant(11, "msg_a", "\"end_turn\""),
        system("stop_hook_summary", 12),
        queue(100),
        dequeue(100),
        prompt(100),
        assistant(101, "msg_b", "\"tool_use\""),
        interrupt(105),
    ]);
    assert!(!row.turns.valid && row.turns.finished.is_empty());
}

/// D7: only an assistant record directly after an abort is neutral. Input
/// taken with no turn running starts a turn, so the abort-adjacency flag is
/// cleared and an assistant record after the take shows a turn whose
/// trigger was not seen (review round 12).
#[test]
fn turns_idle_take_after_an_abort_clears_its_adjacency() {
    let row = run(&[
        user(10, "hello", ""),
        assistant(11, "msg_a", "\"tool_use\""),
        user(12, "[Request interrupted by user]", ""),
        dequeue(13),
    ]);
    assert!(!row.claude.abort_adjacent && !row.claude.ambiguous);
    let row = run(&[
        user(10, "hello", ""),
        assistant(11, "msg_a", "\"tool_use\""),
        user(12, "[Request interrupted by user]", ""),
        dequeue(13),
        assistant(14, "msg_b", "\"tool_use\""),
    ]);
    assert!(row.claude.ambiguous && !row.turns.valid);
}

#[test]
fn turns_pending_start_needs_an_assistant_record_including_synthetic() {
    // Local-command output, wrapped or as a system record, shows an echo was
    // a local command: the next trigger opens a turn normally.
    let echo = user(10, "<command-name>/model</command-name>", "");
    for output in [
        user(11, "<local-command-stdout>ok</local-command-stdout>", ""),
        system("local_command", 11),
    ] {
        let row = run(&[
            echo.clone(),
            output,
            user(12, "second", ""),
            assistant(13, "msg_a", "\"end_turn\""),
            system("turn_duration", 20),
        ]);
        assert!(row.turns.valid && row.claude.pending_start.is_none());
        assert_eq!(finished(&row, "user", 10), None);
        assert_eq!(
            finished(&row, "user", 12),
            Some((second(12), second(20), "completed".into()))
        );
    }
    // The echo is taken to have run no turn, so coverage stays complete, but
    // the command may still be running the model: the current turn is
    // unknown until a trigger, `turn_duration` or an abort.
    let ended = [
        user(1, "hello", ""),
        assistant(2, "msg_a", "\"end_turn\""),
        system("turn_duration", 3),
        echo.clone(),
        user(11, "<local-command-stdout>ok</local-command-stdout>", ""),
    ];
    let row = run(&ended);
    assert!(!row.claude.lost_idle && !row.claude.pending_command);
    assert!(row.claude.local_idle && row.turns.current_known);
    let published = row.published_turns();
    assert!(published.valid && !published.current_known && published.active.is_none());
    let block: ClaudeCursor =
        serde_json::from_value(serde_json::to_value(&row.claude).unwrap()).unwrap();
    assert!(block.local_idle);
    // A `turn_duration` that ended no turn sets it again (review round 13).
    for (next, valid, local) in [
        (user(20, "second", ""), true, false),
        (system("turn_duration", 20), false, true),
        (user(20, "[Request interrupted by user]", ""), false, false),
    ] {
        let mut lines = ended.to_vec();
        lines.push(next.clone());
        let row = run(&lines);
        assert!(
            row.claude.local_idle == local && row.turns.valid == valid,
            "{next}"
        );
    }
    // Local output while a prompt awaits its first response comes from a
    // local command run meanwhile: the prompt's start stays pending, so the
    // assistant record confirms it and a notification injected into the
    // running turn without queue evidence is ambiguous.
    let row = run(&[
        user(10, "hello", ""),
        system("local_command", 11),
        assistant(13, "msg_a", "\"tool_use\""),
        queued(14, "task-notification", &notice("agent-x", "completed")),
        assistant(15, "msg_b", "\"tool_use\""),
    ]);
    assert!(row.claude.ambiguous && !row.turns.current_known);
    // Found by the ground-truth fuzzer: any other unconfirmed start may be a
    // turn killed before its first assistant record, or a running turn the
    // newer trigger joined, so no interval is published until a proven end.
    for first in [user(10, "first", ""), echo] {
        let row = run(&[
            first.clone(),
            user(12, "second", ""),
            assistant(13, "msg_a", "\"end_turn\""),
        ]);
        assert!(row.claude.ambiguous && !row.turns.current_known, "{first}");
        let row = run(&[
            first,
            user(12, "second", ""),
            assistant(13, "msg_a", "\"end_turn\""),
            system("turn_duration", 20),
        ]);
        assert!(!row.turns.valid && row.turns.last.is_none());
        assert!(row.turns.finished.is_empty() && !row.claude.ambiguous);
    }
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

/// After a slash command's local output the command may still be running
/// the model. A task notification, peer or coordinator message can enter
/// that turn without queue evidence, so it is ambiguous (review round 11).
/// Human and shape prompts are queued and taken, so they still open a turn.
#[test]
fn turns_injected_trigger_after_local_output_is_ambiguous() {
    let note = notice("agent-y", "completed");
    let lines = |trigger: String| {
        vec![
            user(1, "hello", ""),
            assistant(2, "msg_a", "\"end_turn\""),
            system("turn_duration", 3),
            user(10, "<command-name>/review</command-name>", ""),
            user(11, "<local-command-stdout>ok</local-command-stdout>", ""),
            trigger,
            assistant(14, "msg_b", "\"tool_use\""),
            assistant(19, "msg_c", "\"end_turn\""),
            system("turn_duration", 20),
        ]
    };
    for trigger in [
        user(12, &note, "\"origin\":{\"kind\":\"task-notification\"}"),
        queued(12, "task-notification", &note),
        user(
            12,
            "synthetic",
            "\"isMeta\":true,\"origin\":{\"kind\":\"peer\"}",
        ),
        user(12, "synthetic", "\"origin\":{\"kind\":\"coordinator\"}"),
    ] {
        let all = lines(trigger.clone());
        // The command turn runs 10..20: no start at 12 is ever published.
        let row = run(&all[..7]);
        let published = row.published_turns();
        assert!(!published.current_known, "{trigger}");
        assert!(row.claude.ambiguous && !row.claude.local_idle, "{trigger}");
        let row = run(&all);
        assert!(!row.turns.valid && !row.claude.ambiguous, "{trigger}");
        assert_eq!(row.turns.last_duration, Some(2), "{trigger}");
        assert!(row.turns.finished.len() == 1, "{trigger}");
    }
    for trigger in [
        user(12, "second", ""),
        user(12, "second", "\"origin\":{\"kind\":\"human\"}"),
    ] {
        let row = run(&lines(trigger.clone()));
        assert!(row.turns.valid, "{trigger}");
        assert_eq!(
            finished(&row, "user", 12),
            Some((second(12), second(20), "completed".into())),
            "{trigger}"
        );
    }
}

#[test]
fn turns_local_output_never_clears_a_pending_prompt() {
    // Review round 5: local output cleared any pending start without making
    // coverage unknown, so the prompt's turn was left out of the total.
    let head = |output: &str| {
        vec![
            user(1, "hello", ""),
            assistant(2, "msg_a", "\"end_turn\""),
            system("turn_duration", 3),
            user(10, "prompt", ""),
            user(11, output, ""),
        ]
    };
    for output in [
        "<local-command-stdout>ok</local-command-stdout>",
        "<local-command-stderr>no</local-command-stderr>",
    ] {
        let row = run(&head(output));
        assert_eq!(
            row.claude.pending_start,
            Some((turn_key(ID, "user-10"), second(10)))
        );
        assert!(!row.claude.lost_idle && !row.published_turns().current_known);
        // A notification before the first assistant record may be input to
        // the prompt's turn: never a start of its own.
        let mut lines = head(output);
        lines.extend([
            queued(12, "task-notification", &notice("agent-y", "completed")),
            assistant(13, "msg_b", "\"tool_use\""),
            assistant(14, "msg_c", "\"end_turn\""),
            system("turn_duration", 20),
        ]);
        let published = run(&lines).published_turns();
        assert!(!published.valid && !published.current_known);
        assert_eq!(published.last_duration, Some(2));
        // An abort confirms the prompt's start and ends it as aborted.
        let mut lines = head(output);
        lines.push(user(15, "[Request interrupted by user]", ""));
        let row = run(&lines);
        // An aborted interval makes accumulated coverage unknown (D7).
        assert!(!row.turns.valid && row.turns.total == 7);
        assert_eq!(
            finished(&row, "user", 10),
            Some((second(10), second(15), "aborted".into()))
        );
        // A kill writes nothing: the next prompt replaces an unconfirmed start.
        let mut lines = head(output);
        lines.extend([
            user(40, "again", ""),
            assistant(41, "msg_b", "\"end_turn\""),
            system("turn_duration", 45),
        ]);
        assert!(!run(&lines).published_turns().valid);
    }
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
        // An aborted interval is published as the last one, but makes
        // accumulated coverage unknown (D7).
        let row = run(&[user(10, "hello", ""), abort.clone()]);
        assert!(!row.turns.valid, "{abort}");
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
    ] {
        let row = run(&lines);
        assert!(row.turns.valid && row.turns.supported, "{lines:?}");
        assert!(!row.claude.abort_adjacent);
    }
    // After an aborted interval, which makes accumulated coverage unknown.
    let row = run(&[
        user(1, "hello", ""),
        assistant(2, "msg_a", "\"tool_use\""),
        marker(5),
        system("turn_duration", 6),
    ]);
    assert!(!row.turns.valid && row.turns.supported && !row.claude.abort_adjacent);
    assert_eq!(row.turns.last_outcome.as_deref(), Some("aborted"));
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
fn turns_assistant_record_with_no_turn_running_is_ambiguous() {
    let ended = [
        user(1, "hello", ""),
        assistant(2, "msg_a", "\"end_turn\""),
        system("turn_duration", 3),
    ];
    // After a proven end, an assistant record with no trigger shows a turn
    // whose start was not seen: nothing about it is published.
    let mut lines = ended.to_vec();
    lines.push(assistant(10, "msg_b", "\"tool_use\""));
    let row = run(&lines);
    assert!(row.claude.ambiguous && !row.turns.valid);
    let published = row.published_turns();
    assert!(!published.current_known && published.last_duration == Some(2));
    // Its end clears the ambiguity, and the next turn is recorded again,
    // never the total. Its end ended no turn, so the next turn's start is
    // published only after a gated end (D7).
    lines.extend([
        assistant(40, "msg_c", "\"end_turn\""),
        system("turn_duration", 41),
    ]);
    let row = run(&lines);
    assert!(!row.claude.ambiguous && !row.turns.valid);
    assert_eq!(row.turns.last, Some(turn_key(ID, "user-1")));
    lines.extend([user(50, "next", ""), assistant(51, "msg_d", "\"tool_use\"")]);
    let row = run(&lines);
    assert!(row.turns.current_known && !row.turns.valid);
    assert!(!row.published_turns().current_known);
    assert_eq!(row.turns.start, Some(second(50)));
    lines.extend([
        assistant(52, "msg_e", "\"end_turn\""),
        system("turn_duration", 55),
        user(60, "then", ""),
        assistant(61, "msg_f", "\"tool_use\""),
    ]);
    let published = run(&lines).published_turns();
    assert!(published.current_known && published.start == Some(second(60)));
    // A slash command that writes local output and then runs the model, with
    // input joined to it: neither its interval nor the total is published.
    let mut lines = ended.to_vec();
    lines.extend([
        user(10, "<command-name>/review</command-name>", ""),
        user(11, "<local-command-stdout>ok</local-command-stdout>", ""),
        assistant(12, "msg_b", "\"tool_use\""),
        queue(20),
        dequeue(30),
        user(20, "more", ""),
        assistant(31, "msg_c", "\"tool_use\""),
        assistant(50, "msg_d", "\"end_turn\""),
        system("turn_duration", 51),
    ]);
    let row = run(&lines);
    assert!(!row.turns.valid && row.turns.last_duration == Some(2));
    // A `<synthetic>` record written directly after an abort shows nothing,
    // with or without a second abort record.
    let synthetic =
        assistant(21, "msg_s", "\"stop_sequence\"").replace("claude-fixture-1", "<synthetic>");
    let marker = user(20, "[Request interrupted by user]", "");
    let again = user(20, "stopped", "\"interruptedMessageId\":\"m-1\"");
    for aborted in [vec![marker.clone()], vec![marker, again]] {
        let mut lines = ended.to_vec();
        lines.extend([user(10, "next", ""), assistant(11, "msg_b", "\"tool_use\"")]);
        lines.extend(aborted);
        lines.extend([
            synthetic.clone(),
            user(30, "third", ""),
            assistant(31, "msg_c", "\"end_turn\""),
            system("turn_duration", 40),
        ]);
        // The aborted interval makes accumulated coverage unknown (D7).
        let row = run(&lines);
        assert!(!row.turns.valid && !row.claude.ambiguous);
        assert_eq!(row.turns.total, 2 + 10 + 10);
        assert_eq!(row.turns.last, Some(turn_key(ID, "user-30")));
        // A second one is not adjacent to the abort.
        lines.insert(lines.len() - 3, synthetic.replace("msg_s", "msg_t"));
        let row = run(&lines);
        assert!(!row.turns.valid && row.turns.last_duration == Some(10));
        assert_eq!(row.turns.last, Some(turn_key(ID, "user-10")));
    }
}

#[test]
fn turns_queued_input_joins_only_after_a_queue_operation() {
    let joined = run(&[
        user(10, "hello", ""),
        assistant(11, "msg_a", "\"tool_use\""),
        queue(12),
        dequeue(13),
        // Taken input keeps the stamp of the time it was queued.
        user(12, "also this", ""),
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
        user(12, "queued", ""),
        system("turn_duration", 20),
    ]);
    assert!(row.turns.valid);
    assert_eq!(finished(&row, "user", 10).map(|t| t.1), Some(second(20)));
    // A queue operation after the silent end is no join evidence: the turn
    // may have ended, so the input takes the unknown path.
    let row = run(&[
        user(10, "hello", ""),
        assistant(11, "msg_a", "\"end_turn\""),
        dequeue(12),
        user(13, "queued", ""),
        system("turn_duration", 20),
    ]);
    assert!(!row.turns.valid && row.turns.finished.is_empty());
}

/// Review round 13: a final response that stops for any reason other than
/// tool use (`max_tokens`, `refusal`, a `<synthetic>` error) ends the turn
/// as `end_turn` does, so leftover input taken at once never joins it.
#[test]
fn turns_final_stop_other_than_tool_use_is_a_silent_end() {
    let synthetic =
        assistant(105, "msg_b", "\"stop_sequence\"").replace("claude-fixture-1", "<synthetic>");
    for last in [
        assistant(105, "msg_b", "\"max_tokens\""),
        assistant(105, "msg_b", "\"refusal\""),
        synthetic,
    ] {
        let mut lines = vec![
            user(100, "first", ""),
            assistant(101, "msg_a", "\"tool_use\""),
            queue(103),
            last.clone(),
            dequeue(106),
            user(103, "more", "\"origin\":{\"kind\":\"human\"}"),
            assistant(107, "msg_c", "\"tool_use\""),
        ];
        // The second turn runs from 106: nothing dates it from 100.
        let row = run(&lines);
        let turns = row.published_turns();
        assert!(!turns.current_known && turns.start.is_none(), "{last}");
        lines.push(system("turn_duration", 121));
        let turns = run(&lines).published_turns();
        assert!(!turns.valid && turns.last_duration.is_none(), "{last}");
    }
}

/// A `turn_duration` that ended no turn proves no idle point. Claude Code
/// was not seen writing one mid-turn (review round 14), but nothing shows
/// which turn it ended, so an injected notification after it may enter a
/// running turn, as after local output.
#[test]
fn turns_injected_trigger_after_a_turn_duration_that_ended_no_turn_is_ambiguous() {
    let human = "\"origin\":{\"kind\":\"human\"}";
    for trigger in [
        queued(152, "task-notification", &notice("agent-a", "completed")),
        notified(152, "agent-a", "completed"),
    ] {
        let mut lines = vec![
            user(100, "first", human),
            assistant(101, "msg_a", "\"tool_use\""),
            launch(102, "agent-a"),
            assistant(110, "msg_b", "\"end_turn\""),
            user(120, "second", human),
            assistant(121, "msg_c", "\"tool_use\""),
            // A `turn_duration` with no active turn, written mid-turn.
            system("turn_duration", 150),
        ];
        let row = run(&lines);
        assert!(row.claude.local_idle && !row.published_turns().current_known);
        lines.extend([
            tool(151, "{\"stdout\":\"ok\"}"),
            trigger.clone(),
            assistant(153, "msg_d", "\"tool_use\""),
        ]);
        // The second turn runs from 120: nothing dates it from 152.
        let row = run(&lines);
        assert!(
            row.claude.ambiguous && row.turns.active.is_none(),
            "{trigger}"
        );
        assert!(!row.published_turns().current_known, "{trigger}");
        lines.extend([
            assistant(159, "msg_e", "\"end_turn\""),
            system("turn_duration", 160),
        ]);
        let turns = run(&lines).published_turns();
        assert!(!turns.valid && turns.last_duration.is_none(), "{trigger}");
    }
    // A human prompt is queued and taken instead, so it opens normally.
    let row = run(&[
        user(100, "first", human),
        assistant(101, "msg_a", "\"end_turn\""),
        user(120, "second", human),
        system("turn_duration", 150),
        user(200, "third", human),
        assistant(201, "msg_b", "\"tool_use\""),
    ]);
    assert_eq!(row.turns.start, Some(second(200)));
}

/// D7 gate: a `turn_duration` whose stamp less `durationMs` floors more than
/// 2 s from the saved start, or that has no usable `durationMs`, publishes no
/// interval and makes accumulated coverage unknown. The previous interval
/// stays the last one, and `durationMs` never replaces a bound.
#[test]
fn turns_duration_ms_rejects_an_interval_whose_saved_bounds_disagree() {
    let first = [
        user(1, "hello", ""),
        assistant(2, "msg_a", "\"end_turn\""),
        ended(3, 1),
        user(10, "second", ""),
        assistant(11, "msg_b", "\"end_turn\""),
    ];
    let with = |fields: &str| {
        let mut lines = first.to_vec();
        lines.push(record(
            "system",
            20,
            &format!("\"subtype\":\"turn_duration\"{fields}"),
        ));
        run_exact(&lines)
    };
    // The end is at 20.25 s: less 12.25 s or 8.25 s it floors to second 8
    // or 12, within 2 s of 10; less 13.251 s or 7.25 s, to 6 or 13.
    for (fields, kept) in [
        (",\"durationMs\":10000", true),
        (",\"durationMs\":12250", true),
        (",\"durationMs\":8250", true),
        (",\"durationMs\":7250", false),
        (",\"durationMs\":13251", false),
        (",\"durationMs\":6249", false),
        (",\"durationMs\":19000", false),
        ("", false),
        (",\"durationMs\":null", false),
        (",\"durationMs\":-1000", false),
        (",\"durationMs\":10000.5", false),
        (",\"durationMs\":\"10000\"", false),
        (",\"durationMs\":9007199254740991", false),
    ] {
        let row = with(fields);
        let turns = row.published_turns();
        assert_eq!(turns.valid, kept, "{fields}");
        if kept {
            // The saved bounds are published, never `durationMs`.
            assert_eq!(
                finished(&row, "user", 10),
                Some((second(10), second(20), "completed".into()))
            );
            assert_eq!(turns.total, 2 + 10);
        } else {
            assert!(row.turns.active.is_none() && finished(&row, "user", 10).is_none());
            assert_eq!(turns.last, Some(turn_key(ID, "user-1")), "{fields}");
            assert_eq!(turns.last_duration, Some(2));
            // The record still proves the turn's end at its own stamp; only
            // its interval is unchecked, and the current turn stays masked.
            assert!(!row.claude.local_idle && !row.claude.ambiguous, "{fields}");
            assert!(!row.claude.clean && !turns.current_known, "{fields}");
            assert_eq!(row.claude.end_floor, second(20), "{fields}");
        }
    }
    // Claude Code writes `turn_duration` at its turn's own end, so the turn
    // is over: an injected notification opens the next turn's pending start,
    // as a human prompt does, and that turn's own end is checked again.
    let mut lines = with(",\"durationMs\":19000");
    let note = queued(30, "task-notification", &notice("agent-y", "completed"));
    lines.apply(&Record::from_value(&serde_json::from_str(&note).unwrap(), ID, now()).unwrap());
    assert!(!lines.claude.ambiguous && lines.turns.active.is_none());
    assert_eq!(
        lines.claude.pending_start,
        Some((turn_key(ID, "attachment-30"), second(30)))
    );
    let mut lines = first.to_vec();
    lines.extend([
        ended(20, 1),
        user(30, "third", ""),
        assistant(31, "msg_c", "\"end_turn\""),
        ended(35, 30),
    ]);
    let row = run_exact(&lines);
    assert!(!row.turns.valid && finished(&row, "user", 30).is_some());
    assert_eq!(row.turns.last_duration, Some(5));
}

/// D7 mask: the current turn is published only when it was opened by a
/// human-origin or shape prompt after a gated `turn_duration` with nothing
/// pending, or from the start of the session file, and no background work
/// launched since is still running.
#[test]
fn turns_current_turn_is_published_only_from_a_clean_state() {
    let human = "\"origin\":{\"kind\":\"human\"}";
    let current = |lines: &[String]| {
        let turns = run_exact(lines).published_turns();
        turns.current_known.then_some(turns.start)
    };
    let head = || {
        vec![
            user(1, "hello", human),
            assistant(2, "msg_a", "\"end_turn\""),
            ended(3, 1),
        ]
    };
    let next = |lines: &mut Vec<String>, at: u64, trigger: String| {
        lines.extend([
            trigger,
            assistant(at + 1, &format!("msg_{at}"), "\"tool_use\""),
        ]);
    };
    // A human prompt or a shape prompt after a gated end.
    for trigger in [user(10, "next", human), user(10, "next", "")] {
        let mut lines = head();
        next(&mut lines, 10, trigger);
        assert_eq!(current(&lines), Some(Some(second(10))));
    }
    // A first turn of the session.
    assert_eq!(
        current(&[
            user(1, "hello", human),
            assistant(2, "msg_a", "\"tool_use\"")
        ]),
        Some(Some(second(1)))
    );
    // A `turn_duration` reporting a background agent or workflow pending,
    // or with a malformed count.
    for pending in [
        "\"pendingBackgroundAgentCount\":1",
        "\"pendingWorkflowCount\":2",
        "\"pendingWorkflowCount\":\"0\"",
    ] {
        let mut lines = head();
        lines[2] = ended(3, 1).replace("\"durationMs\"", &format!("{pending},\"durationMs\""));
        next(&mut lines, 10, user(10, "next", human));
        assert_eq!(current(&lines), None, "{pending}");
        // Claude Code writes each `turn_duration` at its turn's end. The
        // first with nothing pending after a pending run measures from the
        // run's first turn, so the gate rejects it: the turn still ends, and
        // the background work's notification then opens the next turn.
        lines.extend([assistant(12, "msg_b", "\"end_turn\""), ended(13, 1)]);
        next(&mut lines, 20, notified(20, "agent-x", "completed"));
        assert_eq!(current(&lines), None, "{pending}");
        // The next gated end with nothing pending dates the turn after it.
        lines.extend([assistant(22, "msg_c", "\"end_turn\""), ended(23, 20)]);
        next(&mut lines, 30, user(30, "then", human));
        assert_eq!(current(&lines), Some(Some(second(30))), "{pending}");
    }
    // An async launch masks the current turn, with or without an `agentId`
    // (a workflow launch carries only a task id).
    for result in [
        "{\"status\":\"async_launched\",\"agentId\":\"agent-a\"}",
        "{\"status\":\"async_launched\",\"taskId\":\"w1\",\"taskType\":\"local_workflow\"}",
        "{\"status\":\"teammate_spawned\",\"agent_id\":\"mate-a\",\"teammate_id\":\"mate-a\"}",
        "{\"status\":\"remote_launched\",\"taskId\":\"r1\"}",
        "{\"success\":true,\"commandName\":\"s\",\"status\":\"forked\",\"agentId\":\"agent-f\",\"background\":true}",
    ] {
        let mut lines = head();
        next(&mut lines, 10, user(10, "next", human));
        assert_eq!(current(&lines), Some(Some(second(10))));
        lines.push(tool(12, result));
        assert_eq!(current(&lines), None, "{result}");
    }
    // A successful resume masks the current turn even for an agent this file
    // did not launch, and so does a notification taken into the turn without
    // a final status (a resume by the user or another agent); one with an
    // unreadable task id is masked through invalid children.
    let resume = tool(
        12,
        "{\"success\":true,\"message\":\"synthetic\",\"resumedAgentId\":\"agent-z\"}",
    );
    let unstated = "<task-notification>\n<task-id>agent-z</task-id>\n<summary>synthetic</summary>\n</task-notification>";
    let unreadable = "<task-notification>\n<summary>synthetic</summary>\n</task-notification>";
    for tail in [
        vec![resume],
        vec![dequeue(12), queued(12, "task-notification", unstated)],
        vec![dequeue(12), queued(12, "task-notification", unreadable)],
    ] {
        let mut lines = head();
        next(&mut lines, 10, user(10, "next", human));
        assert_eq!(current(&lines), Some(Some(second(10))));
        lines.extend(tail);
        assert_eq!(current(&lines), None, "{lines:?}");
        lines.push(assistant(13, "msg_z", "\"tool_use\""));
        assert_eq!(current(&lines), None, "{lines:?}");
    }
    // `/fork` or `/subtask` writes only `system/local_command` records and may
    // start a background agent, so local output masks the next turn too.
    let mut lines = head();
    lines.extend([
        record(
            "system",
            5,
            "\"subtype\":\"local_command\",\"content\":\"<command-name>/fork</command-name>\"",
        ),
        record(
            "system",
            5,
            "\"subtype\":\"local_command\",\"content\":\"<local-command-stdout>synthetic</local-command-stdout>\"",
        ),
    ]);
    next(&mut lines, 10, user(10, "next", human));
    assert_eq!(current(&lines), None);
    let zero = ended(3, 1).replace(
        "\"durationMs\"",
        "\"pendingBackgroundAgentCount\":0,\"pendingWorkflowCount\":0,\"durationMs\"",
    );
    let mut lines = head();
    lines[2] = zero;
    next(&mut lines, 10, user(10, "next", human));
    assert_eq!(current(&lines), Some(Some(second(10))));
    // Injected input taken into a clean turn joins it: the turn was not
    // opened by it, so its start stays published.
    let mut joined = head();
    next(&mut joined, 10, user(10, "next", human));
    joined.extend([
        dequeue(12),
        queued(12, "task-notification", &notice("agent-x", "completed")),
    ]);
    assert_eq!(current(&joined), Some(Some(second(10))));
    // A turn opened by an injected trigger, after an abort or after local
    // output; and a turn while a launched child is still running.
    let mut injected = head();
    next(&mut injected, 10, notified(10, "agent-x", "completed"));
    let mut aborted = head();
    next(&mut aborted, 10, user(10, "next", human));
    aborted.extend([user(15, "[Request interrupted by user]", "")]);
    next(&mut aborted, 20, user(20, "then", human));
    let mut local = head();
    local.extend([
        user(5, "<command-name>/model</command-name>", ""),
        system("local_command", 6),
    ]);
    next(&mut local, 10, user(10, "next", human));
    let mut child = head();
    next(&mut child, 10, user(10, "next", human));
    child.push(launch(12, "agent-a"));
    for (name, lines) in [
        ("injected", &injected),
        ("aborted", &aborted),
        ("local", &local),
        ("child", &child),
    ] {
        let row = run_exact(lines);
        assert!(row.turns.current_known, "{name}");
        assert_eq!(current(lines), None, "{name}");
    }
    // The child's notification, taken into the turn, and a gated end with
    // nothing pending.
    child.extend([
        dequeue(14),
        queued(14, "task-notification", &notice("agent-a", "completed")),
        assistant(15, "msg_c", "\"end_turn\""),
        ended(16, 10),
    ]);
    next(&mut child, 20, user(20, "then", human));
    assert_eq!(current(&child), Some(Some(second(20))));
}

/// An aborted interval has no `durationMs` to check: it is published as the
/// last interval, but accumulated coverage becomes unknown.
#[test]
fn turns_aborted_interval_is_last_but_makes_the_total_unknown() {
    let row = run(&[
        user(1, "hello", ""),
        assistant(2, "msg_a", "\"end_turn\""),
        system("turn_duration", 3),
        user(10, "second", ""),
        assistant(11, "msg_b", "\"tool_use\""),
        user(15, "[Request interrupted by user]", ""),
    ]);
    let turns = row.published_turns();
    assert!(!turns.valid);
    assert_eq!(
        (turns.last_duration, turns.last_outcome.as_deref()),
        (Some(5), Some("aborted"))
    );
    // Only a dated turn: publishing the interval would reveal a start the
    // mask withheld while the turn ran, such as an injected trigger's.
    for (name, opened) in [
        ("injected", notified(10, "agent-x", "completed")),
        ("after local output", system("local_command", 6)),
    ] {
        let mut lines = vec![
            user(1, "hello", ""),
            assistant(2, "msg_a", "\"end_turn\""),
            system("turn_duration", 3),
        ];
        if name == "after local output" {
            lines.push(user(5, "<command-name>/model</command-name>", ""));
            lines.push(opened);
            lines.push(user(10, "second", ""));
        } else {
            lines.push(opened);
        }
        lines.extend([
            assistant(11, "msg_b", "\"tool_use\""),
            user(15, "[Request interrupted by user]", ""),
        ]);
        let turns = run(&lines).published_turns();
        assert!(!turns.valid, "{name}");
        assert_eq!(turns.last, Some(turn_key(ID, "user-1")), "{name}");
        assert_eq!(turns.last_outcome.as_deref(), Some("completed"), "{name}");
    }
}

#[test]
fn turns_queue_operation_after_a_silent_end_never_joins_the_idle_gap() {
    for taken in ["remove", "dequeue"] {
        let mut lines = vec![
            user(10, "hello", ""),
            assistant(15, "msg_a", "\"tool_use\""),
            assistant(20, "msg_b", "\"end_turn\""),
            system("stop_hook_summary", 20),
            queue(137),
            operation(144, taken),
            user(288, "much later", ""),
            assistant(300, "msg_c", "\"tool_use\""),
        ];
        // While the second turn runs, nothing dates it from second 10.
        let row = run(&lines);
        assert!(
            !row.turns.current_known && row.turns.start.is_none(),
            "{taken}"
        );
        lines.extend([
            assistant(310, "msg_d", "\"end_turn\""),
            system("turn_duration", 311),
        ]);
        let row = run(&lines);
        assert!(!row.turns.valid, "{taken}");
        assert!(row.turns.finished.is_empty() && row.turns.last_duration.is_none());
    }
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
    assert_eq!(row.claude.queued_since_start, Some(micros(12)));
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
    // A pending command echo survives the round trip, and is only valid
    // while its start is pending.
    let echo = run(&[user(30, "<command-name>/model</command-name>", "")]);
    let block = serde_json::to_value(&echo.claude).unwrap();
    assert_eq!(block["pending_command"], json!(true));
    let mut tampered = block.clone();
    tampered["pending_start"] = Value::Null;
    let tampered: ClaudeCursor = serde_json::from_value(tampered).unwrap();
    assert!(!tampered.validate(now()));
    let mut missing = block;
    missing.as_object_mut().unwrap().remove("pending_command");
    assert!(serde_json::from_value::<ClaudeCursor>(missing).is_err());
    // The latest proven end is required, and never later than the latest
    // stamp replayed: a block without it is replayed fresh.
    let ended = run(&[
        user(10, "hello", ""),
        assistant(11, "msg_a", "\"end_turn\""),
        system("turn_duration", 12),
    ]);
    let block = serde_json::to_value(&ended.claude).unwrap();
    assert_eq!(block["end_floor"], json!(second(12)));
    let mut tampered = block.clone();
    tampered["end_floor"] = json!(second(13));
    let tampered: ClaudeCursor = serde_json::from_value(tampered).unwrap();
    assert!(!tampered.validate(now()));
    let mut missing = block;
    missing.as_object_mut().unwrap().remove("end_floor");
    assert!(serde_json::from_value::<ClaudeCursor>(missing).is_err());
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
    // A clean state is never ambiguous.
    assert!(ambiguous.claude.clean && !ambiguous.turn_state());
    ambiguous.claude.clean = false;
    assert!(ambiguous.turn_state());
    ambiguous.claude.pending_start = Some((turn_key(ID, "user-20"), second(20)));
    assert!(!ambiguous.turn_state());
    // A silent end belongs to an active turn without queue evidence.
    let mut silent = run(&[
        user(10, "hello", ""),
        assistant(11, "msg_a", "\"end_turn\""),
    ]);
    assert!(silent.claude.silent_end && silent.turn_state());
    silent.claude.queued_since_start = Some(micros(11));
    assert!(!silent.turn_state());
    silent.claude.queued_since_start = None;
    silent.turns.unknown();
    assert!(!silent.turn_state());
    // A record lost while idle holds no running turn and no ambiguity.
    let mut idle = steps(&[Some(system("turn_duration", 10)), None]);
    assert!(idle.claude.lost_idle && idle.turn_state());
    idle.claude.ambiguous = true;
    assert!(!idle.turn_state());
    idle.claude.ambiguous = false;
    idle.claude.pending_start = Some((turn_key(ID, "user-20"), second(20)));
    assert!(!idle.turn_state());
    // So does a slash command's local output.
    let mut local = run(&[
        user(10, "<command-name>/model</command-name>", ""),
        system("local_command", 11),
    ]);
    assert!(local.claude.local_idle && local.turn_state());
    local.claude.ambiguous = true;
    assert!(!local.turn_state());
    local.claude.ambiguous = false;
    local.claude.pending_start = Some((turn_key(ID, "user-20"), second(20)));
    assert!(!local.turn_state());
    let mut active = run(&[
        user(10, "hello", ""),
        assistant(11, "msg_a", "\"tool_use\""),
    ]);
    active.claude.local_idle = true;
    assert!(!active.turn_state());
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
            user(2, "queued", ""),
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
    // With no active turn a trigger never joins, so an idle gap after a
    // command echo is never absorbed. Input taken while the echo is pending
    // may have joined a running turn, so the trigger is ambiguous. After
    // local-command output the echo ran no turn (D7), so the input taken
    // while idle opens the next turn.
    for local in [false, true] {
        let mut lines = vec![user(4, "<command-name>/model</command-name>", "")];
        lines.extend(local.then(|| system("local_command", 5)));
        lines.extend([
            dequeue(4000),
            user(4000, "queued", ""),
            assistant(4001, "msg_a", "\"end_turn\""),
            system("turn_duration", 4003),
        ]);
        let row = run(&lines);
        assert!(row.claude.queued_since_start.is_none());
        if local {
            assert!(row.turns.valid && row.turns.total == 3);
            assert_eq!(finished(&row, "user", 4), None);
        } else {
            assert!(!row.turns.valid && row.turns.finished.is_empty());
        }
    }
    // Each join consumes the evidence: a second prompt needs its own.
    let row = run(&[
        user(10, "hello", ""),
        assistant(11, "msg_a", "\"tool_use\""),
        dequeue(12),
        user(12, "queued", ""),
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

/// Applies one parsed line, or `invalid()` for a non-JSON line, then checks
/// the state every pass boundary must accept.
fn step(row: &mut Row, line: Option<&String>, trail: &[Option<String>]) {
    match line {
        Some(line) => {
            let value = timed(row, serde_json::from_str(line).unwrap());
            row.apply(&Record::from_value(&value, ID, now()).unwrap());
        }
        None => row.invalid(),
    }
    assert!(row.gate(now()), "rejected after {trail:#?}");
    // Nothing is current while a turn may be running from an unknown start.
    let unknown_start = row.claude.ambiguous || row.claude.lost_idle;
    assert!(
        !row.turns.current_known || !unknown_start,
        "current after {trail:#?}"
    );
}
fn steps(lines: &[Option<String>]) -> Row {
    let mut row = Row::new([1, 2], 0, now());
    for (index, line) in lines.iter().enumerate() {
        step(&mut row, line.as_ref(), &lines[..=index]);
    }
    row
}
/// Nothing is current and no interval is the last since `last`.
fn lost(row: &Row, last: Option<&str>) {
    assert!(
        !row.turns.valid && !row.turns.current_known,
        "{:?}",
        row.turns
    );
    assert!(row.turns.active.is_none() && row.turns.start.is_none());
    assert!(row.claude.pending_start.is_none() && row.claude.queued_since_start.is_none());
    let key = last.map(|uuid| turn_key(ID, uuid));
    assert_eq!(row.turns.last, key);
}

#[test]
fn turns_lost_mid_turn_publish_no_truncated_interval() {
    let opened = [
        user(10, "hello", ""),
        assistant(11, "msg_a", "\"tool_use\""),
    ];
    let bad = user(12, "next", "").replace(&stamp(12), "not-a-time");
    let epoch = user(12, "next", "").replace(&stamp(12), "1970-01-01T00:00:00.500Z");
    let unknown = user(12, "x", "\"origin\":{\"kind\":\"future-kind\"}");
    // A: unknown origin; B: a non-JSON line; C: triggers without a stamp.
    for (case, lost_at, rest) in [
        ("A", Some(unknown), vec![]),
        ("B", None, vec![]),
        (
            "C",
            Some(bad),
            vec![queue(14), dequeue(15), user(16, "queued", "")],
        ),
        ("C0", Some(epoch), vec![]),
    ] {
        let mut lines: Vec<Option<String>> = opened.iter().cloned().map(Some).collect();
        lines.push(lost_at);
        lines.extend(
            [
                assistant(13, "msg_b", "\"tool_use\""),
                notified(17, "agent-x", "completed"),
            ]
            .map(Some),
        );
        lines.extend(rest.into_iter().map(Some));
        lines.push(Some(assistant(18, "msg_c", "\"end_turn\"")));
        // While the real turn may still run, no late trigger is its start.
        let row = steps(&lines);
        lost(&row, None);
        assert!(row.claude.ambiguous, "{case}");
        lines.push(Some(system("turn_duration", 30)));
        let row = steps(&lines);
        lost(&row, None);
        assert!(!row.claude.ambiguous, "{case}");
        // The next trigger after that proven end opens a turn normally.
        lines.extend(
            [
                user(40, "again", ""),
                assistant(41, "msg_d", "\"tool_use\""),
            ]
            .map(Some),
        );
        let row = steps(&lines);
        assert_eq!(row.turns.start, Some(second(40)), "{case}");
        assert!(row.turns.current_known && !row.turns.valid, "{case}");
    }
}

#[test]
fn turns_lost_while_idle_keep_later_turns_publishable() {
    let mut lines: Vec<Option<String>> = [
        user(1, "hello", ""),
        assistant(2, "msg_a", "\"end_turn\""),
        system("turn_duration", 3),
    ]
    .map(Some)
    .to_vec();
    // An unparseable line between turns may have been a prompt whose turn
    // the next trigger joined, so that trigger is ambiguous (review round 5).
    // The turn after a proven end publishes its current turn again.
    lines.push(None);
    lines.extend([user(10, "next", ""), assistant(11, "msg_b", "\"tool_use\"")].map(Some));
    let row = steps(&lines);
    assert!(row.claude.ambiguous && !row.turns.current_known);
    let mut later = lines.clone();
    later.extend(
        [
            assistant(12, "msg_c", "\"end_turn\""),
            system("turn_duration", 20),
            user(30, "again", ""),
            assistant(31, "msg_d", "\"tool_use\""),
        ]
        .map(Some),
    );
    let row = steps(&later);
    assert!(row.turns.current_known && !row.turns.valid);
    assert_eq!(row.turns.start, Some(second(30)));
    // D: an unrecognised origin may itself have opened a turn.
    let mut lines = lines[..3].to_vec();
    lines.extend(
        [
            user(10, "x", "\"origin\":{\"kind\":\"future-kind\"}"),
            assistant(11, "msg_b", "\"tool_use\""),
            notified(14, "agent-x", "completed"),
            assistant(15, "msg_c", "\"end_turn\""),
            system("turn_duration", 30),
        ]
        .map(Some),
    );
    let row = steps(&lines);
    lost(&row, Some("user-1"));
    assert_eq!(row.turns.last_duration, Some(2));
}

#[test]
fn turns_lost_trigger_while_idle_publishes_no_truncated_interval() {
    let anon = user(10, "anon", "").replace(&format!("\"sessionId\":\"{ID}\","), "");
    // The lost record may be the prompt that opened the next turn: an
    // assistant record before any trigger shows that turn running.
    for (case, lost_at) in [("unparseable", None), ("no session", Some(anon))] {
        let mut lines: Vec<Option<String>> = [
            user(1, "hello", ""),
            assistant(2, "msg_a", "\"end_turn\""),
            system("turn_duration", 3),
        ]
        .map(Some)
        .to_vec();
        lines.push(lost_at);
        lines.extend(
            [
                assistant(11, "msg_b", "\"tool_use\""),
                launch(12, "agent-x"),
                assistant(13, "msg_c", "\"end_turn\""),
                notified(40, "agent-x", "completed"),
                assistant(41, "msg_d", "\"end_turn\""),
            ]
            .map(Some),
        );
        let row = steps(&lines);
        assert!(
            !row.turns.current_known && row.turns.start.is_none(),
            "{case}"
        );
        lines.push(Some(system("turn_duration", 100)));
        let row = steps(&lines);
        lost(&row, Some("user-1"));
        assert_eq!(row.turns.last_duration, Some(2), "{case}");
        // The next trigger after that proven end opens a turn normally.
        lines.extend(
            [
                user(110, "again", ""),
                assistant(111, "msg_e", "\"tool_use\""),
            ]
            .map(Some),
        );
        let row = steps(&lines);
        assert_eq!(row.turns.start, Some(second(110)), "{case}");
    }
}

#[test]
fn turns_trigger_after_a_record_lost_while_idle_is_ambiguous() {
    // Review round 5: a prompt lost as a non-JSON line, then a queued
    // notification injected into the turn that prompt opened. Published as a
    // pending start, it showed the turn running from 12 and lasting 8 s.
    let mut lines: Vec<Option<String>> = [
        user(1, "hello", ""),
        assistant(2, "msg_a", "\"end_turn\""),
        system("turn_duration", 3),
    ]
    .map(Some)
    .to_vec();
    lines.push(None);
    lines.extend(
        [
            queued(12, "task-notification", &notice("agent-y", "completed")),
            assistant(13, "msg_b", "\"tool_use\""),
        ]
        .map(Some),
    );
    let row = steps(&lines);
    let published = row.published_turns();
    assert!(!published.current_known && !published.valid);
    lines.extend(
        [
            assistant(14, "msg_c", "\"end_turn\""),
            system("turn_duration", 20),
        ]
        .map(Some),
    );
    let row = steps(&lines);
    lost(&row, Some("user-1"));
    assert_eq!(row.turns.last_duration, Some(2));
}

#[test]
fn turns_user_record_without_origin_text_or_flag_is_unknown() {
    // An image-only prompt from a version that writes no `origin`.
    let image = record(
        "user",
        10,
        "\"message\":{\"role\":\"user\",\"content\":[{\"type\":\"image\"}]}",
    );
    let mut lines = vec![
        user(1, "hello", ""),
        assistant(2, "msg_a", "\"end_turn\""),
        system("turn_duration", 3),
        image.clone(),
        assistant(11, "msg_b", "\"tool_use\""),
        launch(12, "agent-x"),
        notified(40, "agent-x", "completed"),
        assistant(41, "msg_c", "\"end_turn\""),
        system("turn_duration", 100),
    ];
    let row = run(&lines);
    assert!(!row.turns.valid && row.turns.last_duration == Some(2));
    assert_eq!(row.turns.last, Some(turn_key(ID, "user-1")));
    // The same shape with a rule-3 flag stays ignored: it opens no start, so
    // the assistant record after it shows a turn whose trigger was not seen.
    lines[3] = image.replacen("\"message\"", "\"isMeta\":true,\"message\"", 1);
    let row = run(&lines);
    assert!(!row.turns.valid && row.turns.last_duration == Some(2));
    // Without that record, the flagged line changes nothing.
    lines.remove(4);
    let row = run(&lines);
    assert!(row.turns.valid && row.turns.last_duration == Some(60));
}

#[test]
fn turns_unrecognised_origin_with_is_meta_is_unknown() {
    let mut lines = vec![
        user(1, "hello", ""),
        assistant(2, "msg_a", "\"end_turn\""),
        system("turn_duration", 3),
        user(
            10,
            "later",
            "\"isMeta\":true,\"origin\":{\"kind\":\"future-kind\"}",
        ),
        user(15, "[Request interrupted by user]", ""),
        user(20, "next", ""),
        assistant(21, "msg_b", "\"end_turn\""),
        system("turn_duration", 25),
    ];
    let row = run(&lines);
    assert!(
        !row.turns.valid && finished(&row, "user", 10).is_none(),
        "an unrecognised origin may have opened a turn"
    );
    assert!(run(&lines[..4]).claude.ambiguous);
    // A recognised origin with `isMeta` stays a trigger.
    lines[3] = user(
        10,
        "later",
        "\"isMeta\":true,\"origin\":{\"kind\":\"peer\"}",
    );
    let row = run(&lines[..4]);
    assert!(row.claude.pending_start.is_some() && !row.claude.ambiguous);
}

#[test]
fn turns_rejected_start_is_lost_with_its_queue_evidence() {
    let ended = [
        user(10, "hello", ""),
        assistant(11, "msg_a", "\"tool_use\""),
        system("turn_duration", 20),
    ];
    // E: a start before the previous end; F: with a dequeue while pending;
    // G: a repeated uuid of a finished turn.
    for (case, trigger, queued) in [
        ("E", user(15, "next", ""), false),
        ("F", user(15, "next", ""), true),
        ("G", user(10, "hello", ""), true),
    ] {
        let mut lines: Vec<Option<String>> = ended.iter().cloned().map(Some).collect();
        lines.push(Some(trigger));
        if queued {
            lines.push(Some(dequeue(16)));
        }
        lines.push(Some(assistant(21, "msg_b", "\"tool_use\"")));
        let row = steps(&lines);
        lost(&row, Some("user-10"));
        assert!(row.claude.ambiguous, "{case}");
        lines.extend(
            [
                notified(22, "agent-x", "completed"),
                assistant(23, "msg_c", "\"end_turn\""),
                system("turn_duration", 30),
            ]
            .map(Some),
        );
        let row = steps(&lines);
        lost(&row, Some("user-10"));
        assert_eq!(row.turns.last_duration, Some(10), "{case}");
    }
    // An abort that confirms a rejected start is still a proven end.
    let mut lines: Vec<Option<String>> = ended.iter().cloned().map(Some).collect();
    lines.extend(
        [
            user(15, "next", ""),
            user(16, "[Request interrupted by user]", ""),
            user(40, "again", ""),
            assistant(41, "msg_d", "\"tool_use\""),
        ]
        .map(Some),
    );
    let row = steps(&lines);
    assert!(!row.claude.ambiguous && row.turns.current_known);
    assert_eq!(row.turns.start, Some(second(40)));
}

#[test]
fn turns_rejected_start_at_a_pass_boundary_still_catches_up() {
    // H: about 800 KiB after a rejected start with queue evidence, so the
    // first pass boundary falls inside that state.
    let fixture = super::tests::Fixture::new();
    let pad = "p".repeat(8000);
    let mut lines = vec![record("system", 1, "\"subtype\":\"init\"")];
    lines.extend([
        user(10, "hello", ""),
        assistant(11, "msg_a", "\"end_turn\""),
        ended(20, 10),
        user(15, "next", ""),
        dequeue(16),
        assistant(21, "msg_b", "\"tool_use\""),
    ]);
    lines.extend((0..100).map(|n| record("progress", 30 + n, &format!("\"data\":\"{pad}\""))));
    // The lost turn's own end, then a turn that publishes normally.
    lines.extend([
        system("turn_duration", 190),
        user(200, "again", ""),
        assistant(201, "msg_c", "\"tool_use\""),
    ]);
    let text = lines.join("\n") + "\n";
    assert!(text.len() > TAIL);
    let path = fixture.file("slug-a", &format!("{ID}.jsonl"), &text);
    let deadline = || std::time::Instant::now() + std::time::Duration::from_secs(5);
    let mut row = None;
    let mut restarts = 0;
    for _ in 0..4 {
        let (next, resumed) =
            resume(&fixture.projects, &path, ID, row.take(), now(), deadline()).unwrap();
        restarts += usize::from(!resumed);
        row = Some(next);
    }
    let row = row.unwrap();
    assert_eq!(restarts, 1);
    assert!(row.caught_up && row.offset == text.len() as u64);
    assert!(!row.claude.ambiguous && row.turns.current_known);
    assert_eq!(row.turns.start, Some(second(200)));
    let (_, resumed) = resume(&fixture.projects, &path, ID, Some(row), now(), deadline()).unwrap();
    assert!(resumed);
}

#[test]
fn turns_queue_operation_at_unix_second_zero_is_a_missing_stamp() {
    // A dequeue or remove stamped at the epoch is no join evidence, and the
    // block stays valid, as for a trigger at second 0.
    for operation_kind in ["dequeue", "remove"] {
        let epoch = operation(13, operation_kind).replace(&stamp(13), "1970-01-01T00:00:00Z");
        let value: Value = serde_json::from_str(&epoch).unwrap();
        assert_eq!(
            Record::from_value(&value, ID, now()).unwrap().stamp,
            Some(0)
        );
        let mut lines = vec![
            user(10, "hello", ""),
            assistant(11, "msg_a", "\"tool_use\""),
            queue(12),
            epoch,
        ];
        let row = run(&lines);
        assert_eq!(row.claude.queued_since_start, None, "{operation_kind}");
        // Without evidence, the queued prompt cannot join the turn.
        lines.push(user(12, "more", ""));
        let row = run(&lines);
        assert!(row.claude.ambiguous && !row.turns.valid, "{operation_kind}");
    }
}

#[test]
fn turns_epoch_stamped_dequeue_at_a_pass_boundary_still_catches_up() {
    let fixture = super::tests::Fixture::new();
    let pad = "p".repeat(8000);
    let mut lines = vec![record("system", 1, "\"subtype\":\"init\"")];
    lines.extend([
        user(10, "hello", ""),
        assistant(11, "msg_a", "\"tool_use\""),
        queue(12),
        dequeue(13).replace(&stamp(13), "1970-01-01T00:00:00Z"),
    ]);
    lines.extend((0..100).map(|n| record("progress", 30 + n, &format!("\"data\":\"{pad}\""))));
    lines.extend([
        assistant(180, "msg_b", "\"end_turn\""),
        ended(190, 10),
        user(200, "again", ""),
        assistant(201, "msg_c", "\"tool_use\""),
    ]);
    let text = lines.join("\n") + "\n";
    assert!(text.len() > TAIL);
    let path = fixture.file("slug-a", &format!("{ID}.jsonl"), &text);
    let deadline = || std::time::Instant::now() + std::time::Duration::from_secs(5);
    let mut row = None;
    let mut restarts = 0;
    for _ in 0..4 {
        let (next, resumed) =
            resume(&fixture.projects, &path, ID, row.take(), now(), deadline()).unwrap();
        restarts += usize::from(!resumed);
        row = Some(next);
    }
    let row = row.unwrap();
    assert_eq!(restarts, 1);
    assert!(row.caught_up && row.offset == text.len() as u64);
    assert!(row.turns.valid && row.turns.current_known);
    assert_eq!(row.turns.start, Some(second(200)));
    assert_eq!(row.turns.total, 180);
    let (_, resumed) = resume(&fixture.projects, &path, ID, Some(row), now(), deadline()).unwrap();
    assert!(resumed);
}

/// A deterministic xorshift sequence, so a failure always reproduces.
struct Seeded(u64);
impl Seeded {
    fn below(&mut self, n: u64) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 % n
    }
}

/// The D7 role a fuzzed line plays for the truncation oracle.
#[derive(Clone, Copy, PartialEq)]
enum Role {
    /// A record whose turn role is lost: it may have opened a turn.
    Lost,
    Trigger,
    Assistant,
    /// `turn_duration` or an abort: a proven end.
    End,
    Other,
}

#[test]
fn turns_fuzzed_replay_states_stay_resumable() {
    for seed in [
        0x9e37_79b9_7f4a_7c15,
        0x2545_f491_4f6c_dd1d,
        0x1234_5678_9abc_def1,
    ] {
        let mut random = Seeded(seed);
        for _ in 0..1500 {
            let (mut at, mut message) = (100u64, 0);
            let mut lines: Vec<Option<String>> = vec![];
            let mut roles = vec![];
            for index in 0..5 + random.below(30) {
                at = match random.below(6) {
                    0 => at.saturating_sub(random.below(8)),
                    1 => at,
                    _ => at + random.below(5),
                };
                message += u64::from(random.below(5) == 0);
                let id = format!("msg_{message}");
                let (line, role) = match random.below(24) {
                    0..=2 => (user(at, "prompt", ""), Role::Trigger),
                    3 => (user(at, "[Request interrupted by user]", ""), Role::End),
                    4 => (assistant(at, &id, "\"tool_use\""), Role::Assistant),
                    5 => (assistant(at, &id, "\"end_turn\""), Role::Assistant),
                    6 => (assistant(at, &id, "null"), Role::Assistant),
                    7 => (system("turn_duration", at), Role::End),
                    8 => (system("stop_hook_summary", at), Role::Other),
                    9 => (dequeue(at), Role::Other),
                    10 => (queue(at), Role::Other),
                    11 => (operation(at, "remove"), Role::Other),
                    12 => (
                        user(at, "<command-name>/x</command-name>", ""),
                        Role::Trigger,
                    ),
                    13 => (queued(at, "prompt", "later"), Role::Other),
                    14 => (notified(at, "agent-x", "completed"), Role::Trigger),
                    15 => (
                        user(at, "x", "\"origin\":{\"kind\":\"future-kind\"}"),
                        Role::Lost,
                    ),
                    16 => (
                        user(at, "late", "").replace(&stamp(at), "not-a-time"),
                        Role::Lost,
                    ),
                    17 => (
                        user(at, "zero", "").replace(&stamp(at), "1970-01-01T00:00:00.500Z"),
                        Role::Lost,
                    ),
                    18 => (
                        user(at, "anon", "").replace(&format!("\"sessionId\":\"{ID}\","), ""),
                        Role::Lost,
                    ),
                    19 => (
                        assistant(at, &id, "null").replacen(
                            "\"message\"",
                            "\"isAbortedMidStream\":true,\"message\"",
                            1,
                        ),
                        Role::End,
                    ),
                    20 => (String::new(), Role::Lost),
                    21 => (
                        record(
                            "user",
                            at,
                            "\"message\":{\"role\":\"user\",\"content\":[{\"type\":\"image\"}]}",
                        ),
                        Role::Lost,
                    ),
                    23 => (
                        assistant(at, &id, "\"end_turn\"").replace(
                            "\"input_tokens\":1,\"output_tokens\":1",
                            "\"input_tokens\":5000000000000000,\"output_tokens\":5000000000000000",
                        ),
                        Role::Assistant,
                    ),
                    22 if random.below(4) == 0 => (
                        user(at, "other", "").replace(ID, "fixture-session-b"),
                        Role::Lost,
                    ),
                    _ => (
                        user(at, "peer", "\"isMeta\":true,\"origin\":{\"kind\":\"peer\"}"),
                        Role::Trigger,
                    ),
                };
                // Unique uuids except an occasional repeat.
                let line = (!line.is_empty()).then(|| {
                    if random.below(15) == 0 {
                        line
                    } else {
                        line.replacen(&format!("-{at}\""), &format!("-{at}-{index}\""), 1)
                    }
                });
                lines.push(line);
                roles.push(role);
            }
            // Oracle: a lost record, then an assistant record before any
            // trigger, shows a turn running from an unknown start; nothing
            // is current until a proven end.
            let mut row = Row::new([1, 2], 0, now());
            let (mut lost, mut unknown) = (false, false);
            for (index, role) in roles.iter().enumerate() {
                step(&mut row, lines[index].as_ref(), &lines[..=index]);
                match role {
                    Role::Lost => lost = true,
                    Role::Trigger => lost = false,
                    Role::Assistant => unknown |= lost,
                    Role::End => (lost, unknown) = (false, false),
                    Role::Other => {}
                }
                assert!(
                    !unknown || !row.turns.current_known,
                    "{:#?}",
                    &lines[..=index]
                );
            }
        }
    }
}

#[test]
fn turns_two_records_lost_while_idle_may_be_a_running_turn() {
    // Found by the ground-truth fuzzer: the lost prompt and its lost first
    // assistant record leave a turn running, so a queued notification that
    // joins it is no start, and no interval is published until a proven end.
    let mut lines: Vec<Option<String>> = [
        user(1, "hello", ""),
        assistant(2, "msg_a", "\"end_turn\""),
        system("turn_duration", 3),
    ]
    .map(Some)
    .to_vec();
    lines.extend([None, None]);
    lines.extend(
        [
            queued(14, "task-notification", &notice("agent-x", "completed")),
            assistant(15, "msg_b", "\"tool_use\""),
        ]
        .map(Some),
    );
    let row = steps(&lines);
    lost(&row, Some("user-1"));
    assert!(row.claude.ambiguous);
    lines.push(Some(system("turn_duration", 30)));
    let row = steps(&lines);
    lost(&row, Some("user-1"));
    assert_eq!(row.turns.last_duration, Some(2));
}

#[test]
fn turns_input_taken_before_a_kill_never_joins_the_restarted_prompt() {
    // Found by the ground-truth fuzzer: input was taken into the turn, by a
    // queued attachment or not yet written, and the process was killed. The
    // restarted session's prompt is stamped after the evidence, so it is no
    // taken input and never absorbs the dead time.
    for taken in [
        vec![queue(12), dequeue(13)],
        vec![queue(12), operation(13, "remove")],
        vec![
            queue(12),
            operation(13, "remove"),
            queued(12, "prompt", "x"),
        ],
    ] {
        let mut lines = vec![
            user(10, "hello", ""),
            assistant(11, "msg_a", "\"tool_use\""),
        ];
        lines.extend(taken);
        lines.extend([
            user(600, "after a restart", ""),
            assistant(601, "msg_b", "\"end_turn\""),
            system("turn_duration", 605),
        ]);
        let row = run(&lines);
        assert!(!row.turns.valid && row.turns.last.is_none());
        assert!(finished(&row, "user", 10).is_none());
    }
}

#[test]
fn turns_input_taken_after_a_record_lost_while_idle_is_ambiguous() {
    // Found by the ground-truth fuzzer: the lost record was the prompt of a
    // turn that then took queued input, so the input is no start.
    let anon = operation(12, "remove").replace(&format!("\"sessionId\":\"{ID}\","), "");
    for taken in [operation(12, "remove"), anon] {
        let mut lines: Vec<Option<String>> = [
            user(1, "hello", ""),
            assistant(2, "msg_a", "\"end_turn\""),
            system("turn_duration", 3),
        ]
        .map(Some)
        .to_vec();
        lines.extend([None, Some(queue(11)), Some(taken)]);
        lines.extend(
            [
                queued(11, "task-notification", &notice("agent-x", "completed")),
                assistant(13, "msg_b", "\"tool_use\""),
            ]
            .map(Some),
        );
        let row = steps(&lines);
        lost(&row, Some("user-1"));
        assert!(row.claude.ambiguous);
        lines.push(Some(system("turn_duration", 30)));
        let row = steps(&lines);
        lost(&row, Some("user-1"));
        assert_eq!(row.turns.last_duration, Some(2));
    }
}

#[test]
fn turns_command_running_the_model_after_its_local_output_is_unknown() {
    let ended = vec![
        user(1, "hello", ""),
        assistant(2, "msg_a", "\"end_turn\""),
        system("turn_duration", 3),
        user(10, "<command-name>/review</command-name>", ""),
        user(11, "<local-command-stdout>ok</local-command-stdout>", ""),
    ];
    // An abort with no turn running ends the command's unseen turn.
    let mut lines = ended.clone();
    lines.push(user(20, "[Request interrupted by user]", ""));
    let row = run(&lines);
    assert!(!row.turns.valid && !row.claude.local_idle && row.claude.abort_adjacent);
    assert_eq!(row.turns.last_duration, Some(2));
    // Input queued and taken after the output, before any assistant record,
    // starts a turn when it is taken, but keeps the stamp of the time it was
    // queued: stamped before the take, its start is unknown. The documented
    // limit: stamped within the second of the take, it cannot be told from
    // input queued before a local command, so it opens the next turn
    // (`turns_join_needs_a_dequeue_or_remove_and_each_join_consumes_it`).
    for (taken, opens) in [(13, false), (12, true)] {
        let mut lines = ended.clone();
        lines.extend([
            queue(12),
            operation(taken, "remove"),
            user(12, "more", ""),
            assistant(14, "msg_b", "\"tool_use\""),
        ]);
        let row = run(&lines);
        assert_eq!(!row.claude.ambiguous && row.turns.valid, opens, "{taken}");
        assert_eq!(row.turns.start, opens.then(|| second(12)), "{taken}");
    }
}

#[test]
fn turns_input_taken_while_a_start_is_pending_is_ambiguous() {
    // Found by the ground-truth fuzzer: a notification queued and taken
    // before the first assistant record is input to the pending turn, so it
    // is no start, and no interval is published until a proven end.
    let mut lines = vec![
        user(10, "hello", ""),
        queue(11),
        operation(12, "remove"),
        queued(11, "task-notification", &notice("agent-x", "completed")),
        assistant(13, "msg_a", "\"end_turn\""),
    ];
    let row = run(&lines);
    assert!(row.claude.ambiguous && !row.turns.current_known);
    lines.push(system("turn_duration", 20));
    let row = run(&lines);
    assert!(!row.turns.valid && row.turns.last.is_none());
}

/// Review round 8: an end the reader proves while a turn is ambiguous, or
/// with no turn running, never reached `Turns::last_end`, so a trigger
/// stamped before it (leftover queued input keeps its queue time, or the
/// clock stepped back) was published as the start of the next turn.
#[test]
fn zz_trigger_before_a_proven_end_after_ambiguity_is_never_published() {
    let a = |at, message: &str, stop: &str| Some(assistant(at, message, stop));
    let s = |line: String| Some(line);
    let marker = "[Request interrupted by user]";
    // The next turn, opened by a trigger stamped at `at`.
    let next = |at: u64| {
        vec![
            s(user(at, "stamped before the end", "")),
            a(60, "msg_y", "\"tool_use\""),
            a(64, "msg_z", "\"end_turn\""),
            s(system("turn_duration", 65)),
        ]
    };
    let ended = || {
        vec![
            s(user(10, "hello", "")),
            a(11, "msg_a", "\"end_turn\""),
            s(system("turn_duration", 12)),
        ]
    };
    let running = || vec![s(user(31, "next", "")), a(32, "msg_b", "\"tool_use\"")];
    // Each path ends a turn the reader saw no start of, or could not
    // publish, at second 50; the following trigger is stamped at 45.
    let mut paths: Vec<(&str, Vec<Option<String>>)> = vec![];
    // Made ambiguous by a notification without queue evidence, then ended
    // by `turn_duration` or by an abort.
    let mut lines = running();
    lines.extend([
        s(notified(33, "agent-x", "completed")),
        a(36, "msg_c", "\"end_turn\""),
        s(system("turn_duration", 50)),
    ]);
    paths.push(("ambiguous turn, turn_duration", lines));
    let mut lines = running();
    lines.extend([
        s(notified(33, "agent-x", "completed")),
        s(user(50, marker, "")),
    ]);
    paths.push(("ambiguous turn, abort", lines));
    // An assistant record with no turn running.
    let mut lines = ended();
    lines.extend([
        a(40, "msg_b", "\"end_turn\""),
        s(system("turn_duration", 50)),
    ]);
    paths.push(("assistant with no turn running", lines));
    // An orphan `turn_duration`.
    let mut lines = ended();
    lines.push(s(system("turn_duration", 50)));
    paths.push(("orphan turn_duration", lines));
    // A record lost while idle.
    let mut lines = ended();
    lines.extend([None, s(system("turn_duration", 50))]);
    paths.push(("record lost while idle", lines));
    // Input taken with no turn running starts a turn at the take.
    let mut lines = ended();
    lines.push(s(dequeue(50)));
    paths.push(("input taken with no turn running", lines));
    // Control: a published end rejects the same trigger already.
    let mut lines = running();
    lines.extend([
        a(36, "msg_c", "\"end_turn\""),
        s(system("turn_duration", 50)),
    ]);
    paths.push(("control: published end", lines));
    let proven = second(50);
    let mut wrong = vec![];
    for (name, mut lines) in paths {
        let from = lines.len();
        lines.extend(next(45));
        let mut row = Row::new([1, 2], 0, now());
        for (index, line) in lines.iter().enumerate() {
            step(&mut row, line.as_ref(), &lines[..=index]);
            if index + 1 < from {
                continue;
            }
            let published = Published::of(&row);
            if let Some((true, Some(start))) = published.current
                && start < proven
            {
                wrong.push(format!("{name}: current start {start} before {proven}"));
            }
            if let Some((duration, _, end)) = published.last
                && end > proven
                && end - duration < proven
            {
                wrong.push(format!("{name}: last {duration} s overlaps {proven}"));
            }
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
    // A pending start is confirmed only at or after the latest proven end.
    let mut row = run(&[user(45, "pending", "")]);
    row.claude.end_floor = second(50);
    let value: Value = serde_json::from_str(&assistant(60, "msg_a", "\"tool_use\"")).unwrap();
    row.apply(&Record::from_value(&value, ID, now()).unwrap());
    assert!(row.claude.ambiguous && row.turns.active.is_none());
    assert!(!row.published_turns().current_known);
}

/// `line` with its timestamp replaced by `stamp`, or removed when `None`.
fn restamped(line: &str, at: u64, stamp_text: Option<&str>) -> String {
    let field = format!("\"timestamp\":\"{}\"", stamp(at));
    let value = stamp_text.map(|text| format!("\"timestamp\":\"{text}\""));
    match value {
        Some(value) => line.replacen(&field, &value, 1),
        None => line.replacen(&format!("{field},"), "", 1),
    }
}
/// Pushes every published current start before `proven`, and every last
/// interval overlapping it, after the records from `from` onwards.
fn published_before(name: &str, lines: &[Option<String>], from: usize, proven: u64) -> Vec<String> {
    let mut wrong = vec![];
    let mut row = Row::new([1, 2], 0, now());
    for (index, line) in lines.iter().enumerate() {
        step(&mut row, line.as_ref(), &lines[..=index]);
        if index + 1 < from {
            continue;
        }
        let published = Published::of(&row);
        if let Some((true, Some(start))) = published.current
            && start < proven
        {
            wrong.push(format!("{name}: current start {start} before {proven}"));
        }
        if let Some((duration, _, end)) = published.last
            && end > proven
            && end - duration < proven
        {
            wrong.push(format!("{name}: last {duration} s overlaps {proven}"));
        }
    }
    wrong
}

/// Review round 9: an end with no usable second (missing, unparseable,
/// within Unix second 0 or past the horizon) proves an end at an unknown
/// time, so a trigger stamped before it is never published as a start.
#[test]
fn zz_trigger_before_an_unstamped_end_is_never_published() {
    let a = |at, message: &str, stop: &str| Some(assistant(at, message, stop));
    let s = |line: String| Some(line);
    let marker = "[Request interrupted by user]";
    let unusable = [
        Some("not-a-time"),
        Some("1970-01-01T00:00:00.500Z"),
        Some("2099-01-01T00:00:00.250Z"),
        None,
    ];
    let mut wrong = vec![];
    for text in unusable {
        let name = text.unwrap_or("missing");
        // (a) A turn ended by an unstamped `turn_duration` (true time 20),
        // then leftover input stamped 12, with no take, a take without a
        // usable stamp or `sessionId`, or a valid take at 20.
        let takes = [
            ("no take", None),
            ("unstamped take", s(restamped(&dequeue(20), 20, text))),
            (
                "take without sessionId",
                s(dequeue(20).replacen(&format!("\"sessionId\":\"{ID}\","), "", 1)),
            ),
            ("valid take", s(dequeue(20))),
        ];
        for (take, line) in takes {
            let mut lines = vec![
                s(user(10, "hello", "")),
                a(11, "msg_a", "\"tool_use\""),
                s(queue(12)),
                a(18, "msg_b", "\"end_turn\""),
                s(restamped(&system("turn_duration", 20), 20, text)),
            ];
            let from = lines.len();
            lines.extend(line.map(Some));
            lines.extend([
                s(user(12, "queued before the end", "")),
                a(21, "msg_c", "\"tool_use\""),
                a(24, "msg_d", "\"end_turn\""),
                s(system("turn_duration", 25)),
            ]);
            let label = format!("(a) {name}, {take}");
            wrong.extend(published_before(&label, &lines, from, second(20)));
        }
        // (b) The round 8 path with an unstamped closer (true time 50).
        let closers = [
            (
                "turn_duration",
                restamped(&system("turn_duration", 50), 50, text),
            ),
            ("abort", restamped(&user(50, marker, ""), 50, text)),
        ];
        for (closer, line) in closers {
            let mut lines = vec![
                s(user(31, "next", "")),
                a(32, "msg_b", "\"tool_use\""),
                s(notified(33, "agent-x", "completed")),
                a(36, "msg_c", "\"end_turn\""),
                s(line),
            ];
            let from = lines.len();
            lines.extend([
                s(user(45, "stamped before the end", "")),
                a(60, "msg_y", "\"tool_use\""),
                a(64, "msg_z", "\"end_turn\""),
                s(system("turn_duration", 65)),
            ]);
            let label = format!("(b) {name}, {closer}");
            wrong.extend(published_before(&label, &lines, from, second(50)));
        }
    }
    // (c) A stamped end in the second the leftover input was queued, then a
    // take with no usable second or no `sessionId` (true time 25): only the
    // take's second shows the next turn started after that end.
    let takes = unusable
        .map(|text| (text.unwrap_or("missing"), restamped(&dequeue(25), 25, text)))
        .into_iter()
        .chain([(
            "no sessionId",
            dequeue(25).replacen(&format!("\"sessionId\":\"{ID}\","), "", 1),
        )]);
    for (name, take) in takes {
        let mut lines = vec![
            s(user(10, "hello", "")),
            a(11, "msg_a", "\"tool_use\""),
            s(queue(20)),
            a(20, "msg_b", "\"end_turn\""),
            s(system("turn_duration", 20)),
            s(take),
        ];
        let from = lines.len();
        lines.extend([
            s(user(20, "queued in the second of the end", "")),
            a(26, "msg_c", "\"tool_use\""),
            a(28, "msg_d", "\"end_turn\""),
            s(system("turn_duration", 29)),
        ]);
        let label = format!("(c) stamped end, take {name}");
        wrong.extend(published_before(&label, &lines, from, second(25)));
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// Review round 10: input taken while a slash-command echo awaits its
/// output left no trace once local-command output cleared the echo, so the
/// taken input, stamped at its queue time (101), opened a normal start.
/// The truth is the echo's turn (start 100) joined by the input, or a local
/// command and a turn opened by the take (start 103): never 101.
#[test]
fn turns_input_taken_while_a_slash_command_echo_is_pending_is_ambiguous() {
    let s = |line: String| Some(line);
    let unsessioned = dequeue(103).replacen(&format!("\"sessionId\":\"{ID}\","), "", 1);
    let forms = [
        (
            "stamped dequeue",
            dequeue(103),
            user(104, "<local-command-stdout>ok</local-command-stdout>", ""),
        ),
        (
            "stamped dequeue, system output",
            dequeue(103),
            system("local_command", 104),
        ),
        (
            "epoch-stamped dequeue",
            restamped(&dequeue(103), 103, Some("1970-01-01T00:00:00.500Z")),
            system("local_command", 104),
        ),
        (
            "dequeue without timestamp",
            restamped(&dequeue(103), 103, None),
            system("local_command", 104),
        ),
        (
            "dequeue without sessionId",
            unsessioned,
            system("local_command", 104),
        ),
    ];
    let mut wrong = vec![];
    for (name, take, output) in forms {
        let lines = [
            s(user(10, "hello", "")),
            s(assistant(11, "msg_a", "\"end_turn\"")),
            s(system("turn_duration", 12)),
            s(user(100, "<command-name>/compact</command-name>", "")),
            s(queue(101)),
            s(take),
            s(output),
            s(user(101, "more", "")),
            s(assistant(105, "msg_b", "\"tool_use\"")),
            s(assistant(149, "msg_c", "\"end_turn\"")),
            s(system("turn_duration", 150)),
        ];
        let mut row = Row::new([1, 2], 0, now());
        for (index, line) in lines.iter().enumerate() {
            step(&mut row, line.as_ref(), &lines[..=index]);
            let published = Published::of(&row);
            if let Some((_, Some(start))) = published.current
                && ![second(10), second(100), second(103)].contains(&start)
            {
                wrong.push(format!("{name}: current start {start} after line {index}"));
            }
            if let Some((duration, _, _)) = published.last
                && ![2, 50, 47].contains(&duration)
            {
                wrong.push(format!("{name}: last {duration} s after line {index}"));
            }
            if let Some(total) = published.total
                && ![0, 2, 52, 49].contains(&total)
            {
                wrong.push(format!("{name}: total {total} after line {index}"));
            }
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

// Ground-truth turn fuzzer. A generator simulates a Claude Code session whose
// true turn intervals it knows (start at the trigger, end at `turn_duration`,
// an abort, the last silent record of a version without `turn_duration`, or
// a kill), writes the records the D7 model describes and injects lost,
// unclassifiable, foreign and reordered records. After every record each
// published value must be the truth, unknown, or unchanged from the value
// published before it. File mode appends the same session in random byte
// chunks and checks every caught-up pass the same way, and against the
// in-memory replay. Two calibrations come from the counts-only corpus replay
// or plain process facts: taken input keeps the stamp of the time it was
// queued (every corpus join was stamped before its `remove`), and a killed
// process writes nothing for at least 2 s while it restarts.

/// A true finished turn: start, end and outcome; `killed` is never published.
type Interval = (u64, u64, &'static str);

/// The true turn state of the generated session at one point.
#[derive(Clone, Debug, Default, PartialEq)]
struct World {
    running: Option<u64>,
    last: Option<Interval>,
    total: u64,
}

/// What `native::turn_timing` publishes for a Claude row: the current turn,
/// the last interval and the accumulated total, each `None` while unknown.
#[derive(Clone, Debug, Default, PartialEq)]
struct Published {
    current: Option<(bool, Option<u64>)>,
    last: Option<(u64, String, u64)>,
    total: Option<u64>,
}
impl Published {
    fn of(row: &Row) -> Self {
        let turns = row.published_turns();
        let last = match (turns.last_duration, &turns.last_outcome, turns.last_end) {
            (Some(duration), Some(outcome), Some(end)) => Some((duration, outcome.clone(), end)),
            _ => None,
        };
        Self {
            current: turns
                .current_known
                .then(|| (turns.active.is_some(), turns.start)),
            last,
            total: (turns.valid && turns.supported).then_some(turns.total),
        }
    }
    fn truth(world: &World) -> Self {
        Self {
            current: Some((world.running.is_some(), world.running.map(second))),
            last: world
                .last
                .map(|(start, end, outcome)| (end - start, outcome.to_owned(), second(end))),
            total: Some(world.total),
        }
    }
    /// The fields that are neither the truth nor unknown, and the values
    /// allowed although they are not the truth: one unchanged from
    /// `previous` across a kill, which writes no record, and an unchanged
    /// last valid interval of an `earlier` truth while accumulated coverage
    /// is unknown.
    fn wrong(
        &self,
        previous: &Self,
        truth: &Self,
        killed: bool,
        earlier: &[World],
    ) -> (Vec<&'static str>, [usize; 2]) {
        let (mut wrong, mut allowed) = (vec![], [0; 2]);
        let real = || earlier.iter().any(|w| Self::truth(w).last == self.last);
        for (field, differs, unchanged) in [
            (
                "current",
                self.current.is_some() && self.current != truth.current,
                self.current == previous.current,
            ),
            (
                "last",
                self.last.is_some() && self.last != truth.last,
                self.last == previous.last,
            ),
            (
                "total",
                self.total.is_some() && self.total != truth.total,
                self.total == previous.total,
            ),
        ] {
            if !differs {
                continue;
            }
            if killed && unchanged {
                allowed[0] += 1;
            } else if field == "last" && unchanged && self.total.is_none() && real() {
                allowed[1] += 1;
            } else {
                wrong.push(field);
            }
        }
        (wrong, allowed)
    }
}

/// One generated session: each line (`None` is unparseable), the truth after
/// it, whether a kill left it unrecorded, and a tag naming the shape that
/// wrote it.
struct Story {
    random: Seeded,
    lines: Vec<Option<String>>,
    truth: Vec<World>,
    killed: Vec<bool>,
    tags: Vec<String>,
    world: World,
    at: u64,
    uuid: u64,
    message: u64,
    /// Per-mille rate of injected record faults; 0 for a clean session.
    faults: u64,
    /// Per-mille rate of unusable stamps on end, abort and idle-take lines.
    targeted: u64,
    /// Lines over `LINE` are only written for file mode.
    oversized: bool,
    /// Input still queued when a turn ended starts the next turn.
    queued_left: bool,
    /// A background agent launched and not yet reported finished; its
    /// turn's `turn_duration` reports it pending (review round 14).
    agent: Option<String>,
    agents: u64,
    /// A finished background agent whose notification opens the next turn.
    notify: Option<String>,
    /// The start of the first turn of a run whose `turn_duration` records
    /// reported background work pending (review round 14).
    run: Option<u64>,
    /// A kill ended the last turn and no trigger has followed it.
    unrecorded: bool,
    /// When the oldest input still in the queue was queued.
    enqueued: Option<u64>,
}
impl Story {
    fn new(seed: u64, faults: u64, oversized: bool, targeted: u64) -> Self {
        Self {
            random: Seeded(seed),
            lines: vec![],
            truth: vec![],
            killed: vec![],
            tags: vec![],
            world: World::default(),
            at: 100,
            uuid: 0,
            message: 0,
            faults,
            targeted,
            oversized,
            queued_left: false,
            agent: None,
            agents: 0,
            notify: None,
            run: None,
            unrecorded: false,
            enqueued: None,
        }
    }
    fn chance(&mut self, per_mille: u64) -> bool {
        self.random.below(1000) < per_mille
    }
    /// Advances true time; `idle` allows a long gap.
    fn tick(&mut self, idle: bool) {
        self.at += if idle && self.chance(300) {
            30 + self.random.below(600)
        } else {
            self.random.below(4)
        };
    }
    /// A stamp at `at` with a random fraction, which floors to `at`.
    fn stamp(&mut self, at: u64) -> String {
        let fraction = self.random.below(1000);
        stamp(at).replace(".250Z", &format!(".{fraction:03}Z"))
    }
    /// A record of `kind` at `at` with a unique uuid and raw extra fields.
    fn record_at(&mut self, kind: &str, at: u64, fields: &str) -> String {
        self.uuid += 1;
        let stamp = self.stamp(at);
        format!(
            "{{\"type\":\"{kind}\",\"sessionId\":\"{ID}\",\"uuid\":\"g-{}\",\"timestamp\":\"{stamp}\"{}{fields}}}",
            self.uuid,
            if fields.is_empty() { "" } else { "," }
        )
    }
    fn record(&mut self, kind: &str, fields: &str) -> String {
        self.record_at(kind, self.at, fields)
    }
    fn user(&mut self, text: &str, fields: &str) -> String {
        let content = serde_json::to_string(text).unwrap();
        let message = format!("\"message\":{{\"role\":\"user\",\"content\":{content}}}");
        let fields = if fields.is_empty() {
            message
        } else {
            format!("{fields},{message}")
        };
        self.record("user", &fields)
    }
    /// An assistant line of the current message; `next` starts a new one.
    fn assistant(&mut self, stop: &str, next: bool) -> String {
        self.message += u64::from(next);
        let fields = format!(
            "\"message\":{{\"id\":\"msg_{}\",\"model\":\"claude-fixture-1\",\"stop_reason\":{stop},\"usage\":{{\"input_tokens\":1,\"output_tokens\":1,\"cache_read_input_tokens\":0,\"cache_creation_input_tokens\":0}},\"content\":[]}}",
            self.message
        );
        self.record("assistant", &fields)
    }
    fn system(&mut self, subtype: &str) -> String {
        self.record("system", &format!("\"subtype\":\"{subtype}\""))
    }
    fn operation(&mut self, operation: &str) -> String {
        self.record("queue-operation", &format!("\"operation\":\"{operation}\""))
    }
    /// The `turn_duration` Claude Code writes at the end of the turn that
    /// just ended, sometimes with a workflow pending.
    fn duration(&mut self) -> String {
        let pending = self.chance(300).then_some("pendingWorkflowCount");
        self.pending_duration(pending)
    }
    /// A `turn_duration` with `pending` work reported or none. A record with
    /// work pending measures `durationMs` from its own turn's start and
    /// starts or continues a run; the first with nothing pending after a run
    /// measures from the run's first turn (review round 14, counts only), so
    /// the gate rejects it. Otherwise it is within 1.5 s of the truth.
    fn pending_duration(&mut self, pending: Option<&str>) -> String {
        let (start, _, _) = self.world.last.unwrap();
        let from = match pending {
            Some(_) => {
                self.run = self.run.or(Some(start));
                start
            }
            None => self.run.take().unwrap_or(start),
        };
        let jitter = self.random.below(3001) as i64 - 1500;
        let elapsed = ((self.at - from) as i64 * 1000 + jitter).max(0);
        let pending = pending.map_or(String::new(), |key| format!("\"{key}\":1,"));
        self.record(
            "system",
            &format!("\"subtype\":\"turn_duration\",{pending}\"durationMs\":{elapsed}"),
        )
    }
    fn queued(&mut self, mode: &str, prompt: &str) -> String {
        let prompt = serde_json::to_string(prompt).unwrap();
        self.record(
            "attachment",
            &format!(
                "\"attachment\":{{\"type\":\"queued_command\",\"commandMode\":\"{mode}\",\"prompt\":{prompt}}}"
            ),
        )
    }
}

/// Replaces the `timestamp` value of a generated line.
fn restamp(line: &str, stamp: &str) -> String {
    let from = line.find("\"timestamp\":\"").unwrap() + 13;
    let to = from + line[from..].find('"').unwrap();
    format!("{}{stamp}{}", &line[..from], &line[to..])
}
/// The lines that prove an end, take input with no turn running, or take
/// input while a slash-command echo awaits its output.
const TARGETED: [&str; 11] = [
    "turn-duration",
    "turn-duration-again",
    "model-echo-duration",
    "abort",
    "abort-tool",
    "abort-id",
    "abort-stream",
    "abort-again",
    "abort-duration",
    "idle-dequeue",
    "model-echo-take",
];
impl Story {
    /// Pushes a line with the truth after it, sometimes replaced by a fault
    /// that loses or corrupts it. A fault never changes the truth. A
    /// targeted line sometimes has no usable second instead (review round
    /// 9): unparseable, within second 0, past the horizon or missing.
    fn push(&mut self, line: String, tag: &str) {
        let mut tag = tag.to_owned();
        let mut line = Some(line);
        if self.targeted > 0 && TARGETED.contains(&tag.as_str()) && self.chance(self.targeted) {
            let text = line.take().unwrap();
            let faulty = match self.random.below(4) {
                0 => restamp(&text, "not-a-time"),
                1 => restamp(&text, "1970-01-01T00:00:00.500Z"),
                2 => restamp(&text, "2099-01-01T00:00:00.250Z"),
                _ => {
                    let from = text.find("\"timestamp\":\"").unwrap();
                    let to = from + 13 + text[from + 13..].find('"').unwrap() + 2;
                    format!("{}{}", &text[..from], &text[to..])
                }
            };
            tag = format!("{tag}+unusable-stamp");
            line = Some(faulty);
        } else if self.faults > 0 && self.chance(self.faults) {
            let text = line.take().unwrap();
            let (fault, faulty) = match self.random.below(6) {
                0 => ("lost", None),
                1 => ("bad-stamp", Some(restamp(&text, "not-a-time"))),
                2 => {
                    let stamp = if self.chance(500) {
                        "1970-01-01T00:00:00Z"
                    } else {
                        "1970-01-01T00:00:00.500Z"
                    };
                    ("epoch-stamp", Some(restamp(&text, stamp)))
                }
                3 => (
                    "no-session",
                    Some(text.replace(&format!("\"sessionId\":\"{ID}\","), "")),
                ),
                4 if self.chance(150) => ("foreign", Some(text.replace(ID, "fixture-session-b"))),
                // An assistant line without its message id, or a user line
                // whose content has no text.
                _ if text.contains("\"message\":{\"id\"") => (
                    "unclassifiable",
                    Some(text.replacen("\"id\":\"msg_", "\"name\":\"msg_", 1)),
                ),
                _ if text.contains("\"role\":\"user\"") => {
                    let from = text.find("\"content\":").unwrap() + 10;
                    ("shapeless", Some(format!("{}[]}}}}", &text[..from])))
                }
                _ => ("lost", None),
            };
            tag = format!("{tag}+{fault}");
            line = faulty;
        }
        self.lines.push(line);
        self.truth.push(self.world.clone());
        self.killed.push(self.unrecorded);
        self.tags.push(tag);
    }
    /// Pushes a turn-body line, sometimes stamped before earlier records.
    fn body(&mut self, line: String, tag: &str) {
        if self.faults > 0 && self.chance(60) {
            let early = self.at.saturating_sub(1 + self.random.below(20)).max(1);
            let stamp = self.stamp(early);
            return self.push(restamp(&line, &stamp), &format!("body:{tag}+early"));
        }
        self.push(line, &format!("body:{tag}"));
    }
    /// The running turn ends at the current time.
    fn end(&mut self, outcome: &'static str) {
        let start = self.world.running.take().unwrap();
        self.world.last = Some((start, self.at, outcome));
        self.world.total += self.at - start;
    }
    /// A turn trigger of a random D7 shape, which may be a long prompt.
    fn trigger(&mut self) -> (String, &'static str) {
        let image = "\"message\":{\"role\":\"user\",\"content\":[{\"type\":\"image\"}]}";
        if self.oversized && self.chance(40) {
            let size = LINE + 10 + self.random.below(LINE as u64) as usize;
            let prompt = format!("prompt {}", "x".repeat(size));
            return (self.user(&prompt, ""), "oversized-prompt");
        }
        match self.random.below(11) {
            0 | 1 => (self.user("prompt", ""), "prompt"),
            2 => (
                self.user("prompt", "\"origin\":{\"kind\":\"human\"}"),
                "human",
            ),
            3 => (
                self.record(
                    "user",
                    &format!("\"origin\":{{\"kind\":\"human\"}},{image}"),
                ),
                "image-human",
            ),
            4 => (self.record("user", image), "image"),
            5 => (
                self.user(
                    &notice("agent-x", "completed"),
                    "\"origin\":{\"kind\":\"task-notification\"}",
                ),
                "notification",
            ),
            6 => (
                self.queued("task-notification", &notice("agent-x", "completed")),
                "queued-notification",
            ),
            7 => (
                self.user("peer", "\"isMeta\":true,\"origin\":{\"kind\":\"peer\"}"),
                "peer",
            ),
            8 => (
                self.user("plan", "\"origin\":{\"kind\":\"coordinator\"}"),
                "coordinator",
            ),
            _ => (
                self.user("<command-name>/review</command-name>", ""),
                "command",
            ),
        }
    }
    /// An abort of a random shape ends the running turn at its record.
    fn abort(&mut self) {
        self.tick(false);
        self.end("aborted");
        let (line, tag) = match self.random.below(4) {
            0 => (self.user("[Request interrupted by user]", ""), "abort"),
            1 => (
                self.user("[Request interrupted by user for tool use]", ""),
                "abort-tool",
            ),
            2 => (
                self.user("stopped", "\"interruptedMessageId\":\"m-1\""),
                "abort-id",
            ),
            _ => {
                let line = self.assistant("null", true).replacen(
                    "\"message\"",
                    "\"isAbortedMidStream\":true,\"message\"",
                    1,
                );
                (line, "abort-stream")
            }
        };
        self.push(line, tag);
        // A second abort record, and a `<synthetic>` record written directly
        // after the abort, show no turn.
        if self.chance(200) {
            let line = self.user("stopped", "\"interruptedMessageId\":\"m-1\"");
            self.push(line, "abort-again");
        }
        if self.chance(200) {
            let line = self
                .assistant("\"stop_sequence\"", true)
                .replace("claude-fixture-1", "<synthetic>");
            self.push(line, "abort-synthetic");
        }
        if self.chance(500) {
            self.tick(false);
            let line = self.duration();
            self.push(line, "abort-duration");
        }
    }
    /// The process dies: the running turn ends unrecorded, and a process
    /// restarted at least 2 s later may write a record of its own.
    fn kill(&mut self) {
        self.at += 1 + self.random.below(30);
        self.end("killed");
        self.unrecorded = true;
        self.enqueued = None;
        // Background agents die with the process, and so does its run.
        self.agent = None;
        self.run = None;
        self.at += 2 + self.random.below(10);
        self.tick(true);
        if self.chance(500) {
            let line = self.record("permission-mode", "\"permissionMode\":\"default\"");
            self.push(line, "resumed");
        }
    }
}
impl Story {
    /// A slash command that writes local-command output and then runs the
    /// model: the D7 local-output assumption failing. Its echo opens a turn.
    fn model_command(&mut self) {
        self.tick(true);
        let line = self.user("<command-name>/review</command-name>", "");
        self.world.running = Some(self.at);
        self.unrecorded = false;
        self.enqueued = None;
        self.push(line, "model-echo");
        // Input queued and taken while the echo awaits its output (review
        // round 10): the taken record, written after the output, keeps the
        // stamp of the time it was queued, and joins the echo's turn.
        let mut taken = None;
        if self.chance(300) {
            self.tick(false);
            self.enqueue();
            self.tick(false);
            taken = self.enqueued.take();
            let operation = if self.chance(500) {
                "dequeue"
            } else {
                "remove"
            };
            let line = self.operation(operation);
            self.push(line, "model-echo-take");
        }
        self.tick(false);
        let line = if self.chance(500) {
            self.system("local_command")
        } else {
            self.user("<local-command-stdout>ok</local-command-stdout>", "")
        };
        self.push(line, "model-echo-output");
        if let Some(queued) = taken {
            let line = if self.chance(500) {
                self.user("more", "")
            } else {
                self.user("more", "\"origin\":{\"kind\":\"human\"}")
            };
            let stamp = self.stamp(queued);
            self.push(restamp(&line, &stamp), "model-echo-taken");
        }
        // An abort before its first assistant record. A kill there leaves no
        // record of the model running, and input taken there cannot be told
        // from input queued before a local command, which opens the next
        // turn: both are the D7 assumption itself, so neither is generated.
        if self.chance(100) {
            return self.abort();
        }
        // A notification, peer or coordinator message entering the running
        // command turn before its first response, with no queue record
        // (review round 11).
        if self.chance(300) {
            self.tick(false);
            let line = match self.random.below(4) {
                0 => self.queued("task-notification", &notice("agent-y", "completed")),
                1 => self.user(
                    &notice("agent-y", "completed"),
                    "\"origin\":{\"kind\":\"task-notification\"}",
                ),
                2 => self.user(
                    "synthetic",
                    "\"isMeta\":true,\"origin\":{\"kind\":\"peer\"}",
                ),
                _ => self.user("synthetic", "\"origin\":{\"kind\":\"coordinator\"}"),
            };
            self.push(line, "model-echo-injected");
        }
        for _ in 0..1 + self.random.below(3) {
            self.tick(false);
            let line = self.assistant("\"tool_use\"", true);
            self.push(line, "model-echo-assistant");
        }
        if self.chance(500) {
            self.tick(false);
            self.enqueue();
            self.tick(false);
            self.take();
        }
        match self.random.below(10) {
            0 => return self.abort(),
            1 => return self.kill(),
            _ => {}
        }
        self.tick(false);
        let line = self.assistant("\"end_turn\"", true);
        self.push(line, "model-echo-end");
        self.tick(false);
        self.end("completed");
        let line = self.duration();
        self.push(line, "model-echo-duration");
    }
    /// Records between turns that never start or end one, including an idle
    /// slash-command echo that runs no model, and sometimes a slash command
    /// that does.
    fn idle(&mut self) {
        if self.chance(150) {
            self.model_command();
        }
        for _ in 0..self.random.below(3) {
            self.tick(true);
            let (line, tag) = match self.random.below(5) {
                0 => {
                    let line = self.user("<command-name>/model</command-name>", "");
                    self.push(line, "idle-echo");
                    self.tick(false);
                    if self.chance(500) {
                        (self.system("local_command"), "idle-echo-system")
                    } else {
                        let text = "<local-command-stdout>ok</local-command-stdout>";
                        (self.user(text, ""), "idle-echo-output")
                    }
                }
                1 => (self.user("caveat", "\"isMeta\":true"), "idle-meta"),
                2 => (self.user("<bash-input>ls</bash-input>", ""), "idle-bash"),
                3 => (self.system("compact_boundary"), "idle-compact"),
                _ => (self.record("file-history-snapshot", ""), "idle-other"),
            };
            self.push(line, tag);
        }
    }
    /// Input queued while a turn runs, stamped when it is queued.
    fn enqueue(&mut self) {
        self.enqueued = Some(self.enqueued.unwrap_or(self.at));
        let line = self.operation("enqueue");
        self.body(line, "enqueue");
    }
    /// Queued input taken into the running turn: as a joining trigger, as a
    /// `queued_command` attachment, or not yet consumed. A taken record keeps
    /// the stamp of the time it was queued, as corpus joins do, or sometimes
    /// the time it is taken.
    fn take(&mut self) {
        let queued = self.enqueued.take().unwrap_or(self.at);
        let operation = if self.chance(500) {
            "dequeue"
        } else {
            "remove"
        };
        let line = self.operation(operation);
        self.body(line, operation);
        let at = if self.chance(800) { queued } else { self.at };
        let (line, tag) = match self.random.below(5) {
            0 => (self.user("more", ""), "join"),
            1 => (
                self.queued("task-notification", &notice("agent-y", "completed")),
                "join-notification",
            ),
            2 => (self.queued("prompt", "more"), "queued-prompt"),
            3 => (
                self.user("more", "\"origin\":{\"kind\":\"human\"}"),
                "join-human",
            ),
            _ => return,
        };
        let stamp = self.stamp(at);
        self.body(restamp(&line, &stamp), tag);
    }
    /// A task notification injected into the running turn before its first
    /// assistant record, without queue evidence.
    fn injected(&mut self) {
        self.tick(false);
        let note = notice("agent-y", "completed");
        let line = if self.chance(500) {
            self.queued("task-notification", &note)
        } else {
            self.user(&note, "\"origin\":{\"kind\":\"task-notification\"}")
        };
        self.push(line, "pending-notification");
    }
    /// One true turn: a trigger, a body, and an end of a random kind.
    fn turn(&mut self) {
        // Input still queued when the last turn ended is sometimes taken at
        // once and starts this turn: it keeps the stamp of the time it was
        // queued, which is before that end (review round 8).
        let leftover = (self.enqueued)
            .filter(|_| self.queued_left)
            .filter(|_| self.chance(500));
        if std::mem::take(&mut self.queued_left) {
            let line = self.operation("dequeue");
            self.push(line, "idle-dequeue");
        }
        if leftover.is_none() {
            self.tick(true);
        }
        let (mut line, tag) = self.trigger();
        let mut shape = tag;
        if let Some(agent) = self.notify.take() {
            line = self.notification(&agent);
            shape = "agent-notification";
        } else if let Some(queued) = leftover {
            // Sometimes written as a queued prompt attachment, which D7
            // rule 3 ignores inside a turn (review round 12).
            shape = "leftover";
            if self.chance(250) {
                line = self.queued("prompt", "more");
                shape = "leftover-attachment";
            }
            let stamp = self.stamp(queued);
            line = restamp(&line, &stamp);
        }
        self.world.running = Some(self.at);
        self.push(line, shape);
        self.unrecorded = false;
        self.enqueued = None;
        if tag == "command" && self.chance(500) {
            let line = self.user("expanded", "\"isMeta\":true");
            self.push(line, "command-meta");
        }
        match self.random.below(100) {
            0..=5 => return self.abort(),
            6..=9 => return self.kill(),
            // Input queued and taken before the first assistant record.
            10..=13 => {
                self.tick(false);
                self.enqueue();
                self.tick(false);
                self.take();
            }
            // A local command run while the first response is awaited: its
            // echo then its output, or the output alone. A slash-command echo
            // followed by local output is itself a local command, which runs
            // no turn (D7), so a pending command is never followed by output.
            14..=16 if tag != "command" => {
                self.tick(false);
                if self.chance(500) {
                    let line = self.user("<command-name>/model</command-name>", "");
                    self.push(line, "pending-local-echo");
                    self.tick(false);
                }
                let line = if self.chance(500) {
                    self.system("local_command")
                } else {
                    self.user("<local-command-stdout>ok</local-command-stdout>", "")
                };
                self.push(line, "pending-local-output");
                // The prompt's turn still runs: input injected into it, an
                // abort or a kill before its first assistant record.
                match self.random.below(4) {
                    0 => self.injected(),
                    1 => return self.abort(),
                    2 => return self.kill(),
                    _ => {}
                }
            }
            17..=18 => self.injected(),
            _ => {}
        }
        self.tick(false);
        let first = if self.chance(100) {
            self.assistant("null", true)
                .replace("claude-fixture-1", "<synthetic>")
        } else {
            let stop = if self.chance(500) {
                "\"tool_use\""
            } else {
                "null"
            };
            self.assistant(stop, true)
        };
        self.push(first, "first-assistant");
        // A `turn_duration` written while the turn runs, whose `durationMs`
        // puts its start at least 4 s before the turn's. Claude Code was not
        // seen writing one mid-turn (review round 14): it is hostile input
        // that only the gate keeps from ending the turn at its stamp. The
        // turn's next record is an assistant one, so a trigger never follows
        // the rejected record as if the turn had ended.
        if self.chance(40) {
            let start = self.world.running.unwrap();
            self.tick(false);
            let elapsed = (self.at - start) * 1000 + 5000 + self.random.below(60_000);
            let line = self.record(
                "system",
                &format!("\"subtype\":\"turn_duration\",\"durationMs\":{elapsed}"),
            );
            self.push(line, "stray-duration");
            self.tick(false);
            let line = self.assistant("\"tool_use\"", true);
            self.push(line, "stray-assistant");
        }
        for _ in 0..self.random.below(9) {
            self.tick(false);
            let (line, tag) = match self.random.below(12) {
                0 | 1 => {
                    let line = self.assistant("\"tool_use\"", true);
                    self.body(line, "tool-use");
                    self.tick(false);
                    if self.agent.is_none() && self.chance(150) {
                        self.agents += 1;
                        let agent = format!("bg-{}", self.agents);
                        let result =
                            format!("{{\"status\":\"async_launched\",\"agentId\":\"{agent}\"}}");
                        self.agent = Some(agent);
                        (self.result(&result), "launch")
                    } else {
                        (self.result("{\"stdout\":\"ok\"}"), "tool-result")
                    }
                }
                2 => (self.assistant("null", true), "partial"),
                3 => (
                    self.assistant("null", true)
                        .replace("claude-fixture-1", "<synthetic>"),
                    "synthetic",
                ),
                4 => (self.user("hook", "\"isMeta\":true"), "meta"),
                5 => {
                    self.enqueue();
                    continue;
                }
                6 if self.enqueued.is_some() => {
                    self.take();
                    continue;
                }
                7 => {
                    // A silent end, then queued input that joins the turn.
                    let line = self.assistant("\"end_turn\"", true);
                    self.body(line, "silent-end");
                    if self.chance(500) {
                        let line = self.system("stop_hook_summary");
                        self.body(line, "silent-hook");
                    }
                    self.tick(false);
                    self.enqueue();
                    self.tick(false);
                    self.take();
                    continue;
                }
                8 => (
                    self.queued("task-notification", &notice("agent-y", "completed")),
                    "notification-mid",
                ),
                9 => (
                    self.user("<local-command-stdout>x</local-command-stdout>", ""),
                    "wrapper",
                ),
                10 => (self.system("compact_boundary"), "compact"),
                _ if self.oversized && self.chance(100) => {
                    let pad = "p".repeat(LINE + 10 + self.random.below(LINE as u64) as usize);
                    (
                        self.record("progress", &format!("\"data\":\"{pad}\"")),
                        "oversized",
                    )
                }
                _ => (self.record("progress", "\"data\":\"x\""), "progress"),
            };
            self.body(line, tag);
        }
        self.finish();
    }
}
impl Story {
    /// The end of a turn: `turn_duration`, a version without it, an abort or
    /// a kill, which may come after queued input was taken.
    fn finish(&mut self) {
        let queued = self.enqueued.is_some();
        match self.random.below(100) {
            0..=54 if self.agent.is_some() && !queued => self.background(),
            0..=54 => {
                self.tick(false);
                let line = self.final_line();
                self.body(line, "end-turn");
                if self.chance(500) {
                    let line = self.system("stop_hook_summary");
                    self.body(line, "stop-hook");
                }
                self.tick(false);
                self.end("completed");
                let line = self.duration();
                self.push(line, "turn-duration");
                // A repeated `turn_duration` shows a turn whose start was
                // not seen: coverage becomes unknown, never wrong.
                if self.chance(100) {
                    let line = self.duration();
                    self.push(line, "turn-duration-again");
                }
                self.queued_left = queued;
            }
            55..=69 => {
                // Without `turn_duration` the turn ends at its last record,
                // whatever its stop, and input queued before it is left
                // over (review round 13).
                if !queued && self.chance(300) {
                    self.tick(false);
                    self.enqueue();
                }
                let queued = self.enqueued.is_some();
                self.tick(false);
                let line = self.final_line();
                if self.chance(500) {
                    self.body(line, "old-end-turn");
                    self.tick(false);
                    self.end("completed");
                    let line = self.system("stop_hook_summary");
                    self.push(line, "old-stop-hook");
                } else {
                    self.end("completed");
                    self.push(line, "old-end-turn");
                }
                self.queued_left = queued;
            }
            70..=84 => {
                // Input still queued after an abort is taken at once too
                // (review round 12).
                self.abort();
                self.queued_left = queued;
            }
            _ => {
                if self.chance(400) {
                    if !queued {
                        self.tick(false);
                        self.enqueue();
                    }
                    self.tick(false);
                    self.take();
                }
                self.kill();
            }
        }
    }
    /// Swaps a few adjacent turn-body lines that share the same truth.
    fn swap(&mut self) {
        for _ in 0..self.random.below(4) {
            let Some(last) = self.lines.len().checked_sub(1).filter(|n| *n > 0) else {
                return;
            };
            let at = self.random.below(last as u64) as usize;
            if self.tags[at].starts_with("body:")
                && self.tags[at + 1].starts_with("body:")
                && self.truth[at] == self.truth[at + 1]
            {
                self.lines.swap(at, at + 1);
                self.tags.swap(at, at + 1);
                self.tags[at].push_str("+swapped");
            }
        }
    }
}
impl Story {
    /// A tool result line carrying `result` as its `toolUseResult`.
    fn result(&mut self, result: &str) -> String {
        let fields = format!(
            "\"toolUseResult\":{result},\"message\":{{\"role\":\"user\",\"content\":[{{\"type\":\"tool_result\",\"content\":\"done\"}}]}}"
        );
        self.record("user", &fields)
    }
    /// A finished background agent's notification, in either D6 form.
    fn notification(&mut self, agent: &str) -> String {
        let note = notice(agent, "completed");
        if self.chance(500) {
            self.queued("task-notification", &note)
        } else {
            self.user(&note, "\"origin\":{\"kind\":\"task-notification\"}")
        }
    }
    /// The final response of a turn: mostly `end_turn`, sometimes a stop
    /// for another reason or a `<synthetic>` error (review round 13).
    fn final_line(&mut self) -> String {
        match self.random.below(10) {
            0 => self.assistant("\"max_tokens\"", true),
            1 => self.assistant("\"refusal\"", true),
            2 => self
                .assistant("\"stop_sequence\"", true)
                .replace("claude-fixture-1", "<synthetic>"),
            _ => self.assistant("\"end_turn\"", true),
        }
    }
    /// A turn that ends while a background agent runs. Claude Code writes
    /// its `turn_duration` at the turn's own end, after the final response
    /// and sometimes its stop hook, with the agent pending and the turn's own
    /// `durationMs` (review round 14, counts only). The agent's notification
    /// then opens the next turn, or enters a turn a human prompt opened.
    fn background(&mut self) {
        let agent = self.agent.take().unwrap();
        self.tick(false);
        let line = self.final_line();
        self.body(line, "background-end");
        if self.chance(500) {
            let line = self.system("stop_hook_summary");
            self.body(line, "background-hook");
        }
        self.tick(false);
        self.end("completed");
        let line = self.pending_duration(Some("pendingBackgroundAgentCount"));
        self.push(line, "background-duration");
        if self.chance(500) {
            self.notify = Some(agent);
            return;
        }
        self.tick(true);
        let line = self.user("prompt", "\"origin\":{\"kind\":\"human\"}");
        self.world.running = Some(self.at);
        self.push(line, "background-next");
        self.unrecorded = false;
        self.enqueued = None;
        self.tick(false);
        let line = self.assistant("\"tool_use\"", true);
        self.push(line, "background-next-assistant");
        self.tick(false);
        let line = self.result("{\"stdout\":\"ok\"}");
        self.body(line, "tool-result");
        self.tick(false);
        if self.chance(300) {
            let line = self.operation("dequeue");
            self.body(line, "background-take");
        }
        let line = self.notification(&agent);
        self.push(line, "background-notification");
        self.tick(false);
        let line = self.assistant("\"tool_use\"", true);
        self.body(line, "tool-use");
        self.finish();
    }
}
/// A generated session of 2 to 11 turns with idle records between them.
fn story(seed: u64, faults: u64, oversized: bool, targeted: u64) -> Story {
    let mut story = Story::new(seed, faults, oversized, targeted);
    for _ in 0..2 + story.random.below(10) {
        story.idle();
        story.turn();
    }
    if faults > 0 {
        story.swap();
    }
    story
}
/// Turns and the block pass through serde at a checkpoint, as in `enrich`.
fn round_trip(row: &mut Row) {
    row.turns = serde_json::from_value(serde_json::to_value(&row.turns).unwrap()).unwrap();
    row.claude = serde_json::from_value(serde_json::to_value(&row.claude).unwrap()).unwrap();
}
/// Every wrong field with its trail, and the counts of values allowed
/// across a kill and as the last valid interval (`Published::wrong`).
#[derive(Default)]
struct Found {
    wrong: Vec<String>,
    allowed: [usize; 2],
    /// Checks that published a total, a last interval and a running turn's
    /// start, so a pass is never only unknown values.
    known: [usize; 3],
}
impl Found {
    fn push(&mut self, wrong: String) {
        self.wrong.push(wrong);
    }
}
/// Checks one published state against the truth after `index` lines and the
/// previous published state, recording every wrong field with its trail.
fn check(
    row: &Row,
    previous: &Published,
    story: &Story,
    lines: usize,
    label: &str,
    found: &mut Found,
) -> Published {
    let published = Published::of(row);
    let (world, killed) = lines
        .checked_sub(1)
        .map_or((World::default(), false), |at| {
            (story.truth[at].clone(), story.killed[at])
        });
    let truth = Published::truth(&world);
    let (mut wrong, allowed) = published.wrong(previous, &truth, killed, &story.truth[..lines]);
    found.allowed[0] += allowed[0];
    found.allowed[1] += allowed[1];
    found.known[0] += usize::from(published.total.is_some());
    found.known[1] += usize::from(published.last.is_some());
    found.known[2] += usize::from(matches!(published.current, Some((true, Some(_)))));
    if !row.gate(now()) {
        wrong.push("gate");
    }
    for field in wrong {
        let trail = story.tags[lines.saturating_sub(30)..lines].join(" ");
        found.push(format!(
            "{label}: {field} after line {lines}: published {published:?}, before {previous:?}, truth {world:?}\n    {trail}"
        ));
    }
    published
}
/// Applies every line in memory, checking after each one, and returns the
/// published state after each line.
fn replay_memory(story: &Story, label: &str, found: &mut Found) -> Vec<Published> {
    let mut row = Row::new([1, 2], 0, now());
    let mut previous = Published::of(&row);
    let mut published = vec![];
    for (index, line) in story.lines.iter().enumerate() {
        match line
            .as_deref()
            .and_then(|line| serde_json::from_str::<Value>(line).ok())
            .and_then(|value| Record::from_value(&value, ID, now()))
        {
            Some(record) => row.apply(&record),
            None => row.invalid(),
        }
        if index % 7 == 3 {
            round_trip(&mut row);
        }
        previous = check(&row, &previous, story, index + 1, label, found);
        published.push(previous.clone());
    }
    published
}
/// Appends the session in random byte chunks, splitting lines anywhere, and
/// runs `resume` passes after each append as `enrich` does, checking every
/// caught-up pass against the truth, with the state `memory` published
/// before the last line as the previous value, and against `memory` itself:
/// a pass boundary never changes a value. Returns the passes that restarted.
fn replay_file(
    story: &Story,
    memory: &[Published],
    seed: u64,
    label: &str,
    found: &mut Found,
) -> usize {
    use std::io::Write;
    let fixture = super::tests::Fixture::new();
    let header = format!("{{\"type\":\"permission-mode\",\"sessionId\":\"{ID}\"}}\n");
    let path = fixture.file("slug-a", &format!("{ID}.jsonl"), &header);
    let mut body = Vec::new();
    let mut ends = vec![];
    for line in &story.lines {
        // A lost line is a truncated record.
        body.extend_from_slice(
            line.as_deref()
                .unwrap_or("{\"type\":\"user\",\"sessionId\":")
                .as_bytes(),
        );
        body.push(b'\n');
        ends.push(body.len());
    }
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap();
    let mut random = Seeded(seed ^ 0x5bd1_e995);
    let (mut row, mut written, mut restarts) = (None::<Row>, 0, 0);
    let initial = Published::default();
    let after = |lines: usize| lines.checked_sub(1).map_or(&initial, |at| &memory[at]);
    while written < body.len() {
        let size = match random.below(8) {
            0..=2 => 1 + random.below(40),
            3..=5 => 1 + random.below(1000),
            6 => 1 + random.below(6000),
            _ => 1 + random.below(90_000),
        };
        let mut next = (written + size as usize).min(body.len());
        // Half the appends end on a line, so most of those passes catch up.
        if random.below(2) == 0 {
            next = ends[ends.partition_point(|end| *end < next)];
        }
        file.write_all(&body[written..next]).unwrap();
        written = next;
        loop {
            let offset = row.as_ref().map(|row| row.offset);
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            let (mut next, resumed) =
                resume(&fixture.projects, &path, ID, row.take(), now(), deadline).unwrap();
            restarts += usize::from(offset.is_some() && !resumed);
            round_trip(&mut next);
            let done = next.caught_up || Some(next.offset) == offset;
            if next.caught_up && !next.skipping {
                let lines = ends.partition_point(|end| *end <= written);
                let previous = after(lines.saturating_sub(1));
                let published = check(&next, previous, story, lines, label, found);
                if lines > 0 && &published != after(lines) {
                    found.push(format!(
                        "{label}: pass boundary after line {lines}: {published:?} in memory {:?}",
                        after(lines)
                    ));
                }
            }
            row = Some(next);
            if done {
                break;
            }
        }
    }
    restarts
}

#[test]
fn turns_ground_truth_fuzz_never_publishes_a_wrong_value() {
    let started = std::time::Instant::now();
    let (mut found, mut records, mut restarts) = (Found::default(), 0, 0);
    // The last run targets unusable stamps on the lines that prove an end
    // or take input with no turn running, which uniform faults rarely pair.
    for (base, sessions, file, targeted) in [
        (0x9e37_79b9_7f4a_7c15_u64, 3000, false, false),
        (0xc2b2_ae3d_27d4_eb4f, 160, true, false),
        (0x1656_67b1_9e37_79f9, 1000, false, true),
    ] {
        for index in 0..sessions {
            let seed = (base ^ (index + 1_u64).wrapping_mul(0x2545_f491_4f6c_dd1d)) | 1;
            let faults = [0, 10, 40, 120, 250][index as usize % 5];
            let targeted = if targeted {
                [100, 300][index as usize % 2]
            } else {
                0
            };
            let story = story(seed, faults, file, targeted);
            records += story.lines.len();
            let label = format!("seed {seed:#x} faults {faults} file {file} targeted {targeted}");
            let memory = replay_memory(&story, &label, &mut found);
            if file {
                restarts += replay_file(&story, &memory, seed, &label, &mut found);
            }
        }
    }
    let wrong = &found.wrong;
    let [killed, last] = found.allowed;
    let [totals, lasts, currents] = found.known;
    let fields = ["current", "last", "total"].map(|field| {
        (
            field,
            wrong
                .iter()
                .filter(|w| w.contains(&format!(": {field} after")))
                .count(),
        )
    });
    eprintln!(
        "ground-truth turn fuzz: {records} records, {} violations {fields:?}, {restarts} restarts, \
         {killed} unchanged across a kill, {last} last valid intervals, \
         {totals} totals, {lasts} last intervals and {currents} running starts published \
         in {:?}",
        wrong.len(),
        started.elapsed()
    );
    assert!(
        wrong.is_empty() && restarts == 0,
        "{} violations, {restarts} restarts:\n{}",
        wrong.len(),
        wrong[..wrong.len().min(8)].join("\n")
    );
}
