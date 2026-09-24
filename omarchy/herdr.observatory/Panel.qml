import QtQuick
import QtQuick.Controls
import Quickshell
import Quickshell.Io
import qs.Commons
import qs.Ui

Panel {
  id: root
  moduleName: "herdr.observatory"
  ipcTarget: moduleName
  manageIpc: false

  readonly property color ink: bar ? bar.foreground : Color.foreground
  readonly property color subtle: Qt.darker(ink, 1.5)
  readonly property string face: bar ? bar.fontFamily : Style.font.family
  readonly property var overview: snapshot.view
  implicitWidth: button.implicitWidth
  implicitHeight: button.implicitHeight

  function launchCompanion() {
    var file = Qt.resolvedUrl("Companion.qml").toString().replace(/^file:\/\//, "")
    Quickshell.execDetached(["/usr/lib/qt6/bin/qml", file])
    close()
  }

  onOpenedChanged: if (opened) { snapshot.refresh(); Qt.callLater(function() { keyCatcher.forceActiveFocus() }) }

  SnapshotStore { id: snapshot }

  IpcHandler {
    target: root.ipcTarget
    function open(): void { root.open() }
    function close(): void { root.close() }
    function toggle(): void { root.toggle() }
    function refresh(): void { snapshot.refresh() }
  }

  WidgetButton {
    id: button
    anchors.fill: parent
    bar: root.bar
    text: root.overview.connected && root.overview.working !== null ? "◇ " + root.overview.working + (root.overview.partial ? "*" : "") : "◇ ?"
    fontSize: Style.font.bodySmall
    active: root.overview.partial || !root.overview.connected
    tooltipText: root.overview.connected && root.overview.working !== null ? "Herdr: " + root.overview.working + " working" + (root.overview.partial ? ", partial" : "") : "Herdr sources unavailable"
    onPressed: function(code) { if (code === Qt.MiddleButton) snapshot.refresh(); else root.toggle() }
  }

  KeyboardPanel {
    id: panel
    anchorItem: button
    owner: root
    bar: root.bar
    open: root.opened
    focusTarget: keyCatcher
    contentWidth: panel.fittedContentWidth(Style.space(420))
    contentHeight: panel.fittedContentHeight(content.implicitHeight, Style.space(620))

    PanelKeyCatcher {
      id: keyCatcher
      anchors.fill: parent
      onCloseRequested: root.close()
      onActivateRequested: root.launchCompanion()
      onTabRequested: function(direction) { root.switchPanel(direction) }
      onTextKey: function(key) { if (key === "r" || key === "R") snapshot.refresh() }
      onMoveRequested: function(dx, dy) {
        if (dy !== 0)
          panelFlick.contentY = Math.max(0, Math.min(panelFlick.contentY + dy * Style.space(56),
                                                     Math.max(0, panelFlick.contentHeight - panelFlick.height)))
      }

      Flickable {
        id: panelFlick
        anchors.fill: parent
        contentWidth: width
        contentHeight: content.implicitHeight
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        flickableDirection: Flickable.VerticalFlick
        interactive: contentHeight > height
        ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }

        Column {
          id: content
          width: panelFlick.width
          spacing: Style.space(12)

          Text {
            text: "HERDR / COMPANION"
            textFormat: Text.PlainText
            color: Color.accent
            font.family: root.face
            font.pixelSize: Style.font.bodySmall
            font.bold: true
          }
          Text {
            text: root.overview.connected && root.overview.working !== null ? root.overview.working + " working" + (root.overview.partial ? " · partial" : "") : "Current count unavailable"
            textFormat: Text.PlainText
            color: root.ink
            font.family: root.face
            font.pixelSize: Style.font.display
            font.bold: true
          }
          Text {
            text: root.overview.note
            textFormat: Text.PlainText
            color: root.subtle
            font.family: root.face
            font.pixelSize: Style.font.bodySmall
          }

          Rectangle { width: parent.width; height: 1; color: Color.popups.border }
          Text { text: "THREADS"; color: root.subtle; font.family: root.face; font.pixelSize: Style.font.bodySmall; font.bold: true }
          Repeater {
            model: root.overview.threads.slice(0, 5)
            delegate: Column {
              required property var modelData
              width: content.width
              spacing: 2
              Text {
                width: parent.width
                text: modelData.project + "  ·  " + modelData.state.toUpperCase()
                textFormat: Text.PlainText
                elide: Text.ElideRight
                color: modelData.state === "blocked" ? Color.urgent : root.ink
                font.family: root.face
                font.pixelSize: Style.font.body
                font.bold: true
              }
              Text {
                width: parent.width
                text: modelData.host + " · " + modelData.harness + " · " + modelData.age
                textFormat: Text.PlainText
                elide: Text.ElideRight
                color: root.subtle
                font.family: root.face
                font.pixelSize: Style.font.bodySmall
              }
            }
          }
          Text {
            visible: root.overview.connected && root.overview.threads.length === 0
            text: root.overview.partial ? "No threads in reporting sources" : "No current threads"
            textFormat: Text.PlainText
            color: root.subtle
            font.family: root.face
            font.pixelSize: Style.font.body
          }
          Text {
            visible: root.overview.threads.length > 5
            text: "+" + (root.overview.threads.length - 5) + " more in companion"
            textFormat: Text.PlainText
            color: root.subtle
            font.family: root.face
            font.pixelSize: Style.font.bodySmall
          }

          Rectangle { width: parent.width; height: 1; color: Color.popups.border }
          Text { text: "CODEX WEEKLY"; color: root.subtle; font.family: root.face; font.pixelSize: Style.font.bodySmall; font.bold: true }
          Repeater {
            model: root.overview.allowances
            delegate: Text {
              required property var modelData
              text: modelData.label + "  " + (modelData.remaining === null ? "unavailable" : modelData.remaining + "% remaining · " + modelData.age)
              textFormat: Text.PlainText
              color: root.ink
              font.family: root.face
              font.pixelSize: Style.font.body
            }
          }
          Text {
            visible: root.overview.allowances.length === 0
            text: "No mapped account allowance"
            textFormat: Text.PlainText
            color: root.subtle
            font.family: root.face
            font.pixelSize: Style.font.body
          }

          Rectangle { width: parent.width; height: 1; color: Color.popups.border }
          Text { text: "LOCAL INFERENCE / FLEET"; color: root.subtle; font.family: root.face; font.pixelSize: Style.font.bodySmall; font.bold: true }
          Text {
            text: root.overview.gpu ? root.overview.gpu.host + " GPU " + Math.round(root.overview.gpu.percent) + "% · " + root.overview.gpu.age : "GPU sample unavailable"
            textFormat: Text.PlainText
            color: root.ink
            font.family: root.face
            font.pixelSize: Style.font.body
          }
          Text { text: root.overview.inference; textFormat: Text.PlainText; color: root.subtle; font.family: root.face; font.pixelSize: Style.font.bodySmall }
          Repeater {
            model: root.overview.hosts
            delegate: Text {
              required property var modelData
              text: modelData.name + " · " + (modelData.reporting ? "reporting · " + modelData.age : "unavailable")
              textFormat: Text.PlainText
              color: modelData.reporting ? root.ink : Color.urgent
              font.family: root.face
              font.pixelSize: Style.font.bodySmall
            }
          }

          Rectangle {
            width: parent.width
            height: Style.space(38)
            radius: Style.space(8)
            color: Color.accent
            Text { anchors.centerIn: parent; text: "Open companion  ↗"; textFormat: Text.PlainText; color: Color.background; font.family: root.face; font.pixelSize: Style.font.body; font.bold: true }
            MouseArea { anchors.fill: parent; onClicked: root.launchCompanion() }
          }
        }
      }
    }
  }
}
