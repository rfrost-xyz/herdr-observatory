import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import qs.Commons
import "State.js" as State

AntonSurface {
    id: allowanceCard

    readonly property color balanceColour: Color.accent
    readonly property var paceReading: State.allowancePaceReading(known ? entry.paceDifference : null)
    readonly property string paceBand: paceReading.band
    readonly property bool positivePace: known && paceReading.difference !== null && paceReading.difference > 0
    property bool colourReady: false
    required property var entry
    readonly property bool known: entry.remaining !== null
    property color paceColour: targetPaceColour
    readonly property color targetPaceColour: paceBand === "surplus" ? ui.green : paceBand === "caution" ? Qt.tint(ui.muted, ui.alpha(ui.yellow, 0.65)) : paceBand === "warning" ? ui.yellow : paceBand === "deficit" ? ui.red : ui.muted

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

    Accessible.name: (ui.preferences.namesHidden ? accountIdentity.aliasName : accountIdentity.email || "Unknown account") + ". " + hint
    Accessible.role: Accessible.Button
    height: Style.space(59)
    hint: known ? ui.paceText(entry) : "Allowance unavailable"
    tint: known ? balanceColour : ui.muted

    Accessible.onPressAction: ui.toggleIdentity()
    Component.onCompleted: {
        colourReady = true;
        updatePaceColour(false);
    }
    onTargetPaceColourChanged: if (colourReady)
        updatePaceColour(ui.opened && ui.motionEnabled)

    ColorAnimation {
        id: paceTransition

        duration: 180
        property: "paceColour"
        target: allowanceCard
    }

    // Keep the target bound to current telemetry. Cancelling motion settles the
    // display immediately without replacing that binding with an old target.
    Connections {
        function onMotionEnabledChanged() {
            if (!ui.motionEnabled)
                allowanceCard.updatePaceColour(false);
        }
        function onOpenedChanged() {
            if (!ui.opened)
                allowanceCard.updatePaceColour(false);
        }

        target: ui
    }
    Item {
        id: accountIdentity

        readonly property string aliasName: ui.accountAlias(allowanceCard.entry)
        readonly property bool concealed: ui.preferences.namesHidden
        readonly property string email: ui.accountEmails[ui.accountKey(allowanceCard.entry)] || ui.accountEmails[allowanceCard.entry.id] || ui.accountEmails[allowanceCard.entry.label] || ""

        objectName: "allowance-identity"
        height: Style.space(20)
        width: Math.max(0, parent.width - accountNumbers.width - Style.space(12))
        x: 0
        y: 0

        AntonText {
            anchors.verticalCenter: parent.verticalCenter
            elide: Text.ElideMiddle
            text: accountIdentity.concealed ? accountIdentity.aliasName : accountIdentity.email || allowanceCard.entry.label
            ui: allowanceCard.ui
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
            color: allowanceCard.paceReading.difference === 0 ? ui.muted : allowanceCard.paceColour
            anchors.baseline: balanceReading.baseline
            font.pixelSize: Style.font.caption
            objectName: "allowance-pace-reading"
            text: allowanceCard.paceReading.text
            ui: allowanceCard.ui
            visible: allowanceCard.paceReading.difference !== null
        }
        AntonText {
            id: balanceReading

            color: allowanceCard.known ? ui.ink : ui.muted
            font.bold: allowanceCard.known
            objectName: "allowance-balance"
            text: allowanceCard.known ? Math.round(allowanceCard.entry.remaining) + "%" : "—"
            ui: allowanceCard.ui
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

            color: ui.line
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

                readonly property color ink: ui.alpha(allowanceCard.balanceColour, 0.62)
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
                color: ui.green
                objectName: "allowance-pace-halo"
                opacity: allowanceCard.positivePace && allowanceCard.hovered && ui.opened ? 0.24 : 0
            }
            Rectangle {
                anchors.verticalCenter: parent.verticalCenter
                color: ui.green
                height: Style.space(2)
                objectName: "allowance-pace-strip"
                width: parent.width
            }
            BurnEffect {
                active: allowanceCard.hovered && allowanceCard.positivePace
                // Keep the particles' original travel but clip it to this strip.
                anchors.bottom: parent.bottom
                deficit: false
                height: Style.space(19)
                objectName: "allowance-pace-sparks"
                tint: ui.green
                ui: allowanceCard.ui
                width: parent.width
            }
        }
        Rectangle {
            color: ui.ink
            height: balanceTrack.height + Style.space(1)
            objectName: "allowance-expected-tick"
            visible: allowanceCard.entry.timeRemaining !== null
            width: 1
            x: Math.max(0, Math.min(parent.width - width, parent.width * (allowanceCard.entry.timeRemaining || 0) / 100 - width / 2))
        }
    }
    AntonText {
        color: ui.muted
        font.pixelSize: Style.font.caption
        text: "↻ " + (allowanceCard.entry.reset || "—d —h")
        ui: allowanceCard.ui
        y: Style.space(38)
    }
    AntonText {
        anchors.right: parent.right
        color: ui.muted
        font.pixelSize: Style.font.caption
        text: (allowanceCard.entry.resetCount === null ? "—" : allowanceCard.entry.resetCount) + (allowanceCard.entry.resetCount === 1 ? " reset" : " resets")
        ui: allowanceCard.ui
        y: Style.space(38)
    }
    TapHandler {
        onTapped: ui.toggleIdentity()
    }
    HoverHandler {
        cursorShape: Qt.PointingHandCursor
    }
}
