import QtQuick
import QtTest
import "../../../omarchy/herdr.observatory" as Anton
import "../../../omarchy/herdr.observatory/State.js" as State

Item {
    id: scene

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
                        stale: false,
                        at: scene.now - 1000
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
                        stamp: (scene.now - 1000) * 1000
                    }
                }
            ]
        })

    readonly property double now: 1800000000000

    height: 220
    width: 360

    Flickable {
        id: flick

        anchors.fill: parent
        contentHeight: 220
    }
    TestFiles {
        id: files
    }
    Anton.AntonTheme {
        id: fakeTheme

        blue: '#abcdef'
        face: 'monospace'
        green: '#90ad65'
        ink: '#d7d6cd'
        line: '#333333'
        muted: '#777777'
        red: '#ed6050'
        yellow: '#e9b95c'
    }
    Anton.AntonPreferences {
        id: fakePreferences

        location: "file://" + files.directory + "/components-privacy.ini"
    }
    Anton.AntonController {
        id: fakeController

        motionEnabled: true
        opened: true
        preferences: fakePreferences
        runtimePath: '/synthetic/anton-runtime'
        view: ({
                threads: scene.overview.threads,
                hosts: [],
                allowances: []
            })
    }
    Anton.AntonToolTip {
        id: sharedTip

        host: flick
        moving: flick.moving
        theme: fakeTheme
    }
    Anton.ThreadCard {
        id: thread

        contentItem: scene
        controller: fakeController
        entry: scene.overview.threads[0] || {}
        focused: fakeController.focusedKey !== '' && fakeController.focusedKey === State.threadKey(entry)
        now: scene.now
        theme: fakeTheme
        tooltip: sharedTip
        viewport: flick
        width: 360
    }
    Anton.AllowanceCard {
        id: allowance

        entry: ({
                id: 'test',
                provider: 'codex',
                label: 'Example',
                remaining: 40,
                // Structural: 50 s of a 100 s window left at scene.now, so the
                // expected balance is 50% and the pace -10.
                resetAt: scene.now / 1000 + 50,
                durationS: 100,
                sampledAt: scene.now / 1000,
                resetCount: 1
            })
        aliasName: 'Gilfoyle'
        motionEnabled: fakeController.motionEnabled
        namesHidden: false
        now: scene.now
        opened: fakeController.opened
        theme: fakeTheme
        tooltip: sharedTip
        width: 360
        y: 100
    }
    TestCase {
        // Applies changes to the allowance entry. Expected-balance and pace
        // changes (the former presentation fields) become the structural reset
        // time of a 100 s window at scene.now, so the reading has them.
        function paced(changes) {
            var entry = Object.assign({}, allowance.entry, changes), expected = allowance.reading.timeRemaining;
            if (changes.timeRemaining !== undefined)
                expected = changes.timeRemaining;
            else if (changes.paceDifference !== undefined)
                expected = changes.paceDifference === null || entry.remaining === null ? null : entry.remaining - changes.paceDifference;
            delete entry.timeRemaining;
            delete entry.paceDifference;
            entry.resetAt = expected === null ? null : scene.now / 1000 + expected;
            entry.durationS = expected === null ? null : 100;
            entry.sampledAt = scene.now / 1000;
            return entry;
        }
        function test_01_initial_static() {
            compare(thread.entrance, 1);
            compare(thread.flash, 0);
            compare(thread.contextFlash, 0);
            verify(thread.height > 40);
            compare(allowance.hint, '40% left · 50% expected');
        }
        function test_02_arrival_duration() {
            fakeController.newThreads({
                'h:a': true
            });
            verify(thread.entrance < 0.2);
            wait(340);
            compare(thread.entrance, 1);
        }
        function test_03_collapse_is_instant() {
            fakeController.newThreads({
                'h:a': true
            });
            fakeController.visualEpoch++;
            compare(thread.entrance, 1);
        }
        function test_04_compaction_event() {
            fakeController.observedChange({
                'h:a': {
                    compaction: true
                }
            });
            verify(thread.contextFlash > 0);
            wait(500);
            compare(thread.contextFlash, 0);
        }
        function test_05_closed_cancels() {
            fakeController.newThreads({
                'h:a': true
            });
            fakeController.opened = false;
            compare(thread.entrance, 1);
            compare(thread.flash, 0);
            fakeController.opened = true;
            compare(thread.entrance, 1);
        }
        function test_06_reduced_motion_cancels() {
            fakeController.newThreads({
                'h:a': true
            });
            fakeController.motionEnabled = false;
            compare(thread.entrance, 1);
            fakeController.newThreads({
                "h:a": true
            });
            compare(thread.entrance, 1);
            fakeController.motionEnabled = true;
        }
        function test_07_balance_stays_independent_of_pace() {
            var balance = findChild(allowance, "allowance-balance");
            verify(balance !== null);
            compare(balance.color.toString(), fakeTheme.ink.toString());
            var initial = allowance.paceColour.toString();
            allowance.entry = paced({
                paceDifference: 10
            });
            compare(allowance.reading.paceDifference, 10);
            compare(allowance.entry.remaining, 40);
            wait(220);
            verify(allowance.paceColour.toString() !== initial);
            compare(balance.color.toString(), fakeTheme.ink.toString());
        }
        function test_08_closed_colour_changes_are_immediate() {
            fakeController.opened = false;
            allowance.entry = paced({
                paceDifference: -10
            });
            compare(allowance.paceColour, fakeTheme.red);
            fakeController.opened = true;
            fakeController.motionEnabled = false;
            allowance.entry = paced({
                paceDifference: 10
            });
            compare(allowance.paceColour, fakeTheme.green);
            fakeController.motionEnabled = true;
        }
        function test_09_reduced_motion_finishes_active_colour_change() {
            fakeController.opened = true;
            fakeController.motionEnabled = true;
            allowance.entry = paced({
                paceDifference: -10
            });
            wait(40);
            var target = fakeTheme.red;
            verify(allowance.paceColour.toString() !== target.toString());
            fakeController.motionEnabled = false;
            compare(allowance.paceColour.toString(), target.toString());
            // The binding still follows later telemetry without motion.
            allowance.entry = paced({
                paceDifference: 10
            });
            target = fakeTheme.green;
            compare(allowance.paceColour.toString(), target.toString());
            fakeController.motionEnabled = true;
        }
        function test_10_closing_finishes_active_colour_change() {
            allowance.entry = paced({
                paceDifference: -10
            });
            wait(40);
            var target = fakeTheme.red;
            verify(allowance.paceColour.toString() !== target.toString());
            fakeController.opened = false;
            compare(allowance.paceColour.toString(), target.toString());
            fakeController.opened = true;
            compare(allowance.paceColour.toString(), target.toString());
        }
        function test_11_separate_strip_geometry_and_thresholds() {
            fakeController.motionEnabled = false;
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
                    colour: fakeTheme.green
                },
                {
                    remaining: 50,
                    difference: 0,
                    colour: fakeTheme.green
                },
                {
                    remaining: 49.9,
                    difference: -0.1,
                    colour: Qt.tint(fakeTheme.muted, fakeTheme.alpha(fakeTheme.yellow, 0.65))
                },
                {
                    remaining: 45,
                    difference: -5,
                    colour: Qt.tint(fakeTheme.muted, fakeTheme.alpha(fakeTheme.yellow, 0.65))
                },
                {
                    remaining: 44.9,
                    difference: -5.1,
                    colour: fakeTheme.yellow
                },
                {
                    remaining: 40.1,
                    difference: -9.9,
                    colour: fakeTheme.yellow
                },
                {
                    remaining: 40,
                    difference: -10,
                    colour: fakeTheme.red
                },
                {
                    remaining: 0,
                    difference: -50,
                    colour: fakeTheme.red
                },
                {
                    remaining: 100,
                    difference: 50,
                    colour: fakeTheme.green
                }
            ];
            for (var i = 0; i < cases.length; i++) {
                var c = cases[i];
                allowance.entry = paced({
                    remaining: c.remaining,
                    timeRemaining: 50,
                    paceDifference: c.difference
                });
                compare(allowance.paceColour.toString(), c.colour.toString());
                compare(strip.color.toString(), fakeTheme.green.toString());
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
            allowance.entry = paced({
                paceDifference: null,
                timeRemaining: null
            });
            verify(!region.visible);
            verify(!hatch.visible);
            verify(!findChild(allowance, "allowance-expected-tick").visible);
            compare(fill.color.toString(), balanceColour);
            allowance.entry = paced({
                remaining: null
            });
            verify(!fill.visible);
            compare(findChild(allowance, "allowance-balance").text, "—");
            fakeController.motionEnabled = true;
        }
        function test_12_positive_only_hover_and_signed_reading() {
            fakeController.opened = true;
            fakeController.motionEnabled = false;
            allowance.entry = paced({
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
            fakeController.motionEnabled = true;
            tryCompare(sparks, "visible", true);
            compare(sparks.tint.toString(), fakeTheme.green.toString());
            allowance.entry = paced({
                remaining: 40,
                paceDifference: -10
            });
            compare(reading.text, "−10.0%");
            verify(!sparks.active);
            verify(!sparks.visible);
            compare(halo.opacity, 0);
            verify(findChild(allowance, "allowance-deficit-hatch").visible);
            allowance.entry = paced({
                remaining: 49.99,
                paceDifference: -0.01
            });
            compare(reading.text, "0.0%");
            compare(reading.color.toString(), fakeTheme.muted.toString());
            verify(!sparks.visible);
            compare(halo.opacity, 0);
            allowance.entry = paced({
                remaining: 60,
                paceDifference: 10
            });
            tryCompare(sparks, "visible", true);
            fakeController.opened = false;
            verify(!sparks.visible);
            compare(halo.opacity, 0);
            fakeController.opened = true;
            allowance.width = 220;
            var identity = findChild(allowance, "allowance-identity");
            var numbers = findChild(allowance, "allowance-numbers");
            verify(identity.width >= 0);
            verify(identity.x + identity.width < numbers.x);
            allowance.entry = paced({
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
            var entry = scene.overview.threads[0];
            scene.overview = {
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
                            stamp: (scene.now - 1000) * 1000
                        }
                    })]
            };
            compare(dial.failureNotch, false);
            scene.overview = {
                threads: [Object.assign({}, entry, {
                        completion: {
                            done: 1,
                            total: 3,
                            outcomes: null,
                            stale: false,
                            stamp: (scene.now - 1000) * 1000
                        }
                    })]
            };
            compare(dial.failureNotch, false);
        }
        function test_14_turn_timer_receives_real_hover() {
            var entry = scene.overview.threads[0];
            scene.overview = {
                threads: [Object.assign({}, entry, {
                        // Started 75 s before scene.now; 325 s of finished turns.
                        timing: {
                            active: true,
                            settled: false,
                            startedAt: scene.now / 1000 - 75,
                            observedAt: scene.now / 1000 - 1,
                            last: null,
                            complete: true,
                            finishedTotal: 325,
                            outcome: null,
                            stale: false
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
            // The popover's single shared tooltip shows the hovered clock's hint.
            var tooltip = sharedTip;
            compare(tooltip.source, clock, 'Production timer tooltip found');
            compare(tooltip.delay, 450);
            compare(tooltip.text, 'Wall-clock turn 1m 15s\nTotal turn time 6m 40s');
            tryCompare(tooltip, 'opened', true, 1200);
            verify(thread.tooltipSuppressed);
            mouseMove(scene, 2, scene.height - 2);
        }
        function test_15_thread_card_focus_and_activation_use_its_key() {
            fakeController.focusedKey = '';
            verify(!thread.keyed);
            fakeController.focusedKey = 'h:b';
            verify(!thread.keyed);
            fakeController.focusedKey = 'h:a';
            verify(thread.keyed);
            fakeController.focusedKey = '';
            fakeController.launcher.running = false;
            fakeController.launcher.command = [];
            mouseClick(thread, 8, 8);
            compare(fakeController.launcher.command, ['/synthetic/anton-runtime', '--open-thread', 'h', 'a']);
        }
        // Source status text reaches the hint and accessible name; the card
        // itself gains no visible text and invents no balance or pace.
        function test_16_auth_needed_uses_source_status_text() {
            allowance.entry = {
                id: 'team',
                provider: 'synthetic',
                providerLabel: 'Synthetic',
                label: 'Team',
                status: 'auth_needed',
                statusText: 'Sign in required',
                remaining: null,
                resetCount: null,
                resetAt: null,
                durationS: null,
                sampledAt: null
            };
            compare(allowance.hint, 'Sign in required');
            compare(allowance.Accessible.name, 'Unknown account. Sign in required');
            compare(findChild(allowance, 'allowance-balance').text, '—');
            verify(!findChild(allowance, 'allowance-fill').visible);
            verify(!findChild(allowance, 'allowance-pace-reading').visible);
            verify(!findChild(allowance, 'allowance-expected-tick').visible);
            verify(!findChild(allowance, 'allowance-deficit-hatch').visible);
        }
        function test_17_unavailable_without_text_keeps_existing_hint() {
            allowance.entry = paced({
                status: 'unavailable',
                statusText: null
            });
            compare(allowance.hint, 'Allowance unavailable');
            compare(allowance.Accessible.name, 'Unknown account. Allowance unavailable');
        }

        name: 'AntonComponents'
        when: windowShown
    }
}
