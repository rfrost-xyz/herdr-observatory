import QtQuick
import QtTest
import "../../../omarchy/herdr.observatory" as Anton

// The host sits inside a larger window, so the window margins that every Popup
// keeps never interfere with the host clamping under test.
Item {
    height: 240
    width: 340

    Item {
        id: scene

        height: 200
        width: 300
        x: 20
        y: 20

        Anton.AntonTheme {
            id: fixtureTheme

            face: 'monospace'
        }
        Anton.AntonToolTip {
            id: tip

            host: scene
            theme: fixtureTheme
        }
        Anton.AntonSurface {
            id: first

            height: 40
            hint: 'First hint'
            tooltip: tip
            theme: fixtureTheme
            width: 100
        }
        Anton.AntonSurface {
            id: second

            height: 40
            hint: 'Second hint with enough words to wrap across more than one line of the shared tooltip'
            tooltip: tip
            theme: fixtureTheme
            width: 100
            x: 100
        }
        Anton.AntonSurface {
            id: corner

            height: 40
            hint: 'Corner'
            tooltip: tip
            theme: fixtureTheme
            width: 60
            x: 240
            y: 160
        }
        TestCase {
            function cleanup() {
                mouseMove(scene, 150, 120);
                tip.moving = false;
                first.tooltipSuppressed = false;
                tryCompare(tip, 'source', null, 500);
                tryCompare(tip, 'opened', false, 500);
            }
            function test_01_delay_and_source() {
                compare(tip.delay, 450);
                compare(tip.timeout, 6000);
                compare(tip.margins, 8);
                compare(tip.padding, 7);
                var started = Date.now();
                mouseMove(first, 20, 10);
                compare(tip.source, first);
                compare(tip.text, 'First hint');
                wait(300);
                verify(!tip.opened, 'Still inside the delay');
                tryCompare(tip, 'opened', true, 1000);
                verify(Date.now() - started >= 440, 'Opened after ' + (Date.now() - started) + ' ms');
                mouseMove(scene, 150, 120);
                compare(tip.source, null);
                tryCompare(tip, 'opened', false, 500);
            }
            function test_02_switching_source_restarts_the_delay() {
                mouseMove(first, 20, 10);
                tryCompare(tip, 'opened', true, 1000);
                var switched = Date.now();
                mouseMove(second, 20, 10);
                compare(tip.source, second);
                compare(tip.text, second.hint);
                verify(!tip.opened, 'Closed on the switch');
                wait(300);
                verify(!tip.opened, 'Delay restarted for the new source');
                tryCompare(tip, 'opened', true, 1000);
                verify(Date.now() - switched >= 440);
            }
            function test_03_hidden_while_moving_and_suppressed() {
                tip.moving = true;
                mouseMove(first, 20, 10);
                compare(tip.source, first);
                wait(600);
                verify(!tip.opened, 'No hint while the viewport moves');
                tip.moving = false;
                tryCompare(tip, 'opened', true, 1000);
                first.tooltipSuppressed = true;
                compare(tip.source, null);
                tryCompare(tip, 'opened', false, 500);
            }
            function test_04_follows_the_pointer_and_clamps_to_the_host() {
                mouseMove(first, 20, 10);
                tryCompare(tip, 'opened', true, 1000);
                compare(tip.x, 20 + 12);
                compare(tip.y, 10 + 18);
                mouseMove(second, 30, 12);
                tryCompare(tip, 'opened', true, 1000);
                verify(tip.width <= 280);
                verify(tip.width <= scene.width);
                compare(tip.x, Math.min(130 + 12, scene.width - tip.width));
                mouseMove(corner, 50, 30);
                tryCompare(tip, 'opened', true, 1000);
                // No room right or below: clamped left and flipped above the pointer.
                compare(tip.x, scene.width - tip.width);
                compare(tip.y, Math.max(0, 190 - tip.height - 12));
                verify(tip.x >= 0 && tip.x + tip.width <= scene.width);
                verify(tip.y >= 0 && tip.y + tip.height <= scene.height);
            }
            function test_05_times_out() {
                mouseMove(first, 20, 10);
                tryCompare(tip, 'opened', true, 1000);
                var shown = Date.now();
                tryCompare(tip, 'opened', false, 7000);
                // The timeout runs from the show request; polling adds up to ~100 ms.
                verify(Date.now() - shown >= 5400, 'Closed after ' + (Date.now() - shown) + ' ms');
            }

            name: 'AntonToolTip'
            when: windowShown
        }
    }
}
