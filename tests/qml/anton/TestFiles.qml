import QtQuick
import QtCore

// File helpers for the QML tests. Paths live in the private temporary directory
// that run-qml.sh creates (TMPDIR), never in the user's configuration.
QtObject {
    // Writes are asynchronous (synchronous file PUT writes nothing); wait for
    // pending to reach 0, as TestCase.tryCompare(files, "pending", 0) does.
    property int pending: 0
    readonly property string directory: String(StandardPaths.writableLocation(StandardPaths.TempLocation)).replace(/^file:\/\//, "")
    property int serial: 0

    // A fresh file name for one test, unique within the run.
    function path(name) {
        serial++;
        return directory + "/anton-" + serial + "-" + name;
    }
    function read(path) {
        var request = new XMLHttpRequest();
        request.open("GET", "file://" + path, false);
        request.send();
        return request.responseText;
    }
    function write(path, text) {
        var request = new XMLHttpRequest();
        pending++;
        request.onreadystatechange = function () {
            if (request.readyState === XMLHttpRequest.DONE)
                pending--;
        };
        request.open("PUT", "file://" + path);
        request.send(text);
    }
}
