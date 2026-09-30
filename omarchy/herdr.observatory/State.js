// Pure presentation projection. Observatory owns collection and disclosure.
function number(value) {
  return typeof value === "number" && isFinite(value) ? value : null
}

function label(value, fallback) {
  return typeof value === "string" && value !== "" ? value : fallback
}

function ageSeconds(value, nowMs, futureTolerance) {
  var at = number(value)
  if (at === null || at <= 0) return null
  var age = nowMs / 1000 - at
  return age >= -(futureTolerance || 0) && isFinite(age) ? Math.max(0, age) : null
}

function ageLabel(age) {
  if (age === null) return "age unknown"
  if (age < 60) return Math.floor(age) + "s ago"
  if (age < 3600) return Math.floor(age / 60) + "m ago"
  return Math.floor(age / 3600) + "h ago"
}

function percent(value) {
  var n = number(value)
  return n !== null && n >= 0 && n <= 100 ? n : null
}

function counter(value) {
  var n = number(value)
  return n !== null && n >= 0 && n <= 9007199254740991 && Math.floor(n) === n ? n : null
}

function threadUsage(agent, nowMs) {
  var empty = { contextPercent: null, inputTokens: null, outputTokens: null, uncachedTokens: null, cachePercent: null, compactions: null, age: null, stale: false }
  var t = agent.technical && agent.technical.telemetry
  if (!t || counter(t.seq) === null || t.seq <= 0 || t.seq / 1000 > nowMs) return empty
  var stamp = t.usage_seq === null || t.usage_seq === undefined ? t.seq : t.usage_seq
  if (counter(stamp) === null || stamp <= 0 || stamp > t.seq) return empty
  var age = ageSeconds(stamp / 1000000, nowMs)
  if (age === null) return empty
  var input = counter(t.total_input), output = counter(t.total_output)
  var cached = counter(t.total_cache_read), uncached = counter(t.total_uncached_input), written = counter(t.total_cache_write)
  var validPartition = input !== null && cached !== null && uncached !== null
      && cached <= input && uncached <= input - cached && input - cached - uncached === (written || 0)
  var cache = validPartition && input > 0 ? cached / input * 100 : null
  var context = counter(t.context), window = counter(t.window)
  var contextPercent = context !== null && window !== null && window > 0 && context <= window
      ? percent(t.context_percent) !== null ? t.context_percent : context / window * 100 : null
  // Measurement freshness: usage older than 120 s is last-known, not current.
  return { contextPercent: contextPercent, inputTokens: input, outputTokens: output, uncachedTokens: validPartition ? uncached : null, cachePercent: cache, compactions: counter(t.compactions), age: ageLabel(age), stale: age > 120 }
}

// Native turn boundaries measure wall-clock time, including tool waits. No Herdr
// status duration is used. An old source freezes at its verified observation.
function turnTiming(agent, nowMs, hostReporting) {
  var source = agent.technical && agent.technical.turn_timing
  if (!source || typeof source !== "object") return null
  var observed = number(source.observed_at_s), start = number(source.started_at_s)
  // Fleet clocks may differ by a fraction of a second. Keep the source time
  // intact; only native timing freshness/display accepts up to one second ahead.
  var age = observed !== null && observed > 0 && observed <= nowMs / 1000 + 1
      ? Math.max(0, nowMs / 1000 - observed) : null
  var freshness = number(source.freshness_seconds)
  if (freshness === null) freshness = 12
  if (freshness < 1 || freshness > 180 || age === null) return null
  var stale = !hostReporting || age > freshness
  var last = counter(source.last_duration_s), total = counter(source.total_finished_duration_s)
  var active = source.active === true, elapsed = null
  if (active && start !== null && start > 0 && start <= observed) {
    elapsed = Math.max(0, Math.floor((stale ? observed : nowMs / 1000) - start))
  } else if (source.active === false) {
    elapsed = last
  }
  var complete = source.complete === true && total !== null
  var accumulated = complete ? total + (active && elapsed !== null ? elapsed : 0) : null
  if (accumulated !== null && accumulated > 9007199254740991) accumulated = null
  return { active: active, elapsed: elapsed, last: last, total: accumulated,
           complete: complete, stale: stale, age: ageLabel(age), observedAt: observed,
           outcome: ["completed", "aborted"].indexOf(source.last_outcome) >= 0 ? source.last_outcome : null }
}

function durationLabel(seconds) {
  if (counter(seconds) === null) return "—"
  if (seconds < 60) return seconds + "s"
  if (seconds < 3600) return Math.floor(seconds / 60) + "m " + String(seconds % 60).padStart(2, "0") + "s"
  return Math.floor(seconds / 3600) + "h " + String(Math.floor(seconds % 3600 / 60)).padStart(2, "0") + "m"
}

function timingHint(timing) {
  if (!timing || timing.elapsed === null) return "Turn time unavailable"
  var result = (timing.active ? "Wall-clock turn " : "Last wall-clock turn ") + durationLabel(timing.elapsed)
  if (!timing.active && timing.outcome === "aborted") result += " · interrupted"
  result += "\n" + (timing.total === null ? "Total unavailable" : "Total turn time " + durationLabel(timing.total))
  if (timing.stale) result += "\nLast observed " + timing.age
  return result
}

// Hook observations are events, not a live roster or successful completions.
function childCompletion(agent, nowMs) {
  var t = agent.technical && agent.technical.telemetry
  if (!t || counter(t.seq) === null || t.seq / 1000 > nowMs) return null
  var total = counter(t.subagent_total), done = counter(t.subagent_done), stamp = counter(t.subagent_status_seq)
  if (total === null || done === null || total > 128 || done > total || stamp === null || stamp <= 0 || stamp > t.seq) return null
  var age = ageSeconds(stamp / 1000000, nowMs)
  if (age === null) return null
  var outcomes = {}, sum = done, valid = true
  ;["running", "interrupted", "failed", "unknown"].forEach(function(name) {
    var value = counter(t["subagent_" + name]); outcomes[name] = value
    if (value === null) valid = false; else sum += value
  })
  if (!valid || sum !== total) outcomes = null
  // Measurement freshness: child status older than 120 s is last-known.
  return { total: total, done: done, stamp: stamp, stale: age > 120, age: ageLabel(age), outcomes: outcomes }
}

function childObservations(agent, nowMs) {
  var t = agent.technical && agent.technical.telemetry
  if (!t || counter(t.seq) === null || t.seq <= 0 || t.seq / 1000 > nowMs) return null
  var starts = counter(t.subagent_starts), stops = counter(t.subagent_stops)
  if (starts === null || stops === null || starts > 999 || stops > 999) return null
  var stamp = t.subagent_seq
  if (starts === 0 && stops === 0 && (stamp === null || stamp === undefined))
    return { starts: 0, stops: 0, stamp: null, stale: false, age: null }
  if (counter(stamp) === null || stamp <= 0 || stamp > t.seq) return null
  var age = ageSeconds(stamp / 1000000, nowMs)
  if (age === null) return null
  // Measurement freshness: child observations older than 120 s are last-known.
  return { starts: starts, stops: stops, stamp: stamp, stale: age > 120, age: ageLabel(age) }
}

// Compare stable host/thread identities across sorting, not delegate positions.
function transitions(before, after) {
  var previous = {}, changes = {}
  for (var i = 0; i < before.length; i++) previous[before[i].hostId + ":" + before[i].id] = before[i]
  for (var j = 0; j < after.length; j++) {
    var thread = after[j], key = thread.hostId + ":" + thread.id, old = previous[key]
    if (!old) continue
    if (old.generation !== thread.generation || navigationKey(old.navigation) !== navigationKey(thread.navigation)) continue
    var state = old.state !== thread.state && old.state !== "unknown" && thread.state !== "unknown"
    var a = old.children, b = thread.children
    var children = !!(a && b && !b.stale && a.stamp !== null && b.stamp > a.stamp
      && b.starts >= a.starts && b.stops >= a.stops && (b.starts > a.starts || b.stops > a.stops))
    // The first event after an observed zero baseline is also a measured change.
    children = children || !!(a && b && !b.stale && a.starts === 0 && a.stops === 0 && b.stamp !== null && (b.starts > 0 || b.stops > 0))
    var c = old.completion, d = thread.completion
    children = children || !!(c && d && !d.stale && d.stamp > c.stamp && d.done > c.done)
    var compaction = !!(old.usage && thread.usage && !thread.usage.stale
      && old.usage.compactions !== null && thread.usage.compactions > old.usage.compactions)
    if (state || children || compaction) changes[key] = { state: state, children: children, compaction: compaction }
  }
  return changes
}

function resetLabel(seconds) {
  return Math.floor(seconds / 86400) + "d " + (seconds < 3600 ? "<1" : Math.floor(seconds % 86400 / 3600)) + "h"
}

// Highest-priority observed state wins; unavailable sources never imply completion.
function dominantState(threads, acknowledgements) {
  var priority = { unknown: 0, idle: 0, done: 1, working: 2, blocked: 3 }
  var state = "idle"
  for (var i = 0; i < threads.length; i++) {
    if (threads[i].state === "done" && acknowledgements && acknowledgements[threadKey(threads[i])] === completionEpisode(threads[i])) continue
    if (priority[threads[i].state] > priority[state]) state = threads[i].state
  }
  return state
}

function threadKey(thread) { return thread.hostId + ":" + thread.id }
function navigationKey(binding) { return binding && typeof binding === "object" ? String(binding.profile_id || binding.host_id || "") + ":" + String(binding.route_key || "") : "" }
function navigationArgs(thread) {
  var args = ["--open-thread", thread.hostId, thread.id]
  if (thread.navigation !== null && thread.navigation !== undefined) {
    var binding = thread.navigation
    var routeId = binding && (binding.profile_id || binding.host_id)
    if (!binding || typeof binding !== "object" || Array.isArray(binding) || Object.keys(binding).length !== 2
        || typeof routeId !== "string" || !/^[A-Za-z0-9][A-Za-z0-9_-]{0,39}$/.test(routeId)
        || (Object.prototype.hasOwnProperty.call(binding, "profile_id") === Object.prototype.hasOwnProperty.call(binding, "host_id"))
        || typeof binding.route_key !== "string" || !/^[a-f0-9]{64}$/.test(binding.route_key)) return null
    args.push(JSON.stringify(binding))
  }
  return args
}
function completionEpisode(thread) {
  var route = navigationKey(thread.navigation)
  return String(thread.generation || 0) + ":" + String(thread.statusGeneration || 0) + (route ? ":" + route : "")
}

// Keep surviving rows in place, append genuine arrivals. State changes never move a target.
function stableThreads(before, after) {
  var current = {}, result = []
  after.forEach(function(thread) { current[threadKey(thread)] = thread })
  before.forEach(function(thread) {
    var key = threadKey(thread)
    if (current[key]) { result.push(current[key]); delete current[key] }
  })
  after.forEach(function(thread) { var key = threadKey(thread); if (current[key]) { result.push(thread); delete current[key] } })
  return result
}

// Only continuous reporting establishes an arrival. Reconnection is hydration.
function arrivals(before, after) {
  var existing = {}, liveHosts = {}, result = {}
  before.threads.forEach(function(thread) { existing[threadKey(thread)] = true })
  before.hosts.forEach(function(host) { if (host.reporting) liveHosts[host.id] = navigationKey(host.navigation) })
  after.threads.forEach(function(thread) { if (liveHosts[thread.hostId] !== undefined && liveHosts[thread.hostId] === navigationKey(thread.navigation) && !existing[threadKey(thread)]) result[threadKey(thread)] = true })
  return result
}

function providerGroups(allowances) {
  var groups = [], indices = Object.create(null)
  allowances.forEach(function(account) {
    if (indices[account.provider] === undefined) { indices[account.provider] = groups.length; groups.push({ id: account.provider, label: account.providerLabel, accounts: [], keys: [] }) }
    groups[indices[account.provider]].accounts.push(account)
    groups[indices[account.provider]].keys.push(accountKey(account))
  })
  return groups
}

function childHint(completion) {
  if (!completion) return "Subagent status unavailable"
  var parts = [completion.done + " completed"]
  if (completion.outcomes) {
    ;["running", "interrupted", "failed", "unknown"].forEach(function(name) {
      if (completion.outcomes[name]) parts.push(completion.outcomes[name] + " " + (name === "unknown" ? "unavailable" : name))
    })
  } else if (completion.total > completion.done) parts.push((completion.total - completion.done) + " unresolved")
  return parts.join(" · ") + (completion.stale ? "\nLast reported " + completion.age : "")
}

// Preserve configured host order and each thread's original navigation index.
function groupThreads(view, hiddenStates, collapsedHosts) {
  hiddenStates = hiddenStates || []
  collapsedHosts = collapsedHosts || []
  return view.hosts.map(function(host) {
    var indices = []
    view.threads.forEach(function(thread, index) {
      if (host.reporting && thread.hostId === host.id) indices.push(index)
    })
    var filtered = indices.filter(function(index) { return hiddenStates.indexOf(view.threads[index].state) < 0 })
    var collapsed = collapsedHosts.indexOf(host.id) >= 0
    var shown = collapsed ? [] : filtered
    return { host: host, total: indices.length, matching: filtered.length, collapsed: collapsed, indices: shown,
             keys: shown.map(function(index) { return threadKey(view.threads[index]) }) }
  })
}

// Source-supplied text: 1 to 80 characters without control characters, else null.
function statusText(value) {
  if (typeof value !== "string" || value === "" || /[\u0000-\u001f\u007f-\u009f]/.test(value)) return null
  var length = Array.from(value).length
  return length >= 1 && length <= 80 ? value : null
}

function boundedLabel(value, limit) {
  if (typeof value !== "string" || value === "" || /[\u0000-\u001f\u007f-\u009f]/.test(value)) return null
  var length = Array.from(value).length
  return length <= limit ? value : null
}

// The single window the source flags for pacing. Zero or several flagged
// windows leave balance and pace unknown; list order never decides.
function pacingWindow(windows) {
  if (!Array.isArray(windows) || windows.length > 8) return null
  var flagged = windows.filter(function(window) { return window !== null && typeof window === "object" && window.pacing === true })
  if (flagged.length !== 1) return null
  var window = flagged[0], duration = number(window.duration_s)
  if (duration === null || Math.floor(duration) !== duration || duration < 1 || duration > 31622400) return null
  if (typeof window.kind !== "string" || !/^[a-z0-9_]{1,24}$/.test(window.kind) || boundedLabel(window.label, 40) === null) return null
  return window
}

// One provider-neutral snapshot row to its view. The view knows no provider,
// window kind or default duration; missing data stays unknown.
function allowanceView(row, nowMs) {
  if (!row || typeof row !== "object") return null
  var accountId = label(row.account_id, label(row.label, ""))
  var provider = label(row.provider, "")
  if (!/^[a-z0-9][a-z0-9_.-]{0,31}$/.test(provider) || !accountId || accountId.length > 128 || !/^[A-Za-z0-9_.-]+$/.test(accountId)) return null
  var status = ["available", "unavailable", "auth_needed"].indexOf(row.status) >= 0 ? row.status : "unavailable"
  var age = ageSeconds(row.sampled_at, nowMs, 1)
  // Measurement freshness: an allowance observation is current for 600 s.
  var fresh = status === "available" && age !== null && age <= 600
  var pacing = fresh ? pacingWindow(row.windows) : null
  var nowS = nowMs / 1000
  var used = pacing ? percent(pacing.used_percent) : null
  var reset = pacing ? number(pacing.resets_at) : null
  if (reset !== null && reset <= nowS) reset = null
  var current = used !== null && reset !== null
  var remaining = current ? 100 - used : null
  var untilReset = reset !== null ? reset - nowS : null
  var timeRemaining = untilReset !== null && untilReset <= pacing.duration_s ? untilReset / pacing.duration_s * 100 : null
  var resetCount = fresh && counter(row.reset_count) !== null && row.reset_count <= 10000
    && (row.reset_expires_at === null || row.reset_expires_at === undefined
        || (number(row.reset_expires_at) !== null && row.reset_expires_at > nowS)) ? row.reset_count : null
  return { id: accountId, provider: provider, providerLabel: boundedLabel(row.provider_label, 40) || provider, label: boundedLabel(row.label, 40) || accountId,
           statusText: statusText(row.status_text),
           remaining: remaining, timeRemaining: timeRemaining,
           paceDifference: current && timeRemaining !== null ? remaining - timeRemaining : null,
           resetCount: resetCount, reset: untilReset !== null ? resetLabel(untilReset) : null,
           age: current ? ageLabel(age) : "source unavailable" }
}

// Saved aliases (keyed provider:id) win, then the legacy preference table
// (keyed provider:label, supplied by Panel as data), then a stable hash.
function accountAlias(account, saved, legacy, pool) {
  var key = account.provider + ":" + account.id
  var named = function(table, name) {
    var value = table && typeof table === "object" && Object.prototype.hasOwnProperty.call(table, name) ? table[name] : null
    return typeof value === "string" && value !== "" ? value : null
  }
  var chosen = named(saved, key) || named(legacy, account.provider + ":" + account.label)
  if (chosen !== null) return chosen
  var hash = 0
  for (var i = 0; i < key.length; i++) hash = ((hash * 31) + key.charCodeAt(i)) >>> 0
  return pool[hash % pool.length]
}

function project(raw, nowMs) {
  var empty = { connected: false, working: null, partial: false, threads: [], hosts: [],
                allowances: [], discoveryLabel: "", note: "Observatory unavailable" }
  if (!raw || !Array.isArray(raw.hosts) || !Array.isArray(raw.allowances)) return empty
  var discovery = raw.fleet_discovery && raw.fleet_discovery.state
  if (["available", "unavailable", "discovering"].indexOf(discovery) < 0) discovery = "disabled"
  var interval = number(raw.interval)
  // Measurement freshness, not transport: a host sample stays current for its
  // sampling interval plus 20 s of peer and scheduling slack.
  var maxAge = (interval !== null && interval >= 2 && interval <= 60 ? interval : 5) + 20
  var hosts = [], threads = [], working = 0, missing = 0, reportingCount = 0
  for (var i = 0; i < raw.hosts.length; i++) {
    var host = raw.hosts[i]
    if (!host || typeof host !== "object") continue
    var age = ageSeconds(host.sampled_at, nowMs)
    var reporting = host.connection_state !== "setup_needed" && host.online === true && age !== null && age < maxAge && Array.isArray(host.agents)
    var connection = host.connection_state === "setup_needed" ? "setup_needed" : host.connection_state === "connecting" ? "connecting" : reporting ? "connected" : "unreachable"
    hosts.push({ connectionState: connection, connectionLabel: connection === "setup_needed" ? "Setup needed" : connection === "connecting" ? "Connecting" : connection === "connected" ? "Connected" : "Unreachable", id: label(host.id, "unknown"), name: label(host.label, label(host.id, "Host")), navigation: host.navigation === undefined ? null : host.navigation,
                 reporting: reporting, age: reporting ? ageLabel(age) : "source unavailable" })
    if (!reporting) { missing++; continue }
    reportingCount++
    var agents = Array.isArray(host.agents) ? host.agents : []
    for (var j = 0; j < agents.length; j++) {
      var agent = agents[j]
      if (!agent || typeof agent !== "object") continue
      var state = label(agent.status, "unknown").toLowerCase()
      if (["working", "blocked", "done", "idle", "unknown"].indexOf(state) < 0) state = "unknown"
      if (state === "working") working++
      threads.push({ id: label(agent.id, ""), hostId: label(host.id, "unknown"), project: label(agent.project, "Untitled"),
                     navigation: agent.navigation !== undefined && agent.navigation !== null ? agent.navigation : host.navigation === undefined ? null : host.navigation,
                     title: label(agent.title, "No task title reported"), host: label(host.label, label(host.id, "Host")),
                     timing: turnTiming(agent, nowMs, reporting), branch: label(agent.branch, ""), checkout: label(agent.checkout, ""), usage: threadUsage(agent, nowMs), children: childObservations(agent, nowMs), completion: childCompletion(agent, nowMs),
                     generation: counter(agent.technical && agent.technical.session_generation) || 0,
                     statusGeneration: counter(agent.technical && agent.technical.state_change_seq) || 0,
                     state: state, harness: label(agent.harness, "Unknown"), age: ageLabel(age) })
    }
  }
  threads.sort(function(a, b) {
    var order = { working: 0, blocked: 1, idle: 2, done: 3, unknown: 4 }
    return order[a.state] - order[b.state] || a.project.localeCompare(b.project)
  })
  var allowances = []
  for (var k = 0; k < raw.allowances.length; k++) {
    var account = allowanceView(raw.allowances[k], nowMs)
    if (account !== null) allowances.push(account)
  }
  // Account and provider order follows the configured collection order.
  return { connected: true, working: reportingCount > 0 ? working : null, partial: missing > 0, threads: threads,
           discoveryLabel: discovery === "unavailable" ? "Discovery unavailable" : discovery === "discovering" ? "Discovering" : "",
           hosts: hosts, allowances: allowances,
           note: reportingCount === 0 ? "No sources reporting" : missing > 0 ? missing + " source" + (missing === 1 ? "" : "s") + " unavailable" : "All sources reporting" }
}

function allowancePaceReading(difference) {
  if (typeof difference !== "number" || !isFinite(difference))
    return { difference: null, text: "—", band: "unknown" }
  var rounded = Math.round(Math.abs(difference) * 10 + 1e-9) / 10
  if (difference < 0 && rounded !== 0) rounded = -rounded
  return { difference: rounded,
           text: (rounded > 0 ? "+" : rounded < 0 ? "−" : "") + Math.abs(rounded).toFixed(1) + "%",
           band: allowancePaceBand(rounded) }
}

function allowancePaceBand(difference) {
  if (typeof difference !== "number" || !isFinite(difference)) return "unknown"
  if (difference >= 0) return "surplus"
  if (difference >= -5) return "caution"
  if (difference > -10) return "warning"
  return "deficit"
}

// Transport freshness. The runtime states its heartbeat; a snapshot is dropped
// once no line arrives within one heartbeat plus one coordinator loop wait
// (LOOP_WAIT, 1 s) plus 1 s of scheduling and pipe margin. An absent or
// implausible heartbeat falls back to 6 s, the value for today's 4 s heartbeat.
function receiptTimeoutMs(raw) {
  var heartbeat = raw && typeof raw === "object" ? number(raw.heartbeat_seconds) : null
  return heartbeat !== null && heartbeat >= 1 && heartbeat <= 60 ? (heartbeat + 1 + 1) * 1000 : 6000
}

// Keyboard focus follows a stable thread key, never a position in the view.
function focusKeys(view, groups) {
  var keys = []
  if (!view || !Array.isArray(view.threads) || !Array.isArray(groups)) return keys
  groups.forEach(function(group) {
    (group && Array.isArray(group.indices) ? group.indices : []).forEach(function(index) {
      if (view.threads[index]) keys.push(threadKey(view.threads[index]))
    })
  })
  return keys
}
function reconcileFocus(keys, key) { return key && keys.indexOf(key) >= 0 ? key : "" }
function moveFocus(keys, key, delta) {
  if (keys.length === 0) return ""
  var at = key ? keys.indexOf(key) : -1
  if (at < 0) return keys[0]
  return keys[Math.max(0, Math.min(keys.length - 1, at + delta))]
}
function activationKey(keys, key) { return key && keys.indexOf(key) >= 0 ? key : keys.length ? keys[0] : "" }
function threadForKey(view, key) {
  if (!key || !view || !Array.isArray(view.threads)) return null
  for (var i = 0; i < view.threads.length; i++) if (threadKey(view.threads[i]) === key) return view.threads[i]
  return null
}

// Display formatters and preference helpers (formerly in Panel.qml).
function tokens(value) {
  if (value === null || value === undefined) return "—"
  var scale = value >= 1e9 ? 1e9 : value >= 1e6 ? 1e6 : value >= 1e3 ? 1e3 : 1
  return (value / scale).toFixed(scale === 1 ? 0 : 1).replace(/\.0$/, "") + (scale === 1e9 ? "B" : scale === 1e6 ? "M" : scale === 1e3 ? "K" : "")
}
function percentReading(value) {
  if (value === null || value === undefined) return "—"
  if (value > 99 && value < 100) return ">99%"
  if (value > 0 && value < 1) return "<1%"
  return Math.round(value) + "%"
}
// Any object with remaining and timeRemaining percentages (an allowance reading).
function paceText(reading) {
  if (reading.remaining === null || reading.timeRemaining === null) return "Pace unavailable"
  return reading.remaining.toFixed(1).replace(/\.0$/, "") + "% left · " + reading.timeRemaining.toFixed(1).replace(/\.0$/, "") + "% expected"
}
function parseList(text) {
  try { var list = JSON.parse(text); return Array.isArray(list) ? list : [] } catch (error) { return [] }
}
function parseObject(text) {
  try {
    var object = JSON.parse(text)
    return object && typeof object === "object" && !Array.isArray(object) ? object : {}
  } catch (error) { return {} }
}
// Theme colour name for a thread state; the theme resolves the colour.
function stateColourName(state) {
  return state === "working" ? "yellow" : state === "blocked" ? "red" : state === "done" ? "green" : "muted"
}
function accountKey(account) { return account.provider + ":" + account.id }
function toggleListValue(list, value) {
  var next = (Array.isArray(list) ? list : []).slice(), at = next.indexOf(value)
  if (at < 0) next.push(value); else next.splice(at, 1)
  return next
}
// Local acknowledgement storage stays bounded: the earliest keys go first.
function boundAcknowledgements(object, limit) {
  if (limit === undefined) limit = 256
  var copy = {}, keys = Object.keys(object || {})
  keys.forEach(function(key) { copy[key] = object[key] })
  while (keys.length > limit) delete copy[keys.shift()]
  return copy
}
// An acknowledgement lasts only while its listed thread stays done in the same
// completion episode. Threads absent from the list are left untouched.
function reconcileAcknowledgements(acknowledged, threads) {
  var value = {}, changed = false
  Object.keys(acknowledged || {}).forEach(function(key) { value[key] = acknowledged[key] })
  ;(threads || []).forEach(function(thread) {
    var key = threadKey(thread)
    if (value[key] !== undefined && (thread.state !== "done" || value[key] !== completionEpisode(thread))) {
      delete value[key]
      changed = true
    }
  })
  return { value: value, changed: changed }
}
// After a successful launch: acknowledge a done target only if its thread is
// still done in the same episode. Otherwise the input is returned unchanged.
function acknowledgeNavigation(acknowledged, target, threads) {
  if (!target || target.state !== "done") return acknowledged
  var current = null
  ;(threads || []).forEach(function(thread) { if (current === null && threadKey(thread) === target.key) current = thread })
  if (!current || current.state !== "done" || completionEpisode(current) !== target.episode) return acknowledged
  var value = {}
  Object.keys(acknowledged || {}).forEach(function(key) { value[key] = acknowledged[key] })
  value[target.key] = target.episode
  return value
}
// Fisher-Yates shuffle of the alias pool with an injected random source.
function assignAliases(accounts, pool, random) {
  var shuffled = pool.slice(), assigned = {}
  for (var i = shuffled.length - 1; i > 0; i--) {
    var j = Math.floor(random() * (i + 1)), swap = shuffled[i]
    shuffled[i] = shuffled[j]
    shuffled[j] = swap
  }
  accounts.forEach(function(account, index) { assigned[accountKey(account)] = shuffled[index % shuffled.length] })
  return assigned
}
// Ordered ListModel edits turning `before` into `after` (arrays of unique keys).
// Indices refer to the list as left by the preceding edits; move(from, to)
// leaves the item at index `to`. A key in both lists is only ever moved.
function keyedEdits(before, after) {
  var edits = [], current = before.slice(), wanted = {}
  after.forEach(function(key) { wanted[key] = true })
  for (var i = current.length - 1; i >= 0; i--) {
    if (wanted[current[i]] !== true) { edits.push({ op: "remove", index: i }); current.splice(i, 1) }
  }
  for (var j = 0; j < after.length; j++) {
    if (current[j] === after[j]) continue
    var from = current.indexOf(after[j])
    if (from >= 0) { edits.push({ op: "move", from: from, to: j }); current.splice(from, 1) }
    else edits.push({ op: "insert", index: j, key: after[j] })
    current.splice(j, 0, after[j])
  }
  return edits
}
function threadIndex(threads) {
  var index = Object.create(null)
  ;(threads || []).forEach(function(thread) { index[threadKey(thread)] = thread })
  return index
}

if (typeof module !== "undefined") module.exports = { accountAlias: accountAlias, navigationArgs: navigationArgs, turnTiming: turnTiming, durationLabel: durationLabel, timingHint: timingHint, allowancePaceReading: allowancePaceReading, allowancePaceBand: allowancePaceBand, threadKey: threadKey, completionEpisode: completionEpisode, stableThreads: stableThreads, arrivals: arrivals, providerGroups: providerGroups, childHint: childHint, groupThreads: groupThreads, transitions: transitions, dominantState: dominantState, project: project, ageSeconds: ageSeconds, ageLabel: ageLabel, receiptTimeoutMs: receiptTimeoutMs, focusKeys: focusKeys, reconcileFocus: reconcileFocus, moveFocus: moveFocus, activationKey: activationKey, threadForKey: threadForKey,
  tokens: tokens, percentReading: percentReading, paceText: paceText, parseList: parseList, parseObject: parseObject, stateColourName: stateColourName, accountKey: accountKey,
  toggleListValue: toggleListValue, boundAcknowledgements: boundAcknowledgements, reconcileAcknowledgements: reconcileAcknowledgements, acknowledgeNavigation: acknowledgeNavigation, assignAliases: assignAliases,
  keyedEdits: keyedEdits, threadIndex: threadIndex }
