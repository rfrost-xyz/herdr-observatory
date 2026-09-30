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

    readonly property var accountAliases: State.parseObject(settings.accountAliases)
    readonly property var acknowledgements: State.parseObject(settings.acknowledgedCompletions)
    readonly property bool allowancesCollapsed: collapsedSections.indexOf("allowances") >= 0
    readonly property var collapsedHosts: State.parseList(settings.collapsedHosts)
    readonly property var collapsedProviders: State.parseList(settings.collapsedProviders)
    readonly property var collapsedSections: State.parseList(settings.collapsedSections)
    readonly property var hiddenStates: State.parseList(settings.hiddenStates)
    // The legacy table migrates the original Personal/Work alias settings.
    readonly property var legacyAliases: ({
            "codex:Personal": settings.personalAlias,
            "codex:Work": settings.workAlias
        })
    required property string location
    readonly property bool namesHidden: settings.namesHidden
    readonly property Core.Settings settings: Core.Settings {
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
        settings.acknowledgedCompletions = JSON.stringify(State.boundAcknowledgements(value));
        settings.setValue("acknowledgedCompletions", settings.acknowledgedCompletions);
        settings.sync();
    }
    // Saved aliases are replaced only when concealing; showing names keeps them.
    function setIdentity(hidden, aliases) {
        if (hidden && aliases) {
            settings.accountAliases = JSON.stringify(aliases);
            settings.setValue("accountAliases", settings.accountAliases);
        }
        settings.namesHidden = hidden;
        settings.setValue("namesHidden", hidden);
        settings.setValue("personalAlias", settings.personalAlias);
        settings.setValue("workAlias", settings.workAlias);
        settings.sync();
    }
    // name is one of hiddenStates, collapsedHosts, collapsedProviders, collapsedSections.
    function toggle(name, value) {
        settings[name] = JSON.stringify(State.toggleListValue(State.parseList(settings[name]), value));
        settings.setValue(name, settings[name]);
        settings.sync();
    }
}
