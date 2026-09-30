pragma Singleton
import QtQuick

QtObject {
    property var font: ({
            family: 'monospace',
            title: 15,
            bodySmall: 11,
            caption: 10
        })
    property var spacing: ({
            popupPadding: 10
        })

    function space(value) {
        return value;
    }
}
