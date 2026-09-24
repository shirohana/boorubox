## Why

The owner wants the file layout settled before a stable release (2026-09-24): one bucket
level, `images/<a1>/<id>.<ext>`, instead of two. The relayout of an existing library is a
one-time cost they accept now, while the app is alpha and they are its only user. Same ask,
same day: thumbnails are rendered at 384 px on the long edge, and at the new 640 px tile cap
(`sidebar-inspector-polish`) they are "too small and blur to see" on a 2K monitor at DPR 1;
the owner asked for a larger edge and a regeneration of the existing cache rather than
lazy replacement. Requirements §7 (folder layout, thumbnails as a derived cache).

## What Changes

- **One bucket level.** `images/<a1>/<id>.<ext>`, `.thumbs/<a1>/<id>.jpg`, the sidecar
  beside its image as before; `<a1>` is the first two characters of the id. The relative
  path in every record is computed from the id, so no row changes.
- **A library opens into the new shape.** The relayout that already lifts a flat library
  into buckets also lifts every `<a1>/<b2>/` file up into `<a1>/` and removes the emptied
  `<b2>` directories, with the same swallow-and-continue rules.
- **Thumbnail edge 768 px.** New thumbnails render at 768 on the long edge.
- **Regenerate thumbnails**, a Settings → Library action: a background pass that re-renders
  every image's thumbnail at the current edge, with progress, stopping when the library is
  switched; the grid shows the new files without a restart.

## Capabilities

### Modified Capabilities

- `library-folder`: the layout sentence and its scenarios; a new requirement for the
  regeneration pass and for what a thumbnail is.

## Non-goals

- Validating ids at the capture door (the `FIXME` in `library.rs` stands).
- A progress tile in the pending band for the regeneration (Settings shows it where it was
  started; the pass is not pending work in the §5 sense).
- Changing the JPEG quality or format.
