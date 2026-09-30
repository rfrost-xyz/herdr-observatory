import QtQuick
import QtTest
import "../../../omarchy/herdr.observatory" as Anton

// The preference files are byte copies of tests/fixtures/popover-privacy-*.ini,
// which the baseline Panel settings block wrote. Copies live in run-qml.sh's
// private temporary directory.
Item {
    id: scene

    height: 10
    width: 10

    TestFiles {
        id: files
    }
    Component {
        id: preferencesComponent

        Anton.AntonPreferences {}
    }
    TestCase {
        function copy(fixture) {
            var text = files.read(String(Qt.resolvedUrl("../../fixtures/" + fixture)).replace(/^file:\/\//, ""));
            verify(text.length > 0, fixture + " readable");
            var path = files.path(fixture);
            files.write(path, text);
            tryCompare(files, "pending", 0, 2000);
            compare(files.read(path), text);
            return path;
        }
        function load(path) {
            var preferences = preferencesComponent.createObject(scene, {
                location: "file://" + path
            });
            verify(preferences !== null);
            return preferences;
        }
        function replaced(text, key, value) {
            return text.split("\n").map(function (line) {
                return line.indexOf(key + "=") === 0 ? key + "=" + value : line;
            }).join("\n");
        }
        function test_01_v2_file_loads_typed_values() {
            var path = copy("popover-privacy-v2.ini"), before = files.read(path);
            var preferences = load(path);
            compare(preferences.hiddenStates, ["idle"]);
            compare(preferences.collapsedHosts, ["workstation"]);
            compare(preferences.collapsedProviders, ["synthetic"]);
            compare(preferences.collapsedSections, ["allowances"]);
            compare(preferences.allowancesCollapsed, true);
            compare(preferences.threadsCollapsed, false);
            compare(preferences.namesHidden, true);
            compare(preferences.accountAliases, {
                "codex:one": "Gilfoyle",
                "synthetic:team": "Monica Hall"
            });
            compare(preferences.acknowledgements, {
                "laptop:a": "3:7:laptop:" + Array(65).join("a")
            });
            compare(preferences.legacyAliases, {
                "codex:Personal": "Richard Hendricks",
                "codex:Work": "Jared Dunn"
            });
            preferences.destroy();
            wait(0);
            // Loading an up-to-date file does not rewrite it.
            compare(files.read(path), before);
        }
        function test_02_toggle_round_trip_keeps_the_encoding() {
            var path = copy("popover-privacy-v2.ini"), original = files.read(path);
            var preferences = load(path);
            preferences.toggle("hiddenStates", "done");
            compare(preferences.hiddenStates, ["idle", "done"]);
            compare(files.read(path), replaced(original, "hiddenStates", '"[\\"idle\\",\\"done\\"]"'));
            preferences.toggle("hiddenStates", "done");
            compare(preferences.hiddenStates, ["idle"]);
            compare(files.read(path), original);
            preferences.toggle("collapsedSections", "threads");
            compare(preferences.threadsCollapsed, true);
            compare(files.read(path), replaced(original, "collapsedSections", '"[\\"allowances\\",\\"threads\\"]"'));
            preferences.destroy();
            wait(0);
            var reloaded = load(path);
            compare(reloaded.collapsedSections, ["allowances", "threads"]);
            reloaded.destroy();
        }
        function test_03_acknowledgements_are_bounded() {
            var path = copy("popover-privacy-v2.ini");
            var preferences = load(path), many = {};
            for (var i = 0; i < 257; i++)
                many["host:t" + i] = "1:" + i;
            preferences.setAcknowledgements(many);
            var saved = preferences.acknowledgements;
            compare(Object.keys(saved).length, 256);
            verify(saved["host:t0"] === undefined, "The earliest entry is dropped");
            compare(saved["host:t256"], "1:256");
            preferences.destroy();
            wait(0);
            var reloaded = load(path);
            compare(Object.keys(reloaded.acknowledgements).length, 256);
            reloaded.destroy();
        }
        function test_04_identity_writes_aliases_only_when_concealing() {
            var path = copy("popover-privacy-v2.ini"), original = files.read(path);
            var preferences = load(path);
            preferences.setIdentity(false, {
                "codex:one": "Big Head"
            });
            compare(preferences.namesHidden, false);
            compare(preferences.accountAliases["codex:one"], "Gilfoyle");
            compare(files.read(path), replaced(original, "namesHidden", "false"));
            preferences.setIdentity(true, {
                "codex:one": "Big Head"
            });
            compare(preferences.namesHidden, true);
            compare(preferences.accountAliases, {
                "codex:one": "Big Head"
            });
            compare(files.read(path), replaced(original, "accountAliases", '{\\"codex:one\\":\\"Big Head\\"}'));
            preferences.destroy();
        }
        function test_05_v1_file_migrates_once() {
            var path = copy("popover-privacy-v1.ini");
            var preferences = load(path);
            var migrated = files.read(path);
            verify(migrated.indexOf("privacyVersion=2") >= 0, migrated);
            verify(migrated.indexOf("namesHidden=") >= 0, migrated);
            var hidden = preferences.namesHidden;
            // Either legacy flag set conceals names.
            compare(hidden, true);
            preferences.destroy();
            wait(0);
            // Destruction flushes every stored property, as the baseline did.
            migrated = files.read(path);
            var second = load(path);
            compare(second.namesHidden, hidden);
            // The migration does not run again: the file is unchanged.
            compare(files.read(path), migrated);
            second.destroy();
            wait(0);
            compare(files.read(path), migrated);
        }

        name: 'AntonPreferences'
    }
}
