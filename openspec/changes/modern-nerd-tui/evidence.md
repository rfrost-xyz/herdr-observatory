# Verification evidence

| Scenario | Implementation | Verification |
| --- | --- | --- |
| Locally rendered glyphs | Bundled JetBrainsMono Nerd Font, explicit browser load, CSS preload and shared canvas face | Pinned SHA-256 test; font HTTP MIME/allowlist test; Chromium font loaded and Nerd glyph present at 720p/1080p; all 37 WASM effects complete with Unicode/Nerd glyph input |
| Font failure and accessibility | Readable labels and ordinary-symbol fallback; unchanged current-state transcript and controls | Font-load rejection UI test; Chromium font endpoint blocked: font false, private-use glyph false, current states/feed remain and no overflow; reduced-motion browser check |
| Modern one-screen composition | Rounded rules, state glyphs, compact metadata, themed row accents and 140x44 grid | Screenshots inspected at 1280x720 and 1920x1080; 44 rows, no overflow, nine queued observations rendered; existing geometry/scroll/source-loss/pause tests |

42 Python tests and 72 Node tests passed, including all 37 Unicode effect fixtures and the bundled font checksum. JavaScript syntax, strict OpenSpec validation and whitespace gates passed. Browser checks used synthetic data, never live private snapshots. Physical office-monitor presentation was not verified.

Independent implementation review approved without actionable findings. README explains the bundled font, fallback and R/L/F legend. Collection, privacy filtering and runtime permissions remain unchanged.
