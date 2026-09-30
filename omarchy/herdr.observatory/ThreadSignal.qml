import QtQuick

Item {
    id: signal

    // The spinner turns only while the popover is open with motion enabled.
    property bool animate: false
    required property AntonTheme theme
    property string threadState: "unknown"
    property color tint: theme.stateColour(threadState)

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
            running: signal.animate && signal.visible && spinner.visible
            to: 360
        }

        onPaint: {
            var ctx = getContext("2d"), r = width / 2 - 2;
            ctx.reset();
            ctx.lineWidth = 2;
            ctx.strokeStyle = signal.theme.alpha(tone, 0.18).toString();
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
        theme: signal.theme
        visible: signal.threadState !== "working"
    }
}
