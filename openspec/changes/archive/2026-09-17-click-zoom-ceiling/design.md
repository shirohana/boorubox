## Context

See proposal.md — Why. `viewer-zoom.ts` is pure and tested: `clickTarget(natural, viewport)`
answers the cover, or twice the fit when the cover is not above it; `Lightbox.svelte` calls it
from `toggleZoom` and animates to the answer. The wheel and the pinch clamp to `[fit, ZOOM_MAX ×
fit]` with `ZOOM_MAX = 8`. The one stored preference of this shape is the thumbnail size:
`GRID_TILE_MIN/MAX/DEFAULT` in `model.rs` and again in `packages/shared` ("the webview has the
same three numbers for the slider's own bounds; these are the ones that decide what reaches the
settings file"), a `u32` on `Settings` and `AppSettings`, `set_grid_tile_size` clamping and
never refusing, `settings.rs` reading an out-of-range value as the default, a `Slider` on the
settings screen and one in the toolbar, and three test fixtures that list every field of
`AppSettings` (`commands.test.ts`, `settings.svelte.test.ts`, `model.rs`'s wire test).
`Settings` and `AppSettings` derive `Eq` and `Copy`.

## Goals / Non-Goals

**Goals:** the ceiling reaches `clickTarget` as one more argument, so the arithmetic stays pure
and the tests say what the click does at each ceiling; the preference goes through the tile
size's path with no new machinery; `settings.json` without the key reads as the default.

**Non-Goals:** a control in the viewer; touching the wheel ceiling; a float anywhere in the
settings file.

## Decisions

### D1. The ceiling is stored as a percent of the fit, an integer

`clickZoomCeilingPercent: u32`, `125..=350`, default `150`, slider step `25`. Not a float `1.5`:
`Settings` and `AppSettings` derive `Eq`, which `f64` cannot, and a float in a hand-editable
JSON file invites `2` against `2.0` against `1.9999`. The webview divides by 100 in exactly one
place, the derived value the viewer hands to `clickTarget`; the slider works in percent and its
label prints the multiple ("Click zoom — up to 2× the fit", from `percent / 100`, one decimal at
most). Constants `CLICK_ZOOM_CEILING_MIN/MAX/DEFAULT` sit beside `GRID_TILE_*` in `model.rs` and
in `packages/shared`, with the tile size's own comment: the Rust three decide what reaches the
file, the webview three bound the slider.

*Alternative rejected:* dropping `Eq` for an `f64` field — a derive change on two structs for
one field, and the JSON problem stays.

*Amended the same day, after the owner's trial.* The first numbers were `150..=500`, default
`200`, step `50`: 2× was read off one portrait image ("3 is too big"), and 5× kept the wheel's
reach in sight. Running through the library, 1.5× was the size that read right on most
portraits, nothing above 3.5× was ever wanted, and a step of 0.5 was too coarse to land between
them — so the range narrowed and the step went to a quarter. The label now prints two decimals
at 1.25× and 1.75×; nothing else changes.

### D2. `clickTarget(natural, viewport, ceiling)` — the ceiling is a multiple, and it caps both branches

`min(cover > fit ? cover : 2 × fit, ceiling × fit)`. Capping the twice-the-fit fallback too is
what makes a ceiling below 2 honest: a user who set 1.5× gets 1.5× on every image. At the
default the fallback is unchanged (`min(2 × fit, 2 × fit)`). The cover branch still overflows
one axis only whenever the ceiling holds, because the axis the fit is limited by is the one that
overflows first and the other only overflows past the cover — the spec's portrait scenario.
The `zoomed` question (`displayScale > fit`) is untouched: a click at a ceiling of 1.5 still
lands above the fit, so the second click still returns.

*Alternative rejected:* a ceiling in natural pixels (proposal's non-goals).

### D3. The viewer reads the ceiling as a prop, derived once in `LibraryScreen`

`LibraryScreen` already reads `settings.current?.gridTileSize` for the toolbar; it derives
`clickZoomCeiling = (settings.current?.clickZoomCeilingPercent ?? DEFAULT) / 100` and passes it
to `<Lightbox clickZoomCeiling={…}>`. A `$derived`, not an `$effect` on the store object
(CLAUDE.md: a field read off a wholesale-reassigned store object belongs in `$derived`). The
viewer keeps no copy: a change on the settings screen is live the next time the viewer opens,
without a restart, because the settings store is the one copy every writer answers into.

### D4. One Rust command, clamped, one webview method, no toolbar slider

`set_click_zoom_ceiling_percent(percent: u32)` clamps to the range like `set_grid_tile_size`
("a rejected size would leave the control disagreeing with the setting"). `settings.rs` reads an
out-of-range or missing key as the default, for the tile size's reason. The settings screen gets
the slider under "Default thumbnail size", same `Slider`, same commit-on-release pattern
(`commitTileSize`), reading the stored value back after the write so a clamped answer is what
the slider shows. No toolbar slider (proposal's non-goals).

## Risks / Trade-offs

- [Three test fixtures list every `AppSettings` field and fail typecheck until the new one is
  added] → that is the gate doing its job; the task names them.
- [A user with `ZOOM_MAX`-sized expectations finds 2× small] → the slider goes to 5×; the wheel
  still goes to 8×.
- [The label rounds `150` to "1.5×" and `200` to "2×"] → print `percent / 100` with
  `toString()`; the step of 25 never produces more than two decimals.
