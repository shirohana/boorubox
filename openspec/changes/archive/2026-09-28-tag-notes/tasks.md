> Two units. Unit R (Rust + `packages/shared` + the TS invoke wrapper), Sonnet; unit U
> (webview), Sonnet, after R has landed and after `tag-row-and-inspector-fixes`,
> `settings-pages` and `selection-by-id` have landed. Retried on Opus if the gate fails; one
> Opus review of the whole change. Design D1–D11 decide every shape; do not re-decide them.
> The migration's number is whatever `MIGRATIONS.len()` is when unit R lands (planned v13; 12
> today): amend design D1's sentence to the real number, never pin the planned one. Gates:
> unit R `mise run check` (the whole gate — it owns Rust); unit U `pnpm lint && pnpm typecheck
> && pnpm test`. No unit commits or ticks a hand check.
>
> The design was written against the code before the three sibling changes. Unit U re-reads,
> as landed, `FilterRow.svelte` (gains a `?` button cluster and a lookup prop),
> `Inspector.svelte` (gains an editor-reset effect), `ArtistsSection.svelte` (moves under
> `routes/settings/artists`, gains a filter box) and `LibraryScreen.svelte` / the selection
> store, and places D9's pieces in the files as they now stand.

## 1. Unit R — the column, the write, the wire (`packages/app/src-tauri`, `packages/shared`, `packages/app/src/lib/api`)

- [x] 1.1 `db.rs`: `SCHEMA_V13` (the real number) per D1 with its doc comment naming
      `tag-notes` design D1, appended to `MIGRATIONS`. Test
      `a_v12_library_migrates_gaining_a_null_tag_note` in the shape of
      `a_v11_library_migrates_gaining_the_artist_urls_table`: a v12 database with a tag row
      migrates, `user_version == MIGRATIONS.len()`, the row's `note` reads `NULL`.
- [x] 1.2 `model.rs`: `TagEntry.note: Option<String>`, `normalized_note` per D2, the
      hand-written `Serialize` (five fields) and `Deserialize` (`Raw.note` defaulted,
      normalised) updated, doc comments saying why. Tests:
      `a_tag_entry_crosses_the_wire_in_camel_case_with_lowercase_categories` extended with
      `note`; `a_tag_entry_without_a_note_key_reads_none`; `a_blank_note_reads_none`;
      `a_serialised_tag_entry_still_satisfies_an_old_readers_required_pinned_field` extended
      so the old reader parses an entry that carries a note. `packages/shared/src/index.ts`:
      `TagEntry.note: string | null` with its doc line; every TS fixture that builds a
      `TagEntry` (`commands.test.ts`, `vocabulary.svelte.test.ts`, others `pnpm typecheck`
      names) gains `note: null`.
- [x] 1.3 `tags.rs`: `vocabulary()` selects `note` and widens its predicate, `collect_orphans`
      adds `AND note IS NULL` (per D3, doc comments updated; `grep -n pinned_group` as the
      checklist of other exceptions reads); `set_note(library, name, note)` per D4. Tests:
      `set_note_round_trips_and_a_blank_note_clears_it`; `set_note_on_a_name_with_no_row_is_refused`;
      `a_noted_general_tag_survives_losing_its_last_carrier`;
      `vocabulary_lists_a_noted_general_unpinned_tag`;
      `clearing_the_note_of_an_uncarried_general_tag_removes_it`;
      `suggestions_offer_a_noted_tag_with_no_carrier`; `set_note_rewrites_library_json`.
- [x] 1.4 `recover.rs` `insert_vocabulary`: writes `note`, conflict rule per D3; doc comment
      updated. `sidecar.rs`: `LibraryFile.tags` doc says the vocabulary includes noted tags.
      Tests: `a_rebuild_restores_tag_notes` (a noted uncarried general tag and a noted artist);
      `a_library_file_without_note_keys_restores_no_notes`; the two-spellings merge keeps the
      canonical spelling's note and never erases one with `null`;
      `every_tags_column_but_id_is_represented_on_tag_entry` green unchanged.
- [x] 1.5 `artists.rs` `rename` merge branch carries the note per D5. Test
      `a_merge_carries_the_note_when_the_target_has_none_and_keeps_the_targets_own`.
- [x] 1.6 `commands.rs` + `lib.rs`: `set_tag_note(name, note)` per D4, registered; test
      `set_tag_note_reaches_the_open_library_through_the_command` in the shape of
      `set_tag_category_and_set_tag_pinned_group_reach_the_open_library_through_the_commands`.
      `packages/app/src/lib/api/commands.ts`: `setTagNote(name, note)`; `commands.test.ts`:
      its invoke test.
- [x] 1.7 Gate `mise run check` green. Handoff: the real migration number, the refusal
      wording for a name with no row, and any deviation.

## 2. Unit U — the store, the glyph, the menu, the dialog (`packages/app/src`)

- [x] 2.1 `api/vocabulary.svelte.ts`: `noteOf` and `setNote` per D7, header comment updated.
      `vocabulary.svelte.test.ts`: `noteOf reads the note and null for a tag without one`;
      `setNote calls the command and replaces the list with its answer, answering true`;
      `setNote reports a refusal, answers false, and leaves entries as they were`.
      Done ahead of schedule by unit R — see Handoff.
- [x] 2.2 `components/tags/TagNoteIndicator.svelte` per D8. Test
      `TagNoteIndicator.test.ts` (`// @vitest-environment jsdom`, Svelte's `mount`): renders
      no element for `note: null`, renders the glyph for a note. It is the package's first
      component mount: if Svelte resolves to its server build under vitest, add the browser
      resolve condition for vitest only in `vite.config.ts` and say so in the handoff.
- [x] 2.3 `components/tags/TagNoteDialog.svelte` per D11, the `CollectionNameDialog` shape.
      No component test (the package's dialogs have none); the gate plus hand check 2.7.
- [x] 2.4 `TagVocabularyMenuItems.svelte`: required `oneditnote` prop and the "Edit note…"
      item per D10, header comment names it. `TagSidebar.svelte`: `editingNote` snapshot, one
      `TagNoteDialog` mounted unconditionally, `oneditnote` passed. `Inspector.svelte`: the
      same, one dialog for the chip and badge menus, `{portalTo}`, `onclose` calls
      `onrelease?.()`. Typecheck proves no mount is missing the prop.
- [x] 2.5 The glyph and the text per D9: `FilterRow.svelte` `note` prop; `TagSidebar.svelte`
      passes it; `Inspector.svelte` badges and tag chips with `{portalTo}`; `TagInput.svelte`
      completion rows; `ArtistsSection.svelte` note under the tag. `ImageCard.svelte` untouched.
- [x] 2.6 Gate `pnpm lint && pnpm typecheck && pnpm test` green. Handoff: where each piece
      landed in the files as the sibling changes left them, and whether 2.2 needed the
      resolve condition.
- [ ] 2.7 Hand check (owner): notes shown by hover in all three places, and edited from the
      viewer.
      Hand check: give `sky` a note from its sidebar row's context menu ("Edit note…" sits
      right under the Danbooru item); hover the small note glyph after `sky` in the sidebar,
      on its inspector badge and, after pinning it, on its pinned chip — each shows the note
      after a short delay, wrapped, not wider than about 20rem. Open the viewer, right-click a
      tag in its inspector, "Edit note…": the dialog is above the viewer, typing and Save
      work, and the viewer is still open afterwards. Saving unchanged text and Cancel change
      nothing.
      Seen (lead's smoke 2026-09-28): with `1914` standing in for `sky`: "Edit note…" sits right under the Danbooru item (37-tag-context-menu.png, 38-note-dialog.png); hovering the glyph shows the note after a delay, wrapped, about 20rem wide, in the sidebar (40-note-hover-sidebar.png), on the pinned chip (41-note-hover-pinned-chip.png) and on the inspector badge (42-note-hover-inspector-badge.png). In the viewer, right-click `tagme` → "Edit note…": the dialog is above the viewer, Save works and the viewer stays open (43-note-dialog-over-viewer.png, 44-viewer-after-note-save.png); Save unchanged and Cancel left the note as it was (45-viewer-after-cancel.png, checked in the database). Problem: in the inspector's tag list the note glyph wraps onto a second line under the tag name, so that badge is two lines tall (42-note-hover-inspector-badge.png, 44-viewer-after-note-save.png). The sidebar row and the pinned chip keep it inline.
      Seen (lead's re-smoke 2026-09-28, at deab612): in the viewer, right-click the inspector badge `ipsum` → "Edit note…" → Save (re-07-badge-context-menu.png, re-08-note-dialog.png): the badge stays one line with the glyph right after the name (re-09-inspector-badge-with-note.png) and hover shows the note (re-10-inspector-badge-note-hover.png). After pinning, the pinned chip reads pin + `ipsum` + glyph on one line (re-11-pinned-chip-and-sidebar-with-note.png); the sidebar row keeps the glyph inline before the count and hover shows the note (re-12-sidebar-row-with-note.png, re-13-sidebar-note-hover.png).
- [ ] 2.8 Hand check (owner): a noted general tag outlives its images, completes, and
      survives a rebuild.
      Hand check: on a scratch library, give a general unpinned tag carried by one image a
      note, remove the tag from that image; type part of its name in the editor — it is
      suggested with its note in muted text after the name; search for it — the sidebar lists
      it at 0 with the glyph. Settings → Artists shows an artist tag's note between the tag
      and its URLs. Then run "Rebuild library index" in Settings (on the Library page once `settings-pages` lands) and confirm both notes are
      still there.
      Seen (lead's smoke 2026-09-28): gave `been` (one image) a note and removed it from the image. The editor suggests `been` with "Orphan note on been." muted after it (46-editor-suggests-orphan-note.png); a search lists it at 0 with the glyph (47-search-orphan-tag-at-zero.png); Settings → Artists shows iv70311741's note between the tag and its URLs (58-settings-artists-note.png). After Rebuild library index (64-rebuild-confirm.png, 65-rebuild-done.png) both notes are still shown (66-artists-after-rebuild.png, 67-been-after-rebuild.png).

## Handoff

Unit R landed. Gate green: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
`cargo test` (805 passed) in `packages/app/src-tauri`; `pnpm --filter @boorubox/shared test`;
`pnpm --filter @boorubox/app typecheck` (0 errors — it briefly failed on two files under
`selection-by-id`'s "Expected 3 arguments, but got 2" while that agent was mid-edit; a retry a
minute later was clean, not fixed by anything here); `pnpm --filter @boorubox/app exec vitest
run src/lib/api` (277 passed).

- **Migration number: v13.** `MIGRATIONS.len()` was 12 before this landed; `SCHEMA_V13` is now
  the last entry, so the design's planned number and the real one agree — no renumbering
  needed. `db.rs`'s `SCHEMA_V13` doc comment names `tag-notes` design D1.
- **Refusal wording**: `set_note` on a name with no row answers
  `AppError::NotFound(format!("tag {name}"))` — identical shape to `set_category`'s and
  `place_pinned`'s, so `errorText(cause)` in the store renders it the same way.
- **Rust**: `tags::set_note(library: &Library, name: &str, note: Option<&str>) ->
  Result<Vec<TagEntry>>`. One transaction: writes `note` (through
  `model::normalized_note`, so blank clears it), then — only when the note became `NULL` —
  runs `collect_orphans` on that tag's own id (a third door onto it, beside `set_category`
  making a tag general and `place_pinned` unpinning it). `vocabulary()` now also selects
  `note` and widens its predicate with `OR note IS NOT NULL`; `collect_orphans` gained
  `AND note IS NULL`. `recover::insert_vocabulary`'s conflict clause is
  `note = COALESCE(excluded.note, tags.note)`. `artists::rename`'s merge branch carries
  `from`'s note onto `to` only `WHERE note IS NULL` (D5), same shape as the pinned-group
  carry beside it.
- **Command**: `set_tag_note(name: String, note: Option<String>) -> Vec<TagEntry>`, registered
  in `lib.rs` beside `set_tag_category`/`set_tag_pinned_group`.
- **TS wrapper**: `setTagNote(name: string, note: string | null): Promise<TagEntry[]>` in
  `packages/app/src/lib/api/commands.ts`, invoking `set_tag_note`.
- **Store method — task 2.1 ticked, done by unit R.** This run's explicit file ownership gave
  unit R `vocabulary.svelte.ts` and its test (`proposal.md`'s split describes them as unit U's;
  this run's brief overrode that for `packages/app/src/lib/api/**`). Unit U does not need to
  redo task 2.1 — start at 2.2:
  - `noteOf = (name: string): string | null => this.#byName.get(name)?.note ?? null` — an
    arrow property, same shape as `categoryOf`/`groupOf`.
  - `async setNote(name: string, note: string | null): Promise<boolean>` — replaces `entries`
    with the command's answer on success (`true`); on a refusal, sets `error` (read via
    `vocabulary.error`) and answers `false`, leaving `entries` untouched. This is the one
    setter that answers whether it landed (D7): `TagNoteDialog` (task 2.3/2.11) must check the
    return value and stay open with the text on `false`, showing `vocabulary.error`.
- **`TagEntry` (Rust and TS)**: gained `note`/`note: Option<String>` as the fifth field/last
  property. Every literal construction across `src-tauri` and every TS fixture in
  `commands.test.ts`/`vocabulary.svelte.test.ts` now carries it — `pnpm typecheck` named no
  other file building a `TagEntry` literal at the time this landed. If a sibling change adds
  one before Unit U lands, typecheck will name it.
- **Deviations from design**: none in shape; `design.md` D1's sentence was amended in place
  (see above) rather than left saying "when unit R lands" now that it has.
- **Reviewer**: `packages/app/src-tauri/src/tags.rs`'s `set_note` (~line 832) and its six new
  tests; `packages/app/src-tauri/src/recover.rs`'s `insert_vocabulary` conflict clause and its
  note-merge tests; `packages/app/src-tauri/src/artists.rs`'s `rename` note-carry branch
  (~line 441) and its test covering both merge orders.

- Opus review of `66adb59` (unit R; lead's note 2026-09-28): no correctness findings. Fixes to
  apply once `artist-workflow` unit R has landed (it holds `artists.rs`/`model.rs` until
  then): (1) `recover.rs` ~1342 "canonical wins" merge test lists `Tagme` before `tagme`, the
  order the sort already produces — list `tagme` first or add the reversed case; (2) ~1313
  loser-only merge test's note text reads "capitalised, no note" on the spelling that carries
  the note — rename it; (3) `db.rs` ~989 v12→v13 test inserts "kyoto" but never asserts
  `fts_matches` — assert it or drop the insert; (4) `tags.rs` ~1799 and ~1747 repeat the same
  `suggested(...)` assertion — keep it in one; (5) comments narrating the edit: `model.rs`
  ~207 "extended, not re-derived", test doc comments "Extended by `tag-notes` task 1.2" at
  ~1345/~1540, `tags.rs` ~1783 "the third door … Risks entry" — state the rule only; (6)
  `artists.rs` ~356 fresh-rename doc says the note is kept, no test — add a `SELECT note`
  assertion to the fresh-rename test.

Unit U landed (2.2–2.6; 2.7/2.8 are hand checks, left for the owner). Gate green: `pnpm lint`,
`pnpm typecheck` (0 errors), `pnpm test` (shared 1, extension 96, app 775 — includes the new
`TagNoteIndicator.test.ts`'s 2).

- **2.2 needed no resolve-condition change.** `vite.config.ts` already carries it
  (`tag-row-and-inspector-fixes` D7); `FilterRow.svelte.test.ts` and
  `Inspector.svelte.test.ts` already mount components under it, so this was not in fact the
  package's first mount (the task text is stale against the landed siblings, per the brief).
- **Deviation: `TagNoteIndicator.test-harness.svelte`, a new file the task list did not
  name.** `Tooltip.Root` (bits-ui 2.19) reads its provider from context via `Context.get()`,
  which throws when none is an ancestor — true regardless of hover, at construction time. The
  running app is always covered (the frame's `Tooltip.Provider` in
  `ui/sidebar/sidebar-provider.svelte`), but a unit test that mounts `TagNoteIndicator` alone
  has no such ancestor. The harness wraps it in one `Tooltip.Provider`, test-only — it does not
  change the design's "no new provider" decision (D8), which is about the product, not the
  test rig.
- **The callback shape `TagVocabularyMenuItems` now requires:** `oneditnote: (name: string) =>
  void`, fired on select with the tag's own name; the component does not open anything itself
  (its content unmounts on select, so a dialog cannot live inside it).
- **Who owns the dialog snapshot in each host:** `TagSidebar.svelte` and `Inspector.svelte`
  each hold their own `editingNote = $state<string | null>(null)` and mount one
  `TagNoteDialog` unconditionally (the `renamingArtist`/`RenameArtistDialog` shape) —
  `oneditnote={(name) => (editingNote = name)}` at every `TagVocabularyMenuItems` mount in that
  host. Two independent dialogs, two independent pieces of state; neither host reads the
  other's.
- **`portalTo` wiring:** `TagSidebar`'s `TagNoteDialog` passes none — the sidebar is never
  inside the viewer. `Inspector`'s passes `{portalTo}` (its own `portalTarget(root)`, shared
  with every other dialog and menu it mounts) and its `onclose` also calls `onrelease?.()`,
  the same as `RenameArtistDialog`'s.
- **For `artist-workflow`:** the shared menu now has a precedent to follow for "Edit artist…"
  — but note `TagVocabularyMenuItems` has no image in scope (that is exactly why "Rename
  artist…" was kept out of it, per `artist-entries` D6, and stayed on `Inspector.svelte`'s own
  per-image tag-chip menu, not this shared component). An "Edit artist…" item sharing this
  menu's "Edit note…" precedent would need either its own image-bearing prop threaded through
  every mount (sidebar row and pinned chip included, where none exists today) or to stay off
  this component the way rename did. Read `Inspector.svelte`'s tag-chip menu (~line 1043) for
  how "Rename artist…" is mounted today before deciding.
- **Deviations from design:** none in shape. `TagInput.svelte`'s suggestion row is the one
  place D9 changes existing markup rather than adding beside it — `{tag}` alone became two
  spans, both inside the same `Command.Item` (already `flex items-center gap-2`), so no new
  wrapper was needed.
- **Reviewer:** `packages/app/src/lib/components/tags/TagNoteIndicator.svelte` and its test
  harness/test; `TagNoteDialog.svelte`'s `unchanged` comparison and its `onOpenAutoFocus`
  caret placement; `TagVocabularyMenuItems.svelte`'s new required prop and the two callers
  that thread `editingNote` (`TagSidebar.svelte`, `Inspector.svelte`).
