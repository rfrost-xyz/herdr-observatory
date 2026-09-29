# Notion monthly allowance for Anton

This optional Chromium extension reads the monthly AI allowance from the signed-in
browser. It refreshes every five minutes while Chromium runs, and when its toolbar
button is clicked. It does not buy or read purchased credits. The card shows the
percentage used and the reset date. It does not assume a billing-period start for
pacing. After ten minutes without a valid observation, or after the reset, Anton
shows unavailable until a fresh reading arrives.

## Authentication and permissions

The browser owns authentication. The extension uses normal authenticated requests
to `https://app.notion.com/api/v3/getSpaces` and `getCreditRateLimitStatus`. These
are **unsupported Notion web endpoints** and may change. The first verifies the
explicit user/workspace binding; only monthly numeric fields are forwarded.

Permissions are `https://app.notion.com/*`, `nativeMessaging` and `alarms`.
There is no cookies permission, token export, page/content script, arbitrary URL,
web listener, daemon or external messaging surface. Raw identity is used only in
private local setup and bridge messages. The saved observation contains a scoped
identity hash and numeric fields. The normal collector snapshot contains only the
configured label and allowance values. Nothing is forwarded to fleet peers.

The official `ntn` CLI remains managed by mise and keeps its own credentials.
Its public API token was rejected by the web allowance endpoint. This extension
neither reads that token nor changes the CLI login. Browser authentication must
be maintained separately.

## Setup after source review

1. Build and install the reviewed native plugin update using the repository's
   existing installation procedure, preserving `.config.json`, `.accounts.json`,
   privacy state and the installation owner marker. Do not run `install.sh` over
   an existing installation.
2. In Chromium **Profile 1**, open the extensions page, enable Developer mode
   and load this `omarchy/notion-extension` directory unpacked. Review its three
   permissions before enabling it. Copy its extension ID. Keep this directory
   at a stable path; moving an unpacked extension can change its ID.
3. Add the following optional `notion` object to the installed plugin's private
   `.config.json`. Use the intended Notion person's user ID and workspace ID,
   and the extension ID from step 2. `ntn whoami` can identify the person and
   workspace after CLI login; do not use the token's bot ID. Never copy live
   values into the repository.

   ```json
   {
     "notion": {
       "user_id": "11111111-1111-1111-1111-111111111111",
       "workspace_id": "22222222-2222-2222-2222-222222222222",
       "extension_id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
       "label": "Work"
     }
   }
   ```

4. Run the installed `anton-runtime --register-notion-bridge`. This registers
   `world.herdr.notion_allowance` in Chromium's user-local `NativeMessagingHosts`
   directory with only the configured extension origin allowed. It records an
   ownership receipt in the plugin directory. It refuses conflicting files.
5. Sign in to the intended Notion account in Profile 1, then click the extension's
   toolbar button. A `!` badge means setup or source access is unavailable; hover
   for the distinction. Compare the Anton card with Settings > Notion AI > Usage.
   Reloading the plugin configuration uses the existing collector restart path.

Browser closure, logout, wrong workspace, unsupported response, oversized data
and expired monthly windows cannot produce a zero or refill. The extension sends
an unavailable observation on source failure; a broken native bridge expires via
the ten-minute TTL. The receiver rejects older or mismatched observations.

## Removal

Run the installed `anton-runtime --unregister-notion-bridge`, then remove the
extension from Profile 1 and the private `notion` configuration object. The
command removes only an unchanged receipt-owned manifest and its receipt. It
preserves modified or unrecognised files for manual resolution. Plugin uninstall
also unregisters the bridge before retiring the runtime, and removes its owned
`notion.json` and `notion.lock` state. No browser session or CLI credential changes.

## Verification

Run `cargo test --manifest-path omarchy/anton-runtime/Cargo.toml --locked --offline`,
then `node --test tests/test_notion.mjs tests/test_omarchy_state.cjs` and
`tests/run-qml.sh`. Synthetic tests do not establish live endpoint compatibility:
qualify the actual browser profile, native registration, source reset date and
restart before declaring this integration ready.
