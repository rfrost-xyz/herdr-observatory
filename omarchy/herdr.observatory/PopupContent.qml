import QtQuick
import QtQuick.Controls
import qs.Commons
import "State.js" as State

Item {
    id: popup

    // Verified emails keyed provider:id, id or label (the .accounts.json map).
    property var accountEmails: ({})
    // Space rules: the allowance list takes up to allowanceCap, but leaves the
    // threads at least min(threadReserve, threadShare of the body); the thread
    // list takes the rest. Unconstrained, the popover asks for up to threadCap
    // of threads plus up to allowanceCap of allowances.
    readonly property real allowanceCap: Style.space(180)
    readonly property real allowanceHeight: preferences.allowancesCollapsed ? 0 : Math.min(allowanceRows.implicitHeight, allowanceCap, Math.max(0, availableBodyHeight - reservedThreadHeight))
    readonly property alias allowanceViewport: allowanceFlick
    readonly property real availableBodyHeight: Math.max(0, height - chromeHeight - noticeHeight)
    // The heading and the two section headers (95 logical pixels).
    readonly property real chromeHeight: headingHeight + 2 * sectionHeight
    readonly property real headingHeight: Style.space(35)
    readonly property bool moving: threadFlick.moving || allowanceFlick.moving
    readonly property bool animate: controller.opened && controller.motionEnabled
    required property AntonController controller
    // The display instant for time-derived readings (SnapshotStore.now).
    property double now: 0
    required property AntonPreferences preferences
    readonly property int reporting: view.hosts.filter(function (h) {
        return h.reporting;
    }).length
    readonly property real noticeHeight: navigationNotice.visible ? navigationNotice.implicitHeight : 0
    readonly property real reservedThreadHeight: preferences.threadsCollapsed ? 0 : Math.min(threadContent.implicitHeight, threadReserve, availableBodyHeight * threadShare)
    readonly property real sectionHeight: Style.space(30)
    readonly property real threadCap: Style.space(300)
    readonly property alias threadContent: threadContent
    readonly property real threadReserve: Style.space(85)
    readonly property real threadShare: 0.55
    readonly property alias threadViewport: threadFlick
    readonly property real threadViewportHeight: preferences.threadsCollapsed ? 0 : Math.max(0, Math.min(threadContent.implicitHeight, availableBodyHeight - allowanceHeight))
    required property AntonTheme theme
    readonly property alias tooltip: tip
    required property var view

    // Host and provider groups by their model key.
    property var groupsByKey: ({})
    property var providersByKey: ({})

    function email(account) {
        return accountEmails[State.accountKey(account)] || accountEmails[account.id] || accountEmails[account.label] || "";
    }
    // Keyed delegates: each Repeater level follows a model of stable keys
    // (host id, thread key, provider id, account key), so a delegate and its
    // running effects stay with its row. The lookup tables are assigned before
    // the models change, and this runs synchronously in the change handler,
    // before the controller's deferred newThreads and observedChange signals.
    function syncHosts() {
        var groups = preferences.threadsCollapsed ? [] : controller.threadGroups;
        var keys = hostModel.uniqueKeys(groups.map(function (group) {
            return group.host.id;
        }));
        var table = {};
        keys.forEach(function (key, index) {
            table[key] = groups[index];
        });
        hostModel.retain(keys);
        groupsByKey = table;
        hostModel.sync(keys);
    }
    function syncProviders() {
        var groups = preferences.allowancesCollapsed ? [] : controller.providers;
        var keys = providerModel.uniqueKeys(groups.map(function (group) {
            return group.id;
        }));
        var table = {};
        keys.forEach(function (key, index) {
            table[key] = groups[index];
        });
        providerModel.retain(keys);
        providersByKey = table;
        providerModel.sync(keys);
    }

    implicitHeight: chromeHeight + (preferences.threadsCollapsed ? 0 : Math.min(threadContent.implicitHeight, threadCap)) + (preferences.allowancesCollapsed ? 0 : Math.min(allowanceRows.implicitHeight, allowanceCap)) + noticeHeight

    Component.onCompleted: {
        syncHosts();
        syncProviders();
    }

    AntonKeyedModel {
        id: hostModel
    }
    AntonKeyedModel {
        id: providerModel
    }
    Connections {
        function onProvidersChanged() {
            popup.syncProviders();
        }
        function onThreadGroupsChanged() {
            popup.syncHosts();
        }

        target: popup.controller
    }
    Connections {
        function onAllowancesCollapsedChanged() {
            popup.syncProviders();
        }
        function onThreadsCollapsedChanged() {
            popup.syncHosts();
        }

        target: popup.preferences
    }
    AntonToolTip {
        id: tip

        host: popup
        moving: popup.moving
        theme: popup.theme
    }
    // A Column, not a ColumnLayout: the layout snaps each child's position to
    // whole pixels, which moves the rows below a fractional viewport height by
    // a pixel. The heights come from the named properties above.
    Column {
        anchors.fill: parent
        spacing: 0

        Item {
            id: heading

            height: popup.headingHeight
            width: parent.width
            objectName: "anton-heading"

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
            height: popup.sectionHeight
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
            height: popup.threadViewportHeight
            width: parent.width
            interactive: contentHeight > height
            objectName: "thread-viewport"

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
                        model: hostModel

                        Column {
                            id: hostGroup

                            // Thread key (with any occurrence suffix) to its view thread, from
                            // the same view the groups' indices were computed from.
                            property var entries: ({})
                            readonly property var group: popup.groupsByKey[key]
                            required property string key

                            // Retire departed rows, then update the table, then add or move rows.
                            function syncThreads() {
                                if (!group)
                                    return;
                                var keys = threadModel.uniqueKeys(group.keys), table = {};
                                for (var i = 0; i < keys.length; i++)
                                    table[keys[i]] = popup.controller.view.threads[group.indices[i]];
                                threadModel.retain(keys);
                                entries = table;
                                threadModel.sync(keys);
                            }

                            Component.onCompleted: syncThreads()
                            onGroupChanged: syncThreads()

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
                            AntonKeyedModel {
                                id: threadModel
                            }
                            Repeater {
                                model: threadModel

                                ThreadCard {
                                    required property int index
                                    required property string key

                                    contentItem: threadContent
                                    controller: popup.controller
                                    entry: hostGroup.entries[key] || {}
                                    focused: popup.controller.focusedKey !== "" && popup.controller.focusedKey === State.threadKey(entry)
                                    last: index === threadModel.count - 1
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
            height: popup.sectionHeight
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
            width: parent.width
            interactive: contentHeight > height
            objectName: "allowance-viewport"

            ScrollBar.vertical: ScrollBar {
                policy: ScrollBar.AsNeeded
            }

            Column {
                id: allowanceRows

                width: parent.width

                Repeater {
                    model: providerModel

                    Column {
                        id: providerGroup

                        // Account key (with any occurrence suffix) to its view row.
                        property var accounts: ({})
                        readonly property bool collapsed: popup.preferences.collapsedProviders.indexOf(key) >= 0
                        required property string key
                        readonly property var provider: popup.providersByKey[key]

                        // Retire departed rows, then update the table, then add or move rows.
                        function syncAccounts() {
                            if (!provider)
                                return;
                            var keys = collapsed ? [] : accountModel.uniqueKeys(provider.keys), table = {};
                            for (var i = 0; i < keys.length; i++)
                                table[keys[i]] = provider.accounts[i];
                            accountModel.retain(keys);
                            accounts = table;
                            accountModel.sync(keys);
                        }

                        Component.onCompleted: syncAccounts()
                        onCollapsedChanged: syncAccounts()
                        onProviderChanged: syncAccounts()

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

                            AntonKeyedModel {
                                id: accountModel
                            }
                            Repeater {
                                model: accountModel

                                AllowanceCard {
                                    required property string key

                                    aliasName: popup.controller.accountAlias(entry)
                                    email: popup.email(entry)
                                    entry: providerGroup.accounts[key] || ({})
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
