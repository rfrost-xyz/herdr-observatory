# Evidence

`web/app.js` implements source-capture deduplication, bounded replacement queues, freshness checks at emission, low-energy SAMPLE/PANE impacts, stronger milestones, rolling text, signal tearing and outgoing glyph fragments clipped below stable thread rows. README distinguishes sampled observations from native events.

37 Python tests and 13 Node tests pass, including duplicate captures, new captures, stale/disconnected queue rejection, backlog replacement and reduced-motion feed. JS syntax, diff checks and strict OpenSpec pass. Chromium 720p fixture exercised full thread listing with an 800ms-old impact and outgoing fragments: states remain readable and fragments stay below the listing. Existing geometry tests cover 720p/1080p. Independent review found and resolved fragment overlap.

Deployment verification follows.
