import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import qs.Commons
import "State.js" as State

AntonSurface {
    id: section

    readonly property bool collapsed: ui.parseList(ui.preferences.collapsedSections).indexOf(sectionKey) >= 0
    property string sectionKey
    property string title

    Accessible.name: hint
    Accessible.role: Accessible.Button
    height: Style.space(30)
    hint: (collapsed ? "Expand " : "Collapse ") + title.toLowerCase()
    tint: ui.ink

    Accessible.onPressAction: ui.toggleList("collapsedSections", section.sectionKey)

    AntonText {
        anchors.verticalCenter: parent.verticalCenter
        color: ui.muted
        font.pixelSize: Style.space(13)
        text: section.collapsed ? "›" : "⌄"
        ui: section.ui
        x: Style.space(4)
    }
    AntonText {
        anchors.verticalCenter: parent.verticalCenter
        color: ui.muted
        font.bold: true
        font.letterSpacing: 1.2
        font.pixelSize: Style.font.caption
        text: section.title
        ui: section.ui
        x: Style.space(22)
    }
    TapHandler {
        onTapped: ui.toggleList("collapsedSections", section.sectionKey)
    }
    HoverHandler {
        cursorShape: Qt.PointingHandCursor
    }
}
