## Why

In edit mode a click applies the active stamp to a tile, and nothing on the tile says what
the image carried before or carries after — the owner had to open the inspector per image to
know whether a click was needed (2026-09-24). Danbooru's edit mode shows the post's tags
under the preview for the same reason. Requirements §6 (Danbooru-style tagging).

## What Changes

- **A tag footer per tile while edit mode is on.** Under each thumbnail, the image's tags in
  the app's category order and colour, three lines, and the whole list while the tile is hovered. Always on in edit mode.
- The grid's row height grows by the footer while a footer is shown, so the windowed rows
  stay exact.
- **A view setting for the footer outside edit mode** (2026-09-24, from the running app: the
  owner found the footer useful for browsing too, not just editing). A toolbar toggle beside
  the thumbnail-size slider turns it on or off for ordinary browsing, kept with the tile size
  in `settings.json` (design D3); edit mode still forces it on and disables the toggle rather
  than hiding it.

## Capabilities

### Modified Capabilities

- `stamps`: "Edit mode applies the active stamp by a click" gains the footer.

## Non-goals

- Editing tags from the footer.
- A tooltip with the full list past the clamp; the inspector is the full list.

## Impact

- Webview: `library/ImageCard.svelte` (the footer), `LibraryGrid.svelte` (a `showTags` prop,
  the footer's height in the row height), `grid-window.ts` (`rowHeight` takes a footer),
  `LibraryScreen.svelte` (derives `showTags` from edit mode and the setting, the toolbar
  toggle), `grid-window.test.ts`.
- The `showTileTags` setting: `src-tauri/settings.rs` and `model.rs` (the key, the default,
  `AppSettings`), `src-tauri/commands.rs` and `lib.rs` (`set_show_tile_tags`), `packages/shared`
  (`AppSettings.showTileTags`), `api/commands.ts` and `api/settings.svelte.ts`
  (`setShowTileTags`).
