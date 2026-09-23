# Design

The top bar is a two-column grid. The project region takes two thirds and contains the project name with an inset branch box beneath it. The branch uses native state colour. The right region takes one third and contains a large glyph for the current Herdr state, with activity and tool beneath. The glyph has an accessible state label; activity retains its own age and tool text. Status words are not rendered in this bar.

The telemetry row stays below the bar. A bottom line puts compact harness, host, pane and model metadata on the left and compaction/hook/usage ages on the right. Missing metadata leaves an empty value rather than a substitute. Both React and fallback renderers use matching structure and text.

Check eight dense cards at 1280×720, 1280×800, 1920×1080 and 1920×1200 without scroll or collisions. At narrow widths the thread region may scroll, but the card's content remains reachable without horizontal page scroll.
