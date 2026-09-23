> Two units. Unit C (Sonnet, `packages/app` webview only, runs beside agents A and B): tasks
> 1.x. Unit D (Sonnet, `packages/app/src-tauri` + `TagSidebar.svelte`, after A and C have
> landed): tasks 2.x. Design D1–D8 decide every shape; do not re-decide them. Gate for every
> unit: `mise run check` green. Do not commit; the lead commits by file set. Do not tick a
> hand check. Unit C does not touch `query.rs`, `tags.rs`, `tag-utils.ts` or `stamp.ts`
> (agent A owns them) nor `StampBar`, `LibraryScreen`, `LibraryGrid`, `ImageCard` (agent B).

## 1. Unit C — the panels (`packages/app`)

- [x] 1.1 `domain/tag-categories.ts`: `CATEGORY_ORDER` per D1; `tag-categories.test.ts` and
      `tag-input.test.ts` (the editor's lines) updated; the `tag-editing` spec delta already
      names the new line order. `components/tags/categories.ts`: general's colour per D2,
      `SEARCH_MARK_CLASS` and `searchMark` per D3 with a test (`categories.test.ts`) covering
      included / excluded / none for a tag and for an account. Amend the archived
      `openspec/changes/archive/2026-09-23-tag-vocabulary/design.md` D6 with a dated
      *Amended (tag-panel-polish, 2026-09-24)* paragraph carrying D2's argument.
      Verify: `pnpm --filter @boorubox/app test tag-categories tag-input categories` pass.
- [x] 1.2 `Inspector.svelte` per D3, D4, D5, D6: tags as plain text with the shared marking;
      the account row in the facts list above Page and the old button removed; `startEditTags`
      focusing the end through `TagInput.focusEnd()` (D5, `TagInput.svelte`); the pin glyph on
      the chips. The section's comments rewritten to the new shapes (no story of the old
      ones). Verify: `mise run check` green.
      Hand check: an image tagged `kantoku` (artist) and `1girl` shows red and blue plain
      text; searching `1girl` marks it with the tint and underline; the account handle sits
      above Page and toggles `account:` in the search with the same marking; Edit puts the
      caret after the trailing space; pinned chips show a pin glyph and still toggle. Edit
      opens with no suggestion list; one Escape cancels.
      Seen by the lead on a scratch copy of test-1 seeded through the UI (smoke run
      2026-09-24, driven through accessibility, not the owner's hands): red/violet/green/blue
      plain text grouped by colour; the searched tag carried the tint and underline in both
      panels; the Account row sat between Source and Page on an X image and was absent on a
      local one; Edit put the caret after the trailing space with no popover and one Escape
      closed it; the pinned chip showed the pin glyph. Two defects fixed after the run: the
      chip's carried/not-carried fill was unreadable at 1× (now the pin itself fills), and
      Escape did nothing while the focus sat on Save after a refused save.
- [x] 1.3 `TagSidebar.svelte` per D7 (groups, labels, the shared marking on the name; the
      `missing` merge stays until unit D removes it). Verify: `mise run check` green.
      Hand check: with no search, the list reads Artist, Copyright, Character, General, Meta
      groups, each alphabetical; with `cat` searched, the order does not change and `cat` is
      marked.
      Seen by the lead (smoke run 2026-09-24): ARTIST, COPYRIGHT, CHARACTER, GENERAL, META
      labels in that order, alphabetical inside, general blue, meta amber; the searched row
      kept its place and carried the marking.

## 2. Unit D — zero rows from Rust (`packages/app/src-tauri`, `TagSidebar.svelte`)

- [x] 2.1 `query.rs`: `tag_counts` per D8, with tests: an excluded `dog` no result carries
      is listed at zero; a searched `zzz` that no tag has is absent; an or-group member that
      exists but has no carrier in the result is at zero; a name that exists with a carrier
      is counted once, not twice. Verify: `cargo test query::` pass, `mise run clippy` clean.
- [x] 2.2 `TagSidebar.svelte`: the `missing` merge deleted (Rust lists the zero rows); the
      component's comment says so. Verify: `mise run check` green.
      Hand check: searching `non-exist` shows "No tags in these results"; searching `-dog`
      on a result with no `dog` still lists `dog 0`, marked excluded, with a working menu.
      Seen by the lead (smoke run 2026-09-24): `zzzq` gave "No tags in these results";
      `-blue_archive` listed `blue_archive 0` struck through, and its right-click menu opened
      with the categories and Pin.

## 3. Unit E — the panels after the owner checked the app (`packages/app`), folded into this change

> Owner, 2026-09-24, from the running app: the group labels are noise (the colour tells the
> story); the underline makes English hard to read; the search-marked tags belong on top as
> before; a background alone is the marking, on the whole hoverable area, and no
> strike-through now that tags are coloured; the Account row does not look clickable. The
> left panel's marking was fine before unit C touched it — the complaint was the right panel's
> pills. Agent E (Sonnet), running beside agent F (`ImageCard`, `LibraryGrid`,
> `grid-window.ts`, `LibraryScreen`, the `tile-tags-in-edit-mode` change) — do not touch their
> files. Gate: `mise run check` green. Do not commit; do not tick a hand check. This unit is
> folded into this change's commit, so the archived design and the delta specs are amended to
> the shipped shape, and the main specs (`openspec/specs/tag-sidebar`, `tag-editing`,
> `tag-vocabulary`) are edited by hand to match, since the archive has already run.

- [x] 3.1 `components/tags/categories.ts`: `SEARCH_MARK_CLASS` is background only —
      `included: 'bg-emerald-500/15 font-medium'`, `excluded: 'bg-destructive/10'`, `none:
      ''` — with the doc comment saying why there is no underline and no strike-through
      (owner: an underline makes English hard to read; a coloured word struck through is
      one colour too many). `categories.test.ts` updated. Verify: `pnpm --filter
      @boorubox/app test categories` passes.
- [x] 3.2 `TagSidebar.svelte`: no group labels, one flat `<ul>`; order = the tags the search
      includes or excludes first, then the rest, each half through `groupByCategory` (so
      artist, copyright, character, general, meta, alphabetical inside); the marking class
      on the row `div` (the whole hoverable area, as before unit C), not on the name button;
      comments rewritten. Verify: `mise run check` green.
      Hand check: with `1girl -highres` searched, the list reads `1girl` (green row) then
      `highres` (red row, no strike-through), then the rest grouped by colour with no labels.
      Seen by the lead on the seeded scratch vault (smoke run 2026-09-24, second pass): a flat
      list with no labels, red/violet/green then blue then amber; with `blue_archive -highres`
      searched both rows came first, green and red full-row tints, no underline or strike;
      hovering the green row kept it green, brighter. Rows tightened afterwards (owner: more
      rows on screen) — not re-seen.
      Seen later the same day: the rows sit on a 19 px pitch; the inspector's tag list wraps
      with no vertical gap.
- [x] 3.3 `Inspector.svelte`: each tag button gets `rounded-md px-1 py-0.5` and
      `hover:bg-accent` so the tint covers a real box; the marking class on the button; the
      Account row's button reads `@{handle}` with the same padding, hover background, a
      pointer cursor and `title="Search for this account"`, the marking class when active,
      no colour of its own; the collection chips keep the shared marking. The tag-editing
      spec delta (archived copy and `openspec/specs/tag-editing/spec.md`): "Tags on screen are
      search terms" says background only, whole box, no underline or strike-through; "An X
      account on screen is a search term" says the row shows `@handle` as a button that
      looks clickable. Verify: `mise run check` green.
      Hand check: the active tag is a green block, the excluded one a red block, both the
      same weight as their neighbours apart from medium on the included; `@alice` shows a
      hover background and a pointer, and a green block when in the search. Hovering an
      active tag keeps its tint, darker; an excluded account or collection in the sidebar
      shows a red row with no strike-through.
      Seen by the lead (smoke run 2026-09-24, second pass): the searched tag a green block over
      its padded box, hover still green; `@handle` with a hover background, pointer and tooltip,
      a green block once clicked with `account:` in the search; excluded entries red with no
      strike in the account rail (grouped by X account) and the collections rows.
- [x] 3.4 Docs: design D3 amended (background only, why), D7 amended (no labels, active
      first, why: owner 2026-09-24), each as an *Amended* paragraph keeping the earlier
      argument; the tag-sidebar delta (archived copy and `openspec/specs/tag-sidebar/spec.md`)
      requirement "The sidebar lists…" says: the tags the search includes or excludes first,
      then the rest, each part in the category order, alphabetical inside, no labels; its
      "Order" scenario rewritten to that; the tag-vocabulary spec's "A mixed panel" scenario
      and any sentence naming an underline or a strike-through corrected. Verify:
      `openspec validate --specs` passes.

## Handoff

(written by the agents: what landed, deviations)

### Unit C

Landed 1.1–1.3, `packages/app` only:

- `domain/tag-categories.ts`: `CATEGORY_ORDER` is now `['artist', 'copyright', 'character',
  'general', 'meta']` (D1). `tag-categories.test.ts` and `tag-input.test.ts` updated for the
  new order (the `editorText` line-order test's expected string changed to put the general
  line before meta).
- `components/tags/categories.ts`: `CATEGORY_TEXT_CLASS.general` is now `text-blue-600
  dark:text-blue-400` (D2); added `SEARCH_MARK_CLASS` (`included` / `excluded` / `none`) and
  `searchMark(name, included, excluded)` (D3) — the function takes the two sets directly
  (whichever pair `activeTerms` produced: `terms.included`/`terms.excluded` for a tag,
  `terms.accounts`/`terms.excludedAccounts` for an account), not the whole `ActiveTerms`
  object, since the two callers read different fields of it. New `categories.test.ts` covers
  included/excluded/none for both a tag-shaped pair and an account-shaped pair.
- Archived `2026-09-23-tag-vocabulary/design.md` D6 amended with a dated paragraph (general's
  new colour, why both of D6's old reasons ended) between the colour table and D7's opening —
  the decision is not re-argued from the old text.
- `Inspector.svelte`: tag list is now `<button class="text-xs …">` (no `Badge`), `flex
  flex-wrap gap-x-2 gap-y-1`, `CATEGORY_TEXT_CLASS` + `SEARCH_MARK_CLASS[searchMark(...)]`.
  Account moved into the facts `<dl>` as a row between Source and Page, shown in both
  `editingFacts` states (it never becomes an `<Input>`; it is derived from the stored
  address, not the draft), `text-foreground` + `SEARCH_MARK_CLASS`, same `toggleAccountInQuery`
  handler. The old button under the Tags heading is gone. `startEditTags` now calls
  `tick().then(() => tagInput?.focusEnd())` after setting `editingTags = true`, since the
  field mounts on that flag. Pinned chips gained a `PinIcon` (`size-3 shrink-0`) before the
  name; the `Badge` got `flex items-center gap-1` to hold it. Every comment the shape change
  touched (badge-list, pinned-chip, account row) is rewritten to the new shape.
- `TagInput.svelte`: exported `focusEnd()` — `field?.focus()` then
  `field?.setSelectionRange(value.length, value.length)`.
- `TagSidebar.svelte`: `listed` replaced by `groups` — `CATEGORY_ORDER.map(category => ({
  category, rows: [...tags, ...missing].filter(...).sort(localeCompare) })).filter(non-empty)`.
  Active-first/count sort is gone; a label `<li>` (`categoryLabel`) precedes each non-empty
  group. The row's wrapper `<div>` no longer carries the include/exclude tint — that moved
  onto the name button alongside `CATEGORY_TEXT_CLASS`, via `SEARCH_MARK_CLASS[searchMark(...)]`;
  the `+`/`−` buttons and the count stay untinted, and `font-medium` on an included row is
  gone (D3: weight never reflows the list). The `missing` computation (tags the query names
  that the current result does not carry) is untouched, exactly as the brief said — unit D
  removes it once `query.rs` supplies zero rows.
- `TagVocabularyMenuItems.svelte` needed no edit: it already imports `CATEGORY_ORDER` from
  `domain/tag-categories`, so it picked up D1's new order for free.

Deviations: none from the design. `pnpm --filter @boorubox/app test tag-categories tag-input
categories` — 53 passed. Full `pnpm --filter @boorubox/app test` — 627 passed.
`pnpm --filter @boorubox/app typecheck` — 0 errors on my own changes (a later run, after
agent B's concurrent edit landed a `LibraryScreen.svelte` prop mismatch, showed one error in
that file — not touched by unit C, and `pnpm exec eslint` on every file unit C owns comes back
clean). `mise run check` currently fails on two files outside unit C's ownership, both
mid-edit by other agents: `packages/app/src-tauri/src/tags.rs` (agent A, `cargo fmt --check`
diff) and `packages/app/src/lib/components/library/StampBar.svelte` (agent B, `no-undef`
errors for names not yet defined). Neither is in unit C's file list; re-run `mise run check`
once A and B have landed.

Hand checks (1.2, 1.3) are for the owner — left unticked, not attempted.

### Unit D

Landed 2.1–2.2, `packages/app/src-tauri` + `TagSidebar.svelte`:

- `query.rs`: added `zero_rows_for_request_tags(conn, req, counted)`, called from the
  top-level `pub fn tag_counts` right after `Plan::tag_counts` produces the carried rows. It
  canonicalises (`tags::canonical`) every name of `req.query.include_tags`, `exclude_tags`
  and the flattened `or_groups`, drops the ones already carried, and — only for what is left
  — runs one `SELECT name FROM tags WHERE name IN (…) ORDER BY name` to keep only names that
  are real rows; each surviving name becomes a `TagCount { count: 0 }`, appended after the
  carried rows. `Plan::tag_counts` (the per-request SQL over the matched set) is untouched;
  the merge is the one extra query design D8 asks for, over the request's own names, never
  the result.
- `model.rs`: `TagCounts.tags` gained a doc comment naming the zero-row half, since the field
  itself carried none before (its siblings `collections`/`accounts` already did).
- Four new tests in `query.rs`'s `tests` module, all passing: an excluded `dog` with no
  carrier in the result is listed at zero (`an_excluded_tag_with_no_carrier_in_the_result_is_listed_at_zero`);
  a searched `zzz` naming no tag row is absent entirely
  (`a_search_term_naming_no_tag_at_all_is_absent`); an or-group member that is a real
  (pinned) tag with no carrier is listed at zero
  (`an_or_group_member_that_exists_but_has_no_carrier_in_the_result_is_at_zero` — built by
  linking then unlinking a tag pinned in between, so its row outlives the image that once
  carried it); a queried tag the result does carry appears exactly once, not doubled
  (`a_queried_tag_with_a_carrier_is_counted_once_not_twice`).
- `TagSidebar.svelte`: the `missing` computation (the `Set` diff plus zero-count objects) is
  gone from the `groups` derivation, which now just calls `groupByCategory(tags, …)` directly
  — `tags` already carries the zero rows. The header comment and the `groups` doc comment are
  rewritten to say Rust supplies the zero rows now, not the webview. `included`/`excluded`
  (from `activeTerms`) are unchanged and still feed `searchMark` for the row's tint.

Deviations: none from the design. `cargo test query::` — 67 passed. `mise run clippy` —
clean. `mise run check` — green end to end (657 Rust tests + 1 pre-existing ignored, 642
`packages/app` tests, lint/typecheck/clippy/builds all pass; one pre-existing lint *warning*
in `CollectionsSection.svelte:190` — a line-length warning, not an error, in a file unit D
never touched).

Hand check (2.2) is for the owner — left unticked, not attempted.

### Unit E

Landed 3.1–3.4, `packages/app` only, from the owner's read of the running app:

- `components/tags/categories.ts`: `SEARCH_MARK_CLASS` is background only — `included:
  'bg-emerald-500/15 font-medium'`, `excluded: 'bg-destructive/10'`, `none: ''` — no underline,
  no strike-through. `included` keeps `font-medium`, the one weight change left, since a
  background alone read as too faint a cue once the underline was gone. Doc comment rewritten
  to carry the owner's reasons (underline makes English hard to read; a coloured, struck
  tag is one colour too many). `categories.test.ts` gained two assertions: no
  `underline`/`line-through` in either class, and `font-medium` only on `included`.
- `TagSidebar.svelte`: group labels removed, one flat `<ul>`. The `groups` derivation is now
  `rows`: the tags the search includes or excludes first, then the rest, each half through
  `groupByCategory` (so both halves keep the category order, alphabetical inside) —
  `groupByCategory` is called twice rather than a second hand-written comparator for
  "active first", so the order rule stays in one place. The search-marking class moved off
  the name button onto the row `div` (the whole hoverable area, matching the sidebar's shape
  from before this design's first pass); the name button now carries only
  `CATEGORY_TEXT_CLASS`. Header comment and the `rows` doc comment rewritten; `categoryLabel`
  import dropped (no longer called anywhere in the file).
- `Inspector.svelte`: each tag button gained `rounded-md px-1 py-0.5 hover:bg-accent` so the
  background marking has a real box to fill; the marking class is unchanged (already on the
  button). The account row's button dropped `hover:underline` for the same
  `rounded-md px-1 py-0.5 hover:bg-accent`, plus `cursor-pointer`; its label is now `@{account}`
  (the stored handle is unchanged — this is display only, `toggleAccountInQuery` still gets
  the bare handle). Both comments (the tag-list one above the `<ul>`, the Account row's above
  the `<dt>`) rewritten to the new shapes and the owner's 2026-09-24 reasons. The collection
  chips were not touched — they already read `SEARCH_MARK_CLASS`, so the background-only class
  reached them for free.
- Docs: `design.md` D3 and D7 each gained an *Amended (tag-panel-polish, 2026-09-24)* paragraph
  keeping the original argument and recording why it stopped holding (D3: the underline/
  strike-through traded against readability once every tag carried colour; D7: the labels were
  noise once colour carried the grouping, and D7's own "active-first is gone" sentence is
  flagged as answering a different complaint than the one this amendment answers). The
  archived `specs/tag-editing/spec.md` and `specs/tag-sidebar/spec.md` (both this change's
  delta copies) and the matching main specs (`openspec/specs/tag-editing/spec.md`,
  `openspec/specs/tag-sidebar/spec.md`) were edited in lockstep: "Tags on screen are search
  terms" and "An X account on screen is a search term" (background only, `@handle`, looks
  clickable), "The sidebar lists…" and its "Order" scenario (active-first-then-rest, no
  labels; added an "Order: the search's own tags surface first" scenario for the two-tag
  case). `tag-vocabulary`'s "A mixed panel" scenario and its requirement text named no
  underline or strike-through to begin with, so neither the archived nor the main copy needed
  a change there — checked, not skipped.

Deviations: none from the brief. `pnpm --filter @boorubox/app test categories` — 15 passed.
`openspec validate --specs` — 29 passed, 0 failed. `mise run check` — green end to end
(lint, typecheck — 1179 files, 0 errors — 648 `packages/app` tests + 657 Rust tests + 1
pre-existing ignored, clippy, build). It failed once mid-run on 197 pre-existing
`svelte/indent`/`svelte/no-useless-mustaches` errors in `ImageCard.svelte`, agent F's file
mid-edit for `tile-tags-in-edit-mode` — not touched by unit E; a re-run after agent F's file
landed came back clean. The only lint findings inside files unit E owns were two
`@stylistic/max-len` warnings in `TagSidebar.svelte`, from wrapping the new `rows` derivation
and the name button's class list — fixed by hand, both now under the line-length. The
pre-existing `CollectionsSection.svelte:190` length warning (unit D's report) is unchanged
and not unit E's file.

Hand checks (3.1's Verify line has none; 3.2 and 3.3's `Hand check:` lines) are for the
owner — left unticked, not attempted.
