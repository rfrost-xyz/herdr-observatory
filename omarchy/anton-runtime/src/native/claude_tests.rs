//! Claude dispatch through `NativeTelemetry::enrich`, all synthetic.
use super::*;
use std::io::Write;

const ID: &str = "fixture-session-a";
const BASE: u64 = 1_767_225_600;

struct Fixture {
    root: PathBuf,
    projects: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        static SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "anton-native-claude-{}-{}-{}",
            std::process::id(),
            now().to_bits(),
            SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        ));
        let projects = root.join("projects");
        std::fs::create_dir_all(&projects).unwrap();
        claude::TEST_ROOT.with(|value| *value.borrow_mut() = Some(projects.clone()));
        Self { root, projects }
    }
    fn path(&self, entry: &str, id: &str) -> PathBuf {
        let directory = self.projects.join(entry);
        std::fs::create_dir_all(&directory).unwrap();
        directory.join(format!("{id}.jsonl"))
    }
    /// Writes the header and `lines` to a new file, replacing any old inode.
    fn write(&self, lines: &[String]) -> PathBuf {
        let path = self.path("entry-a", ID);
        let _ = std::fs::remove_file(&path);
        std::fs::write(&path, body(lines)).unwrap();
        path
    }
    fn append(&self, text: &str) {
        self.append_to(&self.path("entry-a", ID), text);
    }
    fn append_to(&self, path: &Path, text: &str) {
        let mut file = std::fs::OpenOptions::new().append(true).open(path).unwrap();
        file.write_all(text.as_bytes()).unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        claude::TEST_ROOT.with(|value| *value.borrow_mut() = None);
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
fn body(lines: &[String]) -> String {
    let mut text = format!("{{\"type\":\"permission-mode\",\"sessionId\":\"{ID}\"}}\n");
    for line in lines {
        text.push_str(line);
        text.push('\n');
    }
    text
}
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
fn record(kind: &str, second: u64, fields: &str) -> String {
    format!(
        "{{\"type\":\"{kind}\",\"sessionId\":\"{ID}\",\"uuid\":\"{kind}-{second}\",\"timestamp\":\"{}\",{fields}}}",
        stamp(second)
    )
}
fn prompt(second: u64) -> String {
    record(
        "user",
        second,
        "\"message\":{\"role\":\"user\",\"content\":\"synthetic\"}",
    )
}
fn launch(second: u64, agent: &str) -> String {
    record(
        "user",
        second,
        &format!(
            "\"toolUseResult\":{{\"status\":\"async_launched\",\"agentId\":\"{agent}\"}},\"message\":{{\"role\":\"user\",\"content\":[{{\"type\":\"tool_result\",\"content\":\"done\"}}]}}"
        ),
    )
}
fn notified(second: u64, agent: &str, status: &str) -> String {
    let text = format!(
        "<task-notification>\n<task-id>{agent}</task-id>\n<status>{status}</status>\n</task-notification>"
    );
    record(
        "user",
        second,
        &format!(
            "\"origin\":{{\"kind\":\"task-notification\"}},\"message\":{{\"role\":\"user\",\"content\":{}}}",
            serde_json::to_string(&text).unwrap()
        ),
    )
}
fn assistant(second: u64, message: &str, stop: &str, usage: [u64; 4]) -> String {
    record(
        "assistant",
        second,
        &format!(
            "\"message\":{{\"id\":\"{message}\",\"model\":\"claude-fixture-1\",\"stop_reason\":{stop},\"usage\":{{\"input_tokens\":{},\"output_tokens\":{},\"cache_read_input_tokens\":{},\"cache_creation_input_tokens\":{}}},\"content\":[]}}",
            usage[0], usage[1], usage[2], usage[3]
        ),
    )
}
fn system(subtype: &str, second: u64) -> String {
    record("system", second, &format!("\"subtype\":\"{subtype}\""))
}
fn agent() -> Value {
    json!({"agent":"claude","agent_session":{"agent":"claude","source":"herdr:claude","kind":"id","value":ID}})
}
fn key() -> String {
    sha256(format!("anton-native-session-v1:claude:{ID}").as_bytes())
}
/// One turn with two counted groups, a completed and a blocked child.
fn session() -> Vec<String> {
    vec![
        prompt(10),
        launch(11, "agent-a"),
        launch(12, "agent-b"),
        assistant(13, "msg-1", "\"tool_use\"", [100, 20, 1000, 50]),
        assistant(14, "msg-2", "\"end_turn\"", [10, 5, 1100, 0]),
        system("turn_duration", 15),
        system("compact_boundary", 16),
        notified(20, "agent-a", "completed"),
        notified(21, "agent-b", "blocked"),
        assistant(22, "msg-3", "\"end_turn\"", [1, 1, 1200, 0]),
        system("turn_duration", 23),
    ]
}
/// Enriches one Claude pane and returns its telemetry, timing and cursors.
fn enrich(follower: &mut NativeTelemetry, cursors: &Value) -> (Value, Value, Value) {
    let mut agents = vec![agent()];
    let cursors = follower.enrich(&mut agents, cursors);
    let agent = agents.pop().unwrap();
    (
        agent["_native_telemetry"].clone(),
        agent["_native_turn_timing"].clone(),
        cursors,
    )
}
fn totals(telemetry: &Value) -> Value {
    json!([
        telemetry["total_input"],
        telemetry["total_uncached_input"],
        telemetry["total_cache_read"],
        telemetry["total_cache_write"],
        telemetry["total_output"]
    ])
}

#[test]
fn claude_pane_enrich_publishes_caught_up_sample_and_block_row() {
    let fixture = Fixture::new();
    fixture.write(&session());
    let mut follower = NativeTelemetry::default();
    let (telemetry, timing, cursors) = enrich(&mut follower, &json!({}));
    let row = &cursors[key()];
    assert_eq!(cursors.as_object().unwrap().len(), 1);
    assert_eq!(row["caught_up"], true);
    assert_eq!(row["claude"]["coverage_seq"], json!(micros(23)));
    assert!(row.get("envelope").is_none());
    assert_eq!(row["compaction_markers"], 0);
    assert_eq!(row["seq"], json!(micros(21)));
    assert_eq!(row["claude"]["usage_seq"], json!(micros(22)));
    assert_eq!(
        telemetry,
        json!({
            "cache_read": 1200, "cache_write": 0, "compactions": 1, "context": 1201,
            "event": "session", "input": 1201,
            "model": "claude-fixture-1", "output_tokens": 1, "phase": "ready", "result": null,
            "seq": micros(23), "subagent_completed": 1, "subagent_done": 1,
            "subagent_failed": 0, "subagent_interrupted": 0, "subagent_running": 0,
            "subagent_seq": null, "subagent_starts": null, "subagent_status_seq": micros(21),
            "subagent_stops": null, "subagent_total": 2, "subagent_unknown": 1, "tool": null,
            "total_cache_read": 3300, "total_cache_write": 50, "total_input": 3461,
            "total_output": 26, "total_uncached_input": 111, "usage_seq": micros(22),
            "usage_source": "claude-transcript"
        })
    );
    // The second notification replaces the first one's pending start, which
    // may be a running turn it joined (D7): only the first turn is known.
    let mut timing = timing;
    timing["observed_at_s"] = json!(0);
    assert_eq!(
        timing,
        json!({"active": null, "complete": false, "last_duration_s": 5,
            "last_outcome": "completed", "observed_at_s": 0, "started_at_s": null,
            "total_finished_duration_s": null})
    );
    // A warm pass resumes the block and publishes the same sample.
    let (again, _, warm) = enrich(&mut follower, &cursors);
    assert_eq!(again, telemetry);
    assert_eq!(warm[key()]["claude"], row["claude"]);
    // A Codex pane with the same id has its own key and is not given the block.
    let mut agents = vec![
        agent(),
        json!({"agent":"codex","agent_session":{"agent":"codex","source":"herdr:codex","kind":"id","value":ID}}),
    ];
    let mixed = follower.enrich(&mut agents, &cursors);
    assert_eq!(mixed.as_object().unwrap().len(), 1);
    assert!(agents[1].get("_native_telemetry").is_none());
}

#[test]
fn claude_caught_up_pass_publishes_an_all_null_sample_at_coverage_seq() {
    let fixture = Fixture::new();
    // No counted group and no child: every value is unknown or zero children.
    fixture.write(&[prompt(10), system("stop_hook_summary", 12)]);
    let (telemetry, _, cursors) = enrich(&mut NativeTelemetry::default(), &json!({}));
    assert_eq!(cursors[key()]["claude"]["coverage_seq"], json!(micros(12)));
    assert_eq!(telemetry["seq"], json!(micros(12)));
    assert_eq!(telemetry["event"], "session");
    assert_eq!(telemetry["phase"], "ready");
    assert_eq!(telemetry["subagent_total"], 0);
    assert_eq!(telemetry["subagent_status_seq"], json!(micros(12)));
    for key in [
        "total_input",
        "input",
        "context",
        "model",
        "usage_seq",
        "usage_source",
        "compactions",
    ] {
        assert!(telemetry[key].is_null(), "{key}");
    }
    // Without any source time there is nothing honest to stamp.
    fixture.write(&[]);
    let (telemetry, _, cursors) = enrich(&mut NativeTelemetry::default(), &json!({}));
    assert_eq!(cursors[key()]["caught_up"], true);
    assert!(telemetry.is_null());
}

/// The all-null sample a pane publishes when its bound file cannot be bound
/// or verified, stamped with the incoming row's `coverage_seq` (D3).
fn unknown(telemetry: &Value, seq: u64) {
    assert_eq!(telemetry["seq"], json!(seq));
    assert_eq!(telemetry["event"], "session");
    for (key, value) in telemetry.as_object().unwrap() {
        if !["seq", "event", "phase"].contains(&key.as_str()) {
            assert!(value.is_null(), "{key}");
        }
    }
}
/// The retained numeric subset as a re-emitted sample shows it.
fn retained(telemetry: &Value) -> Value {
    let mut value = telemetry.clone();
    for key in [
        "subagent_total",
        "subagent_done",
        "subagent_status_seq",
        "compactions",
    ]
    .iter()
    .chain(telemetry::OUTCOMES)
    {
        value[*key] = Value::Null;
    }
    value
}

#[test]
fn claude_incomplete_replay_reemits_the_retained_sample_until_replacement() {
    let fixture = Fixture::new();
    fixture.write(&session());
    let mut follower = NativeTelemetry::default();
    let (first, _, cursors) = enrich(&mut follower, &json!({}));
    // A partial trailing line leaves the resumed replay incomplete.
    let partial = assistant(30, "msg-4", "\"end_turn\"", [7, 7, 7, 7]);
    fixture.append(&partial[..40]);
    let (telemetry, timing, cursors) = enrich(&mut follower, &cursors);
    assert_eq!(cursors[key()]["caught_up"], false);
    assert_eq!(telemetry, retained(&first));
    assert!(timing.is_null());
    // Growth keeps it.
    fixture.append(&partial[40..60]);
    let (telemetry, _, cursors) = enrich(&mut follower, &cursors);
    assert_eq!(telemetry, retained(&first));
    // Replacement by a new inode with the same bytes restarts and drops it.
    let bytes = std::fs::read(fixture.path("entry-a", ID)).unwrap();
    std::fs::remove_file(fixture.path("entry-a", ID)).unwrap();
    std::fs::write(fixture.path("entry-a", ID), &bytes).unwrap();
    let (telemetry, _, cursors) = enrich(&mut follower, &cursors);
    unknown(&telemetry, micros(23));
    // It stays dropped on the next resumed incomplete pass.
    let (telemetry, _, cursors) = enrich(&mut follower, &cursors);
    assert!(telemetry.is_null());
    // Completing the line catches up and publishes, which retains again.
    fixture.append(&format!("{}\n", &partial[60..]));
    let (caught, _, cursors) = enrich(&mut follower, &cursors);
    assert_eq!(caught["total_output"], 33);
    assert_eq!(caught["seq"], json!(micros(30)));
    fixture.append(&partial[..10]);
    let (telemetry, _, _) = enrich(&mut follower, &cursors);
    assert_eq!(telemetry, retained(&caught));
    // A pass with no incoming cursor row restarts and does not re-emit.
    let (telemetry, _, _) = enrich(&mut follower, &json!({}));
    assert!(telemetry.is_null());
}

/// Saves `cursors` for one host through a real checkpoint file and loads it.
fn checkpointed(fixture: &Fixture, cursors: &Value) -> Value {
    let owner = fixture.root.join(".herdr-observatory-install");
    std::fs::write(&owner, b"herdr.observatory\n").unwrap();
    let state = fixture.root.join("state");
    let _ = std::fs::create_dir(&state);
    let mut cache = Checkpoints::new(&state, &owner).unwrap();
    cache
        .update(
            &BTreeMap::from([("test".to_owned(), cursors.clone())]),
            true,
        )
        .unwrap();
    let saved = std::fs::read_to_string(state.join("replay-checkpoints.json")).unwrap();
    assert!(!saved.contains(ID) && !saved.contains("entry-a"));
    drop(cache);
    Checkpoints::new(&state, &owner).unwrap().for_host("test")
}

#[test]
fn claude_checkpoint_round_trip_resumes_the_block_and_adds_new_groups() {
    let fixture = Fixture::new();
    fixture.write(&session());
    let (first, _, cursors) = enrich(&mut NativeTelemetry::default(), &json!({}));
    let loaded = checkpointed(&fixture, &cursors);
    assert_eq!(loaded, cursors);
    let offset = cursors[key()]["offset"].as_u64().unwrap();
    fixture.append(&format!(
        "{}\n",
        assistant(30, "msg-4", "\"end_turn\"", [5, 6, 7, 8])
    ));
    // A fresh process: no binding, no retained sample, only the checkpoint.
    let (telemetry, _, resumed) = enrich(&mut NativeTelemetry::default(), &loaded);
    assert!(resumed[key()]["offset"].as_u64().unwrap() > offset);
    assert_eq!(totals(&first), json!([3461, 111, 3300, 50, 26]));
    assert_eq!(totals(&telemetry), json!([3481, 116, 3307, 58, 32]));
    assert_eq!(telemetry["seq"], json!(micros(30)));
    // A marked closed sum shows the block was resumed, not replayed.
    let mut marked = loaded.clone();
    marked[key()]["claude"]["output"] =
        json!(1000 + loaded[key()]["claude"]["output"].as_u64().unwrap());
    let (telemetry, _, _) = enrich(&mut NativeTelemetry::default(), &marked);
    assert_eq!(telemetry["total_output"], 1032);
}

#[test]
fn claude_rows_without_a_valid_block_replay_fresh_from_the_header() {
    let fixture = Fixture::new();
    fixture.write(&session());
    let (_, _, cursors) = enrich(&mut NativeTelemetry::default(), &json!({}));
    fixture.append(&format!(
        "{}\n",
        assistant(30, "msg-4", "\"end_turn\"", [5, 6, 7, 8])
    ));
    // An old binary re-serialises the row through a `Cursor` without the
    // block. With an `unknown` child it reads as an invalid Codex row and is
    // dropped; otherwise it stays valid and survives a checkpoint, but is not
    // resumed.
    let mut stripped = cursors.clone();
    stripped[key()].as_object_mut().unwrap().remove("claude");
    assert!(validate_cursors(&stripped).is_empty());
    for status in stripped[key()]["children"]
        .as_object_mut()
        .unwrap()
        .values_mut()
    {
        if status == "unknown" {
            *status = json!("completed");
        }
    }
    assert_eq!(validate_cursors(&stripped).len(), 1);
    let stripped = checkpointed(&fixture, &stripped);
    assert!(stripped[key()].get("claude").is_none());
    let (telemetry, _, again) = enrich(&mut NativeTelemetry::default(), &stripped);
    assert_eq!(totals(&telemetry), json!([3481, 116, 3307, 58, 32]));
    assert_eq!(again[key()]["claude"]["coverage_seq"], json!(micros(30)));
    // An invalid block drops the whole row before replay.
    let future = (now() as u64 + 60) * 1_000_000;
    for (field, value) in [
        ("coverage_seq", json!(future)),
        ("closed", json!(["not-a-hash"])),
        ("output", json!(1u64 << 60)),
    ] {
        let mut tampered = cursors.clone();
        tampered[key()]["claude"][field] = value;
        assert!(validate_cursors(&tampered).is_empty(), "{field}");
        let (telemetry, _, _) = enrich(&mut NativeTelemetry::default(), &tampered);
        assert_eq!(
            totals(&telemetry),
            json!([3481, 116, 3307, 58, 32]),
            "{field}"
        );
    }
    // A Claude block on a row that breaks the Claude row invariants is invalid.
    for (field, value) in [("compaction_markers", json!(1)), ("turns", Value::Null)] {
        let mut tampered = cursors.clone();
        tampered[key()][field] = value;
        assert!(validate_cursors(&tampered).is_empty(), "{field}");
    }
}

/// Lets the next pass rediscover the binding, as after 60 s.
fn age(follower: &mut NativeTelemetry) {
    follower.claude.get_mut(&key()).unwrap().at =
        Instant::now().checked_sub(Duration::from_secs(61)).unwrap();
}

#[test]
fn claude_positive_binding_is_rediscovered_and_ambiguity_is_unknown() {
    let fixture = Fixture::new();
    fixture.write(&session());
    let mut follower = NativeTelemetry::default();
    let (first, _, cursors) = enrich(&mut follower, &json!({}));
    assert!(first.is_object());
    // A second match within 60 s is not seen; the binding is cached.
    let other = fixture.path("entry-b", ID);
    std::fs::write(&other, body(&session())).unwrap();
    let (telemetry, _, cursors) = enrich(&mut follower, &cursors);
    assert_eq!(telemetry, first);
    // The re-scan finds two matches: unknown, and the retained sample is gone.
    age(&mut follower);
    let (telemetry, timing, cursors) = enrich(&mut follower, &cursors);
    unknown(&telemetry, micros(23));
    assert!(timing.is_null());
    assert!(follower.claude[&key()].retained.is_none());
    // Back to one match with an incomplete replay: nothing is re-emitted.
    std::fs::remove_file(&other).unwrap();
    fixture.append("{\"type\":");
    age(&mut follower);
    let (telemetry, _, cursors) = enrich(&mut follower, &cursors);
    assert_eq!(cursors[key()]["caught_up"], false);
    assert!(telemetry.is_null());
    // A file moved to another entry is rebound at the next re-scan. It keeps
    // its inode, header and tail, so the row resumes (a marked closed sum
    // survives) and catches up.
    std::fs::rename(fixture.path("entry-a", ID), &other).unwrap();
    fixture.append_to(&other, "\"system\"}\n");
    age(&mut follower);
    let mut marked = cursors.clone();
    let output = cursors[key()]["claude"]["output"].as_u64().unwrap();
    marked[key()]["claude"]["output"] = json!(output + 1000);
    let (telemetry, _, cursors) = enrich(&mut follower, &marked);
    assert_eq!(cursors[key()]["caught_up"], true);
    assert_eq!(
        follower.claude[&key()].path.as_deref(),
        Some(other.as_path())
    );
    assert_eq!(
        telemetry["total_output"],
        json!(first["total_output"].as_u64().unwrap() + 1000)
    );
}

#[test]
fn claude_lost_binding_with_a_cursor_row_publishes_an_all_null_sample() {
    let fixture = Fixture::new();
    let path = fixture.write(&session());
    let (_, _, cursors) = enrich(&mut NativeTelemetry::default(), &json!({}));
    std::fs::remove_file(&path).unwrap();
    // A fresh follower, as on a peer, with the row the local sent back.
    let (telemetry, timing, kept) = enrich(&mut NativeTelemetry::default(), &cursors);
    unknown(&telemetry, micros(23));
    assert!(timing.is_null());
    assert_eq!(kept, cursors);
    // Without a row there is no source time to stamp.
    let (telemetry, _, rows) = enrich(&mut NativeTelemetry::default(), &json!({}));
    assert!(telemetry.is_null());
    assert_eq!(rows, json!({}));
}

#[test]
fn claude_zero_matches_and_identity_failures_publish_nothing() {
    let fixture = Fixture::new();
    let mut follower = NativeTelemetry::default();
    let (telemetry, _, cursors) = enrich(&mut follower, &json!({}));
    assert!(telemetry.is_null());
    assert_eq!(cursors, json!({}));
    // A header naming another session fails identity: no sample, no row.
    let path = fixture.path("entry-a", ID);
    std::fs::write(
        &path,
        format!(
            "{{\"type\":\"permission-mode\",\"sessionId\":\"fixture-other\"}}\n{}\n",
            prompt(10)
        ),
    )
    .unwrap();
    age(&mut follower);
    let (telemetry, _, cursors) = enrich(&mut follower, &json!({}));
    assert!(telemetry.is_null());
    assert_eq!(cursors, json!({}));
    // A non-Claude source or a path binding is not a Claude pane.
    let mut agents = vec![
        json!({"agent":"claude","agent_session":{"agent":"claude","source":"herdr:codex","kind":"id","value":ID}}),
        json!({"agent":"claude","agent_session":{"agent":"claude","source":"herdr:claude","kind":"path","value":ID}}),
    ];
    fixture.write(&session());
    assert_eq!(follower.enrich(&mut agents, &json!({})), json!({}));
    assert!(agents.iter().all(|v| v.get("_native_telemetry").is_none()));
}

#[test]
fn claude_rows_share_the_32_row_checkpoint_bound_with_hashed_keys() {
    let fixture = Fixture::new();
    fixture.write(&session());
    let (_, _, cursors) = enrich(&mut NativeTelemetry::default(), &json!({}));
    let row = cursors[key()].clone();
    // Each host sends at most 32 rows; the file keeps the newest 32 overall.
    let mut hosts = BTreeMap::new();
    for host in ["host-a", "host-b"] {
        let mut rows = serde_json::Map::new();
        for index in 0..17 {
            let mut cursor = row.clone();
            let order = if host == "host-a" { index } else { 17 + index };
            cursor["at"] = json!(now() - 100.0 + order as f64);
            rows.insert(sha256(format!("{host}-{index}").as_bytes()), cursor);
        }
        hosts.insert(host.to_owned(), Value::Object(rows));
    }
    let owner = fixture.root.join(".herdr-observatory-install");
    std::fs::write(&owner, b"herdr.observatory\n").unwrap();
    let state = fixture.root.join("state");
    std::fs::create_dir(&state).unwrap();
    Checkpoints::new(&state, &owner)
        .unwrap()
        .update(&hosts, true)
        .unwrap();
    let loaded = Checkpoints::new(&state, &owner).unwrap();
    let (a, b) = (loaded.for_host("host-a"), loaded.for_host("host-b"));
    assert_eq!(
        a.as_object().unwrap().len() + b.as_object().unwrap().len(),
        32
    );
    assert!(a.get(sha256(b"host-a-0")).is_none() && a.get(sha256(b"host-a-1")).is_none());
    assert_eq!(a[sha256(b"host-a-2")]["claude"], row["claude"]);
    assert_eq!(b.as_object().unwrap().len(), 17);
}

/// A caught-up Codex row as `codex_only_enrich_cursor_and_checkpoint_are_unchanged`
/// pins it, stamped `at`.
fn codex_row(at: f64) -> Value {
    json!({
        "at": at, "caught_up": true,
        "children": {"e75c94502a7fbb74b08bc4ffc5219f31d5d16266272e870171adc0310a3e01f7": "completed"},
        "compaction_markers": 1, "compaction_summaries": 1, "compactions_valid": true,
        "file": [1, 2],
        "fingerprint": {"header": "b35d3bda40a5e3ad26bf99af603b7a02d858f8ca2b6a2a1f96fc1879d77f627c", "mtime_us": 1, "size": 1317, "tail": "850b6772e93a8ef47b0c76a3da0a96bebdeb489af8bdda25e0ece15537eaa6a8"},
        "offset": 1317, "seq": 1_767_225_625_000_000_u64, "skipping": false,
        "turns": {"active": null, "current_known": true, "finished": {"4ea1a6a42fdcde0801691c1a": [1767225601, 1767225620, "completed"]}, "last": "4ea1a6a42fdcde0801691c1a", "last_duration": 19, "last_end": 1767225620, "last_outcome": "completed", "start": null, "supported": true, "total": 19, "valid": true},
        "valid": true
    })
}

#[test]
fn child_status_unknown_is_accepted_on_claude_rows_only() {
    let fixture = Fixture::new();
    fixture.write(&session());
    let (_, _, cursors) = enrich(&mut NativeTelemetry::default(), &json!({}));
    let claude = cursors[key()].clone();
    assert!(
        claude["children"]
            .as_object()
            .unwrap()
            .values()
            .any(|v| v == "unknown")
    );
    let mut codex = codex_row(now());
    let codex_key = sha256(b"anton-native-session-v1:fixture-codex");
    let rows = json!({key(): claude, codex_key.clone(): codex.clone()});
    assert_eq!(validate_cursors(&rows).len(), 2);
    // Only Claude rows produce `unknown`: a Codex row carrying it is replayed fresh.
    codex["children"]["e75c94502a7fbb74b08bc4ffc5219f31d5d16266272e870171adc0310a3e01f7"] =
        json!("unknown");
    let rows = json!({key(): claude, codex_key.clone(): codex});
    let valid = validate_cursors(&rows);
    assert!(valid.contains_key(&key()) && !valid.contains_key(&codex_key));
}

#[test]
fn claude_truncated_predecessor_scan_is_rescanned_not_cached() {
    let fixture = Fixture::new();
    fixture.write(&session());
    // Discovery takes one entry; the predecessor scan of the bound file's
    // directory then exhausts the shared 8192-entry budget.
    let directory = fixture.projects.join("entry-a");
    for index in 0..8200 {
        std::fs::write(directory.join(format!("pad-{index}")), b"").unwrap();
    }
    let mut follower = NativeTelemetry::default();
    let (telemetry, _, cursors) = enrich(&mut follower, &json!({}));
    assert!(telemetry.is_null() && cursors == json!({}));
    let binding = &follower.claude[&key()];
    assert!(binding.path.is_none() && binding.rescan);
    // The next pass, well within 60 s, rescans and binds.
    for index in 0..8200 {
        std::fs::remove_file(directory.join(format!("pad-{index}"))).unwrap();
    }
    let (telemetry, _, _) = enrich(&mut follower, &json!({}));
    assert_eq!(totals(&telemetry), json!([3461, 111, 3300, 50, 26]));
}

/// Fills `row` to the 128-child cap, about 11 KB.
fn padded(row: &Value) -> Value {
    let mut row = row.clone();
    let children = row["children"].as_object_mut().unwrap();
    for index in 0.. {
        if children.len() == 128 {
            break;
        }
        children.insert(
            sha256(format!("child-{index}").as_bytes()),
            json!("completed"),
        );
    }
    row
}

#[test]
fn codex_rows_are_charged_before_claude_rows_in_cursor_and_checkpoint_bounds() {
    let fixture = Fixture::new();
    fixture.write(&session());
    let (_, _, cursors) = enrich(&mut NativeTelemetry::default(), &json!({}));
    let claude = cursors[key()].clone();
    // Claude keys sort before Codex keys, so key order alone favours Claude.
    let claude_key = |index: usize| format!("{index:064x}");
    let codex_key = |index: usize| format!("f{index:063x}");
    let rows = |count: usize, pad: bool, at: f64| {
        let mut rows = serde_json::Map::new();
        let (codex, claude) = if pad {
            (padded(&codex_row(at)), padded(&claude))
        } else {
            (codex_row(at), claude.clone())
        };
        for index in 0..count {
            rows.insert(claude_key(index), claude.clone());
            rows.insert(codex_key(index), codex.clone());
        }
        rows
    };
    // One host: 16 padded Codex rows fit alone; with 16 Claude rows they do not.
    let mixed = Value::Object(rows(16, true, now()));
    let valid = validate_cursors(&mixed);
    assert!((0..16).all(|index| valid.contains_key(&codex_key(index))));
    assert!(valid.values().any(Cursor::is_claude));
    assert!(serde_json::to_vec(&valid).unwrap().len() <= LIMIT);
    // Checkpoints: Codex rows are older, yet Claude rows are evicted first,
    // for the 256 KiB bound and for the 32-row bound.
    let owner = fixture.root.join(".herdr-observatory-install");
    std::fs::write(&owner, b"herdr.observatory\n").unwrap();
    let state = fixture.root.join("state");
    std::fs::create_dir(&state).unwrap();
    for (count, pad) in [(16, true), (20, false)] {
        let split = |claude: bool, at: f64| {
            let rows = rows(count, pad, at)
                .into_iter()
                .filter(|(_, row)| row.get("claude").is_some() == claude)
                .collect();
            Value::Object(rows)
        };
        let hosts = BTreeMap::from([
            ("codex-host".to_owned(), split(false, now() - 100.0)),
            ("claude-host".to_owned(), split(true, now() - 10.0)),
        ]);
        Checkpoints::new(&state, &owner)
            .unwrap()
            .update(&hosts, true)
            .unwrap();
        let loaded = Checkpoints::new(&state, &owner).unwrap();
        let codex = loaded.for_host("codex-host");
        assert_eq!(codex.as_object().unwrap().len(), count, "{count}");
        assert!(
            !loaded
                .for_host("claude-host")
                .as_object()
                .unwrap()
                .is_empty()
        );
    }
}

#[test]
fn claude_deadline_skip_reemits_the_retained_sample_of_a_current_binding() {
    let fixture = Fixture::new();
    fixture.write(&session());
    let mut follower = NativeTelemetry::default();
    let (first, _, cursors) = enrich(&mut follower, &json!({}));
    // The shared deadline has passed before this pane is reached.
    let skipped = |follower: &mut NativeTelemetry, cursors: &Value| {
        let mut rows = validate_cursors(cursors);
        let mut active = BTreeSet::new();
        let mut agent = agent();
        follower.enrich_claude(&mut agent, &mut rows, &mut active, now(), Instant::now());
        assert!(active.contains(&key()));
        (
            agent["_native_telemetry"].clone(),
            rows.contains_key(&key()),
        )
    };
    assert_eq!(skipped(&mut follower, &cursors), (retained(&first), true));
    // Without a cursor row the retained sample is not re-emitted.
    assert_eq!(skipped(&mut follower, &json!({})), (Value::Null, false));
    assert!(follower.claude[&key()].retained.is_some());
    // A binding due for rediscovery is unverified, so nothing is re-emitted.
    age(&mut follower);
    assert_eq!(skipped(&mut follower, &cursors), (Value::Null, true));
}

/// A rediscovery that the shared deadline cuts short is a deadline skip,
/// not a truncated discovery: nothing is published, the cursor row is kept
/// and a retained sample survives for the next pass. The pad entries stay
/// well under the entry budget, so only the deadline truncates the scan.
#[test]
fn claude_deadline_inside_rediscovery_is_a_skip_not_a_drop() {
    let fixture = Fixture::new();
    fixture.write(&session());
    for index in 0..6000 {
        std::fs::create_dir(fixture.projects.join(format!("pad-{index:05}"))).unwrap();
    }
    let (first, _, cursors) = enrich(&mut NativeTelemetry::default(), &json!({}));
    assert_eq!(first["total_output"], 26);
    let expiring = |follower: &mut NativeTelemetry| {
        let mut rows = validate_cursors(&cursors);
        let mut agent = agent();
        let deadline = Instant::now() + Duration::from_micros(300);
        follower.enrich_claude(&mut agent, &mut rows, &mut BTreeSet::new(), now(), deadline);
        (
            agent["_native_telemetry"].clone(),
            rows.contains_key(&key()),
        )
    };
    // A peer: a fresh follower with the caught-up row the local sent back.
    let mut peer = NativeTelemetry::default();
    assert_eq!(expiring(&mut peer), (Value::Null, true));
    assert!(peer.claude.is_empty());
    // A local binding due for rediscovery keeps its retained sample, which
    // the next incomplete pass re-emits.
    let mut follower = NativeTelemetry::default();
    let (_, _, cursors) = enrich(&mut follower, &cursors);
    age(&mut follower);
    assert_eq!(expiring(&mut follower), (Value::Null, true));
    assert!(follower.claude[&key()].retained.is_some());
    fixture.append("{\"type\":");
    let (telemetry, _, rows) = enrich(&mut follower, &cursors);
    assert_eq!(rows[key()]["caught_up"], false);
    assert_eq!(telemetry, retained(&first));
}

#[test]
fn claude_record_naming_another_session_keeps_the_binding_unknown() {
    let fixture = Fixture::new();
    fixture.write(&session());
    let mut follower = NativeTelemetry::default();
    let (first, _, cursors) = enrich(&mut follower, &json!({}));
    assert_eq!(first["total_output"], 26);
    // A foreign record in an incomplete pass drops the retained sample and
    // publishes the all-null one.
    let other = prompt(24).replace(ID, "fixture-session-b");
    let later = [
        prompt(30),
        launch(31, "agent-c"),
        assistant(32, "msg-4", "\"end_turn\"", [1, 1, 1, 1]),
        system("compact_boundary", 33),
        system("turn_duration", 34),
    ];
    let partial = &later[0][..30];
    fixture.append(&format!("{other}\n{partial}"));
    let (telemetry, timing, cursors) = enrich(&mut follower, &cursors);
    assert_eq!(cursors[key()]["caught_up"], false);
    unknown(&telemetry, micros(23));
    assert!(timing.is_null());
    // Later complete groups, children, compactions and turns stay unknown.
    let rest = body(&later).split_once('\n').unwrap().1.to_owned();
    fixture.append(&format!("{}\n{rest}", &later[0][30..]));
    let (telemetry, timing, cursors) = enrich(&mut follower, &cursors);
    assert_eq!(cursors[key()]["caught_up"], true);
    unknown(&telemetry, micros(34));
    assert!(timing.is_null());
    // A fresh replay of the same file finds the record again.
    let (telemetry, timing, _) = enrich(&mut NativeTelemetry::default(), &json!({}));
    unknown(&telemetry, micros(34));
    assert!(timing.is_null());
}

/// Seven maximal Claude panes, 512 finished turns and 128 children each, do
/// not fit `LIMIT` whole. The rows that would overflow shed their finished
/// intervals instead of being dropped, so every pane keeps its position and
/// keeps publishing usage on every sample; only their accumulated turn
/// coverage becomes unknown.
#[test]
fn claude_rows_over_the_byte_bound_shrink_and_keep_their_position() {
    let fixture = Fixture::new();
    let mut lines: Vec<_> = (0..128)
        .map(|index| launch(5 + index, &format!("agent-{index}")))
        .collect();
    for turn in 0..512 {
        let second = 200 + turn * 5;
        lines.push(prompt(second));
        lines.push(assistant(
            second + 1,
            &format!("msg-{turn}"),
            "\"end_turn\"",
            [1, 1, 1, 1],
        ));
        lines.push(system("turn_duration", second + 2));
    }
    let text = body(&lines);
    let panes: Vec<_> = (0..7)
        .map(|index| {
            let id = format!("fixture-session-{index}");
            let path = fixture.path(&format!("entry-{index}"), &id);
            std::fs::write(&path, text.replace(ID, &id)).unwrap();
            let key = sha256(format!("anton-native-session-v1:claude:{id}").as_bytes());
            (id, path, key)
        })
        .collect();
    let agents = || -> Vec<_> {
        panes
            .iter()
            .map(|(id, _, _)| json!({"agent":"claude","agent_session":{"agent":"claude","source":"herdr:claude","kind":"id","value":id}}))
            .collect()
    };
    let mut follower = NativeTelemetry::default();
    // The cold replay shares the 750 ms deadline, so a loaded test host may
    // need more than one sample to catch every pane up.
    let mut cursors = json!({});
    for _ in 0..20 {
        cursors = follower.enrich(&mut agents(), &cursors);
        let rows = cursors.as_object().unwrap();
        if panes.iter().all(|(_, path, key)| {
            rows.get(key).is_some_and(|row| {
                row["offset"].as_u64() == Some(std::fs::metadata(path).unwrap().len())
            })
        }) {
            break;
        }
    }
    let mut fingerprints = None;
    for round in 0..3 {
        let mut agents = agents();
        cursors = follower.enrich(&mut agents, &cursors);
        let rows = cursors.as_object().unwrap();
        assert_eq!(rows.len(), 7, "round {round}");
        assert!(serde_json::to_vec(&cursors).unwrap().len() <= LIMIT);
        assert_eq!(
            serde_json::to_value(validate_cursors(&cursors)).unwrap(),
            cursors
        );
        let mut shrunk = 0;
        for ((id, path, key), agent) in panes.iter().zip(&agents) {
            let row = &rows[key];
            let size = std::fs::metadata(path).unwrap().len();
            assert_eq!(
                (row["offset"].as_u64(), &row["caught_up"]),
                (Some(size), &json!(true))
            );
            assert_eq!(
                totals(&agent["_native_telemetry"]),
                json!([1536, 512, 512, 512, 512]),
                "round {round} {id}"
            );
            let timing = &agent["_native_turn_timing"];
            assert_eq!(timing["last_duration_s"], json!(2));
            if row["turns"]["finished"].as_object().unwrap().len() == 512 {
                assert_eq!(timing["complete"], json!(true));
            } else {
                shrunk += 1;
                assert_eq!(row["turns"]["finished"].as_object().unwrap().len(), 1);
                assert_eq!(
                    (&timing["complete"], &timing["total_finished_duration_s"]),
                    (&json!(false), &Value::Null)
                );
            }
            // The row resumes from its offset instead of replaying the file.
            let cursor = validate_cursors(&cursors).remove(key).unwrap();
            let (next, resumed) = claude::resume(
                &claude::projects_root(),
                path,
                id,
                cursor.into_row(),
                now(),
                Instant::now() + Duration::from_secs(5),
            )
            .unwrap();
            assert!(resumed && next.offset == size, "round {round} {id}");
        }
        assert_eq!(shrunk, 2, "round {round}");
        let current: Vec<_> = rows
            .values()
            .map(|row| row["fingerprint"].clone())
            .collect();
        assert_eq!(fingerprints.get_or_insert(current.clone()), &current);
    }
}

/// Codex panes are enriched before Claude panes within the shared deadline,
/// so Claude replay listed first cannot starve them. Each probe uses a fresh
/// follower and rows without their `claude` block, as a peer with an old
/// local does, so every Claude pane replays from the header.
#[test]
fn claude_replay_listed_first_never_starves_a_codex_pane() {
    const PANES: usize = 2;
    struct Codex;
    impl Drop for Codex {
        fn drop(&mut self) {
            TEST_ROOT.with(|value| *value.borrow_mut() = None);
        }
    }
    let fixture = Fixture::new();
    let sessions = fixture.root.join("codex-sessions");
    std::fs::create_dir_all(&sessions).unwrap();
    let usage = json!({"type":"event_msg","timestamp":"2026-01-01T00:00:30+00:00","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":12,"output_tokens":3,"cached_input_tokens":0,"cache_write_input_tokens":4,"total_tokens":185000},"total_token_usage":{"input_tokens":21700000,"output_tokens":4200,"cached_input_tokens":20000000,"cache_write_input_tokens":0},"model_context_window":258400}}});
    std::fs::write(
        sessions.join("rollout-fixture-codex.jsonl"),
        format!(
            "{}\n{usage}\n",
            json!({"type":"session_meta","payload":{"id":"fixture-codex"}})
        ),
    )
    .unwrap();
    TEST_ROOT.with(|value| *value.borrow_mut() = Some(sessions));
    let _codex = Codex;
    // Each Claude pane holds more than the 16 passes of `TAIL` it may replay.
    let filler = "y".repeat(3000);
    let ids: Vec<String> = (0..PANES)
        .map(|index| format!("fixture-heavy-{index}"))
        .collect();
    for id in &ids {
        let mut text = format!("{{\"type\":\"permission-mode\",\"sessionId\":\"{id}\"}}\n");
        for index in 0.. {
            if text.len() > 16 * TAIL + TAIL {
                break;
            }
            text.push_str(&format!(
                "{{\"type\":\"assistant\",\"sessionId\":\"{id}\",\"uuid\":\"u-{index}\",\"timestamp\":\"{}\",\"message\":{{\"id\":\"msg-{index}\",\"model\":\"claude-fixture-1\",\"stop_reason\":\"tool_use\",\"usage\":{{\"input_tokens\":1,\"output_tokens\":1,\"cache_read_input_tokens\":1,\"cache_creation_input_tokens\":1}},\"content\":[{{\"type\":\"text\",\"text\":\"{filler}\"}}]}}}}\n",
                stamp(index % 3600)
            ));
        }
        let directory = fixture.projects.join(format!("entry-{id}"));
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(directory.join(format!("{id}.jsonl")), text).unwrap();
    }
    let codex = json!({"agent":"codex","agent_session":{"agent":"codex","source":"herdr:codex","kind":"id","value":"fixture-codex"}});
    let mut cursors = json!({});
    for _ in 0..2 {
        let mut agents: Vec<Value> = ids
            .iter()
            .map(|id| json!({"agent":"claude","agent_session":{"agent":"claude","source":"herdr:claude","kind":"id","value":id}}))
            .collect();
        agents.push(codex.clone());
        // A short shared deadline that the Claude replay alone outlasts.
        let deadline = Instant::now() + Duration::from_millis(150);
        let next = NativeTelemetry::default().enrich_until(&mut agents, &cursors, deadline);
        assert_eq!(
            agents[PANES]["_native_telemetry"]["total_input"],
            21_700_000
        );
        cursors = next;
        for row in cursors.as_object_mut().unwrap().values_mut() {
            row.as_object_mut().unwrap().remove("claude");
        }
    }
}

/// The window and its percentage are absent from the published object (D4).
fn windowless(telemetry: &Value) {
    let object = telemetry.as_object().expect("published telemetry");
    for key in ["window", "context_percent"] {
        assert!(!object.contains_key(key), "{key} in {telemetry}");
    }
}

#[test]
fn claude_published_telemetry_never_carries_window_or_context_percent() {
    let fixture = Fixture::new();
    // A caught-up sample with known usage, then a warm pass after an append.
    fixture.write(&session());
    let mut follower = NativeTelemetry::default();
    let (telemetry, _, cursors) = enrich(&mut follower, &json!({}));
    assert_eq!(telemetry["context"], 1201);
    windowless(&telemetry);
    fixture.append(&format!(
        "{}\n",
        assistant(30, "msg-4", "\"end_turn\"", [2, 2, 1300, 0])
    ));
    let (telemetry, _, cursors) = enrich(&mut follower, &cursors);
    assert_eq!(telemetry["context"], 1302);
    windowless(&telemetry);
    // A retained sample re-emitted for an incomplete replay.
    fixture.append(&assistant(31, "msg-5", "\"end_turn\"", [7, 7, 7, 7])[..40]);
    let (telemetry, _, cursors) = enrich(&mut follower, &cursors);
    assert_eq!(cursors[key()]["caught_up"], false);
    assert_eq!(telemetry["context"], 1302);
    windowless(&telemetry);
    // The all-null sample of a replaced file.
    let bytes = std::fs::read(fixture.path("entry-a", ID)).unwrap();
    std::fs::remove_file(fixture.path("entry-a", ID)).unwrap();
    std::fs::write(fixture.path("entry-a", ID), &bytes).unwrap();
    let (telemetry, _, _) = enrich(&mut follower, &cursors);
    unknown(&telemetry, micros(30));
    windowless(&telemetry);
    // The all-null sample of a pane with no counted group.
    fixture.write(&[prompt(10), system("stop_hook_summary", 12)]);
    let (telemetry, _, _) = enrich(&mut NativeTelemetry::default(), &json!({}));
    windowless(&telemetry);
}

/// The published turn timing of a fresh pass over `lines`, without its
/// observation time.
fn timing_of(fixture: &Fixture, lines: &[String]) -> Value {
    fixture.write(lines);
    let (_, mut timing, _) = enrich(&mut NativeTelemetry::default(), &json!({}));
    timing.as_object_mut().unwrap().remove("observed_at_s");
    timing
}

#[test]
fn claude_turn_timing_publishes_no_unproven_current_turn_or_total() {
    let fixture = Fixture::new();
    let first = vec![
        prompt(10),
        assistant(11, "msg-1", "\"end_turn\"", [1, 1, 0, 0]),
        system("turn_duration", 12),
        prompt(20),
    ];
    let known = |active: Value, started: Value, total: u64| {
        json!({"active": active, "started_at_s": started, "complete": true,
            "last_duration_s": 2, "last_outcome": "completed",
            "total_finished_duration_s": total})
    };
    // A start pending its first assistant record may be a running turn.
    assert_eq!(
        timing_of(&fixture, &first),
        known(Value::Null, Value::Null, 2)
    );
    let mut confirmed = first.clone();
    confirmed.push(assistant(21, "msg-2", "\"tool_use\"", [1, 1, 0, 0]));
    assert_eq!(
        timing_of(&fixture, &confirmed),
        known(json!(true), json!(BASE + 20), 2)
    );
    // Local-command output clears the pending start, which may still have
    // been a turn (`lost_idle`).
    let mut local = first.clone();
    local.push(record(
        "user",
        21,
        "\"message\":{\"role\":\"user\",\"content\":\"<local-command-stdout>ok</local-command-stdout>\"}",
    ));
    assert_eq!(
        timing_of(&fixture, &local),
        known(Value::Null, Value::Null, 2)
    );
    // A silent end, with or without a stop hook, may have ended the turn
    // without `turn_duration`: neither the running turn nor a total that
    // leaves it out is published across the idle gap.
    let mut silent = confirmed.clone();
    silent.push(assistant(22, "msg-3", "\"end_turn\"", [1, 1, 0, 0]));
    let snapshot = "{\"type\":\"file-history-snapshot\",\"messageId\":\"m\",\"snapshot\":{}}";
    silent.push(snapshot.to_owned());
    let unknown = json!({"active": null, "started_at_s": null, "complete": false,
        "last_duration_s": 2, "last_outcome": "completed",
        "total_finished_duration_s": null});
    assert_eq!(timing_of(&fixture, &silent), unknown);
    let mut hook = silent.clone();
    hook.insert(6, system("stop_hook_summary", 23));
    assert_eq!(timing_of(&fixture, &hook), unknown);
    // A later `turn_duration` proves the end and restores both.
    hook.push(system("turn_duration", 25));
    assert_eq!(
        timing_of(&fixture, &hook),
        json!({"active": false, "started_at_s": null, "complete": true,
            "last_duration_s": 5, "last_outcome": "completed",
            "total_finished_duration_s": 7})
    );
}

/// A file that binds but fails verification when replay opens it (a header
/// naming another session, or a first line over 64 KiB) keeps its incoming
/// row and publishes the all-null sample at that row's `coverage_seq`, so a
/// peer reports a row for every bound pane and the local's retained copy is
/// replaced (D3). Another pane on the same host is unaffected.
#[test]
fn claude_header_failure_with_a_cursor_row_keeps_it_and_publishes_an_all_null_sample() {
    const OTHER: &str = "fixture-session-b";
    let fixture = Fixture::new();
    fixture.write(&session());
    let other = fixture.path("entry-b", OTHER);
    let text = body(&session()).replace(ID, OTHER);
    std::fs::write(&other, &text).unwrap();
    let panes = || {
        let mut b = agent();
        b["agent_session"]["value"] = json!(OTHER);
        vec![agent(), b]
    };
    let other_key = sha256(format!("anton-native-session-v1:claude:{OTHER}").as_bytes());
    let mut agents = panes();
    let cursors = NativeTelemetry::default().enrich(&mut agents, &json!({}));
    assert_eq!(cursors.as_object().unwrap().len(), 2);
    let expected = agents[0]["_native_telemetry"].clone();
    assert_eq!(expected["total_input"], 3461);
    let (_, rest) = text.split_once('\n').unwrap();
    for header in [
        "{\"type\":\"permission-mode\",\"sessionId\":\"fixture-other\"}\n".to_owned(),
        format!(
            "{{\"type\":\"permission-mode\",\"sessionId\":\"{OTHER}\",\"pad\":\"{}\"}}\n",
            "x".repeat(70 * 1024)
        ),
    ] {
        std::fs::write(&other, format!("{header}{rest}")).unwrap();
        // A fresh follower, as on a peer, with the rows the local sent back.
        let mut agents = panes();
        let kept = NativeTelemetry::default().enrich(&mut agents, &cursors);
        unknown(&agents[1]["_native_telemetry"], micros(23));
        assert!(agents[1]["_native_turn_timing"].is_null());
        assert_eq!(kept[&other_key], cursors[&other_key]);
        assert_eq!(kept.as_object().unwrap().len(), 2);
        assert_eq!(agents[0]["_native_telemetry"], expected);
    }
}

/// RFC 3339 text for a Unix time in microseconds (civil-from-days).
fn rfc3339(micros: u64) -> String {
    let (seconds, fraction) = (micros / 1_000_000, micros % 1_000_000);
    let z = (seconds / 86_400) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    let rest = seconds % 86_400;
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{fraction:06}Z",
        rest / 3600,
        rest / 60 % 60,
        rest % 60
    )
}
/// A prompt whose only stamp is half a second after now: the replay horizon
/// accepts it, but no sample can be stamped with it until that time passes.
fn ahead() -> (String, u64) {
    let at = (now() * 1e6) as u64 + 500_000;
    (prompt(10).replace(&stamp(10), &rfc3339(at)), at)
}

/// A caught-up restart with no usable source time still replaces the copy
/// the local retains for a peer: the all-null sample is stamped with the
/// incoming row's time (D3). Each probe uses a fresh follower, as a peer does.
#[test]
fn claude_caught_up_restart_without_a_usable_time_publishes_an_all_null_sample() {
    assert_eq!(timestamp_us(&rfc3339(micros(23))), Some(micros(23)));
    let fixture = Fixture::new();
    let probe = |cursors: &Value| enrich(&mut NativeTelemetry::default(), cursors);
    let header = body(&[]).len() as u64;
    for case in ["new inode", "truncated", "ahead"] {
        // Retried only if the probe began after the record's time.
        let mut exercised = false;
        for _ in 0..10 {
            fixture.write(&session());
            let (first, _, cursors) = probe(&json!({}));
            assert_eq!(first["total_output"], 26, "{case}");
            let mut at = 0;
            match case {
                "new inode" => drop(fixture.write(&[])),
                "truncated" => std::fs::OpenOptions::new()
                    .write(true)
                    .open(fixture.path("entry-a", ID))
                    .unwrap()
                    .set_len(header)
                    .unwrap(),
                _ => {
                    let (line, stamp) = ahead();
                    fixture.write(&[line]);
                    at = stamp;
                }
            }
            let (telemetry, _, kept) = probe(&cursors);
            if at != 0 && (now() * 1e6) as u64 >= at {
                continue;
            }
            unknown(&telemetry, micros(23));
            let row = &kept[key()];
            assert_eq!(row["caught_up"], true, "{case}");
            assert_eq!(row["claude"]["coverage_seq"], json!(at), "{case}");
            exercised = true;
            break;
        }
        assert!(exercised, "{case}");
    }
}
/// An incoming row whose `coverage_seq` is not usable yet stamps the all-null
/// sample with its usable `usage_seq`. With no usable time at all, the row is
/// withheld, so the local cannot re-emit the replaced file's copy.
#[test]
fn claude_restart_with_no_usable_incoming_time_withholds_the_row() {
    let fixture = Fixture::new();
    let probe = |cursors: &Value| enrich(&mut NativeTelemetry::default(), cursors);
    let counted = assistant(13, "msg-1", "\"end_turn\"", [1, 1, 1, 1]);
    for counted in [Some(counted), None] {
        let mut exercised = false;
        for _ in 0..10 {
            let (line, at) = ahead();
            fixture.write(&counted.iter().cloned().chain([line]).collect::<Vec<_>>());
            let (first, _, cursors) = probe(&json!({}));
            fixture.write(&[]);
            let (telemetry, _, kept) = probe(&cursors);
            if (now() * 1e6) as u64 >= at {
                continue;
            }
            assert_eq!(cursors[key()]["claude"]["coverage_seq"], json!(at));
            if counted.is_some() {
                assert_eq!(first["seq"], json!(micros(13)));
                unknown(&telemetry, micros(13));
                assert_eq!(kept[key()]["caught_up"], true);
            } else {
                assert!(first.is_null() && telemetry.is_null());
                assert!(kept.get(key()).is_none());
            }
            exercised = true;
            break;
        }
        assert!(exercised);
    }
}
