import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import qs.Commons
import "State.js" as State

Item {
    id: popup

    readonly property real allowanceHeight: ui.allowancesCollapsed ? 0 : Math.min(allowanceRows.implicitHeight, Style.space(220), Math.max(0, availableBodyHeight - reservedThreadHeight))
    readonly property alias allowanceViewport: allowanceFlick
    readonly property real availableBodyHeight: Math.max(0, height - Style.space(95) - (navigationNotice.visible ? navigationNotice.implicitHeight : 0))
    readonly property bool moving: threadFlick.moving || allowanceFlick.moving
    readonly property real reservedThreadHeight: ui.threadsCollapsed ? 0 : Math.min(threadContent.implicitHeight, Style.space(85), availableBodyHeight * 0.55)
    readonly property alias threadContent: threadContent
    readonly property alias threadViewport: threadFlick
    required property var ui

    implicitHeight: Style.space(95) + (ui.threadsCollapsed ? 0 : Math.min(threadContent.implicitHeight, Style.space(300))) + (ui.allowancesCollapsed ? 0 : Math.min(allowanceRows.implicitHeight, Style.space(220))) + (navigationNotice.visible ? navigationNotice.implicitHeight : 0)

    Column {
        anchors.fill: parent
        spacing: 0

        Item {
            id: heading

            height: Style.space(35)
            objectName: "anton-heading"
            width: parent.width

            AntonText {
                font.bold: true
                font.pixelSize: Style.font.title
                text: "Anton"
                ui: popup.ui
                y: Style.space(4)
            }
            Row {
                anchors.right: parent.right
                spacing: Style.space(3)
                y: Style.space(1)

                Repeater {
                    model: ["idle", "blocked", "working", "done"]

                    AntonSurface {
                        readonly property bool enabledState: ui.hiddenStates.indexOf(modelData) < 0
                        required property string modelData

                        Accessible.checkable: true
                        Accessible.checked: enabledState
                        Accessible.name: modelData
                        Accessible.role: Accessible.CheckBox
                        height: Style.space(25)
                        hint: (enabledState ? "Hide " : "Show ") + modelData
                        objectName: "filter-" + modelData
                        restingOpacity: enabledState ? 0.1 : 0
                        tint: ui.stateColour(modelData)
                        ui: popup.ui
                        width: Style.space(25)

                        Accessible.onToggleAction: ui.toggleList("hiddenStates", modelData)

                        AntonText {
                            anchors.centerIn: parent
                            color: parent.enabledState ? parent.tint : ui.muted
                            font.bold: true
                            font.pixelSize: Style.space(15)
                            opacity: parent.enabledState ? 1 : 0.35
                            text: parent.modelData === "idle" ? "Ⅱ" : parent.modelData === "blocked" ? "!" : parent.modelData === "working" ? "◌" : "✓"
                            ui: popup.ui
                        }
                        TapHandler {
                            onTapped: ui.toggleList("hiddenStates", parent.modelData)
                        }
                        HoverHandler {
                            cursorShape: Qt.PointingHandCursor
                        }
                    }
                }
            }
            Rectangle {
                anchors.bottom: parent.bottom
                color: ui.line
                height: 1
                width: parent.width
            }
        }
        SectionHeader {
            sectionKey: "threads"
            title: "THREADS"
            ui: popup.ui
            width: parent.width

            AntonText {
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                color: ui.yellow
                font.pixelSize: Style.font.caption
                objectName: "fleet-discovery-note"
                text: ui.overview.discoveryLabel || ""
                ui: popup.ui
                visible: text.length > 0
            }
        }
        Flickable {
            id: threadFlick

            boundsBehavior: Flickable.StopAtBounds
            clip: true
            contentHeight: threadContent.implicitHeight
            contentWidth: width
            flickableDirection: Flickable.VerticalFlick
            height: ui.threadsCollapsed ? 0 : Math.max(0, Math.min(threadContent.implicitHeight, popup.availableBodyHeight - popup.allowanceHeight))
            interactive: contentHeight > height
            objectName: "thread-viewport"
            width: parent.width

            ScrollBar.vertical: ScrollBar {
                policy: ScrollBar.AsNeeded
            }

            Column {
                id: threadContent

                width: parent.width

                Column {
                    id: threadRows

                    spacing: Style.space(6)
                    width: parent.width

                    Repeater {
                        model: ui.threadsCollapsed ? 0 : ui.threadGroups.length

                        Column {
                            id: hostGroup

                            readonly property var group: ui.threadGroups[index]
                            required property int index

                            spacing: Style.space(3)
                            width: popup.width

                            AntonSurface {
                                Accessible.name: hint
                                Accessible.role: Accessible.Button
                                height: Style.space(21)
                                hint: (hostGroup.group.collapsed ? "Expand " : "Collapse ") + hostGroup.group.host.name + " · " + hostGroup.group.host.connectionLabel
                                objectName: "machine-" + hostGroup.group.host.id
                                tint: ui.ink
                                ui: popup.ui
                                width: parent.width

                                Accessible.onPressAction: ui.toggleList("collapsedHosts", hostGroup.group.host.id)

                                Rectangle {
                                    anchors.verticalCenter: parent.verticalCenter
                                    color: ["connecting", "setup_needed"].indexOf(hostGroup.group.host.connectionState) >= 0 ? ui.yellow : hostGroup.group.host.reporting ? ui.green : ui.red
                                    height: width
                                    width: Style.space(4)
                                    x: Style.space(22)
                                }
                                AntonText {
                                    anchors.verticalCenter: parent.verticalCenter
                                    color: ui.muted
                                    font.pixelSize: Style.space(13)
                                    text: hostGroup.group.collapsed ? "›" : "⌄"
                                    ui: popup.ui
                                    x: Style.space(4)
                                }
                                TapHandler {
                                    onTapped: ui.toggleList("collapsedHosts", hostGroup.group.host.id)
                                }
                                HoverHandler {
                                    cursorShape: Qt.PointingHandCursor
                                }
                                AntonText {
                                    id: machineName

                                    anchors.verticalCenter: parent.verticalCenter
                                    color: ui.muted
                                    font.pixelSize: Style.font.caption
                                    text: hostGroup.group.host.name
                                    ui: popup.ui
                                    width: Math.min(implicitWidth, parent.width * 0.6)
                                    x: Style.space(32)
                                }
                                Rectangle {
                                    anchors.verticalCenter: parent.verticalCenter
                                    color: ui.line
                                    height: 1
                                    width: Math.max(0, parent.width - x - machineNote.implicitWidth - Style.space(10))
                                    x: machineName.x + machineName.width + Style.space(8)
                                }
                                AntonText {
                                    id: machineNote

                                    anchors.right: parent.right
                                    anchors.rightMargin: Style.space(5)
                                    anchors.verticalCenter: parent.verticalCenter
                                    color: ui.muted
                                    font.pixelSize: Style.font.caption
                                    objectName: "machine-note-" + hostGroup.group.host.id
                                    text: !hostGroup.group.host.reporting ? hostGroup.group.host.connectionLabel : hostGroup.group.total === 0 ? "No threads" : hostGroup.group.collapsed ? String(hostGroup.group.matching) : hostGroup.group.matching === 0 ? "Filtered" : ""
                                    ui: popup.ui
                                }
                            }
                            Repeater {
                                model: hostGroup.group.indices.length

                                ThreadCard {
                                    required property int index

                                    last: index === hostGroup.group.indices.length - 1
                                    threadIndex: hostGroup.group.indices[index]
                                    ui: popup.ui
                                    width: popup.width
                                }
                            }
                        }
                    }
                }
                AntonText {
                    color: ui.muted
                    height: Style.space(36)
                    text: ui.reporting > 0 ? "No current threads" : "Waiting for thread sources"
                    ui: popup.ui
                    verticalAlignment: Text.AlignVCenter
                    visible: !ui.threadsCollapsed && ui.overview.hosts.length === 0
                    width: parent.width - x
                    x: 0
                }
            }
        }
        AntonText {
            id: navigationNotice

            color: ui.red
            elide: Text.ElideNone
            font.pixelSize: Style.font.caption
            text: ui.navigationError
            ui: popup.ui
            visible: ui.navigationError.length > 0
            width: parent.width
            wrapMode: Text.Wrap
        }
        SectionHeader {
            objectName: "allowances-section"
            sectionKey: "allowances"
            title: "ALLOWANCES"
            ui: popup.ui
            width: parent.width
        }
        Flickable {
            id: allowanceFlick

            boundsBehavior: Flickable.StopAtBounds
            clip: true
            contentHeight: allowanceRows.implicitHeight
            contentWidth: width
            flickableDirection: Flickable.VerticalFlick
            height: popup.allowanceHeight
            interactive: contentHeight > height
            objectName: "allowance-viewport"
            width: parent.width

            ScrollBar.vertical: ScrollBar {
                policy: ScrollBar.AsNeeded
            }

            Column {
                id: allowanceRows

                width: parent.width

                Repeater {
                    model: ui.allowancesCollapsed ? 0 : ui.providers.length

                    Column {
                        id: providerGroup

                        readonly property bool collapsed: ui.parseList(ui.preferences.collapsedProviders).indexOf(provider.id) >= 0
                        required property int index
                        readonly property var provider: ui.providers[index]

                        width: popup.width

                        AntonSurface {
                            Accessible.name: hint
                            Accessible.role: Accessible.Button
                            height: Style.space(21)
                            hint: (providerGroup.collapsed ? "Expand " : "Collapse ") + providerGroup.provider.label
                            objectName: "provider-" + providerGroup.provider.id
                            tint: ui.ink
                            ui: popup.ui
                            width: parent.width

                            Accessible.onPressAction: ui.toggleList("collapsedProviders", providerGroup.provider.id)

                            AntonText {
                                anchors.verticalCenter: parent.verticalCenter
                                color: ui.muted
                                font.pixelSize: Style.space(13)
                                text: providerGroup.collapsed ? "›" : "⌄"
                                ui: popup.ui
                                x: Style.space(4)
                            }
                            AntonText {
                                id: providerName

                                anchors.verticalCenter: parent.verticalCenter
                                color: ui.muted
                                font.pixelSize: Style.font.caption
                                text: providerGroup.provider.label
                                ui: popup.ui
                                x: Style.space(22)
                            }
                            Rectangle {
                                anchors.verticalCenter: parent.verticalCenter
                                color: ui.line
                                height: 1
                                width: Math.max(0, parent.width - x - Style.space(5))
                                x: providerName.x + providerName.width + Style.space(8)
                            }
                            TapHandler {
                                onTapped: ui.toggleList("collapsedProviders", providerGroup.provider.id)
                            }
                            HoverHandler {
                                cursorShape: Qt.PointingHandCursor
                            }
                        }
                        Column {
                            topPadding: providerGroup.collapsed ? 0 : Style.space(3)
                            width: parent.width

                            Repeater {
                                model: providerGroup.collapsed ? 0 : providerGroup.provider.accounts.length

                                AllowanceCard {
                                    required property int index

                                    entry: providerGroup.provider.accounts[index]
                                    ui: popup.ui
                                    width: popup.width
                                }
                            }
                        }
                    }
                }
                AntonText {
                    color: ui.muted
                    height: Style.space(30)
                    text: "No allowance accounts"
                    ui: popup.ui
                    visible: !ui.allowancesCollapsed && ui.providers.length === 0
                    width: parent.width
                }
            }
        }
    }
}
