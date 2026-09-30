import QtQuick
import QtQuick.Controls
import qs.Commons

// The single popover tooltip. A surface calls show(itself) when it wants its
// hint shown and hide(itself) when it no longer does. Showing a different
// source closes the tooltip first, so moving between hinted elements restarts
// the delay. `moving` (bound to the popover's scrolling state) suppresses it.
ToolTip {
    id: tip

    readonly property point cursor: source && host ? source.mapToItem(host, source.hoverX, source.hoverY) : Qt.point(0, 0)
    property Item host: null
    property bool moving: false
    readonly property AntonSurface source: current.source
    property AntonTheme theme: null

    function hide(surface: AntonSurface) {
        if (current.source === surface)
            current.source = null;
    }
    function show(surface: AntonSurface) {
        if (current.source === surface)
            return;
        current.source = null;
        current.source = surface;
    }

    delay: 450
    margins: Style.space(8)
    padding: Style.space(7)
    parent: host
    text: source ? source.hint : ""
    timeout: 6000
    visible: source !== null && source.wantsTooltip && !moving
    width: host ? Math.min(host.width, Style.space(280), tipText.implicitWidth + padding * 2) : 0
    x: host ? Math.max(0, Math.min(cursor.x + Style.space(12), host.width - width)) : 0
    y: host && cursor.y + Style.space(18) + height <= host.height ? cursor.y + Style.space(18) : Math.max(0, cursor.y - height - Style.space(12))

    background: Rectangle {
        border.color: Color.tooltip.border
        color: Color.tooltip.background
        radius: 0
    }
    // The same typography as AntonText, in the shell's tooltip text colour.
    contentItem: Text {
        id: tipText

        color: Color.tooltip.text
        elide: Text.ElideNone
        font.family: tip.theme ? tip.theme.face : ""
        font.pixelSize: Style.font.bodySmall
        text: tip.text
        textFormat: Text.PlainText
        wrapMode: Text.Wrap
    }

    QtObject {
        id: current

        property AntonSurface source: null
    }
}
