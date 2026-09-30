import QtQuick
import qs.Commons
import "State.js" as State

AntonSurface {
    id: allowanceCard

    readonly property color balanceColour: Color.accent
    readonly property var paceReading: State.allowancePaceReading(known ? entry.paceDifference : null)
    readonly property string paceBand: paceReading.band
    readonly property bool positivePace: known && paceReading.difference !== null && paceReading.difference > 0
    // The concealed name shown when namesHidden is set.
    property string aliasName: ""
    property bool colourReady: false
    // The verified email for this account, or "" when none is mapped.
    property string email: ""
    required property var entry
    readonly property bool known: entry.remaining !== null
    property bool motionEnabled: true
    property bool namesHidden: false
    // Time-derived readings use this instant, never the view.
    property double now: 0
    property bool opened: false
    property color paceColour: targetPaceColour
    readonly property color targetPaceColour: paceBand === "surplus" ? theme.green : paceBand === "caution" ? Qt.tint(theme.muted, theme.alpha(theme.yellow, 0.65)) : paceBand === "warning" ? theme.yellow : paceBand === "deficit" ? theme.red : theme.muted

    signal identityToggled

    function updatePaceColour(animate) {
        var from = paceColour;
        paceTransition.stop();
        if (animate && from !== targetPaceColour) {
            paceTransition.from = from;
            paceTransition.to = targetPaceColour;
            paceTransition.start();
        } else {
            paceColour = targetPaceColour;
        }
    }

    Accessible.name: (namesHidden ? aliasName : email || "Unknown account") + ". " + hint
    Accessible.role: Accessible.Button
    height: Style.space(59)
    animate: opened && motionEnabled
    hint: known ? State.paceText(entry) : (entry.statusText || "Allowance unavailable")
    tint: known ? balanceColour : theme.muted

    Accessible.onPressAction: allowanceCard.identityToggled()
    Component.onCompleted: {
        colourReady = true;
        updatePaceColour(false);
    }
    onTargetPaceColourChanged: if (colourReady)
        updatePaceColour(opened && motionEnabled)

    ColorAnimation {
        id: paceTransition

        duration: 180
        property: "paceColour"
        target: allowanceCard
    }

    // Keep the target bound to current telemetry. Cancelling motion settles the
    // display immediately without replacing that binding with an old target.
    onMotionEnabledChanged: {
        if (!motionEnabled)
            updatePaceColour(false);
    }
    onOpenedChanged: {
        if (!opened)
            updatePaceColour(false);
    }
    Item {
        id: accountIdentity

        objectName: "allowance-identity"
        height: Style.space(20)
        width: Math.max(0, parent.width - accountNumbers.width - Style.space(12))
        x: 0
        y: 0

        AntonText {
            anchors.verticalCenter: parent.verticalCenter
            elide: Text.ElideMiddle
            text: allowanceCard.namesHidden ? allowanceCard.aliasName : allowanceCard.email || allowanceCard.entry.label
            theme: allowanceCard.theme
            width: parent.width
        }
    }
    Row {
        id: accountNumbers
        objectName: "allowance-numbers"

        anchors.right: parent.right
        anchors.rightMargin: 0
        spacing: Style.space(10)
        y: Style.space(2)

        AntonText {
            color: allowanceCard.paceReading.difference === 0 ? allowanceCard.theme.muted : allowanceCard.paceColour
            anchors.baseline: balanceReading.baseline
            font.pixelSize: Style.font.caption
            objectName: "allowance-pace-reading"
            text: allowanceCard.paceReading.text
            theme: allowanceCard.theme
            visible: allowanceCard.paceReading.difference !== null
        }
        AntonText {
            id: balanceReading

            color: allowanceCard.known ? allowanceCard.theme.ink : allowanceCard.theme.muted
            font.bold: allowanceCard.known
            objectName: "allowance-balance"
            text: allowanceCard.known ? Math.round(allowanceCard.entry.remaining) + "%" : "—"
            theme: allowanceCard.theme
        }
    }
    Item {
        id: allowanceGraph

        height: Style.space(14)
        visible: allowanceCard.known
        width: parent.width
        x: 0
        y: Style.space(22)

        Rectangle {
            id: balanceTrack

            color: allowanceCard.theme.line
            height: Style.space(6)
            objectName: "allowance-track"
            width: parent.width

            Rectangle {
                color: allowanceCard.balanceColour
                height: parent.height
                objectName: "allowance-fill"
                width: parent.width * (allowanceCard.entry.remaining || 0) / 100
            }
            Canvas {
                id: deficitHatch

                readonly property color ink: allowanceCard.theme.alpha(allowanceCard.balanceColour, 0.62)
                clip: true
                height: parent.height
                objectName: "allowance-deficit-hatch"
                visible: allowanceCard.known && allowanceCard.paceReading.difference !== null && allowanceCard.entry.paceDifference < 0
                width: parent.width * Math.max(0, -(allowanceCard.entry.paceDifference || 0)) / 100
                x: parent.width * (allowanceCard.entry.remaining || 0) / 100

                onPaint: {
                    var ctx = getContext("2d");
                    ctx.reset();
                    if (!visible) return;
                    ctx.strokeStyle = ink.toString();
                    ctx.lineWidth = Style.space(1);
                    ctx.beginPath();
                    for (var offset = -height; offset < width; offset += Style.space(5)) {
                        ctx.moveTo(offset, height);
                        ctx.lineTo(offset + height, 0);
                    }
                    ctx.stroke();
                }
                onWidthChanged: requestPaint()
                onHeightChanged: requestPaint()
                onInkChanged: requestPaint()
                onVisibleChanged: requestPaint()
            }
        }
        Item {
            id: paceInterval

            clip: true
            height: Style.space(6)
            objectName: "allowance-pace-region"
            visible: allowanceCard.positivePace
            width: parent.width * Math.abs(allowanceCard.entry.paceDifference || 0) / 100
            x: parent.width * Math.min(allowanceCard.entry.remaining || 0, allowanceCard.entry.timeRemaining || 0) / 100
            y: balanceTrack.height + Style.space(2)

            // This clipped halo cannot recolour the remaining balance above it.
            Rectangle {
                anchors.fill: parent
                color: allowanceCard.theme.green
                objectName: "allowance-pace-halo"
                opacity: allowanceCard.positivePace && allowanceCard.hovered && allowanceCard.opened ? 0.24 : 0
            }
            Rectangle {
                anchors.verticalCenter: parent.verticalCenter
                color: allowanceCard.theme.green
                height: Style.space(2)
                objectName: "allowance-pace-strip"
                width: parent.width
            }
            BurnEffect {
                active: allowanceCard.hovered && allowanceCard.positivePace
                animate: allowanceCard.animate
                // Keep the particles' original travel but clip it to this strip.
                anchors.bottom: parent.bottom
                deficit: false
                height: Style.space(19)
                objectName: "allowance-pace-sparks"
                theme: allowanceCard.theme
                tint: allowanceCard.theme.green
                width: parent.width
            }
        }
        Rectangle {
            color: allowanceCard.theme.ink
            height: balanceTrack.height + Style.space(1)
            objectName: "allowance-expected-tick"
            visible: allowanceCard.entry.timeRemaining !== null
            width: 1
            x: Math.max(0, Math.min(parent.width - width, parent.width * (allowanceCard.entry.timeRemaining || 0) / 100 - width / 2))
        }
    }
    AntonText {
        color: allowanceCard.theme.muted
        font.pixelSize: Style.font.caption
        text: "↻ " + (allowanceCard.entry.reset || "—d —h")
        theme: allowanceCard.theme
        y: Style.space(38)
    }
    AntonText {
        anchors.right: parent.right
        color: allowanceCard.theme.muted
        font.pixelSize: Style.font.caption
        text: (allowanceCard.entry.resetCount === null ? "—" : allowanceCard.entry.resetCount) + (allowanceCard.entry.resetCount === 1 ? " reset" : " resets")
        theme: allowanceCard.theme
        y: Style.space(38)
    }
    TapHandler {
        onTapped: allowanceCard.identityToggled()
    }
    HoverHandler {
        cursorShape: Qt.PointingHandCursor
    }
}
