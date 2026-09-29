import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import qs.Commons
import "State.js" as State

Text {
    id: textRoot

    required property var ui

    color: ui.ink
    elide: Text.ElideRight
    font.family: ui.face
    font.pixelSize: Style.font.bodySmall
    textFormat: Text.PlainText
}
