## Why

The sidebar's Tags list shows every tag of the result, grouped by category and coloured, and
nothing else. With `auto-artist-tag` every capture from X or Pixiv adds an artist tag, so the
artist group grows with every capture and pushes the general and character tags the owner
browses by below the fold. The owner asked for a way to hide a category's rows in the sidebar,
and for the controls to double as the colour legend the sidebar never had (2026-09-23).
Requirements §6 (Danbooru-style tag search, sidebars).

## What Changes

- **Five toggles under the Tags heading**, one per category in the app's one category order
  (artist, copyright, character, general, meta), each drawn in its category's colour — the
  legend. Pressing one hides that category's rows from the sidebar's list; pressing it again
  shows them. A hidden category's toggle is dimmed.
- **A tag the search itself uses is never hidden.** A tag the query includes, excludes or
  names in an or-group stays listed, marked as today, whatever its category, so a term can
  always be taken out from where it is shown.
- **The choice is a preference of this machine**, kept in `settings.json` beside the notes and
  collections folds, and survives a restart. One stored list of hidden categories, empty by
  default.
- **Sidebar only.** The inspector's tags, the tile footers and the editor keep showing every
  category; the counts and the search are untouched.

### A non-goal narrowed

`tag-panel-polish` (archive `2026-09-23-tag-panel-polish/proposal.md`, Non-goals) listed
"Group headings that collapse, or counts per group". That was right when it was written: the
groups were a colour order over a list of a few dozen tags, the labels had just been removed as
noise, and nothing made one category outnumber the rest. What changes is volume: automatic
artist tags (`auto-artist-tag`) make the artist group grow with every capture without the owner
typing one. This change still builds no group headings and no per-group counts — a toggle row
filters the flat list — so the non-goal narrows to exactly that: no headings, no counts per
group; hiding a category is in.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `tag-sidebar`: "The sidebar lists the tags of the current results with counts" — a hidden
  category's rows are left out, except the search's own tags; and a new requirement for the
  five category toggles (hiding, the search's own tags staying, the state surviving a restart,
  every category hidden, the toggles as the legend).

## Non-goals

- Hiding a category anywhere but the sidebar's Tags list: the inspector, tile footers,
  editor, suggestions and pinned chips keep every category. The inspector shows one image's
  tags, a short list the owner reads whole.
- Group headings, collapsing groups, or counts per group (the narrowed `tag-panel-polish`
  non-goal above).
- Any change to the counts Rust returns or to the search: hiding is a view filter, and a hidden
  category's tags still narrow the results when searched.
- A per-library setting: which categories are worth reading is a habit of the person at this
  machine, the same argument that put the collections fold in `settings.json`.
- Searching by category (`arttags:` and friends): that is `category-count-search`.

## Impact

- Webview: `components/tags/TagSidebar.svelte` (the toggle row, the list through a pure
  function), `domain/tag-categories.ts` (that pure function and its tests),
  `api/settings.svelte.ts`, `api/commands.ts` and their tests.
- Shared: `AppSettings` gains `hiddenTagCategories: TagCategory[]` — the first list-valued
  field of that type.
- Rust: `settings.rs` (the key, load with validation, save, tests), `model.rs`
  (`AppSettings` mirror, which loses `Copy`), `commands.rs` (one setter, a test), `lib.rs`
  (registration). No migration, no library data.
