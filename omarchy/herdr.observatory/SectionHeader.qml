import QtQuick
import qs.Commons

AntonSurface {
    id: section

    property bool collapsed: false
    property string title

    signal toggled

    Accessible.name: hint
    Accessible.role: Accessible.Button
    height: Style.space(30)
    hint: (collapsed ? "Expand " : "Collapse ") + title.toLowerCase()
    tint: theme.ink

    Accessible.onPressAction: section.toggled()

    AntonText {
        anchors.verticalCenter: parent.verticalCenter
        color: section.theme.muted
        font.pixelSize: Style.space(13)
        text: section.collapsed ? "›" : "⌄"
        theme: section.theme
        x: Style.space(4)
    }
    AntonText {
        anchors.verticalCenter: parent.verticalCenter
        color: section.theme.muted
        font.bold: true
        font.letterSpacing: 1.2
        font.pixelSize: Style.font.caption
        text: section.title
        theme: section.theme
        x: Style.space(22)
    }
    TapHandler {
        onTapped: section.toggled()
    }
    HoverHandler {
        cursorShape: Qt.PointingHandCursor
    }
}
