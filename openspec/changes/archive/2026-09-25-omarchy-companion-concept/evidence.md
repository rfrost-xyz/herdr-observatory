# Delivery evidence

| Requirement and scenario | Implementation | Verification |
| --- | --- | --- |
| Read-only Omarchy companion; bar to companion | `omarchy/herdr.observatory/{manifest.json,Panel.qml,Companion.qml,SnapshotStore.qml}` | Installed plugin registered and enabled in the running Omarchy shell. Popover rendered on the current monitor; Enter launched one `org.qt-project.qml` window, reported by Hyprland as `floating: false`. The UI only requested the loopback `GET /api/state`. |
| Truthful current state; host loss and missing inference source | `omarchy/herdr.observatory/State.js` | Seven projection tests cover host loss, all-host loss, sample expiry, zero allowance, expired reset, ws-255 GPU scope and disconnected state. Live popover displayed source ages and explicitly unavailable inference use. |
| Desktop fit; narrow tile | `Panel.qml`, `Companion.qml` | Escape closed the focused popover. The companion rendered at a 1,512-pixel wide tile and at a 484-pixel narrow tile in Hyprland. At narrow width its cards formed one scrolling column, with visible thread status and source age. No custom animation was added. |
| Reversible local installation; remove the trial | `omarchy/herdr.observatory/{install.sh,uninstall.sh}` | The installer validated and enabled the plugin. The installed uninstaller removed its directory and bar entry; `jankeesvw.herdr` and the other plugins remained. Reinstallation from this worktree succeeded, verified the shell IPC and left one enabled bar entry. |

Screenshots were inspected locally and excluded from the repository because the
live desktop contains private content. The installed copy is independent of the
development worktree. The separate Rust/Slint comparison and measured inference
request source remain future work, not claims of this change.

The independent implementation review covered commit `225317d` and found no
actionable issues. The four scenarios above trace to seven projection tests,
the Omarchy plugin validator, QML formatting checks and the live desktop checks.
The archived change and canonical specification passed strict OpenSpec validation.
