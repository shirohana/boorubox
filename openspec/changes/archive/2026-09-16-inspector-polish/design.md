## Context

See proposal.md — Why. What the code does today, from a read on 2026-09-15:

- The grid binds its keys on its own scroll container (`LibraryGrid.svelte`), so they fire only
  while the DOM focus is inside it. The inspector beside the grid mounts with no `onrated`;
  only the viewer's placement passes one (`Lightbox.svelte`, `surface?.focus()`). A rating click
  leaves the focus on the chosen radio; a keyboard save in the tag editor blurs to `<body>`;
  the Save button and a tag badge keep it on themselves. None of these is inside the grid.
- The grid's only ways to take the focus back are `focusCard(index)` (which also moves the
  selection's focus and anchor) and the two effects that fire after a key move or a column
  change. There is no "put the focus back where it is" entry point.
- `LibraryScreen.runSearch` resets the selection (focus, anchor, ids) and closes the viewer
  before every run, and `searchFor` — every tag, rating and sidebar click — goes through it.
  The selection is index-based: after a new search the old row names another image.
- `search_ids(req)` returns ids for an offset/limit window; nothing answers "which row is this
  id at" without walking. `results.at(index)` answers only for loaded pages.
- `TagSidebar.svelte` derives `included`/`excluded` sets from `parseTagSearch(tagQuery)` and
  styles rows green / red-strikethrough. The inspector's badges are always `secondary`.
- The `account:` filter compiles to `x_account(images.page_url)` in `query.rs` (X hosts, first
  path segment, X's own pages excluded). `artist-from-url.ts`, lifted for the auto-tag rules,
  has a looser regex of its own. No TypeScript computes the account the search uses.
- Outward links go through `openExternal` (`api/opener.ts`, the opener plugin); an `<a href>`
  would navigate the webview. `PostedLabel.svelte` is the one place that does it.

## Goals / Non-Goals

**Goals:**
- One hand-back path for every completed action in the panel, in both placements.
- A search rewritten by a click keeps the inspected image current, with one round trip.
- One reader for "which terms are active" shared by the sidebar and the panel.
- The account entry can only ever name what the search matches.

**Non-Goals:**
- Changing what a typed search does (still a full reset).
- A third X-handle rule in TypeScript, or unifying the two that exist.
- Keeping a multi-image selection across a rewritten search.

## Decisions

### D1. The panel reports "done" through one callback, `onrelease`, and each placement decides where the focus goes

`onrated` becomes `onrelease` and fires after every completed action: a rating chosen, a
successful tag save (keyboard or button), a tag removed, a tag or account acted on as a search
term. A failed save does not fire it — the editor keeps the focus so the text can be fixed
(the existing rule in `submitFromEditor`, kept). The viewer keeps passing `surface?.focus()`.
The grid placement passes `grid?.refocus()`.

`LibraryGrid.refocus()` is new and small: it sets `focusWanted` and scrolls to the focused row
(`showCard(selection.focus)`) when a focus exists; the existing catch-up effect then focuses
the card. It does **not** call `selection.focusAt` — the selection's focus and anchor are not
what moved, the DOM focus is, and `focusCard` would drag the anchor along.

*Alternative rejected:* binding the grid's keys on the window and guarding by "focus is not in
a control". The keyboard map spec says a region's keys fire while the focus is in it; moving
them to the window is a different contract, and the panel's radios and badges would need
their own exclusions. The hand-back keeps the map as specified.

### D2. A click-driven search keeps its subject by id, found through one new command

`searchFor(next)` (tag, rating and sidebar clicks) becomes `searchKeeping(next, id)` where `id`
is the image the panel describes — the `image` the inspector already derives (the single
selected image, else the focused one). The screen runs the search and, in parallel, asks the
library `search_position(req, id)`: the row that id occupies in the new search's order, or
none. The query field and `selection.reset()` go with the run, not with the answer — they cost
the same whatever the position turns out to be, and between the two a reset that waited would
leave the toolbar counting a result that no longer exists. Only the focus waits: on a row,
`grid.focusCard(row)` (focus and anchor at the new row, scroll to it) and, if the viewer is
open, `lightboxIndex = row` plus `results.ensureRange(row, row + 1)` — the run loaded page 0
and the row is usually not on it — with the viewer left open. On none: the viewer closed and
no current image.

The viewer's own `results.at(index)` follows the new result because `results` is the same store;
its prev/next order is the new result's for free.

`search_position` compiles the same `Plan` as `search_ids` and asks for the row with
`ROW_NUMBER() OVER (<the plan's order>)` filtered on the id — one query, no page walk. The
typed search bar keeps `runSearch` (a full reset): the owner changes a query by typing when
they mean to look at something else.

*Alternatives rejected:* walking `results.at` (only loaded pages; the image is usually off the
first page — that is why the user is filtering); fetching all ids with `search_ids` (25k ids to
find one row); keeping the viewer open on the old index (it would show whatever moved into that
row).

### D3. Active terms come from one reader in `tag-utils`

`activeTerms(query): { included: Set<string>, excluded: Set<string>, accounts: Set<string>,
excludedAccounts: Set<string> }` moves the sidebar's derivation into `tag-utils` and the
inspector calls the same function. The sidebar's styles are the marking the spec names; the
badge uses the same colour classes. Anything that later needs "is this term active"
(collections, the seventh change in the queue) reads the same function.

### D4. The account comes from the record, derived on the Rust side

`ImageRecord` gains `account: Option<String>`, filled in `ingest::row_to_record` from
`page_url` by the `x_account` function `query.rs` already owns (made `pub(crate)`). Never a
column, never in the sidecar: it is a projection of `page_url`, and the sidecar drift guard
covers columns, not record fields. The webview shows it and toggles `account:<handle>` with a
new `toggleAccountInQuery(query, handle)` rewriter beside `toggleRatingInQuery`, rewriting the
whole `account:` list the way that one rewrites `rating:`.

*Alternative rejected:* porting `x_account` to TypeScript. The reserved-page list and the host
list would then live twice (three times, with `artist-from-url.ts`), and the button's promise —
"the search will find this" — depends on the two never disagreeing.

The entry is drawn blue (`sky`) when inactive so it reads as a filter, not a tag; included and
excluded take the tag marking from D3, so "in the search" looks the same for every term.

### D5. Open-address buttons are one `ExternalLink` component

An icon button that calls `openExternal(url)` and shows the failure line under itself, dropped
after the Page and Image values; rendered only for `http:`/`https:` addresses (a `file:` or
malformed value has no browser to go to). `PostedLabel` keeps its own text-link shape; the
shared part is `openExternal`, which already exists.

### D6. The account:handle regex and the handle itself

`parseTagSearch` accepts `[a-zA-Z0-9_]+` after `account:`; X handles are exactly that set, and
`x_account` returns the raw path segment. The entry passes the handle through unchanged, case
included, because `query.rs` compares the raw segment (its tests keep `Bob`).

## Risks / Trade-offs

- [The position round trip races a fast second click] → both requests carry the new inputs;
  the screen applies a position only while `results.generation` is still the one its own `run`
  bumped, which is literally the generation guard `SearchResults` uses for its pages. Not by
  comparing `results.inputs` against the object it passed in, which is how this was first
  built: `$state` hands that field back as a *proxy* of what was assigned, so the comparison
  is false on every click and the guard swallows every answer instead of the racing ones.
- [`refocus` scrolls the grid to the focused row on every panel action] → that is where the
  user's image is, and a rating click never moved the scroll before; if it reads as a jump, the
  scroll can be dropped from `refocus` without touching the contract.
- [Two X-handle rules still exist (`query.rs`, `artist-from-url.ts`)] → different questions
  (search match vs. artist guess); noted here so the next reader does not add a third. The
  panel uses the search's.
- [`ROW_NUMBER()` over the whole matched set for one id] → same cost class as `tag_counts`, which
  already runs over the matched set on every search; measured on the 25k vault in the hand
  check.
