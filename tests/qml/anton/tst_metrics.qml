import QtQuick
import QtTest
import "../../../omarchy/herdr.observatory" as Anton

// Measurement only (restructure-anton-popover): counts the ToolTip instances in
// a rendered popover with the standard synthetic fixture (two machines, three
// threads, two allowance accounts) and logs "ANTON_METRIC name=value" lines.
// The fixture wiring may change with the popover; the counted scene must not.
Item {
    id: scene

    height: 600
    width: 388

    FixtureUi {
        id: fixtureUi

        popup: popup
    }
    Anton.PopupContent {
        id: popup

        height: Math.min(540, implicitHeight)
        ui: fixtureUi
        width: 360
    }
    TestCase {
        function agent(id, state) {
            return {
                id: id,
                status: state,
                project: 'Project ' + id,
                branch: 'feature/' + id,
                harness: 'codex',
                technical: {}
            };
        }
        function account(id) {
            return {
                provider: 'codex',
                provider_label: 'Codex',
                account_id: id,
                label: id,
                status: 'available',
                status_text: null,
                plan: null,
                sampled_at: fixtureUi.now / 1000,
                reset_count: 1,
                reset_expires_at: null,
                windows: [{
                        kind: 'weekly',
                        label: 'Weekly',
                        used_percent: 30,
                        resets_at: fixtureUi.now / 1000 + 302400,
                        duration_s: 604800,
                        pacing: true
                    }]
            };
        }
        function host(id, agents) {
            return {
                id: id,
                label: id,
                online: true,
                connection_state: 'connected',
                sampled_at: fixtureUi.now / 1000,
                agents: agents
            };
        }
        function count(object, pattern, seen) {
            if (!object || seen.indexOf(object) >= 0)
                return 0;
            seen.push(object);
            var total = String(object).indexOf(pattern) >= 0 ? 1 : 0;
            var data = object.data || [];
            for (var i = 0; i < data.length; i++)
                total += count(data[i], pattern, seen);
            var children = object.children || [];
            for (var j = 0; j < children.length; j++)
                total += count(children[j], pattern, seen);
            return total;
        }
        function test_rendered_tooltip_instances() {
            fixtureUi.raw = {
                hosts: [host('laptop', [agent('a', 'working'), agent('b', 'done')]), host('workstation', [agent('c', 'idle')])],
                allowances: [account('one'), account('two')]
            };
            wait(40);
            // Matches QQuickToolTip, ToolTip_QMLTYPE and any wrapper type named *ToolTip.
            var tooltips = count(scene, 'ToolTip', []);
            console.log('ANTON_METRIC rendered_tooltip_instances=' + tooltips);
            verify(tooltips > 0);
        }

        name: 'AntonMetrics'
        when: windowShown
    }
}
