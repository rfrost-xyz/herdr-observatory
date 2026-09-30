pragma Singleton
import QtQml

// Test stub for the Quickshell singleton: environment lookups return "".
QtObject {
    function env(name: string): string {
        return "";
    }
}
