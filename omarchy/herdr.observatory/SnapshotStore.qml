import QtQuick
import Quickshell.Io
import "State.js" as State

// Owns the collector and the structural view. Every path (receipt, refresh,
// restart, collector exit and the 1 s tick) goes through State.storeStep, so
// the view is replaced only when its structure changes: on a receipt, or at a
// freshness deadline, whether the popover is open or closed.
Item {
    id: root

    property double lastReceipt: 0
    // Display instant for relative-time readings. It advances only while the
    // popover shows them (visualUpdates): on opening, each tick and each receipt.
    property double now: Date.now()
    property var raw: null
    // The storeStep state; callers read raw, lastReceipt and view instead.
    readonly property var store: ({
            raw: null,
            lastReceipt: 0,
            view: null,
            signature: "",
            deadline: null
        })
    property var view: State.project(null, Date.now())
    property bool visualUpdates: true

    function accept(line) {
        var parsed = null;
        try {
            if (line.length > 2097152)
                throw new Error("Oversized snapshot");

            parsed = JSON.parse(line);
        } catch (error) {
            parsed = null;
        }
        step({
            type: "receipt",
            raw: parsed
        });
    }
    // Explicit operator refresh (r, middle-click, IPC): ask a running collector
    // for a fresh local sample round, or start a collector that has exited.
    function refresh() {
        if (collector.running)
            collector.write("refresh\n");
        else
            collector.running = true;

        step({
            type: "update"
        });
    }
    // Opening the popover and the retry timer only restart a dead collector.
    // They never write to stdin, so opening adds no host sample round.
    function restart() {
        if (!collector.running)
            collector.running = true;

        step({
            type: "update"
        });
    }
    function step(event) {
        var at = Date.now();
        var replaced = State.storeStep(store, event, at);
        if (raw !== store.raw)
            raw = store.raw;
        lastReceipt = store.lastReceipt;
        if (visualUpdates && event.type !== "update")
            now = at;
        // Assign only on replacement: reassigning the same object still notifies.
        if (replaced)
            view = store.view;
    }

    Component.onCompleted: {
        store.view = view;
        store.signature = State.viewSignature(view);
    }
    onVisualUpdatesChanged: {
        if (visualUpdates)
            now = Date.now();
    }

    Process {
        id: collector

        command: [Qt.resolvedUrl("anton-runtime").toString().replace(/^file:\/\//, "")]
        running: true
        stdinEnabled: true

        stdout: SplitParser {
            onRead: function (line) {
                root.accept(line);
            }
        }
    }
    // A collector exit drops the snapshot and schedules a restart.
    Connections {
        function onExited() {
            root.store.raw = null;
            root.step({
                type: "update"
            });
            retry.restart();
        }

        target: collector
    }
    Timer {
        id: retry

        interval: 5000

        onTriggered: root.restart()
    }
    Timer {
        interval: 1000
        repeat: true
        running: true

        // Receipt timeout (see State.receiptTimeoutMs), then any freshness deadline.
        onTriggered: root.step({
            type: "tick"
        })
    }
}
