import QtQml

// Test stub for Quickshell.Io.Process (Quickshell 0.3.1 names). It never
// spawns anything; writes are recorded so tests can assert owner commands.
QtObject {
    readonly property bool antonStub: true
    property var command: []
    property bool running: false
    property bool stdinEnabled: false
    property QtObject stdout: null
    property var writes: []

    signal exited(int exitCode, int exitStatus)

    function write(data) {
        writes = writes.concat([String(data)]);
    }
}
