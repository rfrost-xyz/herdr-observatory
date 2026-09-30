import QtQuick
import QtTest
import "../../../omarchy/herdr.observatory" as Anton
import "../../../omarchy/herdr.observatory/State.js" as State

Item {
    id: scene

    readonly property double now: 1800000000000
    readonly property string route: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"

    height: 10
    width: 10

    TestFiles {
        id: files
    }
    Component {
        id: preferencesComponent

        Anton.AntonPreferences {}
    }
    Component {
        id: controllerComponent

        Anton.AntonController {
            motionEnabled: true
            opened: true
            runtimePath: "/synthetic/anton-runtime"
        }
    }
    SignalSpy {
        id: closeSpy

        signalName: "closeRequested"
    }
    SignalSpy {
        id: changeSpy

        signalName: "observedChange"
    }
    SignalSpy {
        id: arrivalSpy

        signalName: "newThreads"
    }
    TestCase {
        property var controller: null
        property var preferences: null
        property string settingsPath: ""

        function agent(id, state, extra) {
            return Object.assign({
                id: id,
                status: state,
                project: "Project " + id,
                harness: "codex",
                technical: {
                    session_generation: 3,
                    state_change_seq: 7
                }
            }, extra || {});
        }
        function cleanup() {
            controller.destroy();
            preferences.destroy();
            wait(0);
        }
        function host(id, agents) {
            return {
                id: id,
                label: id,
                online: true,
                connection_state: "connected",
                sampled_at: scene.now / 1000,
                agents: agents
            };
        }
        function init() {
            settingsPath = files.path("privacy.ini");
            preferences = preferencesComponent.createObject(scene, {
                location: "file://" + settingsPath
            });
            controller = controllerComponent.createObject(scene, {
                preferences: preferences
            });
            closeSpy.target = controller;
            changeSpy.target = controller;
            arrivalSpy.target = controller;
            closeSpy.clear();
            changeSpy.clear();
            arrivalSpy.clear();
        }
        function show(hosts) {
            controller.view = State.project({
                hosts: hosts,
                allowances: []
            }, scene.now);
        }
        function standard() {
            show([host("laptop", [agent("a", "working"), agent("b", "done", {
                        navigation: {
                            profile_id: "desk",
                            route_key: scene.route
                        }
                    })]), host("workstation", [agent("c", "idle")])]);
        }
        function test_01_navigation_arguments() {
            standard();
            controller.openThread("laptop:a");
            compare(controller.launcher.command, ["/synthetic/anton-runtime", "--open-thread", "laptop", "a"]);
            verify(controller.launcher.running);
            controller.launcher.running = false;
            controller.openThread("laptop:b");
            compare(controller.launcher.command, ["/synthetic/anton-runtime", "--open-thread", "laptop", "b", JSON.stringify({
                    profile_id: "desk",
                    route_key: scene.route
                })]);
            compare(controller.navigationTarget, {
                key: "laptop:b",
                episode: "3:7:desk:" + scene.route,
                state: "done"
            });
        }
        function test_02_invalid_route_launches_nothing() {
            show([host("laptop", [agent("a", "working", {
                        navigation: {
                            profile_id: "desk",
                            route_key: "not-a-route"
                        }
                    })])]);
            controller.openThread("laptop:a");
            compare(controller.navigationError, "Invalid thread route. Try again shortly.");
            verify(!controller.launcher.running);
            compare(controller.launcher.command, []);
            // An unknown key does nothing at all.
            controller.navigationError = "";
            controller.openThread("laptop:missing");
            compare(controller.navigationError, "");
            verify(!controller.launcher.running);
        }
        function test_03_second_open_while_running_is_ignored() {
            standard();
            controller.openThread("laptop:a");
            controller.openThread("workstation:c");
            compare(controller.launcher.command, ["/synthetic/anton-runtime", "--open-thread", "laptop", "a"]);
            compare(controller.navigationTarget.key, "laptop:a");
        }
        function test_04_successful_open_acknowledges_a_done_thread() {
            standard();
            controller.navigationError = "Earlier error";
            controller.openThread("laptop:b");
            controller.launcher.running = false;
            controller.launcher.exited(0, 0);
            compare(closeSpy.count, 1);
            compare(controller.navigationError, "");
            var episode = "3:7:desk:" + scene.route;
            compare(preferences.acknowledgements, {
                "laptop:b": episode
            });
            verify(files.read(settingsPath).indexOf("laptop:b") >= 0, "Persisted");
            // An open of a thread that is not done acknowledges nothing.
            controller.openThread("laptop:a");
            controller.launcher.running = false;
            controller.launcher.exited(0, 0);
            compare(closeSpy.count, 2);
            compare(Object.keys(preferences.acknowledgements), ["laptop:b"]);
        }
        function test_05_thread_changed_before_exit_is_not_acknowledged() {
            standard();
            controller.openThread("laptop:b");
            show([host("laptop", [agent("a", "working"), agent("b", "working", {
                        navigation: {
                            profile_id: "desk",
                            route_key: scene.route
                        }
                    })])]);
            controller.launcher.running = false;
            controller.launcher.exited(0, 0);
            compare(closeSpy.count, 1);
            compare(preferences.acknowledgements, {});
        }
        function test_06_failed_open_reports_an_error() {
            standard();
            controller.openThread("laptop:a");
            controller.launcher.stderr.text = "";
            controller.launcher.stderr.streamFinished();
            controller.launcher.running = false;
            controller.launcher.exited(1, 0);
            compare(controller.navigationError, "Could not open this thread in Herdr.");
            compare(closeSpy.count, 0);
            controller.openThread("laptop:a");
            controller.launcher.stderr.text = "  Synthetic route failure\n";
            controller.launcher.stderr.streamFinished();
            controller.launcher.running = false;
            controller.launcher.exited(2, 0);
            compare(controller.navigationError, "Synthetic route failure");
        }
        function test_07_acknowledgements_invalidate_and_bar_state() {
            standard();
            compare(controller.barState, "working");
            show([host("laptop", [agent("b", "done", {
                        navigation: {
                            profile_id: "desk",
                            route_key: scene.route
                        }
                    })])]);
            compare(controller.barState, "done");
            controller.openThread("laptop:b");
            controller.launcher.running = false;
            controller.launcher.exited(0, 0);
            compare(controller.barState, "idle", "An acknowledged completion no longer marks the bar");
            // A new status episode of the same thread removes the acknowledgement.
            var next = agent("b", "done", {
                navigation: {
                    profile_id: "desk",
                    route_key: scene.route
                }
            });
            next.technical.state_change_seq = 8;
            show([host("laptop", [next])]);
            compare(preferences.acknowledgements, {});
            compare(controller.barState, "done");
            verify(files.read(settingsPath).indexOf("laptop:b") < 0, "Removal persisted");
            // A thread that stopped being done also loses its acknowledgement;
            // an absent thread keeps its entry.
            preferences.setAcknowledgements({
                "laptop:b": "3:8:desk:" + scene.route,
                "elsewhere:z": "1:1"
            });
            show([host("laptop", [agent("b", "working", {
                        navigation: {
                            profile_id: "desk",
                            route_key: scene.route
                        }
                    })])]);
            compare(preferences.acknowledgements, {
                "elsewhere:z": "1:1"
            });
        }
        function test_08_focus_movement_activation_and_reconciliation() {
            standard();
            compare(controller.threadKeys, ["laptop:a", "laptop:b", "workstation:c"]);
            controller.moveFocus(1);
            compare(controller.focusedKey, "laptop:a");
            controller.moveFocus(1);
            compare(controller.focusedKey, "laptop:b");
            controller.moveFocus(5);
            compare(controller.focusedKey, "workstation:c");
            controller.moveFocus(-1);
            compare(controller.focusedKey, "laptop:b");
            controller.activate();
            compare(controller.launcher.command.slice(1, 4), ["--open-thread", "laptop", "b"]);
            controller.launcher.running = false;
            // The focused thread leaves the view: focus clears; activation takes the first.
            show([host("laptop", [agent("a", "working")]), host("workstation", [agent("c", "idle")])]);
            compare(controller.focusedKey, "");
            controller.activate();
            compare(controller.launcher.command.slice(1, 4), ["--open-thread", "laptop", "a"]);
            controller.launcher.running = false;
            // Filtering the focused thread's state clears focus by reconciliation.
            standard();
            controller.focusedKey = "workstation:c";
            preferences.settings.hiddenStates = '["idle"]';
            compare(controller.threadKeys, ["laptop:a", "laptop:b"]);
            compare(controller.focusedKey, "");
        }
        function test_09_toggle_list_clears_focus_and_bumps_the_epoch() {
            standard();
            controller.focusedKey = "laptop:a";
            var epoch = controller.visualEpoch;
            controller.toggleList("collapsedHosts", "workstation");
            compare(controller.visualEpoch, epoch + 1);
            compare(controller.focusedKey, "");
            compare(preferences.collapsedHosts, ["workstation"]);
            compare(controller.threadGroups[1].collapsed, true);
            controller.focusedKey = "laptop:a";
            controller.toggleList("collapsedSections", "threads");
            compare(controller.threadKeys, []);
            compare(controller.focusedKey, "");
            controller.toggleList("collapsedSections", "threads");
            compare(controller.threadKeys, ["laptop:a", "laptop:b"]);
            // Opening and closing also bump the epoch; opening clears focus.
            controller.focusedKey = "laptop:a";
            controller.opened = false;
            compare(controller.visualEpoch, epoch + 4);
            compare(controller.focusedKey, "laptop:a");
            controller.opened = true;
            compare(controller.visualEpoch, epoch + 5);
            compare(controller.focusedKey, "");
        }
        function test_10_emission_gating() {
            standard();
            wait(0);
            changeSpy.clear();
            arrivalSpy.clear();
            // A new thread on a reporting host arrives while open with motion.
            show([host("laptop", [agent("a", "blocked"), agent("b", "done", {
                        navigation: {
                            profile_id: "desk",
                            route_key: scene.route
                        }
                    }), agent("d", "working")]), host("workstation", [agent("c", "idle")])]);
            compare(arrivalSpy.count, 0, "Emission is deferred");
            tryCompare(arrivalSpy, "count", 1);
            compare(changeSpy.count, 1);
            compare(arrivalSpy.signalArguments[0][0], {
                "laptop:d": true
            });
            compare(changeSpy.signalArguments[0][0]["laptop:a"].state, true);
            // Closed, reduced motion or an epoch change before delivery: nothing.
            controller.opened = false;
            standard();
            wait(10);
            compare(arrivalSpy.count, 1);
            controller.opened = true;
            controller.motionEnabled = false;
            show([host("laptop", [agent("a", "working"), agent("e", "working")])]);
            wait(10);
            compare(arrivalSpy.count, 1);
            controller.motionEnabled = true;
            show([host("laptop", [agent("a", "working"), agent("e", "working"), agent("f", "working")])]);
            controller.visualEpoch++;
            wait(10);
            compare(arrivalSpy.count, 1);
            compare(changeSpy.count, 1);
        }
        function test_11_identity_toggle_and_aliases() {
            controller.view = State.project({
                hosts: [],
                allowances: [
                    {
                        provider: "codex",
                        account_id: "one",
                        label: "one",
                        status: "unavailable"
                    },
                    {
                        provider: "codex",
                        account_id: "two",
                        label: "two",
                        status: "unavailable"
                    }
                ]
            }, scene.now);
            controller.random = function () {
                return 0;
            };
            compare(preferences.namesHidden, false);
            controller.toggleIdentity();
            compare(preferences.namesHidden, true);
            var expected = State.assignAliases(controller.view.allowances, controller.aliasPool, function () {
                return 0;
            });
            compare(preferences.accountAliases, expected);
            compare(controller.accountAlias(controller.view.allowances[0]), expected["codex:one"]);
            controller.toggleIdentity();
            compare(preferences.namesHidden, false);
            compare(preferences.accountAliases, expected, "Showing names keeps the saved aliases");
        }
        function test_12_refresh_is_a_request() {
            var spy = Qt.createQmlObject('import QtTest; SignalSpy { signalName: "refreshRequested" }', scene);
            spy.target = controller;
            controller.refresh();
            compare(spy.count, 1);
            spy.destroy();
        }

        name: 'AntonController'
        when: windowShown
    }
}
