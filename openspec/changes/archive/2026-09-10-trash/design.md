## Context

See proposal.md for motivation. The state this design has to fit into:

- `images.deleted_at` is Phase 1 D2's schema v1 column and has never been written by anything but
  a `#[cfg(test)]` helper — `query::mark_deleted`, whose doc comment says it stands in for "the
  way a trash view eventually will". `query::compile` already adds `images.deleted_at IS NULL`
  unless `SearchRequest.include_deleted` is set, and `Library::image_count` and
  `maintenance::image_counts` already exclude deleted rows.
- The only deletion in the app is `maintenance::drop_image_record`: it deletes the row (cascading
  to `image_tags` and `posts`, and firing the `images_fts_delete` trigger) and the thumbnail, and
  leaves the file under `images/` alone — Phase 1 D16, which ends "a destructive variant is a
  later decision, not a silent one".
- `app-shell`'s slot map reserves four places for this change: a `Trash` nav item with a count in
  Sidebar · nav, a "move to trash" entry in Grid · tile's context menu, "Move to trash" /
  "Restore" in Inspector · actions, and "bulk move-to-trash and restore" in Toolbar · actions.
  Its keyboard map is the one table of bindings and it does not bind `Delete` or `Backspace`.
- `selection-and-bulk` owns `selection.svelte.ts` (which resolves an index range to ids through
  `search_ids(req)`, its D3) and `SelectionToolbar.svelte` (which replaces the toolbar's action
  row while a selection exists, its D6). Its D10 fixes the shape of a bulk write: one transaction
  over every id, never a loop over the single-image command.
- Rust owns the filesystem and storage; the webview is UI only (§6). Every command goes through
  one `Connection` behind a `Mutex` (Phase 1 D1) and is wrapped once in `lib/api/` (Phase 1 D12).
- `packages/shared/src/index.ts` and `src-tauri/src/model.rs` are hand-mirrored (Phase 1 D11):
  every type here is written twice, in one commit.

## Goals / Non-Goals

**Goals:**

- One grid, one search, one Inspector, two views. The trash is a parameter, not a second screen.
- The reversible act and the irreversible one look different, are reached differently, and cannot
  be confused: one is a key press away, the other is behind a confirmation that counts what it
  will destroy.
- A permanent delete either removes the record or changes nothing; it never half-succeeds
  silently, and it never lies about a file it could not remove.
- Every number on screen that says how much the library holds keeps meaning what it meant before
  this change, and the trash is added beside them rather than folded into them.

**Non-Goals:**

- Making the trash a general-purpose version history. It holds rows the user deleted, in the
  state they were deleted in; it does not record edits.
- An index or query plan for the deleted filter. `deleted_at IS NULL` scans, as it did in Phase 1,
  and the trash count is one scan of one column; a ten-thousand-row library does not notice.
- Trash semantics for anything but images. Tags, rules and notes are not trashed.

## Decisions

**D1. `/trash` is a route, and it renders the same library screen with a `view` parameter.**

`routes/+page.svelte`'s body moves into `lib/components/library/LibraryScreen.svelte`, which
takes `view: 'library' | 'trash'`; `routes/+page.svelte` and `routes/trash/+page.svelte` are two
lines each. Everything below the toolbar — the search inputs, `SearchResults`, `LibraryGrid`,
`Lightbox`, `Inspector`, the selection store — is identical in both; the view decides the request
sent to `search`, which actions the tile menu, the Inspector and the selection toolbar offer, and
what the empty state says.

A route rather than a toggle button on `/`, because Sidebar · nav is a list of routes
(`app-shell` D2 fixes `/`, `/start`, `/settings`) and a nav entry that changed no URL would be
the only one in that list that behaved differently. It also has to survive a reload and a back
gesture: a view held in component state would drop the user into the library after a hot reload
while the sidebar still highlighted `Trash`.

Alternative — a second `TrashGrid` component and a second screen — rejected outright: the grid,
the search, the lightbox and the Inspector would exist twice, and every later change (`tags`,
`ratings`, `booru-upload`) would have to land in both or explain why not.

**D2. `SearchRequest.includeDeleted: boolean` becomes `view: 'library' | 'trash'`.**

The trash needs "only deleted", which a single boolean cannot say. Keeping the boolean and adding
`onlyDeleted` beside it gives four combinations of which one is nonsense ("only the deleted ones,
but do not include deleted ones") and leaves the compiler unable to reject it. A two-variant enum
has exactly the states the app has screens for.

The field is replaced, not deprecated. Its only production caller is `buildSearchRequest`, which
passes `false`, and it now passes the screen's view. Its test callers split by what they were
asserting: the ones that set `include_deleted: false` become `view: Library` unchanged; the ones
that set `true` meant "find this row regardless" and each becomes the view the row is actually in
— `Trash` where the fixture marked it deleted, `Library` where it did not. Rust names the enum
`SearchView` with serde `rename_all = "camelCase"`, matching the wire values.

No third `all` variant. Nothing in the app shows the library and the trash at once, and a variant
with no caller is the dead control `app-shell`'s proposal forbids on screen and this design
declines to add to a type.

`search_ids(req)` (`selection-and-bulk` D3) takes the same `SearchRequest`, so select-all inside
the trash resolves trashed ids with no new query, and bulk restore over a whole trash costs one
round trip.

**D3. A soft delete writes `deleted_at` and `updated_at`, and nothing else.**

The file stays. The thumbnail stays, because the trash draws the same grid and regenerating
thumbnails on the way into a view the user reached to look at pictures would be work done for no
reason. Tags, rating, `posts`, `source`, `captured_at` and the FTS row all stay: restore has to
return the image *as it was* (spec `trash`), and a delete that stripped anything would be a lossy
delete wearing a trash's clothes.

`updated_at` moves because the row changed; `captured_at` does not, because the image was not
re-captured — which is also what keeps a restored image in its original place in the grid's order
(D10).

Nothing collects orphan tags on a soft delete. A tag carried only by trashed images is still
carried by images the library holds (§2 guarantee 2), so it stays in the vocabulary that
`tag_suggestions(prefix, limit)` reads and simply does not appear in `tag_counts(req)` for a
library-view search, because that request no longer matches those rows. The orphan collection
`tags-and-ratings` owns runs when a tag is *removed from an image*, and permanent deletion is the
only thing in this change that removes one — so `delete_forever` runs it and `trash_images` does
not.

**D4. Permanent delete: the rows go in one transaction, then the files are unlinked, and what
could not be unlinked is named rather than swept.**

Order first. Unlinking before the row delete leaves, if the row delete fails or the process dies
between the two, a library holding a record that points at nothing — the exact state the user
believed they had removed, now presented as a broken card they must delete a second time.
Deleting the row first leaves, in the same failure, bytes under `images/` that no screen shows
and no query reaches: invisible, harmless, and reclaimable in Finder. Between an inconsistency
that contradicts the user's intent and one that costs disk, the second is the one to choose.
Phase 1 D16 already made orphan files a state this library tolerates, so nothing new has to be
taught to survive them.

The row delete is one transaction over every id, per `selection-and-bulk` D10: fifty rows deleted
by fifty transactions is fifty chances to stop half way with no way to tell which half went. The
cascades and the `images_fts_delete` trigger do the rest, exactly as `drop_image_record` does
today.

Then, outside the transaction, each file is unlinked and each thumbnail removed with the existing
`remove_if_present` (a thumbnail is a derived cache; its absence has never been a failure, Phase 1
D7). A file that will not go — a Windows handle held by a preview or an antivirus scanner, a
read-only volume — is collected into `DeleteReport { deleted, filesLeft }` and shown, with its
full path, to the user.

No automatic sweep. The sweep that suggests itself — at library open, delete every file under
`images/` with no row — is refused: it would destroy a file a user copied into the folder by hand,
and it would destroy the entire library folder the one time `library.sqlite` is replaced or
restored from an older copy, which is precisely the cloud-sync accident §7 and §10 already worry
about. Guarantee 2 says nothing auto-deletes, and a sweep is the app deleting files it was never
asked to delete. The report is the alternative: the app says what it could not do and where, and
the user decides (§2 guarantee 4's posture, applied to the app's own folder). Re-running the
delete is not possible — the row is gone — which is why the path is put in front of the user
rather than into a log.

**D5. Permanent delete unlinks the file, reversing Phase 1 D16's "dropping a record leaves
`images/` alone".**

Why the old reading was right at the time: Phase 1's only deletion was offered on a card whose
file was already gone, and the app had nowhere to put a row a user regretted removing. An action
that unlinked the file would have been the single unrecoverable thing in an app whose §2
guarantees are all about not losing what the user has, and D16 said as much — the destructive
variant was to be "a later decision, not a silent one".

Why it stopped being right: this change is that decision, and it supplies the thing D16 lacked.
Deletion is now two acts, not one. The first — move to trash — writes `deleted_at` and touches
no byte on disk, and it is the one bound to a key, offered in menus, and applied to selections.
Only the second — "Delete forever", reached from inside the trash, confirmed with a count, bound
to nothing — unlinks. The user reaches the destructive act by choosing it twice, which is what
D16 was protecting and what it could not express with one action. Meanwhile the state D16 left
behind stopped being tolerable: a library whose delete never removes a file accumulates images no
screen can show and no action can reclaim, and the only way to get the disk back was Finder — in
an app whose entire proposition is that the folder is the product. The half of D16 that still
holds is kept verbatim: nothing that merely takes an image out of view unlinks anything.

**D6. `drop_image_record` is removed, and the missing-file card offers the trash instead.**

The command and `delete_forever` would differ in one thing: whether an unlink is attempted. For
the row `drop_image_record` exists to serve — an image whose file is already gone — that unlink
finds nothing and does nothing, so the two are the same action with two names and two call sites.
`maintenance::drop_image_record` becomes the per-id half of the permanent delete (row plus
thumbnail), `delete_forever` wraps it with the unlink, and the Tauri command and its
`lib/api/commands.ts` wrapper go.

That leaves the missing card without its action, and moving to trash is the better replacement on
its own terms, not just as a substitute. The `library-folder` spec's other scenario is "File
restored": a file that vanished may come back — an unmounted volume, a sync client catching up, a
rename undone — and the spec requires the image to render again when it does. Dropping the record
made that scenario unreachable for any row the user had tidied away, and did it irreversibly.
Trashing the record keeps it: the file returns, the missing pass clears the flag, and Restore puts
the image back where it was. It also collapses the grid's two "get rid of this" actions into one,
with one meaning and one way back — two removals on one grid with different recoverability is the
kind of trap that only shows up after it has cost something.

The card's copy changes with it: "Remove record… / Nothing on disk is deleted" becomes "Move to
trash", and the comment in `ImageCard.svelte` that cites D16 is replaced by one citing this
decision.

**D7. Empty trash is `delete_forever` over every trashed id, behind a confirmation naming the
count.**

`empty_trash()` reads the trashed ids and calls the same function, rather than issuing its own
`DELETE … WHERE deleted_at IS NOT NULL`: a second statement would be a second place the unlink,
the thumbnail removal, the orphan-tag collection and the report have to be kept in step, and it
would drift the first time one of them changes.

The confirmation names the number ("Permanently delete 37 images? This cannot be undone."), as
the legacy viewer's empty-trash handler did. Permanent delete and Empty trash are the only two
irreversible actions in the app and the only two that confirm; a confirmation on a reversible
action would teach the user to dismiss confirmations. With an empty trash the action is not
offered as something that would do anything.

**D8. Nothing is purged on a timer, and there is no setting to make it so.**

§2 guarantee 2 is "nothing auto-deletes" and guarantee 4 is that the app never claims data is
safe to remove. A retention window would be the app deciding on the user's behalf that some data
was safe to remove, and doing it while they were not looking — the exact failure both guarantees
name. Recorded as a decision rather than left unmentioned, because "empty the trash after 30
days" is the default in every product this borrows its vocabulary from, and a later reader will
otherwise assume it was forgotten.

Alternative — a configurable retention defaulting to "never" — rejected: a setting whose only
safe value is off is a trap, and the trash's size is already bounded by the user's own deletions.
The bound on disk is therefore explicit and visible: the count is in the sidebar, always.

**D9. The per-source counts keep excluding trashed images; the trashed count is a line beside
them.**

Their job is §9 step 4 — put the app's number next to the browser's and see whether the migration
landed. The legacy viewer's own count excludes its trash (`getImageCount` filters `isDeleted`)
and shows the trash as a separate badge, and the Phase-0 bundle imports trash as trash (§8). If
the app folded trashed rows into the total, a 1000-image browser library with 40 in its trash
would show 960 in the browser and 1000 in the app, and the one comparison these numbers exist for
would report a discrepancy that is not there. So `image_counts`, `Library::image_count`,
`LibraryStatus.imageCount` and `GET /status` are all left exactly as they are.

"What the app holds" (§2 guarantee 4) is still answered, in two places rather than folded into
one: the sidebar's Trash badge while browsing, and an "In trash" line beside the per-source counts
on Settings → Library (Slot: Settings · Library), where the reconciliation actually happens.
Nothing is hidden and nothing is double-counted. This is also why `library-browse` needs no delta:
its counts requirement describes numbers this change does not touch.

**D10. The trash uses the grid's one ordering; there is no "recently trashed" sort.**

`selection-and-bulk` D3 rests on there being exactly one `ORDER BY captured_at DESC, id DESC` in
the codebase — "the selection's row *n* and the grid's row *n* must be the same image, and they
are only guaranteed to be while one exists". A view-specific ordering would put a second one
beside it, reachable only from the trash, where a range selection followed by Restore would be the
first thing to expose a mismatch. Ordering choices belong to `sort` in `SearchRequest`, which
`tags-and-ratings` adds and which means the same thing in both views.

The cost is that "what did I just delete" is not the top-left tile. The badge answers how many,
and the search narrows the trash as it narrows the library, which is the same answer the user has
for the library itself.

**D11. `trash_count()` is its own command, not a field on `LibraryStatus`.**

`LibraryStatus` is what the layout gate reads on every route change and what `library_status`,
`open_library` and `pick_library` all return; `imageCount` is on it because it is the library's
size, the number `GET /status` reports to the extension and the migration notice (§5). The trash
count is a navigation badge with exactly one class of writer — the four commands in this change —
and nothing else invalidates it: a capture arriving over HTTP, an import, a tag edit and a library
switch all leave it alone (a switch replaces it wholesale, which is a fresh read either way).

The trash commands return their own results, not a status: `delete_forever` and `empty_trash`
return `DeleteReport`, because which files were left behind is the thing the caller has to show.
A count carried on a status those commands do not return would be refreshed by a second call
anyway, so the second call is the design.

**D12. `Delete` and `Backspace` move to the trash, and nothing destroys from the keyboard.**

Two rows are added to `app-shell`'s keyboard map — this design's addition to that table, stated
here so the table stays the one place bindings are listed:

| Where | Key | Action |
| --- | --- | --- |
| Grid (library view) | `Delete` `Backspace` | move the selection, or the focused image, to the trash (two or more images ask first) |
| Grid (trash view) | `Delete` `Backspace` | nothing |

They ride `LibraryGrid.svelte`'s existing keydown handler behind `isTypingTarget` (`app-shell`
D14, `selection-and-bulk` D5), so neither fires while the user is editing a search field or a tag.
Both keys, unmodified: `Delete` is what a Windows user presses and `Backspace` is what the key is
called on a Mac keyboard, and requiring a modifier for an action that cannot lose anything would
only make it slower.

An unmodified destructive-looking key is acceptable *because* the action it fires is the
reversible one — that is the whole reason the two acts are separated. In the trash view the same
keys do nothing at all: binding them to permanent deletion would put the app's only unrecoverable
action one keystroke from a grid the user reached by clicking around, and no confirmation dialog
survives contact with a key that is pressed reflexively.

**Amended after the owner's pass: trashing two or more images asks first.** The paragraph above
stands for what it was written about — one image, one keystroke, one reversible act — and it is
still why a single `Delete` on a focused card asks nothing and why the trash view binds neither
key. What it did not weigh is scale: `Cmd A` then `Backspace` is two keystrokes that empty the
whole library into the trash, and the same reflex the argument protects (a key pressed without
looking) is what makes that reachable by accident. Undo-ability is not the whole of the cost — a
thousand-image trash is a thousand images the user now has to select and restore to get back
where they were, and the count is the one fact the keystroke never showed them. So the rule is
the count, not the path: any trash write naming two or more images goes through the same
`ConfirmDialog` ("Move N images to the trash?", confirm "Move to trash"), whichever control asked
for it — the keys, the selection toolbar, the tile menu, the tile's overlay button. One image is
unchanged, in every path. The dialog does not say "cannot be undone", because it can be: that
sentence stays the mark of the two acts that destroy.

The decision is enforced where the writes converge rather than at each caller —
`LibraryScreen`'s `actions.trash` asks `needsTrashConfirmation(ids.length)` — so a control added
later cannot forget it. The selection is untouched until the answer comes back: dismissing the
dialog leaves the user exactly the selection they made.

**D13. Where each control lands (`app-shell` slot map, filled).**

| Slot | This change puts there |
| --- | --- |
| Sidebar · nav | `Trash` entry to `/trash`, with the count as a badge |
| Grid · tile | context menu entry — "Move to trash" in the library view, "Restore" and "Delete forever…" in the trash view; and, in the hover overlay beside the selection checkbox, an icon button for the reversible one of that pair |
| Inspector · actions | the same pair of actions for the one image the panel is showing |
| Toolbar · actions | inside `SelectionToolbar`: "Move to trash" in the library view; "Restore" and "Delete forever…" in the trash view. On `/trash` with nothing selected, the `Import` menu is replaced by "Empty trash…" |
| Settings · Library | the "In trash" line beside the per-source counts (D9) |

The tile context menu is created by `tags-and-ratings` (remove tag, set rating); this change adds
entries to it rather than a second menu.

**Amended after the owner's pass: the overlay button beside the checkbox.** The menu entry was
meant to be the whole of the tile's trash action, on the reasoning that the grid's hover overlay
belongs to selection and the menu is where per-tile actions live. The owner met the missing-file
card — whose only control *is* a "Move to trash" / "Restore" button — reached for the same thing
on a tile that rendered, and found nothing: the action existed but had to be discovered by
right-clicking. So the reversible half of the pair is on every tile now, as a small icon button in
the same hover overlay as the selection checkbox, shown on hover and focus-visible, named by
`aria-label` and `title`. Only the reversible half: "Delete forever…" stays behind the menu, where
an action that cannot be undone is not one stray click from a thumbnail. The menu entries stay as
they are — the overlay is a shortcut to one of them, not a replacement, and the menu is what a
keyboard user reaches without hunting. Being one tile's action it is one image, so it never
confirms (D12, amended), and it leaves the selection alone; it does make its tile the current card, so the row it empties is where the focus and the scroll come back to after the write, not wherever the focus was. `Import` is replaced rather than hidden on `/trash`
because importing into the trash is meaningless and an action row with one dead control is worse
than one with the action the screen actually has.

**Amended 2026-09-25 (`menu-polish` design D6): "Move to trash" is red now, same as "Delete
forever…".** Why the old reading was right at the time: colour marked the one act that cannot
be undone, and that reading held while colour was the only mark in the menu — a plain "Move to
trash" beside a red "Delete forever…" read as the deliberate pair it was. Why it changed: the
owner (2026-09-25) wants colour to read as the action's identity, not as a warning, in a list of
items that are otherwise uniform. What still marks the irreversible act: the ellipsis and the
confirmation stay on "Delete forever…" alone — "Move to trash" is still one click, still
unconfirmed for one image (D12, amended), and still the reversible act; only its colour changed.

**D14. A capture whose id is already in the trash stays idempotent and stays trashed.**

`POST /captures` is idempotent by the extension's UUID (§5, Phase 1 D4: `ON CONFLICT(id) DO
NOTHING`, 200 with the existing record). An id only recurs on a Retry of the same capture, so if
the user trashed it in between, the honest answer is the one the code already gives: the capture
was delivered, then the user deleted it, and the retry changes nothing. Written down because
"untrash on re-ingest" looks like a bug fix to anyone who meets this case without the argument,
and it would silently resurrect images the user deleted.

**D15. No schema change, no migration.**

`deleted_at` is Phase 1 D2's schema v1 column and needs nothing added. Schema v2
(`images.adapter_json`) belongs to `bridge-extension`; this change introduces no version, so v3
remains free for whichever Phase 2 change first needs one. Stated because a Phase 2 change that
quietly added a migration would collide with a parallel one.

**D16. The trash opens on what was trashed last, through a sort of its own.**

Slot: Toolbar · view, trash only. `Delete` and `Backspace` trash without asking (D12), so a slip
has to be cheap to undo — and it is only cheap if the image is where the user looks first. With
the library's orders the slip lands wherever its capture time puts it, and a user who does not
remember which image they hit has nothing to search for. So `SortField` gains `trashed`
(`deleted_at`), the trash view's `SearchResults` defaults to it descending, and `ViewControls`
offers "Trashed last" / "Trashed first" in the trash view only: on a library row `deleted_at` is
NULL and the order would mean nothing. The library's default (newest capture) is untouched.

Rejected: ⌘Z undo. An undo stack is a feature over every write, not over the trash, and it would
still leave the user unable to see what they undid. It stays a follow-up; the order is what makes
the slip visible.

## Risks / Trade-offs

- ["Delete forever" leaves a file behind on a locked or read-only volume, and the row is already
  gone] → `DeleteReport.filesLeft` names the full path on screen; nothing in the library refers to
  the file any more, so the only cost is disk the user can reclaim in Finder. Refused the
  alternative (a startup sweep of `images/` against the database) in D4, because it destroys
  hand-added files and would empty the folder if the database were ever restored from an older
  copy.
- [`Delete` on a focused card is pressed by accident] → the action is reversible by construction,
  the Trash badge in the sidebar increments where the user is looking, and Restore is one click
  away in a view that is one click away. This is the trade the two-act split exists to make.
- [Replacing `includeDeleted` with `view` is a breaking IPC change while three other Phase 2
  changes are in flight] → it lands in group 1 as one commit across `packages/shared` and
  `model.rs` (Phase 1 D11), and every caller is inside this repo; a missed one is a type error,
  not a runtime surprise. `tags-and-ratings` adds `sort` and `group` to the same type, so the two
  changes touch it in sequence rather than in parallel (the sequencing already has this change
  after it).
- [The trash grows without bound because nothing purges it] → deliberate (D8); the count is
  permanently in the sidebar, which is the visible bound, and Empty trash is one action.
- [A permanent delete of an image with a `posts` row loses the record that it was posted to a
  booru] → the `posts` cascade is the correct behaviour for a row that no longer exists, and §6's
  pull-back feature reads posts of images the library holds. Noted because it is the one fact in
  this change that cannot be restored even in principle once the image is gone.
- [Trashed rows still cost every library query the `deleted_at IS NULL` predicate] → unchanged
  from Phase 1; the filter has been on every search since v1 and no index was needed for it.

## Open Questions

- Whether the trash view should offer a bulk "Restore all", the mirror of Empty trash. It is one
  button calling `restore_images` with the ids `empty_trash` would have taken, changes no
  requirement and no other task, and is worth deciding against a trash the owner has actually
  filled.
- Whether the missing-file card should offer "Delete forever" directly once the row is in the
  trash and the file is known to be gone. Both paths already exist; this is a shortcut, not a
  behaviour.
