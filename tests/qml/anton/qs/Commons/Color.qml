pragma Singleton
import QtQuick

// Test stub for the Omarchy qs.Commons Color singleton. currentThemePath points
// at a directory with no palette, so the theme falls back to the accent colours.
QtObject {
    property color accent: "#74b9b0"
    property color background: "#101315"
    readonly property string currentThemePath: "/nonexistent/anton-test-theme"
    property color foreground: "#d7d6cd"
    property var popups: ({
            text: '#d7d6cd'
        })
    property var tooltip: ({
            text: '#eee',
            background: '#111',
            border: '#aaa'
        })
    property color urgent: "#a55555"
}
