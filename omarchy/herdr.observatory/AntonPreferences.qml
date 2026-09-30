import QtQuick
import QtCore as Core
import "State.js" as State

// Local popover preferences. The stored names, types and defaults are those of
// the original Panel settings block: JSON text for lists and objects, so the
// existing privacy.ini loads and is written back in the same encoding. Each
// stored string is parsed once into a typed read-only value.
//
// `location` must be supplied when the object is created. The inner Settings
// would otherwise load, and migrate, at Qt's default location.
QtObject {
    id: preferences

    readonly property var accountAliases: State.parseObject(store.accountAliases)
    readonly property var acknowledgements: State.parseObject(store.acknowledgedCompletions)
    readonly property bool allowancesCollapsed: collapsedSections.indexOf("allowances") >= 0
    readonly property var collapsedHosts: State.parseList(store.collapsedHosts)
    readonly property var collapsedProviders: State.parseList(store.collapsedProviders)
    readonly property var collapsedSections: State.parseList(store.collapsedSections)
    readonly property var hiddenStates: State.parseList(store.hiddenStates)
    // The legacy table migrates the original Personal/Work alias settings.
    readonly property var legacyAliases: ({
            "codex:Personal": store.personalAlias,
            "codex:Work": store.workAlias
        })
    required property string location
    readonly property bool namesHidden: store.namesHidden
    // The stored values; tests may assign them directly, as QSettings would load them.
    readonly property Core.Settings settings: Core.Settings {
        id: store

        property string accountAliases: "{}"
        property string acknowledgedCompletions: "{}"
        property string collapsedHosts: "[]"
        property string collapsedProviders: "[]"
        property string collapsedSections: "[]"
        property string hiddenStates: "[]"
        property bool namesHidden: false
        property string personalAlias: "Richard Hendricks"
        property string workAlias: "Jared Dunn"

        location: preferences.location

        Component.onCompleted: {
            if (value("privacyVersion", 0) < 2) {
                namesHidden = value("personalHidden", false) || value("workHidden", false);
                setValue("namesHidden", namesHidden);
                setValue("privacyVersion", 2);
                sync();
            }
        }
    }
    readonly property bool threadsCollapsed: collapsedSections.indexOf("threads") >= 0

    // Keep local acknowledgement storage bounded, even as sessions come and go.
    function setAcknowledgements(value) {
        store.acknowledgedCompletions = JSON.stringify(State.boundAcknowledgements(value));
        store.setValue("acknowledgedCompletions", store.acknowledgedCompletions);
        store.sync();
    }
    // Saved aliases are replaced only when concealing; showing names keeps them.
    function setIdentity(hidden, aliases) {
        if (hidden && aliases) {
            store.accountAliases = JSON.stringify(aliases);
            store.setValue("accountAliases", store.accountAliases);
        }
        store.namesHidden = hidden;
        store.setValue("namesHidden", hidden);
        store.setValue("personalAlias", store.personalAlias);
        store.setValue("workAlias", store.workAlias);
        store.sync();
    }
    // name is one of hiddenStates, collapsedHosts, collapsedProviders, collapsedSections.
    function toggle(name, value) {
        store[name] = JSON.stringify(State.toggleListValue(State.parseList(store[name]), value));
        store.setValue(name, store[name]);
        store.sync();
    }
}
