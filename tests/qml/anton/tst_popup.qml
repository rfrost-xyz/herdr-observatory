import QtQuick
import QtTest
import qs.Commons
import "../../../omarchy/herdr.observatory" as Anton

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
        function account(id, balance, expected) {
            return {
                provider: 'codex',
                account_id: id,
                label: id,
                available: true,
                sampled_at: fixtureUi.now / 1000,
                weekly_remaining: balance,
                weekly_resets_at: fixtureUi.now / 1000 + 604800 * expected / 100,
                reset_count: 1
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
            fixtureUi.focusedThread = -1;
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
            verify(popup.threadViewport.height > 0);
            verify(popup.height <= 540);
            verify(findChild(popup, "provider-notion") !== null);
            verify(popup.allowanceViewport.contentHeight <= popup.allowanceViewport.height + 1);
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
            fixtureUi.focusedThread = fixtureUi.threadOrder[fixtureUi.threadOrder.length - 1];
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

        name: 'AntonPopup'
        when: windowShown
    }
}
