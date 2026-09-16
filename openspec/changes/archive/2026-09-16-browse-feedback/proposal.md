## Why

The owner's first pass over the 2026-09-16 queue (`sidebar-layout`, `viewer-chrome-and-zoom`,
`collections`, archived the same day) on the built app found fourteen things that read wrong in
use and one crash: the tile's right-click menu throws the moment it opens, so nothing that
change put on it — rating, collections, move to trash — is reachable. The rest are placement
and feel: controls that drift, a field in the wrong region, a section that squashes its
neighbour, a viewer whose zoom and pan are hard to aim. Requirements §6 ("UI: redesign, not
port") names the legacy viewer as the reference for what is missing here, the account rail.

## What Changes

- **Toolbar**: the same three groups as today, rearranged — sidebar toggle, the title-or-URL
  field, the selection's actions centred while there is a selection, then the screen's action,
  the thumbnail size and the inspector toggle held at the right edge on every platform.
- **Sidebar**: search (the tag field alone, one line, the example syntax as its placeholder),
  rating, tags taking the rest of the height, collections, order and grouping, note, navigation.
  The Clear button goes.
- **Collections section**: folds like the note (a stored preference), holds a list of a height
  the user drags, scrolls inside it, lists in name order only, and offers rename and delete on a
  row's context menu instead of a trailing button.
- **Tile**: the context menu opens again; a mark says the image is in at least one collection.
- **Collection menus** (the inspector's Add to…, the toolbar's Collection, the tile's submenu)
  stay on screen when the list is long, scrolling instead.
- **Viewer**: no controls drawn over the image; one click zooms the image to cover its space,
  a second returns it to the fit; the zoom animates; the pointer pans across the middle third
  of the space, not the whole of it; the wheel scales by how far it turned; a pinch zooms.
- **Screens keep their state**: the library's search fields and result, and the inspector's
  shown-or-hidden state, survive a trip to the trash, the import or the settings screen.
- **Account rail**: while grouped by X account, a list of the accounts in the result, largest
  first, beside the grid, each a filter with include and exclude.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `app-frame`: the order of the toolbar and the sidebar (§6 sidebars, §11 UI redesign); the
  inspector's state is session state across screens.
- `library-browse`: the viewer's controls, click, zoom target, pan mapping, wheel and pinch
  (§6 lightbox); the search fields survive a change of screen.
- `collections`: the sidebar section's fold, height, order and row menu; the tile mark; long
  menus.
- `sort-and-group`: grouping by account offers the accounts as filters.

## Non-goals

- Persisting the search query, the sort or the grouping across launches: `sort-and-group`
  keeps them for the session only, and this change extends that to the query and the inspector,
  no further.
- Touch. Pinch here is the trackpad's gesture; a touch screen is out as before.
- A new viewer. The owner has a plan for a more useful full-size view; this change only removes
  the controls that were in the way of it.
- A count on the collection mark: the mark answers "in any collection"; the number is the
  inspector's.
- Reordering the account rail by anything but the result's own group order.

## Impact

Webview only except one stored preference: `packages/app` (`LibraryScreen`, `SearchBar`,
`CollectionsSection`, `ImageCard`, `Inspector`, `SelectionToolbar`, `Lightbox`, `viewer-zoom`,
`tag-utils`, a new session store and a new rail component), `packages/shared` (one settings
field), `packages/app/src-tauri` (`settings.rs`, `commands.rs`, `lib.rs` for that field). No
schema change; sidecar and library-file formats unchanged. `click-intent.ts` and its test are
deleted with the double click.
