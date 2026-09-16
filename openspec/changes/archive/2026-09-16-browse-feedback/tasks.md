> Four implementing agents, disjoint file sets, all four in parallel; the lead wires the rail
> (4.4) after A and D land. Gate per agent: `pnpm -r typecheck`, `pnpm lint`,
> `pnpm --filter @boorubox/app test` (B also `cargo test` and `cargo clippy` in
> `packages/app/src-tauri`); the lead runs `mise run check` on the joined tree.
> Nobody but the owner ticks a `Hand check:`.

## 1. Toolbar, search, session state (agent A — `LibraryScreen.svelte`, `SearchBar.svelte`, `keyboard.ts`, `api/browse-session.svelte.ts`, `api/index.ts`, `api/search.svelte.ts` comment only)

- [x] 1.1 `packages/app/src/lib/api/browse-session.svelte.ts` + test per design D1:
      `browseSession` with `resultsFor(view)` (one `SearchResults` per view, created on first
      ask, the same instance on the second), `inspectorOpen` (initially `true`) and
      `lightboxMode` (initially `'gallery'`); exported from `api/index.ts`. Amend the comment on
      `SearchResults.view` (the "one per route mount" sentence) to the new lifetime. Verify:
      tests — two asks for `'library'` are one instance; `'library'` and `'trash'` are two; the
      trash instance's sort is the trash default.
- [x] 1.2 `packages/app/src/lib/keyboard.ts` + test: `blurOnEscape(event)` — the handler
      `SearchBar.leaveOnEscape` is today, moved so both fields share it. Verify: test — Escape
      blurs and prevents default, another key does neither.
- [x] 1.3 `packages/app/src/lib/components/library/SearchBar.svelte` per design D3: the tag
      field alone (single line, `h-7 text-xs`, `id="tag-query"`, the example placeholder, no
      `title` wrapper), `oninput` / `onsubmit` props, Escape through `blurOnEscape`; the text
      field, the Clear button and the typing pause are gone from it. Verify: typecheck, lint.
- [x] 1.4 `packages/app/src/lib/components/library/LibraryScreen.svelte` per design D1–D3:
      `results` / `inspectorOpen` / `lightboxMode` from `browseSession`; `tagQuery` / `text`
      initialised from `results.inputs` and the mount running `results.run(results.inputs)`
      instead of `run(noQuery)`; `searchAfterPause` / `searchNow` (the 250 ms pause, one timer,
      cleared on destroy) driving both fields; the toolbar snippet in D2's order with the text
      `Input` and the two `flex-1` spacers; the sidebar snippet in the order `SearchBar`,
      `RatingPills`, `TagSidebar`, `CollectionsSection`, `ViewControls`. The `/` shortcut's
      comment about a textarea is amended (the field is an input again). Verify: typecheck,
      lint, tests.
      Hand check: on macOS with nothing selected the Import button, the slider and the
      inspector toggle sit at the right edge; select two — the selection controls appear in
      the middle and neither end moves; type `cat` in the sidebar field and `foo` in the
      toolbar field, open Trash, come back — both fields still read what was typed and the
      grid shows that result; hide the inspector, open Import, come back — still hidden;
      press `/` — the sidebar field has the focus; Escape leaves it.

## 2. Collections section, tile menu and mark, menu heights, the fold setting (agent B — `CollectionsSection.svelte`, `ImageCard.svelte`, `Inspector.svelte`, `SelectionToolbar.svelte`, `packages/shared/src/index.ts`, `api/commands.ts`, `api/settings.svelte.ts` + tests, `src-tauri/src/{settings,commands,lib}.rs`)

      Lead saw (macOS, scratch library, 2026-09-16): nothing selected — the text field sits
      left, Import · slider · inspector toggle sit at the right edge; the sidebar reads
      search, rating, tags, collections, filter, notes, nav. The selection, the Trash round
      trip and the Import round trip were not driven; those stay for the owner.
- [x] 2.1 The stored fold per design D4, copied from the notes field: `collectionsCollapsed`
      in `AppSettings`; `settings.rs` key, field, default `false`, load, save, the two tests the
      notes field has; `set_collections_collapsed` in `commands.rs` with its test, registered
      in `lib.rs`; `setCollectionsCollapsed` in `api/commands.ts` and on the `settings` store
      with the test its sibling has. Verify: `cargo test`, `cargo clippy`, typecheck, app tests.
- [x] 2.2 `packages/app/src/lib/components/tags/CollectionsSection.svelte` per design D4: the
      `Collapsible` on the setting, the heading row with chevron and "+", the `resize-y`
      list box with default, floor and ceiling heights, rows in the given order (the
      active-first sort and its comment deleted), a `ContextMenu` per row with Rename… and
      Delete… in place of the ellipsis `DropdownMenu`. Verify: typecheck, lint.
      Hand check: with 25 collections the box shows a few and scrolls, the Tags list above
      keeps its height, dragging the corner grows the box and the tag list shrinks; fold it,
      restart — folded; activate `collection 9` — marked, not moved; right-click a row —
      Rename… / Delete…; no "…" on any row.
      Lead saw: the box shows five rows and scrolls, with the resize corner; no "…" on any
      row. At an 800px window the default box starved the tag list (fixed: `h-32` default,
      `min-h-32` on Tags, and the filter column scrolls past that — commit 3bf3366). The
      fold across a restart and the row context menu were not driven.
- [x] 2.3 `packages/app/src/lib/components/library/ImageCard.svelte` per design D6: the rating
      heading and items inside `ContextMenu.Group`; the bookmark mark in the top-left badge
      group when the image is in a collection, `title` the names. Verify: typecheck, lint,
      tests; and — since this is the crash — a vitest DOM test (`// @vitest-environment
      jsdom`) that mounts `ImageCard` with a loaded image and opens its context menu without
      an error, if the copy-in renders under jsdom; if it does not, say so in the handoff and
      leave the check to the lead's app run.
      Hand check: right-click a tile — the menu opens with Rating, Collections, Move to trash;
      pick `q` — the badge appears; add to Favorites — the tile gains the mark, hover names it;
      a tile in no collection has no mark.
      Lead saw: right-click (AXShowMenu on the tile) opens the menu — Rating with the five
      choices, Collections, Move to trash — and the dev log records no client error, where
      before this change it recorded the GroupHeading context error. Rating from the menu
      and the mark were not driven.
- [x] 2.4 Menu heights per design D5: the class on `Inspector`'s Add to… content,
      `SelectionToolbar`'s Collection content and `ImageCard`'s collections `SubContent`.
      Verify: typecheck, lint.
      Hand check: with 25 collections, open Add to… on an image low in the window — the menu
      ends inside the window and scrolls; same for the toolbar's Collection and the tile's
      submenu.

## 3. The viewer (agent C — `Lightbox.svelte`, `viewer-zoom.ts` + test, `click-intent.ts` + test deleted)

- [x] 3.1 `packages/app/src/lib/components/library/viewer-zoom.ts` + test per design D7–D8:
      `coverScale`, `clickTarget` (replacing `zoomTarget`), `zoomBy` (replacing `zoomStep`),
      `wheelZoomFactor`, `MINIMAP_FRACTION` in `panOffset`, the constants. Verify: tests — a
      4000×3000 image in 1500×900 covers at 0.375 (1500 wide) and the click target is that; a
      4000×1000 in 1500×900 covers at 0.9 (900 tall); a 400×300 in 1500×900 covers at 3.75; an
      image whose aspect matches the viewport clicks to 2× fit; `zoomBy` clamps at the fit and
      the ceiling; a 100 px wheel is ×≈1.22, a 5 px one ≈×1.01, `deltaMode` 1 scales lines, a
      ctrl wheel uses the pinch sensitivity; the pan at 900 wide reads 0 at ≤300 px, 0.5 at
      450, 1 at ≥600.
- [x] 3.2 `Lightbox.svelte` per design D7–D8: the `header`, `chrome`, the click timer and the
      Tab-reveals-chrome branch removed; `onclick` on the image is `toggleZoom` with the
      `zooming` flag and the transition classes; the wheel through `wheelZoomFactor`
      (`ctrlKey` → pinch sensitivity); the WebKit gesture events through a typed
      `addEventListener` in an effect with `preventDefault`. Delete `click-intent.ts` and its
      test. Verify: typecheck, lint, tests.
      Hand check (macOS trackpad, then Windows mouse): open an image — nothing over it; click —
      it grows smoothly to cover, only one direction pans, the dark margin stays; move the
      pointer across the middle third — the far edges arrive, outside it nothing more; click —
      back to the fit; two-finger scroll up slowly — small smooth steps; a mouse notch — one
      visible step; pinch out / in — zooms and returns to the fit; Tab in inspect mode still
      cycles the panel; Escape, Space and a click on the dark area close; → resets the zoom.

## 4. The account rail (agent D — `AccountRail.svelte`, `domain/tag-utils.ts` + test; 4.4 the lead)

      Lead saw: the viewer opens with nothing over the image; one click zooms it to cover
      the width, overflowing vertically, with the dark margin kept. The wheel, the pinch
      (both engines), the animation feel and the second click were not driven.
- [x] 4.1 `packages/app/src/lib/domain/tag-utils.ts` + test: `addAccountToQuery` and
      `excludeAccountFromQuery` per design D9, beside `toggleAccountInQuery`. Verify: tests —
      add on an empty query, add when present (unchanged), add when excluded (excluded stops,
      not also included), exclude the mirror three; the rest of the query untouched.
- [x] 4.2 `packages/app/src/lib/components/library/AccountRail.svelte` per design D9: rows in
      the given order with `+`, `−`, handle, count, included/excluded marking from
      `activeTerms`. Verify: typecheck, lint.
- [x] 4.3 (agent D) Handoff: the component's props and any helper the lead needs, written
      below.
- [x] 4.4 (lead, after 1.4 and 4.2) `LibraryScreen.svelte`: the rail `aside` between the grid
      column and the inspector while `results.group === 'x-account'`, fed by `results.groups`,
      clicks through `searchKeeping`. Verify: typecheck, lint, tests, `mise run check`.
      Hand check: group by X account — a list appears left of the inspector, largest first;
      `+` on one — `account:<handle>` in the search, the row blue; `−` on another — struck
      through; group by nothing — gone.

## 5. Change-level verification (owner)

- [ ] 5.1 `mise run check` green on the joined tree; every hand check above on macOS and
      Windows.
      Owner, 2026-09-16: every macOS check above passes on the dev build (the viewer after the
      frame-loop zoom included). Windows still to run on a test build: F11 and the full-screen
      button are the only Windows-specific parts of this change.

## Handoff

**Lead, 2026-09-16 night.** All four units landed as checkpoint commits and `mise run check`
was green on the joined tree (exit 0). Two things the agents could not settle, for the owner's
hand checks: the tile menu has no automated test — vitest resolves Svelte's server build here,
so `mount()` is unavailable (handoff-B.md); the lead verified it in the running app instead. The
viewer ignores a ctrl-wheel while a WebKit gesture is running (`gestureActive` in Lightbox),
because WKWebView can report one pinch both ways; whether it does on this build is the macOS
pinch hand check. The account rail was not driven: the scratch library has no X captures.
