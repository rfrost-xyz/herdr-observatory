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

        name: 'AntonStore'
        when: windowShown
    }
}
