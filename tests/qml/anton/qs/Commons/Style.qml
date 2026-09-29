pragma Singleton
import QtQuick

QtObject {
    property var font: ({
            title: 15,
            bodySmall: 11,
            caption: 10
        })

    function space(value) {
        return value;
    }
}
