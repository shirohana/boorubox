## Why

The owner's manual pass over `tags-and-ratings` (its tasks.md group 7, archived
2026-09-08) found eleven defects in the screen the app is used on. Group 7 fixed the ones
that change owned; the rest were deferred, by name, to this change: "card drag hitting the
dropzone, lightbox focus and key handling, Up/Down in the viewer, sidebar edge resize, local
imports stamped with the file's mtime".

None of them is a missing feature. Each is a control that does the wrong thing while looking
right: a drag inside the grid raises the import overlay, the viewer opens without the focus
so its arrows go to the grid behind it, the rating radios move the image while moving between
themselves, the edge of the sidebar shows a resize cursor for a resize that does not exist,
and a file imported today lands somewhere in the middle of a "Newest capture" grid. §6 makes
the app's browsing screen the reference the legacy viewer set; these are the places it does
not meet it yet.

**Depends on:** `app-shell` (the slot map, the keyboard map and `isTypingTarget`) and
`tags-and-ratings` (the rating control, the tile context menu, the grid's group slices), both
archived and implemented. It touches nothing `trash`, `selection-and-bulk`, `auto-tag-rules`
or `booru-upload` owns, except that it appends the next schema migration (design D11).

## What Changes

- **The import dropzone reacts to external drags only** (§6 Sources): the tile stops starting
  an OS drag of its own, so dragging a card no longer raises "Drop images or folders to
  import them". Dropping a file from Finder is unchanged. In-app drag and drop for grouping
  or folders stays out of scope.
- **The viewer takes the focus when it opens** and owns its keys: opening with Space and
  pressing an arrow moves the viewer, not the grid behind it. Its container is not a tab stop,
  so Shift-Tab from `Previous` reaches a control rather than outlining the whole box, and
  Space closes it again from wherever the focus sits inside it.
- **A click on the dark area around the image closes the viewer**, which is what a lightbox
  does everywhere else and the one gesture the current dialog answers with nothing.
- **Up and Down move a grid row in the viewer** — **reverses** `app-shell`'s deferred "Up/down
  in the viewer: decided against" (its tasks 6.6), with the argument recorded in design D5 and
  in the `library-browse` delta.
- **Inspect mode is remembered for the session**: the viewer opens the way it was left, like
  the inspector column beside the grid. Still session state, not a setting.
- **The rating radios keep their keys to themselves**: Left/Right move between the five
  choices without also moving the image, and Tab/Shift-Tab step through them.
- **A click on the card that is already current opens it** — **reverses** the half of
  `app-shell` D9 that made every single click focus-only (design D7). Double click, Enter and
  Space are unchanged.
- **The current card is visibly current**: the focus marking on the grid's tile becomes a ring
  that reads at a glance, without taking anything from the hover overlay.
- **The sidebar's edge stops advertising a resize it does not do** (design D10): it keeps its
  click-to-toggle and loses the resize cursor. The alternative — making it a real width
  resizer — is rejected there, with its argument.
- **A locally imported file is captured when it is imported**, and the file's own modification
  time is kept beside it as a fact and shown in the inspector — **BREAKING** for the IPC
  contract (`ImageRecord` gains `fileModifiedAt`) and the schema (one migration, design D11).
  §6's "local file import (drop files/folders; metadata = filename, mtime, dimensions)" still
  holds in full: the mtime is still read, still stored and now visible; only which column
  receives it changes. The reversal argument is recorded in the `local-file-import` delta.

Nothing in the list turned out to be already fixed. One is not deliverable as stated: bits-ui
documents its toggle group's `rovingFocus` as an either/or ("when enabled, users navigate
between the items using the arrow keys; when disabled, users navigate between the items using
the tab key"), so "arrows *and* Tab" needs the rating control to own the arrow step itself.
Design D3 does that in one place rather than accepting half the request.

## Capabilities

### New Capabilities

None. Every defect here is a requirement the app already has and does not meet, or a small
reversal of one.

### Modified Capabilities

- `library-browse`: the "Lightbox" requirement gains the focus, the key ownership, the row
  step, the remembered inspect mode and the backdrop click, and loses the "a single click
  never opens" absolute; the "Grid shows the library" requirement gains a visible current-card
  marking.
- `local-file-import`: "Metadata comes from the file" — capture time becomes the import time
  and the file's modification time becomes its own recorded, visible fact; "Drop files or
  folders to import" gains the rule that the drop target answers external drags only.
- `app-frame`: "One keyboard map" gains the viewer's `↑` `↓` row and the rule that a binding
  does not fire while the focus is in a control that acts on that key itself; "The frame has
  fixed regions" gains the rule that a control that only toggles the sidebar does not present
  itself as a resize handle.
- `rating`: the rating control is reachable and operable from the keyboard without moving
  anything but the rating.

## Non-goals

- **In-app drag and drop.** Dragging a tile onto a folder, a group or another tile is a
  feature, not a defect; this change only stops the tile starting a drag the OS then reports
  as an incoming file.
- **A resizable sidebar or inspector.** Both widths stay fixed (design D10). A drag-to-resize
  panel needs a persisted width, which would be the third setting in an app whose non-goals
  say only the theme and the tile size are settings (`app-shell`).
- **Sorting by the file's modification time.** The column lands and is shown; a
  `SortField::FileModified` is a later change's one-line addition and is only worth adding
  with a screen that asks for it (design D11).
- **Deduplicating or re-reading mtimes of images already imported.** Rows imported before this
  change keep the capture time they were given; nothing rewrites them (design D12).
- **Reworking the viewer.** No zoom, no pan, no filmstrip, no slideshow. The image, the chrome
  it already has, and the keys it should already have owned.
- **A focus-visible redesign across the app.** Item 6 is an audit of the viewer's own tab
  order, not a pass over every component's focus ring.

## Impact

- `packages/shared/src/index.ts` + `packages/app/src-tauri/src/model.rs` (hand-mirrored, Phase
  1 D11 — one commit, both files): `ImageRecord` gains `fileModifiedAt: number | null`.
- `packages/app/src-tauri`: `db.rs` appends one migration (`images.file_modified_at`);
  `ingest.rs` gains the field on `IngestInput`, in `IMAGE_COLUMNS` and in the insert;
  `import.rs` stamps `captured_at` with the import time and passes the file's mtime as the new
  field. No other Rust changes.
- `packages/app/src`: `routes/+page.svelte` (viewer mode as session state, the grid's column
  count, the dropzone), `components/library/Lightbox.svelte` (focus, keys, backdrop, tab
  order, row step), `components/library/LibraryGrid.svelte` (publishes its column count),
  `components/library/ImageCard.svelte` (no OS drag, click-to-open, focus ring),
  `components/library/Inspector.svelte` (the "File modified" fact),
  `components/tags/RatingControl.svelte` (its own arrow step),
  `components/frame/Sidebar.svelte` (the rail's cursor), `lib/keyboard.ts` (`KEYBOARD_MAP`
  gains the viewer's `↑` `↓`).
- No new dependency and no new shadcn copy-in. No copy-in is edited: the sidebar rail and the
  toggle group are configured from the call site (design D3, D10).
