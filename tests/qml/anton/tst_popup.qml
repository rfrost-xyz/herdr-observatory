import QtQuick
import QtTest
import qs.Commons
import "../../../omarchy/herdr.observatory" as Anton
import "../../../omarchy/herdr.observatory/State.js" as State

Rectangle {
    id: scene

    border.color: Color.accent
    border.width: 1
    color: fixtureUi.light ? '#f0f3ed' : '#101c22'
    height: popup.height + 28
    width: 388

    FixtureUi {
        id: fixtureUi

        popup: popup
    }
    Anton.PopupContent {
        id: popup

        height: Math.min(540, implicitHeight)
        ui: fixtureUi
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
                sampled_at: fixtureUi.now / 1000,
                reset_count: 1,
                reset_expires_at: null,
                windows: [
                    {
                        kind: 'weekly',
                        label: 'Weekly',
                        used_percent: 100 - balance,
                        resets_at: fixtureUi.now / 1000 + 604800 * expected / 100,
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
                        seq: fixtureUi.now * 1000,
                        usage_seq: fixtureUi.now * 1000,
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
                        subagent_status_seq: fixtureUi.now * 1000
                    },
                    turn_timing: {
                        active: state === 'working',
                        started_at_s: fixtureUi.now / 1000 - 754,
                        observed_at_s: fixtureUi.now / 1000,
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
                sampled_at: fixtureUi.now / 1000,
                agents: agents
            };
        }
        function init() {
            fixtureUi.light = false;
            fixtureUi.raw = standard();
            fixtureUi.preferences = {
                namesHidden: true,
                collapsedSections: '[]',
                collapsedProviders: '[]'
            };
            fixtureUi.collapsedHosts = [];
            fixtureUi.hiddenStates = [];
            fixtureUi.focusedKey = '';
            fixtureUi.openedKeys = [];
            fixtureUi.navigationError = "";
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
            fixtureUi.light = true;
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
            fixtureUi.raw = {
                hosts: hosts,
                allowances: [account('one', 1, 30), account('two', 75, 62)]
            };
            popup.height = 390;
            wait(40);
            var heading = findChild(popup, 'anton-heading'), allowances = findChild(popup, 'allowances-section');
            var headerY = heading.mapToItem(scene, 0, 0).y, allowanceY = allowances.mapToItem(scene, 0, 0).y;
            verify(popup.threadViewport.interactive);
            fixtureUi.focusedKey = fixtureUi.threadKeys[fixtureUi.threadKeys.length - 1];
            wait(40);
            verify(popup.threadViewport.contentY > 0);
            compare(heading.mapToItem(scene, 0, 0).y, headerY);
            compare(allowances.mapToItem(scene, 0, 0).y, allowanceY);
            verify(allowanceY + 30 + popup.allowanceViewport.height <= popup.height + popup.y + 1);
            capture('many-short');
        }
        function test_04_instant_collapse() {
            var expanded = popup.implicitHeight;
            fixtureUi.toggleList('collapsedSections', 'threads');
            wait(0);
            compare(popup.threadViewport.height, 0);
            verify(popup.implicitHeight < expanded);
            fixtureUi.toggleList('collapsedSections', 'allowances');
            wait(0);
            compare(popup.allowanceViewport.height, 0);
        }
        function test_05_connection_states() {
            fixtureUi.raw = {
                hosts: [host('connecting', [], 'connecting'), host('unreachable', [], 'unreachable'), host('connected', [agent('a', 'idle', true)], 'connected')],
                allowances: [account('one', 0.2, 50)]
            };
            wait(40);
            capture('connections-missing');
        }
        function test_06_short_popup_reserves_both_scroll_regions() {
            var data = standard();
            data.allowances = [account('one', 30, 40), account('two', 40, 60), account('three', 75, 62), account('four', 20, 25)];
            fixtureUi.raw = data;
            popup.height = 240;
            wait(40);
            verify(popup.threadViewport.height >= 70);
            verify(popup.allowanceViewport.height > 0);
            verify(popup.threadViewport.interactive);
            verify(popup.allowanceViewport.interactive);
            var bottom = popup.allowanceViewport.mapToItem(popup, 0, popup.allowanceViewport.height).y;
            verify(bottom <= popup.height + 1);
            fixtureUi.navigationError = 'Synthetic navigation error that wraps to another line in this constrained popover.';
            wait(40);
            verify(popup.threadViewport.height > 0);
            verify(popup.allowanceViewport.height > 0);
            bottom = popup.allowanceViewport.mapToItem(popup, 0, popup.allowanceViewport.height).y;
            verify(bottom <= popup.height + 1);
            capture('short-with-notice');
        }
        function test_07_discovery_and_setup_are_compact_machine_states() {
            var data = standard();
            data.fleet_discovery = {state: 'unavailable'};
            data.hosts.push(host('new-profile', [], 'setup_needed'));
            fixtureUi.raw = data;
            wait(0);
            var discovery = findChild(popup, 'fleet-discovery-note');
            var setup = findChild(popup, 'machine-note-new-profile');
            verify(discovery.visible);
            compare(discovery.text, 'Discovery unavailable');
            compare(setup.text, 'Setup needed');
            compare(fixtureUi.overview.threads.length, 3);
            verify(discovery.x >= 100);
            verify(discovery.x + discovery.width <= popup.width + 1);
            capture('discovery-unavailable');
            var initial = popup.implicitHeight;
            fixtureUi.toggleList('collapsedHosts', 'laptop');
            wait(0);
            compare(fixtureUi.threadGroups[0].indices.length, 0);
            tryVerify(function () { return popup.implicitHeight < initial; }, 100);
            data = JSON.parse(JSON.stringify(data));
            data.hosts[0].label = 'Renamed laptop';
            data.fleet_discovery.state = 'available';
            fixtureUi.raw = data;
            wait(0);
            verify(!discovery.visible);
            verify(fixtureUi.threadGroups[0].collapsed);
            compare(fixtureUi.threadGroups[0].indices.length, 0);
            compare(popup.implicitHeight < initial, true);
            capture('discovery-setup');
        }

        function threadCards(item, found) {
            found = found || [];
            if (!item)
                return found;
            if (item.threadIndex !== undefined && item.keyed !== undefined && item.visible)
                found.push(item);
            for (var i = 0; i < item.children.length; i++)
                threadCards(item.children[i], found);
            return found;
        }
        function keyedThreads() {
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
            fixtureUi.raw = focusData(['a', 'b', 'c']);
            wait(0);
            fixtureUi.focusedKey = 'laptop:b';
            wait(0);
            compare(keyedThreads(), ['laptop:b']);
        }
        function test_08_focus_survives_earlier_thread_disappearing() {
            focusB();
            fixtureUi.raw = focusData(['b', 'c']);
            wait(0);
            compare(fixtureUi.focusedKey, 'laptop:b');
            compare(keyedThreads(), ['laptop:b']);
            compare(State.activationKey(fixtureUi.threadKeys, fixtureUi.focusedKey), 'laptop:b');
            var card = cardFor('laptop:b');
            verify(card !== null);
            mouseClick(card, 8, 8);
            compare(fixtureUi.openedKeys, ['laptop:b']);
        }
        function test_09_focus_clears_when_its_thread_disappears() {
            focusB();
            fixtureUi.raw = focusData(['a', 'c']);
            wait(0);
            compare(fixtureUi.focusedKey, '');
            compare(keyedThreads(), []);
            compare(State.activationKey(fixtureUi.threadKeys, fixtureUi.focusedKey), fixtureUi.threadKeys[0]);
            fixtureUi.raw = focusData(['a', 'b', 'c']);
            wait(0);
            compare(fixtureUi.focusedKey, '');
            compare(keyedThreads(), []);
        }
        function test_10_focus_clears_when_filtered() {
            focusB();
            // A later snapshot moves the focused thread into a hidden status.
            fixtureUi.hiddenStates = ['done'];
            var data = focusData(['a', 'b', 'c']);
            data.hosts[0].agents[1].status = 'done';
            fixtureUi.raw = data;
            wait(0);
            compare(fixtureUi.focusedKey, '');
            compare(keyedThreads(), []);
            fixtureUi.hiddenStates = [];
            fixtureUi.raw = focusData(['a', 'b', 'c']);
            focusB();
            fixtureUi.toggleList('hiddenStates', 'working');
            wait(0);
            compare(fixtureUi.focusedKey, '');
            fixtureUi.toggleList('hiddenStates', 'working');
            wait(0);
            compare(keyedThreads(), []);
        }
        function test_11_focus_clears_on_machine_and_section_collapse() {
            focusB();
            // Reconciliation alone clears focus, without the toggle reset.
            fixtureUi.collapsedHosts = ['laptop'];
            wait(0);
            compare(fixtureUi.focusedKey, '');
            fixtureUi.collapsedHosts = [];
            wait(0);
            verify(cardFor('laptop:b') !== null);
            compare(keyedThreads(), []);
            focusB();
            fixtureUi.toggleList('collapsedHosts', 'laptop');
            wait(0);
            compare(fixtureUi.focusedKey, '');
            fixtureUi.toggleList('collapsedHosts', 'laptop');
            wait(0);
            compare(keyedThreads(), []);
            focusB();
            fixtureUi.preferences = Object.assign({}, fixtureUi.preferences, {
                collapsedSections: '["threads"]'
            });
            wait(0);
            compare(fixtureUi.threadKeys.length, 0);
            compare(fixtureUi.focusedKey, '');
            fixtureUi.preferences = Object.assign({}, fixtureUi.preferences, {
                collapsedSections: '[]'
            });
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
                    resets_at: fixtureUi.now / 1000 + 15 * 86400,
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
            fixtureUi.raw = {
                hosts: [host('laptop', [agent('a', 'working', false)], 'connected')],
                allowances: [account('one', 73, 74), synthetic, locked]
            };
            wait(40);
            compare(fixtureUi.providers.map(function (group) {
                return group.label;
            }), ['Codex', 'Synthetic']);
            verify(findChild(popup, 'provider-codex') !== null);
            verify(findChild(popup, 'provider-synthetic') !== null);
            var cards = allowanceCards(popup);
            compare(cards.length, 3);
            compare(cards[1].entry.id, 'team');
            compare(findChild(cards[1], 'allowance-balance').text, '75%');
            compare(caption(cards[1]), '↻ 15d 0h');
            compare(cards[1].entry.paceDifference, 25);
            compare(cards[2].entry.id, 'office');
            compare(findChild(cards[2], 'allowance-balance').text, '—');
            verify(!findChild(cards[2], 'allowance-fill').visible);
            verify(!findChild(cards[2], 'allowance-pace-reading').visible);
            compare(cards[2].hint, 'Sign in required');
        }

        name: 'AntonPopup'
        when: windowShown
    }
}
