import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import qs.Commons
import "State.js" as State

Rectangle {
    id: surface

    property string hint: ""
    readonly property bool hovered: hover.containsMouse
    property bool keyed: false
    property real restingOpacity: 0
    property bool selected: false
    property color tint: ui.blue
    property bool tooltipSuppressed: false
    required property var ui

    border.color: ui.alpha(tint, 0.5)
    border.width: keyed ? 1 : 0
    color: ui.alpha(tint, hovered || selected || keyed ? Math.max(0.12, restingOpacity) : restingOpacity)
    radius: 0

    Behavior on border.color {
        enabled: ui.opened && ui.motionEnabled

        ColorAnimation {
            duration: ui.motionEnabled ? 120 : 0
        }
    }
    Behavior on color {
        enabled: ui.opened && ui.motionEnabled

        ColorAnimation {
            duration: ui.motionEnabled ? 120 : 0
        }
    }

    MouseArea {
        id: hover

        acceptedButtons: Qt.NoButton
        anchors.fill: parent
        hoverEnabled: true
    }
    ToolTip {
        property point cursor: surface.mapToItem(ui.tooltipHost, hover.mouseX, hover.mouseY)

        delay: 450
        margins: Style.space(8)
        padding: Style.space(7)
        parent: ui.tooltipHost
        text: surface.hint
        timeout: 6000
        visible: !surface.tooltipSuppressed && !ui.tooltipHost.moving && hover.containsMouse && surface.hint.length > 0
        width: Math.min(ui.tooltipHost.width, Style.space(280), tipText.implicitWidth + padding * 2)
        x: Math.max(0, Math.min(cursor.x + Style.space(12), ui.tooltipHost.width - width))
        y: cursor.y + Style.space(18) + height <= ui.tooltipHost.height ? cursor.y + Style.space(18) : Math.max(0, cursor.y - height - Style.space(12))

        background: Rectangle {
            border.color: Color.tooltip.border
            color: Color.tooltip.background
            radius: 0
        }
        contentItem: AntonText {
            id: tipText

            color: Color.tooltip.text
            elide: Text.ElideNone
            text: surface.hint
            ui: surface.ui
            wrapMode: Text.Wrap
        }
    }
}
