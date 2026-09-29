//! Native plugin-owned coordinator and bounded command entry points. No listener,
//! interpreter, web feed, independent service or background observer installation.
use anton_runtime::{
    Result, allowances, collection, common, config, fleet, hooks_install, identity,
    model::{Agent, AllowanceRow},
    native, navigation, packaging, reporter,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};
static SIGNAL_STOP: AtomicBool = AtomicBool::new(false);
extern "C" fn stop_signal(_: libc::c_int) {
    SIGNAL_STOP.store(true, Ordering::Relaxed);
}
#[derive(Default)]
struct Cancellation {
    flag: AtomicBool,
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
    fn wait(&self, delay: Duration) {
        let guard = self.lock.lock().unwrap();
        let _ = self
            .changed
            .wait_timeout_while(guard, delay, |_| !self.stopped());
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
    metrics: Option<Value>,
    trend: Vec<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    protocol: Option<Option<u64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<String>,
}
#[derive(Deserialize)]
struct Sample {
    agents: Vec<Agent>,
    theme: Value,
    sampled_at: f64,
    error: Option<String>,
    protocol: Option<u64>,
    version: String,
    cursors: Value,
}
enum Event {
    Stop,
    Host(String, u64, Result<Sample>),
    RemoteAllowances(String, u64, Vec<Value>),
    Discovery(Result<Vec<Value>>),
}
struct State {
    config: Value,
    hosts: Vec<HostState>,
    theme: Value,
    allowances: Vec<AllowanceRow>,
    revision: u64,
    discovery: String,
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
                    navigation: host["profile_id"].as_str().and_then(|id| {
                        navigation::profile_binding(
                            &json!({"id":id,"target":host["target"],"session":host["session"]}),
                        )
                        .ok()
                    }),
                    label: collection::clean(host.get("label").unwrap_or(&host["id"]), &id),
                    id,
                    online: false,
                    connection_state: "connecting".into(),
                    error: Some("Awaiting first sample".into()),
                    sampled_at: None,
                    agents: vec![],
                    metrics: None,
                    trend: vec![],
                    protocol: None,
                    version: None,
                }
            })
            .collect();
        Self {
            config,
            hosts,
            theme: collection::fallback_theme(),
            allowances: vec![],
            revision: 0,
            discovery: "disabled".into(),
        }
    }
    fn snapshot(&self) -> Value {
        let local = self.config["hosts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| {
                v.get("transport")
                    .and_then(Value::as_str)
                    .unwrap_or("local")
                    == "local"
            })
            .and_then(|v| v["id"].as_str())
            .unwrap_or("");
        json!({"profile":"personal","at":common::now(),"interval":self.config.get("interval").cloned().unwrap_or(json!(5)),"hosts":self.hosts,"theme":self.theme,"display":{"host":local,"role":"Host"},"allowances":self.allowances,"fleet_discovery":{"state":self.discovery}})
    }
    fn sample(&mut self, id: &str, result: Result<Sample>) -> Option<Value> {
        let index = self.hosts.iter().position(|v| v.id == id)?;
        let mut before = self.hosts[index].clone();
        let old_theme = self.theme.clone();
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
                let interval = if host
                    .get("transport")
                    .and_then(Value::as_str)
                    .unwrap_or("local")
                    == "local"
                {
                    2.0
                } else {
                    self.config
                        .get("interval")
                        .and_then(Value::as_f64)
                        .unwrap_or(5.0)
                };
                for agent in &mut sample.agents {
                    agent.navigation = state.navigation.clone();
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
                state.agents = if state.online { sample.agents } else { vec![] };
                state.protocol = Some(sample.protocol);
                state.version = Some(sample.version);
                if id
                    == self
                        .config
                        .get("theme_host")
                        .and_then(Value::as_str)
                        .unwrap_or(self.config["hosts"][0]["id"].as_str().unwrap_or(""))
                {
                    self.theme = sample.theme;
                }
                cursors = Some(
                    serde_json::to_value(native::validate_cursors(&sample.cursors))
                        .unwrap_or_else(|_| json!({})),
                );
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
            }
        }
        before.sampled_at = state.sampled_at;
        if before != *state || old_theme != self.theme {
            self.revision += 1;
        }
        cursors
    }
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
        cancellation.wait(Duration::from_millis(20));
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
            let result = if spec["transport"] == "ssh" {
                collection::remote(&spec, &previous, Some(&stop.flag))
            } else {
                collection::local(&spec, &previous, &mut follower, Some(&stop.flag))
            }
            .and_then(|v| serde_json::from_value(v).map_err(|_| "Invalid native sample".into()));
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
    let stdin_cancel = cancel.clone();
    thread::spawn(move || {
        let mut input = io::stdin().lock();
        let mut bytes = [0; 1024];
        while matches!(input.read(&mut bytes), Ok(1..)) {}
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
    unsafe {
        let flags = libc::fcntl(libc::STDOUT_FILENO, libc::F_GETFL);
        if flags >= 0 {
            libc::fcntl(libc::STDOUT_FILENO, libc::F_SETFL, flags | libc::O_NONBLOCK);
        }
    }
    while !cancel.stopped() {
        if last_allowances.elapsed() >= Duration::from_secs(2) {
            let remote: Vec<_> = remote_rows.values().flatten().cloned().collect();
            let rows = allowances::snapshot(&state.config, &state_path, &remote);
            if let Ok(rows) = serde_json::from_value::<Vec<AllowanceRow>>(json!(rows)) {
                if rows != state.allowances {
                    state.allowances = rows;
                    state.revision += 1;
                }
            }
            last_allowances = Instant::now();
        }
        if emitted != Some(state.revision) || last_emitted.elapsed() >= Duration::from_secs(4) {
            if emit(&state.snapshot(), &cancel).is_err() {
                cancel.stop();
                break;
            }
            emitted = Some(state.revision);
            last_emitted = Instant::now();
        }
        if inventory_known && last_checkpoint.elapsed() >= Duration::from_secs(10) {
            let values = checkpoint_values(&state.config, &cursors.lock().unwrap());
            if !meaningful(&values).as_object().unwrap().is_empty() {
                let _ = checkpoints.update(&values, false);
            }
            last_checkpoint = Instant::now();
        }
        match rx.recv_timeout(Duration::from_secs(1)) {
            Ok(Event::Stop) => break,
            Ok(Event::Host(id, epoch, sample)) => {
                accept_host(&mut state, &hosts, &cursors, &id, epoch, sample)
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
            navigation::open_observed(&commands[1], &commands[2], observed.as_ref())
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
        serde_json::from_value(json!({"agents":[{"id":"test:pane","host":"test","status":status,"technical":{}}],"theme":collection::fallback_theme(),"sampled_at":at,"error":null,"protocol":1,"version":"test","cursors":{}})).unwrap()
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
            "anton-reconcile-{}-{}",
            std::process::id(),
            common::now().to_bits()
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
