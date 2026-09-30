import QtQuick
import QtTest
import qs.Commons
import "../../../omarchy/herdr.observatory" as Anton

Item {
    id: scene

    height: 10
    width: 10

    TestFiles {
        id: files
    }
    Anton.AntonTheme {
        id: theme
    }
    Component {
        id: themeComponent

        Anton.AntonTheme {}
    }
    TestCase {
        function write(path, text) {
            files.write(path, text);
            tryCompare(files, "pending", 0, 2000);
        }
        function same(a, b) {
            compare(Qt.color(a).toString(), Qt.color(b).toString());
        }
        function test_01_default_path_follows_the_shell_theme() {
            compare(theme.paletteUrl, Color.currentThemePath + "/colors.toml");
            // The stub theme directory has no palette: every status colour falls back.
            same(theme.blue, Color.accent);
            same(theme.cyan, Color.accent);
            same(theme.green, Color.accent);
            same(theme.yellow, Color.accent);
            same(theme.red, Color.urgent);
            same(theme.ink, Color.popups.text);
            same(theme.muted, theme.alpha(theme.ink, 0.62));
            same(theme.line, theme.alpha(theme.ink, 0.12));
        }
        function test_02_palette_parsing_and_partial_fallback() {
            var path = files.path("colors.toml");
            write(path, 'accent = "#123456"\nblue = "#0000aa"\n  green = \'#00aa00\'\nyellow="#aaaa00"\nred = "#aa0000"\ncyan = "bad"\n# comment = "#ffffff"\n');
            var loaded = themeComponent.createObject(scene, {
                paletteUrl: path
            });
            same(loaded.blue, "#0000aa");
            same(loaded.green, "#00aa00");
            same(loaded.yellow, "#aaaa00");
            same(loaded.red, "#aa0000");
            // A malformed value is ignored, so cyan keeps the accent fallback.
            same(loaded.cyan, Color.accent);
            write(path, 'blue = "#0000bb"\n');
            loaded.reload();
            same(loaded.blue, "#0000bb");
            same(loaded.red, Color.urgent);
            same(loaded.green, Color.accent);
            loaded.destroy();
        }
        function test_03_reloads_when_the_shell_accent_changes() {
            var path = files.path("colors.toml");
            write(path, 'green = "#00aa00"\n');
            var loaded = themeComponent.createObject(scene, {
                paletteUrl: path
            });
            same(loaded.green, "#00aa00");
            write(path, 'green = "#00cc00"\n');
            same(loaded.green, "#00aa00");
            var accent = Color.accent;
            Color.accent = "#010203";
            same(loaded.green, "#00cc00");
            write(path, 'green = "#00dd00"\n');
            Color.background = "#020304";
            same(loaded.green, "#00dd00");
            Color.accent = accent;
            Color.background = "#101315";
            loaded.destroy();
        }
        function test_04_state_colours() {
            same(theme.stateColour("working"), theme.yellow);
            same(theme.stateColour("blocked"), theme.red);
            same(theme.stateColour("done"), theme.green);
            same(theme.stateColour("idle"), theme.muted);
            same(theme.stateColour("unknown"), theme.muted);
        }

        name: 'AntonTheme'
    }
}
