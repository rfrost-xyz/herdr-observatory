pragma ComponentBehavior: Bound

import QtQuick

Item {
    id: sheen

    property bool active: false
    // The sweep runs only while the popover is open with motion enabled.
    property bool animate: false
    property real phase: -1
    property string text
    required property AntonTheme theme
    property color tint

    clip: true
    implicitHeight: titleText.implicitHeight
    implicitWidth: titleText.implicitWidth

    // Qt 6.8 qmllint reads the literal initial phase as a binding.
    // qmllint disable duplicate-property-binding
    SequentialAnimation on phase {
        // qmllint enable duplicate-property-binding
        loops: Animation.Infinite
        running: sheen.active && sheen.animate && sheen.visible

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
        theme: sheen.theme
    }
    Repeater {
        model: 9

        Item {
            required property int index

            clip: true
            height: sheen.height
            opacity: [0.08, 0.18, 0.38, 0.7, 1, 0.7, 0.38, 0.18, 0.08][index]
            visible: sheen.active && sheen.animate && sheen.visible
            width: 8
            x: sheen.phase * (sheen.width + 100) - 72 + index * 8

            AntonText {
                color: Qt.lighter(sheen.theme.ink, 1.45)
                font.bold: true
                height: sheen.height
                text: sheen.text
                theme: sheen.theme
                width: sheen.width
                x: -parent.x
            }
        }
    }
}
