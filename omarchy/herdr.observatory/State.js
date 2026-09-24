// Pure presentation projection. Observatory owns collection and disclosure.
function number(value) {
  return typeof value === "number" && isFinite(value) ? value : null
}

function label(value, fallback) {
  return typeof value === "string" && value !== "" ? value : fallback
}

function ageSeconds(value, nowMs) {
  var at = number(value)
  if (at === null || at <= 0) return null
  var age = nowMs / 1000 - at
  return age >= 0 && isFinite(age) ? age : null
}

function ageLabel(age) {
  if (age === null) return "age unknown"
  if (age < 60) return Math.floor(age) + "s ago"
  if (age < 3600) return Math.floor(age / 60) + "m ago"
  return Math.floor(age / 3600) + "h ago"
}

function project(raw, nowMs) {
  var empty = { connected: false, working: null, partial: false, threads: [], hosts: [],
                allowances: [], gpu: null, inference: "Inference use unavailable", note: "Observatory unavailable" }
  if (!raw || !Array.isArray(raw.hosts) || raw.hosts.length === 0 || !Array.isArray(raw.allowances)) return empty
  var interval = number(raw.interval)
  var maxAge = (interval !== null && interval >= 2 && interval <= 60 ? interval : 5) + 20
  var hosts = [], threads = [], working = 0, missing = 0, reportingCount = 0, gpu = null
  for (var i = 0; i < raw.hosts.length; i++) {
    var host = raw.hosts[i]
    if (!host || typeof host !== "object") continue
    var age = ageSeconds(host.sampled_at, nowMs)
    var reporting = host.online === true && age !== null && age < maxAge && Array.isArray(host.agents)
    hosts.push({ id: label(host.id, "unknown"), name: label(host.label, label(host.id, "Host")),
                 reporting: reporting, age: reporting ? ageLabel(age) : "source unavailable" })
    if (!reporting) { missing++; continue }
    reportingCount++
    if (host.id === "ws-255" && host.metrics && host.metrics.gpu && number(host.metrics.gpu.percent) !== null
        && number(host.metrics.gpu.percent) >= 0 && number(host.metrics.gpu.percent) <= 100) {
      gpu = { host: label(host.label, label(host.id, "Host")), percent: host.metrics.gpu.percent, age: ageLabel(age) }
    }
    var agents = Array.isArray(host.agents) ? host.agents : []
    for (var j = 0; j < agents.length; j++) {
      var agent = agents[j]
      if (!agent || typeof agent !== "object") continue
      var state = label(agent.status, "unknown").toLowerCase()
      if (["working", "blocked", "done", "idle", "unknown"].indexOf(state) < 0) state = "unknown"
      if (state === "working") working++
      threads.push({ id: label(agent.id, ""), project: label(agent.project, "Untitled"),
                     title: label(agent.title, "No task title reported"), host: label(host.label, label(host.id, "Host")),
                     state: state, harness: label(agent.harness, "Unknown"), age: ageLabel(age) })
    }
  }
  threads.sort(function(a, b) {
    var order = { working: 0, blocked: 1, idle: 2, done: 3, unknown: 4 }
    return order[a.state] - order[b.state] || a.project.localeCompare(b.project)
  })
  var allowances = []
  for (var k = 0; k < raw.allowances.length; k++) {
    var row = raw.allowances[k]
    if (!row || (row.label !== "Personal" && row.label !== "Work")) continue
    var balance = number(row.weekly_remaining)
    var reset = number(row.weekly_resets_at)
    var age = ageSeconds(row.sampled_at, nowMs)
    var current = row.available === true && age !== null && age <= 600
                  && balance !== null && balance >= 0 && balance <= 100
                  && reset !== null && reset > nowMs / 1000
    allowances.push({ label: row.label, remaining: current ? balance : null,
                      age: current ? ageLabel(age) : "source unavailable" })
  }
  allowances.sort(function(a, b) { return a.label === "Personal" ? -1 : b.label === "Personal" ? 1 : 0 })
  return { connected: true, working: reportingCount > 0 ? working : null, partial: missing > 0, threads: threads,
           hosts: hosts, allowances: allowances, gpu: gpu,
           inference: "Inference use unavailable",
           note: reportingCount === 0 ? "No sources reporting" : missing > 0 ? missing + " source" + (missing === 1 ? "" : "s") + " unavailable" : "All sources reporting" }
}

if (typeof module !== "undefined") module.exports = { project: project, ageSeconds: ageSeconds, ageLabel: ageLabel }
