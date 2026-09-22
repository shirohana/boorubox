> Three units. Unit R (Rust + shared) first; then unit W1 (webview stores, pure modules, menu
> fragment, wiring); then units W2a and W2b side by side (disjoint files). Design D1–D9 decide
> every shape; do not re-decide them. Gate for every unit: `mise run check` green. No unit ticks
> a hand check; each writes a `Hand check:` line under it and leaves the box. The migration's
> number is whatever `MIGRATIONS.len()` is when unit R lands: amend design D1's sentence to it.

## 1. Unit R — the vocabulary in Rust (`packages/app/src-tauri`, `packages/shared`)

- [x] 1.1 `db.rs`: the migration per D1 with a doc comment saying why two columns and why the
      `CHECK`; the eight `MIGRATIONS.len()` assertions keep passing; a test that an upgraded v6
      database reads every tag as `general`, unpinned. Verify: `cargo test db::` passes.
- [x] 1.2 `model.rs` + `packages/shared/src/index.ts`: `TagCategory` (five lower-case names,
      serde lower-case; a `FromStr`/`as_str` pair), `TagEntry { name, category, pinned }`, with
      a wire test in camel case. Verify: `cargo test tag_entry`, `pnpm -r typecheck` green.
- [x] 1.3 `tags.rs`: `read_metatags` replacing `split_rating` (D4; short forms included),
      `Conflict { Refuse, Keep }`, `link_tags` answering whether a categorised row was born, the
      one conflict message function; `update_tags` and `bulk_update_tags` use `Refuse` and
      rewrite `library.json` when told to; `ingest::insert_rows` uses `Keep`;
      `collect_orphans` spares categorised or pinned rows (D3); `suggestions` LEFT JOINs (D3);
      `vocabulary(conn)`, `set_category(library, name, category)`, `set_pinned(library, name,
      pinned)` each rewriting `library.json` and answering the vocabulary. Tests, copying the
      existing shapes: every spec scenario of `tag-vocabulary` "A prefix creates a tag under a
      category" (creating, existing plain, existing with prefix, conflict in editor, conflict in
      bulk with nothing changed, conflict in a rule at capture keeps the tag), "The vocabulary
      outlives its carriers" (artist survives delete-forever and is suggested at 0; general goes),
      and `read_metatags` over `Artist:Cat rating:s art:foo copyright: bar` yielding tags
      `[Cat, foo, bar]`, rating `s`, categories `[(Cat, artist), (foo, artist)]`. Verify:
      `cargo test tags::` passes, `mise run clippy` clean.
- [x] 1.4 `sidecar.rs` + `recover.rs`: `LibraryFile.tags` per D2, written by `write_library`
      from `vocabulary`; the rebuild restores it after the sidecar pass with the upsert of D2;
      tests: the round-trip test covers the new field, the old-file test shows a file with no
      `tags` key reads as `None`, and a rebuild test seeds `kantoku` artist on one image, `tagme`
      pinned, `azur_lane` copyright on none, rebuilds, and finds all three as the spec's "The
      vocabulary comes back" says. Verify: `cargo test sidecar:: recover::` pass.
- [x] 1.5 `commands.rs` + `lib.rs`: `tag_vocabulary()`, `set_tag_category(name, category)`,
      `set_tag_pinned(name, pinned)`, each answering `Vec<TagEntry>`; `selection_tag_counts`
      gains an optional `names: Option<Vec<String>>` filter (D8). Command-level tests copied
      from `collection_list`'s. Verify: `cargo test commands::` passes; `mise run check` green.

## 2. Unit W1 — stores, pure modules, the menu fragment, wiring (`packages/app`), after unit R

- [x] 2.1 `api/commands.ts`: `tagVocabulary`, `setTagCategory`, `setTagPinned`, and the
      `names` argument on `selectionTagCounts`, with invoke-shape tests. `api/vocabulary.svelte.ts`
      per D5, exported from `api/index.ts`, with `vocabulary.svelte.test.ts` copied from
      `collections.svelte.test.ts`: refresh, `categoryOf` defaults to general, `pinned` sorted,
      the setters replace the list. Verify: `pnpm --filter @boorubox/app test vocabulary commands` pass.
- [x] 2.2 `components/tags/categories.ts` per D6 with a test that `CATEGORY_ORDER` names every
      `TagCategory` once and general has no class; `domain/tag-input.ts`: `editorText(tags,
      categoryOf)` per D6 and the category prefixes in `METATAG`; tests: the spec's "One line per
      category" text, no tags → empty, one general tag → `tag `, `artist:kan` shuts the popover,
      `art:` too. Verify: `pnpm --filter @boorubox/app test tag-input categories` pass.
- [x] 2.3 `components/tags/TagVocabularyMenuItems.svelte` per D9; `Sidebar.svelte` refreshes
      the vocabulary where it refreshes collections; `LibraryScreen.svelte` refreshes it in
      `afterWrite` and after a single-image tag save; `pending-write.ts` gains the `edit` kind
      and its prompt per D8 with tests (add prompt, remove prompt, not destructive, count rule
      unchanged); `LibraryScreen` runs a confirmed `edit` through `bulkUpdateTags` then
      `afterWrite`. `Inspector.svelte`'s `editorText` call passes `vocabulary.categoryOf` (no
      other inspector change here). Verify: `mise run check` green.

## 3. Unit W2a — the inspector (`packages/app`), after unit W1, beside W2b

- [x] 3.1 `Inspector.svelte`: the tag section per D7 (Edit button in the heading row, read mode
      with grouped coloured badges, edit mode with the field, Save, Cancel, Escape; re-seed only
      while closed; successful save closes and releases) and the pinned chips per D8 (one image:
      toggle through `results.saveTags`; selection: tri-state from `selectionTagCounts(ids,
      names)`, click raises the `edit` pending write through a new `onedit` prop the screen
      binds). The badge menu gains `TagVocabularyMenuItems`; the chip's menu offers Unpin and
      the category group. Verify: `mise run check` green.
      Hand check: the panel shows badges and no field; Edit opens the field with the artist on
      the first line and the general tags on the last; Enter saves and closes it; a refused
      `artist:cat` (with `cat` general) keeps the field open with the reason; pin `tagme` from a
      badge's menu — the chip appears at the top, one click adds it (badge appears, sidebar count
      moves), one click removes it; select twelve, click the chip — the dialog names twelve;
      inside the viewer the same section behaves the same.
      Seen by the lead (same run): the panel shows badges and a pencil, no field; after Pin from the
      sidebar row's menu a `tagme` chip appeared above the badges, ringed on an image carrying it.
      The editor's lines, the refused save and the selection chips were not exercised.

## 4. Unit W2b — the sidebar and the popover (`packages/app`), after unit W1, beside W2a

- [x] 4.1 `TagSidebar.svelte`: rows coloured by `CATEGORY_TEXT_CLASS[vocabulary.categoryOf(name)]`
      (the search marking's background classes unchanged), each row wrapped in a `ContextMenu`
      in `CollectionsSection`'s row shape holding `TagVocabularyMenuItems`; `TagInput.svelte`'s
      popover rows take the same class. Verify: `mise run check` green.
      Hand check: an artist row is red, a copyright violet, a character green, a meta amber, a
      general row plain, in both themes; the included marking still shows on a coloured row;
      right-click a row — Pin and the five categories with the current one checked; choose
      Copyright — the row, the badge and the popover all change colour without a search; the
      +/− buttons and the name still work.
      Seen by the lead (same run): right-click on a sidebar row (through the accessibility "show
      menu" action) offered Pin and the five categories with General checked; choosing Copyright
      turned `blue_archive` violet in the sidebar and in the panel's badges at once. Only dark theme
      and only that colour were seen.

## Handoff

Unit R landed on commit `18963d9`'s tree (main, `browse-fixes` unit A's `matching_ids` command
already in). The migration is the real **v7** (`MIGRATIONS.len() == 7`); design D1's "(planned
as v7 by queue position)" sentence is left as written above since v7 is what it turned out to
be — no amendment needed. `mise run check` is green (599 Rust tests, 0 failed; `pnpm -r
typecheck` 0 errors). Nothing under `packages/app/src` was touched.

**Commands the webview unit calls** (all `#[tauri::command]`, async, registered in `lib.rs`):

- `tag_vocabulary() -> Vec<TagEntry>` — the vocabulary's exceptions (every tag that is not
  `(general, unpinned)`), sorted by name.
- `set_tag_category(name: string, category: TagCategory) -> Vec<TagEntry>` — refuses
  `AppError::NotFound` (surfaces to the webview as a plain string) when `name` is not a tag at
  all; answers the vocabulary as it now stands.
- `set_tag_pinned(name: string, pinned: boolean) -> Vec<TagEntry>` — same refusal, same answer
  shape.
- `selection_tag_counts(ids: string[], limit: number, names?: string[]) -> TagCount[]` — the
  existing command, now taking an optional fourth argument. Tauri resolves an absent `names` key
  to `None` on the Rust side (an `Option<T>` command argument the frontend omits is not an
  error), so the bulk dialog's existing call is unchanged. Pass `names` to get counts of exactly
  those tag names over the selection (e.g. every pinned tag) rather than the top `limit` by
  frequency; a name the selection carries zero times is simply absent from the answer, not a
  zero-count entry — treat "not in the list" as 0. `limit` is ignored entirely when `names`
  is given: the filter is already the bound, so every named tag's count comes back regardless
  of what `limit` is passed.
- `update_tags` and `bulk_update_tags` (existing commands, unchanged signatures) now read a
  category prefix in the tag text — `artist:`, `copyright:`, `character:`, `meta:`, `general:`
  (also `art:`, `copy:`, `char:`, `gen:`), case-insensitive — and refuse the whole write with an
  `AppError::BadRequest` string naming the tag and both categories on a conflict (e.g. `"cat" is
  a general tag and cannot become an artist tag; use another name, e.g. cat_(artist)"`). Neither
  command's Rust signature changed; this is new behaviour on text already being sent.

**Wire shapes** (`packages/shared/src/index.ts`, mirrored in `model.rs`):

```ts
export type TagCategory = 'artist' | 'copyright' | 'character' | 'meta' | 'general'
export interface TagEntry {
  name: string
  category: TagCategory
  pinned: boolean
}
```

**What a cold webview agent needs to know:**

- The vocabulary is *the exceptions list*, not a full tag list — hundreds of names in a library
  of thousands of tags. A general, unpinned tag is never in it; look it up as "general" when
  absent, exactly as design D5 describes for the `api/vocabulary.svelte.ts` store's `categoryOf`.
- A prefix never re-categorises an existing tag (D4's deliberate rule) — only `set_tag_category`
  does that, and only from a context menu (D9), never from typed text.
- `read_metatags`/`link_tags`/`Conflict` are Rust-internal (`tags.rs`); the webview never calls
  them directly. The webview's own `domain/tag-input.ts` (task 2.2, unit W1) needs the *same*
  nine prefix tokens (five long forms + four short forms) in its `METATAG` list, kept in step by
  tests per design D6 — copy the list from `tags.rs`'s `CATEGORY_PREFIXES` rather than
  re-deriving it, since a drift here is silent (nothing checks the two against each other).
- `TagCategory`'s five variants serialise as their lower-case name in both directions (Rust
  `serde` and the TS union agree byte-for-byte); `CATEGORY_ORDER` in unit W1's
  `components/tags/categories.ts` (task 2.2) should list them `artist, copyright, character,
  meta, general` per design D6 — the wire values already sort that way if you want to reuse them,
  but design D6 fixes the order explicitly, so don't infer it from enum declaration order.

Owner decision pending: tag names are case-sensitive (`tags.name` carries no case-folding), so
`Cat` as a general tag and `cat` as an artist tag can coexist as two separate rows. Whether that
split should be allowed to stand or the vocabulary should fold case is not decided; left as is.

### Unit W1

Landed on unit R's tree. `mise run check`: lint 0 errors (2 pre-existing/unrelated warnings, see
below), typecheck 0 errors, `pnpm test` 605+585+94+1 passing, clippy clean, `pnpm build` green.
`cargo fmt --check` currently fails on `tags.rs` — that file is mid-edit by the concurrent Opus
review pass (not unit W1's; no Rust file was touched here), so this is not this unit's gate to
close.

**The vocabulary store** (`api/vocabulary.svelte.ts`, exported as `vocabulary` from `api/index.ts`,
class also exported as `Vocabulary` for a fresh instance in tests):

```ts
class Vocabulary {
  entries: TagEntry[]              // $state, the exceptions list as Rust answers
  error: string | null             // $state, null while in step
  pinned: string[]                 // $derived, sorted by name
  categoryOf(name: string): TagCategory   // 'general' when name is outside entries
  isPinned(name: string): boolean         // false when name is outside entries
  refresh(): Promise<void>
  setCategory(name: string, category: TagCategory): Promise<void>  // replaces entries with the answer
  setPinned(name: string, pinned: boolean): Promise<void>          // replaces entries with the answer
}
```

`vocabulary.refresh()` is already wired at every point design D5 names except the pinned chip's
own write, which does not exist yet: `Sidebar.svelte`'s library-path effect (beside
`collections.refresh()`), `LibraryScreen.afterWrite()` (every bulk writer that ends up there), and
`LibraryScreen`'s `onrelease` handler passed to `<Inspector>` — that handler now runs
`vocabulary.refresh()` before `grid?.refocus()`, and `onrelease` already fires after every
successful panel write (`Inspector.svelte`'s `write()`, `saveFacts()`, `collectionsWritten()`,
`RatingControl`'s `onchosen`). **W2a's pinned-chip write over one image (`results.saveTags`) is
therefore already covered by this hook and needs no refresh call of its own** — only the
*selection* branch (`bulkUpdateTags` through the `edit` pending write, below) needs one, and it
gets it from `afterWrite()`.

**`components/tags/categories.ts`**: `CATEGORY_ORDER: TagCategory[]` (`['artist', 'copyright',
'character', 'meta', 'general']`), `categoryLabel(category: TagCategory): string` (capitalises the
name — there is no separate label table to drift from the five wire values), `CATEGORY_TEXT_CLASS:
Record<TagCategory, string>` (general is `''`).

**`TagVocabularyMenuItems.svelte`**: props `{ name: string }` only. Renders `ContextMenu.Item`
(Pin/Unpin, from `vocabulary.isPinned`), a `ContextMenu.Separator`, then a `ContextMenu.Group` +
`ContextMenu.GroupHeading` ("Category") holding five `ContextMenu.CheckboxItem`s in
`CATEGORY_ORDER`, checked against `vocabulary.categoryOf`. It renders `ContextMenu.*` primitives
directly (not snippets, unlike `CollectionMenuItems`) because every mount point is a
`ContextMenu.Root`; **it must be mounted inside a `ContextMenu.Group`-capable tree**
(`ContextMenu.Content`), same constraint `ImageCard.svelte`'s rating group notes. It never takes a
"suppress Pin" flag: a pinned chip's tag is pinned by construction, so `isPinned` already reads
true there and only Unpin renders.

**`domain/tag-input.ts`**: `editorText`'s signature is now `editorText(tags: string[], categoryOf:
(name: string) => TagCategory): string` — every caller must pass a `categoryOf` function (pass
`vocabulary.categoryOf`). `METATAG` also shuts the popover behind the nine category prefixes now.

**`pending-write.ts`**: `PendingWrite` gained `{ kind: 'edit', ids: string[], add: string[], remove:
string[] }`; `confirmPrompt` and `confirmedCount` handle it (one of `add`/`remove` is non-empty per
activation — the prompt names whichever one is). `LibraryScreen.svelte` already runs a confirmed
`edit` through `bulkUpdateTags` then `afterWrite()`.

**The `onedit` prop — not yet wired, W2a's to finish**: `LibraryScreen.svelte` has
`editSelectionTags(ids: string[], add: string[], remove: string[]): void` (private to the
component, mirrors `rateSelection`'s "one or many" rule: two or more ids raise `pendingWrite = {
kind: 'edit', ids, add, remove }`, one writes straight through `bulkUpdateTags` + `afterWrite`). It
is **not** passed to `<Inspector>` yet, because `Inspector.svelte`'s `Props` has no `onedit` field
to receive it — adding the prop to the call site first would fail typecheck against the interface
W2a is about to write. It currently shows as an unused-function ESLint *warning* (not an error;
does not fail the gate). **W2a must, as part of task 3.1: (a) add `onedit?: (ids: string[], add:
string[], remove: string[]) => void` to `Inspector.svelte`'s `Props`, called from the pinned chip's
selection branch; (b) add `onedit={editSelectionTags}` to the `<Inspector>` call in
`LibraryScreen.svelte`** (the one line task 3.1 needs outside `Inspector.svelte` itself — no other
file of unit W1's is touched).

Hand check: none — no UI was reachable in a browser from unit W1's changes alone (the tag section
still reads plainly through the old `editorText(tags)` call site's single-image path; W2a's read/edit
split is what makes the vocabulary visible on screen).

### Unit W2a

Landed on unit W1's tree. Files touched: `Inspector.svelte` (owned), one line in
`LibraryScreen.svelte`'s `<Inspector>` call site (`onedit={editSelectionTags}`), and a new
`components/library/pinned-state.ts` + `pinned-state.test.ts`. No other file was touched.

`pinned-state.ts` holds the two pure rules design D8 needs, each written once rather than once
per placement: `fillState(tag, counts, total) -> 'all' | 'some' | 'none'` (a name absent from
`counts` reads 0, per `selectionTagCounts`'s own doc comment) and `toggledSelection(state, tag)
-> { add, remove }` for the selection click, `toggledTag(tags, tag) -> string[]` for the
one-image click. Six tests, `pnpm --filter @boorubox/app test pinned-state` passes.

`Inspector.svelte`'s tag section is now read-first (design D7): a heading row with the count and
a pencil Edit button (`startEditTags` seeds `draft` fresh from `editorText(tags,
vocabulary.categoryOf)` on every open, not from the re-seed effect); read mode shows the pinned
chips (if any) then `groupedTags` — `CATEGORY_ORDER.flatMap(sortTags(...))`, a new `$derived`
next to `tags` — as badges whose text takes `CATEGORY_TEXT_CLASS[vocabulary.categoryOf(tag)]`;
the search marking (`terms.included`/`excluded`) now contributes only its background/line-through
classes, per design D6's own sentence quoting exactly those two classes as "the marking" —
text colour is the category's, layered on top, rather than the two fighting over the same
property. Edit mode swaps the badges for `TagInput` (unchanged, still `multiline`) with Save and
Cancel under it; `TagInput`'s existing `onescape` prop (not a new one — it was already there,
"the map's way out of a field") drives Cancel on Escape. The re-seed `$effect` is gated on
`!editingTags`, per the brief; note its practical effect is now small, since `startEditTags`
already reseeds fresh on every open regardless of `shown` — it is kept because draft's only other
reader is inside the edit branch, and gating it costs nothing.

Pinned chips share one `{#snippet pinnedChip(tag, state, onactivate)}` (defined once, at the
component's top level so both placements below can `{@render}` it) — `state` is the same
`'all' | 'some' | 'none'` for one image (`'some'` never reached there) and for a selection.
The selection's tri-state is read from a `$effect` that calls `selection.ids()` synchronously
(tracking `Selection`'s own `$state` through its methods, the way `preview` above it already
does, rather than depending on the `Selection` object itself, which never changes identity) and
also reads `results.generation` purely as a dependency, so a bulk edit's own refresh
(`LibraryScreen.afterWrite`) triggers a refetch of the same tag's count rather than leaving the
chip's fill stale after the write it just caused. The one-image chip writes through the existing
`write()` (closes `editingTags` and fires `onrelease` on success, same as Save); the selection
chip calls the new `onedit` prop, wired to `LibraryScreen`'s existing `editSelectionTags` at the
one call-site line — that function already existed from unit W1, unused until this line.

Deviation: the brief allowed a `pinnable={false}`-style prop on `TagVocabularyMenuItems` if
needed; not needed. A pinned chip's tag reads `vocabulary.isPinned` true by construction, so the
component (unmodified) already renders only "Unpin", exactly as its own doc comment says. No
change to that file.

Deviation: pinned chips are read-mode only (hidden while `editingTags` is true), not shown
alongside the field. Design D7's sentence lists chips and the badge list together as what read
mode shows, then says edit mode "swaps the badge list" — it does not say chips stay. A chip write
mid-edit would call `results.saveTags` with the *stored* tag set while the open draft holds
unsaved edits, silently discarding whichever one lost the race; hiding chips during edit avoids
that class of bug. Not covered by the spec's own scenarios either way.

Gate: `pnpm lint` 0 errors (1 pre-existing warning in `CollectionsSection.svelte`, W2b's file,
untouched here), `pnpm -r typecheck` 0 errors, `pnpm -r test` 599+94+1 passing (up from before by
this unit's 6 new `pinned-state` tests). `mise run check`'s Rust step fails to compile
(`cargo test`: `bulk_update_tags` missing from `tags.rs`, `tags::stamp` missing, a `spec` helper
missing in a `tags.rs` test) — every failing site is in `commands.rs`, `facts.rs`, `rules.rs` and
`tags.rs`, none of them touched by this unit; `git status` at the time showed `collections.rs`,
`db.rs`, `model.rs` and `tags.rs` modified by the concurrent Unit R review pass. Not this unit's
gate to close, per the brief.

Hand check: unticked, as instructed — the twelve-image dialog wording, the badge/chip colours in
both themes, and the pin-from-menu round trip all want a running app.

For W2b or a reviewer: checked `TagSidebar.svelte` and `TagInput.svelte`'s popover after landing
this — both already split the search marking to a background-only class plus
`CATEGORY_TEXT_CLASS`, the same formula the inspector's badges now use, so all three read the
same way. No action needed; noted here since the two units touched this in parallel and nothing
outside the tests pins the formula against drifting apart.

### Webview review pass (Opus reviewer findings on `1543254`)

Seven findings applied, webview files only (the concurrent Rust review pass owns `tags.rs`,
`collections.rs`, `db.rs`, `model.rs`, `sidecar.rs`, `recover.rs`, `commands.rs`, `lib.rs`,
`stamps.rs`, `packages/shared` and was not touched here):

- The lightbox never refreshed the vocabulary on a tag save (Lightbox's own `onrelease` only
  refocuses the surface). Moved the refresh into `results.saveTags` itself
  (`api/search.svelte.ts`) — every placement that saves tags goes through this one write, so it
  is now the single hook, and `LibraryScreen`'s `onrelease` no longer refreshes on every rating
  click.
- `vocabulary.setCategory`/`setPinned` now catch and report through `error`, the same shape
  `refresh()` already used, instead of an unhandled rejection; `TagVocabularyMenuItems`'s
  `onCheckedChange` no-ops on the currently-checked category, so it stops rewriting
  `library.json` for nothing. `vocabulary.error` is rendered in the two places `Inspector.svelte`
  already showed an action error line (the single-image tags section, the multi-selection pinned
  chips section) — `TagSidebar.svelte`'s own row menu has no error line of its own to reuse, so a
  refusal there still only reaches the store; a future finding could give the sidebar row one.
- `LibraryScreen.refreshResults()` (capture and import landing) and `RulesSection`'s `run` now
  also refresh the vocabulary — a rule that creates an artist reached neither before.
- `vocabulary.pinned` sorts with `sortTags` (`domain/tag-utils.ts`), not a second
  `localeCompare`.
- `pending-write.ts`'s `edit` prompt titles use curly quotes (`"Add “tag” to…"`), matching
  `CollectionsSection.svelte`'s delete-dialog title, not literal backticks.
- `CATEGORY_ORDER` and `categoryLabel` moved to a new `domain/tag-categories.ts` (test moved to
  `domain/tag-categories.test.ts`); `components/tags/categories.ts` keeps only
  `CATEGORY_TEXT_CLASS`, re-exporting `CATEGORY_ORDER` from the domain module for a UI caller
  that wants both from one place. `domain/tag-input.ts`, `TagVocabularyMenuItems.svelte` and
  `Inspector.svelte` import `CATEGORY_ORDER`/`categoryLabel` from the domain module directly;
  `TagSidebar.svelte` and `TagInput.svelte` only ever imported `CATEGORY_TEXT_CLASS`, which did
  not move, so neither needed a change.
- Nits: `LibraryScreen`'s `editSelectionTags`/`writeEdit` build the `edit` pending write once,
  typed `Extract<PendingWrite, { kind: 'edit' }>`, instead of the literal twice; `Sidebar.svelte`'s
  "copied from the line above" comment now says why (the vocabulary is per library, like the
  collections above it, so it re-reads on the same path change); `tag-input.test.ts`'s
  "one line per category" test now includes a character tag (`tashkent`), so all five
  `CATEGORY_ORDER` lines are exercised, not four of five.

Gate: `pnpm lint` 0 errors (the same pre-existing `CollectionsSection.svelte` warning noted
above, untouched here), `pnpm -r typecheck` 0 errors, `pnpm -r test` 596 (app) + 94 (extension) +
1 (shared) passing. Rust was not built here — the concurrent Rust review pass may leave it
mid-edit, which is not this pass's gate to close.

Hand check: unticked — a rating click no longer round-tripping the vocabulary, the lightbox's
tag save updating a badge's colour, and a refused category pick showing its reason in the
inspector all want a running app.

### Pinned-chip review pass (Opus reviewer findings on `dc35d00`)

Six findings applied, `Inspector.svelte` and `api/selection.svelte.ts` (+test) only — no other
webview file touched, and no Rust file read or written.

- The pinned-chip effect called `selection.ids()` inside its own tracking scope: `ids()`
  promotes a range to id mode as it resolves, which reassigns the `#state` the effect depends on
  and reruns it mid-flight (double `selectionTagCounts` fetch per gesture), and resolves a plain
  select-all's ids at selection time — what `selection-and-bulk` D3 restricts to "only when an
  action needs ids". Added `Selection.peekIds()`: `ids()`'s body minus the state writes, doc
  comment cross-referencing which of the two writes state. The effect now calls `peekIds()`;
  `toggleSelectionPinned` (a real action) still calls `ids()`. Two new tests: a peeked range
  resolves again on a second peek (not promoted), and a peeked id selection returns its ids with
  no resolver call. Design D8 amended with the honest remaining cost and a `FIXME` in the
  effect's doc comment naming the Rust shape that would remove it.
- A filled chip was `variant="default"` (`bg-primary`), near-white in dark mode, under
  `CATEGORY_TEXT_CLASS`'s coloured text. Every chip now draws `secondary`; `all` adds
  `ring-1 ring-foreground/40`, `some` the same ring dashed — the fill state is a ring, never the
  background, so the category colour stays the text's alone. Design D8 amended.
- `pinnedCounts` could hold the previous selection's answer while a new fetch was in flight, and
  a click during that window could send `remove` over images that never carried the tag. Added
  `pinnedCountsKnown` (`$state(false)` at every point `pinnedCounts` is cleared): while false the
  chips read `none` (via `fillState`'s own empty-counts fallback) and `toggleSelectionPinned`
  no-ops.
- `vocabulary.pinned` is a `$derived` array with a new identity every refresh, so the effect
  memoises on `names.join('\n')` plus a new `Selection.generation` (bumped by the one place
  `#state` is written, `#setState`) plus `results.generation`, and bails before touching the
  network when none of the three moved since the last run.
- `pinnedChip`'s button gained `aria-pressed` in `'true' | 'mixed' | 'false'`, matching `state`.
- Gate: `pnpm lint` 0 errors (the same pre-existing `CollectionsSection.svelte` warning, untouched
  here), `pnpm -r typecheck` 0 errors, `pnpm -r test` 598 (app, up 2 for the new `peekIds` tests)
  + 94 (extension) + 1 (shared) passing. `mise run check` green end to end: 631 Rust tests passing
  (Rust compiled cleanly this time — no file of this pass touched it), clippy clean, `pnpm build`
  green.

Hand check: unticked — the ring/dashed-ring fill in both themes, a fast re-select-then-click not
misfiring a remove, and select-all not resolving ids until a chip is actually clicked all want a
running app.
