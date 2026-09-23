> Three Sonnet units and one that follows them. Lane 1: unit A then unit C (they share
> `LibraryScreen.svelte` and `ImageCard.svelte`). Lane 2: unit B, whose files are disjoint from
> both. Design D1–D4 decide every shape; do not re-decide them. Gate for every unit:
> `mise run check` green (lint, typecheck, vitest, cargo test, clippy, builds). No unit ticks a
> hand check; each writes a `Hand check:` line under it and leaves the box.

## 1. Unit A — the selection: prune by the search, pick up the anchored card (`packages/app`, `packages/app/src-tauri`)

- [x] 1.1 `query.rs`: `matching_ids(conn, req, ids)` per D1 on the same `Plan` as `search_ids`,
      chunked by `ID_CHUNK`, plus a test beside `search_ids_names_the_same_page_search_would_load`
      that seeds four images, two tagged `cat`, and shows `matching_ids` for the `cat` request over
      all four ids answers exactly the two. Verify: `cargo test matching_ids` passes.
- [x] 1.2 `commands.rs` + `lib.rs`: command `matching_ids(req: SearchRequest, ids: Vec<String>)`,
      registered after `search_ids`, with a command-level test copied from `search_ids`'s.
      `api/commands.ts`: `matchingIds(req, ids)` and its invoke-shape test beside `searchIds`'s.
      Verify: `cargo test`, `pnpm --filter @boorubox/app test commands` pass, `mise run clippy` clean.
- [x] 1.3 `api/selection.svelte.ts`: `MatchResolver` injected beside `IdResolver`;
      `keepMatching()` per D1 through `#editIds`; the anchor pickup in the `multi` branch per D2
      (read `anchor`, resolve it with `#resolve(anchor, 1)`, only when the resolved set is empty
      and the anchor is another row); `toggle(index, id)` with today's toggle body;
      `removeMany` deleted. Doc comments say why the anchor and not the focus. Tests in
      `selection.svelte.test.ts`: `click(10, i10)` → `focusEntered(13)` → `click(13, i13,
      {multi:true})` gives count 2 with both ids and the resolver called once with `(10, 1)`;
      the same with `click(10, i10)` then `click(10, i10, {multi:true})` gives one; `toggle`
      after a plain click on another card gives one; `keepMatching` over ids `[a, b, c]` with a
      match resolver answering `[b]` leaves `{b}`; over a range it resolves the range first;
      an empty selection makes no call; a gesture during the round trip wins. The `removeMany`
      test goes. Verify: `pnpm --filter @boorubox/app test selection` passes.
- [x] 1.4 `LibraryScreen.svelte`: build the `Selection` with both resolvers (the match one from
      `buildSearchRequest(results.inputs, {sort, group, view}, 0, 0)` and `matchingIds`);
      `afterTrashWrite` becomes `afterWrite()` per D1 (refresh, `keepMatching`, focus clamp) and
      every caller uses it; `writeRating`'s bulk branch and the sidebar collection `onchanged`
      call `afterWrite`; `BulkTagDialog.svelte` drops its own `results.refresh()` for an
      `onapplied` prop the screen binds to `afterWrite`. `ImageCard.svelte`: the checkbox calls
      a new `ontoggle` prop; `LibraryGrid.svelte` threads it to `selection.toggle`. Verify:
      `mise run check` green.
      Hand check: search `tagme`, select 50, bulk-remove `tagme` from them — the grid empties of
      them, the count and the strip follow, the toolbar's actions disappear at zero, then press
      → and the focus moves. Search `rating:s`, select five, rate `q` — same. Click A, ⌘-click B
      (macOS) / Ctrl-click B (Windows) — two selected. Click A, tick B's checkbox — one selected.
      Trash and restore still prune as before.

## 2. Unit B — every overlay of the inspector works inside the viewer (`packages/app`)

- [x] 2.1 `src/lib/portal.ts`: `portalTarget(el)` per D3 with `TagInput`'s comment moved onto it;
      `src/lib/portal.test.ts` (`// @vitest-environment jsdom`): an element inside a `<dialog>`
      answers that dialog, one outside answers `undefined`, `null` answers `undefined`. Verify:
      `pnpm --filter @boorubox/app test portal` passes.
- [x] 2.2 `TagInput.svelte` calls `portalTarget(field)`; `Inspector.svelte` binds its root,
      derives `portalTo`, passes `portalProps={{ to: portalTo }}` to the add-menu content and
      both context-menu contents, and hands `portalTo` to `CollectionNameDialog` and
      `UploadAction`, which pass it to their own `Dialog.Content` / `DropdownMenu.Content` /
      `UploadDialog`. Verify: `mise run check` green; grep shows exactly one `closest('dialog')`
      in `src`.
      Hand check: open the viewer, press `i`, open "Add to…" — the menu opens over the viewer and
      choosing a collection adds the image; right-click a tag badge and a collection badge —
      both menus open; "New collection…" opens its dialog; Escape inside an open menu closes the
      menu and leaves the viewer open; Escape again closes the viewer. Beside the grid nothing
      has changed.
      Seen by the lead on a scratch copy of test-1 (2026-09-23 early, driven through accessibility, not
      the owner's hands): inside the viewer in inspect mode, "Add to…" opened its list over the image
      (sixteen collections, scrolling). Escape and the badge menus were not exercised.

## 3. Unit C — a click that lands on nothing hands the keys back (`packages/app`), after unit A

- [x] 3.1 `components/library/focus-handback.ts`: `isOrphanedFocus(active, within?)` per D4;
      `focus-handback.test.ts` (jsdom): `null`, `document.body` and an element inside `within`
      are orphaned; a button elsewhere is not. `ImageCard.svelte`'s `onmenuclose` uses it.
      Verify: `pnpm --filter @boorubox/app test focus-handback` passes.
- [x] 3.2 `LibraryScreen.svelte`: the root's `onclick` per D4, calling `grid.refocus()` when a
      card is current and the focus is orphaned; `LibraryGrid.refocus()` focuses with
      `preventScroll: true` (doc comment: why). Amend the `onrelease` doc comment in
      `Inspector.svelte` and `refocus()`'s in `LibraryGrid.svelte` from "a completed action" to
      the widened rule. Verify: `mise run check` green.
      Hand check: click a card, click the panel's title text, press `→` — the focus moves to the
      next card without the grid scrolling; the same after clicking the page address text, the
      panel's empty space, the account rail's gap and the toolbar's band; click into the tag
      editor and press `→` — the caret moves and the grid does not; in the viewer, click the
      panel's text and press `→` — the next image is shown. Scroll the grid several rows away from the
      focused card, click the panel, press `→` — the grid scrolls back and the focus moves. Drag-select
      the panel's address text — the text selection is kept.
      Seen by the lead (same run): click a card, click the panel's title text, press → — the next card
      took the focus and the panel followed it, without the grid scrolling. The rail, toolbar, editor
      and viewer variants were not exercised.

## Handoff

### Unit B (task group 2)

- 2.1: `packages/app/src/lib/portal.ts` — `portalTarget(el)` exactly per D3, carrying
  `TagInput`'s former doc comment. `packages/app/src/lib/portal.test.ts` covers the three cases
  D3's task names. `pnpm --filter @boorubox/app test portal` — 3 passed.
- 2.2: `TagInput.svelte` now calls `portalTarget(field)`, with a one-line pointer to the doc
  comment that moved. `Inspector.svelte` binds its root div (`bind:this={root}`), derives
  `portalTo = portalTarget(root)`, and passes `portalProps={{ to: portalTo }}` to the add-menu's
  `DropdownMenu.Content` and both `ContextMenu.Content`s (tag badge, collection badge).
  `CollectionNameDialog` and `UploadAction` both take an optional `portalTo?: Element` prop and
  pass it through the same way; `UploadAction` also threads it into `UploadDialog`, which takes
  the same prop and passes it to its own `Dialog.Content`. No deviation from D3 — every shape
  (prop name, `portalProps={{ to }}` wiring, which components take the prop) matches the design
  as written.
- Gate: `mise run check` exited 0 (lint, cargo fmt, tests — 573 rust passed, typecheck, clippy,
  builds all green). `grep -rn "closest('dialog')" packages/app/src` shows exactly one hit, in
  `portal.ts`. One pre-existing ESLint *warning* (not error) on `CollectionsSection.svelte:190`
  (max-len) — that file belongs to the other agent working on `domain/tag-utils.ts` /
  `CollectionsSection.svelte`, not touched here.
- For a reviewer: `Dialog.Content` (shadcn dialog-content.svelte) already destructures
  `portalProps` and forwards it to `DialogPortal`, so `UploadDialog`'s `Dialog.Content` took the
  prop with no shim needed — same for `DropdownMenu.Content` and `ContextMenu.Content`. Worth a
  look: `CollectionNameDialog`'s `portalTo` prop has no default binding to `undefined` beyond
  TypeScript's optional-prop default, matching how `TagInput`'s own `portalTo` behaves when
  `field` is `null`.

### Unit A (task group 1)

- 1.1: `query.rs` — `matching_ids(conn, req, ids)` on `Plan::for_request(req,
  RatingClause::Included)`, `SELECT id FROM {rows} WHERE id IN (…)` chunked by `ID_CHUNK`,
  `limit`/`offset` ignored. Added to `ID_CHUNK`'s doc-comment list of chunking callers. Test
  `matching_ids_answers_the_subset_the_request_still_matches` beside
  `search_ids_names_the_same_page_search_would_load`, same fixture. `cargo test matching_ids` —
  2 passed (this one plus the command-level one below).
- 1.2: `commands.rs` — `matching_ids` command registered in `lib.rs` right after
  `commands::search_ids`. Command-level test copied from `search_ids_matches_the_ids_search_would_page`'s
  shape. `api/commands.ts` — `matchingIds(req, ids)` beside `searchIds`, plus its invoke-shape
  test in `commands.test.ts`. `cargo test`, `pnpm --filter @boorubox/app test commands`, `mise
  run clippy` — all clean.
- 1.3: `MatchResolver` type added beside `IdResolver`. `keepMatching()` goes through `#editIds`.
  The anchor pickup lives inside the `multi` branch's `#editIds` callback (reads `this.anchor`,
  never `this.focus` — see the doc comment on `focusEntered` for why `focus` already moved by
  the time `click()` runs). `toggle(index, id)` is the checkbox's own entry, sharing the
  in-or-out rule with the multi-click through a module-level `toggleId(ids, id)` helper (zero
  duplication). `removeMany` and its test are gone; `remove(id)` stays, reimplemented directly
  on `#editIds` since it no longer has `removeMany` to delegate to.
  **Deviation from D1, not from its shape but from its mechanics:** to make "a gesture during the
  round trip wins" hold for `keepMatching` (whose round trip is resolve-then-ask-the-match-resolver,
  two awaits, not `#editIds`'s original one), `#editIds` itself now takes `edit: (ids) => void |
  Promise<void>` and checks the before/after guard *after* awaiting `edit`, not before it runs.
  The anchor pickup's own extra `#resolve(anchor, 1)` call rides inside the same `edit` closure
  for the same reason. This isn't a new rule, just the existing guard's span widened to cover
  whatever `edit` itself awaits — the design's text already assumes this ("the existing guard
  applies"; risk section: "the test for it copies the existing … shape"). `pnpm --filter
  @boorubox/app test selection` — 19 passed.
- 1.4: `Selection` built with both resolvers in `LibraryScreen.svelte`, sharing a `currentView()`
  closure for the `{sort, group, view}` object both pass to `buildSearchRequest` (avoids a third
  copy of that literal; `searchKeeping`'s own copy was left alone — out of scope here).
  `afterTrashWrite` is now `afterWrite()`: `results.refresh()`, `selection.keepMatching()`, the
  same focus clamp as before, no arguments. `write()` (trash/restore) and `destroy()`
  (delete-forever/empty) now call `library.refresh()`/`trash.refresh()` themselves — the badge
  counts only these two writers ever moved — then `afterWrite()`; `writeRating` and the sidebar
  `CollectionsSection`'s `onchanged` call `afterWrite()` directly. `ImageCard.svelte`'s checkbox
  calls a new `ontoggle` prop; `LibraryGrid.svelte` threads it to `selection.toggle(index,
  image.id)`.
  **Deviation, files outside the owned list:** `BulkTagDialog` is mounted only by
  `SelectionToolbar.svelte`, which the brief didn't list as owned by any unit. Wiring
  `onapplied` from the screen to the dialog needs a pass-through prop on it, so it was touched —
  a two-line addition (`onapplied` in `Props`, forwarded to `<BulkTagDialog>`), no other change.
  Flagging in case group review expected it untouched. Separately, `grid-focus.test.ts` (not
  owned by any unit either) constructed `new Selection(resolve)` with one argument; the
  `MatchResolver` parameter being required broke its typecheck, so both call sites there got a
  second `() => Promise.resolve([])` stub — mechanical, no behaviour asserted by those tests
  changed. `Inspector.svelte:311`'s doc comment still says `afterTrashWrite` by name (not an
  owned file); worth a one-line fix by whoever touches that file next.
  Verify: `mise run check` — lint clean but for the pre-existing `CollectionsSection.svelte`
  max-len *warning* noted by unit B (not from this unit), 557 app tests + 575 rust tests passed,
  clippy clean, both builds succeeded.
  Hand check: search `tagme`, select 50, bulk-remove `tagme` from them — the grid empties of
  them, the count and the strip follow, the toolbar's actions disappear at zero. Search
  `rating:s`, select five, rate `q` — same. Click A, ⌘-click B (macOS) / Ctrl-click B (Windows) —
  two selected. Click A, tick B's checkbox — one selected. Trash and restore still prune as
  before.
- Hand check (2.2) left for the owner, per instructions — not ticked, nothing added under it.

### Unit C (task group 3)

- 3.1: `packages/app/src/lib/components/library/focus-handback.ts` — `isOrphanedFocus(active,
  within?)` exactly per D4: `active` is `null`, is `document.body`, or sits inside `within`.
  `focus-handback.test.ts` covers the four cases D4's task names (jsdom). `ImageCard.svelte`'s
  `onmenuclose` now calls it with `menu` as `within`, replacing the inline three-way test it used
  to carry. `pnpm --filter @boorubox/app test focus-handback` — 4 passed.
- 3.2: `LibraryScreen.svelte` binds a new `onScreenClick` on `<svelte:window>` beside `screenKeys`
  (a DOM element under this screen's own root would miss the toolbar and the sidebar, which
  render into the frame's regions through `frame.toolbar`/`frame.sidebar`, not into this
  component's own subtree — the window is the one node every click bubbles through regardless of
  where it rendered). It calls `grid?.refocus()` when `selection.focus >= 0` and
  `isOrphanedFocus(document.activeElement)`. `LibraryGrid.refocus()` no longer routes through
  `showCard`/`scrollIntoView`: it queries the current card directly and calls
  `.focus({ preventScroll: true })`, with a doc comment on why (the card is already on screen, so
  a click that only orphaned the focus must not also move the scroll — unlike a keyboard move
  onto a row that may not be mounted yet). `Inspector.svelte`'s `onrelease` doc and
  `LibraryGrid.svelte`'s `refocus()` doc both widened from "a completed action" to name
  `browse-fixes` design D4's rule (any click that leaves no control focused) and say why
  `onrelease` still exists on its own (a keyboard-confirmed action reaches no click handler).
  Verify: `mise run check` — see the Rust-race note below; webview half all green.
  Hand check: click a card, click the panel's title text, press `→` — the focus moves to the
  next card without the grid scrolling; the same after clicking the page address text, the
  panel's empty space, the account rail's gap and the toolbar's band; click into the tag editor
  and press `→` — the caret moves and the grid does not; in the viewer, click the panel's text
  and press `→` — the next image is shown.
- Gate: another agent's in-progress Rust edits (`commands.rs`, `tags.rs`, `rules.rs` and others,
  none owned by this unit) left the crate mid-refactor — `selection_tag_counts` and
  `tags::split_rating`/`set_rating` call sites not yet matching their own changed signatures — so
  `cargo test`/`cargo clippy` fail to compile on errors entirely outside this unit's files, both
  before and after this unit's own edits. `query.rs`'s own diff (the `matching_ids` test's
  `limit: 1, offset: 1` from job 1) is a clean, self-contained hunk with no fmt drift. Ran instead,
  per the brief's fallback: `pnpm lint` (1 pre-existing warning, `CollectionsSection.svelte:190`,
  not from this unit), `pnpm typecheck` (0 errors), `pnpm test` (561 app + 94 extension + 1 shared,
  all passed — up from 557 app tests before this unit's 4 new `focus-handback` tests). Not
  re-verified against a green `cargo test`/`clippy`; owner should re-run `mise run check` once the
  other agent's Rust edits land.
