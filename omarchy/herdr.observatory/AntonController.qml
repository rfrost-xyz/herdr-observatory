import QtQuick
import Quickshell.Io
import "State.js" as State

// Popover actions and derived presentation state: navigation, completion
// acknowledgements, keyboard focus, filters and collapse, identity concealment
// and refresh requests. Panel.qml wires it to the store, preferences and shell.
QtObject {
    id: controller

    // Concealed account names are drawn from this pool.
    readonly property var aliasPool: ["Richard Hendricks", "Erlich Bachman", "Gilfoyle", "Dinesh Chugtai", "Jared Dunn", "Monica Hall", "Gavin Belson", "Big Head", "Jian-Yang", "Russ Hanneman"]
    // The bar ignores completions the operator has already opened.
    readonly property string barState: State.dominantState(view.threads, preferences ? preferences.acknowledgements : {})
    property string focusedKey: ""
    readonly property Process launcher: Process {
        stderr: StdioCollector {
            onStreamFinished: controller.navigationError = text.trim()
        }

        onExited: function (code) {
            if (code === 0) {
                var acknowledged = controller.preferences.acknowledgements;
                var next = State.acknowledgeNavigation(acknowledged, controller.navigationTarget, controller.view.threads);
                if (next !== acknowledged)
                    controller.preferences.setAcknowledgements(next);
                controller.navigationError = "";
                controller.closeRequested();
            } else if (!controller.navigationError)
                controller.navigationError = "Could not open this thread in Herdr.";
        }
    }
    property bool motionEnabled: true
    property string navigationError: ""
    property var navigationTarget: null
    // Panel assigns this on each open and close, so the epoch and focus reset
    // below happen before the store restarts.
    property bool opened: false
    property AntonPreferences preferences: null
    property var previousView: ({
            threads: [],
            hosts: []
        })
    readonly property var providers: State.providerGroups(view.allowances)
    // Injected so tests can make the alias shuffle deterministic.
    property var random: Math.random
    property string runtimePath: Qt.resolvedUrl("anton-runtime").toString().replace(/^file:\/\//, "")
    readonly property var threadGroups: State.groupThreads(view, preferences ? preferences.hiddenStates : [], preferences ? preferences.collapsedHosts : [])
    readonly property var threadKeys: preferences && preferences.threadsCollapsed ? [] : State.focusKeys(view, threadGroups)
    property var view: State.project(null, 0)
    property int visualEpoch: 0

    signal closeRequested
    signal newThreads(var keys)
    signal observedChange(var changes)
    signal refreshRequested

    function accountAlias(account) {
        return State.accountAlias(account, preferences.accountAliases, preferences.legacyAliases, aliasPool);
    }
    function activate() {
        openThread(State.activationKey(threadKeys, focusedKey));
    }
    function moveFocus(delta) {
        focusedKey = State.moveFocus(threadKeys, focusedKey, delta);
    }
    function openThread(key) {
        var entry = State.threadForKey(view, key);
        if (entry === null || launcher.running)
            return;
        var args = State.navigationArgs(entry);
        if (args === null) {
            navigationError = "Invalid thread route. Try again shortly.";
            return;
        }
        navigationTarget = {
            key: State.threadKey(entry),
            episode: State.completionEpisode(entry),
            state: entry.state
        };
        launcher.command = [runtimePath].concat(args);
        launcher.running = true;
    }
    function refresh() {
        refreshRequested();
    }
    // Concealing assigns fresh aliases; showing names keeps the saved ones.
    function toggleIdentity() {
        var hidden = preferences.namesHidden;
        preferences.setIdentity(!hidden, hidden ? null : State.assignAliases(view.allowances, aliasPool, random));
    }
    function toggleList(name, value) {
        visualEpoch++;
        preferences.toggle(name, value);
        focusedKey = "";
    }

    onOpenedChanged: {
        visualEpoch++;
        if (opened)
            focusedKey = "";
    }
    // Focus is a thread key; it clears when that thread leaves the visible order.
    onThreadKeysChanged: focusedKey = State.reconcileFocus(threadKeys, focusedKey)
    onViewChanged: {
        var before = previousView || {
            threads: [],
            hosts: []
        };
        var changes = State.transitions(before.threads, view.threads);
        var added = State.arrivals(before, view);
        previousView = view;
        if (preferences) {
            var reconciled = State.reconcileAcknowledgements(preferences.acknowledgements, view.threads);
            if (reconciled.changed)
                preferences.setAcknowledgements(reconciled.value);
        }
        var epoch = visualEpoch;
        if (opened && motionEnabled)
            Qt.callLater(function () {
                if (!controller.opened || !controller.motionEnabled || controller.visualEpoch !== epoch)
                    return;
                controller.observedChange(changes);
                controller.newThreads(added);
            });
    }
}
