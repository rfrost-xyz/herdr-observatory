import QtQml

// Test stub for Quickshell.Io.FileView. It reads the file synchronously through
// XMLHttpRequest (run-qml.sh sets QML_XHR_ALLOW_FILE_READ) when the path is set
// and on reload(), and emits loaded() only when the file has content.
QtObject {
    id: file

    property string content: ""
    property string path: ""
    property bool printErrors: true
    property int reloads: 0
    property bool watchChanges: false

    signal fileChanged
    signal loadFailed(int error)
    signal loaded

    function reload() {
        reloads++;
        content = "";
        if (path === "") {
            loadFailed(1);
            return;
        }
        var request = new XMLHttpRequest();
        try {
            request.open("GET", "file://" + path, false);
            request.send();
            content = request.responseText || "";
        } catch (error) {
            content = "";
        }
        if (content.length > 0)
            loaded();
        else
            loadFailed(1);
    }
    function text() {
        return content;
    }

    Component.onCompleted: reload()
    onPathChanged: reload()
}
