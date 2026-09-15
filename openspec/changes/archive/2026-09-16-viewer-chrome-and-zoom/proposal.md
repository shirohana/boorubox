## Why

The owner's Windows pass (2026-09-15), on the viewer: "the top bar (title and buttons) takes
the space and makes the image smaller. I want to release the space for the images. The bar can
be overlapped on a corner, small and default hidden. When user clicks the image itself, toggle
shown or hidden." And: "Add a zoom feature. Double click on the image to scale in and move the
mouse to pan … it functions like having a minimap viewport that reflects the small mouse move
to a larger image pan. When mouse moves to the edge, it should keep a margin so user can click
the background to dismiss." The same request was raised in the browse-polish hand check
(2026-09-10) and deferred there by name: "auto-hiding the chrome and toggling it by a click on
the image … moves the close gesture and the header's whole reason for being on screen, which
is more than a hand-check fix."

**Reverses two recorded non-goals**, with the argument: `app-shell` ("Zoom and pan in the
lightbox. Fit-to-viewport only.") and `browse-polish` ("Reworking the viewer. No zoom, no pan,
no filmstrip, no slideshow."). Both were right for what those changes were doing: the first
was building the frame, the second fixing focus and keys in a dialog whose markup it chose not
to touch. What ended them is use: on a Windows laptop the bar costs a visible slice of every
image, and a 4000-pixel capture fitted to 96vh cannot be read at all. §6 names the lightbox as
a Phase 1 deliverable and the legacy viewer as the reference for behaviour; a lightbox that
cannot zoom is below that reference.

**Depends on:** `browse-polish` (the dialog's focus rules, D2/D4), archived; lands after
`inspector-polish` (this delta carries its viewer text).

## What Changes

- **The chrome overlays the image and is hidden by default.** Title, counter and the four
  buttons sit in a small bar in the stage's top-left corner (clear of the traffic lights on
  macOS) over the image, not above it; the image is fitted to the whole stage. A single click
  on the image shows or hides the bar, after the double-click interval so a double click never
  flashes it (the X app's behaviour, the owner's reference). Tab shows it too, so the keyboard
  can reach its buttons.
- **Double click zooms the image to its natural pixel size** — or to twice the fitted size
  when the natural size is not larger than the fit — and double click again returns to fit.
  The mouse wheel steps the scale between fit and a ceiling.
- **While zoomed, the pointer's position pans the image**: the image is drawn inside an inset
  viewport, the pointer's position across that viewport maps to the image's position across
  its overflow, so a small move at the edge shows the far edge. The inset is the margin: a
  strip of the dark stage stays around the picture at every zoom, and a click there closes
  the viewer as it does today.
- **Moving to another image resets the zoom to fit.** Escape closes the viewer as before, at
  any zoom. Space and the click-beside-the-image rule are unchanged.

## Non-goals

- Drag-to-pan, pinch, keyboard zoom keys, rotation, a filmstrip, a slideshow.
- Remembering the chrome's state or the zoom across images or sessions.
- Any change to inspect mode's panel.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `library-browse`: "Lightbox" — the chrome's placement and visibility, zoom and pan, and the
  close gestures at any zoom.

## Impact

- `packages/app/src/lib/components/library/Lightbox.svelte` (markup and gestures), two new
  pure modules with tests beside it: `viewer-zoom.ts` (fit, zoom step, pan mapping) and
  `click-intent.ts` (single click delayed behind a double). No Rust, schema or sidecar change.
