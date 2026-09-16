## Context

See proposal.md — Why. `Lightbox.svelte` is a `<dialog>` at 96vw × 96vh with the viewport box
`absolute inset-6` inside the stage; `viewer-chrome-and-zoom` design D4 named that inset
`PAN_MARGIN_PX` and argued for it: a click beside a zoomed image closes the view.

## Goals / Non-Goals

**Goals:** the image can reach every window edge; every other close and focus rule holds.

**Non-Goals:** the click zoom's size control (see the proposal's non-goals).

## Decisions

### D1. The dialog is the window and the viewport is the stage

The dialog becomes `h-screen w-screen` (its `m-auto`, `max-*-none` and `p-0` stay), the
viewport `inset-0`. The margin is deleted, not made configurable. This reverses
`viewer-chrome-and-zoom` D4: that decision was right while the bar and the double click were
the model — the margin was the one guaranteed close target at any zoom — and it stopped being
once the click became the zoom and the keys the way out; the owner's own reading of a covered
image (2026-09-17: "looks contained, unable to know there are some part cut by the margin") is
the argument. Escape and Space close at any zoom; at the fit the dark area beside the image
still does, and the inspect panel's `gap-3` between image and panel stays as it was.

## Risks / Trade-offs

- [No mouse-only way to close a covering image] → one click returns it to the fit, where the
  dark area is back; the keys never left.
- [macOS traffic lights over the top-left of the image] → they sat over the stage before too;
  the viewer draws nothing there.
