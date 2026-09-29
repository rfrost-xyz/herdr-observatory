import QtQuick
import "../../../omarchy/herdr.observatory/State.js" as State

QtObject {
    id: ui

    property var accountEmails: ({})
    readonly property bool allowancesCollapsed: parseList(preferences.collapsedSections).indexOf('allowances') >= 0
    property color blue: '#739fae'
    property var collapsedHosts: []
    readonly property var contentItem: popup ? popup.threadContent : null
    property string face: 'monospace'
    property int focusedThread: -1
    property color green: light ? '#397332' : '#9dc473'
    property var hiddenStates: []
    property color ink: light ? '#243030' : '#d7d6cd'
    property bool light: false
    property color line: light ? '#d1d9d7' : '#28383c'
    property bool motionEnabled: false
    property color muted: light ? '#637173' : '#8d9497'
    property string navigationError: ''
    property double now: 1800000000000
    property bool opened: true
    readonly property var overview: State.project(raw, now)
    property var popup: null
    property var preferences: ({
            namesHidden: true,
            collapsedSections: '[]',
            collapsedProviders: '[]'
        })
    readonly property var providers: State.providerGroups(overview.allowances)
    property var raw: ({
            hosts: [],
            allowances: []
        })
    property color red: light ? '#aa3e34' : '#ed7968'
    readonly property int reporting: overview.hosts.filter(function (h) {
        return h.reporting;
    }).length
    readonly property var threadGroups: State.groupThreads(overview, hiddenStates, collapsedHosts)
    readonly property var threadOrder: threadsCollapsed ? [] : threadGroups.reduce(function (a, g) {
        return a.concat(g.indices);
    }, [])
    readonly property var threadViewport: popup ? popup.threadViewport : null
    readonly property bool threadsCollapsed: parseList(preferences.collapsedSections).indexOf('threads') >= 0
    readonly property var tooltipHost: popup
    property int visualEpoch: 0
    property color yellow: light ? '#856117' : '#d7af68'

    signal newThreads(var keys)
    signal observedChange(var changes)

    function accountAlias(a) {
        return a.id === 'one' ? 'Gilfoyle' : 'Jared Dunn';
    }
    function accountKey(a) {
        return a.provider + ':' + a.id;
    }
    function alpha(c, a) {
        return Qt.rgba(c.r, c.g, c.b, a);
    }
    function openThread(i) {
    }
    function paceText(a) {
        return a.remaining.toFixed(1).replace(/\.0$/, '') + '% left · ' + a.timeRemaining.toFixed(1).replace(/\.0$/, '') + '% expected';
    }
    function parseList(v) {
        return JSON.parse(v || '[]');
    }
    function percentReading(n) {
        return n === null ? '—' : Math.round(n) + '%';
    }
    function stateColour(s) {
        return s === 'working' ? yellow : s === 'blocked' ? red : s === 'done' ? green : muted;
    }
    function toggleIdentity() {
        preferences = Object.assign({}, preferences, {
            namesHidden: !preferences.namesHidden
        });
    }
    function toggleList(key, value) {
        visualEpoch++;
        var list = (key === 'collapsedHosts' ? collapsedHosts : key === 'hiddenStates' ? hiddenStates : parseList(preferences[key])).slice();
        var i = list.indexOf(value);
        if (i < 0)
            list.push(value);
        else
            list.splice(i, 1);
        if (key === 'collapsedHosts')
            collapsedHosts = list;
        else if (key === 'hiddenStates')
            hiddenStates = list;
        else {
            var next = Object.assign({}, preferences);
            next[key] = JSON.stringify(list);
            preferences = next;
        }
    }
    function tokens(n) {
        return n === null ? '—' : n >= 1000000 ? (n / 1000000).toFixed(1) + 'M' : n >= 1000 ? (n / 1000).toFixed(1) + 'K' : String(n);
    }
}
