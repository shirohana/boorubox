> Depends on `browse-polish`, `selection-and-bulk` and `tags-and-ratings`, all archived and
> implemented. Schema stays v5; sidecar format stays 1.
>
> Two implementing agents, serial, split by file ownership: **R** owns group 1
> (`packages/app/src-tauri`, `packages/shared/src/index.ts`, `packages/app/src/lib/api/commands.ts`)
> and lands first; **T** owns groups 2–3 (`packages/app/src/lib/domain/`,
> `packages/app/src/lib/components/`, `packages/app/src/lib/api/search.svelte.ts` if it needs a
> helper) and starts from R's commit. Group 4 is the owner's.
>
> Every component in this repo is verified by hand: there are no component tests. The gate for
> each group is named in its tasks; `mise run check` is the gate for the change.

## 1. The record's account and the position of an id (agent R)

- [x] 1.1 `packages/app/src-tauri`: `ImageRecord` gains `account: Option<String>` (serde
      `account`), filled in `ingest::row_to_record` from `page_url` by `query::x_account` (made
      `pub(crate)`), never stored. Mirror it in `packages/shared/src/index.ts` as
      `account: string | null` with a doc comment saying it is derived from `pageUrl` by the
      search's own rule (design D4). Verify: a Rust test in `ingest.rs` that an image with page
      `https://x.com/alice/status/1` loads with `account == Some("alice")`, one with
      `https://x.com/home` with `None`, one with no page URL with `None`; `cargo test` and
      `pnpm -r typecheck` pass (the sidecar's `From<&ImageRecord>` and every struct literal
      compile).
- [x] 1.2 `packages/app/src-tauri/src/query.rs`: `search_position(conn, req, id) -> Result<Option<i64>>`
      — the zero-based row `id` occupies in `req`'s order, `None` when it does not match;
      built on the same `Plan` as `search_ids` with `ROW_NUMBER() OVER (<order>)` so the row
      agrees with what `search` pages (design D2). Verify: a test that on a five-image library
      sorted newest-first the third-newest id is at row 2, that the row matches the index
      `search_ids(everything)` returns it at, that an id excluded by the request's tags gives
      `None`, and that a request with a group set still agrees with `search_ids`.
- [x] 1.3 `packages/app/src-tauri/src/commands.rs` + `lib.rs` handler list +
      `packages/app/src/lib/api/commands.ts`: command `search_position(req: SearchRequest, id: String)
      -> Option<i64>` and wrapper `searchPosition(req, id): Promise<number | null>`. Verify: a
      command-level test beside `search_ids_matches_the_ids_search_would_page`; `mise run check`
      passes. Commit as one unit with 1.1–1.2.

## 2. Query readers and rewriters (agent T)

- [x] 2.1 `packages/app/src/lib/domain/tag-utils.ts`: `activeTerms(query)` returning the
      included/excluded tag sets and the included/excluded account sets from one
      `parseTagSearch` (design D3); `TagSidebar.svelte` switches to it and keeps its behaviour.
      Verify: unit tests in `tag-query.test.ts` for a query with tags, an `or` group, an
      exclusion and an account; the sidebar's existing rendering unchanged in the app.
- [x] 2.2 `packages/app/src/lib/domain/tag-utils.ts`: `toggleAccountInQuery(query, handle)`
      adding or removing `account:<handle>` by rewriting the whole `account:` list (design D4,
      D6), leaving `-account:` terms and everything else intact. Verify: unit tests — empty
      query → `account:alice`; `cat account:alice` toggled → `cat`; `account:alice,bob` toggled
      `alice` → `account:bob`; `-account:eve` untouched by toggling `alice`; case kept (`Bob`).

## 3. The panel, the grid and the screen (agent T)

- [ ] 3.1 `packages/app/src/lib/components/common/ExternalLink.svelte`: an icon button taking
      `url` that calls `openExternal` and shows "This link could not be opened: …" under
      itself on failure; rendered only for `http:`/`https:` (design D5). `Inspector.svelte`
      places it after the Page and Image values. Verify: typecheck and lint pass; in the app the
      button opens the page in the browser and is absent for a local import.
      Hand check: press the button after Page on an X capture — the browser opens the post and
      the app stays on the library; a file-imported image shows no button.
- [ ] 3.2 `packages/app/src/lib/components/library/LibraryGrid.svelte`: export `refocus()` —
      when `selection.focus >= 0`, `showCard(selection.focus)`; no change to the selection
      (design D1). `Inspector.svelte`: rename `onrated` to `onrelease`, fire it after a chosen
      rating, a successful save from the editor or the Save button, a removed tag, and a tag or
      account acted on as a search term; never after a failed save. `Lightbox.svelte` passes
      `onrelease={() => surface?.focus()}`; `LibraryScreen.svelte` passes
      `onrelease={() => grid?.refocus()}` in the grid placement. Verify: typecheck and lint
      pass.
      Hand check (Windows and macOS): click a rating in the panel beside the grid, press → — the
      focus moves to the next card; confirm the tag editor with Enter, press Space — the viewer
      opens; press Save with the mouse, press `i` — the panel toggles; in the viewer, click a
      rating then press Space — the viewer closes; make a save fail (unplug the library folder
      or use a bad tag write) — the focus stays in the editor.
      Hand check (reviewer, 2026-09-15): inside the viewer, use a tag badge's right-click menu →
      "Search for this tag" — bits-ui restores focus to the badge as the menu closes, which may land
      after `onrelease`'s `surface.focus()`; if Space then does not close the viewer, the release
      must run after the menu's close (a `tick()` or the menu's `onOpenChange`), not before.
- [ ] 3.3 `packages/app/src/lib/components/library/Inspector.svelte`: badges take the marking
      from `activeTerms` — included green, excluded struck through, the sidebar's classes; the
      account entry (`image.account`) renders first under the editor on its own row, blue when
      inactive, the tag marking when included or excluded, toggling with
      `toggleAccountInQuery`; absent when `account` is null (spec `tag-editing`, "An X account
      on screen is a search term"). Verify: typecheck and lint pass.
      Hand check: with search `cat -dog`, an image tagged both shows `cat` green and `dog` struck;
      an X capture shows its handle in blue above the tags; clicking it puts `account:<handle>`
      in the search bar and turns it green; a Pixiv capture shows no account entry.
- [ ] 3.4 `packages/app/src/lib/components/library/LibraryScreen.svelte`: `searchFor` becomes
      `searchKeeping(next, id)` per design D2 — the inspector passes the id of the image it
      describes (`onquery(next, id)`); the screen runs the search, asks `searchPosition` in
      parallel with a generation guard, then `selection.reset()` and `grid.focusCard(row)`; with
      the viewer open, `lightboxIndex = row` and the viewer stays open; on `null`, today's reset
      and the viewer closes. `runSearch` (typed) unchanged. The sidebar's and rating pills'
      `onquery` keep the focused image the same way (the id is the focused image's), so every
      click-driven rewrite behaves alike. Verify: typecheck and lint pass; `pnpm --filter
      @boorubox/app test` passes.
      Hand check: with the inspector beside the grid, scroll to row ~200 of an empty search, click
      a tag in the panel — the grid shows the narrowed result scrolled to the same image with its
      ring, the panel still describes it, the arrows move from it; right-click a tag → "Exclude
      from the search" — no image is current; open the viewer in inspect mode, click a tag — the
      viewer stays open on the same image and → goes to the next one in the narrowed result;
      exclude from inside the viewer — it closes; select three images, click a tag in the panel —
      the selection toolbar disappears and the focused image is current; on the 25k vault a tag
      click from the panel lands within a second.

## 4. Change-level verification (owner)

- [ ] 4.1 `mise run check` green on the folded change; the four hand checks above pass on
      Windows.

## Handoff

Group 1 (agent R) landed on `main`, uncommitted (per instructions — owner commits). Full gate
(`mise run check`) is green: lint (eslint + `cargo fmt --check`), typecheck (0 errors), tests
(cargo 511 passed, `pnpm -r test` 465+94+1 passed), clippy (`-D warnings`, clean), build. No
migration; schema stays v5, sidecar format stays 1.

**Signatures as they exist:**
- Rust: `pub fn search_position(conn: &Connection, req: &SearchRequest, id: &str) -> Result<Option<i64>>`
  in `query.rs`, built on `Plan::for_request` + `ROW_NUMBER() OVER (ORDER BY <plan.order_by()>) - 1`
  filtered to `id`, via `.optional()`.
- Command: `pub async fn search_position(req: SearchRequest, id: String, state: State<'_, AppState>) -> Result<Option<i64>>`
  in `commands.rs`, registered in `lib.rs`'s `generate_handler!` right after `commands::search_ids`.
- Wrapper: `export function searchPosition(req: SearchRequest, id: string): Promise<number | null>`
  in `packages/app/src/lib/api/commands.ts`, right after `searchIds`.
- `query::x_account` is now `pub(crate) fn x_account(url: &str) -> Option<&str>` (was private).
- `ImageRecord.account: Option<String>` (Rust) / `account: string | null` (TS) sits right after
  `page_title`/`pageTitle` in both structs. Derived in `ingest::row_to_record` from the row's own
  `page_url` via `crate::query::x_account`, never a column, never in `Sidecar` (its
  `From<&ImageRecord>` is a field-by-field literal that simply doesn't name it — nothing to guard
  there).

**Deviation from the plan (small, mechanical, flagged here):** the boundary said not to touch
`packages/app/src/lib/domain/`, but `packages/app/src/lib/domain/image-fixture.ts`'s `img()`
builder is a full, non-partial `ImageRecord` object literal (`const merged: ImageRecord = {...}`),
so adding a required field to the shared type left it one property short and `pnpm -r typecheck`
red. Added one line, `account: null,`, next to `pageTitle: null,` — no other change to that file.
Flagging rather than silently leaving it since agent T owns that directory; nothing else under
`domain/` or `components/` needed a touch (grepped for other full `ImageRecord` literals — this
was the only one).

**For agent T:** `image.account` is populated wherever `ImageRecord` comes back from any command
(`search`, `search_ids` gives ids only, `update_tags`, `set_rating`, single-image reads) — no
extra round trip needed to read it off the inspected image. `searchPosition(req, id)` returns
`null` (not `undefined`) when `id` is outside `req`'s matched set, mirroring Rust's `None`; JSON
`null` round-trips as TS `null` the same way every other nullable field here does. The account
column ordering in both structs (`page_url`/`pageUrl` → `page_title`/`pageTitle` → `account`) is
not load-bearing for JSON (field order doesn't matter over the wire) — just where it physically
sits in the source if you're grepping.

## Group 2–3 (agent T)

Groups 2 and 3 landed on `main`, uncommitted. Gate green: `pnpm -r typecheck` (0 errors),
`pnpm lint` (clean), `pnpm --filter @boorubox/app test` (475 passed), `mise run check` (lint,
typecheck, cargo 511 passed, `pnpm -r test`, clippy `-D warnings` clean, build) all green. 2.1
and 2.2 ticked — their tests pass. 3.1–3.4 left unticked: each carries a Hand check.

**Signatures as they exist:**
- `activeTerms(query: string): { included: Set<string>, excluded: Set<string>, accounts:
  Set<string>, excludedAccounts: Set<string> }` in `tag-utils.ts`. `TagSidebar.svelte` now
  derives its `included`/`excluded` from this instead of its own `parseTagSearch` read — same
  sets, same rendering.
- `toggleAccountInQuery(query: string, handle: string): string` in `tag-utils.ts`, beside
  `toggleRatingInQuery`. Implemented by filtering out tokens matching `/^account:/i` and
  rebuilding the metatag from `parsed.accounts` (not a global regex substitution like
  `RATING_METATAG`) — `-account:` tokens never match that filter, so exclusions survive
  untouched without a lookbehind.
- `ExternalLink.svelte` (new, `components/common/`): props `{ url: string | null }`. Decides
  its own visibility (`new URL(url).protocol` is `http:`/`https:`, caught otherwise) rather
  than making each caller check, since both the Page and the Image row need the identical
  check — `Inspector.svelte` just drops `<ExternalLink url={image.pageUrl} />` /
  `imageUrl` after each value, no protocol logic duplicated there.
- `LibraryGrid.svelte` export `refocus(): void` — `showCard(selection.focus)` when a focus
  exists, never touches `selection.focusAt`.
- `Inspector.svelte`: `onrated` renamed to `onrelease?: () => void`; `onquery` is now
  `(next: string, id: string) => void` (the id is `image` — the panel's own selection-aware
  derived, not always the focused card). A local `query(next)` helper calls
  `onquery(next, image.id)` then `onrelease?.()` synchronously (before the rewrite settles,
  same rule `RatingControl`'s `onchosen` already followed) — every tag badge, the context
  menu's two query items, and the new account button all go through it. `write()` (the tag
  save/remove path both the Save button and `submitFromEditor` funnel through) calls
  `onrelease?.()` itself, only after `results.saveTags` resolves — never on a caught error.
  Badge marking and the account row's colours read `activeTerms(tagQuery)` (design D3) with the
  sidebar's own emerald/destructive classes; the account row adds a third, blue
  (`bg-sky-500/15 text-sky-700 dark:text-sky-300`), for "shown but not in the query".
- `Lightbox.svelte`: `onquery` retyped to `(next: string, id: string) => void` (forwarded
  through to `Inspector` unchanged); `onrated` → `onrelease={() => surface?.focus()}`.
- `LibraryScreen.svelte`: `searchFor` is gone, replaced by
  `async function searchKeeping(next: string, id: string | undefined): Promise<void>`. With
  `id === undefined` it's `runSearch({ tagQuery: next, text })` verbatim (nothing focused —
  no subject to keep, falls back to the plain reset a typed search already does). Otherwise it
  builds `nextInputs` once, sets `tagQuery = next` and `selection.reset()` with the run (they
  cost the same whatever the position turns out to be), then runs `results.run(nextInputs)` and
  `searchPosition(...)` in `Promise.all`. **Review fix:** the race guard (design D2 risk) was
  written as reference equality on `results.inputs` and could never hold — `#start` does assign
  `this.inputs = inputs` before its first `await`, but `$state` stores a *proxy* of what was
  assigned, so `results.inputs !== nextInputs` was true on every click and the whole tail of
  `searchKeeping` was dead. It now captures `results.generation` right after `run()` has started
  (bumped synchronously, same as `inputs`) and returns when a newer run has replaced it — the
  guard `SearchResults` uses for its own pages. On a row: `grid?.focusCard(row)` and (only if
  `lightboxOpen`) `lightboxIndex = row` plus `results.ensureRange(row, row + 1)`, leaving the
  viewer open. On `null`: `lightboxOpen = false`.
  - `TagSidebar`/`RatingPills` keep their own `onquery: (next: string) => void` (unchanged
    components); wired at the call site as
    `onquery={(next) => void searchKeeping(next, focused?.id)}` — `focused`, not the
    inspector's selection-aware `image`, per the task's own wording ("the id is the focused
    image's").
  - `Inspector`'s and `Lightbox`'s `onquery` are wired as `onquery={searchKeeping}` directly —
    both already call with `(next, id)`, matching `searchKeeping`'s own signature (a
    `string | undefined` id parameter accepts the `string` both pass).
  - `Inspector`'s `onrelease` (grid placement) is `() => grid?.refocus()`.

**Open items for the owner / reviewer (group 4, hand checks):** all four hand checks under
3.1–3.4 are unverified by hand — this agent does not run the app. Two worth a closer look
given the design's own risk notes:
- **3.4's 25k-vault timing hand check** — `search_position` is one `ROW_NUMBER()` query per
  click (design D2, same cost class as `tag_counts`), but nothing in this pass measured it
  against a real 25k-row library; the risk note in `design.md` under D2/Risks asks for this
  explicitly.
- **The Windows half of 3.2's hand check** — the focus hand-back (`refocus()`,
  `surface?.focus()`) was built and typechecks/lints clean, but keyboard-focus behaviour is
  exactly the class of thing that has differed between engines in this codebase before
  (WebKit's button-focus-on-click note in the briefing); it has not been run on Windows or
  macOS by this agent.

No change was needed to `packages/app/src/lib/api/search.svelte.ts` — `searchKeeping` reuses
`buildSearchRequest` and `SearchInputs` exactly as `LibraryScreen` already imported them; no new
helper was needed there.
