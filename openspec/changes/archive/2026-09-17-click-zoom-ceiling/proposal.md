## Why

A click in the viewer zooms the image to *cover* its space (`viewer-chrome-and-zoom` D7): the
larger of the two axis ratios. For a portrait image in a landscape window that is the ratio that
fills the width, several times the fit — the owner's words (2026-09-17): "cover size is
sometimes too large for portrait images". Landscape images cover at a modest step and are fine.
Requirements §6 (lightbox); raised in the backlog after `viewer-edge-to-edge`, whose non-goals
deferred it for a place to live and a model.

## What Changes

- The click's target gains a **ceiling, as a multiple of the fit**: the image zooms to the cover
  or to the ceiling times the fit, whichever is smaller. The twice-the-fit fallback for an image
  whose aspect matches the window's is capped the same way.
- The ceiling is a **stored preference**, kept with the theme and the thumbnail size, set by a
  slider on the settings screen: 1.25× to 3.5× in steps of 0.25, default 1.5× (design D1 records
  the first numbers and the trial that moved them).
- The wheel's and the pinch's own ceiling (eight times the fit) does not move: the click is the
  one-step gesture, the wheel is the user choosing a size by hand.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `library-browse`: the Lightbox requirement's click sentence takes the ceiling; the "Zoom to
  natural size" and "A small image zooms to twice the fit" scenarios are restated at the default
  ceiling, and a portrait scenario is added.
- (The setting itself is stated in that requirement, the way the grid requirement states the
  thumbnail size: no `app-frame` delta.)

## Non-goals

- A control inside the viewer: the viewer draws no chrome (`viewer-chrome-and-zoom`, amended by
  `browse-feedback`), and the inspect panel describes the image, not the view. If the settings
  screen proves too far from the picture, a second slider at the foot of the viewer's panel is a
  later change — the thumbnail size already has two controls on one setting, so there is a
  precedent either way.
- A target in natural pixels ("fit → natural"): the complaint is about the image against the
  window, not against its own pixel count; a pixel target would make a 600-pixel-wide capture
  and a 4000-pixel one behave differently under the same click.
- Changing the wheel ceiling `ZOOM_MAX`.

## Impact

- `packages/shared/src/index.ts`: `AppSettings.clickZoomCeilingPercent` and its three constants.
- Rust: `model.rs` (field, constants, wire test), `settings.rs` (key, load, save, round-trip
  test), `commands.rs` (`set_click_zoom_ceiling_percent`, clamped, with its test), `lib.rs`
  (registration).
- Webview: `api/commands.ts` + `settings.svelte.ts` (one method each, with their tests and the
  fixtures that list every settings field), `routes/settings/+page.svelte` (the slider),
  `LibraryScreen.svelte` (the prop into the viewer), `Lightbox.svelte` (reads the prop),
  `viewer-zoom.ts` + test (`clickTarget` takes the ceiling).
- `settings.json` gains one key; a file without it reads as the default, like every other key.
