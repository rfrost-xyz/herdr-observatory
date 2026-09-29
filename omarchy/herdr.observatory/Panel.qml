import QtQuick
import QtCore as Core
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import qs.Commons
import qs.Ui
import "State.js" as State

Panel {
    id: root

    property var accountEmails: ({})
    readonly property var aliases: ["Richard Hendricks", "Erlich Bachman", "Gilfoyle", "Dinesh Chugtai", "Jared Dunn", "Monica Hall", "Gavin Belson", "Big Head", "Jian-Yang", "Russ Hanneman"]
    readonly property bool allowancesCollapsed: parseList(identitySettings.collapsedSections).indexOf("allowances") >= 0
    readonly property color barColour: bar ? bar.barForeground : Color.foreground
    readonly property string barState: State.dominantState(overview.threads, parseObject(identitySettings.acknowledgedCompletions))
    readonly property int blocked: overview.threads.filter(function (t) {
        return t.state === "blocked";
    }).length
    readonly property color blue: palette.blue || Color.accent
    readonly property var collapsedHosts: parseList(identitySettings.collapsedHosts)
    readonly property var contentItem: popupContent.threadContent
    readonly property color cyan: palette.cyan || Color.accent
    readonly property string face: bar ? bar.fontFamily : Style.font.family
    property string focusedKey: ""
    readonly property color green: palette.green || Color.accent
    readonly property var hiddenStates: parseList(identitySettings.hiddenStates)
    readonly property color ink: Color.popups.text
    readonly property color line: alpha(ink, 0.12)
    readonly property bool motionEnabled: Quickshell.env("ANTON_REDUCED_MOTION") !== "1"
    readonly property color muted: alpha(ink, 0.62)
    property string navigationError: ""
    property var navigationTarget: null
    readonly property var overview: snapshot.view
    property var palette: ({})
    readonly property var preferences: identitySettings
    property var previousOverview: ({
            threads: [],
            hosts: []
        })
    readonly property var providers: State.providerGroups(overview.allowances)
    readonly property color red: palette.red || Color.urgent
    readonly property int reporting: overview.hosts.filter(function (h) {
        return h.reporting;
    }).length
    readonly property var threadGroups: State.groupThreads(overview, hiddenStates, collapsedHosts)
    readonly property var threadKeys: threadsCollapsed ? [] : State.focusKeys(overview, threadGroups)
    readonly property var threadViewport: popupContent.threadViewport
    readonly property bool threadsCollapsed: parseList(identitySettings.collapsedSections).indexOf("threads") >= 0
    readonly property var tooltipHost: popupContent
    property int visualEpoch: 0
    readonly property color yellow: palette.yellow || Color.accent

    signal newThreads(var keys)
    signal observedChange(var changes)

    function accountAlias(account) {
        var saved = parseObject(identitySettings.accountAliases)[accountKey(account)];
        if (saved)
            return saved;
        if (account.provider === "codex" && account.label === "Personal")
            return identitySettings.personalAlias;
        if (account.provider === "codex" && account.label === "Work")
            return identitySettings.workAlias;
        var key = accountKey(account), hash = 0;
        for (var i = 0; i < key.length; i++)
            hash = ((hash * 31) + key.charCodeAt(i)) >>> 0;
        return aliases[hash % aliases.length];
    }
    function accountKey(account) {
        return account.provider + ":" + account.id;
    }
    function alpha(colour, opacity) {
        return Qt.rgba(colour.r, colour.g, colour.b, opacity);
    }
    function openThread(key) {
        var entry = State.threadForKey(overview, key);
        if (entry === null || threadLauncher.running)
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
        threadLauncher.command = [Qt.resolvedUrl("anton-runtime").toString().replace(/^file:\/\//, "")].concat(args);
        threadLauncher.running = true;
    }
    function paceText(account) {
        if (account.remaining === null || account.timeRemaining === null)
            return "Pace unavailable";
        return account.remaining.toFixed(1).replace(/\.0$/, "") + "% left · " + account.timeRemaining.toFixed(1).replace(/\.0$/, "") + "% expected";
    }
    function parseList(value) {
        try {
            var list = JSON.parse(value);
            return Array.isArray(list) ? list : [];
        } catch (error) {
            return [];
        }
    }
    function parseObject(value) {
        try {
            var object = JSON.parse(value);
            return object && typeof object === "object" && !Array.isArray(object) ? object : {};
        } catch (error) {
            return {};
        }
    }
    function parsePalette(raw) {
        var next = {}, lines = String(raw || "").split("\n");
        for (var i = 0; i < lines.length; i++) {
            var match = lines[i].match(/^\s*([A-Za-z_]+)\s*=\s*["'](#[0-9A-Fa-f]{6})["']/);
            if (match)
                next[match[1]] = match[2];
        }
        palette = next;
    }
    function percentReading(value) {
        if (value === null || value === undefined)
            return "—";
        if (value > 99 && value < 100)
            return ">99%";
        if (value > 0 && value < 1)
            return "<1%";
        return Math.round(value) + "%";
    }
    function saveAcknowledgements(value) {
        // Keep local acknowledgement storage bounded, even as sessions come and go.
        var keys = Object.keys(value);
        while (keys.length > 256)
            delete value[keys.shift()];
        identitySettings.acknowledgedCompletions = JSON.stringify(value);
        identitySettings.setValue("acknowledgedCompletions", identitySettings.acknowledgedCompletions);
        identitySettings.sync();
    }
    function stateColour(state) {
        return state === "working" ? yellow : state === "blocked" ? red : state === "done" ? green : muted;
    }
    function toggleIdentity() {
        var hidden = identitySettings.namesHidden;
        if (!hidden) {
            var shuffled = aliases.slice(), assigned = {};
            for (var i = shuffled.length - 1; i > 0; i--) {
                var j = Math.floor(Math.random() * (i + 1)), swap = shuffled[i];
                shuffled[i] = shuffled[j];
                shuffled[j] = swap;
            }
            overview.allowances.forEach(function (account, index) {
                assigned[accountKey(account)] = shuffled[index % shuffled.length];
            });
            identitySettings.accountAliases = JSON.stringify(assigned);
            identitySettings.setValue("accountAliases", identitySettings.accountAliases);
        }
        identitySettings.namesHidden = !hidden;
        identitySettings.setValue("namesHidden", !hidden);
        identitySettings.setValue("personalAlias", identitySettings.personalAlias);
        identitySettings.setValue("workAlias", identitySettings.workAlias);
        identitySettings.sync();
    }
    function toggleList(key, value) {
        visualEpoch++;
        var list = parseList(identitySettings[key]).slice(), at = list.indexOf(value);
        if (at < 0)
            list.push(value);
        else
            list.splice(at, 1);
        identitySettings[key] = JSON.stringify(list);
        identitySettings.setValue(key, identitySettings[key]);
        identitySettings.sync();
        focusedKey = "";
    }
    function tokens(value) {
        if (value === null || value === undefined)
            return "—";
        var scale = value >= 1e9 ? 1e9 : value >= 1e6 ? 1e6 : value >= 1e3 ? 1e3 : 1;
        return (value / scale).toFixed(scale === 1 ? 0 : 1).replace(/\.0$/, "") + (scale === 1e9 ? "B" : scale === 1e6 ? "M" : scale === 1e3 ? "K" : "");
    }

    implicitHeight: button.implicitHeight
    implicitWidth: button.implicitWidth
    ipcTarget: moduleName
    manageIpc: false
    moduleName: "herdr.observatory"

    onOpenedChanged: {
        visualEpoch++;
        if (opened) {
            focusedKey = "";
            paletteFile.reload();
            snapshot.refresh();
            Qt.callLater(function () {
                keyCatcher.forceActiveFocus();
            });
        }
    }
    onOverviewChanged: {
        var before = previousOverview || {
            threads: [],
            hosts: []
        };
        var changes = State.transitions(before.threads, overview.threads);
        var added = State.arrivals(before, overview);
        previousOverview = overview;
        if (identitySettings) {
            var acknowledged = parseObject(identitySettings.acknowledgedCompletions), changed = false;
            overview.threads.forEach(function (thread) {
                var key = State.threadKey(thread);
                if (acknowledged[key] !== undefined && (thread.state !== "done" || acknowledged[key] !== State.completionEpisode(thread))) {
                    delete acknowledged[key];
                    changed = true;
                }
            });
            if (changed)
                saveAcknowledgements(acknowledged);
        }
        var epoch = visualEpoch;
        if (opened && motionEnabled)
            Qt.callLater(function () {
                if (!root.opened || !root.motionEnabled || root.visualEpoch !== epoch)
                    return;
                root.observedChange(changes);
                root.newThreads(added);
            });
    }
    // Focus is a thread key; it clears when that thread leaves the visible order.
    onThreadKeysChanged: focusedKey = State.reconcileFocus(threadKeys, focusedKey)

    Core.Settings {
        id: identitySettings

        property string accountAliases: "{}"
        property string acknowledgedCompletions: "{}"
        property string collapsedHosts: "[]"
        property string collapsedProviders: "[]"
        property string collapsedSections: "[]"
        property string hiddenStates: "[]"
        property bool namesHidden: false
        property string personalAlias: "Richard Hendricks"
        property string workAlias: "Jared Dunn"

        location: "file://" + (Quickshell.env("XDG_STATE_HOME") || Quickshell.env("HOME") + "/.local/state") + "/herdr.observatory/privacy.ini"

        Component.onCompleted: {
            if (value("privacyVersion", 0) < 2) {
                namesHidden = value("personalHidden", false) || value("workHidden", false);
                setValue("namesHidden", namesHidden);
                setValue("privacyVersion", 2);
                sync();
            }
        }
    }
    FileView {
        id: identitiesFile

        path: Qt.resolvedUrl(".accounts.json").toString().replace(/^file:\/\//, "")
        printErrors: false
        watchChanges: true

        onFileChanged: reload()
        onLoaded: {
            try {
                root.accountEmails = JSON.parse(text());
            } catch (error) {
                root.accountEmails = ({});
            }
        }
    }
    Process {
        id: threadLauncher

        stderr: StdioCollector {
            onStreamFinished: root.navigationError = text.trim()
        }

        onExited: function (code) {
            if (code === 0) {
                if (root.navigationTarget && root.navigationTarget.state === "done") {
                    var current = root.overview.threads.filter(function (thread) {
                        return State.threadKey(thread) === root.navigationTarget.key;
                    })[0];
                    if (current && current.state === "done" && State.completionEpisode(current) === root.navigationTarget.episode) {
                        var acknowledged = root.parseObject(identitySettings.acknowledgedCompletions);
                        acknowledged[root.navigationTarget.key] = root.navigationTarget.episode;
                        root.saveAcknowledgements(acknowledged);
                    }
                }
                root.navigationError = "";
                root.close();
            } else if (!root.navigationError)
                root.navigationError = "Could not open this thread in Herdr.";
        }
    }
    SnapshotStore {
        id: snapshot

        visualUpdates: root.opened
    }
    FileView {
        id: paletteFile

        path: Quickshell.env("HOME") + "/.local/state/omarchy/current/theme/colors.toml"
        printErrors: false
        watchChanges: true

        onFileChanged: reload()
        onLoaded: root.parsePalette(text())
    }
    Connections {
        function onAccentChanged() {
            paletteFile.reload();
        }
        function onBackgroundChanged() {
            paletteFile.reload();
        }

        target: Color
    }
    IpcHandler {
        function close(): void {
            root.close();
        }
        function diagnostics(): string {
            return JSON.stringify({
                connected: root.overview.connected,
                hosts: root.overview.hosts.map(function (h) {
                    return {
                        id: h.id,
                        connection: h.connectionState,
                        reporting: h.reporting
                    };
                }),
                threads: root.overview.threads.length,
                usageReported: root.overview.threads.filter(function (t) {
                    return t.usage && t.usage.inputTokens !== null;
                }).length,
                timingReported: root.overview.threads.filter(function (t) {
                    return t.timing && t.timing.elapsed !== null;
                }).length,
                timingCurrent: root.overview.threads.filter(function (t) {
                    return t.timing && t.timing.active && !t.timing.stale && t.timing.elapsed !== null;
                }).length,
                timingTotals: root.overview.threads.filter(function (t) {
                    return t.timing && t.timing.total !== null;
                }).length,
                timingStale: root.overview.threads.filter(function (t) {
                    return t.timing && t.timing.stale;
                }).length,
                allowances: root.overview.allowances.map(function (a) {
                    return {
                        label: a.label,
                        available: a.remaining !== null
                    };
                })
            });
        }
        function open(): void {
            root.open();
        }
        function refresh(): void {
            snapshot.refresh();
            paletteFile.reload();
        }
        function status(): string {
            return root.opened ? "open" : "closed";
        }
        function toggle(): void {
            root.toggle();
        }

        target: root.ipcTarget
    }
    BarIconButton {
        id: button

        anchors.fill: parent
        bar: root.bar
        foreground: root.barColour
        text: "󱚣"
        tooltipText: "Anton · " + root.barState + (root.overview.partial || !root.overview.connected ? " · sources unavailable" : "")
        useActiveColor: false

        onPressed: function (code) {
            if (code === Qt.MiddleButton)
                snapshot.refresh();
            else
                root.toggle();
        }

        Item {
            anchors.centerIn: parent
            anchors.horizontalCenterOffset: Style.space(6)
            anchors.verticalCenterOffset: -Style.space(5)
            height: Style.space(12)
            width: Style.space(10)

            AntonText {
                anchors.centerIn: parent
                color: root.stateColour(root.barState)
                font.bold: true
                font.pixelSize: Style.space(12)
                text: root.barState === "blocked" ? "!" : root.barState === "done" ? "✓" : ""
                ui: root
            }
            Rectangle {
                anchors.centerIn: parent
                color: root.yellow
                height: width
                radius: width / 2
                visible: root.barState === "working"
                width: Style.space(6)
            }
        }
    }
    KeyboardPanel {
        id: panel

        anchorItem: button
        bar: root.bar
        contentHeight: panel.fittedContentHeight(popupContent.implicitHeight, Style.space(540))
        contentWidth: panel.fittedContentWidth(Style.space(360))
        focusTarget: keyCatcher
        open: root.opened
        owner: root
        padding: Style.spacing.popupPadding

        PanelKeyCatcher {
            id: keyCatcher

            anchors.fill: parent

            onActivateRequested: root.openThread(State.activationKey(root.threadKeys, root.focusedKey))
            onCloseRequested: root.close()
            onMoveRequested: function (dx, dy) {
                root.focusedKey = State.moveFocus(root.threadKeys, root.focusedKey, dx + dy);
            }
            onTabRequested: function (direction) {
                root.switchPanel(direction);
            }
            onTextKey: function (key) {
                if (key === "r" || key === "R")
                    snapshot.refresh();
            }

            PopupContent {
                id: popupContent

                anchors.fill: parent
                ui: root
            }
        }
    }

    // Shared typography and surfaces keep one spacing and interaction system.

}
