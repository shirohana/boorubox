## Why

The library can be searched by tag and rating, and there is no way to give an image either.
Phase 1 ships the query language, the SQL that runs it and the `images` / `tags` /
`image_tags` tables (§7), but the only writer of tags is ingest: a capture arrives with
whatever the source sent, and it stays that way forever. So the tag search of
`library-browse` cannot be exercised on a real library, and `rating:` matches nothing an
image was ever given. §8 Phase 2 opens with "tags/ratings editing"; this is it.

Editing is only half the loop. The legacy viewer's working rhythm is: see which tags the
current results actually carry, click one to narrow, click a rating pill to switch, sort or
group the result, edit tags on the image in front of you, watch the counts move. Every part
of that is a read of the *current result set*, which no screen in the app has today. The
sidebar and the toolbar's view controls are the other half, and app-shell already reserved
the slots for both (Slot: Sidebar · filters, Slot: Toolbar · view).

**Depends on:** `app-shell` (every control lands in one of its named slots; the Inspector it
builds read-only becomes editable here). Independent of `bridge-extension`.

## What Changes

- **Tags are editable** (§8 Phase 2): the Inspector's read-only tag list becomes an editor
  with autocomplete over the library's own tag vocabulary, in both Inspector placements —
  beside the grid and inside the full-size viewer. A tag can be removed from its context menu
  without retyping the list.
- **Tags are clickable**: a tag in the Inspector or the sidebar goes into the tag search as an
  include or an exclude, and clicking an active one takes it out again. This is what makes
  `library-browse`'s query language usable without typing it.
- **Ratings are editable**: `g` / `s` / `q` / `e` / unrated, set from the Inspector or from a
  tile's context menu, shown as a badge on the tile (§6, the rating half of the tag system).
- **The sidebar shows the current result set** (Slot: Sidebar · filters): every tag in the
  results with its count, `+` to include and `−` to exclude, and rating pills whose counts say
  what each rating *would* yield — the legacy viewer's two sidebars, computed in SQL over the
  whole result rather than over one loaded page.
- **Sort and group** (Slot: Toolbar · view): sort by capture time, last change, file size or
  pixel dimensions, ascending or descending; group by nothing, by X account, or by duplicates
  (same dimensions and byte size). Both are part of the search request, so they order and
  slice the whole result set, not the page that happens to be loaded.
- **One query language, one implementation.** `filters.ts` and `grouping.ts` were lifted from
  the legacy repo as the tested statement of the semantics the SQL had to reproduce, each
  carrying a FIXME saying "two implementations of one rule". Grouping moves into SQL, nothing
  in the webview filters or groups any more, and both modules are deleted with their cases
  ported into `query.rs`'s tests — which also spends the matching FIXME on `x_account()`.

## Capabilities

### New Capabilities

- `tag-editing`: changing the tags of one image, with suggestions from the library's
  vocabulary, and tags that act as search terms when clicked.
- `rating`: giving one image a rating or taking it away, and seeing it on the tile.
- `tag-sidebar`: the tags and ratings of the current result set, with counts, as filters.
- `sort-and-group`: the order of the result set and its division into groups.

### Modified Capabilities

- `library-browse`: the grid's order stops being "newest capture first" unconditionally — that
  becomes the default of a user-chosen sort, and grouping can restrict which images are shown
  at all.

## Non-goals

- **Bulk edits.** Tagging or rating many images at once is `selection-and-bulk`, which owns
  the selection model and the bulk toolbar. Everything here acts on exactly one image.
- **Deleting anything.** No trash, no record drop beyond what Phase 1 already has (`trash`).
- **Rules that assign tags.** `auto-tag-rules` owns rules, their storage and the ingest hook;
  this change only makes the tags they will write visible and editable.
- **Library-wide tag operations**: rename, merge, alias, categories, colours, per-tag counts
  across the whole library. Nothing here changes a tag on more than the image being edited.
- **Persisting the sort and the group across launches.** app-shell decided per-view state is
  session state and only the theme and the tile size are settings; sort and group are view
  state.
- **New keyboard bindings.** `app-frame` holds one keyboard map; adding a key would mean
  editing that requirement. Ratings and tags are reachable by pointer and by the controls the
  map already focuses.
- **A schema change.** Nothing here needs a migration (design D1), so the version stays where
  `bridge-extension` left it.
- **`posted:` / `unposted` filters.** §8 Phase 3, and `booru-upload` owns the label half.

## Impact

- `packages/shared` + `packages/app/src-tauri/src/model.rs` (hand-mirrored, Phase 1 D11, one
  commit for both): `SearchRequest` gains `sort` and `group`; `SearchResult` gains `groups`;
  new `Sort`, `SortField`, `SortDirection`, `GroupBy`, `GroupSlice`, `TagCount`,
  `RatingCounts`, `TagCounts`.
- `packages/app/src-tauri`: new `tags.rs` (tag write path, rating extraction, orphan
  collection, suggestions, counts); `query.rs` compiles the sort, the grouping and the count
  queries and absorbs the ported fixture cases; `commands.rs` gains `update_tags`,
  `set_rating`, `tag_suggestions`, `tag_counts`; `maintenance.rs`'s `drop_image_record`
  collects orphan tags. No schema migration.
- `packages/app/src`: new `lib/components/tags/` (`TagInput`, `RatingControl`, `TagSidebar`,
  `RatingPills`); the Inspector's tag and rating blocks become editable; the toolbar gains
  sort and group selects; `lib/api/commands.ts` gains one wrapper per command and
  `search.svelte.ts` carries the sort, the group, the group slices and the sidebar counts;
  `lib/domain/tag-utils.ts` gains the query-string mutations the clicks perform.
- Deleted: `lib/domain/filters.ts`, `filters.test.ts`, `grouping.ts`, `grouping.test.ts`.
- New shadcn-svelte copy-ins: `command`, `popover`, `context-menu`, `toggle-group` (excluded
  from lint and format, CLAUDE.md).
