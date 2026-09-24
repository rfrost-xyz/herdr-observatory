import QtQuick
import QtQuick.Controls

ApplicationWindow {
  id: root
  visible: true
  title: "Herdr Companion"
  width: 920
  height: 700
  minimumWidth: 360
  minimumHeight: 320

  SnapshotStore { id: snapshot }
  readonly property var overview: snapshot.view
  readonly property var colours: snapshot.raw && snapshot.raw.theme && snapshot.raw.theme.colours
                                 ? snapshot.raw.theme.colours : ({})
  readonly property color bg: colours.background || "#1a1b26"
  readonly property color panel: colours.lighter_background || "#24283b"
  readonly property color ink: colours.foreground || "#c0caf5"
  readonly property color muted: colours.muted || "#8893aa"
  readonly property color accent: colours.accent || "#7aa2f7"
  readonly property color warning: colours.yellow || "#e0af68"
  readonly property color bad: colours.red || "#f7768e"
  readonly property bool wide: width >= 870
  color: bg

  component Caption: Text {
    width: parent ? parent.width : implicitWidth
    textFormat: Text.PlainText
    color: root.muted
    font.pixelSize: 11
    font.bold: true
    font.letterSpacing: 1.5
    wrapMode: Text.WordWrap
  }
  component Body: Text {
    width: parent ? parent.width : implicitWidth
    textFormat: Text.PlainText
    color: root.ink
    font.pixelSize: 13
    wrapMode: Text.WordWrap
  }

  Flickable {
    id: scroll
    anchors.fill: parent
    contentWidth: width
    contentHeight: page.implicitHeight + 40
    clip: true
    boundsBehavior: Flickable.StopAtBounds
    ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }

    Column {
      id: page
      x: 20
      y: 20
      width: Math.max(0, scroll.width - 40)
      spacing: 18

      Column {
        width: parent.width
        spacing: 5
        Caption { text: "HERDR / COMPANION"; color: root.accent }
        Text {
          width: parent.width
          text: root.overview.connected && root.overview.working !== null ? root.overview.working + " working" + (root.overview.partial ? " · partial" : "") : "Current count unavailable"
          textFormat: Text.PlainText
          color: root.ink
          font.pixelSize: 28
          font.bold: true
          wrapMode: Text.WordWrap
        }
        Body { text: root.overview.note; color: root.muted }
      }

      Flow {
        id: cards
        width: parent.width
        spacing: 14

        Rectangle {
          width: root.wide ? Math.max(0, cards.width * 0.61 - 7) : cards.width
          height: threadColumn.implicitHeight + 32
          radius: 13
          color: root.panel
          border.color: Qt.rgba(root.ink.r, root.ink.g, root.ink.b, 0.17)
          Column {
            id: threadColumn
            x: 16; y: 16
            width: parent.width - 32
            spacing: 11
            Caption { text: "HERDR THREADS"; color: root.accent }
            Repeater {
              model: root.overview.threads
              delegate: Column {
                required property var modelData
                width: threadColumn.width
                spacing: 3
                Body { width: parent.width; text: modelData.state.toUpperCase(); font.bold: true; color: modelData.state === "blocked" ? root.bad : root.accent }
                Body { width: parent.width; text: modelData.project; font.bold: true }
                Body { width: parent.width; text: modelData.title; color: root.ink }
                Body { width: parent.width; text: modelData.host + " / " + modelData.harness + " · sample " + modelData.age; color: root.muted; font.pixelSize: 11 }
              }
            }
            Body {
              visible: root.overview.connected && root.overview.threads.length === 0
              text: root.overview.partial ? "No threads in reporting sources" : "No current threads"
              color: root.muted
            }
            Body { text: "Herdr owns thread input and lifecycle."; color: root.muted; font.pixelSize: 11 }
          }
        }

        Column {
          width: root.wide ? Math.max(0, cards.width * 0.39 - 7) : cards.width
          spacing: 14

          Rectangle {
            width: parent.width
            height: allowanceColumn.implicitHeight + 32
            radius: 13
            color: root.panel
            border.color: Qt.rgba(root.ink.r, root.ink.g, root.ink.b, 0.17)
            Column {
              id: allowanceColumn
              x: 16; y: 16
              width: parent.width - 32
              spacing: 9
              Caption { text: "CODEX WEEKLY"; color: root.accent }
              Repeater {
                model: root.overview.allowances
                delegate: Column {
                  required property var modelData
                  width: allowanceColumn.width
                  spacing: 2
                  Body { text: modelData.label; font.bold: true }
                  Body {
                    text: modelData.remaining === null ? "Unavailable" : modelData.remaining + "% remaining · " + modelData.age
                    color: modelData.remaining === null ? root.warning : root.ink
                  }
                }
              }
              Body { visible: root.overview.allowances.length === 0; text: "No mapped allowance source"; color: root.muted }
            }
          }

          Rectangle {
            width: parent.width
            height: inferenceColumn.implicitHeight + 32
            radius: 13
            color: root.panel
            border.color: Qt.rgba(root.ink.r, root.ink.g, root.ink.b, 0.17)
            Column {
              id: inferenceColumn
              x: 16; y: 16
              width: parent.width - 32
              spacing: 8
              Caption { text: "LOCAL AI / GPU"; color: root.accent }
              Body {
                text: root.overview.gpu ? root.overview.gpu.host + " GPU " + Math.round(root.overview.gpu.percent) + "% · " + root.overview.gpu.age : "GPU sample unavailable"
                font.bold: true
              }
              Body { text: root.overview.inference; color: root.warning }
              Body { text: "GPU utilisation is not request throughput."; color: root.muted; font.pixelSize: 11 }
            }
          }

          Rectangle {
            width: parent.width
            height: fleetColumn.implicitHeight + 32
            radius: 13
            color: root.panel
            border.color: Qt.rgba(root.ink.r, root.ink.g, root.ink.b, 0.17)
            Column {
              id: fleetColumn
              x: 16; y: 16
              width: parent.width - 32
              spacing: 8
              Caption { text: "FLEET REPORTING"; color: root.accent }
              Repeater {
                model: root.overview.hosts
                delegate: Body {
                  required property var modelData
                  width: fleetColumn.width
                  text: modelData.name + " · " + (modelData.reporting ? "reporting · " + modelData.age : "unavailable")
                  color: modelData.reporting ? root.ink : root.warning
                }
              }
              Body { visible: root.overview.hosts.length === 0; text: "Sources unavailable"; color: root.muted }
            }
          }
        }
      }
    }
  }
}
