import QtQuick
import QtTest
import qs.Commons
import "../../../omarchy/herdr.observatory" as Anton
import "../../../omarchy/herdr.observatory/State.js" as State

// The real popover parts with the former fixture colours: a fixed theme, a
// controller whose launcher is the Process stub and preferences in the run's
// private temporary directory.
Rectangle {
    id: scene

    // The display instant; tests advance it without touching the view.
    property double displayNow: now
    property bool light: false
    readonly property double now: 1800000000000
    property var raw: ({
            hosts: [],
            allowances: []
        })

    border.color: Color.accent
    border.width: 1
    color: light ? '#f0f3ed' : '#101c22'
    height: popup.height + 28
    width: 388

    TestFiles {
        id: files
    }
    Anton.AntonTheme {
        id: fixtureTheme

        blue: '#739fae'
        face: 'monospace'
        green: scene.light ? '#397332' : '#9dc473'
        ink: scene.light ? '#243030' : '#d7d6cd'
        line: scene.light ? '#d1d9d7' : '#28383c'
        muted: scene.light ? '#637173' : '#8d9497'
        red: scene.light ? '#aa3e34' : '#ed7968'
        yellow: scene.light ? '#856117' : '#d7af68'
    }
    Anton.AntonPreferences {
        id: fixturePreferences

        location: "file://" + files.directory + "/popup-privacy.ini"
    }
    Anton.AntonController {
        id: fixtureController

        motionEnabled: false
        opened: true
        preferences: fixturePreferences
        runtimePath: '/synthetic/anton-runtime'
        view: State.project(scene.raw, scene.now)
    }
    Anton.PopupContent {
        id: popup

        controller: fixtureController
        height: Math.min(540, implicitHeight)
        now: scene.displayNow
        preferences: fixturePreferences
        theme: fixtureTheme
        view: fixtureController.view
        width: 360
        x: 14
        y: 14
    }
    TestCase {
        // A D1 snapshot row. The runtime names the provider; the view does not.
        function account(id, balance, expected) {
            return {
                provider: 'codex',
                provider_label: 'Codex',
                account_id: id,
                label: id,
                status: 'available',
                status_text: null,
                plan: null,
                sampled_at: scene.now / 1000,
                reset_count: 1,
                reset_expires_at: null,
                windows: [
                    {
                        kind: 'weekly',
                        label: 'Weekly',
                        used_percent: 100 - balance,
                        resets_at: scene.now / 1000 + 604800 * expected / 100,
                        duration_s: 604800,
                        pacing: true
                    }
                ]
            };
        }
        function agent(id, state, missing) {
            return {
                id: id,
                status: state,
                project: id === 'a' ? 'Example Project With A Long Name' : 'Project ' + id,
                branch: id === 'a' ? 'feature/very-long-worktree-branch-name' : 'feature/' + id,
                harness: 'codex',
                technical: missing ? {} : {
                    telemetry: {
                        seq: scene.now * 1000,
                        usage_seq: scene.now * 1000,
                        context: 42,
                        window: 100,
                        context_percent: 42,
                        total_input: 2100000,
                        total_output: 34000,
                        total_cache_read: 1950000,
                        total_uncached_input: 150000,
                        compactions: 2,
                        subagent_total: 4,
                        subagent_done: 2,
                        subagent_running: 1,
                        subagent_interrupted: 0,
                        subagent_failed: 1,
                        subagent_unknown: 0,
                        subagent_status_seq: scene.now * 1000
                    },
                    turn_timing: {
                        active: state === 'working',
                        started_at_s: scene.now / 1000 - 754,
                        observed_at_s: scene.now / 1000,
                        last_duration_s: 420,
                        last_outcome: 'completed',
                        total_finished_duration_s: 2100,
                        complete: true,
                        freshness_seconds: 12
                    }
                }
            };
        }
        function capture(name) {
            // Settle pending layout first: positioners lay out on the next
            // frame, and the popover height follows their implicit heights.
            waitForRendering(scene, 100);
            waitForRendering(scene, 100);
            var saved = false;
            scene.grabToImage(function (result) {
                saved = result.saveToFile('/tmp/anton-continuity-' + name + '.png');
            });
            tryVerify(function () {
                return saved;
            }, 2000);
        }
        function host(id, agents, state) {
            return {
                id: id,
                label: id,
                online: state === 'connected',
                connection_state: state,
                sampled_at: scene.now / 1000,
                agents: agents
            };
        }
        function init() {
            scene.light = false;
            scene.displayNow = scene.now;
            scene.raw = standard();
            var settings = fixturePreferences.settings;
            settings.namesHidden = true;
            settings.collapsedSections = '[]';
            settings.collapsedProviders = '[]';
            settings.collapsedHosts = '[]';
            settings.hiddenStates = '[]';
            // The former fixture's aliases: Gilfoyle for "one", Jared Dunn otherwise.
            settings.accountAliases = JSON.stringify({
                'codex:one': 'Gilfoyle',
                'codex:two': 'Jared Dunn',
                'codex:three': 'Jared Dunn',
                'codex:four': 'Jared Dunn',
                'synthetic:team': 'Jared Dunn',
                'synthetic:office': 'Jared Dunn'
            });
            fixtureController.focusedKey = '';
            fixtureController.launcher.running = false;
            fixtureController.launcher.command = [];
            fixtureController.navigationError = "";
            popup.view = Qt.binding(function () {
                return fixtureController.view;
            });
            popup.height = Qt.binding(function () {
                return Math.min(540, popup.implicitHeight);
            });
            wait(40);
        }
        function standard() {
            return {
                hosts: [host('laptop', [agent('a', 'working', false), agent('b', 'done', false)], 'connected'), host('workstation', [agent('c', 'idle', true)], 'connected')],
                allowances: [account('one', 73, 74), account('two', 1, 62)]
            };
        }
        function test_01_compact_three_threads() {
            verify(popup.threadViewport.contentHeight <= popup.threadViewport.height + 1);
            verify(popup.implicitHeight <= 540);
            capture('dark');
        }
        function test_02_light_and_missing() {
            scene.light = true;
            Color.accent = '#527e77';
            wait(40);
            capture('light');
            Color.accent = '#74b9b0';
        }
        function test_03_many_threads_fixed_header_and_allowances() {
            var hosts = [];
            for (var h = 0; h < 4; h++) {
                var agents = [];
                for (var a = 0; a < 5; a++)
                    agents.push(agent(h + '-' + a, a % 2 ? 'idle' : 'working', a === 3));
                hosts.push(host('machine-' + h, agents, 'connected'));
            }
            hosts.push(host('sleeping', [], 'connecting'));
            hosts.push(host('offline', [], 'unreachable'));
            scene.raw = {
                hosts: hosts,
                allowances: [account('one', 1, 30), account('two', 75, 62)]
            };
            popup.height = 390;
            wait(40);
            var heading = findChild(popup, 'anton-heading'), allowances = findChild(popup, 'allowances-section');
            var headerY = heading.mapToItem(scene, 0, 0).y, allowanceY = allowances.mapToItem(scene, 0, 0).y;
            verify(popup.threadViewport.interactive);
            fixtureController.focusedKey = fixtureController.threadKeys[fixtureController.threadKeys.length - 1];
            wait(40);
            verify(popup.threadViewport.contentY > 0);
            compare(heading.mapToItem(scene, 0, 0).y, headerY);
            compare(allowances.mapToItem(scene, 0, 0).y, allowanceY);
            verify(allowanceY + 30 + popup.allowanceViewport.height <= popup.height + popup.y + 1);
            capture('many-short');
        }
        function test_04_instant_collapse() {
            var expanded = popup.implicitHeight;
            fixtureController.toggleList('collapsedSections', 'threads');
            wait(0);
            compare(popup.threadViewport.height, 0);
            verify(popup.implicitHeight < expanded);
            fixtureController.toggleList('collapsedSections', 'allowances');
            wait(0);
            compare(popup.allowanceViewport.height, 0);
        }
        function test_05_connection_states() {
            scene.raw = {
                hosts: [host('connecting', [], 'connecting'), host('unreachable', [], 'unreachable'), host('connected', [agent('a', 'idle', true)], 'connected')],
                allowances: [account('one', 0.2, 50)]
            };
            wait(40);
            capture('connections-missing');
        }
        function test_06_short_popup_reserves_both_scroll_regions() {
            var data = standard();
            data.allowances = [account('one', 30, 40), account('two', 40, 60), account('three', 75, 62), account('four', 20, 25)];
            scene.raw = data;
            popup.height = 240;
            wait(40);
            verify(popup.threadViewport.height >= 70);
            verify(popup.allowanceViewport.height > 0);
            verify(popup.threadViewport.interactive);
            verify(popup.allowanceViewport.interactive);
            var bottom = popup.allowanceViewport.mapToItem(popup, 0, popup.allowanceViewport.height).y;
            verify(bottom <= popup.height + 1);
            fixtureController.navigationError = 'Synthetic navigation error that wraps to another line in this constrained popover.';
            wait(40);
            verify(popup.threadViewport.height > 0);
            verify(popup.allowanceViewport.height > 0);
            bottom = popup.allowanceViewport.mapToItem(popup, 0, popup.allowanceViewport.height).y;
            verify(bottom <= popup.height + 1);
            capture('short-with-notice');
        }
        function test_07_discovery_and_setup_are_compact_machine_states() {
            var data = standard();
            data.fleet_discovery = {
                state: 'unavailable'
            };
            data.hosts.push(host('new-profile', [], 'setup_needed'));
            scene.raw = data;
            wait(0);
            var discovery = findChild(popup, 'fleet-discovery-note');
            var setup = findChild(popup, 'machine-note-new-profile');
            verify(discovery.visible);
            compare(discovery.text, 'Discovery unavailable');
            compare(setup.text, 'Setup needed');
            compare(fixtureController.view.threads.length, 3);
            verify(discovery.x >= 100);
            verify(discovery.x + discovery.width <= popup.width + 1);
            capture('discovery-unavailable');
            var initial = popup.implicitHeight;
            fixtureController.toggleList('collapsedHosts', 'laptop');
            wait(0);
            compare(fixtureController.threadGroups[0].indices.length, 0);
            tryVerify(function () {
                return popup.implicitHeight < initial;
            }, 100);
            data = JSON.parse(JSON.stringify(data));
            data.hosts[0].label = 'Renamed laptop';
            data.fleet_discovery.state = 'available';
            scene.raw = data;
            wait(0);
            verify(!discovery.visible);
            verify(fixtureController.threadGroups[0].collapsed);
            compare(fixtureController.threadGroups[0].indices.length, 0);
            compare(popup.implicitHeight < initial, true);
            capture('discovery-setup');
        }

        function threadCards(item, found) {
            found = found || [];
            if (!item)
                return found;
            if (item.focused !== undefined && item.keyed !== undefined && item.entry !== undefined && item.visible)
                found.push(item);
            for (var i = 0; i < item.children.length; i++)
                threadCards(item.children[i], found);
            return found;
        }
        function keyedThreads() {
            verify(threadCards(popup).length > 0, 'Thread cards found');
            return threadCards(popup).filter(function (card) {
                return card.keyed;
            }).map(function (card) {
                return card.entry.hostId + ':' + card.entry.id;
            });
        }
        function focusData(ids) {
            var laptop = [], workstation = [];
            ids.forEach(function (id) {
                (id === 'c' ? workstation : laptop).push(agent(id, 'working', true));
            });
            return {
                hosts: [host('laptop', laptop, 'connected'), host('workstation', workstation, 'connected')],
                allowances: []
            };
        }
        function cardFor(key) {
            return threadCards(popup).filter(function (card) {
                return card.entry.hostId + ':' + card.entry.id === key;
            })[0] || null;
        }
        function focusB() {
            scene.raw = focusData(['a', 'b', 'c']);
            wait(0);
            fixtureController.focusedKey = 'laptop:b';
            wait(0);
            compare(keyedThreads(), ['laptop:b']);
        }
        function test_08_focus_survives_earlier_thread_disappearing() {
            focusB();
            scene.raw = focusData(['b', 'c']);
            wait(0);
            compare(fixtureController.focusedKey, 'laptop:b');
            compare(keyedThreads(), ['laptop:b']);
            compare(State.activationKey(fixtureController.threadKeys, fixtureController.focusedKey), 'laptop:b');
            var card = cardFor('laptop:b');
            verify(card !== null);
            mouseClick(card, 8, 8);
            compare(fixtureController.launcher.command, ['/synthetic/anton-runtime', '--open-thread', 'laptop', 'b']);
        }
        function test_08b_tap_after_closing_launches_nothing() {
            focusB();
            var card = cardFor('laptop:b');
            verify(card !== null);
            fixtureController.opened = false;
            mouseClick(card, 8, 8);
            fixtureController.opened = true;
            verify(!fixtureController.launcher.running);
            compare(fixtureController.launcher.command, []);
            mouseClick(card, 8, 8);
            compare(fixtureController.launcher.command, ['/synthetic/anton-runtime', '--open-thread', 'laptop', 'b']);
        }
        function test_09_focus_clears_when_its_thread_disappears() {
            focusB();
            scene.raw = focusData(['a', 'c']);
            wait(0);
            compare(fixtureController.focusedKey, '');
            compare(keyedThreads(), []);
            compare(State.activationKey(fixtureController.threadKeys, fixtureController.focusedKey), fixtureController.threadKeys[0]);
            scene.raw = focusData(['a', 'b', 'c']);
            wait(0);
            compare(fixtureController.focusedKey, '');
            compare(keyedThreads(), []);
        }
        function test_10_focus_clears_when_filtered() {
            focusB();
            // A later snapshot moves the focused thread into a hidden status.
            fixturePreferences.settings.hiddenStates = '["done"]';
            var data = focusData(['a', 'b', 'c']);
            data.hosts[0].agents[1].status = 'done';
            scene.raw = data;
            wait(0);
            compare(fixtureController.focusedKey, '');
            compare(keyedThreads(), []);
            fixturePreferences.settings.hiddenStates = '[]';
            scene.raw = focusData(['a', 'b', 'c']);
            focusB();
            fixtureController.toggleList('hiddenStates', 'working');
            wait(0);
            compare(fixtureController.focusedKey, '');
            fixtureController.toggleList('hiddenStates', 'working');
            wait(0);
            compare(keyedThreads(), []);
        }
        function test_11_focus_clears_on_machine_and_section_collapse() {
            focusB();
            // Reconciliation alone clears focus, without the toggle reset.
            fixturePreferences.settings.collapsedHosts = '["laptop"]';
            wait(0);
            compare(fixtureController.focusedKey, '');
            fixturePreferences.settings.collapsedHosts = '[]';
            wait(0);
            verify(cardFor('laptop:b') !== null);
            compare(keyedThreads(), []);
            focusB();
            fixtureController.toggleList('collapsedHosts', 'laptop');
            wait(0);
            compare(fixtureController.focusedKey, '');
            fixtureController.toggleList('collapsedHosts', 'laptop');
            wait(0);
            compare(keyedThreads(), []);
            focusB();
            fixturePreferences.settings.collapsedSections = '["threads"]';
            wait(0);
            compare(fixtureController.threadKeys.length, 0);
            compare(fixtureController.focusedKey, '');
            fixturePreferences.settings.collapsedSections = '[]';
            wait(0);
            verify(cardFor('laptop:b') !== null);
            compare(keyedThreads(), []);
        }

        function allowanceCards(item, found) {
            found = found || [];
            if (!item)
                return found;
            if (item.entry !== undefined && item.paceReading !== undefined && item.visible)
                found.push(item);
            for (var i = 0; i < item.children.length; i++)
                allowanceCards(item.children[i], found);
            return found;
        }
        function caption(item) {
            if (!item)
                return '';
            if (typeof item.text === 'string' && item.text.indexOf('↻ ') === 0)
                return item.text;
            for (var i = 0; i < item.children.length; i++) {
                var text = caption(item.children[i]);
                if (text)
                    return text;
            }
            return '';
        }
        // Any provider renders through the same cards with no provider code.
        function test_12_provider_neutral_rows_render_generically() {
            var synthetic = account('team', 75, 50);
            synthetic.provider = 'synthetic';
            synthetic.provider_label = 'Synthetic';
            synthetic.windows = [
                {
                    kind: 'monthly',
                    label: 'Monthly',
                    used_percent: 25,
                    resets_at: scene.now / 1000 + 15 * 86400,
                    duration_s: 30 * 86400,
                    pacing: true
                }
            ];
            var locked = account('office', 0, 0);
            locked.provider = 'synthetic';
            locked.provider_label = 'Synthetic';
            locked.status = 'auth_needed';
            locked.status_text = 'Sign in required';
            locked.sampled_at = null;
            locked.reset_count = null;
            locked.windows = [];
            scene.raw = {
                hosts: [host('laptop', [agent('a', 'working', false)], 'connected')],
                allowances: [account('one', 73, 74), synthetic, locked]
            };
            wait(40);
            compare(fixtureController.providers.map(function (group) {
                return group.label;
            }), ['Codex', 'Synthetic']);
            verify(findChild(popup, 'provider-codex') !== null);
            verify(findChild(popup, 'provider-synthetic') !== null);
            var cards = allowanceCards(popup);
            compare(cards.length, 3);
            compare(cards[1].entry.id, 'team');
            compare(findChild(cards[1], 'allowance-balance').text, '75%');
            compare(caption(cards[1]), '↻ 15d 0h');
            compare(cards[1].reading.paceDifference, 25);
            compare(cards[2].entry.id, 'office');
            compare(findChild(cards[2], 'allowance-balance').text, '—');
            verify(!findChild(cards[2], 'allowance-fill').visible);
            verify(!findChild(cards[2], 'allowance-pace-reading').visible);
            compare(cards[2].hint, 'Sign in required');
        }

        // Keyed delegates (D9). Motion is enabled for these scenarios only.
        function motionData(states) {
            var laptop = [];
            Object.keys(states).forEach(function (id) {
                laptop.push(agent(id, states[id], true));
            });
            return {
                hosts: [host('laptop', laptop, 'connected'), host('workstation', [agent('c', 'idle', true)], 'connected')],
                allowances: []
            };
        }
        function withMotion(body) {
            fixtureController.motionEnabled = true;
            try {
                body();
            } finally {
                fixtureController.motionEnabled = false;
            }
        }
        function entrances() {
            return threadCards(popup).map(function (card) {
                return card.entrance;
            });
        }
        function test_13_earlier_thread_disappears_during_a_highlight() {
            withMotion(function () {
                scene.raw = motionData({
                    a: 'working',
                    b: 'working'
                });
                wait(0);
                var card = cardFor('laptop:b');
                verify(card !== null);
                scene.raw = motionData({
                    a: 'working',
                    b: 'blocked'
                });
                tryVerify(function () {
                    return card.flash > 0;
                }, 500, 'The state highlight starts');
                compare(cardFor('laptop:b'), card);
                // The earlier thread disappears while the highlight runs.
                scene.raw = motionData({
                    b: 'blocked'
                });
                wait(0);
                verify(cardFor('laptop:a') === null);
                compare(cardFor('laptop:b'), card, 'The same delegate keeps the thread');
                var level = card.flash;
                verify(level > 0, 'The highlight is still running');
                tryVerify(function () {
                    return card.flash < level;
                }, 500, 'and still animating');
                threadCards(popup).forEach(function (other) {
                    if (other !== card)
                        compare(other.flash, 0, 'No other card starts or inherits the highlight');
                });
                tryCompare(card, 'flash', 0, 1500);
            });
        }
        function test_14_reorder_keeps_delegates() {
            scene.raw = motionData({
                a: 'working',
                b: 'idle',
                d: 'done'
            });
            wait(0);
            var a = cardFor('laptop:a'), b = cardFor('laptop:b'), d = cardFor('laptop:d');
            // A new status order sorts d first and a last.
            scene.raw = motionData({
                a: 'done',
                b: 'blocked',
                d: 'working'
            });
            wait(0);
            compare(fixtureController.threadGroups[0].keys, ['laptop:d', 'laptop:b', 'laptop:a']);
            compare(cardFor('laptop:a'), a);
            compare(cardFor('laptop:b'), b);
            compare(cardFor('laptop:d'), d);
        }
        function test_15_new_thread_enters_but_hydration_filter_and_collapse_do_not() {
            withMotion(function () {
                // Hydration: the first snapshot after none is not an arrival.
                scene.raw = {
                    hosts: [],
                    allowances: []
                };
                wait(0);
                scene.raw = motionData({
                    a: 'working'
                });
                wait(20);
                compare(entrances(), [1, 1]);
                // A genuine arrival on a reporting host enters (positive control).
                scene.raw = motionData({
                    a: 'working',
                    e: 'working'
                });
                tryVerify(function () {
                    var card = cardFor('laptop:e');
                    return card !== null && card.entrance < 1;
                }, 500);
                tryCompare(cardFor('laptop:e'), 'entrance', 1, 1000);
                // Row reappears after filtering: full opacity, no entrance.
                fixtureController.toggleList('hiddenStates', 'working');
                wait(0);
                verify(cardFor('laptop:e') === null);
                fixtureController.toggleList('hiddenStates', 'working');
                wait(20);
                compare(cardFor('laptop:e').entrance, 1);
                compare(entrances(), [1, 1, 1]);
                // Machine collapse and expand.
                fixtureController.toggleList('collapsedHosts', 'laptop');
                wait(0);
                fixtureController.toggleList('collapsedHosts', 'laptop');
                wait(20);
                compare(entrances(), [1, 1, 1]);
                // Section collapse and expand.
                fixtureController.toggleList('collapsedSections', 'threads');
                wait(0);
                compare(threadCards(popup).length, 0);
                fixtureController.toggleList('collapsedSections', 'threads');
                wait(20);
                compare(entrances(), [1, 1, 1]);
                // Reconnect: a host that stopped reporting and returns is hydration.
                var offline = motionData({
                    a: 'working',
                    e: 'working'
                });
                offline.hosts[0].online = false;
                offline.hosts[0].connection_state = 'unreachable';
                scene.raw = offline;
                wait(0);
                scene.raw = motionData({
                    a: 'working',
                    e: 'working'
                });
                wait(20);
                compare(entrances(), [1, 1, 1]);
                // Sorting: a state change reorders rows without an entrance.
                scene.raw = motionData({
                    a: 'idle',
                    e: 'working'
                });
                wait(20);
                compare(entrances(), [1, 1, 1]);
            });
        }

        // Rows come from the view the groups were computed from, whatever order
        // the two view bindings are notified in.
        function test_15b_cards_follow_the_controller_view() {
            scene.raw = motionData({
                a: 'working',
                b: 'idle'
            });
            wait(0);
            popup.view = fixtureController.view;
            scene.raw = motionData({
                b: 'working',
                d: 'blocked'
            });
            wait(0);
            compare(threadCards(popup).map(function (card) {
                return card.entry.hostId + ':' + card.entry.id;
            }), ['laptop:b', 'laptop:d', 'workstation:c']);
        }
        // Time separation (3.7): readings follow the display instant only.
        function clockText(key) {
            var clock = findChild(cardFor(key), 'thread-turn-clock');
            verify(clock !== null);
            return durationText(clock);
        }
        function durationText(item) {
            if (typeof item.text === 'string' && /^[0-9]/.test(item.text))
                return item.text;
            for (var i = 0; i < item.children.length; i++) {
                var text = durationText(item.children[i]);
                if (text)
                    return text;
            }
            return '';
        }
        function test_16_fresh_stopwatch_advances_without_a_view_change() {
            var view = fixtureController.view;
            compare(clockText('laptop:a'), '12m 34s');
            scene.displayNow = scene.now + 5000;
            compare(fixtureController.view, view, 'The view is not rebuilt');
            compare(clockText('laptop:a'), '12m 39s');
            var card = cardFor('laptop:a');
            verify(card.hint.indexOf('Reported 5s ago') >= 0, card.hint);
        }
        function test_17_stale_stopwatch_is_frozen() {
            var data = standard();
            // Observed 30 s ago with 12 s freshness: stale and frozen at 12m 04s.
            data.hosts[0].agents[0].technical.turn_timing.observed_at_s = scene.now / 1000 - 30;
            scene.raw = data;
            wait(0);
            compare(clockText('laptop:a'), '12m 04s');
            scene.displayNow = scene.now + 5000;
            compare(clockText('laptop:a'), '12m 04s');
        }
        function test_18_reset_caption_changes_at_the_hour_boundary() {
            var row = account('one', 73, 74);
            row.windows[0].resets_at = scene.now / 1000 + 3602;
            scene.raw = {
                hosts: [],
                allowances: [row]
            };
            wait(0);
            var card = allowanceCards(popup)[0];
            compare(caption(card), '↻ 0d 1h');
            var view = fixtureController.view;
            scene.displayNow = scene.now + 2000;
            compare(caption(card), '↻ 0d 1h');
            scene.displayNow = scene.now + 3000;
            compare(caption(card), '↻ 0d <1h');
            compare(fixtureController.view, view);
        }

        name: 'AntonPopup'
        when: windowShown
    }
}
