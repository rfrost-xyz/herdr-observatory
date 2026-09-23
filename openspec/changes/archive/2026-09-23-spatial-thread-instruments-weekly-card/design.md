# Design

The telemetry model retains full source-bound tiles and exact detail. Both React and fallback renderers select up to three visible instruments: context, cumulative session input/output, and validated cumulative cache-read share. When cumulative session counters are absent, a response-only instrument may appear with its last-response scope visible. Unknown cache composition remains unknown, not zero.

The three instruments use bounded, separate regions. A context arc gives percentage of window used; session input/output are paired values with directional glyphs; a cache arc or clear proportional bar gives the measured share. Counts and provenance remain in accessible descriptions. Activity is one line: state-derived action followed by the reported tool name in muted text. Tool observations do not override Herdr's native state.

The weekly card uses an explicit full-width grid column. Allowance fill and remaining-week marker share the same bar. The space between them encodes pace with a subtle theme colour; the exact percentage-point gap remains in the panel's accessible label and hover text. Daily bars expand across the available card width and use only reported dates. The design must hold eight cards at 1280×720, 1280×800, 1920×1080 and 1920×1200, plus narrow tile reachability.
