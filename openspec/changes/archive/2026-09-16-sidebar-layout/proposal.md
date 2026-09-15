## Why

The owner's Windows pass (2026-09-15): move the search fields and the sort/group controls out
of the top bar into the left panel, in Danbooru's order — Search, Rating, Tags (filling the
height), Filter, Notes — so the top bar has room for the bulk-editing actions that are coming
(and the collections that follow). §6 names the Danbooru-style tag search and sidebars as the
reference. Today the tag list and the rating pills share one scrolling region, the search sits
in a band that also has to hold the selection's actions, and at a half-screen window the
selection row is what gives.

**Depends on:** `app-shell` (the frame's slot map), `tags-and-ratings` (the sidebar sections),
`notes`, archived. Lands after `inspector-polish` (shared `LibraryScreen.svelte`) and after
`windows-fullscreen` (this delta carries its text for the shared requirement).

## What Changes

- **The sidebar holds the search**: the tag query and the free-text query, stacked, at the top
  of the panel, at the sidebar's smaller text size, with the Clear action beside them.
- **Then Rating, then Tags, which takes the remaining height and scrolls on its own**; the rest
  of the panel never scrolls away.
- **Then a `Filter` section holding the order and the grouping** (the owner's name for it; it
  reads as Danbooru's sidebar), then Notes, nav and the library footer as today.
- **The top bar keeps** the sidebar toggle, the thumbnail size, the inspector toggle, and the
  row that changes with the screen (selection actions, empty trash, import) — and the
  full-screen button from `windows-fullscreen`.
- **`/` reaches the field wherever the sidebar is**: with the sidebar collapsed to its rail the
  shortcut expands it first, then focuses the tag query.

## Non-goals

- Making the sidebar resizable (app-shell D10 stands).
- Changing what any control does: the search, the pills, the tag list, the order and the
  grouping behave as their specs say; only where they sit changes.
- Collections and the account row: their own changes.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `app-frame`: "The frame has fixed regions" names what the sidebar and the toolbar hold, in
  order, and the `/` behaviour on a collapsed sidebar.

## Impact

- `packages/app/src/lib/components/frame/{Sidebar.svelte,frame.svelte.ts}`,
  `packages/app/src/lib/components/library/{LibraryScreen,SearchBar,ViewControls}.svelte`,
  `packages/app/src/lib/components/tags/TagSidebar.svelte`, `packages/app/src/app.css` (the
  sidebar scroll rules). No Rust, schema or sidecar change.
