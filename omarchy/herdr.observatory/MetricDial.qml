import QtQuick
import qs.Commons

Item {
    id: dial

    property bool failureNotch: false
    property real pulse: 0
    property real ratio: -1
    property string reading: "—"
    property string symbol: ""
    property color symbolTint: theme.muted
    required property AntonTheme theme
    property color tint: theme.muted

    implicitHeight: Style.space(40)
    implicitWidth: Style.space(52)

    Canvas {
        id: arc

        property bool failure: dial.failureNotch
        property color failureTone: dial.theme.red
        property color tone: dial.tint
        property real value: dial.ratio

        anchors.fill: parent

        onFailureChanged: requestPaint()
        onFailureToneChanged: requestPaint()
        onPaint: {
            var ctx = getContext("2d"), cx = width / 2, cy = height / 2 + 2, r = Math.min(width / 2 - 3, height / 2 - 2);
            ctx.reset();
            ctx.lineWidth = 2;
            ctx.lineCap = "butt";
            ctx.strokeStyle = dial.theme.alpha(tone, 0.2).toString();
            ctx.beginPath();
            ctx.arc(cx, cy, r, Math.PI * 0.75, Math.PI * 2.25);
            ctx.stroke();
            if (value >= 0) {
                ctx.strokeStyle = tone.toString();
                ctx.beginPath();
                ctx.arc(cx, cy, r, Math.PI * 0.75, Math.PI * (0.75 + 1.5 * Math.min(1, value)));
                ctx.stroke();
            }
            if (failure) {
                ctx.lineWidth = 3;
                ctx.strokeStyle = failureTone.toString();
                ctx.beginPath();
                ctx.arc(cx, cy, r, Math.PI * 2.16, Math.PI * 2.25);
                ctx.stroke();
            }
        }
        onToneChanged: requestPaint()
        onValueChanged: requestPaint()
        onWidthChanged: requestPaint()
    }
    AntonText {
        anchors.centerIn: parent
        anchors.verticalCenterOffset: Style.space(2)
        color: dial.tint
        font.bold: true
        font.pixelSize: Style.font.caption
        fontSizeMode: Text.Fit
        height: Style.space(18)
        horizontalAlignment: Text.AlignHCenter
        minimumPixelSize: Style.space(8)
        scale: 1 + dial.pulse * 0.12
        text: dial.reading
        theme: dial.theme
        verticalAlignment: Text.AlignVCenter
        width: parent.width - Style.space(14)
    }
    AntonText {
        anchors.bottom: parent.bottom
        anchors.horizontalCenter: parent.horizontalCenter
        color: dial.symbolTint
        font.pixelSize: Style.space(9)
        text: dial.symbol
        theme: dial.theme
    }
}
