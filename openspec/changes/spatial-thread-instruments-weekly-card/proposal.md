# Spatial thread instruments and weekly card

## Why

The first visual thread layout still packs redundant token text into overlapping rows. Context can collide with session values, the specific tool reads as a competing badge, and cache composition is hard to interpret. The weekly card has a narrow bar and cramped pace prose despite unused width.

## What changes

- Make each thread's measured context, session tokens and cache hit rate three distinct visual instruments that adapt to card width and height.
- Keep a reported tool name inside the activity summary at lower emphasis. Show last-response counters only when session totals are unavailable, with their scope explicit.
- Let the weekly bar and observed activity chart span the account panel. Express pace through the relative positions of allowance fill and the time marker; retain precise pace in accessible detail.
- Keep source scope, stale ages, unknown states and eight-card full-screen fit.

## Modified capabilities

- `activity-dashboard`: spatial thread metrics and tool summary.
- `account-allowances`: wider weekly and activity instruments with visual pace.
- `office-display`: eight-card fit with three non-overlapping instruments.

## Impact

Browser rendering and CSS, focused UI and account tests, and OpenSpec presentation requirements. No telemetry collection, account sharing or service protocol changes.
