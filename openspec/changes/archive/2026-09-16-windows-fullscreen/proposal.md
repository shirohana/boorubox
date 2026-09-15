## Why

The owner's Windows pass (2026-09-15): "Can we have a full-screen mode also on Windows? macOS
has built-in full-screen but Windows doesn't." The app has no fullscreen handling of its own; on
macOS the window's green button does it, on Windows nothing does. A grid of thumbnails and a
viewer are what a whole screen is for (§6: browse, lightbox). "Not everyone knows pressing F11
enters full screen mode" — so a visible control, not only a key.

**Depends on:** `app-shell` (the frame and its top bar), archived. Independent of the two
changes ahead of it in the queue except that `inspector-polish` also amends the keyboard map
requirement, so this delta carries that change's text.

## What Changes

- **F11 enters and leaves full screen** on every platform, bound on the window like the zoom
  keys, and listed in the keyboard map on the settings screen.
- **A full-screen button at the end of the top bar** on platforms whose window has no native
  full-screen control — Windows today. On macOS the green traffic light already does this and
  the button is not shown (spec `app-frame`: no control that duplicates one the OS draws in the
  same bar).
- The window's `set-fullscreen` and `is-fullscreen` permissions are granted to the main
  window.

## Non-goals

- A "kiosk" mode, hiding the app's own chrome in full screen: the sidebar and top bar stay.
- Remembering full screen across launches.
- Any change to the viewer's chrome (the fifth change in the queue).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `app-frame`: "One keyboard map" gains F11; "The frame has fixed regions" gains the
  full-screen control on platforms without a native one.

## Impact

- `packages/app/src-tauri/capabilities/default.json`: two `core:window` permissions.
- `packages/app/src/lib/fullscreen.svelte.ts` (new), `src/routes/+layout.svelte` (the key),
  `src/lib/components/frame/TopBar.svelte` (the button), `src/lib/keyboard.ts` (the map row).
