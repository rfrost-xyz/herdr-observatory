# Evidence

- Reporter previously assumed decimal-only pane IDs. Restarted real sessions can have alphanumeric IDs, causing all reports to be silently discarded before socket lookup.
- Runtime fix `a9a0948`: bounded opaque identifier validation; exact Herdr lookup and native session/source binding unchanged.
- 54 Python tests passed, including alphanumeric full-report round trips and malformed rejection; same suite passed in production image. All 77 UI/WASM tests and syntax checks passed. Strict OpenSpec validation passed. Independent review approved without findings.
- Both authorised hosts run the corrected healthy image. The existing adapters required no reinstall or additional session restart.
- Actual automatic hooks from the restarted session now publish tool-start events with tool/model, and the Personal HTTP view receives that telemetry after normal polling. No synthetic event was used for this live acceptance check.
- This session is classified Personal, so the Work view correctly excludes it. Classification and disclosure rules were not changed.
- Retained `94def4f` rollback with matching previous environment on both hosts; removed superseded `756fe78` images. No new host payloads, services or diagnostic files.
