import QtQuick
import Quickshell.Io
import "State.js" as State

Item {
    id: root

    property double lastReceipt: 0
    property string projectedSignature: ""
    property var raw: null
    property var view: State.project(null, Date.now())
    property bool visualUpdates: true

    function accept(line) {
        try {
            if (line.length > 2097152)
                throw new Error("Oversized snapshot");

            raw = JSON.parse(line);
            lastReceipt = Date.now();
        } catch (error) {
            raw = null;
        }
        update();
    }
    // Explicit operator refresh (r, middle-click, IPC): ask a running collector
    // for a fresh local sample round, or start a collector that has exited.
    function refresh() {
        if (collector.running)
            collector.write("refresh\n");
        else
            collector.running = true;

        update();
    }
    // Opening the popover and the retry timer only restart a dead collector.
    // They never write to stdin, so opening adds no host sample round.
    function restart() {
        if (!collector.running)
            collector.running = true;

        update();
    }
    function update() {
        var next = State.project(raw, Date.now());
        next.threads = State.stableThreads(view.threads, next.threads);
        var signature = JSON.stringify(next);
        if (signature !== projectedSignature) {
            projectedSignature = signature;
            view = next;
        }
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

        onExited: {
            root.raw = null;
            root.update();
            retry.restart();
        }
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

        onTriggered: {
            // The runtime states its heartbeat; see State.receiptTimeoutMs.
            if (root.raw !== null && Date.now() - root.lastReceipt > State.receiptTimeoutMs(root.raw)) {
                root.raw = null;
                root.update();
            } else if (root.visualUpdates) {
                root.update();
            }
        }
    }
}
