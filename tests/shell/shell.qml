import QtQuick
import Quickshell

// Installed-shell harness for the Anton popover (see tests/run-shell-harness.sh).
// It loads the plugin's Panel.qml the way omarchy-shell loads a bar widget
// (asynchronous Qt.createComponent), against the installed qs.Commons and
// qs.Ui modules, and checks that:
// - repeated hot reloads, including a reload while open, create and destroy
//   the widget cleanly;
// - an opened popover has a visible, non-empty card with its thread rows;
// - the popover stays open with no input, across close/reopen cycles and a
//   reopen during the fade-out;
// - nothing asks the runtime to open a thread (the fake runtime logs every
//   invocation and the runner script checks that log).
// It prints HARNESS lines and exits 0 on success, 1 on a failed check.
ShellRoot {
    id: shell

    property var component: null
    property int failures: 0
    readonly property string pluginUrl: "file://" + Quickshell.env("ANTON_PLUGIN_DIR") + "/Panel.qml"
    property var plugin: null
    property int step: 0
    property double stepStarted: Date.now()
    // Each step returns true when done; a step may be polled many times.
    readonly property var steps: [
        // Hot-reload storm: create, optionally open, destroy and clear the
        // component cache, as the shell's plugin reload does.
        () => shell.create(), () => shell.after(300) && shell.destroyPlugin(),
        () => shell.create(), () => shell.open() && shell.after(200) && shell.destroyPlugin(),
        () => shell.create(), () => shell.after(50) && shell.destroyPlugin(),
        () => shell.create(), () => shell.open() && shell.after(400) && shell.destroyPlugin(),
        () => shell.create(), () => shell.after(300) && shell.destroyPlugin(),
        // The widget that stays: wait for the first snapshot, then open.
        () => shell.create(), () => shell.threadsLoaded(), () => shell.open() && shell.after(700), () => shell.checkOpen("first open"),
        // Close and reopen, once after the fade and once during it.
        () => shell.closePanel() && shell.after(400), () => shell.open() && shell.after(700), () => shell.checkOpen("reopen after fade"),
        () => shell.closePanel() && shell.after(60), () => shell.open() && shell.after(700), () => shell.checkOpen("reopen during fade"),
        // Focus moves inside the popover, then it must stay open unattended.
        () => shell.refocus() && shell.after(3000), () => shell.checkOpen("unattended after 3 s"),
        () => shell.finish()]

    function after(ms) {
        return Date.now() - stepStarted >= ms;
    }
    function card() {
        var panel = keyboardPanel();
        var content = panel && panel.contentItem.length ? panel.contentItem[0] : null;
        // contentHolder, then the card surface, own the panel content.
        return content && content.parent ? content.parent.parent : null;
    }
    function check(condition, message) {
        if (!condition) {
            failures++;
            console.log("HARNESS FAIL " + message);
        }
        return condition;
    }
    function checkOpen(label) {
        var panel = keyboardPanel(), surface = card(), content = popupContent();
        check(plugin.opened, label + ": the popover closed without input");
        check(panel && panel.visible, label + ": the panel window is not visible");
        check(surface && surface.visible && surface.opacity === 1 && surface.width > 0 && surface.height > 0, label + ": the card is empty or hidden");
        check(content && content.visible && content.width > 0 && content.height > 0, label + ": the popover content has no size");
        var view = content ? content.view || (content.ui && content.ui.overview) : null;
        var cards = [];
        collect(content, cards);
        check(view && view.threads.length > 0 && cards.length === view.threads.length, label + ": " + cards.length + " visible thread rows for " + (view ? view.threads.length : 0) + " threads");
        cards.forEach(function (row) {
            check(row.width > 0 && row.height > 0, label + ": a thread row has no size");
        });
        console.log("HARNESS " + label + ": card " + (surface ? surface.width + "x" + surface.height : "none") + ", rows " + cards.length);
        return true;
    }
    function closePanel() {
        if (plugin.opened)
            plugin.close();
        return true;
    }
    // Visible thread rows: delegates that carry a thread entry and a controller.
    function collect(item, out) {
        if (!item)
            return;
        if (item.entry && item.entry.hostId !== undefined && item.visible && item.height > 0)
            out.push(item);
        var kids = item.children || [];
        for (var i = 0; i < kids.length; i++)
            collect(kids[i], out);
    }
    function create() {
        if (!component) {
            component = Qt.createComponent(pluginUrl, Component.Asynchronous);
            return false;
        }
        if (component.status === Component.Loading)
            return false;
        if (!check(component.status === Component.Ready, "plugin failed to load: " + component.errorString()))
            return finish();
        plugin = component.createObject(holder, {
            bar: fakeBar
        });
        component = null;
        if (!check(plugin !== null, "plugin failed to instantiate"))
            return finish();
        plugin.anchors.right = holder.right;
        return true;
    }
    function destroyPlugin() {
        plugin.destroy();
        plugin = null;
        // As the shell's reload does, where the engine offers it.
        if (typeof Qt.clearComponentCache === "function")
            Qt.clearComponentCache();
        return true;
    }
    function finish() {
        console.log("HARNESS done, failures " + failures);
        step = steps.length;
        Qt.exit(failures ? 1 : 0);
        return false;
    }
    function keyboardPanel() {
        var data = plugin ? plugin.data : [];
        for (var i = 0; i < data.length; i++)
            if (data[i] && data[i].fittedContentHeight !== undefined)
                return data[i];
        return null;
    }
    function open() {
        if (!plugin.opened)
            plugin.open();
        return true;
    }
    function popupContent() {
        var panel = keyboardPanel();
        var catcher = panel && panel.contentItem.length ? panel.contentItem[0] : null;
        return catcher && catcher.children.length ? catcher.children[0] : null;
    }
    function refocus() {
        var panel = keyboardPanel();
        var catcher = panel && panel.contentItem.length ? panel.contentItem[0] : null;
        var content = popupContent();
        if (content)
            content.forceActiveFocus();
        if (catcher)
            catcher.forceActiveFocus();
        return true;
    }
    function threadsLoaded() {
        var data = plugin.data;
        for (var i = 0; i < data.length; i++)
            if (data[i] && data[i].visualUpdates !== undefined && data[i].view && data[i].view.threads.length > 0)
                return true;
        return after(8000) ? check(false, "no snapshot arrived from the fake runtime") || finish() : false;
    }

    // The subset of the plugin bar API that Panel and KeyboardPanel read.
    QtObject {
        id: fakeBar

        property var activePopout: null
        property int barSize: 26
        property color barForeground: "#cacccc"
        property var clickTargets: []
        property string fontFamily: "monospace"
        property string position: "top"

        function releasePopout(owner) {
            if (activePopout === owner)
                activePopout = null;
        }
        function requestPopout(owner) {
            activePopout = owner;
        }
    }
    FloatingWindow {
        implicitHeight: 30
        implicitWidth: 400
        visible: true

        Item {
            id: holder

            anchors.fill: parent
        }
    }
    Timer {
        interval: 25
        repeat: true
        running: true

        onTriggered: {
            if (shell.step >= shell.steps.length)
                return;
            if (Date.now() - shell.stepStarted > 20000) {
                shell.check(false, "step " + shell.step + " timed out");
                shell.finish();
                return;
            }
            var done = false;
            try {
                done = shell.steps[shell.step]();
            } catch (error) {
                shell.check(false, "step " + shell.step + " threw " + error);
                shell.finish();
                return;
            }
            if (done) {
                shell.step++;
                shell.stepStarted = Date.now();
            }
        }
    }
}
