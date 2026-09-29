import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import qs.Commons
import "State.js" as State

Item {
    id: signal

    property string threadState: "unknown"
    property color tint: ui.stateColour(threadState)
    required property var ui

    Canvas {
        id: spinner

        property color tone: signal.tint

        anchors.centerIn: parent
        height: width
        visible: signal.threadState === "working"
        width: parent.width * 0.72

        NumberAnimation on rotation {
            duration: 1100
            from: 0
            loops: Animation.Infinite
            running: ui.opened && ui.motionEnabled && signal.visible && spinner.visible
            to: 360
        }

        onPaint: {
            var ctx = getContext("2d"), r = width / 2 - 2;
            ctx.reset();
            ctx.lineWidth = 2;
            ctx.strokeStyle = ui.alpha(tone, 0.18).toString();
            ctx.beginPath();
            ctx.arc(width / 2, height / 2, r, 0, Math.PI * 2);
            ctx.stroke();
            ctx.strokeStyle = tone.toString();
            ctx.beginPath();
            ctx.arc(width / 2, height / 2, r, -Math.PI / 2, Math.PI);
            ctx.stroke();
        }
        onToneChanged: requestPaint()
        onWidthChanged: requestPaint()
    }
    AntonText {
        anchors.centerIn: parent
        color: signal.tint
        font.bold: true
        font.pixelSize: signal.height * 0.72
        text: signal.threadState === "blocked" ? "!" : signal.threadState === "done" ? "✓" : signal.threadState === "idle" ? "Ⅱ" : "?"
        ui: signal.ui
        visible: signal.threadState !== "working"
    }
}
