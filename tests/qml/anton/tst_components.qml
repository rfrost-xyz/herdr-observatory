import QtQuick
import QtTest
import "../../../omarchy/herdr.observatory/State.js" as State
import "../../../omarchy/herdr.observatory" as Anton

Item {
    id: scene

    height: 220
    width: 360

    Flickable {
        id: flick

        anchors.fill: parent
        contentHeight: 220
    }
    QtObject {
        id: fakeUi

        property var accountEmails: ({})
        property color blue: '#abcdef'
        property var contentItem: scene
        property string face: 'monospace'
        property int focusedThread: -1
        property color green: '#90ad65'
        property color ink: '#d7d6cd'
        property color line: '#333333'
        property bool motionEnabled: true
        property color muted: '#777777'
        property bool opened: true
        property var overview: ({
                threads: [
                    {
                        id: 'a',
                        hostId: 'h',
                        project: 'Example',
                        state: 'working',
                        harness: 'codex',
                        host: 'h',
                        usage: {
                            contextPercent: 40,
                            inputTokens: 10000,
                            outputTokens: 100,
                            uncachedTokens: 500,
                            cachePercent: 95,
                            compactions: 1,
                            age: '1s ago'
                        },
                        completion: {
                            done: 1,
                            total: 3,
                            outcomes: {
                                running: 1,
                                failed: 1,
                                interrupted: 0,
                                unknown: 0
                            },
                            stale: false,
                            age: '1s ago'
                        }
                    }
                ]
            })
        property var preferences: ({
                namesHidden: false
            })
        property color red: '#ed6050'
        property var tooltipHost: flick
        property int visualEpoch: 0
        property color yellow: '#e9b95c'

        signal newThreads(var keys)
        signal observedChange(var changes)

        property int setupOpened: 0
        function openNotionSetup() { setupOpened++; }
        function accountAlias(a) {
            return 'Gilfoyle';
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
            return '40% left · 50% expected';
        }
        function percentReading(n) {
            return n + '%';
        }
        function stateColour(s) {
            return s === 'working' ? '#cca555' : '#888888';
        }
        function toggleIdentity() {
        }
        function tokens(n) {
            return String(n);
        }
    }
    Anton.ThreadCard {
        id: thread

        threadIndex: 0
        ui: fakeUi
        width: 360
    }
    Anton.AllowanceCard {
        id: allowance

        entry: ({
                id: 'test',
                provider: 'codex',
                label: 'Example',
                remaining: 40,
                timeRemaining: 50,
                pace: 'deficit',
                paceDifference: -10,
                paceStrength: 0.66,
                reset: '2d 1h',
                resetCount: 1
            })
        ui: fakeUi
        width: 360
        y: 100
    }
    TestCase {
        function test_00_notion_setup() {
            var saved = allowance.entry;
            allowance.entry = State.project({hosts: [], allowances: []}, Date.now()).allowances[0];
            compare(findChild(allowance, "allowance-balance").text, "Set up");
            verify(allowance.hint.indexOf("Connect your Notion browser session") >= 0);
            verify(!findChild(allowance, "allowance-expected-tick").visible);
            allowance.activate();
            compare(fakeUi.setupOpened, 1);
            allowance.entry = saved;
        }
        function test_00_notion_monthly() {
            var saved = allowance.entry;
            var now = new Date(2026, 9, 1).getTime();
            var reset = new Date(2026, 9, 27).getTime() / 1000;
            allowance.entry = State.project({hosts: [], allowances: [{account_id: "notion-monthly", provider: "notion", label: "Work", available: true, window_seconds: 0, sampled_at: now / 1000, monthly_used_percent: 1.61, monthly_resets_at: reset}]}, now).allowances[0];
            fakeUi.accountEmails = {Work: "codex@example.invalid"};
            compare(findChild(allowance, "allowance-identity").email, "");
            verify(allowance.Accessible.name.indexOf("codex@example.invalid") < 0);
            compare(findChild(allowance, "allowance-balance").text, "2% used");
            verify(allowance.hint.indexOf("27 Oct 2026") >= 0);
            verify(!findChild(allowance, "allowance-expected-tick").visible);
            fakeUi.accountEmails = {};
            allowance.entry = saved;
        }
        function test_01_initial_static() {
            compare(thread.entrance, 1);
            compare(thread.flash, 0);
            compare(thread.contextFlash, 0);
            verify(thread.height > 40);
            compare(allowance.hint, '40% left · 50% expected');
        }
        function test_02_arrival_duration() {
            fakeUi.newThreads({
                'h:a': true
            });
            verify(thread.entrance < 0.2);
            wait(340);
            compare(thread.entrance, 1);
        }
        function test_03_collapse_is_instant() {
            fakeUi.newThreads({
                'h:a': true
            });
            fakeUi.visualEpoch++;
            compare(thread.entrance, 1);
        }
        function test_04_compaction_event() {
            fakeUi.observedChange({
                'h:a': {
                    compaction: true
                }
            });
            verify(thread.contextFlash > 0);
            wait(500);
            compare(thread.contextFlash, 0);
        }
        function test_05_closed_cancels() {
            fakeUi.newThreads({
                'h:a': true
            });
            fakeUi.opened = false;
            compare(thread.entrance, 1);
            compare(thread.flash, 0);
            fakeUi.opened = true;
            compare(thread.entrance, 1);
        }
        function test_06_reduced_motion_cancels() {
            fakeUi.newThreads({
                'h:a': true
            });
            fakeUi.motionEnabled = false;
            compare(thread.entrance, 1);
            fakeUi.newThreads({
                "h:a": true
            });
            compare(thread.entrance, 1);
            fakeUi.motionEnabled = true;
        }
        function test_07_balance_stays_independent_of_pace() {
            var balance = findChild(allowance, "allowance-balance");
            verify(balance !== null);
            compare(balance.color.toString(), fakeUi.ink.toString());
            var initial = allowance.paceColour.toString();
            allowance.entry = Object.assign({}, allowance.entry, {
                pace: "reserve",
                paceDifference: 10
            });
            compare(allowance.entry.pace, "reserve");
            compare(allowance.entry.remaining, 40);
            wait(220);
            verify(allowance.paceColour.toString() !== initial);
            compare(balance.color.toString(), fakeUi.ink.toString());
        }
        function test_08_closed_colour_changes_are_immediate() {
            fakeUi.opened = false;
            allowance.entry = Object.assign({}, allowance.entry, {
                pace: "deficit",
                paceDifference: -10
            });
            compare(allowance.paceColour, fakeUi.red);
            fakeUi.opened = true;
            fakeUi.motionEnabled = false;
            allowance.entry = Object.assign({}, allowance.entry, {
                pace: "reserve",
                paceDifference: 10
            });
            compare(allowance.paceColour, fakeUi.green);
            fakeUi.motionEnabled = true;
        }
        function test_09_reduced_motion_finishes_active_colour_change() {
            fakeUi.opened = true;
            fakeUi.motionEnabled = true;
            allowance.entry = Object.assign({}, allowance.entry, {
                pace: "deficit",
                paceDifference: -10
            });
            wait(40);
            var target = fakeUi.red;
            verify(allowance.paceColour.toString() !== target.toString());
            fakeUi.motionEnabled = false;
            compare(allowance.paceColour.toString(), target.toString());
            // The binding still follows later telemetry without motion.
            allowance.entry = Object.assign({}, allowance.entry, {
                pace: "reserve",
                paceDifference: 10
            });
            target = fakeUi.green;
            compare(allowance.paceColour.toString(), target.toString());
            fakeUi.motionEnabled = true;
        }
        function test_10_closing_finishes_active_colour_change() {
            allowance.entry = Object.assign({}, allowance.entry, {
                pace: "deficit",
                paceDifference: -10
            });
            wait(40);
            var target = fakeUi.red;
            verify(allowance.paceColour.toString() !== target.toString());
            fakeUi.opened = false;
            compare(allowance.paceColour.toString(), target.toString());
            fakeUi.opened = true;
            compare(allowance.paceColour.toString(), target.toString());
        }
        function test_11_separate_strip_geometry_and_thresholds() {
            fakeUi.motionEnabled = false;
            var fill = findChild(allowance, "allowance-fill");
            var region = findChild(allowance, "allowance-pace-region");
            var strip = findChild(allowance, "allowance-pace-strip");
            var halo = findChild(allowance, "allowance-pace-halo");
            var hatch = findChild(allowance, "allowance-deficit-hatch");
            var sparks = findChild(allowance, "allowance-pace-sparks");
            var balanceColour = fill.color.toString();
            var cases = [
                {
                    remaining: 70,
                    difference: 20,
                    colour: fakeUi.green
                },
                {
                    remaining: 50,
                    difference: 0,
                    colour: fakeUi.green
                },
                {
                    remaining: 49.9,
                    difference: -0.1,
                    colour: Qt.tint(fakeUi.muted, fakeUi.alpha(fakeUi.yellow, 0.65))
                },
                {
                    remaining: 45,
                    difference: -5,
                    colour: Qt.tint(fakeUi.muted, fakeUi.alpha(fakeUi.yellow, 0.65))
                },
                {
                    remaining: 44.9,
                    difference: -5.1,
                    colour: fakeUi.yellow
                },
                {
                    remaining: 40.1,
                    difference: -9.9,
                    colour: fakeUi.yellow
                },
                {
                    remaining: 40,
                    difference: -10,
                    colour: fakeUi.red
                },
                {
                    remaining: 0,
                    difference: -50,
                    colour: fakeUi.red
                },
                {
                    remaining: 100,
                    difference: 50,
                    colour: fakeUi.green
                }
            ];
            for (var i = 0; i < cases.length; i++) {
                var c = cases[i];
                allowance.entry = Object.assign({}, allowance.entry, {
                    remaining: c.remaining,
                    timeRemaining: 50,
                    paceDifference: c.difference
                });
                compare(allowance.paceColour.toString(), c.colour.toString());
                compare(strip.color.toString(), fakeUi.green.toString());
                compare(region.visible, c.difference > 0);
                compare(hatch.visible, c.difference < 0);
                fuzzyCompare(hatch.width, allowance.width * Math.max(0, -c.difference) / 100, 0.001);
                fuzzyCompare(hatch.x, fill.width, 0.001);
                compare(hatch.height, fill.height);
                compare(fill.color.toString(), balanceColour);
                fuzzyCompare(fill.width, allowance.width * c.remaining / 100, 0.001);
                fuzzyCompare(region.width, allowance.width * Math.abs(c.difference) / 100, 0.001);
                fuzzyCompare(region.x, allowance.width * Math.min(c.remaining, 50) / 100, 0.001);
            }
            compare(fill.height, 6);
            compare(strip.height, 2);
            verify(region.y > fill.height);
            verify(region.clip);
            compare(halo.parent, region);
            compare(sparks.parent, region);
            compare(sparks.tint.toString(), strip.color.toString());
            allowance.entry = Object.assign({}, allowance.entry, {
                paceDifference: null,
                timeRemaining: null
            });
            verify(!region.visible);
            verify(!hatch.visible);
            verify(!findChild(allowance, "allowance-expected-tick").visible);
            compare(fill.color.toString(), balanceColour);
            allowance.entry = Object.assign({}, allowance.entry, {
                remaining: null
            });
            verify(!fill.visible);
            compare(findChild(allowance, "allowance-balance").text, "—");
            fakeUi.motionEnabled = true;
        }
        function test_12_positive_only_hover_and_signed_reading() {
            fakeUi.opened = true;
            fakeUi.motionEnabled = false;
            allowance.entry = Object.assign({}, allowance.entry, {
                remaining: 60,
                timeRemaining: 50,
                paceDifference: 10
            });
            mouseMove(allowance, 20, 10);
            tryCompare(allowance, "hovered", true);
            var sparks = findChild(allowance, "allowance-pace-sparks");
            var halo = findChild(allowance, "allowance-pace-halo");
            var reading = findChild(allowance, "allowance-pace-reading");
            compare(reading.text, "+10.0%");
            verify(halo.opacity > 0);
            verify(!sparks.visible);
            fakeUi.motionEnabled = true;
            tryCompare(sparks, "visible", true);
            compare(sparks.tint.toString(), fakeUi.green.toString());
            allowance.entry = Object.assign({}, allowance.entry, {
                remaining: 40,
                paceDifference: -10
            });
            compare(reading.text, "−10.0%");
            verify(!sparks.active);
            verify(!sparks.visible);
            compare(halo.opacity, 0);
            verify(findChild(allowance, "allowance-deficit-hatch").visible);
            allowance.entry = Object.assign({}, allowance.entry, {
                remaining: 49.99,
                paceDifference: -0.01
            });
            compare(reading.text, "0.0%");
            compare(reading.color.toString(), fakeUi.muted.toString());
            verify(!sparks.visible);
            compare(halo.opacity, 0);
            allowance.entry = Object.assign({}, allowance.entry, {
                remaining: 60,
                paceDifference: 10
            });
            tryCompare(sparks, "visible", true);
            fakeUi.opened = false;
            verify(!sparks.visible);
            compare(halo.opacity, 0);
            fakeUi.opened = true;
            allowance.width = 220;
            var identity = findChild(allowance, "allowance-identity");
            var numbers = findChild(allowance, "allowance-numbers");
            verify(identity.width >= 0);
            verify(identity.x + identity.width < numbers.x);
            allowance.entry = Object.assign({}, allowance.entry, {
                remaining: null,
                timeRemaining: null,
                paceDifference: null
            });
            verify(!reading.visible);
            verify(!sparks.visible);
            compare(halo.opacity, 0);
            allowance.width = 360;
            mouseMove(scene, 359, 219);
        }
        function test_13_failure_notch_requires_verified_failure() {
            var dial = findChild(thread, "subagent-dial");
            verify(dial !== null);
            compare(dial.failureNotch, true);
            var entry = fakeUi.overview.threads[0];
            fakeUi.overview = {
                threads: [Object.assign({}, entry, {
                        completion: {
                            done: 1,
                            total: 3,
                            outcomes: {
                                running: 1,
                                interrupted: 1,
                                failed: 0,
                                unknown: 0
                            },
                            stale: false,
                            age: '1s ago'
                        }
                    })]
            };
            compare(dial.failureNotch, false);
            fakeUi.overview = {
                threads: [Object.assign({}, entry, {
                        completion: {
                            done: 1,
                            total: 3,
                            outcomes: null,
                            stale: false,
                            age: '1s ago'
                        }
                    })]
            };
            compare(dial.failureNotch, false);
        }
        function test_14_turn_timer_receives_real_hover() {
            var entry = fakeUi.overview.threads[0];
            fakeUi.overview = {
                threads: [Object.assign({}, entry, {
                        timing: {
                            active: true,
                            elapsed: 75,
                            total: 400,
                            stale: false,
                            age: '1s ago'
                        }
                    })]
            };
            var clock = findChild(thread, 'thread-turn-clock');
            verify(clock !== null);
            wait(50);
            verify(clock.visible);
            verify(clock.width > 0);
            verify(clock.height > 0, 'Clock height: ' + clock.height);
            mouseMove(clock, clock.width / 2, clock.height / 2);
            tryCompare(clock, 'hovered', true, 1200);
            tryVerify(function () {
                return clock.color.a > 0;
            }, 500);
            var tooltip = null;
            for (var i = 0; i < clock.data.length; i++) {
                var candidate = clock.data[i];
                if (candidate && candidate.text === clock.hint && candidate.delay === 450)
                    tooltip = candidate;
            }
            verify(tooltip !== null, 'Production timer tooltip found');
            compare(tooltip.text, 'Wall-clock turn 1m 15s\nTotal turn time 6m 40s');
            tryCompare(tooltip, 'opened', true, 1200);
            verify(thread.tooltipSuppressed);
            mouseMove(scene, 2, scene.height - 2);
        }

        name: 'AntonComponents'
        when: windowShown
    }
}
