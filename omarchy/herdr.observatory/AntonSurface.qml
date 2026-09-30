import QtQuick
import QtQuick.Controls

// A hoverable, tinted surface. Its hint is shown by the popover's single
// shared tooltip: the surface asks for it while hovered and not suppressed.
Rectangle {
    id: surface

    // Colour transitions run only while the popover is open with motion enabled.
    property bool animate: false
    property string hint: ""
    readonly property bool hovered: hover.containsMouse
    readonly property real hoverX: hover.mouseX
    readonly property real hoverY: hover.mouseY
    property bool keyed: false
    property real restingOpacity: 0
    property bool selected: false
    required property AntonTheme theme
    property color tint: theme.blue
    // Typed as the base ToolTip: AntonToolTip already names AntonSurface, and
    // Qt 6.8 cannot resolve two files that name each other.
    property ToolTip tooltip: null
    property bool tooltipSuppressed: false
    readonly property bool wantsTooltip: hovered && hint.length > 0 && !tooltipSuppressed

    border.color: theme.alpha(tint, 0.5)
    border.width: keyed ? 1 : 0
    color: theme.alpha(tint, hovered || selected || keyed ? Math.max(0.12, restingOpacity) : restingOpacity)
    radius: 0

    Behavior on border.color {
        enabled: surface.animate

        ColorAnimation {
            duration: 120
        }
    }
    Behavior on color {
        enabled: surface.animate

        ColorAnimation {
            duration: 120
        }
    }

    Component.onDestruction: {
        if (tooltip)
            tooltip.hide(surface);
    }
    onWantsTooltipChanged: {
        if (!tooltip)
            return;
        if (wantsTooltip)
            tooltip.show(surface);
        else
            tooltip.hide(surface);
    }

    MouseArea {
        id: hover

        acceptedButtons: Qt.NoButton
        anchors.fill: parent
        hoverEnabled: true
    }
}
