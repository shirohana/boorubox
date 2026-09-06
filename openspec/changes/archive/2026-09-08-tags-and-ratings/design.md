## Context

Phase 1 built the reading half: `parseTagSearch` in the webview is the only definition of the
query language (Phase 1 D3), `query.rs` compiles it to SQL, and `search.svelte.ts` asks for one
200-record page at a time over a sparse array. Tags are rows (`tags` / `image_tags`, Phase 1
D2) and `images.rating` is a nullable column, but the only writer of either is
`ingest::store_image`. `filters.ts` and `grouping.ts` were lifted from the legacy repo as the
tested statement of what the SQL must reproduce, each carrying a FIXME that says two
implementations of one rule can drift with nothing failing; `query.rs`'s `x_account()` carries
the third.

app-shell lands first and fixes where everything goes: the Inspector is one component in two
placements showing tags and rating read-only, the sidebar has a `filters` region that is absent
rather than empty, the toolbar has a `view` row holding the tile-size slider, and one keyboard
map with one guard (`isTypingTarget`). This change fills those slots and writes nothing outside
them. Motivation: see proposal.md.

Constraints that shape the approach:

- Rust owns storage; the webview is UI only (§6). Every rule about what a tag edit means to the
  library is Rust's; every rule about how a tag is displayed or how a query string is rewritten
  is the webview's.
- `packages/shared/src/index.ts` and `model.rs` are hand-mirrored (Phase 1 D11): each new type
  is written twice, in one commit.
- The result set is paged (`PAGE_SIZE = 200`) and sparse. Anything computed over "the images"
  in the webview is computed over a fraction of them.
- CLAUDE.md: extraction in the extension, policy in the app — tag sorting and rating extraction
  are named there as app-side policy.

## Goals / Non-Goals

**Goals:**

- One implementation of every rule about what matches, in what order, in what group: SQL. When
  this change is done, no TypeScript in the app decides membership of a result.
- Everything the sidebar and the toolbar show describes the *whole* current result set, never
  the pages that happen to be loaded.
- An edit costs one write and one refresh of the counts, not a re-run of the search under the
  user's hands.

**Non-Goals:**

- Content hashing for duplicate detection. "Duplicates" keeps the legacy meaning — same pixel
  dimensions and same byte size — because a real content hash is a new column, a migration and
  a backfill over every image, which is its own change with its own argument.
- A tag vocabulary cached in the webview. The library is never held in memory (Phase 1 D3).
- Anything that acts on more than one image (`selection-and-bulk`).

## Decisions

**D1. No schema change: this change adds no migration, and the schema version stays where
`bridge-extension` left it.**

Every write this change makes has a home in the schema Phase 1 shipped (D2): tags are
`tags` + `image_tags` rows, the rating is `images.rating`, the edit stamp is `images.updated_at`,
and collecting an orphan tag is a `DELETE` from `tags`. Sorting reads `captured_at`,
`updated_at`, `size`, `width` and `height`, all present. The only candidate for a migration is
an index per sort column, and D6 rejects those on their own merits.

Version ownership, stated because two changes are being drafted in parallel: v1 is
`phase-1-app-mvp` (D2), v2 is `bridge-extension` (`images.adapter_json`, its D11). This change
appends nothing to `MIGRATIONS`, so v3 is unclaimed and falls to the next Phase 2 change that
needs one — `auto-tag-rules`, for its `rules` and `notes` tables. If this change somehow lands
before `bridge-extension`, nothing here changes: appending nothing is version-independent.

**D2. `update_tags(id, tags)` replaces the image's whole tag set.**

The editor is a text box holding every tag of one image, so the whole set is what the user
edited and the whole set is what is sent. Alternative — add/remove deltas, which is what
`selection-and-bulk`'s `bulk_update_tags(ids, add, remove)` will take — rejected here: a delta
has to be computed against a record the webview is holding, and that record is as old as the
last search; two edits from the Inspector and the lightbox on the same image would each diff
against their own copy. A bulk edit has no "whole set" to send, which is why that command has a
different shape; this one does, and takes it.

Rust does the work in one transaction: insert missing `tags` rows, delete the `image_tags` rows
that are no longer wanted, insert the new ones, collect the orphans (D5), stamp `updated_at`
(D9). The command returns the reloaded `ImageRecord` (D10).

**D3. `rating:g|s|q|e` typed among the tags sets the rating, and the rule lives in Rust beside
the write.**

Ported from the legacy `extractRatingFromTags`, which every tag write in that codebase went
through. Putting it in the webview before the invoke would work for this change and break the
next two: `bulk_update_tags` and the auto-tag rules both write tags without passing through this
editor, and each would need its own copy of the rule. It goes in the one Rust helper every tag
write calls, so a rule that adds `rating:e` behaves like a user typing it. Anything else after
`rating:` is an ordinary tag — the parser's own metatag alphabet is `g|s|q|e`, and silently
dropping `rating:unknown` would lose a tag the user typed.

**D4. `sortTags` is the only tag order, and it is applied when tags are rendered, not when they
are stored.**

`image_tags` is a set; there is no column to store an order in and no requirement that wants
one. Two orders exist today in potential: SQL's `ORDER BY tags.name` (BINARY, case-sensitive)
in `load_records`, and `sortTags` (case-insensitive `localeCompare`), the lifted and tested
module. The webview sorts what it renders with `sortTags` — Inspector, tile overlay, and the
text the editor is re-composed with after a save — so the SQL order never reaches a screen and
stays what it is: a determinism aid for tests. Alternative — `ORDER BY tags.name COLLATE
NOCASE` so the database returns the display order — rejected: it writes a second, subtly
different ordering rule into the schema (NOCASE is ASCII-only; `localeCompare` is not), and the
one CLAUDE.md names as app policy is the TypeScript one.

"Sorted on save" therefore means the editor's text is re-composed sorted after the save
succeeds, which is the only place a user can observe an order at all.

**D5. Orphan tags are collected in the same transaction as the unlink, scoped to the tags that
were unlinked. There is no sweep.**

`drop_image_record` leaves orphan `tags` rows today (the defect the backlog assigns to this change; it was never marked in the source), and removing
the last use of a tag through the editor would leave another. The helper takes the tag ids the
statement just unlinked and deletes each one that no `image_tags` row still references. It is
called from `update_tags` and from `drop_image_record`.

Alternative — a periodic or on-demand maintenance sweep over the whole `tags` table — rejected:
it is a second answer to "is this tag still in use", and it lets the wrong answer live until
someone runs it. The `tags` table is not bookkeeping here: it is the autocomplete vocabulary and
the sidebar's list, so an orphan is visible to the user as a tag that suggests itself and
matches nothing. Scoped collection makes the invariant "a row in `tags` has at least one use"
hold at every commit.

Two later changes inherit the helper and must be told: `trash`'s soft delete SHALL NOT collect
(a restored image keeps its tags — the `image_tags` rows survive a soft delete), and its
`delete_forever` SHALL collect, because the cascade removes the links without touching `tags`.
The helper is public and named for them.

**D6. Sort is part of `SearchRequest` as a whitelisted field plus a direction, with `id DESC`
always breaking the tie. No new indexes.**

`sort: { field, direction }` where `field` is an enum (`captured` | `updated` | `size` |
`dimensions`) that `query.rs` maps to a fixed column expression — `captured_at`, `updated_at`,
`size`, `width * height` (the area, which is what the legacy `sortImages` compared). Alternative
— the legacy's string key `capturedAt-desc`, split at the hyphen — rejected: it arrives from IPC
and would be interpolated into SQL, and this file's rule is that nothing but placeholders is
ever formatted in. The enum cannot spell a column that does not exist.

The tie-break is not decoration. `query.rs` already carries the comment: without `id` in the
`ORDER BY`, two rows with equal sort values can come back in different orders for the page-1 and
page-2 queries, and a row then repeats on one page and is skipped on the other. Every sort
column here has ties by nature — `size` and `dimensions` especially — so the tie-break moves
from "captured_at plus id" to "whatever was chosen, plus id".

No index is added for the three unindexed sorts. Ten thousand rows is a temporary B-tree sort of
a few milliseconds, while four more indexes cost write amplification on every ingest and file
size in a folder that may be cloud-synced (§7). Revisit when a sort is visibly slow on a real
library; adding an index then is a one-line migration, and it will be a measured one.

**D7. Grouping is compiled into the search, not computed in the webview, and the group slices
come back with the result.**

The webview cannot group. `search.svelte.ts` holds a sparse array: 200 records of a result that
may be 5000, and `at(index)` answers `undefined` for a row whose page has not arrived. Grouping
that array gives a group whose other members are in an unloaded page, a heading count that is
the loaded fraction, and headings that re-sort themselves as pages arrive. Worse, grouping is
not only ordering: by X account it drops every image whose page names no account, and by
duplicates it drops every image with no twin — so `total`, which SQL computes, and the list,
which the webview would compute, would disagree on the same screen about how many results there
are.

So `group: 'none' | 'x-account' | 'duplicates'` joins `SearchRequest`, and `query.rs`:

- adds the group's restriction to the `WHERE` — `x_account(images.page_url) IS NOT NULL`, or
  membership in a `(width, height, size)` triple that occurs more than once in the matched set;
- puts the group key ahead of the sort in the `ORDER BY`, ordered as the legacy viewer ordered
  its groups: accounts by size descending then name, duplicate keys by key;
- returns `groups: [{ key, count }]` for the whole result set from one extra `GROUP BY`.

The duplicates restriction needs the matched set twice (once to list it, once to count the
triples in it), so the compiled filter is emitted into a `WITH matched AS (…)` CTE. As drafted,
this paragraph went on to say the bound parameters are "pushed twice, in order"; that was
written on the assumption the filter text would appear twice in the SQL. Implementation put it
once, in the CTE, and every read of the set (`grouped`, `slices`, the join) reads the CTE, so the
parameters are bound once and a second push would be a bug. All three group modes share that
plan, so the filter and its values live in one place, which is the hazard `Filter` exists to
remove. `EXPLAIN QUERY PLAN` shows the ungrouped CTE still flattens onto `images_captured_at`.

`groups` is what makes the grid's layout computable before any page is loaded: headings, their
counts and the row each group starts on are arithmetic over the slice counts, so scrolling to
the fortieth group shows the right heading over the right images without having visited the
thirty-nine before it. `key` is the raw key — the account handle, or `WIDTHxHEIGHT-SIZE` — and
the webview formats the label, because a display string is UI and §6 keeps UI out of the storage
layer.

**D8. One command answers the sidebar, and its two halves are computed differently on purpose.**

`tag_counts(req)` returns `{ tags: [{ name, count }], ratings: { g, s, q, e, unrated } }`. Both
halves use the same compiled filter as `search` — group restriction included, `limit` and
`offset` ignored — so the sidebar always describes the whole result set.

The halves differ in one clause, and this is the legacy behaviour ported rather than made
uniform:

- **tags** are counted over the result *as filtered*, rating clause included
  (`updateTagSidebar(state.filteredImages)`). The question a tag count answers is "how many of
  what I am looking at also carry this", which is the narrowing move.
- **ratings** are counted with the rating clause removed
  (`computeRatingCounts(…, includeRating: false)`). The question a rating pill answers is "how
  many would I get if I switched to this", which is a sideways move. Counting them after the
  rating filter would show the selected rating's own count and four zeros.

Making both uniform breaks one of the two questions, whichever direction is chosen. The
asymmetry is written here so the next reader does not "fix" it.

One command rather than two: the two halves are one panel refreshed as one unit, two commands
would let the pills and the tag list describe different queries while one is in flight, and the
filter would be compiled twice for the same request. The command keeps the name the parallel
drafts agreed on; the payload carries both halves.

Tags the query names but the result does not contain are added by the webview at count zero (as
the legacy did): SQL returns what the results carry, and the webview is where the parsed query
already is.

**D9. Every edit stamps `updated_at`, and the FTS trigger firing on it is accepted.**

Without the stamp, "sort by last change" is a lie, and it is one of the four sorts. The write
uses the same `db::now_ms()` every other timestamp uses. `images_fts_update` fires on any
`UPDATE` of `images`, so setting a rating re-indexes that row's three text columns — the values
are unchanged, the index ends up identical, and the cost is one row. Scoping the trigger to
`AFTER UPDATE OF page_title, page_url, image_url` would remove it and would be a schema change
(D1) for a cost nobody has measured; noted here for whoever measures it.

**D10. `update_tags` and `set_rating` return the updated `ImageRecord`, and the search is not
re-run.**

Returning `LibraryStatus` (what `drop_image_record` returns, because it changes the count)
tells the caller nothing about the row it just changed. Returning the record lets the webview
replace it in the loaded page and redraw the tile and the Inspector.

The result set is deliberately not re-run after an edit. An image that no longer matches the
current search stays on screen until the next search: re-running would move the grid under the
hands of a user in the middle of tagging, and would make the image being edited vanish the
moment a tag stopped matching — which is the ordinary case of removing the tag you searched for.
`search.svelte.ts` already separates `generation` from `queryGeneration` so a refresh keeps the
scroll position; an edit needs neither. The sidebar counts *are* refetched, so for a moment the
counts describe the query and the grid describes the last run of it; the next search reconciles
them. That is the honest trade and it is recorded as a risk.

**D11. The rating written by an edit is validated; the rating read from a foreign library is
not.**

`ImageRecord.rating` is `Option<String>` in Rust on purpose (its comment: legacy-bundle import
carries whatever the old library stored, and ingest must not drop an image over an unknown
rating). `set_rating` is the opposite case: the value originates in this app, so a value that is
not `g`, `s`, `q`, `e` or null is a bug in our own UI, and storing it would put a rating in the
library that no filter can ever select. The command rejects it; ingest keeps tolerating.

**D12. Suggestions come from a SQL prefix query, and the autocomplete's rules live in a pure
module; the popover is shadcn `command` inside `popover`.**

`tag_suggestions(prefix, limit)` matches `tags.name` by prefix — served by the UNIQUE index
that is already on it — orders by how many images use the tag and then by name, and returns at
most `limit`. The legacy viewer built its vocabulary by walking every image it held in memory;
this app never holds the library in memory (Phase 1 D3, and the paged store), so a webview-side
vocabulary would be a second cache to invalidate on every edit, every capture and every import.
Ordering by usage is a departure from the legacy's alphabetical list, and it is what makes a
short list useful: with an eight-row popover, alphabetical order buries the tag you use daily
behind five you used once. Ties break by name so the list is deterministic. Tags already typed
into the input are dropped by the webview, not by SQL: the input is a query string, and the
thing that knows how to read one is the parser, which is in the webview (Phase 1 D3).


The rules are all edge cases — where the current token starts and ends relative to the caret,
the `-` prefix that must survive an accepted suggestion, the metatag and `or` contexts that
suppress the list, and the three-way meaning of confirming. They are ported from the legacy
`autocomplete.ts` (`isCurrentTokenIncomplete`, `completeCurrentToken`, `insertTag`, and the
`Enter` branch order) into `lib/domain/tag-input.ts` as functions over `(value, caret)`, with
tests. This repo has no component-test harness — app-shell recorded that, and this change does
not add one — so anything left inside the `.svelte` file is untested by construction.

`TagInput.svelte` is the input plus the popover, used in two places: the toolbar's tag search
(Slot: Toolbar · search, where app-shell reserved the popover) and the Inspector's tag editor
(Slot: Inspector · tags). One component, two placements, matching the Inspector's own rule.

**D13. Confirming acts on what is pending, then submits — accepting a suggestion finishes the
token in the same step, reversing this decision's own "no space on accept" (owner's pass, task
7.5).**

First confirmation with a highlighted suggestion accepts it and finishes the token — a space
after the tag, stepping over one already there — leaving the caret ready for the next tag; with
an unfinished token and no highlight it finishes the token the same way; with nothing pending it
submits. `Tab` accepts a highlight, arrows move it, `Escape` closes the list without touching the
text.

Why the old reading was right at the time: accepting inserted only the tag, so every confirmation
meant exactly one of the same three things (accept, complete, submit) regardless of where the
caret was — a uniform count was simpler to specify, and it read as a virtue: three keystrokes for
three effects.

Why it stopped being right: in use it leaves the caret glued to the accepted word (`apple ball|`
after accepting `ball`) — the very thing that opens the next suggestion list is a caret past a
word boundary, and nothing reopens it until the owner types a space by hand. The second Enter,
which the old model reserved for "finish the token," then has nothing left to do but insert the
space the accept should already have made. A suggestion is chosen with the intent that it is a
whole tag; accepting it and finishing it are the same event, and folding them removes a
keystroke the uniform count was never buying anything with.

This adds no binding to app-frame's keyboard map, and cannot conflict with it: every one of
these keys acts while the focus is in a text field, which is exactly where that map declares
nothing fires.

**D14. Query-string mutation joins the parser in `tag-utils.ts`.**

`removeTagFromQuery` is already there. `addTagToQuery`, `excludeTagFromQuery`,
`toggleTagInQuery` and `toggleRatingInQuery` join it, with tests. In the legacy viewer these
were five functions in `index.ts`, each with its own inline copy of the `rating:` regex that the
parser also carries — the pills' toggle rewrites the whole `rating:` metatag, and getting it
wrong silently produces a query that parses to something else. Every click in the sidebar, the
Inspector and the pills goes through these, so they are one module beside the parser whose
output they have to round-trip through.

**D15. `filters.ts` and `grouping.ts` are deleted, and their cases move into `query.rs`'s
tests.**

Both were lifted as executable documentation of semantics that lived in two places (their
FIXMEs). After D7 nothing in the webview filters or groups, so keeping them is keeping a second
implementation that no longer even runs. Deleting them without moving the cases would lose
coverage, so the mapping is fixed here:

| From | To |
| --- | --- |
| `filterByView` (all hides deleted, trash shows only deleted) | already `deleted_rows_are_excluded_until_asked_for` |
| URL/title substring, case-insensitive | a new free-text case in `query.rs` (FTS5 is case-insensitive; the existing cases are lower-case only) |
| include / exclude / OR / rating / `is:` / `tagcount:` / `account:` | already covered by `query.rs`'s suite |
| `computeRatingCounts` — counts across the filtered set, and ignoring the rating metatag | the two rating-count tests of `tag_counts` (D8) |
| `sortImages` — `capturedAt` both ways, `updatedAt`, `size`, `dimensions` as area | the sort tests (D6) |
| `getXAccountFromUrl` — x.com, twitter.com, `www.` variants, reserved first segments, non-X hosts, null URL | the `x_account` tests, which is what closes that FIXME |
| `groupImagesByXAccount`, `groupImagesByDuplicates`, `getVisualOrder` ordering | the grouping tests (D7) |
| `parseSearchQuery` — a `tagcount:` metatag inside the free-text box | **not ported, deliberately** |

The last row is a decision, not an omission. The legacy viewer had one search box, so a metatag
had to be parseable out of the free-text input. Phase 1 D14 gave the app two inputs and made the
free-text one data: `fts_string` quotes every character in it so nothing there is syntax. A test
for parsing `tagcount:` out of that box would test a behaviour the app decided against.

**D16. The `library-browse` delta is copied from app-shell's delta, not from the main spec.**

MODIFIED requires the full requirement block, and "Grid shows the library" is modified by
app-shell too — it is where the aspect-fit tiles, the hover overlay and the tile-size slider are
written. app-shell lands first but archives into `openspec/specs/` on its own schedule, so
copying the block that is in the main spec today would quietly revert all three at sync time.
The base text is therefore app-shell's version of the requirement, edited only where the order
and the membership of the grid change.

`library-browse`'s other requirement, "Tag search uses the legacy query language", is left
alone. The language does not change here — only whether a user can produce the tags to search
for, which is `tag-editing`'s requirement to make.

**D17. Slots filled, by the row names of app-shell's slot map.**

| Slot | What this change puts there |
| --- | --- |
| Inspector · tags | the tag editor with its suggestion popover, each tag clickable into the search and removable from its context menu |
| Inspector · rating | the rating control (`g`, `s`, `q`, `e`, clear) |
| Sidebar · filters | the tag list with counts and include/exclude, and the rating pills with counts |
| Toolbar · search | the tag input gains the suggestion popover the map reserved — the same component as the Inspector's |
| Toolbar · view | the sort select and the group select, beside the tile-size slider |
| Grid · tile | the rating badge, and a context menu that sets the rating |

One departure from the map's wording: it lists the tile's context menu as "remove tag / set
rating". The tile shows no tags — app-shell moved the caption strip into the hover overlay,
which carries title, source and date — so a "remove tag" entry there would have no tag to name.
Removing a tag is offered where tags are shown, in the Inspector. The map's intent (a context
menu on the tile, owned by this change) is kept.

## Risks / Trade-offs

- [An unindexed sort is slow on a large library] → 10k rows sort in milliseconds; the trade is
  written in D6 with the threshold to revisit and the fact that an index is a cheap later
  migration.
- [The duplicates grouping binds the compiled filter twice and scans the matched set twice] →
  both scans are over the same filtered set the search already computes, and the triple counting
  is a `GROUP BY` over three integer columns; verified at 10k rows in the grouping tests before
  the UI is wired.
- [The sidebar recomputes counts on every keystroke in the search box] → the counts request
  rides the same debounce and the same generation as the search itself, so it is one extra
  `GROUP BY` per search, not per keystroke; the `image_tags_by_tag` index already serves it.
- [Counts and grid disagree for a moment after an edit (D10)] → the counts describe the query
  and the grid describes the last run of it; both are correct answers to different questions,
  and the next search reconciles them. Chosen over a grid that reorganises while the user types
  a tag.
- [Group headings make the virtualised grid's row math a second layout algorithm] → the layout
  is derived from `groups`, which is authoritative and arrives with the result; `grid-window`'s
  existing claim (rendered rows depend on the viewport, never on the library) is re-asserted in
  its tests for the grouped case.
- [`rating:s` typed as a tag disappearing surprises someone who meant a literal tag] → it is the
  legacy behaviour on the same text, it is specified, and `rating:unknown` shows where the rule
  stops.
- [A tag deleted as an orphan while another window holds it in an autocomplete list] → one
  library is open in one app instance (Phase 1 D1, app-shell D1); the list is refetched per
  keystroke.

## Open Questions

- Whether the sidebar's tag list needs a cap or its own filter box on a library with tens of
  thousands of distinct tags. It is one `LIMIT` in one query plus a text input; it changes no
  spec, no task and no other screen. Decide against a real library.
- How many suggestions to show at once (the legacy showed eight). One constant.
