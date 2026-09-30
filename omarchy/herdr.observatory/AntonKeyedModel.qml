import QtQuick
import "State.js" as State

// A ListModel of unique string keys. sync() applies the minimal remove, insert
// and move edits from State.keyedEdits, so a Repeater keeps each surviving
// key's delegate (and any running animation) instead of recreating it.
ListModel {
    id: model

    function keys(): var {
        var result = [];
        for (var i = 0; i < count; i++)
            result.push(get(i).key);
        return result;
    }
    // Removes the keys that are not in next, keeping the order of the rest.
    // Callers do this before updating the tables delegates read, so no
    // delegate ever sees its key missing from them.
    function retain(next: var) {
        var wanted = ({}), unique = uniqueKeys(next);
        for (var i = 0; i < unique.length; i++)
            wanted[unique[i]] = true;
        for (var j = count - 1; j >= 0; j--) {
            if (wanted[get(j).key] !== true)
                remove(j, 1);
        }
    }
    function sync(next: var) {
        var edits = State.keyedEdits(keys(), uniqueKeys(next));
        for (var i = 0; i < edits.length; i++) {
            var edit = edits[i];
            if (edit.op === "remove")
                remove(edit.index, 1);
            else if (edit.op === "insert")
                insert(edit.index, {
                    key: edit.key
                });
            else
                move(edit.from, edit.to, 1);
        }
    }
    // Repeated keys (for example hosts without an id, all projected to
    // "unknown") get an occurrence suffix, so every row still renders.
    function uniqueKeys(list: var): var {
        var seen = ({}), result = [];
        for (var i = 0; i < list.length; i++) {
            var key = String(list[i]);
            seen[key] = (seen[key] || 0) + 1;
            result.push(seen[key] === 1 ? key : key + "#" + seen[key]);
        }
        return result;
    }
}
