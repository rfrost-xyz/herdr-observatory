import QtQuick
import Quickshell.Io
import qs.Commons
import "State.js" as State

// Popover colours and typeface. The status palette comes from colors.toml in
// the theme directory the shell reports as current; a missing colour falls
// back to the shell accent (or the urgent colour for red). Every colour is a
// plain property with a default binding, so tests can assign fixed values.
QtObject {
    id: theme

    property color blue: palette.blue || Color.accent
    property color cyan: palette.cyan || Color.accent
    property string face: ""
    property color green: palette.green || Color.accent
    property color ink: Color.popups.text
    property color line: alpha(ink, 0.12)
    property color muted: alpha(ink, 0.62)
    property var palette: ({})
    readonly property FileView paletteFile: FileView {
        path: theme.paletteUrl
        printErrors: false
        watchChanges: true

        onFileChanged: reload()
        onLoaded: theme.parsePalette(text())
    }
    property string paletteUrl: Color.currentThemePath + "/colors.toml"
    property color red: palette.red || Color.urgent
    readonly property Connections shellColours: Connections {
        function onAccentChanged() {
            theme.reload();
        }
        function onBackgroundChanged() {
            theme.reload();
        }

        target: Color
    }
    property color yellow: palette.yellow || Color.accent

    function alpha(colour: color, opacity: real): color {
        return Qt.rgba(colour.r, colour.g, colour.b, opacity);
    }
    function parsePalette(raw: string) {
        var next = {}, lines = String(raw || "").split("\n");
        for (var i = 0; i < lines.length; i++) {
            var match = lines[i].match(/^\s*([A-Za-z_]+)\s*=\s*["'](#[0-9A-Fa-f]{6})["']/);
            if (match)
                next[match[1]] = match[2];
        }
        palette = next;
    }
    function reload() {
        paletteFile.reload();
    }
    function stateColour(state: string): color {
        var name = State.stateColourName(state);
        return name === "yellow" ? yellow : name === "red" ? red : name === "green" ? green : muted;
    }
}
