## Why

The viewer keeps a 24px strip of dark between the image and the window's edge at every zoom
(`viewer-chrome-and-zoom` design D4), so a click beside a zoomed image can still close it. On
the owner's screen the strip makes a covering image read as *contained* — a picture with
its own edges — and hides that the edge is cutting part of it off. Requirements §6 (lightbox).

## What Changes

- The viewer's space is the whole window; the image reaches the window's edges with no gap.
- Closing while the image covers its space is the keys' (Escape, Space); at the fit the dark
  area beside the image still closes it. **BREAKING** for the `library-browse` sentence that
  promised a margin at every zoom.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `library-browse`: the Lightbox requirement drops the margin and its click-to-close-while-zoomed
  promise.

## Non-goals

- A control for the click's zoom size (the owner's slider ask, 2026-09-17): it needs a place to
  live now that the viewer has no chrome, and a decision about what it scales between; raised
  in the backlog for the next release.

## Impact

`Lightbox.svelte` only: the dialog's size and the viewport's inset. No pure module changes.
