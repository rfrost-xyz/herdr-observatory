//! Native plugin-owned coordinator and bounded command entry points. No listener,
//! interpreter, web feed, independent service or background observer installation.
use anton_runtime::{
    Result, allowances, collection, common, config, fleet, hooks_install, identity,
    model::{Agent, AllowanceRow, Telemetry},
    native, navigation, packaging, reporter, telemetry,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};
/// Snapshot heartbeat: an unchanged state is re-emitted at least this often. It
/// is published as `heartbeat_seconds`, from which QML derives its receipt
/// timeout (heartbeat plus one `LOOP_WAIT` plus pipe margin).
const HEARTBEAT: Duration = Duration::from_secs(4);
/// Longest coordinator wait for an event before it re-checks the heartbeat,
/// allowance recompute and pending refresh, so emissions can lag by this much.
const LOOP_WAIT: Duration = Duration::from_secs(1);
/// Minimum spacing between honoured owner refreshes. Requests inside the window
/// coalesce into one pending refresh served when it ends. It matches the local
/// sampling cadence, so refresh at most doubles local Herdr reads.
const REFRESH_SPACING: Duration = Duration::from_secs(2);
/// Upper bound on waiting for nudged local samples before a forced emission
/// goes out anyway. It exceeds the six-second local Herdr RPC deadline.
const REFRESH_DEADLINE: Duration = Duration::from_secs(8);
/// Longest owner command line in bytes, excluding the newline.
const OWNER_LINE_LIMIT: usize = 64;
static SIGNAL_STOP: AtomicBool = AtomicBool::new(false);
extern "C" fn stop_signal(_: libc::c_int) {
    SIGNAL_STOP.store(true, Ordering::Relaxed);
}
#[derive(Default)]
struct Cancellation {
    flag: AtomicBool,
    nudged: AtomicBool,
    lock: Mutex<()>,
    changed: Condvar,
    notifier: Mutex<Option<mpsc::SyncSender<Event>>>,
}
impl Cancellation {
    fn stopped(&self) -> bool {
        self.flag.load(Ordering::Relaxed) || SIGNAL_STOP.load(Ordering::Relaxed)
    }
    fn stop(&self) {
        self.flag.store(true, Ordering::Relaxed);
        self.changed.notify_all();
        if let Some(sender) = self.notifier.lock().unwrap().as_ref() {
            let _ = sender.try_send(Event::Stop);
        }
    }
    /// Wakes a cadence `wait`. A nudge that arrives while the worker is busy
    /// stays pending, so its next `wait` returns at once. The lock is held while
    /// notifying so the wake cannot fall between predicate check and park.
    fn nudge(&self) {
        let _guard = self.lock.lock().unwrap();
        self.nudged.store(true, Ordering::Relaxed);
        self.changed.notify_all();
    }
    /// Cadence wait: returns on stop, timeout or a pending nudge, which it clears.
    fn wait(&self, delay: Duration) {
        let guard = self.lock.lock().unwrap();
        let _ = self.changed.wait_timeout_while(guard, delay, |_| {
            !self.stopped() && !self.nudged.load(Ordering::Relaxed)
        });
        self.nudged.store(false, Ordering::Relaxed);
    }
    /// Backoff pause that honours only stop and never consumes a nudge.
    fn pause(&self, delay: Duration) {
        let guard = self.lock.lock().unwrap();
        let _ = self
            .changed
            .wait_timeout_while(guard, delay, |_| !self.stopped());
    }
}
/// Commands the owner (QML) may write on stdin, one per line.
#[derive(Debug, PartialEq)]
enum OwnerCommand {
    Refresh,
}
/// Bounded framing for the owner pipe. The buffer never exceeds
/// `OWNER_LINE_LIMIT`: an oversized line is discarded up to its newline.
#[derive(Default)]
struct OwnerLines {
    line: Vec<u8>,
    discarding: bool,
}
impl OwnerLines {
    fn push(&mut self, bytes: &[u8], mut command: impl FnMut(OwnerCommand)) {
        for &byte in bytes {
            if byte == b'\n' {
                if !self.discarding {
                    let mut line = self.line.as_slice();
                    if let [rest @ .., b'\r'] = line {
                        line = rest;
                    }
                    if std::str::from_utf8(line) == Ok("refresh") {
                        command(OwnerCommand::Refresh);
                    }
                }
                self.line.clear();
                self.discarding = false;
            } else if !self.discarding {
                if self.line.len() == OWNER_LINE_LIMIT {
                    self.line.clear();
                    self.discarding = true;
                } else {
                    self.line.push(byte);
                }
            }
        }
    }
}
#[derive(Clone, PartialEq, Serialize)]
struct HostState {
    id: String,
    label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    navigation: Option<Value>,
    online: bool,
    connection_state: String,
    error: Option<String>,
    sampled_at: Option<f64>,
    agents: Vec<Agent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    protocol: Option<Option<u64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<String>,
}
#[derive(Deserialize)]
struct Sample {
    // Peers before this change still send `theme`; unknown fields are ignored.
    agents: Vec<Agent>,
    sampled_at: f64,
    error: Option<String>,
    protocol: Option<u64>,
    version: String,
    cursors: Value,
    /// The validated Claude cursor rows the request for this sample carried,
    /// set by the host worker; unknown counts as none.
    #[serde(skip)]
    requested: Option<usize>,
}
enum Event {
    Stop,
    Refresh,
    Host(String, u64, Result<Sample>),
    RemoteAllowances(String, u64, Vec<Value>),
    Discovery(Result<Vec<Value>>),
}
struct State {
    config: Value,
    hosts: Vec<HostState>,
    allowances: Vec<AllowanceRow>,
    revision: u64,
    discovery: String,
    /// D3: the last caught-up Claude sample of each peer agent, by host and
    /// agent id, with its `session_generation`. Filled only from live peer
    /// samples in this owner run and never checkpointed.
    retained: BTreeMap<String, BTreeMap<String, (u64, Telemetry)>>,
}
impl State {
    fn new(config: Value) -> Self {
        let hosts = config["hosts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|host| {
                let id = host["id"].as_str().unwrap().to_owned();
                HostState {
                    navigation: navigation::host_binding(host).ok(),
                    label: collection::clean(host.get("label").unwrap_or(&host["id"]), &id),
                    id,
                    online: false,
                    connection_state: "connecting".into(),
                    error: Some("Awaiting first sample".into()),
                    sampled_at: None,
                    agents: vec![],
                    protocol: None,
                    version: None,
                }
            })
            .collect();
        Self {
            config,
            hosts,
            allowances: vec![],
            revision: 0,
            discovery: "disabled".into(),
            retained: BTreeMap::new(),
        }
    }
    fn snapshot(&self) -> Value {
        json!({"at":common::now(),"interval":self.config.get("interval").cloned().unwrap_or(json!(5)),"heartbeat_seconds":HEARTBEAT.as_secs(),"hosts":self.hosts,"allowances":self.allowances,"fleet_discovery":{"state":self.discovery}})
    }
    fn sample(&mut self, id: &str, result: Result<Sample>) -> Option<Value> {
        let index = self.hosts.iter().position(|v| v.id == id)?;
        let mut before = self.hosts[index].clone();
        let state = &mut self.hosts[index];
        let mut cursors = None;
        match result {
            Ok(mut sample) => {
                if !sample.sampled_at.is_finite()
                    || sample.sampled_at <= 0.0
                    || sample.sampled_at > common::now() + 1.0
                    || sample.agents.len() > 4096
                    || sample
                        .agents
                        .iter()
                        .any(|agent| agent.host != id || !agent.id.starts_with(&format!("{id}:")))
                {
                    return self.sample(id, Err("Invalid peer sample".into()));
                }
                let host = &self.config["hosts"][index];
                let local = host
                    .get("transport")
                    .and_then(Value::as_str)
                    .unwrap_or("local")
                    == "local";
                let interval = if local {
                    2.0
                } else {
                    self.config
                        .get("interval")
                        .and_then(Value::as_f64)
                        .unwrap_or(5.0)
                };
                let mut rejected = BTreeSet::new();
                for agent in &mut sample.agents {
                    agent.navigation = state.navigation.clone();
                    if !local && revalidate(agent, sample.sampled_at) {
                        rejected.insert(agent.id.clone());
                    }
                    let previous = if state.online {
                        state.agents.iter().find(|old| {
                            old.id == agent.id
                                && old.status == agent.status
                                && old.technical.session_generation
                                    == agent.technical.session_generation
                        })
                    } else {
                        None
                    };
                    agent.since = previous.map(|old| old.since).unwrap_or_else(common::now);
                    // Turn-timing measurement freshness: three sampling
                    // intervals, never under 12 s. This is measurement
                    // freshness, independent of the transport heartbeat.
                    if let Some(timing) = &mut agent.technical.turn_timing {
                        timing.freshness_seconds = (interval * 3.0).max(12.0);
                    }
                }
                state.online = sample.error.is_none();
                state.connection_state = if state.online {
                    "connected"
                } else {
                    "unreachable"
                }
                .into();
                state.error = sample
                    .error
                    .map(|_| "Herdr unavailable or incompatible".into());
                state.sampled_at = Some(sample.sampled_at);
                let rows = native::validate_cursors(&sample.cursors);
                if local || !state.online {
                    self.retained.remove(id);
                } else {
                    let claude = rows.values().filter(|v| v.is_claude()).count();
                    let requested = sample.requested.unwrap_or(0);
                    let retained = self.retained.entry(id.to_owned()).or_default();
                    retain_claude(retained, &mut sample.agents, claude, requested, &rejected);
                }
                state.agents = if state.online { sample.agents } else { vec![] };
                state.protocol = Some(sample.protocol);
                state.version = Some(sample.version);
                cursors = Some(serde_json::to_value(rows).unwrap_or_else(|_| json!({})));
            }
            Err(error) => {
                state.online = false;
                let setup = error == "setup_needed";
                state.connection_state = if setup { "setup_needed" } else { "unreachable" }.into();
                state.error = Some(
                    if setup {
                        "Setup needed"
                    } else {
                        "Collector unreachable or invalid response"
                    }
                    .into(),
                );
                state.agents.clear();
                self.retained.remove(id);
            }
        }
        before.sampled_at = state.sampled_at;
        if before != *state {
            self.revision += 1;
        }
        cursors
    }
}
/// Peer agents come from another binary, so their telemetry and turn timing
/// pass the local views again; an invalid value becomes unknown (D9). Peer
/// times may lead the later of the local clock and `sampled_at` by the 1 s
/// transport skew `sampled_at` allows plus the snapshot timeout, which bounds
/// how long after `sampled_at` the peer stamps its values; original
/// timestamps are kept. Returns whether telemetry was present and rejected.
fn revalidate(agent: &mut Agent, sampled_at: f64) -> bool {
    fn view<T: Serialize + serde::de::DeserializeOwned>(
        value: Option<T>,
        check: impl Fn(&Value) -> Option<Value>,
    ) -> Option<T> {
        let value = serde_json::to_value(value?).ok()?;
        serde_json::from_value(check(&value)?).ok()
    }
    let time = common::now().max(sampled_at) + 1.0 + collection::SNAPSHOT_TIMEOUT.as_secs_f64();
    let technical = &mut agent.technical;
    let present = technical.telemetry.is_some();
    technical.telemetry = view(technical.telemetry.take(), |raw| {
        telemetry::telemetry_view_at(raw, time)
    });
    technical.turn_timing = view(technical.turn_timing.take(), |raw| {
        telemetry::turn_timing_view_at(raw, time)
    });
    present && technical.telemetry.is_none()
}
/// D3 local retention of one peer's Claude samples. `rows` counts the
/// validated rows with a Claude block in the response and `requested` those in
/// the request; rows exist only for current panes, so a shortfall means a row
/// may have been evicted. A caught-up sample with known usage replaces the
/// retained numeric subset only when the response has a row for every bound
/// pane; any other sample drops it. A sample without telemetry re-emits it for
/// the same `session_generation` only when the request and the response both
/// had a row for every bound pane: without the request row the peer replayed
/// from the header and could not detect a replaced file. A sample that
/// revalidation `rejected` was sent, so it drops the retained copy.
fn retain_claude(
    retained: &mut BTreeMap<String, (u64, Telemetry)>,
    agents: &mut [Agent],
    rows: usize,
    requested: usize,
    rejected: &BTreeSet<String>,
) {
    let bound =
        |agent: &Agent| agent.harness == "claude" && agent.technical.session_generation.is_some();
    let panes = agents.iter().filter(|v| bound(v)).count();
    let mut next = BTreeMap::new();
    for agent in agents.iter_mut().filter(|v| bound(v)) {
        let generation = agent.technical.session_generation.unwrap();
        match &agent.technical.telemetry {
            Some(sample) => {
                if sample.usage_source.as_deref() == Some("claude-transcript")
                    && sample.usage_seq.is_some()
                    && rows >= panes
                {
                    let mut subset = sample.clone();
                    subset.compactions = None;
                    for field in [
                        &mut subset.subagent_total,
                        &mut subset.subagent_done,
                        &mut subset.subagent_status_seq,
                        &mut subset.subagent_running,
                        &mut subset.subagent_completed,
                        &mut subset.subagent_interrupted,
                        &mut subset.subagent_failed,
                        &mut subset.subagent_unknown,
                    ] {
                        *field = None;
                    }
                    next.insert(agent.id.clone(), (generation, subset));
                }
            }
            None if rows.min(requested) >= panes && !rejected.contains(&agent.id) => {
                if let Some((generation, subset)) = retained
                    .remove(&agent.id)
                    .filter(|(old, _)| *old == generation)
                {
                    agent.technical.telemetry = Some(subset.clone());
                    next.insert(agent.id.clone(), (generation, subset));
                }
            }
            None => {}
        }
    }
    *retained = next;
}
fn send(sender: &mpsc::SyncSender<Event>, mut event: Event, cancellation: &Cancellation) {
    loop {
        match sender.try_send(event) {
            Ok(()) | Err(mpsc::TrySendError::Disconnected(_)) => return,
            Err(mpsc::TrySendError::Full(value)) => event = value,
        }
        if cancellation.stopped() {
            return;
        }
        cancellation.pause(Duration::from_millis(20));
    }
}
fn emit(value: &Value, cancel: &Cancellation) -> Result<()> {
    let mut bytes = serde_json::to_vec(value).map_err(|_| "Invalid snapshot")?;
    if bytes.len() > 2 * 1024 * 1024 {
        bytes = b"null".to_vec();
    }
    bytes.push(b'\n');
    let mut offset = 0;
    while offset < bytes.len() && !cancel.stopped() {
        let count = unsafe {
            libc::write(
                libc::STDOUT_FILENO,
                bytes[offset..].as_ptr().cast(),
                bytes.len() - offset,
            )
        };
        if count > 0 {
            offset += count as usize;
            continue;
        }
        let error = io::Error::last_os_error();
        if error.kind() == io::ErrorKind::Interrupted {
            continue;
        }
        if error.kind() != io::ErrorKind::WouldBlock {
            return Err("Owner output closed".into());
        }
        let mut fd = libc::pollfd {
            fd: libc::STDOUT_FILENO,
            events: libc::POLLOUT,
            revents: 0,
        };
        unsafe {
            libc::poll(&mut fd, 1, 50);
        }
    }
    if offset == bytes.len() {
        Ok(())
    } else {
        Err("Owner closed".into())
    }
}
fn load(root: &Path) -> Result<Value> {
    let bytes = common::read_owned(&root.join(".config.json"), 4 * 1024 * 1024, true)?;
    config::validate(serde_json::from_slice(&bytes).map_err(|_| "Invalid configuration JSON")?)
}
fn current_config(root: &Path) -> Result<Value> {
    let config = load(root)?;
    if fleet::enabled(&config) {
        let profiles = fleet::discover(&config, Some(&SIGNAL_STOP))?;
        fleet::effective(&config, Some(&profiles))
    } else {
        Ok(config)
    }
}
fn prepare(root: &Path, state: &Path) -> Result<()> {
    let _owner = common::owner_guard(&root.join(".herdr-observatory-install"))?;
    common::ensure_private_directory(state)
}
fn meaningful(cursors: &BTreeMap<String, Value>) -> Value {
    let mut result = json!({});
    for (host, rows) in cursors {
        if rows.as_object().is_some_and(|v| !v.is_empty()) {
            let mut rows = rows.clone();
            for row in rows.as_object_mut().unwrap().values_mut() {
                if let Some(row) = row.as_object_mut() {
                    row.remove("at");
                }
            }
            result[host] = rows;
        }
    }
    result
}
struct Worker {
    host: Value,
    generation: u64,
    cancel: Arc<Cancellation>,
    join: thread::JoinHandle<()>,
}
impl Worker {
    fn stop(self) {
        self.cancel.stop();
        let _ = self.join.join();
    }
}
fn host_worker(
    host: Value,
    generation: u64,
    config: &Value,
    cursors: Arc<Mutex<BTreeMap<String, Value>>>,
    sender: mpsc::SyncSender<Event>,
) -> Worker {
    let cancel = Arc::new(Cancellation::default());
    let stop = cancel.clone();
    let spec = host.clone();
    let interval = if host["transport"] == "ssh" {
        config
            .get("interval")
            .and_then(Value::as_f64)
            .unwrap_or(5.0)
    } else {
        2.0
    };
    let join = thread::spawn(move || {
        let mut follower = native::NativeTelemetry::default();
        let id = spec["id"].as_str().unwrap().to_owned();
        while !stop.stopped() {
            let previous = cursors
                .lock()
                .unwrap()
                .get(&id)
                .cloned()
                .unwrap_or(json!({}));
            let requested = native::validate_cursors(&previous)
                .values()
                .filter(|v| v.is_claude())
                .count();
            let result = if spec["transport"] == "ssh" {
                collection::remote(&spec, &previous, Some(&stop.flag))
            } else {
                collection::local(&spec, &previous, &mut follower, Some(&stop.flag))
            }
            .and_then(|v| serde_json::from_value(v).map_err(|_| "Invalid native sample".into()))
            .map(|mut sample: Sample| {
                sample.requested = Some(requested);
                sample
            });
            send(&sender, Event::Host(id.clone(), generation, result), &stop);
            stop.wait(Duration::from_secs_f64(interval));
        }
    });
    Worker {
        host,
        generation,
        cancel,
        join,
    }
}
fn allowance_worker(
    source: Value,
    generation: u64,
    config: &Value,
    sender: mpsc::SyncSender<Event>,
) -> Worker {
    let cancel = Arc::new(Cancellation::default());
    let stop = cancel.clone();
    let host = source.clone();
    let mut config = config.clone();
    config["allowances"]["sources"] = json!([source]);
    let target = host["target"].as_str().unwrap().to_owned();
    let join = thread::spawn(move || {
        while !stop.stopped() {
            send(
                &sender,
                Event::RemoteAllowances(
                    target.clone(),
                    generation,
                    allowances::remote(&config, Some(&stop.flag)),
                ),
                &stop,
            );
            stop.wait(Duration::from_secs(60));
        }
    });
    Worker {
        host,
        generation,
        cancel,
        join,
    }
}
fn source_key(_config: &Value, target: &str) -> String {
    common::sha256(target.as_bytes())
}
fn checkpoint_values(config: &Value, cursors: &BTreeMap<String, Value>) -> BTreeMap<String, Value> {
    config["hosts"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|host| {
            (
                fleet::checkpoint_key(host),
                cursors
                    .get(host["id"].as_str().unwrap())
                    .cloned()
                    .unwrap_or(json!({})),
            )
        })
        .collect()
}
#[allow(clippy::too_many_arguments)]
fn reconcile(
    config: Value,
    state: &mut State,
    hosts: &mut BTreeMap<String, Worker>,
    accounts: &mut BTreeMap<String, Worker>,
    rows: &mut BTreeMap<String, Vec<Value>>,
    cursors: &Arc<Mutex<BTreeMap<String, Value>>>,
    checkpoints: &native::Checkpoints,
    generation: &mut u64,
    sender: &mpsc::SyncSender<Event>,
) {
    let wanted: BTreeMap<_, _> = config["hosts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|host| (host["id"].as_str().unwrap().to_owned(), host.clone()))
        .collect();
    let retired: Vec<_> = hosts
        .iter()
        .filter(|(id, worker)| {
            wanted
                .get(*id)
                .is_none_or(|h| fleet::route_key(h) != fleet::route_key(&worker.host))
        })
        .map(|(id, _)| id.clone())
        .collect();
    // Signal all retirees before joining any one of them.
    for id in &retired {
        hosts[id].cancel.stop();
    }
    for id in &retired {
        hosts.remove(id).unwrap().stop();
        cursors.lock().unwrap().remove(id);
        state.retained.remove(id);
    }
    let mut next = State::new(config.clone());
    for host in &mut next.hosts {
        if !retired.contains(&host.id) {
            if let Some(old) = state.hosts.iter().find(|v| v.id == host.id) {
                let label = host.label.clone();
                *host = old.clone();
                host.label = label;
            }
        }
    }
    state.hosts = next.hosts;
    state.config = config.clone();
    state.revision += 1;
    for (id, host) in wanted {
        if let Some(worker) = hosts.get_mut(&id) {
            worker.host = host;
            continue;
        }
        *generation += 1;
        let restored = if retired.contains(&id) {
            json!({})
        } else {
            checkpoints.for_host(&fleet::checkpoint_key(&host))
        };
        cursors.lock().unwrap().insert(id.clone(), restored);
        hosts.insert(
            id,
            host_worker(host, *generation, &config, cursors.clone(), sender.clone()),
        );
    }
    let wanted: BTreeMap<_, _> = config["allowances"]["sources"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| v["target"].as_str().map(|t| (t.to_owned(), v.clone())))
        .collect();
    let retired: Vec<_> = accounts
        .iter()
        .filter(|(target, worker)| {
            !wanted.contains_key(*target) || worker.host["route_key"] != source_key(&config, target)
        })
        .map(|(target, _)| target.clone())
        .collect();
    for target in &retired {
        accounts[target].cancel.stop();
    }
    for target in retired {
        accounts.remove(&target).unwrap().stop();
        rows.remove(&target);
    }
    for (target, mut source) in wanted {
        if accounts.contains_key(&target) {
            continue;
        }
        *generation += 1;
        let worker = allowance_worker(source.clone(), *generation, &config, sender.clone());
        source["route_key"] = json!(source_key(&config, &target));
        accounts.insert(
            target,
            Worker {
                host: source,
                ..worker
            },
        );
    }
}
fn accept_host(
    state: &mut State,
    hosts: &BTreeMap<String, Worker>,
    cursors: &Arc<Mutex<BTreeMap<String, Value>>>,
    id: &str,
    epoch: u64,
    sample: Result<Sample>,
) {
    if hosts.get(id).is_some_and(|w| w.generation == epoch) {
        if let Some(value) = state.sample(id, sample) {
            cursors.lock().unwrap().insert(id.to_owned(), value);
        }
    }
}
fn accept_allowances(
    accounts: &BTreeMap<String, Worker>,
    remote: &mut BTreeMap<String, Vec<Value>>,
    target: String,
    epoch: u64,
    rows: Vec<Value>,
) -> bool {
    if accounts.get(&target).is_some_and(|w| w.generation == epoch) {
        remote.insert(target, rows);
        true
    } else {
        false
    }
}
fn stream(root: PathBuf, state_path: PathBuf) -> Result<()> {
    prepare(&root, &state_path)?;
    let base = load(&root)?;
    let initial_config = fleet::effective(&base, None)?;
    let owner = root.join(".herdr-observatory-install");
    let mut checkpoints = native::Checkpoints::new(&state_path, &owner)?;
    let cursors = Arc::new(Mutex::new(BTreeMap::new()));
    let cancel = Arc::new(Cancellation::default());
    let (tx, rx) = mpsc::sync_channel(64);
    *cancel.notifier.lock().unwrap() = Some(tx.clone());
    // Owner pipe: newline-delimited commands. EOF or a hard read error means
    // the owner has gone and stops the runtime. `queued` keeps at most one
    // refresh event in the channel, however fast the owner writes.
    let stdin_cancel = cancel.clone();
    let stdin_sender = tx.clone();
    let queued = Arc::new(AtomicBool::new(false));
    let stdin_queued = queued.clone();
    thread::spawn(move || {
        let mut input = io::stdin().lock();
        let mut bytes = [0; 1024];
        let mut lines = OwnerLines::default();
        loop {
            match input.read(&mut bytes) {
                Ok(0) => break,
                Ok(count) => lines.push(&bytes[..count], |OwnerCommand::Refresh| {
                    if !stdin_queued.swap(true, Ordering::Relaxed) {
                        send(&stdin_sender, Event::Refresh, &stdin_cancel);
                    }
                }),
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(_) => break,
            }
        }
        stdin_cancel.stop();
    });
    let mut state = State::new(initial_config.clone());
    state.discovery = if fleet::enabled(&base) {
        "discovering"
    } else {
        "disabled"
    }
    .into();
    let mut hosts = BTreeMap::new();
    let mut accounts = BTreeMap::new();
    let mut remote_rows = BTreeMap::new();
    let mut generation = 0;
    reconcile(
        initial_config,
        &mut state,
        &mut hosts,
        &mut accounts,
        &mut remote_rows,
        &cursors,
        &checkpoints,
        &mut generation,
        &tx,
    );
    let mut inventory_known = !fleet::enabled(&base);
    if inventory_known {
        let _ = checkpoints.reconcile(&checkpoint_values(&state.config, &cursors.lock().unwrap()));
    }
    let mut workers = Vec::new();
    if fleet::enabled(&base) {
        let config = base.clone();
        let stop = cancel.clone();
        let sender = tx.clone();
        workers.push(thread::spawn(move || {
            while !stop.stopped() {
                let began = Instant::now();
                send(
                    &sender,
                    Event::Discovery(fleet::discover(&config, Some(&stop.flag))),
                    &stop,
                );
                stop.wait(Duration::from_secs(10).saturating_sub(began.elapsed()));
            }
        }));
    }
    if allowances::configuration(&base).is_some() {
        let config = base.clone();
        let path = state_path.clone();
        let owner = owner.clone();
        let stop = cancel.clone();
        workers.push(thread::spawn(move || {
            while !stop.stopped() {
                let _ = allowances::refresh(&config, &path, Some(&owner), Some(&stop.flag));
                stop.wait(Duration::from_secs(60));
            }
        }));
    }
    let mut last_allowances = Instant::now().checked_sub(Duration::from_secs(2)).unwrap();
    let mut last_checkpoint = Instant::now();
    let mut last_emitted = Instant::now();
    let mut emitted = None;
    // Owner refresh: `refresh_pending` holds one coalesced request until the
    // spacing window allows it. `awaiting` names nudged local hosts whose
    // post-refresh sample has not yet been accepted.
    let mut refresh_pending = false;
    let mut last_refresh: Option<Instant> = None;
    let mut refresh_at = 0.0;
    let mut awaiting = BTreeSet::new();
    let mut force_emit = false;
    unsafe {
        let flags = libc::fcntl(libc::STDOUT_FILENO, libc::F_GETFL);
        if flags >= 0 {
            libc::fcntl(libc::STDOUT_FILENO, libc::F_SETFL, flags | libc::O_NONBLOCK);
        }
    }
    while !cancel.stopped() {
        if refresh_pending && last_refresh.is_none_or(|at| at.elapsed() >= REFRESH_SPACING) {
            refresh_pending = false;
            last_refresh = Some(Instant::now());
            refresh_at = common::now();
            awaiting.clear();
            for (id, worker) in &hosts {
                if worker.host["transport"] != "ssh" {
                    worker.cancel.nudge();
                    awaiting.insert(id.clone());
                }
            }
            // Recompute from the local cache only; no Codex RPC is made here.
            last_allowances = Instant::now().checked_sub(Duration::from_secs(2)).unwrap();
            force_emit = true;
        }
        if !awaiting.is_empty() && last_refresh.is_some_and(|at| at.elapsed() >= REFRESH_DEADLINE) {
            awaiting.clear();
        }
        if last_allowances.elapsed() >= Duration::from_secs(2) {
            let remote: Vec<_> = remote_rows.values().flatten().cloned().collect();
            let rows = allowances::snapshot(&state.config, &state_path, &remote);
            if rows != state.allowances {
                state.allowances = rows;
                state.revision += 1;
            }
            last_allowances = Instant::now();
        }
        if emitted != Some(state.revision)
            || last_emitted.elapsed() >= HEARTBEAT
            || (force_emit && awaiting.is_empty())
        {
            if emit(&state.snapshot(), &cancel).is_err() {
                cancel.stop();
                break;
            }
            emitted = Some(state.revision);
            last_emitted = Instant::now();
            if awaiting.is_empty() {
                force_emit = false;
            }
        }
        if inventory_known && last_checkpoint.elapsed() >= Duration::from_secs(10) {
            let values = checkpoint_values(&state.config, &cursors.lock().unwrap());
            if !meaningful(&values).as_object().unwrap().is_empty() {
                let _ = checkpoints.update(&values, false);
            }
            last_checkpoint = Instant::now();
        }
        // Wake in time to serve a pending refresh when its window ends.
        let wait = match (refresh_pending, last_refresh) {
            (true, Some(at)) => LOOP_WAIT.min(REFRESH_SPACING.saturating_sub(at.elapsed())),
            _ => LOOP_WAIT,
        };
        match rx.recv_timeout(wait) {
            Ok(Event::Stop) => break,
            Ok(Event::Refresh) => {
                queued.store(false, Ordering::Relaxed);
                refresh_pending = true;
            }
            Ok(Event::Host(id, epoch, sample)) => {
                accept_host(&mut state, &hosts, &cursors, &id, epoch, sample);
                // A sample stamped before the refresh was in flight already and
                // cannot satisfy it; the nudge makes the worker sample again.
                if state
                    .hosts
                    .iter()
                    .find(|host| host.id == id)
                    .is_none_or(|host| host.sampled_at.is_some_and(|at| at >= refresh_at))
                {
                    awaiting.remove(&id);
                }
            }
            Ok(Event::RemoteAllowances(target, epoch, rows)) => {
                if accept_allowances(&accounts, &mut remote_rows, target, epoch, rows) {
                    last_allowances = Instant::now().checked_sub(Duration::from_secs(2)).unwrap();
                }
            }
            Ok(Event::Discovery(result)) => {
                let result = result.and_then(|profiles| fleet::effective(&base, Some(&profiles)));
                let status = if result.is_ok() {
                    "available"
                } else {
                    "unavailable"
                };
                if state.discovery != status {
                    state.discovery = status.into();
                    state.revision += 1;
                }
                if let Ok(config) = result {
                    if !inventory_known || config != state.config {
                        reconcile(
                            config,
                            &mut state,
                            &mut hosts,
                            &mut accounts,
                            &mut remote_rows,
                            &cursors,
                            &checkpoints,
                            &mut generation,
                            &tx,
                        );
                        awaiting.retain(|id| hosts.contains_key(id));
                        let values = checkpoint_values(&state.config, &cursors.lock().unwrap());
                        // Force route retirement even when all live cursor maps are empty.
                        let _ = checkpoints.reconcile(&values);
                        last_allowances =
                            Instant::now().checked_sub(Duration::from_secs(2)).unwrap();
                    }
                    inventory_known = true;
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    cancel.stop();
    for worker in hosts.values().chain(accounts.values()) {
        worker.cancel.stop();
    }
    for (_, worker) in hosts.into_iter().chain(accounts) {
        worker.stop();
    }
    for worker in workers {
        let _ = worker.join();
    }
    if inventory_known {
        let values = checkpoint_values(&state.config, &cursors.lock().unwrap());
        if !meaningful(&values).as_object().unwrap().is_empty() {
            let _ = checkpoints.update(&values, true);
        }
    }
    Ok(())
}

fn input(limit: usize, timeout: Duration) -> Result<Value> {
    let deadline = Instant::now() + timeout;
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 16384];
    unsafe {
        let flags = libc::fcntl(libc::STDIN_FILENO, libc::F_GETFL);
        libc::fcntl(libc::STDIN_FILENO, libc::F_SETFL, flags | libc::O_NONBLOCK);
    }
    loop {
        if Instant::now() >= deadline || SIGNAL_STOP.load(Ordering::Relaxed) {
            return Err("Input deadline".into());
        }
        let count =
            unsafe { libc::read(libc::STDIN_FILENO, chunk.as_mut_ptr().cast(), chunk.len()) };
        if count == 0 {
            break;
        }
        if count > 0 {
            bytes.extend_from_slice(&chunk[..count as usize]);
            if bytes.len() > limit {
                return Err("Input exceeds limit".into());
            }
            continue;
        }
        let error = io::Error::last_os_error();
        if !matches!(
            error.kind(),
            io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
        ) {
            return Err("Input unavailable".into());
        }
        let mut fd = libc::pollfd {
            fd: libc::STDIN_FILENO,
            events: libc::POLLIN,
            revents: 0,
        };
        unsafe {
            libc::poll(&mut fd, 1, 20);
        }
    }
    serde_json::from_slice(&bytes).map_err(|_| "Invalid input JSON".into())
}
fn output(value: &Value) -> Result<()> {
    let bytes = serde_json::to_string(value).map_err(|_| "Invalid output JSON")?;
    if bytes.len() > 4 * 1024 * 1024 {
        return Err("Output exceeds limit".into());
    }
    println!("{bytes}");
    Ok(())
}
fn cli() -> Result<()> {
    let mut root = std::env::current_exe()
        .map_err(|_| "Executable unavailable")?
        .parent()
        .ok_or("Executable directory unavailable")?
        .to_owned();
    let mut state = None;
    let mut args = std::env::args().skip(1);
    let mut commands = Vec::new();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--root" => root = args.next().ok_or("Missing root")?.into(),
            "--state" => state = Some(PathBuf::from(args.next().ok_or("Missing state")?)),
            _ => commands.push(arg),
        }
    }
    if !root.is_absolute() {
        return Err("Absolute plugin root required".into());
    }
    let state = state.unwrap_or_else(|| {
        std::env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| common::expand_home("~/.local/state"))
            .join(
                if root
                    .file_name()
                    .is_some_and(|v| v == "herdr.observatory-peer")
                {
                    "herdr.observatory-peer"
                } else {
                    "herdr.observatory"
                },
            )
    });
    let home = common::expand_home("~");
    let owner = root.join(".herdr-observatory-install");
    let mode = commands.first().map(String::as_str).unwrap_or("");
    match mode {
        "" => stream(root, state),
        "--version" => {
            println!("anton-runtime {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        "--migrate-config" => output(&packaging::migrate(input(
            4 * 1024 * 1024,
            Duration::from_secs(5),
        )?)?),
        "--record-peer" => packaging::record_peer(&root),
        "--remove-peers" => packaging::remove_peers(&root),
        "--uninstall-peer" => packaging::remove_peer(&root, &state, &home),
        "--install-hooks" => hooks_install::install(
            &root,
            &home,
            commands.iter().any(|v| v == "--adopt-legacy-hooks"),
        ),
        "--uninstall-hooks" => hooks_install::uninstall(&root, &home),
        "--repair-retired-hooks" => hooks_install::repair_retired(&root, &home),
        "--retire-checkpoints" => native::retire(&state, &owner),
        "--report" => {
            if commands.len() != 4 {
                return Err("Invalid reporter arguments".into());
            }
            let raw = input(1024 * 1024, Duration::from_millis(1500))?;
            reporter::report(
                &root,
                &state,
                &commands[1],
                &commands[2],
                commands[3].parse().map_err(|_| "Invalid report sequence")?,
                raw,
            )?;
            Ok(())
        }
        "--open-thread" => {
            if !(3..=4).contains(&commands.len()) {
                return Err("Invalid navigation arguments".into());
            }
            let observed = commands
                .get(3)
                .map(|v| {
                    if v.len() > 4096 {
                        Err("Invalid navigation binding".into())
                    } else {
                        serde_json::from_str::<Value>(v)
                            .map_err(|_| String::from("Invalid navigation binding"))
                    }
                })
                .transpose()?;
            navigation::open_observed(&root, &commands[1], &commands[2], observed.as_ref())
        }
        "--focus" => {
            let _owner = common::owner_guard(&owner)?;
            navigation::peer_focus(&input(65536, Duration::from_secs(3))?)
        }
        "--probe" => {
            let _owner = common::owner_guard(&owner)?;
            let cfg = load(&root)?;
            let request = input(4 * 1024 * 1024, Duration::from_secs(3))?;
            if request["version"] != 1 {
                return Err("Invalid probe protocol".into());
            }
            let host = cfg["hosts"]
                .as_array()
                .unwrap()
                .iter()
                .find(|host| {
                    host["id"] == request["host_id"]
                        && host
                            .get("transport")
                            .and_then(Value::as_str)
                            .unwrap_or("local")
                            == "local"
                })
                .ok_or("Peer host is not configured locally")?;
            let mut host = host.clone();
            if let Some(session) = request.get("session") {
                let session = session
                    .as_str()
                    .filter(|v| fleet::session(v))
                    .ok_or("Invalid peer session")?;
                host["session"] = json!(session);
                host.as_object_mut().unwrap().remove("socket_path");
            }
            let result = collection::local(
                &host,
                &request["cursors"],
                &mut native::NativeTelemetry::default(),
                Some(&SIGNAL_STOP),
            )?;
            output(
                &json!({"version":1,"host_id":host["id"],"session":host.get("session").cloned().unwrap_or(json!("default")),"ok":true,"result":result}),
            )
        }
        "--allowances-probe" => {
            let _owner = common::owner_guard(&owner)?;
            let config = load(&root)?;
            output(&json!(
                allowances::probe(Some(&SIGNAL_STOP))
                    .ok()
                    .filter(|row| row["account_key"]
                        .as_str()
                        .is_some_and(|key| config["allowances"]["accounts"].get(key).is_some()))
                    .into_iter()
                    .collect::<Vec<_>>()
            ))
        }
        "--identity-probe" => {
            let _owner = common::owner_guard(&owner)?;
            let config = load(&root)?;
            let value = identity::probe(Some(&SIGNAL_STOP))?;
            if value["account_key"]
                .as_str()
                .is_none_or(|key| config["allowances"]["accounts"].get(key).is_none())
            {
                return Err("Account identity is not configured on this peer".into());
            }
            output(&value)
        }
        "--refresh-identities" => {
            let _owner = common::owner_guard(&owner)?;
            let count = identity::refresh(
                &current_config(&root)?,
                &root.join(".accounts.json"),
                Some(&SIGNAL_STOP),
            )?;
            println!("Saved {count} verified account labels locally.");
            Ok(())
        }
        "--refresh-allowances" => {
            prepare(&root, &state)?;
            allowances::refresh(
                &current_config(&root)?,
                &state,
                Some(&owner),
                Some(&SIGNAL_STOP),
            )?;
            Ok(())
        }
        _ => Err("Unknown Anton command".into()),
    }
}
fn main() {
    unsafe {
        libc::umask(0o077);
        libc::signal(
            libc::SIGTERM,
            stop_signal as *const () as libc::sighandler_t,
        );
        libc::signal(libc::SIGINT, stop_signal as *const () as libc::sighandler_t);
        libc::signal(libc::SIGPIPE, libc::SIG_IGN);
    }
    if let Err(error) = cli() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn state() -> State {
        State::new(json!({"hosts":[{"id":"test"}]}))
    }
    fn sample(status: &str, at: f64) -> Sample {
        serde_json::from_value(json!({"agents":[{"id":"test:pane","host":"test","status":status,"technical":{}}],"theme":null,"sampled_at":at,"error":null,"protocol":1,"version":"test","cursors":{}})).unwrap()
    }
    fn frame(chunks: &[&[u8]]) -> usize {
        let mut lines = OwnerLines::default();
        let mut count = 0;
        for chunk in chunks {
            lines.push(chunk, |command| {
                assert_eq!(command, OwnerCommand::Refresh);
                count += 1;
            });
            assert!(lines.line.len() <= OWNER_LINE_LIMIT);
        }
        count
    }
    #[test]
    fn owner_lines_frame_split_reads_crlf_and_known_commands_only() {
        assert_eq!(frame(&[b"refresh\n"]), 1);
        assert_eq!(frame(&[b"ref", b"re", b"sh", b"\n"]), 1);
        assert_eq!(frame(&[b"refresh\r\n", b"refresh\r", b"\n"]), 2);
        assert_eq!(
            frame(&[b"refresh"]),
            0,
            "an unterminated line is not a command"
        );
        assert_eq!(frame(&[b"\n\n", b"Refresh\n", b"refresh \n", b"stop\n"]), 0);
        assert_eq!(frame(&[b"\r\rrefresh\n", b"refresh\r\r\n"]), 0);
        assert_eq!(frame(&[b"junk\nrefresh\nmore junk\n"]), 1);
    }
    #[test]
    fn owner_lines_discard_oversized_and_invalid_utf8_without_growing() {
        let long = vec![b'x'; 10 * 1024];
        assert_eq!(frame(&[&long, b"refresh\n", b"refresh\n"]), 1);
        assert_eq!(frame(&[&long[..OWNER_LINE_LIMIT + 1], b"\nrefresh\n"]), 1);
        let mut padded = vec![b' '; OWNER_LINE_LIMIT - 7];
        padded.extend_from_slice(b"refresh\n");
        assert_eq!(frame(&[&padded]), 0, "a 64-byte unknown line is ignored");
        assert_eq!(frame(&[b"\xffrefresh\n", b"refr\xc3\n", b"refresh\n"]), 1);
        assert_eq!(frame(&[b"refresh\xff\n"]), 0);
    }
    #[test]
    fn nudge_wakes_wait_once_and_survives_busy_worker_but_not_pause() {
        let cancel = Arc::new(Cancellation::default());
        cancel.nudge();
        cancel.pause(Duration::from_millis(30));
        let began = Instant::now();
        cancel.wait(Duration::from_secs(5));
        assert!(
            began.elapsed() < Duration::from_secs(1),
            "pending nudge must wake"
        );
        let began = Instant::now();
        cancel.wait(Duration::from_millis(100));
        assert!(
            began.elapsed() >= Duration::from_millis(100),
            "nudge is consumed once"
        );
        let waiter = cancel.clone();
        let join = thread::spawn(move || {
            let began = Instant::now();
            waiter.wait(Duration::from_secs(5));
            began.elapsed()
        });
        thread::sleep(Duration::from_millis(50));
        cancel.nudge();
        assert!(join.join().unwrap() < Duration::from_secs(1));
    }
    #[test]
    fn old_peer_theme_is_accepted_and_new_snapshot_has_contract_keys() {
        let mut value = serde_json::to_value(json!({"agents":[],"theme":{"name":"Legacy","colours":{"accent":"#123456"}},"sampled_at":common::now(),"error":null,"protocol":1,"version":"old","cursors":{}})).unwrap();
        let sample: Sample = serde_json::from_value(value.clone()).unwrap();
        let mut state = state();
        state.sample("test", Ok(sample));
        assert_eq!(state.hosts[0].connection_state, "connected");
        value.as_object_mut().unwrap().remove("theme");
        assert!(serde_json::from_value::<Sample>(value).is_ok());
        let snapshot = state.snapshot();
        let keys: Vec<_> = snapshot.as_object().unwrap().keys().cloned().collect();
        assert_eq!(
            keys,
            [
                "allowances",
                "at",
                "fleet_discovery",
                "heartbeat_seconds",
                "hosts",
                "interval"
            ]
        );
        assert_eq!(snapshot["heartbeat_seconds"], 4);
        // Typed rows are assigned directly and serialise with every D1 key.
        state.allowances = allowances::snapshot(
            &json!({"accounts":{"a".repeat(64):"Personal"}}),
            Path::new("/nonexistent/anton-allowances"),
            &[],
        );
        let row = &state.snapshot()["allowances"][0];
        let allowance: Vec<_> = row.as_object().unwrap().keys().cloned().collect();
        assert_eq!(
            allowance,
            [
                "account_id",
                "label",
                "plan",
                "provider",
                "provider_label",
                "reset_count",
                "reset_expires_at",
                "sampled_at",
                "status",
                "status_text",
                "windows"
            ]
        );
        assert_eq!(row["status"], "unavailable");
        assert_eq!(row["windows"], json!([]));
        let host: Vec<_> = snapshot["hosts"][0]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        assert_eq!(
            host,
            [
                "agents",
                "connection_state",
                "error",
                "id",
                "label",
                "navigation",
                "online",
                "protocol",
                "sampled_at",
                "version"
            ]
        );
    }
    #[test]
    fn unchanged_sample_preserves_since_and_revision() {
        let mut state = state();
        state.sample("test", Ok(sample("working", common::now())));
        let since = state.hosts[0].agents[0].since;
        let revision = state.revision;
        state.sample("test", Ok(sample("working", common::now())));
        assert_eq!(state.hosts[0].agents[0].since, since);
        assert_eq!(state.revision, revision);
    }
    /// A caught-up peer Claude sample: totals, last response and children.
    fn caught_up(seq: u64) -> Value {
        json!({"seq":seq,"event":"session","phase":"ready","model":"claude-fixture-1","input":1201,"output_tokens":1,"cache_read":1200,"cache_write":0,"context":1201,"usage_seq":seq - 1,"total_input":3461,"total_output":26,"total_cache_read":3300,"total_cache_write":50,"total_uncached_input":111,"compactions":1,"subagent_total":2,"subagent_done":1,"subagent_status_seq":seq - 2,"subagent_running":0,"subagent_completed":1,"subagent_interrupted":0,"subagent_failed":0,"subagent_unknown":1,"usage_source":"claude-transcript"})
    }
    fn claude(generation: u64, telemetry: Value) -> Agent {
        serde_json::from_value(json!({"id":"test:pane","host":"test","harness":"claude","status":"working","technical":{"session_generation":generation,"telemetry":telemetry}})).unwrap()
    }
    fn peer() -> State {
        State::new(json!({"hosts":[{"id":"test","transport":"ssh","target":"fixture"}]}))
    }
    #[test]
    fn peer_telemetry_and_turn_timing_are_revalidated() {
        let mut state = peer();
        let now = common::now();
        let mut value = sample("working", now);
        value.agents[0] = claude(7, caught_up((now as u64 + 60) * 1_000_000));
        value.agents[0].technical.turn_timing = serde_json::from_value(json!({"active":true,"started_at_s":null,"observed_at_s":now,"complete":false,"last_duration_s":null,"total_finished_duration_s":null,"last_outcome":null,"freshness_seconds":12.0})).unwrap();
        state.sample("test", Ok(value));
        let technical = &state.hosts[0].agents[0].technical;
        assert!(technical.telemetry.is_none() && technical.turn_timing.is_none());
        // Valid values survive, and freshness is still set by the owner.
        let mut value = sample("working", now);
        value.agents[0] = claude(7, caught_up((now as u64 - 60) * 1_000_000));
        value.agents[0].technical.turn_timing = serde_json::from_value(json!({"active":false,"started_at_s":null,"observed_at_s":now,"complete":true,"last_duration_s":2,"total_finished_duration_s":7,"last_outcome":"completed","freshness_seconds":12.0})).unwrap();
        state.sample("test", Ok(value));
        let technical = &state.hosts[0].agents[0].technical;
        assert_eq!(
            technical.telemetry.as_ref().unwrap().total_input,
            Some(3461)
        );
        let timing = technical.turn_timing.as_ref().unwrap();
        assert_eq!(
            (timing.total_finished_duration_s, timing.freshness_seconds),
            (Some(7), 15.0)
        );
    }
    /// Peer revalidation allows the 1 s transport skew `sampled_at` allows,
    /// plus the peer's own gap between stamping `sampled_at` and stamping its
    /// values, which the Herdr snapshot timeout bounds, for every harness. It
    /// keeps the original timestamps.
    #[test]
    fn peer_revalidation_tolerates_clock_skew_and_the_stamping_gap() {
        let now = common::now();
        let gap = collection::SNAPSHOT_TIMEOUT.as_secs_f64();
        // (`sampled_at` lead, value lead, kept). A sample just inside the
        // `sampled_at` gate keeps values stamped just over 1 s ahead.
        for (at, skew, kept) in [
            (0.0, 0.5, true),
            (0.95, 1.05, true),
            (0.95, 1.94 + gap, true),
            (0.0, 1.5 + gap, false),
            (0.95, 2.0 + gap, false),
        ] {
            let seq = ((now + skew) * 1e6) as u64;
            let mut state = peer();
            let mut value = sample("working", now + at);
            value.agents[0] = claude(7, caught_up(seq));
            value.agents[0].harness = "codex".into();
            let technical = &mut value.agents[0].technical;
            technical.telemetry.as_mut().unwrap().usage_source = Some("codex-rollout".into());
            technical.turn_timing = serde_json::from_value(json!({"active":false,"started_at_s":null,"observed_at_s":now + skew,"complete":true,"last_duration_s":2,"total_finished_duration_s":7,"last_outcome":"completed","freshness_seconds":12.0})).unwrap();
            state.sample("test", Ok(value));
            let technical = &state.hosts[0].agents[0].technical;
            let telemetry = technical.telemetry.as_ref();
            assert_eq!(
                telemetry.map(|v| (v.seq, v.usage_seq, v.total_input)),
                kept.then_some((seq, Some(seq - 1), Some(3461))),
                "{at} {skew}"
            );
            assert_eq!(
                technical.turn_timing.as_ref().map(|v| v.observed_at_s),
                kept.then_some(now + skew),
                "{at} {skew}"
            );
        }
        // A Claude all-null sample within the skew replaces the retained one.
        let mut state = peer();
        let with = |telemetry: Value| {
            let mut value = sample("working", now);
            value.agents[0] = claude(7, telemetry);
            value.cursors = claude_row();
            value.requested = Some(1);
            value
        };
        state.sample("test", Ok(with(caught_up((now as u64 - 60) * 1_000_000))));
        assert_eq!(state.retained["test"].len(), 1);
        let seq = ((now + 0.5) * 1e6) as u64;
        state.sample(
            "test",
            Ok(with(json!({"seq":seq,"event":"session","phase":"ready"}))),
        );
        let telemetry = state.hosts[0].agents[0].technical.telemetry.as_ref();
        assert_eq!(telemetry.map(|v| (v.seq, v.total_input)), Some((seq, None)));
        assert!(state.retained["test"].is_empty());
    }
    /// A peer sample that revalidation rejects is not an absent one: it drops
    /// the retained copy, which is not re-emitted then or on a later pass.
    #[test]
    fn claude_sample_rejected_by_revalidation_drops_the_retained_copy() {
        let now = common::now();
        let with = |telemetry: Value| {
            let mut value = sample("working", now);
            value.agents[0] = claude(7, telemetry);
            value.cursors = claude_row();
            value.requested = Some(1);
            value
        };
        let skewed = ((now + 60.0) * 1e6) as u64;
        for rejected in [
            json!({"seq":skewed,"event":"session","phase":"ready"}),
            caught_up(skewed),
        ] {
            let mut state = peer();
            state.sample("test", Ok(with(caught_up((now as u64 - 60) * 1_000_000))));
            assert_eq!(state.retained["test"].len(), 1);
            state.sample("test", Ok(with(rejected.clone())));
            let shown = || state.hosts[0].agents[0].technical.telemetry.clone();
            assert!(shown().is_none(), "{rejected}");
            assert!(state.retained["test"].is_empty(), "{rejected}");
            // A later incomplete pass has nothing to re-emit.
            state.sample("test", Ok(with(Value::Null)));
            assert!(state.hosts[0].agents[0].technical.telemetry.is_none());
        }
    }
    #[test]
    fn claude_retention_replaces_reemits_and_drops() {
        let seq = 1_767_225_623_250_000;
        let telemetry = |agents: &[Agent]| agents[0].technical.telemetry.clone();
        let mut retained = BTreeMap::new();
        let mut agents = vec![claude(7, caught_up(seq))];
        retain_claude(&mut retained, &mut agents, 1, 1, &BTreeSet::new());
        let mut subset = telemetry(&agents).unwrap();
        subset.compactions = None;
        (
            subset.subagent_total,
            subset.subagent_done,
            subset.subagent_status_seq,
        ) = (None, None, None);
        (subset.subagent_running, subset.subagent_completed) = (None, None);
        (
            subset.subagent_interrupted,
            subset.subagent_failed,
            subset.subagent_unknown,
        ) = (None, None, None);
        // An incomplete pass: no telemetry, a cursor row for every pane.
        for _ in 0..2 {
            let mut agents = vec![claude(7, Value::Null)];
            retain_claude(&mut retained, &mut agents, 1, 1, &BTreeSet::new());
            assert_eq!(telemetry(&agents).as_ref(), Some(&subset));
        }
        // Fewer Claude rows than bound panes: a row may be evicted, so drop.
        let mut agents = vec![claude(7, Value::Null), claude(8, Value::Null)];
        agents[1].id = "test:other".into();
        retain_claude(&mut retained, &mut agents, 1, 1, &BTreeSet::new());
        assert!(agents.iter().all(|v| v.technical.telemetry.is_none()) && retained.is_empty());
        // A generation change, a missing pane, unknown totals, another source.
        let caught = |retained: &mut BTreeMap<_, _>| {
            let mut agents = vec![claude(7, caught_up(seq))];
            retain_claude(retained, &mut agents, 1, 1, &BTreeSet::new());
            assert_eq!(retained.len(), 1);
        };
        caught(&mut retained);
        let mut agents = vec![claude(8, Value::Null)];
        retain_claude(&mut retained, &mut agents, 1, 1, &BTreeSet::new());
        assert!(telemetry(&agents).is_none() && retained.is_empty());
        caught(&mut retained);
        retain_claude(&mut retained, &mut [], 0, 0, &BTreeSet::new());
        assert!(retained.is_empty());
        for (field, value) in [
            ("usage_seq", Value::Null),
            ("usage_source", json!("pi-extension")),
        ] {
            caught(&mut retained);
            let mut sample = caught_up(seq);
            sample[field] = value;
            let mut agents = vec![claude(7, sample)];
            retain_claude(&mut retained, &mut agents, 1, 1, &BTreeSet::new());
            assert!(retained.is_empty(), "{field}");
            let mut agents = vec![claude(7, Value::Null)];
            retain_claude(&mut retained, &mut agents, 1, 1, &BTreeSet::new());
            assert!(telemetry(&agents).is_none(), "{field}");
        }
        // A harness that is not Claude is never retained.
        let mut agents = vec![claude(7, caught_up(seq))];
        agents[0].harness = "codex".into();
        retain_claude(&mut retained, &mut agents, 1, 1, &BTreeSet::new());
        assert!(retained.is_empty());
    }
    #[test]
    fn claude_retention_needs_a_row_for_every_pane_in_response_and_request() {
        let seq = 1_767_225_623_250_000;
        let mut retained = BTreeMap::new();
        // A caught-up sample is shown, but a response short of rows is not
        // stored: its row may have been evicted.
        let mut agents = vec![claude(7, caught_up(seq))];
        retain_claude(&mut retained, &mut agents, 0, 1, &BTreeSet::new());
        assert!(agents[0].technical.telemetry.is_some() && retained.is_empty());
        // A caught-up replay after a request without the row is a measurement.
        retain_claude(
            &mut retained,
            &mut [claude(7, caught_up(seq))],
            1,
            0,
            &BTreeSet::new(),
        );
        assert_eq!(retained.len(), 1);
        // A request without the row made the peer replay from the header, so
        // an incomplete pass is not covered by the retained sample: dropped.
        let mut agents = vec![claude(7, Value::Null)];
        retain_claude(&mut retained, &mut agents, 1, 0, &BTreeSet::new());
        assert!(agents[0].technical.telemetry.is_none() && retained.is_empty());
    }
    /// A synthetic caught-up Claude cursor row, as a peer returns it.
    fn claude_row() -> Value {
        let mut row: Value = serde_json::from_str(r#"{"caught_up":true,"children":{},"claude":{"abort_adjacent":false,"ambiguous":false,"foreign":false,"cache_creation":0,"cache_read":0,"classifier":null,"closed":[],"compaction_iteration":false,"compactions":0,"coverage_seq":1767225612250000,"input":0,"last":null,"last_valid":true,"lost_idle":false,"open":{"id":"3a0de37932e8b197","response":{"model":"claude-fixture-1","usage":[1,1,1,1]},"stop":2,"tainted":false,"usage":[1,1,1,1]},"output":0,"pending_start":null,"queued_since_start":null,"silent_end":false,"totals_valid":true,"usage_seq":1767225611250000},"compaction_markers":0,"compaction_summaries":0,"compactions_valid":true,"file":[1,2],"fingerprint":{"header":"47b6f0a22fd24b24fe54e82f5ba3bb0b310819634f922c08a6c6e72ba5e132c5","mtime_us":1,"size":654,"tail":"ffc6d285d1377c43ed044721bbe51bdbb916ee8891654de6362a784d098d80e5"},"offset":654,"seq":0,"skipping":false,"turns":{"active":null,"current_known":true,"finished":{"4c318c012df919977122e3ca":[1767225610,1767225612,"completed"]},"last":"4c318c012df919977122e3ca","last_duration":2,"last_end":1767225612,"last_outcome":"completed","start":null,"supported":true,"total":2,"valid":true},"valid":true}"#).unwrap();
        row["at"] = json!(common::now());
        json!({ common::sha256(b"claude-row"): row })
    }
    #[test]
    fn peer_retention_is_dropped_without_rows_on_failure_and_for_local_hosts() {
        let seq = (common::now() as u64 - 60) * 1_000_000;
        let with = |telemetry: Value| {
            let mut value = sample("working", common::now());
            value.agents[0] = claude(7, telemetry);
            value.cursors = claude_row();
            value
        };
        let rowless = |telemetry: Value| {
            let mut value = with(telemetry);
            value.cursors = json!({});
            value
        };
        let mut state = peer();
        state.sample("test", Ok(with(caught_up(seq))));
        assert_eq!(state.retained["test"].len(), 1);
        // No cursor row came back: drop instead of re-emitting.
        state.sample("test", Ok(rowless(Value::Null)));
        assert!(state.hosts[0].agents[0].technical.telemetry.is_none());
        assert!(state.retained["test"].is_empty());
        state.sample("test", Ok(with(caught_up(seq))));
        state.sample("test", Err("unavailable".into()));
        assert!(!state.retained.contains_key("test"));
        state.sample("test", Ok(with(caught_up(seq))));
        let mut offline = with(Value::Null);
        offline.error = Some("down".into());
        state.sample("test", Ok(offline));
        assert!(!state.retained.contains_key("test"));
        // Local hosts keep their own follower retention.
        let mut local = self::state();
        local.sample("test", Ok(with(caught_up(seq))));
        assert!(local.retained.is_empty());
    }
    #[test]
    fn failed_collection_removes_old_agents() {
        let mut state = state();
        state.sample("test", Ok(sample("working", common::now())));
        state.sample("test", Err("unavailable".into()));
        assert!(!state.hosts[0].online);
        assert!(state.hosts[0].agents.is_empty());
    }
    #[test]
    fn mismatched_peer_host_cannot_enter_state() {
        let mut state = state();
        let mut value = sample("working", common::now());
        value.agents[0].host = "other".into();
        state.sample("test", Ok(value));
        assert!(!state.hosts[0].online);
        assert!(state.hosts[0].agents.is_empty());
    }
    #[test]
    fn reconciliation_preserves_rename_and_rejects_retired_host_and_account_events() {
        let root = std::env::temp_dir().join(format!(
            "anton-reconcile-{}-{}-{}",
            std::process::id(),
            common::now().to_bits(),
            {
                static SEQUENCE: std::sync::atomic::AtomicUsize =
                    std::sync::atomic::AtomicUsize::new(0);
                SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            }
        ));
        std::fs::create_dir(&root).unwrap();
        let owner = root.join("owner");
        std::fs::write(&owner, b"herdr.observatory\n").unwrap();
        let cache = native::Checkpoints::new(&root, &owner).unwrap();
        let config = json!({"hosts":[{"id":"test","herdr":"/bin/false","session":"default"}],"allowances":{"accounts":{"a".repeat(64):"Personal"},"sources":[{"target":"fixture"}]}});
        let mut state = State::new(config.clone());
        let mut hosts = BTreeMap::new();
        let mut accounts = BTreeMap::new();
        let mut rows = BTreeMap::new();
        let cursors = Arc::new(Mutex::new(BTreeMap::new()));
        let (tx, _rx) = mpsc::sync_channel(64);
        let mut epoch = 1;
        // A fixture-owned existing account worker lets reconciliation exercise
        // retention and retirement without invoking SSH in a unit test.
        let stop = Arc::new(Cancellation::default());
        let stopped = stop.clone();
        accounts.insert(
            "fixture".into(),
            Worker {
                host: json!({"target":"fixture","route_key":source_key(&config,"fixture")}),
                generation: 1,
                cancel: stop,
                join: thread::spawn(move || {
                    while !stopped.stopped() {
                        stopped.wait(Duration::from_secs(1));
                    }
                }),
            },
        );
        reconcile(
            config.clone(),
            &mut state,
            &mut hosts,
            &mut accounts,
            &mut rows,
            &cursors,
            &cache,
            &mut epoch,
            &tx,
        );
        let generation = hosts["test"].generation;
        accept_host(
            &mut state,
            &hosts,
            &cursors,
            "test",
            generation,
            Ok(sample("working", common::now())),
        );
        let since = state.hosts[0].agents[0].since;
        let mut renamed = config.clone();
        renamed["hosts"][0]["label"] = json!("Renamed");
        reconcile(
            renamed.clone(),
            &mut state,
            &mut hosts,
            &mut accounts,
            &mut rows,
            &cursors,
            &cache,
            &mut epoch,
            &tx,
        );
        assert_eq!(hosts["test"].generation, generation);
        assert_eq!(accounts["fixture"].generation, 1);
        assert_eq!(state.hosts[0].agents[0].since, since);
        reconcile(
            renamed.clone(),
            &mut state,
            &mut hosts,
            &mut accounts,
            &mut rows,
            &cursors,
            &cache,
            &mut epoch,
            &tx,
        );
        assert_eq!(hosts["test"].generation, generation);
        assert_eq!(accounts["fixture"].generation, 1);
        let mut rerouted = renamed;
        rerouted["hosts"][0]["session"] = json!("another");
        rerouted["allowances"]["sources"] = json!([]);
        reconcile(
            rerouted,
            &mut state,
            &mut hosts,
            &mut accounts,
            &mut rows,
            &cursors,
            &cache,
            &mut epoch,
            &tx,
        );
        assert_ne!(hosts["test"].generation, generation);
        assert!(accounts.is_empty());
        assert!(state.hosts[0].agents.is_empty());
        accept_host(
            &mut state,
            &hosts,
            &cursors,
            "test",
            generation,
            Ok(sample("done", common::now())),
        );
        assert!(state.hosts[0].agents.is_empty());
        assert!(!accept_allowances(
            &accounts,
            &mut rows,
            "fixture".into(),
            1,
            vec![json!({"poison":true})]
        ));
        assert!(rows.is_empty());
        let generation = hosts["test"].generation;
        accept_host(
            &mut state,
            &hosts,
            &cursors,
            "test",
            generation,
            Ok(sample("idle", common::now())),
        );
        assert_eq!(state.hosts[0].agents.len(), 1);
        for (_, worker) in hosts {
            worker.stop();
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}
