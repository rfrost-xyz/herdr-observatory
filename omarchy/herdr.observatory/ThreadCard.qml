import QtQuick
import QtQuick.Layouts
import qs.Commons
import "State.js" as State

AntonSurface {
    id: threadCard

    property real childFlash: 0
    // The item the viewport scrolls; reveal() maps this card into it.
    property Item contentItem: null
    property real contextFlash: 0
    required property AntonController controller
    property real entrance: 1
    property var entry: ({})
    property real flash: 0
    property bool focused: false
    property bool last: false
    // Time-derived labels read this instant, never the view.
    property double now: 0
    property Flickable viewport: null
    readonly property var usage: entry.usage || {
        "contextPercent": null,
        "inputTokens": null,
        "outputTokens": null,
        "uncachedTokens": null,
        "cachePercent": null,
        "compactions": null,
        "age": null
    }

    function reveal() {
        if (!keyed)
            return;

        if (!viewport || !contentItem)
            return;
        var top = mapToItem(contentItem, 0, 0).y;
        viewport.contentY = Math.max(0, Math.min(Math.max(0, viewport.contentHeight - viewport.height), top < viewport.contentY ? top : Math.max(viewport.contentY, top + height - viewport.height)));
    }
    function stopEffects() {
        stateFlash.stop();
        childrenFlash.stop();
        compactionFlash.stop();
        arrival.stop();
        flash = 0;
        childFlash = 0;
        contextFlash = 0;
        entrance = 1;
    }

    Accessible.name: "Open in Herdr: " + hint
    Accessible.role: Accessible.Button
    height: threadMetrics.y + threadMetrics.implicitHeight + Style.space(8)
    hint: entry.state + " · " + entry.host + " · " + entry.harness + "\n" + (usage.age ? (usage.stale ? "Last reported " : "Reported ") + usage.age : "Usage not reported")
    animate: controller.opened && controller.motionEnabled
    keyed: focused
    opacity: entrance
    restingOpacity: 0
    tint: theme.stateColour(entry.state)
    tooltipSuppressed: contextMetric.hovered || tokenMetric.hovered || cacheMetric.hovered || childMetric.hovered || turnClock.hovered

    transform: Translate {
        y: (1 - threadCard.entrance) * Style.space(5)
    }

    Accessible.onPressAction: threadCard.controller.openThread(State.threadKey(threadCard.entry))
    onKeyedChanged: Qt.callLater(reveal)

    Connections {
        function onMotionEnabledChanged() {
            if (!threadCard.controller.motionEnabled)
                threadCard.stopEffects();
        }
        function onNewThreads(keys) {
            if (!threadCard.controller.opened || !threadCard.controller.motionEnabled || !threadCard.visible)
                return;
            if (keys[State.threadKey(threadCard.entry)])
                arrival.restart();
        }
        function onObservedChange(changes) {
            if (!threadCard.controller.opened || !threadCard.controller.motionEnabled || !threadCard.visible)
                return;
            var change = changes[threadCard.entry.hostId + ":" + threadCard.entry.id];
            if (!change)
                return;

            if (change.state && ["done", "blocked"].indexOf(threadCard.entry.state) >= 0)
                stateFlash.restart();

            if (change.children)
                childrenFlash.restart();

            if (change.compaction)
                compactionFlash.restart();
        }
        function onOpenedChanged() {
            if (!threadCard.controller.opened)
                threadCard.stopEffects();
        }
        function onVisualEpochChanged() {
            threadCard.stopEffects();
        }

        target: threadCard.controller
    }
    NumberAnimation {
        id: arrival

        duration: 300
        easing.type: Easing.OutCubic
        from: 0
        property: "entrance"
        target: threadCard
        to: 1
    }
    NumberAnimation {
        id: compactionFlash

        duration: 450
        easing.type: Easing.OutCubic
        from: 1
        property: "contextFlash"
        target: threadCard
        to: 0
    }
    NumberAnimation {
        id: stateFlash

        duration: 850
        easing.type: Easing.OutCubic
        from: 1
        property: "flash"
        target: threadCard
        to: 0
    }
    NumberAnimation {
        id: childrenFlash

        duration: 700
        easing.type: Easing.OutCubic
        from: 1
        property: "childFlash"
        target: threadCard
        to: 0
    }
    Rectangle {
        anchors.fill: parent
        color: threadCard.tint
        opacity: threadCard.flash * 0.12
    }
    Rectangle {
        anchors.bottom: parent.bottom
        color: threadCard.theme.alpha(threadCard.theme.ink, 0.09)
        height: 1
        visible: !threadCard.last
        width: parent.width - Style.space(10)
        x: Style.space(5)
    }
    Rectangle {
        color: "transparent"
        height: Style.space(25)
        width: parent.width

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: Style.space(5)
            anchors.rightMargin: Style.space(5)
            spacing: Style.space(6)

            ThreadSignal {
                id: threadStatus

                Layout.alignment: Qt.AlignVCenter
                Layout.maximumWidth: Style.space(18)
                Layout.minimumWidth: Style.space(18)
                Layout.preferredHeight: Style.space(18)
                scale: 1 + threadCard.flash * 0.18
                animate: threadCard.animate
                theme: threadCard.theme
                threadState: threadCard.entry.state || "unknown"
            }
            SheenTitle {
                Layout.fillWidth: !checkout.visible
                Layout.maximumWidth: Math.max(0, parent.width - Style.space(26) - (turnClock.visible ? turnClock.width + Style.space(6) : 0)) * (checkout.visible ? 0.62 : 1)
                Layout.preferredWidth: implicitWidth
                active: threadCard.entry.state === "working"
                animate: threadCard.animate
                text: threadCard.entry.project || "Untitled"
                theme: threadCard.theme
                tint: threadCard.tint
            }
            AntonText {
                id: checkout

                Layout.fillWidth: true
                color: threadCard.theme.muted
                font.pixelSize: Style.font.caption
                objectName: "thread-checkout"
                text: threadCard.entry.branch ? "⑂ " + threadCard.entry.branch : threadCard.entry.checkout && threadCard.entry.checkout !== ".bare" && threadCard.entry.checkout.toLowerCase() !== (threadCard.entry.project || "").toLowerCase() ? threadCard.entry.checkout : ""
                theme: threadCard.theme
                visible: text.length > 0
            }
            AntonSurface {
                id: turnClock

                Layout.alignment: Qt.AlignRight | Qt.AlignVCenter
                Layout.minimumWidth: Layout.preferredWidth
                Layout.preferredHeight: Style.space(21)
                Layout.preferredWidth: clockReading.implicitWidth + Style.space(4)
                hint: State.timingHint(threadCard.entry.timing)
                objectName: "thread-turn-clock"
                opacity: threadCard.entry.timing && threadCard.entry.timing.stale ? 0.62 : 1
                tint: threadCard.tint
                theme: threadCard.theme
                tooltip: threadCard.tooltip
                animate: threadCard.animate
                visible: !!threadCard.entry.timing && threadCard.entry.timing.elapsed !== null

                Row {
                    id: clockReading

                    anchors.centerIn: parent
                    spacing: Style.space(3)

                    Canvas {
                        property color tone: threadCard.tint

                        anchors.verticalCenter: parent.verticalCenter
                        height: Style.space(12)
                        width: Style.space(10)

                        onPaint: {
                            var ctx = getContext("2d"), unit = width / 10;
                            ctx.reset();
                            ctx.strokeStyle = tone.toString();
                            ctx.lineWidth = unit;
                            ctx.beginPath();
                            ctx.arc(5 * unit, 7 * unit, 3.5 * unit, 0, Math.PI * 2);
                            ctx.stroke();
                            ctx.beginPath();
                            ctx.moveTo(3.5 * unit, 1 * unit);
                            ctx.lineTo(6.5 * unit, 1 * unit);
                            ctx.moveTo(5 * unit, 1 * unit);
                            ctx.lineTo(5 * unit, 3.5 * unit);
                            ctx.moveTo(5 * unit, 7 * unit);
                            ctx.lineTo(5 * unit, 4.5 * unit);
                            ctx.stroke();
                        }
                        onToneChanged: requestPaint()
                        onWidthChanged: requestPaint()
                    }
                    AntonText {
                        color: threadCard.tint
                        font.pixelSize: Style.font.caption
                        text: State.durationLabel(threadCard.entry.timing ? threadCard.entry.timing.elapsed : null)
                        theme: threadCard.theme
                    }
                }
            }
        }
    }
    GridLayout {
        id: threadMetrics

        columnSpacing: Style.space(5)
        columns: width >= Style.space(235) ? 4 : 2
        rowSpacing: Style.space(4)
        width: parent.width - x - Style.space(5)
        x: Style.space(5)
        y: Style.space(27)

        AntonSurface {
            id: contextMetric

            Layout.fillWidth: true
            Layout.preferredHeight: Style.space(40)
            Layout.preferredWidth: Style.space(60)
            hint: "Context used" + (threadCard.usage.compactions !== null ? " · " + threadCard.usage.compactions + " compactions" : "") + "\n" + (threadCard.usage.age || "Not reported")
            opacity: threadCard.usage.stale ? 0.72 : 1
            tint: threadCard.tint
            theme: threadCard.theme
            tooltip: threadCard.tooltip
            animate: threadCard.animate

            MetricDial {
                anchors.centerIn: parent
                pulse: threadCard.contextFlash
                ratio: threadCard.usage.contextPercent === null ? -1 : threadCard.usage.contextPercent / 100
                reading: State.percentReading(threadCard.usage.contextPercent)
                symbol: threadCard.usage.compactions === null ? "" : "↻ " + threadCard.usage.compactions
                tint: threadCard.tint
                theme: threadCard.theme
            }
        }
        AntonSurface {
            id: tokenMetric

            Layout.fillWidth: true
            Layout.preferredHeight: Style.space(40)
            Layout.preferredWidth: Style.space(60)
            hint: "↓ Input · ↑ Output\n" + (threadCard.usage.age || "Not reported")
            opacity: threadCard.usage.stale ? 0.72 : 1
            tint: threadCard.tint
            theme: threadCard.theme
            tooltip: threadCard.tooltip
            animate: threadCard.animate

            Column {
                anchors.centerIn: parent
                spacing: Style.space(3)

                Row {
                    spacing: Style.space(5)

                    AntonText {
                        color: threadCard.theme.ink
                        text: "↓"
                        theme: threadCard.theme
                    }
                    AntonText {
                        color: threadCard.tint
                        font.bold: true
                        text: State.tokens(threadCard.usage.inputTokens)
                        theme: threadCard.theme
                    }
                }
                Row {
                    spacing: Style.space(5)

                    AntonText {
                        color: threadCard.theme.ink
                        text: "↑"
                        theme: threadCard.theme
                    }
                    AntonText {
                        color: threadCard.tint
                        font.bold: true
                        text: State.tokens(threadCard.usage.outputTokens)
                        theme: threadCard.theme
                    }
                }
            }
        }
        AntonSurface {
            id: cacheMetric

            Layout.fillWidth: true
            Layout.preferredHeight: Style.space(40)
            Layout.preferredWidth: Style.space(60)
            hint: "◇ Uncached input · ↻ Cached input %\n" + (threadCard.usage.age || "Not reported")
            opacity: threadCard.usage.stale ? 0.72 : 1
            tint: threadCard.tint
            theme: threadCard.theme
            tooltip: threadCard.tooltip
            animate: threadCard.animate

            Column {
                anchors.centerIn: parent
                spacing: Style.space(3)

                Row {
                    spacing: Style.space(5)

                    AntonText {
                        color: threadCard.theme.ink
                        text: "◇"
                        theme: threadCard.theme
                    }
                    AntonText {
                        color: threadCard.tint
                        font.bold: true
                        text: State.tokens(threadCard.usage.uncachedTokens)
                        theme: threadCard.theme
                    }
                }
                Row {
                    spacing: Style.space(5)

                    AntonText {
                        color: threadCard.theme.ink
                        text: "↻"
                        theme: threadCard.theme
                    }
                    AntonText {
                        color: threadCard.tint
                        font.bold: true
                        text: threadCard.usage.cachePercent === null ? "—" : threadCard.usage.cachePercent.toFixed(1) + "%"
                        theme: threadCard.theme
                    }
                }
            }
        }
        AntonSurface {
            id: childMetric

            readonly property var completion: threadCard.entry.completion

            Layout.fillWidth: true
            Layout.preferredHeight: Style.space(40)
            Layout.preferredWidth: Style.space(60)
            hint: State.childHint(completion)
            opacity: completion && completion.stale ? 0.62 : 1
            tint: threadCard.tint
            theme: threadCard.theme
            tooltip: threadCard.tooltip
            animate: threadCard.animate

            MetricDial {
                anchors.centerIn: parent
                failureNotch: !!(childMetric.completion && childMetric.completion.outcomes && childMetric.completion.outcomes.failed > 0)
                objectName: "subagent-dial"
                pulse: threadCard.childFlash
                ratio: childMetric.completion && childMetric.completion.total > 0 ? childMetric.completion.done / childMetric.completion.total : -1
                reading: childMetric.completion ? childMetric.completion.total === 0 ? "0" : childMetric.completion.done + "/" + childMetric.completion.total : "—"
                symbol: "⑂"
                symbolTint: threadCard.theme.muted
                tint: threadCard.tint
                theme: threadCard.theme
            }
        }
    }
    TapHandler {
        onTapped: threadCard.controller.openThread(State.threadKey(threadCard.entry))
    }
    HoverHandler {
        cursorShape: Qt.PointingHandCursor
    }
}
