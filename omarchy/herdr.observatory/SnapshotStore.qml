import QtQuick
import "State.js" as State

Item {
  id: root
  property var view: State.project(null, Date.now())
  property var raw: null
  property var pending: null
  property int generation: 0
  property string endpoint: "http://127.0.0.1:8789/api/state"

  function update() { view = State.project(raw, Date.now()) }

  function refresh() {
    if (pending) return
    var request = new XMLHttpRequest()
    pending = request
    var serial = ++generation
    request.onreadystatechange = function() {
      if (request.readyState !== XMLHttpRequest.DONE || serial !== generation) return
      pending = null
      deadline.stop()
      try {
        if (request.status !== 200 || request.responseText.length > 2097152) throw new Error("Unavailable state")
        raw = JSON.parse(request.responseText)
      } catch (error) { raw = null }
      update()
    }
    try {
      request.open("GET", endpoint)
      request.send()
      deadline.restart()
    } catch (error) {
      pending = null
      raw = null
      update()
    }
  }

  Timer { id: poll; interval: 5000; running: true; repeat: true; triggeredOnStart: true; onTriggered: root.refresh() }
  Timer {
    id: deadline
    interval: 4000
    onTriggered: {
      if (!root.pending) return
      root.generation++
      var request = root.pending
      root.pending = null
      request.abort()
      root.raw = null
      root.update()
    }
  }
  Timer { interval: 1000; running: true; repeat: true; onTriggered: root.update() }
}
