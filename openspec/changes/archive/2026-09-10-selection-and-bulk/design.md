## Context

See proposal.md for motivation. The state this design has to fit into:

- The grid never holds the whole result. `SearchResults` (`src/lib/api/search.svelte.ts`) is a
  sparse array of `total` rows in which only the 200-row pages that have been asked for hold
  records; `at(index)` answers `undefined` for every other row. A selection therefore cannot
  assume it can name the images it covers.
- `app-shell` D9 gave the grid a focus index and said so in as many words: "This focus index is
  also what `selection-and-bulk` extends into a selection model, so the interaction is built
  once." Its keyboard map binds the unmodified arrows, `Home`/`End`, `Enter`/`Space` and `i` in
  the grid; `Esc` is bound only in the search field and the lightbox.
- `app-shell` D14 put the bindings where they act and allowed exactly one shared thing:
  `lib/keyboard.ts`'s `isTypingTarget`. A second dispatcher is the thing it forbids.
- Rust owns storage and the filesystem; the webview is UI only (§6). Every command goes through
  one `Connection` behind a `Mutex` in `AppState` (Phase 1 D1), and `lib/api/` wraps each
  command once (Phase 1 D12).
- `tags-and-ratings` lands first and owns `TagInput.svelte`, `RatingControl.svelte`,
  `update_tags(id, tags)`, `set_rating(id, rating)`, `tag_suggestions(prefix, limit)` and the
  orphan-tag collection that follows a tag removal.
- `query::search` already resolves a page as ids first (`SELECT id … ORDER BY captured_at DESC,
  id DESC LIMIT ? OFFSET ?`) and only then loads records for them.

## Goals / Non-Goals

**Goals:**

- The count is honest the instant the user asks for it, for any selection size, without the app
  reading records it does not need.
- One place decides which card is current. Focus and selection are one state machine, not two
  that have to agree.
- A bulk write is one write. Fifty images edited by fifty statements in fifty transactions is
  fifty chances to stop half way.
- Nothing about selection reaches into the lightbox, the search, or the ingest path.

**Non-Goals:**

- **Bulk delete, in any form.** `trash` adds "Move to trash" and "Restore" to this change's
  `SelectionToolbar` and owns their specs. It is not deferred for tidiness: the only deletion the
  app has today is `drop_image_record`, which Phase 1 D16 defines as "the file is gone, forget the
  row" and which leaves `images/` alone. Pointing that at a 200-image selection would produce an
  action that reads as "delete" and cannot be undone, in an app that has nowhere to undo it. The
  toolbar is built with the slot for it empty.
- Persisting a selection across launches, or across a library switch.
- Selecting in the lightbox. It shows one image and navigates its own way (`app-shell` D10).
- Undo. A bulk tag edit is reversed by the inverse bulk tag edit; a bulk rating by another one.

## Decisions

**D1. The selection store owns the grid's focus index.**

`app-shell` task 4.4 put the focus index in `LibraryGrid.svelte` as component state.
`selection.svelte.ts` takes it over: `focus` (the card the arrows move and the inspector reads
when nothing is selected) and `anchor` (the card a range extends from) live beside the selected
set, and `LibraryGrid` reads them.

Every selection gesture is a function of both — shift+arrow moves the focus *and* rewrites the
range from the anchor; a plain click moves both and clears the set; the toolbar's clear leaves the
focus alone. Split across two owners, each of those needs a rule about which one wins and where
the anchor is stored, and the rules would be written twice: once in the component and once in the
store. Alternative — keep focus in the component and pass it into the store on every gesture —
rejected: the store would then be told the focus rather than deciding it, and `Shift+ArrowRight`
would move the focus in one file and extend the range in another with the two reading different
totals.

Amended after the owner ran it: `focusAt` — the focus *and* the anchor — is what a *deliberate*
move calls, and a card's own `focusin` calls `focusEntered`, which moves the focus alone. The
original rule ("keyboard movement takes the anchor with it") was right about the gesture and wrong
about the event. Every gesture ends with the DOM focus landing on a card, and the store heard that
landing as a fresh move: the grid focuses the card a shift-arrow moved to, and WebKit focuses a
tile on `mousedown` before the click can say whether shift was held. So the anchor was re-pinned
to the focus after every shift gesture — a shift-arrow selected the last two cards it passed
instead of the range from where the user started, and a shift-click selected one. The anchor now
moves only on a plain arrow, a click that is not a shift-click, and the card the viewer closed on,
which is the same set of gestures the rule always meant.

Focus is *not* membership. A plain click focuses and clears the selection, which keeps
`app-shell`'s shipped behaviour ("a single click focuses a card and the inspector follows")
literally true and keeps the search inputs on screen while the user is merely looking at images.
Alternative — a plain click selects one image, as the legacy viewer's `selectCard` did — rejected:
the toolbar's action row would be replaced by the selection toolbar the moment anyone clicked a
thumbnail, so browsing would cost the import menu and (with `tags-and-ratings`' controls beside
it) the rest of the row.

**D2. The selection has two representations and is only ever in one of them: a set of ids, or a
single index range.**

```
type SelectionState =
  | { kind: 'ids'; ids: Set<string> }
  | { kind: 'range'; start: number; end: number }   // half-open, over the current result order
```

- Multi-select-click, the tile's checkbox, and a plain click build `ids`: the tile is drawn, so
  its record — and its id — is in hand.
- Shift+click, shift+arrow, shift+`Home`/`End` and select-all build `range`, replacing whatever
  was selected. `range` always runs between the anchor and the focus (select-all is
  `[0, total)` with the anchor at the focus), so one range is all the model ever needs.
- A multi-select-click while a range is up resolves the range to ids first (D3) and then toggles.

Count is exact in both: `ids.size`, or `end - start`. Membership is `ids.has(record.id)` or
`start <= index < end`, and the tile has both to hand.

Alternative — one representation holding both, a set of ids plus a list of pending ranges —
rejected: the two overlap the moment a range covers a row that is already in the set, so `count`
stops being a sum and every gesture has to reconcile them. Alternative — indices only — rejected
in D4. Alternative — ids only — rejected in D3.

**D3. The ids under a range are asked for in one Rust call, not collected as the grid's pages
arrive.**

`search_ids(req)` returns just the ids matching a `SearchRequest`, in the grid's order, with no
records, no tags and no missing-file pass. It is not new logic: `query::search`'s id query is
extracted into `query::search_ids`, and `query::search` calls it. That extraction is the point —
the selection's row *n* and the grid's row *n* must be the same image, and they are only
guaranteed to be while one `ORDER BY captured_at DESC, id DESC` exists in the codebase.

Resolving a range means one `search_ids` with `offset = start`, `limit = end - start`. It happens
when an action needs ids, and at no other time.

Alternative — the brief's first reading, resolve ids opportunistically as the pages the grid
scrolls over arrive — rejected on cost and on completeness. Cost: a page of `search` loads 200
full records with their tags *and* runs `refresh_missing_for` over them, which is 200 `stat`
calls; select-all over a ten-thousand-image library would be 50 such calls and 10,000 `stat`s,
paid the instant the user presses the shortcut, for an answer needed only if an action follows.
Completeness: rows the user never scrolls to would never resolve, so the action would have to
force the pages anyway — the expensive path, just later and less predictably. What the
opportunistic reading gets right is kept: ranges are index ranges, because "from here to there"
is defined by the result *order*, and the rows in between may have no record in the webview at
all.

**D4. Ids, not indices, are what survives an action.**

Every command takes ids, and every bulk action ends by re-reading the current search so the grid
shows the edit. An index range that outlived that refresh would be a hazard: with a rating filter
in the query, a bulk rating drops rows out of the result, every later index shifts up, and the
still-highlighted range would cover images the user never picked. Since resolving is exactly what
running an action does (D3), the store is in `kind: 'ids'` from the moment an action starts, and
that is the state the refresh finds. A *new* query clears the selection outright (spec
`selection`): the count would otherwise describe a result set that is no longer on screen, and the
anchor and focus would index into a different order.

Alternative — keep the range and re-resolve it against the new result — rejected: it silently
re-points the selection at different images, which is the failure this decision exists to prevent.

**D5. The new keys ride the grid's existing keydown handler and its existing guard.**

`Shift`+arrows, `Shift`+`Home`/`End`, the select-all shortcut and `Esc` are added to the handler
`app-shell` put in `LibraryGrid.svelte`, behind the same `isTypingTarget` check (D14). Movement
still goes through `offsetIndexClamped` — a shift-arrow at the last card must stop there, the same
way a plain arrow does, or the two edge behaviours drift apart, which is the bug
`navigation-math.ts` exists to prevent.

`Esc` is unbound in the grid today (`app-shell` binds it in the search field and in the lightbox),
so "clear the selection" adds no ambiguity; with nothing selected it does nothing, and it never
clears the focus — there would be no way to get the focus back except with the pointer.

The select-all shortcut calls `preventDefault`, which suppresses the webview's text select-all.
That is only reached with the grid focused and a text field excluded by the guard, so nothing the
user could have meant by it is lost.

Alternative — a window-level handler for the selection keys, since a selection is not really "in"
the grid — rejected: that is the central dispatcher D14 refused, and it would need to work out
whether the grid or the lightbox is live, which is the thing focus already answers.

Amended for select-all only, after the owner pressed it with the focus anywhere but the grid and
got the webview's own select-all highlighting the page. That reading held for the keys that are
about the focused card — shift-movement still rides the grid's handler, because a card
the arrows move is exactly what the grid owns — but not for this one: select-all is about the
*result*, which the screen owns, and it is the one selection key a user presses without having
clicked into the grid first. `Esc` went with it: a selection made from outside the grid needs
a way out that does not go through the grid, or the Clear button is the only exit. It is now bound in `LibraryScreen`'s window handler beside the `/`
binding (`app-shell` D14's own example of a binding that belongs to a screen rather than a
region), guarded by `isTypingTarget` and by `isInDialog` — a dialog or the viewer is what the user
is looking at, and a shortcut fired underneath one would act on a grid they cannot see. The grid
no longer reads the key at all, so nothing fires twice; this is still not a dispatcher, since each
binding remains spelled once where it acts.

**D6. `SelectionToolbar` replaces the toolbar's action row, and only that row.**

Slot: Toolbar · actions. While `count > 0` the `Import` menu is replaced by the count, "Select
all", "Clear", "Tags…", the bulk rating control and "Export…". Search and the view controls stay
where they are: a selection is a thing you have, not a mode you are in, and hiding the search
would make the app feel modal for an action that is one `Esc` away from over. Typing a new query
does clear the selection (D4), which is the trade and is spelled out in the spec rather than
hidden in a disabled input.

Alternative — a floating bar over the grid, as the legacy viewer had — rejected: `app-shell`'s
slot map names this row, and a second floating region is exactly the "invent a second frame" its
proposal forbids.

Amended: the actions are icon buttons, not words. Six labelled buttons plus the rating control ran
to roughly 640px, and this row shares the band with the two search fields, the two view selects,
the tile slider and the inspector toggle — on the owner's window the band overflowed horizontally
the moment one tile was ticked. The words are not gone: each button carries its label as its
`aria-label` and its tooltip, and shows it again as text at `xl` and wider, where the band has the
room. A window breakpoint rather than a container query on purpose — the row is a
shrink-to-content flex child, so a container query would be measuring a width its own labels
decide. The row is `min-w-0` and scrolls (`overflow-x-auto`) rather than pushing or clipping:
whatever ends up in it, it can never make the top bar wider than the window, and its last action
stays a swipe away instead of unreachable. The rating control keeps its five choices at full size
and is now the widest thing in the row (~160px of ~380px with the labels hidden); shrinking it
further would mean a second rating control that reads differently from the inspector's.

Amended again (2026-09-10): the row is the thing that gives, not the search fields. The first
amendment let the fields shrink first, and on a half-screen window they shrank to nothing the
moment a tile was ticked — a search bar that vanishes when a selection exists is the "mode"
this decision refuses. The search form now keeps a floor (`min-w-56`) that its two fields share;
the view selects and the slider never shrank. So the band's fixed width is about 810px, and below
it the band overflows the window rather than any control becoming unusable.

**D7. The inspector's multi header starts at two, and its thumbnails are the ones it can draw
without fetching.**

Slot: Inspector · header. `count === 1` shows the image's full panel — the same component, the
same fields, no special case — because "one selected" and "one focused" mean the same thing to
the person reading it. `count >= 2` replaces the identity block with the count and a strip of up
to twelve thumbnails.

Amended: a thumbnail in the strip is a control, not a picture. Pressing one opens that image in
the screen's viewer — through the same `onactivate` the grid opens with, so there is one viewer
and one thing to close — and each carries a small remove button that takes exactly that image out
of the selection (`Selection.remove`, which resolves a range first like every other action on
ids). Drawn as a read-only preview it answered "which images are these?" and then refused every
question that follows it: the owner's report was that the panel shows thumbnails and there is
nothing to do with them. Dropping to one selected image shows the single-image panel as it always
did, so the strip can be emptied down to a normal inspector. The twelve cap and "+N more" are
unchanged — the count above the strip is still the number that has to be right.

A thumbnail needs only an id (`thumbnail_path(id)`), so in `kind: 'ids'` the strip is exact. In
`kind: 'range'` the strip draws the first twelve rows of the range whose records the grid already
has, and says "+N more"; the range always starts at the anchor or at row 0, both of which are on a
loaded page, so it is full in practice. Alternative — fetch pages so the strip is always complete
— rejected: fetching records to draw twelve 40px thumbnails is the cost D3 exists to avoid, and
the count above the strip is the number that has to be right.

**D8. The bulk tag edit is a dialog; the bulk rating is a control in the toolbar.**

`BulkTagDialog.svelte` (shadcn `dialog`, added by `app-shell`) holds two `TagInput.svelte` fields
— "Add tags" suggesting from the library's vocabulary (`tag_suggestions`), "Remove tags"
suggesting only from the selection's own tags — plus the quick-remove pills of D9. It is a dialog
because the operation is two multi-value fields and a confirm, which is not a toolbar's shape.

The two fields are titled by headings, not by `<label for>`. A label activates its field on click,
and the gap between the two fields is exactly where a user reaches when they aim past the
suggestion list or the pills below it — clicking the dead space re-opened the list the click was
trying to leave. Each field already names itself in `aria-label`, so the association was buying
nothing that was not already there.

The rating is a `RatingControl.svelte` in the toolbar, applied on click. The legacy modal carried
a "No change" rating radio because the extension's toolbar had no room; here the slot map gives
the row to this change, a rating is one click rather than a form, and a rating row inside a tag
dialog is a second way to do a thing that already has one. Alternative — the legacy shape, rating
inside the dialog — rejected for that duplication; the cost is a second dialog trip when a user
wants to tag and rate in one pass, which is two clicks, not a wrong result.

**D9. The quick-remove pills' tag counts are computed in Rust, over the whole selection.**

`selection_tag_counts(ids, limit)` returns the `limit` commonest tags among `ids` with their
counts, from one `GROUP BY` over `image_tags`. The webview cannot answer this: the selection can
span pages it never loaded, so `results.at()` has no tags for those rows, and a count computed
from what is loaded would be confidently wrong — it would say "artist_x (12)" for a selection of
300 in which 260 carry it. Opening the dialog is an action, so it resolves the selection to ids
(D3) and asks; that is one query against an indexed join table, whatever the selection size.

Alternative — count in the webview from the loaded records — rejected as above. Alternative —
count in the webview after materialising every record page — rejected: it is D3's expensive path
run to draw ten pills.

**D10. A bulk write is one transaction over every id, not a loop over the single-image command.**

`bulk_update_tags(ids, add, remove)` opens one transaction, links every added tag and unlinks
every removed one for every id, collects orphan tags once at the end, and commits.
`bulk_set_rating(ids, rating)` is one `UPDATE images SET rating = ?, updated_at = ? WHERE id IN
(…)`. Neither calls `update_tags` / `set_rating` in a loop: N transactions is N chances to stop
half way and leave the user unable to tell which images were edited, and it is N times the fsync.
The spec's "the whole selection or none of it" is the transaction, not a retry loop.

The per-image logic is still written once. `tags-and-ratings` owns the tag helpers in Rust
(`update_tags` and the orphan collection); this change adds `add_tags` / `remove_tags` beside them
and both command layers call those. Alternative — a generic `apply_to(ids, f)` helper — rejected:
the two operations are one SQL statement each once the ids are a list, and the helper would exist
only to have a helper.

**D11. Export is written in Rust with the `zip` crate, entries stored, outside the library lock.**

`zip = { version = "8", default-features = false }`: with no features the crate can only store,
which is what an archive of JPEG, PNG and WebP wants — deflating already-compressed bytes costs
CPU for no measurable size gain, and the default features pull in bzip2, lzma, zstd and deflate64
for a writer that will never use them.

Entries are named `<id> <tag1> <tag2> ….<ext>` at the archive root — `yande.re`-style, the id
first so two selected images can never collide (ids are UUIDs) and never move, the tags after so
a Finder/Explorer listing of the archive reads as something without opening the app. The entry's
last-modified time is the image's `captured_at`, not the crate's `1980-01-01` default. Both were
originally plain `<id>.<ext>` with no mtime set (a UUID cannot collide, which was the whole
argument at the time) — reversed after the owner ran the feature and reported a UUID-only name has
no order, and a folder of exports that all read `1980/1/1 12:00` gives no time axis to sort by;
the id staying first keeps the original argument (no collision, no directory-crossing tag) while
the tags and the mtime give the export something to sort and skim by, which "just a UUID" never
did. [`export.rs`'s `entry_name`](../../../packages/app/src-tauri/src/export.rs) sanitizes `/`,
`\` and control characters within a tag to `_` (a tag must never open a directory) and caps the
full name at 255 bytes — the common filesystem/zip limit — by dropping whole tags off the end
rather than cutting one in the middle. `entry_mtime` converts through the `zip` crate's `time`
feature (`DateTime: TryFrom<time::PrimitiveDateTime>`) rather than a hand-rolled epoch-to-civil-date
computation; `time` was already resolved in the dependency tree (pulled in transitively), so
enabling the feature adds no new dependency. The fields are written in the webview's zone: a zip
timestamp carries no zone and every file manager shows it as local time, so UTC fields would read
hours off, and Rust has no sound way to ask a multithreaded process for its offset (the `time`
crate's `local-offset` refuses), so `export_zip` takes `utc_offset_minutes` from the webview, which is
already the zone capture times are rendered in. A `captured_at` the format cannot hold — before 1980,
or otherwise not a valid instant — falls back to the crate's `1980-01-01` default rather than
failing the export over one image's date, the same "don't fail the whole export over one image"
principle the missing-file case below already uses.

`export_zip(ids, path)` takes the library mutex once to resolve `ids` to `(id, ext, captured_at,
tags)` rows, then releases it and copies the files. Holding it for the copy would block every
capture and every search for the length of the export, and it buys nothing: a file under
`images/` is written once by a rename and never modified afterwards (Phase 1 D4), so reading it
without the lock cannot see a torn file. A file that has gone missing between the query and the
copy is left out and named in the report — the library is required to tolerate exactly that
(`library-folder`). The whole thing runs on `spawn_blocking`, as `import_paths` does, so the
window keeps painting.

Alternative — build the zip in the webview from the asset protocol, as the legacy viewer did with
JSZip — rejected: it would pull megabytes of image bytes through the IPC boundary and buffer the
archive in the webview's heap, and the requirement that Rust owns the filesystem (§6) exists
precisely so this kind of thing is not written twice.

**D12. The save dialog runs in the webview; the command takes a path.**

`pick_library` runs its dialog in Rust because the chosen folder is opened there and the path
never needs to reach the webview. Export is the other way round: the shared name is
`export_zip(ids, path)`, and a command that takes a path is one a Rust test can drive against a
`tempfile::TempDir` — a command that opened a dialog could not be tested at all. So
`lib/api/dialog.ts` gains `pickExportZipPath()` on `@tauri-apps/plugin-dialog`'s `save()`, which
needs `dialog:allow-save` in `capabilities/default.json` beside the `dialog:allow-open` the
import picker already has.

**D13. Export progress is an event, mirroring the import one.**

`export:progress` carries `{ done, total }` and is emitted per file, read through
`lib/api/events.ts` exactly as `import:progress` is, and shown inline in the toolbar where
`app-shell` D8 put import progress. A dropped tick is a bar that skips a number, which is not
worth failing an export over.

Bulk tag and bulk rating get no progress: they are one statement each and finish in milliseconds
however large the selection, so a progress bar for them would be a spinner that never shows.

**D14. No schema change, and no migration.**

Bulk tag writes go through `tags` / `image_tags` and the rating through `images.rating`, all of
which Phase 1 D2 ships. Schema v2 (`images.adapter_json`) belongs to `bridge-extension`; this
change introduces no version, so the next one belongs to whichever change first needs it. Stated
because a Phase 2 change that quietly added a migration would collide with a parallel one.

## Risks / Trade-offs

- [Select-all over a large library, then an action, resolves every id in one call — hundreds of
  kilobytes of strings across the IPC boundary] → it is one query and one round trip, against the
  50 paged `search` calls and 10,000 `stat`s the alternative would cost (D3); the count is
  already correct before it, so the user is never waiting to find out what they selected.
- [A range selection is drawn over rows whose thumbnails have not loaded, so the user sees
  selected placeholders] → the placeholder carries the same selected mark as a tile, and the
  count is authoritative; this is the honest rendering of a selection that is genuinely larger
  than what is on screen.
- [Typing in the search box silently clears a large selection] → the spec makes it a stated
  behaviour rather than a surprise, and the selection toolbar disappearing is the feedback; the
  alternative (a selection describing a result set the user is no longer looking at) is worse.
- [A bulk edit's refresh moves the grid under the user] → the refresh keeps its scroll position,
  because it bumps `generation` and not `queryGeneration` (Phase 1's `SearchResults`), and the
  selection is by id at that point (D4).
- [An export lands on a full disk or a read-only volume half way through] → the zip is written to
  the path the user chose and the error is reported with the count written so far; a partial
  archive at a path the user named is visible and deletable, unlike a temp file the app hides.
- [`tags-and-ratings` renames a Rust tag helper before this change lands] → the bulk commands are
  written against `tags.rs` after that change is merged, and task 1.2 is where the mismatch would
  show as a compile error rather than at runtime.

## Open Questions

- Whether twelve thumbnails is the right cap for the inspector's multi header (D7). It changes one
  constant, no spec and no task; decide against a real selection.
- Whether the quick-remove pills should stay at ten (the legacy number) once the owner uses them on
  selections of several hundred. Also one constant, and `selection_tag_counts` already takes the
  limit as an argument.
