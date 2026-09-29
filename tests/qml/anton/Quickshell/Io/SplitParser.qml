import QtQml

// Test stub for Quickshell.Io.SplitParser: tests emit read(data) directly.
QtObject {
    property string splitMarker: "\n"

    signal read(string data)
}
