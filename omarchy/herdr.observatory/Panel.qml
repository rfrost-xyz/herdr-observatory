import QtQuick
import Quickshell
import Quickshell.Io
import qs.Commons
import qs.Ui
import "State.js" as State

Panel {
    id: root

    property var accountEmails: ({})
    readonly property color barColour: root.bar ? root.bar.barForeground : Color.foreground
    readonly property bool motionEnabled: Quickshell.env("ANTON_REDUCED_MOTION") !== "1"

    implicitHeight: button.implicitHeight
    implicitWidth: button.implicitWidth
    ipcTarget: root.moduleName
    manageIpc: false
    moduleName: "herdr.observatory"

    // The controller resets the visual epoch and focus first; then the palette
    // reloads and a dead collector restarts, as before.
    onOpenedChanged: {
        popoverController.opened = root.opened;
        if (root.opened) {
            popoverTheme.reload();
            snapshot.restart();
            Qt.callLater(function () {
                keyCatcher.forceActiveFocus();
            });
        }
    }

    AntonTheme {
        id: popoverTheme

        face: root.bar ? root.bar.fontFamily : Style.font.family
    }
    AntonPreferences {
        id: popoverPreferences

        location: "file://" + (Quickshell.env("XDG_STATE_HOME") || Quickshell.env("HOME") + "/.local/state") + "/herdr.observatory/privacy.ini"
    }
    AntonController {
        id: popoverController

        motionEnabled: root.motionEnabled
        preferences: popoverPreferences
        view: snapshot.view

        onCloseRequested: root.close()
        onRefreshRequested: snapshot.refresh()
    }
    FileView {
        id: identitiesFile

        path: Qt.resolvedUrl(".accounts.json").toString().replace(/^file:\/\//, "")
        printErrors: false
        watchChanges: true

        onFileChanged: reload()
        onLoaded: {
            try {
                root.accountEmails = JSON.parse(text());
            } catch (error) {
                root.accountEmails = ({});
            }
        }
    }
    SnapshotStore {
        id: snapshot

        visualUpdates: root.opened
    }
    IpcHandler {
        function close(): void {
            root.close();
        }
        function diagnostics(): string {
            return State.diagnostics(snapshot.view, Date.now());
        }
        function open(): void {
            root.open();
        }
        function refresh(): void {
            snapshot.refresh();
            popoverTheme.reload();
        }
        function status(): string {
            return root.opened ? "open" : "closed";
        }
        function toggle(): void {
            root.toggle();
        }

        target: root.ipcTarget
    }
    BarIconButton {
        id: button

        anchors.fill: parent
        bar: root.bar
        foreground: root.barColour
        text: "󱚣"
        tooltipText: "Anton · " + popoverController.barState + (snapshot.view.partial || !snapshot.view.connected ? " · sources unavailable" : "")
        useActiveColor: false

        onPressed: function (code) {
            if (code === Qt.MiddleButton)
                snapshot.refresh();
            else
                root.toggle();
        }

        Item {
            anchors.centerIn: parent
            anchors.horizontalCenterOffset: Style.space(6)
            anchors.verticalCenterOffset: -Style.space(5)
            height: Style.space(12)
            width: Style.space(10)

            AntonText {
                anchors.centerIn: parent
                color: popoverTheme.stateColour(popoverController.barState)
                font.bold: true
                font.pixelSize: Style.space(12)
                text: popoverController.barState === "blocked" ? "!" : popoverController.barState === "done" ? "✓" : ""
                theme: popoverTheme
            }
            Rectangle {
                anchors.centerIn: parent
                color: popoverTheme.yellow
                height: width
                radius: width / 2
                visible: popoverController.barState === "working"
                width: Style.space(6)
            }
        }
    }
    KeyboardPanel {
        id: panel

        anchorItem: button
        bar: root.bar
        contentHeight: panel.fittedContentHeight(popupContent.implicitHeight, Style.space(540))
        contentWidth: panel.fittedContentWidth(Style.space(360))
        focusTarget: keyCatcher
        open: root.opened
        owner: root
        padding: Style.spacing.popupPadding

        PanelKeyCatcher {
            id: keyCatcher

            anchors.fill: parent

            onActivateRequested: popoverController.activate()
            onCloseRequested: root.close()
            onMoveRequested: function (dx, dy) {
                popoverController.moveFocus(dx + dy);
            }
            onTabRequested: function (direction) {
                root.switchPanel(direction);
            }
            onTextKey: function (key) {
                if (key === "r" || key === "R")
                    snapshot.refresh();
            }

            PopupContent {
                id: popupContent

                accountEmails: root.accountEmails
                anchors.fill: parent
                controller: popoverController
                now: snapshot.now
                preferences: popoverPreferences
                theme: popoverTheme
                view: snapshot.view
            }
        }
    }
}
