import QtQuick
import QtTest
import "../../../omarchy/herdr.observatory" as Anton

// Repeater retention over AntonKeyedModel: a surviving key keeps its delegate
// object across removals of other keys, insertions and moves.
Item {
    id: scene

    height: 200
    width: 200

    Anton.AntonKeyedModel {
        id: keyedModel
    }
    Column {
        Repeater {
            id: repeater

            model: keyedModel

            Rectangle {
                required property int index
                required property string key

                height: 10
                objectName: "row-" + key
                width: 10
            }
        }
    }
    TestCase {
        function items() {
            var result = {};
            for (var i = 0; i < repeater.count; i++)
                result[repeater.itemAt(i).key] = repeater.itemAt(i);
            return result;
        }
        function order() {
            var result = [];
            for (var i = 0; i < repeater.count; i++)
                result.push(repeater.itemAt(i).key);
            return result;
        }
        function test_01_delegates_survive_removal_insertion_and_moves() {
            keyedModel.sync(["a", "b", "c", "d"]);
            var before = items();
            compare(order(), ["a", "b", "c", "d"]);
            // An earlier key disappears.
            keyedModel.sync(["b", "c", "d"]);
            compare(order(), ["b", "c", "d"]);
            verify(items().b === before.b);
            verify(items().d === before.d);
            // Reorder and insert.
            keyedModel.sync(["d", "e", "b", "c"]);
            compare(order(), ["d", "e", "b", "c"]);
            var after = items();
            verify(after.b === before.b);
            verify(after.c === before.c);
            verify(after.d === before.d);
            compare(after.b.index, 2);
            keyedModel.sync([]);
            compare(repeater.count, 0);
        }
        function test_02_repeated_keys_render_every_row() {
            keyedModel.sync(["unknown", "unknown", "x"]);
            compare(order(), ["unknown", "unknown#2", "x"]);
            var first = items()["unknown#2"];
            keyedModel.sync(["x", "unknown", "unknown"]);
            compare(order(), ["x", "unknown", "unknown#2"]);
            verify(items()["unknown#2"] === first);
            keyedModel.sync([]);
        }

        name: 'AntonKeyedModel'
    }
}
