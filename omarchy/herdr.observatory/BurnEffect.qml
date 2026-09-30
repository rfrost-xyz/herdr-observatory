import QtQuick

Canvas {
    id: burn

    property bool active: false
    // Particles run only while the popover is open with motion enabled.
    property bool animate: false
    property bool deficit: false
    property real phase: 0
    required property AntonTheme theme
    property color tint

    // Independent clocks avoid the diagonal bands of the old shared phase.
    function random(seed) {
        var n = Math.sin(seed * 127.1 + 311.7) * 43758.5;
        return n - Math.floor(n);
    }

    clip: true
    visible: active && width > 0 && animate

    onPaint: {
        var ctx = getContext("2d");
        ctx.reset();
        if (!visible)
            return;
        // Each particle has its own lifetime and a new position on each respawn.
        var count = Math.min(150, Math.max(18, Math.ceil(width * 0.8)));
        for (var i = 0; i < count; i++) {
            var duration = 0.45 + random(i + 1) * 1.15;
            var clock = phase / duration + random(i + 401), cycle = Math.floor(clock), life = clock - cycle;
            var seed = i * 173 + cycle * 7919;
            var x = random(seed + 17) * width;
            var y = height - 4 - life * (4 + random(seed + 71) * 10);
            var strength = Math.sin(life * Math.PI);
            var bright = Qt.lighter(tint, 1.3 + random(seed + 109) * 0.7);
            ctx.globalAlpha = strength * 0.22;
            ctx.fillStyle = bright.toString();
            ctx.fillRect(x - 2, y - 2, 5, 5);
            ctx.globalAlpha = strength;
            ctx.fillStyle = Qt.tint(bright, theme.alpha(theme.ink, 0.3)).toString();
            ctx.fillRect(x, y, random(seed + 203) > 0.7 ? 2 : 1.3, 1.5);
        }
    }
    onTintChanged: requestPaint()
    onVisibleChanged: {
        if (visible) {
            requestPaint();
        }
    }
    onWidthChanged: requestPaint()

    Timer {
        interval: 33
        repeat: true
        running: burn.visible

        onTriggered: {
            burn.phase = (burn.phase + 0.033) % 1000;
            burn.requestPaint();
        }
    }
}
