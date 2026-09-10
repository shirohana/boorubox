> Lands after `app-shell` and `tags-and-ratings` (proposal.md, "Depends on"). Group 1 is Rust and
> the command wrappers; group 2 is one self-contained TypeScript module with its own test and can
> run beside group 1. Groups 3–6 all touch
> `packages/app/src/lib/components/library/` and `routes/+page.svelte`, so they are one agent's
> work in order. Two implementing agents at most: **A** owns group 1 (`packages/shared`,
> `packages/app/src-tauri`, `packages/app/src/lib/api/`), **B** owns groups 2–6
> (`packages/app/src/lib/api/selection.svelte.ts`, `src/lib/components/library/`,
> `src/routes/+page.svelte`).

## 1. Rust: bulk commands, tag counts and the zip export

- [x] 1.1 `packages/shared` + `packages/app/src-tauri`: add `TagCount { tag, count }`,
      `ExportReport { path, written, missing }` and `ExportProgress { done, total }` to
      `src/index.ts` and mirror them in `model.rs` (Phase 1 D11 — one commit, both files); verify a
      serde test asserts the camelCase JSON keys and `mise run typecheck` passes.
- [x] 1.2 `packages/app/src-tauri`: extract `query::search_ids(conn, req) -> Vec<String>` out of
      `query::search` so both use one `ORDER BY captured_at DESC, id DESC` (design D3), and add the
      `search_ids(req)` command; verify a Rust test that `search_ids` returns the same ids in the
      same order as `search` over the same request with `limit`/`offset`, and that it answers
      `NoLibrary` with no library open.
- [x] 1.3 `packages/app/src-tauri`: `bulk_update_tags(ids, add, remove)` — one transaction over
      every id, reusing `tags-and-ratings`' per-image add/remove helpers and its orphan-tag
      collection (design D10); verify Rust tests that fifty images all carry an added tag, that
      adding a tag an image already has and removing one it lacks change nothing, that an unknown
      id in `ids` fails the whole call with no image edited, and that a tag whose last use was
      removed is gone from the `tags` table.
- [x] 1.4 `packages/app/src-tauri`: `bulk_set_rating(ids, rating)` as one `UPDATE … WHERE id IN
      (…)`, `rating = null` clearing to unrated (design D10); verify Rust tests that every id
      carries the new rating, that `null` unrates, and that `updated_at` moved on each row.
- [x] 1.5 `packages/app/src-tauri`: `selection_tag_counts(ids, limit)` — the commonest tags among
      `ids` with counts, from one `GROUP BY` over `image_tags`, ordered by count then tag (design
      D9); verify Rust tests on a fixture where three tags have different frequencies that the
      order and counts are right, that `limit` truncates, and that a selection with no tags returns
      an empty list.
- [x] 1.6 `packages/app/src-tauri`: add `zip = { version = "8", default-features = false }`, write
      `export.rs` and the `export_zip(ids, path)` command — resolve the rows under the library lock,
      release it, write stored entries named `<id>.<ext>`, emit `export:progress` per file, run on
      `spawn_blocking` (design D11, D13); verify Rust tests that the archive holds one entry per id
      with bytes identical to the file under `images/`, that an id whose file was deleted is left
      out and named in `ExportReport.missing` while the rest are written, that every entry is
      stored rather than deflated, and that the last progress tick reports `done == total`.
- [x] 1.7 `packages/app/src` + `packages/app/src-tauri`: one wrapper per new command in
      `lib/api/commands.ts` with the `index.ts` exports, `pickExportZipPath()` in `lib/api/dialog.ts`
      on the dialog plugin's `save()`, `onExportProgress` in `lib/api/events.ts`, and
      `dialog:allow-save` in `capabilities/default.json` (design D12, D13); verify
      `commands.test.ts` gains a case per command asserting the command name and argument keys that
      reach the IPC boundary, and `events.test.ts` one for the export event name.

## 2. The selection store

- [x] 2.1 `packages/app`: `src/lib/api/selection.svelte.ts` — `focus`, `anchor`, the
      `ids` / `range` state of design D2, `has(index, id)`, `count`, `click(index, id, modifiers)`,
      `extendTo(index)`, `selectAll(total)`, `clear()`, `previewIds(limit)` and an async `ids()`
      that resolves a range through an injected `search_ids` caller (design D1–D4); verify
      `selection.svelte.test.ts` covers: a plain click focuses and clears, multi-select-click
      toggles one id and leaves the rest, shift-click takes the range both directions from the
      anchor, repeated shift-arrows grow and shrink the range from a fixed anchor, `selectAll`
      counts `total` without resolving anything, `ids()` on a range makes exactly one resolver call
      with the range's offset and limit and leaves the store in id mode, a multi-select-click over
      a live range resolves it first, and `clear()` leaves the focus alone.

## 3. Grid and keyboard

- [x] 3.1 `packages/app`: `pnpm dlx shadcn-svelte@latest add checkbox progress`, and give
      `ImageCard.svelte` a selection checkbox in the hover/focus overlay plus the selected ring,
      with the checkbox click not also activating or focusing the card; verify by hand that
      checking a tile selects only it, that the ring shows on a selected tile and on an
      unloaded placeholder inside a range selection, and that `mise run lint` still passes with the
      new copy-ins excluded (CLAUDE.md).
      Hand check: hover a tile and tick its checkbox: only that tile gains the primary ring and
      the count reads 1, and the viewer does not open. Shift-click far down a long result, then
      scroll into rows that have not loaded — the grey placeholders carry the same ring.
      Human update: when tick a tile, the top bar width growth and if window width not wide enough, it overflowed horizontally.
      Probe (2026-09-10, agent-driven app on a scratch copy of the vault): at a 1200px window with 51 selected the icon row fits with the two search fields narrowed; nothing overflowed. Narrower windows yours.
      Agent (2026-09-09): the row was the cause, not the tile — `SelectionToolbar` was `shrink-0`
      and about 640px of labelled buttons, in a band it shares with two search fields, two view
      selects, the slider and the inspector toggle. Its actions are now icon buttons (lucide, one
      `action` snippet so a button cannot be added without a name), each keeping its words as
      `aria-label` and `title` and showing them as text again at `xl` and wider; the row itself is
      `min-w-0` with `overflow-x-hidden`, so whatever ends up in it, it clips instead of pushing
      the band past the window. Re-check by ticking a tile at a narrow window (nothing scrolls
      sideways, the count and the first actions stay put) and at a wide one (the labels come
      back). Widest item left is the rating control (~160px); estimated row minimum ~380px, so the
      band needs roughly 900px before it starts clipping — say so if that is still too wide, since
      the remaining width is `ViewControls` and the slider, which this unit does not own. Design
      D6 amended.
      Human update (2026-09-10): at a window about half the screen wide, ticking a tile squashes
      the two search fields to empty rings; the selection row shows whole.
      Agent (2026-09-10): the search form was `min-w-0` on a `flex-1` zero basis, so it was the
      first thing in the band to give. It now carries a `min-w-56` floor that its two fields
      share, and the selection row — already `overflow-x-auto` — is what gives and scrolls. The
      band's fixed width is now roughly 810px (traffic lights, toggle, the two selects, the slider,
      the inspector toggle, the floor); narrower than that the band itself overflows the window.
      D6 amended again. Hand check: at a half-screen window tick a tile — both fields still take a
      word, and the selection row scrolls sideways to reach its last action.
      Human update (2026-09-10, second pass): the multi-select top bar still feels squashed; a
      redesign will be called in a later change.
      Agent (2026-09-10): deferred to that change, with the follow-up already in the backlog (a
      narrower toolbar band: `ViewControls` 272px + slider 96px are fixed). The 900px window
      minimum from browse-polish 4.1 is the floor it has to fit at zoom 1.
- [x] 3.2 `packages/app`: `LibraryGrid.svelte` reads `focus` and `anchor` from the store instead of
      its own state, routes click modifiers into `click()`, and extends its existing keydown
      handler with shift+arrows, shift+`Home`/`End`, the select-all shortcut and `Esc` behind
      `isTypingTarget` and through `offsetIndexClamped` (design D1, D5); verify a unit test that
      shift+arrow movement clamps at both edges like plain movement, and a manual pass: shift-arrow
      from the middle of the grid in all four directions, select-all, `Esc`, then the `app-shell`
      keyboard map (plain arrows, `Enter`, `i`, `/`) still behaving as it did.
      Hand check: focus a card, shift-arrow in all four directions and watch the count grow and
      shrink from the card you started on; shift-`Home`/`End`; ⌘A over a few hundred images (count
      immediate, no pause); `Esc` (count gone, ring stays); then plain arrows, `Enter`, `i` and
      `/` unchanged, and none of them firing while typing in either search field.
      Human update: it not works, when holding *Shift* and arrow keys to select, the selection window slides to only pick the range between *current focused* and *next focused*, but expected it pinned a starting point.
      Probe (2026-09-10, agent-driven app on a scratch copy of the vault): Home, shift-↓ twice: 9 selected; shift-↑: 5. The anchor stayed on the first tile.
      Agent (2026-09-09): the store was right and the DOM was overwriting it — every gesture ends
      with the grid focusing the destination card, whose `focusin` called `focusAt`, which moves
      the anchor with the focus. `Selection` now separates the two: `focusAt` is a deliberate move
      (plain arrow, plain or ⌘-click, the card the viewer closed on) and takes the anchor with it,
      `focusEntered` is the DOM landing on a card and moves the focus alone; `LibraryGrid`'s tiles
      call the latter. This also fixes shift-*click*, which was selecting a range of one for the
      same reason (WebKit focuses a tile on `mousedown`, before the click says shift was held).
      Covered by two new cases in `selection.svelte.test.ts` that interleave `focusEntered` the
      way the grid does. Re-check by shift-arrowing four cards out and two back — the count grows
      from the card you started on, never a two-card window — then a plain arrow and a shift-arrow
      to see the anchor move again. Design D1 amended.

## 4. Toolbar and inspector

- [x] 4.1 `packages/app`: `src/lib/components/library/SelectionToolbar.svelte` — count, "Select
      all", "Clear", "Tags…", a `RatingControl.svelte` writing through `bulk_set_rating`, and
      "Export…" — swapped into the toolbar's action row in `routes/+page.svelte` while
      `count > 0`, replacing the `Import` menu and nothing else (Slot: Toolbar · actions, design
      D6, D8); verify by hand that selecting shows the row and clearing brings `Import` back, that
      the search inputs and the tile-size slider stay put throughout, and that a bulk rating on a
      selection lands on every selected image and keeps the selection.
      Hand check: select a tile: the `Import` menu is replaced by the selection row while the
      search fields, the view controls and the tile-size slider stay exactly where they were;
      `Clear` brings `Import` back. Pick a rating on a selection of several — every selected tile
      gains the badge and the selection is still there afterwards.
      Human update: it just too wide. need better design.
      Probe (2026-09-10, agent-driven app on a scratch copy of the vault): see 3.1. If a medium window still overflows, a later change moves ViewControls and the slider.
      Agent (2026-09-09): same fix as 3.1 — icons with `aria-label`/`title`, labels back at `xl`,
      `min-w-0` + `overflow-x-hidden` on the row. Order is unchanged (count, Select all, Clear,
      Tags…, rating, Export…, trash actions), so what clips first at a very narrow window is the
      trash action, not the count. The rating control is untouched and is now the widest element;
      a compact mode for it was not built, since a second rating shape would read differently from
      the inspector's. Design D6 amended.
- [x] 4.2 `packages/app`: `Inspector.svelte` renders the multi-selection header — count and up to
      twelve thumbnails with "+N more" — at two or more selected, and its existing single-image
      panel at one or at a bare focus (Slot: Inspector · header, design D7); verify by hand that
      one selected shows the full panel, four show the header and four thumbnails, a range
      selection over unloaded rows shows the thumbnails it has plus the overflow count, and
      clearing returns to the focused image.
      Hand check: one selected shows the full single-image panel; a second makes the inspector a
      count and a thumbnail strip; ⌘A over a long result shows twelve thumbnails and "+N more"
      with N the rest of the count; `Esc` returns the panel to the focused image.
      Human update: when multi-select, the right panel shows thumbs but just can't doing anything on them, not useful at all. Expect users can press thumbs to preview, click right top corner with *x* icon to unselect specific image.
      Probe (2026-09-10, agent-driven app on a scratch copy of the vault): clicking a thumb opened the viewer at that image; × on the first thumb dropped it (5 → 4) and the focus landed on the thumb that took its place. The first × probe found nothing happened: `$state` proxies broke the resolve guard in the webview only, fixed in the same pass.
      Agent (2026-09-09): `SelectionThumbs` thumbnails are controls now. Pressing one opens the
      screen's viewer at that row — through the same `openViewer` the grid uses, via a new
      optional `onactivate` on `Inspector`, so there is one viewer — and each has an × button in
      its top-right corner (shown on hover, and on focus, so it is reachable by Tab) calling a new
      `Selection.remove(id)`, which resolves a range to ids first like every other action and
      leaves the rest of the selection alone. `Inspector`'s `rowWithId` walk now answers with the
      row rather than the record, since both the panel and the strip want the index. The twelve
      cap and "+N more" are unchanged, and dropping to one selected shows the single-image panel.
      Re-check by selecting several, pressing a thumbnail (viewer opens on that image, selection
      intact), then ×-ing thumbnails down to one and watching the panel come back. Design D7
      amended; the `selection` spec's inspector requirement gained the two behaviours and three
      scenarios.

## 5. The bulk tag dialog

      Human update (2026-09-10): pressing a thumbnail in the multi-selection strip and then the
      arrow keys behaves strangely; the arrows should either move within the selected items or do
      nothing.
      Agent (2026-09-10): what happens today — a press opens the viewer on that image (D7,
      amended), and the viewer's arrows then walk the whole result, not the selection, so the
      strip's neighbour and the viewer's neighbour disagree. Making the viewer walk the selection
      is a viewer over a subset (its index, its row step and its grid scroll all read the result),
      which is a design decision, not a patch: deferred as a follow-up beside the top-bar
      redesign, with the owner's two acceptable shapes recorded here.
- [x] 5.1 `packages/app`: `src/lib/components/library/BulkTagDialog.svelte` — "Add tags" and
      "Remove tags" as `TagInput.svelte` fields (remove suggesting only from the selection's tags),
      quick-remove pills from `selection_tag_counts` showing tag and count and toggling the tag in
      the remove field, Apply calling `bulk_update_tags` and then refreshing the current search
      (design D8, D9); verify by hand on a selection spanning more than one loaded page: the pills
      show counts covering the whole selection, clicking one adds and removes it from the field,
      Apply adds and removes across every selected image, the grid and inspector show the new tags
      without a re-search, and a failed apply leaves the fields filled with the error named.
      Hand check: select across more than one loaded page (⌘A on a few hundred), open "Tags…": the
      pills' counts describe the whole selection, not the loaded part. Click a pill twice — it
      goes into the "Remove tags" field and comes back out. Type in "Remove tags": only tags the
      selection carries are suggested. Apply, then check the grid and the inspector show the new
      tags with no re-search. Close the library folder mid-dialog to see a failed apply keep the
      fields filled with the error under them.
      Human update: the ⌘A shortcut needs user focused on the gallery grid area, if no, it'll just select everything like dragging cursor on a webpage, but expect to select all images.
      Probe (2026-09-10, agent-driven app on a scratch copy of the vault): ⌘A with the focus in the inspector selected 51; Escape from there cleared it.
      Agent (2026-09-09): ⌘A moved off the grid's keydown handler onto `LibraryScreen`'s window
      handler, beside the `/` binding — it is about the result, which the screen owns, not about
      the focused card. Guarded by `isTypingTarget` and a new `isInDialog` (in `keyboard.ts`, with
      tests: a native `<dialog>` for the viewer, `role="dialog"` for everything shadcn draws) plus
      the screen's `lightboxOpen`, so it never fires under a dialog and never eats a field's own
      select-all. The grid no longer reads the key, so there is one binding and no double fire.
      Re-check with the focus on the frame, on the sidebar and straight after launch (⌘A selects
      every image, nothing highlights blue), then inside a search field and inside the Tags…
      dialog (the field's own select-all). `KEYBOARD_MAP`'s row now reads "Library"; design D5 and
      the `selection` spec's keyboard table amended.
      Human update: in the multi-tagging tool, don't make clicking label focus to the input. while it shows remove-tags dropdown and I want to just click on the showing tags under the dropdown, I might click somewhere outside the text input, it commonly the spaces between two text input. But when I click there, it'll clicked on the label and shows the dropdown again. just annoying.
      Probe (2026-09-10, agent-driven app on a scratch copy of the vault): not probed. Yours.
      Agent (2026-09-09): both `<label for>` elements in `BulkTagDialog` are plain `<h3>`
      headings now, so clicking the dead space between the fields — or the heading itself — no
      longer focuses a field and re-opens its suggestion list. The fields keep their accessible
      names from `TagInput`'s `aria-label` ("Tags to add to every selected image" / "…remove
      from…"), which is why no `aria-labelledby` was needed and `TagInput` was not touched.
      Re-check by opening Tags… on a selection, typing in "Remove tags" to raise the list, and
      clicking a pill and the gap above it — the list closes and stays closed. Design D8
      amended.

## 6. Export

- [x] 6.1 `packages/app`: wire "Export…" — `pickExportZipPath()`, then `export_zip` with the
      resolved ids, progress from `export:progress` shown inline in the toolbar, and the
      `ExportReport` shown as a dismissible line naming how many were written and any missing image
      (design D11–D13); verify by hand that cancelling the save dialog writes nothing, that a
      several-hundred-image export shows progress and leaves the window responsive, and that the
      report names a missing file when one has been deleted from `images/` beforehand.
      Hand check: press "Export…" and cancel the save dialog — nothing is written and no report
      line appears. Export a few hundred images: the bar and the `done / total` count move in the
      toolbar and the window keeps painting. Delete one file from `images/` first — the report
      line under the toolbar names how many were written and lists that id, and survives clearing
      the selection until it is dismissed.
      Human update: the export zip tagged images all modified at `1980/1/1 12:00`, expect at least it be capture time so users can have at least the time axis to sort the exports. It used UUID as filename, it has no order and make users difficult to use. Maybe we can follow the format like `yande.re` (another dnabooru distribution) it named files like `yande.re 1265807 animal_ears japanese_clothes kani_biimu kitsune shimotsuki_potofu skirt_lift tail thighhighs.jpg` I know we use UUID so it might be long, but just much strong then UUID only.
      Probe (2026-09-10, agent-driven app on a scratch copy of the vault): exported 5 through the save dialog: entries read `<id> blue_archive test1 test2.jpg` for the tagged one, `<id>.jpg` for the rest, dated 09-07-2026 22:26 in the archive, which is the local capture time the inspector shows.
      Agent (2026-09-09): `export.rs` now names each entry `<id> <tag1> <tag2>….<ext>` (tags in
      the image's stored order — alphabetical, `ingest::load_records`'s order — sanitized so `/`,
      `\` and control characters can't reach the name, whole tags dropped from the end past a
      255-byte cap) and sets the entry's mtime to `captured_at` via the `zip` crate's `time`
      feature, falling back to the crate's `1980-01-01` default only when `captured_at` predates
      1980 or is otherwise unrepresentable. `image_meta_by_id` (beside the old
      `extensions_by_id`, which it replaces) fetches ext/captured_at/tags for the whole selection
      in two chunked queries, not one query per image. New unit tests in `export.rs`
      (`entry_name`/`entry_mtime`, including the 255-byte cap and a multibyte tag near it) and two
      updated/added integration tests in `commands.rs`
      (`export_zip_names_entries_by_id_and_tags_and_dates_them_by_captured_at`). Re-check by
      exporting a tagged selection and an untagged one in the same run and confirming both name
      shapes and the mtime in the same archive (`unzip -l`/Finder "Date Modified" both work — no
      running app needed, `cargo test --manifest-path packages/app/src-tauri/Cargo.toml export`
      covers it). Design D11 and the `export-selected` spec's naming requirement are amended
      alongside; no schema change.

## 7. Verification

- [x] 7.1 `mise run check` passes (lint, typecheck, tests, clippy, builds).
- [ ] 7.2 Manual keyboard pass in the running app: focus a card, shift-arrow in all four
      directions and watch the count, shift-`Home`/`End`, select-all over a library of at least a
      few hundred images and confirm the count is right and immediate, `Esc` clears while the focus
      stays, multi-select-click a few tiles, shift-click a range across a scroll that loads new
      pages, then confirm the `app-shell` map (plain arrows, `Enter`/`Space`, `i`, `/`, lightbox
      keys) is unchanged and that no shortcut fires while typing in either search field.
- [ ] 7.3 Bulk tag 50 images: select them, open the dialog, check the quick-remove pills' counts
      against a tag you know 50 images carry, add two tags and remove one, apply, then search for
      an added tag and confirm all 50 come back and the removed tag no longer matches them.
- [ ] 7.4 Export 20 images to a chosen path, open the zip in Finder, and confirm 20 files named
      `<id>.<ext>`, each opening as the same image the grid shows; repeat with one file deleted
      from `images/` first and confirm 19 written with the missing one named in the report.

## Handoff notes (agent A)

- **1.1's `TagCount` was reused, not re-added, and its field is `name` not `tag`.** By the time
  this change lands, `tags-and-ratings` had already shipped `TagCount { name: string, count:
  number }` in `packages/shared/src/index.ts` / `model.rs`, used by `tag_suggestions` and the
  sidebar's `tag_counts`. Adding a second `TagCount { tag, count }` as task 1.1 literally reads
  would have been a duplicate type with a colliding name — impossible in the same module — so
  `selection_tag_counts` returns `Vec<TagCount>` (the existing type), and its pairs read as
  `{ name, count }` on the wire, not `{ tag, count }`. `ExportReport` and `ExportProgress` are new
  and added exactly as specified. If `BulkTagDialog.svelte` (task 5.1) reads the quick-remove
  pills' tag text, read `.name`, the same field `tagCounts(...).tags` and `tagSuggestions(...)`
  already use.
- **No migration.** Confirmed against `db.rs`: schema is at v3 (`browse-polish`,
  `file_modified_at`); this change added no column and no `MIGRATIONS` entry, matching design D14.
- **New Rust module `export.rs`**, registered in `lib.rs` (`pub mod export;`). New dependency `zip
  = { version = "8", default-features = false }` (Cargo.toml/Cargo.lock already updated).
- **New commands** (all registered in `lib.rs`'s `generate_handler!`, all wrapped in
  `lib/api/commands.ts`): `search_ids(req)`, `bulk_update_tags(ids, add, remove)`,
  `bulk_set_rating(ids, rating)`, `selection_tag_counts(ids, limit)`, `export_zip(ids, path)`.
  TS names: `searchIds`, `bulkUpdateTags`, `bulkSetRating`, `selectionTagCounts`, `exportZip`,
  `pickExportZipPath` (`lib/api/dialog.ts`), `onExportProgress` (`lib/api/events.ts`,
  `EXPORT_PROGRESS_EVENT = 'export:progress'`). All re-exported already via `index.ts`'s
  `export *`, so no `index.ts` edit was needed.
- **Side effect of the gate, not a group-1 change:** `mise run format`'s `eslint . --fix` step ran
  repo-wide before I ran `cargo fmt` separately, and it auto-fixed two pre-existing, non-semantic
  lint issues in agent B's in-progress files: an operator-linebreak spacing fix and the removal of
  an unused `eslint-disable` comment, in `selection.svelte.ts` and
  `components/library/thumbnail-cache.ts`. I did not otherwise edit either file. Both files still
  fail `mise run lint` on the `svelte/prefer-svelte-reactivity` rule (three `new Set()` /
  `Set<...>` uses in `selection.svelte.ts` want `SvelteSet`) — that's agent B's to fix, unrelated
  to group 1.

## Handoff notes (agent B)

- **`RatingControl.svelte` now takes a value and a writer, not an image and the search store.**
  Task 4.1 asks for the same control in the toolbar writing through `bulk_set_rating`, so its
  props are `value: Rating | null | undefined` and `onchoose(rating)`: the inspector passes the
  image's rating and `results.saveRating`, the toolbar passes `undefined` — "no single current
  rating" — and the bulk write. It lives in `components/tags/`, which neither agent owned.
- **`TagInput.svelte` gained an optional `suggest` prop** (default: the library's vocabulary
  through `tagSuggestions`). The bulk dialog's "Remove tags" field passes a matcher over the
  `selection_tag_counts` answer, which is the only way to suggest from the selection's own tags
  (design D8). Also in `components/tags/`.
- **`thumbnail-cache.ts` is new** (`components/library/`): the per-window `thumbnail_path` memo
  that used to be `ImageCard.svelte`'s module script, lifted so the inspector's selection strip
  draws from the same cache instead of asking again.
- **`Esc` in the grid only acts while something is selected**, so the key is still free for
  whatever binds it next, and it never clears the focus (design D5).
- **The bulk rating reports its own failures** under the control (`RatingControl`'s error line);
  export failures and the export report go to the page, so the report outlives clearing the
  selection.
- **Three review fixes from the coordinator are folded in**: `ImageCard`'s click reads
  `focused` before anything moves the focus (a live prop, so a first click was opening the
  viewer); `Lightbox`'s `onclick` ignores `event.detail > 1` (the second click of a double click
  was closing the dialog the first one opened); `Lightbox`'s Space branch accepts the `<dialog>`
  as well as the surface (a click on the image left the dialog as the active element and Space
  went dead).
- **Not run here:** `mise run clippy` and `mise run build` (task 7.1's other halves) and every
  task 7.2–7.4 pass, which need the app running.

## Handoff notes (review fixes)

- **`Inspector.svelte` walks the result rows to find the one selected record.** With exactly one
  image selected the panel has to show *that* image (spec `selection`), and it is not always the
  focused one — a multi-select click that deselects leaves the focus on the card it cleared. The
  selection names it by id (`previewIds(1, …)`), but `SearchResults` can only be asked row by row,
  so `rowWithId` walks `at(index)` the way the store's own `replace` walks `#images`. A
  `SearchResults.byId(id)` (or `indexOf(id)`) would replace that walk with the lookup the store is
  already doing internally; it was not added here because `src/lib/api/` is being edited by the
  `trash` change at the time of writing.
