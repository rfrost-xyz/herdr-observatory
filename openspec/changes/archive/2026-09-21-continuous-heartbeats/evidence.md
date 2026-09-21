# Acceptance evidence

Runtime commit a24aa91 implements Continuous status heartbeats in web/app.js and web/style.css. The regression in tests/test_ui.cjs covers elapsed phase through 0–4100 ms, inactive states and stale-source removal. Existing CSS disables all animations for reduced motion; absolute thread tracks preserve card geometry.

31 Python tests, six Node tests, JavaScript syntax, strict OpenSpec and diff checks pass. Independent review approved a24aa91 with no actionable findings.

Installed headless Chromium at 1280x720 with synthetic activity measured machine sweep positions 35.75, 339.28, 642.80 and 885.61 pixels on a 1110-pixel track at approximately 118, 1118, 2118 and 2918 ms despite intervening renders. The 18% segment reaches the far edge before wrapping. Thread positions advanced 8.45, 80.30, 152.13 and 209.59 pixels on a 263-pixel track. No document vertical overflow was reported. This verifies computed browser geometry and timing, not the physical Windows monitor.

The local Personal service serves the changed assets. The workstation Work container uses release a24aa91 with the existing configuration and restart policy. Refresh open browsers to load the new JavaScript and CSS. Heartbeats indicate sampled working status, not measured token or tool-call rates. No collector or disclosure changes.
