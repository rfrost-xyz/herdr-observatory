import QtQuick
import QtTest
import "../../../omarchy/herdr.observatory" as Anton

Item {
    id: scene

    height: 10
    width: 10

    Anton.SnapshotStore {
        id: store
    }
    TestCase {
        function collector() {
            for (var i = 0; i < store.data.length; i++) {
                if (store.data[i] && store.data[i].antonStub === true)
                    return store.data[i];
            }
            return null;
        }
        function init() {
            store.visualUpdates = true;
            var process = collector();
            process.running = true;
            process.writes = [];
        }
        function snapshot(heartbeat) {
            var raw = {
                at: Date.now() / 1000,
                interval: 5,
                hosts: [],
                allowances: [],
                fleet_discovery: {
                    state: 'disabled'
                }
            };
            if (heartbeat !== undefined)
                raw.heartbeat_seconds = heartbeat;
            return JSON.stringify(raw);
        }
        function test_01_stub_process_is_in_use() {
            var process = collector();
            verify(process !== null, 'Stub Quickshell.Io Process found');
            verify(process.stdinEnabled);
            verify(process.stdout !== null);
        }
        function test_02_refresh_writes_one_command_to_a_running_collector() {
            store.refresh();
            compare(collector().writes, ['refresh\n']);
            compare(collector().running, true);
        }
        function test_03_refresh_starts_a_stopped_collector_without_writing() {
            var process = collector();
            process.running = false;
            store.refresh();
            compare(process.running, true);
            compare(process.writes, []);
        }
        function test_04_restart_never_writes() {
            var process = collector();
            store.restart();
            compare(process.writes, []);
            compare(process.running, true);
            process.running = false;
            store.restart();
            compare(process.running, true);
            compare(process.writes, []);
        }
        function test_05_snapshot_drops_only_after_derived_timeout() {
            // One-second heartbeat: timeout is (1 + 1 loop wait + 1 margin) s.
            var received = Date.now();
            collector().stdout.read(snapshot(1));
            verify(store.raw !== null);
            verify(store.view.connected);
            wait(2200);
            verify(store.raw !== null, 'Still current before the derived timeout');
            verify(store.view.connected);
            tryVerify(function () {
                return store.raw === null;
            }, 4000);
            verify(Date.now() - received >= 3000, 'Dropped after ' + (Date.now() - received) + ' ms');
            verify(!store.view.connected);
        }
        function test_06_malformed_line_clears_the_snapshot() {
            collector().stdout.read(snapshot(4));
            verify(store.view.connected);
            collector().stdout.read('{not json');
            compare(store.raw, null);
            verify(!store.view.connected);
        }
        function test_07_now_advances_only_while_visual_updates() {
            store.visualUpdates = true;
            var start = store.now;
            verify(start > 0);
            tryVerify(function () {
                return store.now > start;
            }, 2500, 'now advances on the tick while visual updates run');
            store.visualUpdates = false;
            var frozen = store.now;
            wait(1300);
            collector().stdout.read(snapshot(4));
            compare(store.now, frozen, 'now is unchanged by ticks and receipts while closed');
            wait(5);
            store.visualUpdates = true;
            verify(store.now > frozen, 'opening sets now at once');
        }
        function hostSnapshot(sampledAt, heartbeat) {
            return JSON.stringify({
                at: Date.now() / 1000,
                interval: 5,
                heartbeat_seconds: heartbeat,
                hosts: [{
                        id: 'alpha',
                        label: 'Alpha',
                        online: true,
                        connection_state: 'connected',
                        sampled_at: sampledAt,
                        agents: []
                    }],
                allowances: [],
                fleet_discovery: {
                    state: 'disabled'
                }
            });
        }
        function test_08_unchanged_snapshot_keeps_the_view_while_now_advances() {
            store.visualUpdates = true;
            var line = hostSnapshot(Date.now() / 1000, 4);
            collector().stdout.read(line);
            verify(store.view.hosts[0].reporting);
            var view = store.view, start = store.now, replaced = 0, ticks = 0;
            var watch = function () {
                replaced++;
            };
            var tick = function () {
                ticks++;
            };
            store.viewChanged.connect(watch);
            store.nowChanged.connect(tick);
            wait(1100);
            collector().stdout.read(line);
            tryVerify(function () {
                return ticks >= 4;
            }, 5000, 'three ticks and the re-sent receipt advance now');
            store.viewChanged.disconnect(watch);
            store.nowChanged.disconnect(tick);
            verify(store.view === view, 'Same view object over three ticks and a re-sent snapshot');
            compare(replaced, 0);
            verify(store.now > start + 2000, 'now advanced by ' + (store.now - start) + ' ms');
        }
        function test_09_closed_host_stops_reporting_at_max_age_without_a_receipt() {
            // interval 5 gives maxAge 25 s; the sample reaches it 0.5 s after receipt.
            store.visualUpdates = false;
            var sampled = Date.now() / 1000 - 24.5;
            collector().stdout.read(hostSnapshot(sampled, 60));
            verify(store.view.hosts[0].reporting);
            tryVerify(function () {
                return !store.view.hosts[0].reporting;
            }, 3000);
            var late = Date.now() - (sampled + 25) * 1000;
            verify(late <= 1500, 'Stopped reporting ' + late + ' ms after max age');
            verify(store.raw !== null, 'The 60 s heartbeat keeps the snapshot');
            store.visualUpdates = true;
        }
        function test_10_collector_exit_drops_the_snapshot_and_restarts() {
            var process = collector();
            collector().stdout.read(hostSnapshot(Date.now() / 1000, 4));
            verify(store.view.connected);
            var receipt = store.lastReceipt;
            process.running = false;
            process.exited(1, 0);
            compare(store.raw, null);
            verify(!store.view.connected);
            compare(store.lastReceipt, receipt, 'An exit is not a receipt');
            compare(process.running, false);
            tryVerify(function () {
                return process.running;
            }, 7000, 'The retry restarts the collector');
            compare(process.writes, [], 'The restart never writes');
            collector().stdout.read(hostSnapshot(Date.now() / 1000, 4));
            verify(store.view.connected, 'The next line reconnects');
        }

        name: 'AntonStore'
        when: windowShown
    }
}
