## Context

See proposal.md — Why. The layout (`src/routes/+layout.svelte`) already binds window-level keys
(`zoomKeys`) and already talks to the window (`getCurrentWindow().isFocused()` in
`reclaimKeys`). The top bar is the frame's (`TopBar.svelte`): sidebar trigger, then the
route's toolbar snippet. `$lib/platform` exposes `isMacos`, read once from the attribute
`app.html` stamps. The main window's permissions are in `src-tauri/capabilities/default.json`;
a permission the code calls without is a runtime error, not a build error, so the grant is
part of the unit, not an afterthought.

## Goals / Non-Goals

**Goals:** one place that knows whether the window is full screen; the key and the button both
go through it.

**Non-Goals:** reacting to full screen entered by the OS (the green button) — the button that
would show that state is not drawn on macOS.

## Decisions

### D1. One store, `fullscreen.svelte.ts`, owns the state and the toggle

`fullscreen.active` (`$state`) and `fullscreen.toggle()`: `setFullscreen(!active)` then re-read
`isFullscreen()` into `active` — read back, not assumed, because the OS can refuse. `refresh()`
re-reads it, called from the window's `resize` event by the layout: on Windows the only other
way out of full screen is a resize the app did not make (a display change), and a stale button
is worse than none. Shaped like `zoom.ts` (a module beside it, one webview/window call).

### D2. F11 is bound where the zoom keys are, on every platform

`+layout.svelte`'s window `onkeydown` gains F11 → `fullscreen.toggle()`, `preventDefault` so the
webview's own F11 does nothing. Not guarded by the typing check: F11 types nothing. On macOS
the system may take F11 first (Show Desktop by default); the app still binds it — when the
system does not, it works, and the map says so. The native ⌃⌘F stays.

### D3. The button is the frame's, at the end of the top bar, off macOS

`TopBar.svelte` renders it after the toolbar snippet with `ms-auto shrink-0`, only when
`!isMacos` — the same read that decides the drag region. Icon `maximize-2` / `minimize-2` by
`fullscreen.active`, `aria-pressed` likewise, title "Full screen (F11)". Ghost variant like the
sidebar trigger. It is not part of the route's toolbar snippet because it belongs to the window,
not the screen: it must be there on /settings too.

### D4. Permissions

`core:window:allow-set-fullscreen` and `core:window:allow-is-fullscreen` join the main window's
capability. `cargo build` validates identifiers, so a typo fails the gate.

## Risks / Trade-offs

- [Leaving full screen on Windows blurs the webview like a title-bar drag does] → `reclaimKeys`
  is macOS-only by design (CLAUDE.md); if the keyboard goes dead after F11 on Windows, the
  fix is a one-shot `setFocus()` after `toggle()`, not re-enabling the blur handler. Hand check
  covers it.
- [F11 on macOS is taken by the system] → the map row is still true where it is not; the
  button is the Windows affordance and macOS has the green one.
