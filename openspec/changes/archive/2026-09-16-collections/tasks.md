> Depends on `library-sidecars`, `selection-and-bulk`, `tags-and-ratings`, archived; lands
> after `inspector-polish` (`activeTerms`, `onrelease`) and `sidebar-layout` (the sidebar
> column). Three agents: **R** owns group 1 (Rust, `packages/shared`, `api/commands.ts`) and
> lands first; then **T1** (group 2: `domain/`, `api/collections.svelte.ts`, `components/tags/`,
> `components/common/`) and **T2** (group 3: `components/library/`) run side by side from R's
> commit, with the store's and the rewriter's names pinned by design D7/D8 so T2 can import
> them before T1 lands — T2's gate runs after both land. Group 4 is the owner's. Gate per
> group is in its tasks; `mise run check` for the change.

## 1. Storage, query and commands (agent R)

- [x] 1.1 `db.rs`: `SCHEMA_V6` per design D1 with the Favorites seed, appended to `MIGRATIONS`;
      amend D1 if the real number differs. Verify: a migration test beside the v4→v5 one —
      a v5 library opens at v6, keeps its rows, and has exactly one collection, `favorites` /
      `Favorites`; a fresh library has the same one; `open` twice seeds once.
- [x] 1.2 `model.rs` + `packages/shared/src/index.ts`: `Collection { id, name, slug, createdAt,
      updatedAt }`, `CollectionCount { id, name, slug, count }`, `ImageRecord.collections:
      string[]`, `ParsedTagSearch.collections` / `excludeCollections`, `TagCounts.collections`,
      `RebuildReport.collections`. Every struct literal in tests compiles. Verify: `cargo test`,
      `pnpm -r typecheck`.
- [x] 1.3 `collections.rs` (+ `mod` in `lib.rs`): `slug`, `list`, `create`, `rename`, `delete`,
      `add`, `remove` per design D2/D3, `add`/`remove` returning the written records (D8).
      Verify: tests — slug rules (`My  Favorites` → `my_favorites`, blank refused); create/
      rename clash on slug names the holder; rename rewrites `library.json` and no sidecar
      (compare bytes before and after); delete removes memberships and rewrites the members'
      sidecars; add is idempotent and never stamps `images.updated_at`; remove; unknown id
      refused.
- [x] 1.4 `ingest.rs` (`load_records` fills `collections`), `sidecar.rs` (`Sidecar.collections`,
      `LibraryFile.collections`, `write_library`), `recover.rs` per design D5. Verify: tests —
      a sidecar round-trips ids; an old sidecar without the field reads as none; a rebuild
      restores three collections and their members by id; a rebuild with no `library.json`
      gives a placeholder named by the id; a rebuild whose file renamed Favorites keeps the
      rename (the seed yields); the sidecar drift guard still passes.
- [x] 1.5 `query.rs`: `push_collections` in `compile`, `collection_counts` in `tag_counts` per
      design D6/D7. Verify: tests — `collection:favorites` selects members; `-collection:`
      excludes and keeps non-members; an unknown slug matches nothing; counts list every
      collection, zero included, over the matched set with the rating clause included.
- [x] 1.6 `commands.rs` + `lib.rs` handler list + `api/commands.ts`: `collection_list`,
      `collection_create(name)`, `collection_rename(id, name)`, `collection_delete(id)`,
      `collection_add(ids, collectionId)`, `collection_remove(ids, collectionId)` with wrappers
      `collectionList` … `collectionRemove`. Verify: one command-level test per command;
      `cargo clippy --all-targets -- -D warnings`, `cargo fmt`, `mise run check` green. Commit as
      one unit with 1.1–1.5.
      Verified: `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt` (all
      green — see Handoff). `mise run check` was **not** run, on the coordinator's explicit
      instruction (it would race T1/T2's concurrently-edited tree); group 4's owner runs it
      once T1 and T2 have landed.

## 2. Query language, store and sidebar (agent T1)

- [x] 2.1 `domain/tag-utils.ts` (+ tests): the `collection:` term in `parseTagSearch`
      (lower-cased), `toggleCollectionInQuery(query, slug)`, `activeTerms` extended with
      `collections` / `excludedCollections`; `domain/tag-input.ts`'s `METATAG` list gains
      `collection` so the editor offers no suggestions inside it. Verify: unit tests for each,
      including `collection:Favorites` parsing as `favorites`.
- [x] 2.2 `api/collections.svelte.ts` (+ test): `collections.list`, `refresh()`, `byId(id)`,
      exported from `api/index.ts` (design D7); `frame/Sidebar.svelte`'s library-switch effect
      refreshes it. Verify: the store test mocks the command; typecheck passes.
      Deferred: the `Sidebar.svelte` edit itself — `sidebar-layout` owns that file
      concurrently (see Handoff below).
- [ ] 2.3 `components/common/CollectionNameDialog.svelte` and
      `components/tags/CollectionsSection.svelte` per design D8, with `ConfirmDialog` for
      delete. Verify: typecheck and lint pass.
      Hand check: the section sits between Rating and Tags; `Favorites` shows 0 on an empty
      library; New collection… → `To upload` → appears; rename it `Queue`; create `queue` →
      refused naming `Queue`; click `Queue` → `collection:queue` in the search, row green;
      delete `Queue` → the dialog names it and its count.

## 3. Tiles, toolbar, inspector, screen (agent T2)

- [x] 3.1 `components/library/CollectionMenuItems.svelte` per design D8 (imports
      `collections` from `$lib/api` and `CollectionNameDialog` from `components/common`, both
      T1's, by the pinned names). Verify: typecheck passes once T1 lands.
- [ ] 3.2 `ImageCard.svelte`: the context menu per design D8; `LibraryGrid.svelte` /
      `LibraryScreen.svelte` pass what it needs (`selection`, the add/remove handlers that
      `replace` the returned records, the FIXME of D9 at the bulk call). Verify: typecheck,
      lint and tests pass.
      Hand check: right-click an unselected tile → it becomes current, the menu lists
      `Favorites` unchecked; add → checked next time, the inspector shows it; select twelve,
      right-click one → add to `Favorites` → all twelve in; right-click a tile outside the
      selection → only that one; the menu's Move to trash works; on the 25k vault the grid
      scrolls as before.
- [ ] 3.3 `SelectionToolbar.svelte`: the Collection dropdown. `Inspector.svelte`: the
      Collections section per design D8, `onrelease` after a write. Verify: typecheck, lint,
      tests pass.
      Hand check: toolbar → Collection → `Queue` opens a submenu of Add selection / Remove
      selection (design D8, amended) → on a selection of three already in it, Remove selection
      → removed from all three; the inspector lists the described image's collections, click
      one → search filters and it turns green, menu → Remove from this collection; in the
      viewer's panel the same.

## 4. Change-level verification (owner)

- [ ] 4.1 `mise run check` green; the hand checks above pass on Windows; on the 25k vault
      add-all to Favorites finishes and the app comes back.

## Handoff (agent R, group 1 done)

**Real schema version:** v6, matching design D1's claim exactly (`MIGRATIONS.len()` was 5
before this change) — no amendment needed.

**What landed:** `packages/app/src-tauri/src/collections.rs` (new — `slug`, `list`, `create`,
`rename`, `delete`, `add`, `remove`, 25 tests), `db.rs` (`SCHEMA_V6`, migration + seed tests),
`model.rs` (`Collection`, `CollectionCount`, the four extended types), `ingest.rs`
(`load_records` fills `ImageRecord.collections`, sorted, via a fourth statement), `sidecar.rs`
(`Sidecar.collections` additive/optional, `LibraryFile.collections` `#[serde(default)]`,
`write_library` calls `collections::list`), `recover.rs` (collections restored from
`library.json` when it parses — seed deleted first; seed left standing otherwise; placeholders
for orphan membership ids; memberships inserted last, after collections exist, from the
`Sidecar`s `insert_sidecars` already parsed — no second file read), `query.rs`
(`push_collections`/`has_any_collection` in `compile`; `Plan::collection_counts` in
`tag_counts`), `commands.rs` + `lib.rs` handler list, `packages/shared/src/index.ts`,
`packages/app/src/lib/api/commands.ts`, and the one line in
`packages/app/src/lib/domain/image-fixture.ts` (`collections: [],`).

**Rust signatures (`collections.rs`):**
```rust
pub fn slug(name: &str) -> String;
pub fn list(conn: &Connection) -> Result<Vec<Collection>>;
pub fn create(library: &Library, name: &str) -> Result<Collection>;
pub fn rename(library: &Library, id: &str, name: &str) -> Result<Collection>;
pub fn delete(library: &Library, id: &str) -> Result<()>;
pub fn add(library: &Library, ids: &[String], collection_id: &str) -> Result<Vec<ImageRecord>>;
pub fn remove(library: &Library, ids: &[String], collection_id: &str) -> Result<Vec<ImageRecord>>;
```
`CollectionCount { id: String, name: String, slug: String, count: i64 }` (`model.rs`).
Commands: `collection_list() -> Vec<Collection>`, `collection_create(name) -> Collection`,
`collection_rename(id, name) -> Collection`, `collection_delete(id) -> ()`,
`collection_add(ids, collection_id) -> Vec<ImageRecord>`,
`collection_remove(ids, collection_id) -> Vec<ImageRecord>` — `collection_add`/`remove` answer
with `ingest::load_records` over exactly the ids passed in, in that order (unknown ids simply
absent from the answer, the same convention `load_records` already has everywhere else); the
caller `replace`s each returned record rather than re-searching (design D8/D10).

TS wrappers in `packages/app/src/lib/api/commands.ts`: `collectionList()`,
`collectionCreate(name)`, `collectionRename(id, name)`, `collectionDelete(id)`,
`collectionAdd(ids, collectionId): Promise<ImageRecord[]>`,
`collectionRemove(ids, collectionId): Promise<ImageRecord[]>` — the IPC key is `collectionId`
(camelCase; Tauri converts to the Rust param `collection_id`).

**Deviations from the design, and why:**
- D5 names only the `Ok` branch of the `library.json` read as removing the seed
  (`"DELETE FROM collections (removes the seed)"`); I read that literally and left the
  migration's own `favorites` seed standing in both the "no `library.json`" and the
  "unreadable `library.json`" branches, rather than deleting it there too. Effect: a rebuild of
  a library with no readable library-level file always ends with at least the one seeded
  collection (matching what a freshly opened library gets), and any sidecar membership pointing
  at `favorites` is never orphaned into a placeholder. Covered by
  `recover::tests::an_unreadable_library_file_is_named_in_the_report` (asserts
  `report.collections == 1`) and `a_rebuild_with_no_library_json_gives_a_placeholder_named_by_the_id`.
- `RebuildReport.collections` is computed once at the end as `SELECT COUNT(*) FROM collections`
  against the rebuilt database, rather than summed from the three sources (seed / file-verbatim
  / placeholders) that touch the table across the function — single source of truth, and it is
  the one number that has to agree with what a query against the rebuilt library would find.
- `image_collections.added_at` has no field of its own in `Sidecar` (a membership carries only
  the collection id) — a rebuilt membership's `added_at` is backfilled from the sidecar's own
  `updated_at`. Nothing in the app reads that column today (design D3's `add`/`remove` write
  it, nothing queries it), so this is a safe approximation, not a lossy one worth a FIXME.
- `collection_add`/`collection_remove`'s `FIXME` for the D9 per-item-lock shape lives at the
  **webview's** bulk call site (T2, task 3.2), not in `commands.rs` — the Rust functions just
  do the accepted-cost thing (D9); T2's doc comment on the call site is where the shortcut is
  actually taken.

**Gate, verbatim:**
- `cargo test` (in `packages/app/src-tauri`): `test result: ok. 560 passed; 0 failed; 0 ignored;
  0 measured; 0 filtered out; finished in ~45s` (three suites: lib, main, doc-tests — all ok).
- `cargo clippy --all-targets -- -D warnings`: clean, no output, exit 0.
- `cargo fmt` / `cargo fmt -- --check`: clean, no diff after the one run that reformatted
  `collections.rs`'s long lines.
- `pnpm -r typecheck` (repo root): `packages/shared` and `packages/extension` pass.
  `packages/app` fails with 10 errors in **4 files, none in my ownership**:
  - `src/lib/domain/tag-utils.ts:47:9` — the parser's default `ParsedTagSearch` object, missing
    `collections`/`excludeCollections`. **Expected to resolve when T1 lands task 2.1** (adds the
    `collection:` term and these two fields) — this is exactly the coordination point tasks.md's
    header calls out.
  - `src/lib/api/commands.test.ts:130:9` (a `RebuildReport` literal), `:190:3` (a
    `ParsedTagSearch` literal), `:299:9` (a `TagCounts` literal) — all missing `collections`
    (and `excludeCollections` for the `ParsedTagSearch` one).
  - `src/lib/api/rebuild.svelte.test.ts:15:3` — a `RebuildReport` literal typed with
    `collections` absent (structurally read as `number | undefined`, not assignable to the
    required `number`).
  - `src/lib/api/search.svelte.test.ts:18:7`, `:254:34`, `:272:17`, `:291:14`, `:293:14` — the
    `noCounts`/inline `TagCounts` literals (`{ tags, ratings }`), missing `collections`.
  None of these four files are in my ownership (only `commands.ts` itself is); I did not edit
  them. `tag-parser.test.ts` and `tag-query.test.ts` (the other two files named in my brief)
  typecheck clean — they only ever build partial `ParsedTagSearch` objects through
  `toMatchObject`/field reads, never a full literal.
- `pnpm lint` (repo root): fails, but only on files under the other agent's ownership
  (`packages/app/src/lib/components/library/Lightbox.svelte`,
  `packages/app/src/lib/components/library/click-intent.ts` — 1 error, 8 warnings, all
  pre-existing style issues unrelated to `collections`). Nothing flagged in any file I touched.
- `mise run check` was not run (would race T1/T2's tree, per the brief).

**For T1:** your task 2.1 is what makes `tag-utils.ts` (and therefore `commands.test.ts:190`)
compile again — nothing else needed from group 1 for that. `api/collections.svelte.ts`'s
`list`/`refresh()`/`byId(id)` can call `collectionList()` from `$lib/api/commands` directly;
`Collection`/`CollectionCount` are exported from `packages/shared`.

**For T2:** `collection_add`/`collection_remove` return `ImageRecord[]` for exactly the ids
passed, in that order, ids with no row silently dropped — `replace` each one; no `results.refresh()`
needed for that path (per D8). The D9 `FIXME` (per-item-lock shape from
`library::backfill_sidecars`, not built) belongs at your bulk call site, not in Rust.

**For the owner (group 4):** the four typecheck breaks above outside group 1's ownership, and
the `pnpm lint` failures in the other agent's files, are pre-existing/expected-to-resolve as
T1 and T2 land — not blockers introduced by group 1's commit. `mise run check` should be run
fresh once all three groups have landed.

## Handoff (agent T1, group 2 done)

**What landed:** `domain/tag-utils.ts` (the `collection:` metatag in `parseTagSearch`,
`toggleCollectionInQuery`, `activeTerms.collections`/`excludedCollections`, and
`rewriteAccounts` renamed/generalised to `rewriteMetatagList` so `account:` and `collection:`
share one rewriter — zero duplication, same shape design D6 already calls out), `domain/tag-input.ts`
(`METATAG` gains `collection`), new `api/collections.svelte.ts` + test, `api/index.ts` (exports
`collections`), new `components/common/CollectionNameDialog.svelte`, new
`components/tags/CollectionsSection.svelte`. Tests added to `tag-parser.test.ts`,
`tag-query.test.ts`, `tag-input.test.ts`.

**Exports T2 imports:**
- `collections` (`$lib/api`): `collections.list: Collection[]`, `collections.error: string | null`,
  `collections.refresh(): Promise<void>`, `collections.byId(id): Collection | undefined`. No
  `create`/`rename`/`delete` on the store — callers use `collectionCreate`/`collectionRename`/
  `collectionDelete` from `$lib/api/commands` directly and then `collections.refresh()`, the
  way `RulesTable` re-reads after a write.
- `toggleCollectionInQuery(query, slug)`, `activeTerms(query).collections` /
  `.excludedCollections` (both `Set<string>`) — `$lib/domain/tag-utils`.
- `CollectionNameDialog` (`$lib/components/common`): props `collection: Pick<Collection, 'id' |
  'name'> | null` (not the full `Collection` — the dialog reads only `id`/`name`, so a
  `CollectionCount` row satisfies it structurally with no cast), `open: boolean`,
  `onclose: () => void`, `onsaved: (collection: Collection) => void`. `collection: null` creates;
  non-null renames.
- `CollectionsSection` (`$lib/components/tags`): props exactly `counts: CollectionCount[] | null`,
  `tagQuery: string`, `onquery: (next: string) => void` — the three the brief pinned. Not yet
  rendered anywhere; mount it between `RatingPills` and `TagSidebar` in `LibraryScreen.svelte`'s
  sidebar snippet, same prop wiring as those two (`counts={results.counts?.collections ?? null}`).

**Deviations, and why:**
- **Sidebar's library-switch effect not touched.** Task 2.2 says that effect should refresh the
  collections store; `frame/Sidebar.svelte` is owned by the concurrent `sidebar-layout` agent
  per this brief's boundaries, so I left it alone. **Someone still needs to add
  `void collections.refresh()` to the `$effect` at `Sidebar.svelte:58-64`** (beside
  `libraryCounts.refresh()`) once that file is free to edit — until then `collections.list` is
  populated only by whatever first calls `collections.refresh()` (T2's screen mount, if it adds
  one) and never follows a library switch.
- **`CollectionsSection` has no `onchanged`/`oncreate` prop**, only the three the brief pinned
  (`counts`, `tagQuery`, `onquery`). Design D8 says rows come from `results.counts.collections`
  and names no refresh path for create/rename/delete (only membership writes get a `replace`,
  per D8's own text). With no dedicated callback available, `CollectionsSection` refreshes
  itself after a create/rename/delete by calling `onquery(tagQuery)` — the same string,
  unchanged — which `LibraryScreen.svelte`'s `searchKeeping` (confirmed by reading it, not
  edited) runs unconditionally regardless of whether the string changed, forcing a fresh
  `results.run` and therefore fresh counts. This is what makes the hand check's "New collection…
  → To upload → appears" hold without a new prop. **Trade-off to flag for the owner/T2:** this
  also resets the selection and refocuses the search the way any other query edit does — a
  rename or delete now has the side effect a tag click has. If that is unwanted, the fix is a
  fourth prop (e.g. `onchanged: () => void`) wired to `results.refresh()` alone; I did not add
  one because the brief pinned the prop list and D8 does not ask for it. Hand check 2.3 is
  unticked either way, per the brief's boundary on hand-check lines.
- **`rewriteAccounts` → `rewriteMetatagList`, widened to `marker: string`.** Grepped first per
  CLAUDE.md: the function was already exactly the shape `collection:`'s rewrite needed (comma
  list, either side of one metatag), so I renamed and generalised it in place rather than
  writing a second copy. `toggleAccountInQuery` calls it unchanged; behaviour is identical for
  `account:`, pinned by the existing `tag-query.test.ts` account suite still passing.

**Gate, verbatim:**
- `pnpm -r typecheck`: `packages/shared` and `packages/extension` pass. `packages/app` fails
  with the same 9 errors R's Handoff named, all outside my ownership (`commands.test.ts`,
  `rebuild.svelte.test.ts`, `search.svelte.test.ts` — missing `collections`/`excludeCollections`
  fields in test fixtures) — `tag-utils.ts:47`, the one error R named as mine to fix, is gone.
  1152 files checked (was 1149), confirming my three new files typecheck clean.
- `pnpm lint`: clean, 0 errors, 0 warnings, repo-wide.
- `pnpm --filter @boorubox/app test`: `Test Files 46 passed (46)`, `Tests 511 passed (511)`.

**For T2:** `CollectionMenuItems.svelte` (3.1) needs `collections.list` (the full `Collection[]`,
for the checkbox items) and `collections.refresh()` after a membership write only insofar as new
collections might have been created elsewhere — membership itself refreshes via `replace`, per
R's handoff, not via this store. Import `CollectionNameDialog` from `components/common` for its
own "New collection…" item, same props as above.

**For the owner (group 4):** two open items from this group — wire `collections.refresh()` into
`Sidebar.svelte`'s switch effect once it is free, and decide whether `CollectionsSection` should
get a dedicated refresh callback instead of the `onquery(tagQuery)` reuse (see Deviations).
Neither blocks typecheck/lint/tests; both are behaviour the hand checks (2.3, and whichever of
T2's covers a library switch) should surface if left unfixed.

## Handoff (agent T2, group 3: 3.1 done, 3.2/3.3 implemented — hand checks unticked)

**What landed:** new `components/library/CollectionMenuItems.svelte` (3.1); `ImageCard.svelte`
(the tile's collection submenu, 3.2); `LibraryGrid.svelte` (`selection`, `onwritten`, `onerror`
plumbed to every tile; `onCollectionsWritten` owns the `results.replace` per record);
`LibraryScreen.svelte` (`CollectionsSection` mounted between `RatingPills` and `TagSidebar`,
`onerror` wired to `actionError` for `LibraryGrid`); `SelectionToolbar.svelte` (the Collection
dropdown, 3.3); `Inspector.svelte` (the Collections section — badges with the tag list's own
marking, per-badge "Remove from this collection", and an "Add to…" dropdown, 3.3);
`components/tags/CollectionsSection.svelte` (the `onchanged` prop, replacing the `onquery(tagQuery)`
reuse T1's Handoff flagged — see Deviations); `components/frame/Sidebar.svelte` (one line,
`collections.refresh()` in the library-switch effect, T1's Handoff item).

**`CollectionMenuItems.svelte`'s shape:** it owns the list, the checkmark, the write
(`collectionAdd`/`collectionRemove`, chosen by whether `memberships` already has the row) and
the "New collection…" dialog — once. It does **not** render `ui/context-menu` or
`ui/dropdown-menu` markup itself: bits-ui's two primitives are different components with
different context providers, so a `ContextMenu.CheckboxItem` cannot sit inside a
`DropdownMenu.Content` or vice versa. Three snippet props (`checkboxItem`, `item`, `separator`)
let each of the three call sites supply its own menu's actual item components while the logic
above stays written once — the alternative (three near-identical copies of the whole component)
would have been the duplication CLAUDE.md rules out.

Props: `resolveIds: () => Promise<string[]>` (async, like `selection.ids()` — resolved at write
time, not render time, so a live range is never read from a stale array); `memberships:
ReadonlySet<string> | null` (`null` only from the toolbar, which knows no single record);
`onwritten: (records: ImageRecord[]) => void`; `onerror: (message: string) => void`.

**The ids rule for a right-click, exactly as implemented:** `ImageCard`'s `ContextMenu.Root`
gets an `onOpenChange` that calls `onselect({})` (a plain click's own reset-and-focus) the
moment the menu opens on a tile that is *not* part of the current selection — this is what makes
the tile visibly current right away, matching "as a left click would" (design D8), independent
of whether the user then picks a collection. `resolveCollectionIds` (called lazily, only when a
checkbox is actually clicked) is simpler than that: `selected ? selection.ids() : [image.id]`.
Because the `onOpenChange` reset already clears the selection for an outside-click before any
write happens, that branch is in practice always `[image.id]` for such a tile by the time it
runs — the two are separate concerns (visible current-ness vs. what gets written) that happen to
agree here, not one computed from the other.

**No second D9 FIXME added.** The coordinator confirmed mid-task that Rust's `collections::add`
already carries the D9 FIXME (commit `2919383`) and that tasks.md's "FIXME belongs at your call
site" sentence (R's Handoff) is stale — the webview's bulk call site (`CollectionMenuItems`'s
`toggle`, `LibraryGrid`'s `onCollectionsWritten`) carries none.

**Deviation — `CollectionMenuItems`' "New collection…" auto-adds the ids.** Design D8 says the
row is offered; it does not say what happens after. Every other row in the same list is "toggle
membership for the ids this menu was opened for" — treating "New collection…" as *only*
creating, with no write, would mean the one thing the user opened this menu to do (put these
images somewhere) takes two trips: create, then reopen the same menu to find and check the new
row. I made `created()` call the same `toggle()` an existing row's click does, right after
`collections.refresh()`, so the write is the same one path either way. `CollectionsSection`'s
own "New collection…" (the sidebar) is unrelated and unchanged — it has no ids in scope, so
nothing to add.

**Deviation — `CollectionsSection`'s `onchanged` prop (fix (a) from the brief).** Replaced the
`onquery(tagQuery)` reuse T1's Handoff flagged (it re-ran the search and reset the selection on
every rename/delete) with a dedicated `onchanged: () => void` prop, wired at `LibraryScreen` to
`() => void results.refresh()` — the same re-read a bulk edit uses, which keeps the selection and
the scroll position. `CollectionsSection.svelte`'s internal `refresh()` helper is gone; `saved()`
and `confirmDelete()` call `onchanged()` directly.

**Deviation — Inspector's Collections section guard.** Per spec ("Absent when the image is in
none and there is nothing to add to"), the whole section is wrapped in
`{#if image.collections.length > 0 || collections.list.length > 0}` rather than always rendered
with an empty state — matching the sentence literally rather than showing a permanently-empty
section on a library with no collections at all.

**Gate, verbatim:**
- `pnpm -r typecheck` (repo root): `packages/shared` and `packages/extension` pass.
  `packages/app`: `COMPLETED 1153 FILES 0 ERRORS 0 WARNINGS 0 FILES_WITH_PROBLEMS` — the four
  test-fixture files R's and T1's Handoffs named as pre-existing/out-of-ownership breaks are
  clean now too (someone else's concurrent commit fixed the fixtures; not touched here).
- `pnpm lint` (repo root): clean, `0 errors, 0 warnings` after fixing five `max-len` warnings of
  my own (wrapped lines in `CollectionMenuItems.svelte`, `ImageCard.svelte`,
  `SelectionToolbar.svelte`) — no other file flagged, including the viewer-review files the
  brief said not to touch.
- `pnpm --filter @boorubox/app test`: `Test Files 46 passed (46)`, `Tests 513 passed (513)`.

**Hand checks left unticked, for the owner (3.2, 3.3):** right-click an unselected tile becomes
current and lists `Favorites` unchecked; add from a 12-image selection puts all twelve in; a
right-click outside the selection acts on that one tile only; Move to trash still works from the
tile menu; the 25k-vault scroll is unaffected (bits-ui only mounts an open `ContextMenu.Content`,
and the grid already windows its tiles, so this should hold, but it wants eyes on the real
vault). Toolbar → Collection → an already-in collection shows checked and removes on click; the
inspector lists an image's collections with the tag list's own marking, click filters, menu
offers Remove; the viewer's own panel (same `Inspector` component) should match.
