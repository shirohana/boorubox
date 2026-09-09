---
name: drive-app
description: Launch and drive the BooruBox Tauri app from the terminal on macOS — scratch library, screenshots, keystrokes, clicks through the accessibility tree. Use when a change must be seen working in the real app, not just in tests, or when a tasks.md `Hand check:` needs evidence. Never ticks a hand check; those are the owner's.
allowed-tools: Bash, Read
---

# Drive the BooruBox App From the Terminal

What one coordinator learned driving the app for smoke runs. Everything here is macOS;
the process name for System Events is `BooruBox` (tauri.conf.json `productName`).

## Before launching: the owner's window

The dev server pins port 1420 (`packages/app/vite.config.ts`, `devUrl` in tauri.conf.json),
so the owner's dev window and yours cannot coexist. Check first:

```
lsof -nP -iTCP:1420 -sTCP:LISTEN
```

A listener means the owner is running the app. Stop and say so; do not launch a second
instance and do not touch settings.json (a write lands in their next launch).

## Scratch library, restored after

Settings live in `~/Library/Application Support/me.shirohana.boorubox/settings.json`
(tauri-plugin-store, flat JSON; `libraryPath` is the key). The real library must not take a
smoke run's writes, and migrations are one-way, so:

1. `cp settings.json settings.json.bak` in that folder.
2. `cp -R <library> <scratch>` — a plain copy of the folder is a complete library.
3. Point `libraryPath` at the scratch copy, launch with `mise run dev`.
4. After the run, restore `settings.json.bak` over `settings.json`. Not optional.

## Seeing the window

Bounds come from System Events, then `screencapture -x -R x,y,w,h out.png`. Crop with
`sips -c <h> <w> out.png` before viewing: a full 2400×1600 capture costs ~1.5k tokens
and half of it is the terminal overlay.

```
osascript -e 'tell application "System Events" to tell process "BooruBox" to get {position, size} of window 1'
```

A temporary debug label hot-reloaded through Vite (write it into the page, screenshot,
revert) answers a layout question in one round; it found `aspect-ratio: auto` on the tile.

## Keys and clicks

- System Events `keystroke` and `key code` work everywhere, including inside a modal
  `<dialog>`.
- `click at {x, y}` resolves through the accessibility tree and lands on whatever AX element
  owns that point: it pressed the grid tile *behind* a modal dialog every time. Named
  buttons and tiles are AX elements and click fine; an empty region of a dialog is
  unreachable this way.
- A JXA CGEvent click was not delivered (Accessibility trust for the calling process).
  `cliclick` is not installed.
- AX positions from an `entire contents of window 1` walk are in the same coordinate space
  as the page's `getBoundingClientRect` plus the window origin. Use them, not
  window-relative guesses, when a click must land on an element.
- The Import menu's native file dialog can be driven: click the pop-up, arrow + Enter,
  then Cmd+Shift+G, type a path, Enter, Enter.

## What this does not settle

A screenshot proves what rendered, not what the owner sees over a session: a two-click
test could not tell "first click opens the viewer" from "second click opens". Write what
you saw as a finding under the task's `Hand check:` line and leave the box for the owner.
