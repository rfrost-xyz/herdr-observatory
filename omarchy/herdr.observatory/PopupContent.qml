import QtQuick
import QtQuick.Controls
import qs.Commons
import "State.js" as State

Item {
    id: popup

    // Verified emails keyed provider:id, id or label (the .accounts.json map).
    property var accountEmails: ({})
    readonly property real allowanceHeight: preferences.allowancesCollapsed ? 0 : Math.min(allowanceRows.implicitHeight, Style.space(180), Math.max(0, availableBodyHeight - reservedThreadHeight))
    readonly property alias allowanceViewport: allowanceFlick
    readonly property real availableBodyHeight: Math.max(0, height - Style.space(95) - (navigationNotice.visible ? navigationNotice.implicitHeight : 0))
    readonly property bool moving: threadFlick.moving || allowanceFlick.moving
    readonly property bool animate: controller.opened && controller.motionEnabled
    required property AntonController controller
    // The display instant for time-derived readings (SnapshotStore.now).
    property double now: 0
    required property AntonPreferences preferences
    readonly property int reporting: view.hosts.filter(function (h) {
        return h.reporting;
    }).length
    readonly property real reservedThreadHeight: preferences.threadsCollapsed ? 0 : Math.min(threadContent.implicitHeight, Style.space(85), availableBodyHeight * 0.55)
    readonly property alias threadContent: threadContent
    readonly property alias threadViewport: threadFlick
    required property AntonTheme theme
    readonly property alias tooltip: tip
    required property var view

    function email(account) {
        return accountEmails[State.accountKey(account)] || accountEmails[account.id] || accountEmails[account.label] || "";
    }

    implicitHeight: Style.space(95) + (preferences.threadsCollapsed ? 0 : Math.min(threadContent.implicitHeight, Style.space(300))) + (preferences.allowancesCollapsed ? 0 : Math.min(allowanceRows.implicitHeight, Style.space(180))) + (navigationNotice.visible ? navigationNotice.implicitHeight : 0)

    AntonToolTip {
        id: tip

        host: popup
        moving: popup.moving
        theme: popup.theme
    }
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
                theme: popup.theme
                y: Style.space(4)
            }
            Row {
                anchors.right: parent.right
                spacing: Style.space(3)
                y: Style.space(1)

                Repeater {
                    model: ["idle", "blocked", "working", "done"]

                    AntonSurface {
                        id: filter

                        readonly property bool enabledState: popup.preferences.hiddenStates.indexOf(modelData) < 0
                        required property string modelData

                        Accessible.checkable: true
                        Accessible.checked: enabledState
                        Accessible.name: modelData
                        Accessible.role: Accessible.CheckBox
                        height: Style.space(25)
                        hint: (enabledState ? "Hide " : "Show ") + modelData
                        objectName: "filter-" + modelData
                        restingOpacity: enabledState ? 0.1 : 0
                        animate: popup.animate
                        theme: popup.theme
                        tint: popup.theme.stateColour(modelData)
                        tooltip: tip
                        width: Style.space(25)

                        Accessible.onToggleAction: popup.controller.toggleList("hiddenStates", modelData)

                        AntonText {
                            anchors.centerIn: parent
                            color: filter.enabledState ? filter.tint : popup.theme.muted
                            font.bold: true
                            font.pixelSize: Style.space(15)
                            opacity: filter.enabledState ? 1 : 0.35
                            text: filter.modelData === "idle" ? "Ⅱ" : filter.modelData === "blocked" ? "!" : filter.modelData === "working" ? "◌" : "✓"
                            theme: popup.theme
                        }
                        TapHandler {
                            onTapped: popup.controller.toggleList("hiddenStates", filter.modelData)
                        }
                        HoverHandler {
                            cursorShape: Qt.PointingHandCursor
                        }
                    }
                }
            }
            Rectangle {
                anchors.bottom: parent.bottom
                color: popup.theme.line
                height: 1
                width: parent.width
            }
        }
        SectionHeader {
            animate: popup.animate
            collapsed: popup.preferences.threadsCollapsed
            theme: popup.theme
            title: "THREADS"
            tooltip: tip
            width: parent.width

            onToggled: popup.controller.toggleList("collapsedSections", "threads")

            AntonText {
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                color: popup.theme.yellow
                font.pixelSize: Style.font.caption
                objectName: "fleet-discovery-note"
                text: popup.view.discoveryLabel || ""
                theme: popup.theme
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
            height: popup.preferences.threadsCollapsed ? 0 : Math.max(0, Math.min(threadContent.implicitHeight, popup.availableBodyHeight - popup.allowanceHeight))
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
                        model: popup.preferences.threadsCollapsed ? 0 : popup.controller.threadGroups.length

                        Column {
                            id: hostGroup

                            readonly property var group: popup.controller.threadGroups[index]
                            required property int index

                            spacing: Style.space(3)
                            width: popup.width

                            AntonSurface {
                                Accessible.name: hint
                                Accessible.role: Accessible.Button
                                height: Style.space(21)
                                hint: (hostGroup.group.collapsed ? "Expand " : "Collapse ") + hostGroup.group.host.name + " · " + hostGroup.group.host.connectionLabel
                                objectName: "machine-" + hostGroup.group.host.id
                                animate: popup.animate
                                theme: popup.theme
                                tint: popup.theme.ink
                                tooltip: tip
                                width: parent.width

                                Accessible.onPressAction: popup.controller.toggleList("collapsedHosts", hostGroup.group.host.id)

                                Rectangle {
                                    anchors.verticalCenter: parent.verticalCenter
                                    color: ["connecting", "setup_needed"].indexOf(hostGroup.group.host.connectionState) >= 0 ? popup.theme.yellow : hostGroup.group.host.reporting ? popup.theme.green : popup.theme.red
                                    height: width
                                    width: Style.space(4)
                                    x: Style.space(22)
                                }
                                AntonText {
                                    anchors.verticalCenter: parent.verticalCenter
                                    color: popup.theme.muted
                                    font.pixelSize: Style.space(13)
                                    text: hostGroup.group.collapsed ? "›" : "⌄"
                                    theme: popup.theme
                                    x: Style.space(4)
                                }
                                TapHandler {
                                    onTapped: popup.controller.toggleList("collapsedHosts", hostGroup.group.host.id)
                                }
                                HoverHandler {
                                    cursorShape: Qt.PointingHandCursor
                                }
                                AntonText {
                                    id: machineName

                                    anchors.verticalCenter: parent.verticalCenter
                                    color: popup.theme.muted
                                    font.pixelSize: Style.font.caption
                                    text: hostGroup.group.host.name
                                    theme: popup.theme
                                    width: Math.min(implicitWidth, parent.width * 0.6)
                                    x: Style.space(32)
                                }
                                Rectangle {
                                    anchors.verticalCenter: parent.verticalCenter
                                    color: popup.theme.line
                                    height: 1
                                    width: Math.max(0, parent.width - x - machineNote.implicitWidth - Style.space(10))
                                    x: machineName.x + machineName.width + Style.space(8)
                                }
                                AntonText {
                                    id: machineNote

                                    anchors.right: parent.right
                                    anchors.rightMargin: Style.space(5)
                                    anchors.verticalCenter: parent.verticalCenter
                                    color: popup.theme.muted
                                    font.pixelSize: Style.font.caption
                                    objectName: "machine-note-" + hostGroup.group.host.id
                                    text: !hostGroup.group.host.reporting ? hostGroup.group.host.connectionLabel : hostGroup.group.total === 0 ? "No threads" : hostGroup.group.collapsed ? String(hostGroup.group.matching) : hostGroup.group.matching === 0 ? "Filtered" : ""
                                    theme: popup.theme
                                }
                            }
                            Repeater {
                                model: hostGroup.group.indices.length

                                ThreadCard {
                                    required property int index

                                    contentItem: threadContent
                                    controller: popup.controller
                                    entry: popup.view.threads[hostGroup.group.indices[index]] || {}
                                    focused: popup.controller.focusedKey !== "" && popup.controller.focusedKey === State.threadKey(entry)
                                    last: index === hostGroup.group.indices.length - 1
                                    now: popup.now
                                    theme: popup.theme
                                    tooltip: tip
                                    viewport: threadFlick
                                    width: popup.width
                                }
                            }
                        }
                    }
                }
                AntonText {
                    color: popup.theme.muted
                    height: Style.space(36)
                    text: popup.reporting > 0 ? "No current threads" : "Waiting for thread sources"
                    theme: popup.theme
                    verticalAlignment: Text.AlignVCenter
                    visible: !popup.preferences.threadsCollapsed && popup.view.hosts.length === 0
                    width: parent.width - x
                    x: 0
                }
            }
        }
        AntonText {
            id: navigationNotice

            color: popup.theme.red
            elide: Text.ElideNone
            font.pixelSize: Style.font.caption
            text: popup.controller.navigationError
            theme: popup.theme
            visible: popup.controller.navigationError.length > 0
            width: parent.width
            wrapMode: Text.Wrap
        }
        SectionHeader {
            animate: popup.animate
            collapsed: popup.preferences.allowancesCollapsed
            objectName: "allowances-section"
            theme: popup.theme
            title: "ALLOWANCES"
            tooltip: tip
            width: parent.width

            onToggled: popup.controller.toggleList("collapsedSections", "allowances")
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
                    model: popup.preferences.allowancesCollapsed ? 0 : popup.controller.providers.length

                    Column {
                        id: providerGroup

                        readonly property bool collapsed: popup.preferences.collapsedProviders.indexOf(provider.id) >= 0
                        required property int index
                        readonly property var provider: popup.controller.providers[index]

                        width: popup.width

                        AntonSurface {
                            Accessible.name: hint
                            Accessible.role: Accessible.Button
                            height: Style.space(21)
                            hint: (providerGroup.collapsed ? "Expand " : "Collapse ") + providerGroup.provider.label
                            objectName: "provider-" + providerGroup.provider.id
                            animate: popup.animate
                            theme: popup.theme
                            tint: popup.theme.ink
                            tooltip: tip
                            width: parent.width

                            Accessible.onPressAction: popup.controller.toggleList("collapsedProviders", providerGroup.provider.id)

                            AntonText {
                                anchors.verticalCenter: parent.verticalCenter
                                color: popup.theme.muted
                                font.pixelSize: Style.space(13)
                                text: providerGroup.collapsed ? "›" : "⌄"
                                theme: popup.theme
                                x: Style.space(4)
                            }
                            AntonText {
                                id: providerName

                                anchors.verticalCenter: parent.verticalCenter
                                color: popup.theme.muted
                                font.pixelSize: Style.font.caption
                                text: providerGroup.provider.label
                                theme: popup.theme
                                x: Style.space(22)
                            }
                            Rectangle {
                                anchors.verticalCenter: parent.verticalCenter
                                color: popup.theme.line
                                height: 1
                                width: Math.max(0, parent.width - x - Style.space(5))
                                x: providerName.x + providerName.width + Style.space(8)
                            }
                            TapHandler {
                                onTapped: popup.controller.toggleList("collapsedProviders", providerGroup.provider.id)
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

                                    aliasName: popup.controller.accountAlias(entry)
                                    email: popup.email(entry)
                                    entry: providerGroup.provider.accounts[index]
                                    motionEnabled: popup.controller.motionEnabled
                                    namesHidden: popup.preferences.namesHidden
                                    now: popup.now
                                    opened: popup.controller.opened
                                    theme: popup.theme
                                    tooltip: tip
                                    width: popup.width

                                    onIdentityToggled: popup.controller.toggleIdentity()
                                }
                            }
                        }
                    }
                }
                AntonText {
                    color: popup.theme.muted
                    height: Style.space(30)
                    text: "No allowance accounts"
                    theme: popup.theme
                    visible: !popup.preferences.allowancesCollapsed && popup.controller.providers.length === 0
                    width: parent.width
                }
            }
        }
    }
}
