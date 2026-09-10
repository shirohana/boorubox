> Lands after `app-shell` and `selection-and-bulk` (proposal.md, "Depends on"). Group 1 is
> `packages/shared` + `packages/app/src-tauri` + the command wrappers and can be done alone;
> groups 2–4 all touch `packages/app/src/lib/components/library/` and the routes, so they are one
> agent's work in order. Two implementing agents at most: **A** owns group 1, **B** owns groups
> 2–4 and cannot start before 1.5 lands.
>
> Task 1.1 is a breaking change to `SearchRequest` — one commit across both mirrored files
> (Phase 1 D11) — so nothing else in the repo should be mid-edit on that type when it goes in.

## 1. Rust and the shared contract: the view split and the trash commands

- [x] 1.1 `packages/shared` + `packages/app/src-tauri`: replace `SearchRequest.includeDeleted`
      with `view: 'library' | 'trash'` in `src/index.ts` and `SearchView` in `model.rs` (serde
      camelCase), add `DeleteReport { deleted, filesLeft }` to both, and update every caller —
      `query::compile` (`deleted_at IS NULL` for `Library`, `IS NOT NULL` for `Trash`),
      `buildSearchRequest` in `src/lib/api/search.svelte.ts`, and the Rust tests that set
      `include_deleted` (design D2 — the `false` ones become `Library`, the `true` ones become the
      view their fixture row is actually in); verify a serde test pins the camelCase JSON for both
      types, `query.rs` tests that `Library` excludes a row marked deleted and `Trash` returns
      only marked rows and never an undeleted one, and `mise run typecheck` passes.
- [x] 1.2 `packages/app/src-tauri`: new `trash.rs` with `trash_images(ids)` and
      `restore_images(ids)` — one transaction over every id setting or clearing `deleted_at` and
      moving `updated_at`, nothing else touched (design D3, `selection-and-bulk` D10) — plus their
      commands; verify Rust tests that a trashed image is absent from a `Library` search and
      present in a `Trash` search, that its file under `images/`, its thumbnail, its tags and its
      rating are all still there, that `image_counts` and `Library::image_count` drop by one while
      the row still exists, that restore returns it to the `Library` search with its tags and
      rating and clears `deleted_at`, and that an unknown id anywhere in `ids` fails the call with
      no row changed.
- [x] 1.3 `packages/app/src-tauri`: `delete_forever(ids)` — one transaction deleting the rows
      (cascades plus the `images_fts_delete` trigger), committed, then per id the file under
      `images/` unlinked, the thumbnail removed with `remove_if_present`, and the orphan-tag
      collection run once; unlink failures collected into `DeleteReport.filesLeft` rather than
      raised (design D4). Fold `maintenance::drop_image_record` in as the per-id row-and-thumbnail
      half and delete the `drop_image_record` command (design D6); verify Rust tests that the row,
      its `image_tags`, its `posts`, its FTS entry, its thumbnail and its file are all gone, that a
      row whose file was already removed deletes cleanly with an empty `filesLeft`, that
      `#[cfg(unix)]` with `images/` made read-only the row is still gone and the id's path is in
      `filesLeft`, that a tag whose last use was deleted is gone from `tags`, and that the existing
      `maintenance` tests for the row-and-thumbnail behaviour still pass through the new path.
- [x] 1.4 `packages/app/src-tauri`: `empty_trash()` — read every trashed id and call
      `delete_forever` on them, one code path (design D7) — and `trash_count()`, one
      `SELECT COUNT(*) FROM images WHERE deleted_at IS NOT NULL` (design D11); verify Rust tests
      that emptying a library with three trashed and four live images leaves the four untouched
      with their files, that the returned `DeleteReport.deleted` is three, that emptying an empty
      trash is a no-op reporting zero, and that `trash_count` tracks trash / restore / delete /
      empty across a sequence.
- [x] 1.5 `packages/app` + `packages/app/src-tauri`: register the five commands in
      `generate_handler!`, wrap each once in `src/lib/api/commands.ts` with its `index.ts` export,
      and remove `dropImageRecord` and its wrapper; verify `commands.test.ts` gains a case per
      command asserting the command name and the argument keys that cross the IPC boundary, has no
      case left for `drop_image_record`, and `mise run typecheck` fails nowhere on the removed
      wrapper.

## 2. The Trash entry, its count, and the counts screen

- [x] 2.1 `packages/app`: `src/lib/api/trash.svelte.ts` — the trash count as reactive state with
      a `refresh()` over `trash_count()`, called when a library opens and after every trash,
      restore, delete-forever and empty (design D11); verify a unit test with a stubbed command
      that `refresh()` reads the command once and publishes the number, and that a failed call
      leaves the last known count rather than showing zero.
- [x] 2.2 `packages/app`: the `Trash` nav entry in the sidebar pointing at `/trash`, with the
      count as a badge (Slot: Sidebar · nav); verify by hand that the badge shows the number
      `trash_count` returns, updates without a reload after trashing and restoring an image, and
      that the entry highlights on `/trash` exactly as `Library` does on `/`.
      Hand check: trash an image and restore it from `/trash`; the badge beside the sidebar's
      Trash entry follows both without a reload, is absent while the trash is empty, and the
      entry takes the active highlight on `/trash` as Library does on `/`.
- [x] 2.3 `packages/app`: the "In trash" line beside the per-source counts on Settings → Library
      (Slot: Settings · Library, design D9); verify by hand that with 5 images trashed out of 100
      the per-source counts still total 95 and the new line reads 5, and that no existing count on
      that screen changed.
      Hand check: with images in the trash, open Settings → Library; the four per-source counts
      are the numbers they were before this change and exclude the trashed rows, and the
      "In trash" line under them reads the badge's number.

## 3. The grid in two views

- [x] 3.1 `packages/app`: extract `routes/+page.svelte`'s body into
      `src/lib/components/library/LibraryScreen.svelte` taking `view: 'library' | 'trash'`, thread
      it into `buildSearchRequest`, and add `routes/trash/+page.svelte` rendering it with `trash`
      (design D1); verify by hand that `/` behaves exactly as it did — search, tile size, import,
      lightbox, inspector — and that `/trash` shows only trashed images, with an empty state that
      says the trash is empty rather than naming a query when no query is set.
      Hand check: on `/` run a search, drag the tile-size slider, import, open the lightbox and
      the inspector — all unchanged; then open `/trash` and confirm only trashed images are
      listed, that with no query the empty state reads "The trash is empty", and that a query
      matching nothing there still names the query.
- [x] 3.2 `packages/app`: the tile context menu entries (Slot: Grid · tile, design D13) — "Move to
      trash" in the library view, "Restore" and "Delete forever…" in the trash view, added to the
      menu `tags-and-ratings` created rather than a second menu; verify by hand that each acts on
      the tile the menu was opened on, that the grid and the badge update without a reload, and
      that "Delete forever…" opens the confirmation and does nothing when it is dismissed.
      Hand check: right-click a tile on `/` and choose "Move to trash" — that tile leaves the
      grid and the badge rises; right-click one on `/trash` and choose Restore, then another and
      choose "Delete forever…", dismissing the dialog: nothing changes.
- [x] 3.3 `packages/app`: `Inspector.svelte` gains its action row (Slot: Inspector · actions) with
      the same pair of actions for the image it is showing, in both its grid placement and the
      lightbox's inspect mode (`app-shell` D9); verify by hand that trashing from the Inspector
      empties the panel to its no-selection state, that restoring from the trash view does the
      same, and that the row is absent when no image is shown.
      Hand check: focus a card and trash it from the inspector's action row — the panel falls
      back to "No image selected"; do the same with Restore on `/trash`; open the lightbox, press
      `i`, and confirm the same row is there; with nothing focused the row is absent.
- [x] 3.4 `packages/app`: `LibraryGrid.svelte`'s keydown handler gains `Delete` and `Backspace` —
      the selection if there is one, otherwise the focused image, moved to the trash in the library
      view and doing nothing in the trash view — behind the existing `isTypingTarget` guard (design
      D12); verify a unit test that both keys are ignored in the trash view and while the guard is
      true, and a manual pass that pressing either in the library grid trashes the focused image,
      moves the focus to the next card, and leaves the `app-shell` and `selection-and-bulk`
      bindings unchanged.
      Hand check: focus a card in the library grid and press `Delete`, then `Backspace` — each
      trashes the focused image and leaves the focus on the card that took its row; select a few
      and press either to trash the whole selection; press both on `/trash` and nothing happens;
      the arrows, `Esc`, `Enter`, `i` and `Cmd A` are unchanged.
      Human update: `Cmd A` + `Backspace` will move all the images to Trash, too hardcore, expect a confirmation for removing multiple images.
      Probe (2026-09-10, agent-driven app on a scratch copy of the vault): Backspace on 5 selected asked "Move 5 images to the trash?" with a plain Move to trash button; Escape kept the 5 selected. Still yours from the review: focus the last card, select the last 3, Backspace, confirm — check the focus does not fall to the body.
      Human update (2026-09-10): a slip with Delete/Backspace is hard to take back — order the
      trash by removal date, newest first, or add ⌘Z.
      Agent (2026-09-10): design D16 — `SortField` gains `trashed` (`deleted_at`), the trash
      view's results default to it descending, and the sort control offers "Trashed last" /
      "Trashed first" in the trash view only (Rust `the_trash_view_orders_by_trash_time`, store
      test `the trash defaults to the last trashing first`; spec requirement "The trash opens on
      what was trashed last"). ⌘Z rejected there: an undo stack is a feature over every write and
      still would not show what it undid. Hand check: Backspace on a tile, open /trash — it is
      the first tile and the control reads "Trashed last".
      Agent (2026-09-10): trashing two or more images now asks first, through the one
      `ConfirmDialog` the screen already had — "Move N images to the trash?", confirm "Move to
      trash", no "cannot be undone" because it can be. One image (focused, or a selection of one)
      is unchanged and asks nothing. The rule sits on `LibraryScreen`'s `actions.trash`
      (`needsTrashConfirmation(ids.length)` in `trash-actions.ts`, unit-tested), so every path
      that names several — the keys, the selection toolbar, a menu — asks, and a later control
      cannot forget to. Design D12 amended with why the "no confirmation on a reversible action"
      argument stopped covering the multi case; the delta spec's selection and keyboard scenarios
      follow. Re-check: `Cmd A` then `Backspace` names the whole count and, dismissed, leaves the
      selection exactly as it was; `Delete` on one focused card still trashes it with no dialog;
      Restore and the tile's own button still ask nothing.
- [x] 3.5 `packages/app`: `ImageCard.svelte`'s missing-file state offers "Move to trash" instead of
      the "Remove record…" confirmation, with the D16 comment replaced by one citing design D6
      (spec `library-folder`); verify by hand with a file deleted from `images/` that the card
      offers the trash, that the record lands in the trash with the missing state intact, and that
      putting the file back and restoring the record renders the image again.
      Hand check: delete a file under `images/` in Finder and reload — the card offers "Move to
      trash" and no action that drops the record; trash it, put the file back, then Restore it
      from `/trash` (the missing card there offers Restore) and confirm the image renders again.

## 4. Trash actions on a selection

- [x] 4.1 `packages/app`: `SelectionToolbar.svelte` gains "Move to trash" in the library view and
      "Restore" plus "Delete forever…" in the trash view, resolving the selection to ids through
      the selection store and refreshing the current search and the trash count afterwards (Slot:
      Toolbar · actions, `selection-and-bulk` D6, design D13); verify by hand that a range
      selection spanning unloaded rows moves entirely to the trash, that the selection is cleared
      afterwards because the images left the result, and that "Delete forever…" names the exact
      selection count in its confirmation.
      Hand check: select a range spanning rows that never scrolled into view and move it to the
      trash — every image goes and the selection toolbar is gone afterwards; on `/trash` select
      several and confirm "Delete forever…" names exactly that many.
      Human update: on a broken image, it shows *Move to trash* in Library and *Restore* in Trash, that's so convenience. But regular images doesn't have that action to use.
      Probe (2026-09-10, agent-driven app on a scratch copy of the vault): with 3 selected elsewhere, the overlay trash on an unselected tile trashed that one image (50 left, Trash 3), the 3 stayed selected, the grid did not scroll, and the ring sat on the tile that took its row.
      Agent (2026-09-10): every tile now carries that button, not just the missing-file card:
      an icon button in the hover overlay beside the selection checkbox (`ImageCard.svelte`),
      "Move to trash" on `/` and "Restore" on `/trash`, `aria-label` + `title`, shown on hover,
      on focus-within and while the tile is current or selected. It is outside the tile's button
      like the checkbox, so it neither opens the image nor changes the selection (it does make its
      tile current, so the focus returns to that row after the write), and being one
      image it never confirms. "Delete forever…" stays in the context menu only. Design D13's
      slot map amended (Grid · tile) with the reason. Re-check: hover any tile in both views,
      Tab reaches the button after the card without breaking the arrow-key roving focus, and the
      grid keeps its scroll position after the write.
- [x] 4.2 `packages/app`: "Empty trash…" replaces the `Import` menu in the toolbar's action row on
      `/trash` while nothing is selected, with the confirmation naming the count and the
      `DeleteReport` shown as a dismissible line when any file was left behind (design D7, D13);
      verify by hand that the button is absent on `/`, that it is not offered as an action when the
      trash is empty, that confirming empties the trash and zeroes the badge, and that dismissing
      the confirmation changes nothing.
      Hand check: on `/` the toolbar still shows Import; on `/trash` with an empty trash it
      shows neither; with images in the trash it shows "Empty trash…", whose dialog names the
      badge's count — dismissing it changes nothing, confirming empties the trash and zeroes the
      badge.

## 5. Verification

- [ ] 5.1 `mise run check` passes (lint, typecheck, tests, clippy, builds).
- [ ] 5.2 End-to-end trash pass in the running app: trash an image from the tile menu, from the
      Inspector, and with the `Delete` key; confirm each leaves the grid, drops the library total
      in the sidebar and raises the Trash badge; open `/trash` and confirm the three are there with
      their thumbnails, tags and ratings; search the trash by a tag and confirm it narrows; restore
      one from the tile menu and one from a selection and confirm both return to the library in
      their original place in the order with their tags intact.
- [ ] 5.3 Permanent deletion pass: from `/trash`, "Delete forever…" on one image — confirm the
      dialog names one, that dismissing it changes nothing, and that confirming removes the row,
      the thumbnail under `.thumbs/` and the file under `images/` (check in Finder); then select
      several and confirm the dialog names the selection count; then "Empty trash…" and confirm the
      dialog names the remaining count, the badge goes to zero, and the library's own images and
      files are untouched.
- [ ] 5.4 Guarantee pass: with images in the trash, quit and relaunch, and confirm the trash holds
      exactly the same images with the same count; switch to another library and back and confirm
      the same; confirm nothing anywhere in the app offers a retention period (§2 guarantee 2,
      design D8).
- [ ] 5.5 Missing-file pass: delete a file from `images/` outside the app, confirm the card offers
      "Move to trash" and no action that destroys the record outright, trash it, put the file back,
      restore it from the trash, and confirm the image renders again (spec `library-folder`).
- [ ] 5.6 The Phase 1 and `app-shell` end-to-end lists still pass, with the two trash-specific
      changes: a `curl` capture with `Origin: chrome-extension://test` still lands (201) and a
      retry of an id that is now in the trash returns 200 and leaves it in the trash (design D14);
      per-source counts on Settings → Library still exclude trashed rows and the "In trash" line
      reports them (design D9).

## Handoff notes (agent A)

Group 1 is done: Rust core, shared contract, and the `lib/api/` command wrappers. Gate results
below; group 2 (agent B) can start.

**The request shape after 1.1.** `SearchRequest.view: 'library' | 'trash'` replaces
`includeDeleted: boolean`, in both `packages/shared/src/index.ts` and (as `SearchView`, a
`#[serde(rename_all = "camelCase")]` enum with `Library` / `Trash` variants) `model.rs`.
`query::compile` matches on it: `Library` → `images.deleted_at IS NULL`, `Trash` → `IS NOT NULL`.
No third `all` variant, per design D2.

In the webview, `lib/api/search.svelte.ts`'s `SearchView` interface (the one holding `sort` and
`group` — an unrelated, pre-existing name collision with the Rust enum, not worth renaming) grew a
third field: `view: 'library' | 'trash'`. `buildSearchRequest` reads it straight into the request.
`SearchResults` now takes an optional constructor argument — `new SearchResults(view: 'library' |
'trash' = 'library')` — stored as the public readonly `results.view`, and its internal `#view`
getter includes it. **This is the hook for the trash screen**: `LibraryScreen.svelte` (task 3.1)
should instantiate its `SearchResults` as `new SearchResults(view)` where `view` is the prop it
takes, and thread the same value into every `buildSearchRequest` call site it owns (there is
exactly one left outside `search.svelte.ts` itself — the `Selection`'s `IdResolver` in
`routes/+page.svelte`, which now passes `view: results.view`; do the same in whatever replaces it).

**New commands** (all in `commands.ts`/`commands.rs`, registered in `generate_handler!`):
- `trashImages(ids: string[]): Promise<void>` → `trash_images`
- `restoreImages(ids: string[]): Promise<void>` → `restore_images`
- `deleteForever(ids: string[]): Promise<DeleteReport>` → `delete_forever`
- `emptyTrash(): Promise<DeleteReport>` → `empty_trash`
- `trashCount(): Promise<number>` → `trash_count`

`DeleteReport { deleted: number, filesLeft: string[] }` is the new shared type (also in
`model.rs`). No new events.

`dropImageRecord` / `drop_image_record` are gone (command, TS wrapper, and
`maintenance::drop_image_record`). The row-and-thumbnail logic it held now lives inside
`trash::delete_forever`, which also unlinks the file — reversing Phase 1 D16, per design D5/D6.

**Minimal fix already made outside `lib/api/`:** `routes/+page.svelte`'s `forget()` (wired to
`ImageCard`'s missing-file action) called `dropImageRecord` and no longer compiles once that's
gone. It now calls `trashImages([image.id])` then `library.refresh()` (there's no status to
`library.set()` any more) then the existing `refreshResults()` helper. This keeps the tree green
and behaviourally correct (the record moves to the trash instead of being dropped), but the
*label* is still "Remove record…" with the old D16 copy — task 3.5 owns changing `ImageCard.svelte`
to offer "Move to trash" and citing D6 instead, and should feel free to touch `forget()`'s body
again once the Inspector/tile-menu trash actions exist to share it with.

**Not started:** everything in groups 2–4, including `lib/api/trash.svelte.ts` (task 2.1) — it
wraps `trash_count()` as reactive state; the command exists and is tested, nothing else is built.

**Review fixes folded in (selection-and-bulk, requested mid-task):** `tags::bulk_set_rating`,
`tags::selection_tag_counts` and `export::extensions_by_id` no longer build one SQL parameter per
id in a single statement (SQLite's variable limit made a whole-library selection fail with "too
many SQL variables"); all three now chunk to `query::ID_CHUNK` (900) ids per statement — the rating
update inside one transaction, the tag-count query summed across chunks in memory, the extension
lookup merged into one map. `bulk_update_tags` was already per-id statements and untouched.
`export::read_image` (renamed from an inline closure) now distinguishes `ErrorKind::NotFound`
(reported as `missing`, unchanged) from any other read failure (now propagated as an error instead
of silently becoming another "missing" entry). New Rust tests: one per chunked function at ~2500
ids, plus one forcing a non-`NotFound` read error (a directory where a file belongs).

**Gate results:**
- `cargo test --manifest-path packages/app/src-tauri/Cargo.toml`: 242 passed, 0 failed.
- `mise run clippy`: clean.
- `mise run typecheck`: 0 errors across all three packages.
- `pnpm --filter @boorubox/app test`: 288 passed.
- `pnpm --filter @boorubox/shared test`: 1 passed.
- `mise run format` then `mise run lint`: clean (cargo fmt reformatted the five Rust files this
  task touched; eslint made no changes).

No schema change, no migration (design D15 confirmed against the current `db.rs`: still v3).

## Handoff notes (agent B)

Groups 2–4 are done. Nothing was needed from agent A beyond the five wrappers already in
`lib/api/commands.ts`; `packages/shared`, `src-tauri` and `commands.ts` were not touched.

**Where each decision landed.**

- D1 — `lib/components/library/LibraryScreen.svelte` takes `view: 'library' | 'trash'` and holds
  everything `routes/+page.svelte` used to. Both routes are three lines. The view reaches the
  request through `new SearchResults(view)`, and every control reads it back as `results.view`
  rather than being handed a second copy; `ImageCard` is the one component with an explicit
  `view` prop, because it never receives `results`.
- D13 — the three writes travel as one object, `TrashActions` in
  `lib/components/library/trash-actions.ts` (`trash`, `restore`, `deleteForever`), built once in
  `LibraryScreen` and passed to `LibraryGrid` → `ImageCard`, to `Inspector` (both placements, via
  `Lightbox`), and to `SelectionToolbar`. `deleteForever` opens the confirmation; it never
  deletes by itself.
- D7 — one `ConfirmDeleteDialog.svelte` serves both irreversible actions: the caller owns what is
  being destroyed (`pendingDelete`, either a list of ids or the whole trash) and therefore owns
  whether the dialog is up, so `open` is a plain prop with `onclose`, not a bindable.
- D4 — `DeleteReportCard.svelte` renders a `DeleteReport` only when `filesLeft` is non-empty, in
  the same band as the import and export reports, dismissible.
- D11 — `lib/api/trash.svelte.ts` holds the count. It is refreshed from `Sidebar.svelte` on the
  open library's path changing (which covers a library switch) and by `afterTrashWrite()` in
  `LibraryScreen` after every trash, restore, delete-forever and empty. Settings reads the same
  store, so the "In trash" line and the badge cannot disagree.
- D12 — `isTrashKey(event, view)` in `lib/keyboard.ts`, beside `isTypingTarget` and covered by
  `keyboard.test.ts`; `LibraryGrid`'s handler calls it and trashes the selection, or the focused
  image. One row was added to `KEYBOARD_MAP`, which is what the settings screen lists.
- D6 — `ImageCard`'s missing-file card lost its "Remove record…" confirmation. Its button is the
  card's tab stop, so it exists in both views: "Move to trash" in the library, "Restore" in the
  trash.

**One judgement call to sanity-check by hand.** A trash write clears the selection but keeps the
grid's focus, so `Delete` can be pressed again on the card that took the row (task 3.4). Task 3.3
wants the Inspector to fall back to "No image selected" after trashing from its action row, which
needs the focus cleared — so the Inspector's action row resets the selection itself (the comment
on `act()` says why). Every other path keeps the focus.

**Left alone deliberately.** On `/trash` the drop-to-import overlay and the pending-capture band
still behave as they do on `/` (a drop still imports into the library); only the `Import` menu is
replaced, per D13. The selection toolbar still offers Tags…, the rating control and Export… in
the trash view — nothing in the change says to take them away.

**Gate:** `mise run lint` clean, `mise run typecheck` 0 errors, `pnpm --filter @boorubox/app test`
294 passed (288 before, +2 for the trash store, +4 for the delete keys). Rust was not touched, so
`clippy` and `cargo test` are unchanged from agent A's run.
