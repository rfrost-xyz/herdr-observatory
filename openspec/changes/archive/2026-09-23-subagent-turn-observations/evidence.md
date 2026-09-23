# Evidence

| Requirement and scenario | Implementation | Verification |
| --- | --- | --- |
| Parent-turn summary: starts, stops and unrelated hooks | `observatory/telemetry.py` carries bounded `obs_children` under the existing session lock | `test_current_turn_subagent_observations_survive_other_hooks_and_reset` |
| New turn/session and missing baseline | Reporter resets on UserPromptSubmit; `telemetry_from_agent` checks binding and leaves absent or malformed counts unknown | `test_current_turn_subagent_observations_survive_other_hooks_and_reset`, `test_missing_or_malformed_subagent_baseline_remains_unknown`, v1/v2 migration tests |
| Private disclosure | `observatory/probe.py` validates numeric counts and source time; Work feed revalidates `technical` | `test_work_filter_and_feed_resanitise`, `test_subagent_hooks_discard_child_identity_and_content`, 14 of 16 native metadata keys |
| Glanceable parent card and age | `web/app.js`, `web/react-view.jsx`, built `web/react-view.mjs`, `web/style.css` show a separate observed-event row | `test_ui.cjs`: observed counts, independent last-known age, Pi/unknown omission and source-loss clearing. Synthetic local Chromium fixture: eight cards at 1280×720, 1280×800, 1920×1080 and 1920×1200 without document overflow; narrow 580×445 keeps the row readable. |
| Interpretation | `README.md` and `AGENTS.md` explain hook observations, incomplete coverage and parent state | Reviewed against OpenAI Hooks and Subagents documentation; independent adversarial review clean after two UI findings were fixed. |

Repository gates: 124 Python tests passed (with local Unix socket permission), seven Node test files passed, `node --check web/app.js`, bundle rebuilt, and `openspec validate --all --strict` passed. The geometry fixture was local, synthetic and removed after inspection; it is not a live fleet claim.
