# Local remote-navigation fix evidence

## Reproduction

Root reproduced an unsupported machine API forwarding error from the saved remote Herdr profile while direct SSH collection remained healthy. A plain non-interactive SSH invocation could not find `herdr` on PATH. These are navigation compatibility and executable-resolution failures, not proof that the remote feed is unavailable.

## Ownership and boundary

Root owns `open-thread.py`, navigation tests, installed update and live acceptance. This coordinator owns this change and plugin navigation documentation. Earlier dirty work remains preserved. No collector, layout, remote installation/service, Docker or Git/forge mutation is included.

## Gates and acceptance

Root's direct SSH focus of the actual remote pane succeeded and a fresh snapshot confirmed focus. The remote Herdr is version 0.9.0 while the local client is 0.9.1; the remote executable is at the established user-local location and absent from non-interactive PATH. The final targeted navigation suite contains eight passing tests.

An initial launch check ran from inside Herdr and inherited its nested-session markers, causing Herdr to refuse a nested client. Root identified this as a verification-environment limitation, not a confirmed plugin defect, and repeated navigation with desktop-equivalent environment markers. That check passed. Production scope remains direct SSH focus. Final regressions, independent review and installed remote-view acceptance subsequently passed, as recorded below.

## Launcher selector preservation

Subsequent live inspection identified a distinct real routing defect: the
unseparated Omarchy terminal/Herdr invocation resolved a named launcher alias,
which started bare Herdr and discarded explicit remote/session selectors. Root
confirmed this from the terminal process command line. A supported argument
delimiter now separates the terminal launch route from the explicit Herdr
command. A main-path regression covers selector preservation in the final eight-test
navigation suite. The corrected source remote view was verified and test-client
cleanup is owned by root.

## Frozen-source gates and live source check

The final helper has eight targeted navigation tests, including exact host/session
routing, JSON parameter transport, missing non-interactive PATH, one-attempt
failure, concise exit-code errors, preserved terminal command selectors and
most-recently-focused matching window selection. Root's full source navigation
check succeeded: the exact remote pane was focused, and a matching remote client
existed and was the active window.

Coordinator independently ran the full Python suite: **197 tests passed**. All
eight JavaScript suites passed. Manifest validation, whitespace checks and strict
OpenSpec validation passed (six items). QML is unchanged by this fix.

```sh
python -m unittest discover -s tests -q
node --test tests/test_ui.cjs tests/test_wasm.mjs tests/test_background.mjs tests/test_title.mjs tests/test_music_title.mjs tests/test_pi_hooks.mjs tests/test_allowances.mjs tests/test_omarchy_state.cjs
omarchy-plugin-validate omarchy/herdr.observatory
openspec validate --all --strict
git diff --check
```

Narrow inventory: `omarchy/herdr.observatory/open-thread.py`, its `README.md`,
`tests/test_omarchy_navigation.py`, and this OpenSpec change. Canonical specification
sync was authorised after independent review and installed acceptance.

Independent review approved the frozen implementation with no actionable code
findings and independently reran all eight navigation tests. Review confirmed
quoted static SSH code plus JSON stdin, separate exact target argument, one focus
attempt, unchanged local/recency behaviour and selector-preserving launch.

## Final installed acceptance

Root installed the reviewed navigation helper and README and confirmed both match
source. Installed navigation to the remote host succeeded: the exact remote pane
was focused, the existing matching terminal was raised, and no duplicate window
was created. The shell restarted successfully. Root inspected the live popover,
confirmed its normal layout and absence of the old error banner, and closed it.

Root removed two diagnostic-only bare Herdr clients while preserving user clients.
No worker temporary files were created for this documentation change. No Git or
forge operation, remote installation/service change or Docker mutation occurred.
Only the authorised explicit remote focus operation was performed. Root approved
canonical sync and local archive.

Root rechecked installed helper/README byte parity after acceptance and removed
the temporary live screenshot. Canonical requirement parity and preservation of
all prior scenarios/unrelated requirements were confirmed. Strict validation
passed before archive; all five tasks are complete. No installed files changed
during specification finalisation.

Post-archive strict validation passed all five canonical capabilities. The active
change list is empty, archived delta/canonical parity is exact, metadata is
preserved, all five tasks are checked and whitespace validation passed.
