import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import qs.Commons
import "State.js" as State

Item {
    id: sheen

    property bool active: false
    property real phase: -1
    property string text
    property color tint
    required property var ui

    clip: true
    implicitHeight: titleText.implicitHeight
    implicitWidth: titleText.implicitWidth

    SequentialAnimation on phase {
        loops: Animation.Infinite
        running: sheen.active && ui.opened && ui.motionEnabled && sheen.visible

        NumberAnimation {
            duration: 1500
            easing.type: Easing.InOutSine
            from: 0
            to: 1
        }
        PauseAnimation {
            duration: 900
        }
    }

    AntonText {
        id: titleText

        anchors.fill: parent
        color: sheen.tint
        font.bold: true
        text: sheen.text
        ui: sheen.ui
    }
    Repeater {
        model: 9

        Item {
            required property int index

            clip: true
            height: sheen.height
            opacity: [0.08, 0.18, 0.38, 0.7, 1, 0.7, 0.38, 0.18, 0.08][index]
            visible: sheen.active && ui.opened && ui.motionEnabled && sheen.visible
            width: 8
            x: sheen.phase * (sheen.width + 100) - 72 + index * 8

            AntonText {
                color: Qt.lighter(ui.ink, 1.45)
                font.bold: true
                height: sheen.height
                text: sheen.text
                ui: sheen.ui
                width: sheen.width
                x: -parent.x
            }
        }
    }
}
