import QtQuick
import qs.Commons

Text {
    required property AntonTheme theme

    color: theme.ink
    elide: Text.ElideRight
    font.family: theme.face
    font.pixelSize: Style.font.bodySmall
    textFormat: Text.PlainText
}
