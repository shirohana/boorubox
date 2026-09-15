> Depends on `app-shell` and `library-sidecars`, archived; lands after `inspector-polish`
> (`onrelease`, `account`). Two agents, serial: **R** owns group 1 (Rust, shared types, the
> command wrapper); **T** owns group 2 (webview). Gate per group is in its tasks; `mise run
> check` for the change.

## 1. The write (agent R)

- [x] 1.1 `packages/app/src-tauri/src/facts.rs` (+ `mod facts` in `lib.rs`): `FactsEdit` in
      `model.rs`, `facts::update` per design D1. Verify: tests — a title and two addresses are
      stored and answered back trimmed; empty strings store `NULL`; `not a url` is refused with
      `BadRequest` and nothing changes; a missing id is refused; `updated_at` moves; the sidecar
      on disk carries the new values after the call; a free-text search for the new title finds
      the image (through `query::search`).
- [x] 1.2 `commands.rs` + `lib.rs` handler list + `packages/shared/src/index.ts` (`FactsEdit`) +
      `api/commands.ts` (`updateFacts`). Verify: a command-level test beside `set_rating`'s;
      `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt`, `pnpm -r typecheck`
      pass. Commit as one unit with 1.1.

## Handoff (agent R, group 1)

- `facts::update` reuses `tags::stamp(&tx, id, None)` for the "missing id refuses first" check
  and the `updated_at` bump, then a second `UPDATE images SET page_title, page_url, image_url`
  in the same transaction — the "reuse stamp… or one statement" choice design D1 left open.
- No `url` crate dependency existed (`Cargo.toml` checked) and none was added; validation is
  `strip_prefix("https://").or_else(|| strip_prefix("http://"))` with a non-empty remainder.
- `FactsEdit` (Rust and shared TS) carries no `#[serde(default)]`/optional TS fields: all three
  keys are always sent together by the one form this change adds, so there is no case where a
  key is legitimately absent — kept `pageTitle: string | null` etc. rather than `?:` to match
  how `ImageRecord` spells nullable fields elsewhere in the same file, and to guarantee
  `JSON.stringify` never drops a `null` key the way it would drop `undefined`.
- Added `update_facts` to the two command-level lists in `commands.rs` that already gather
  `update_tags`/`set_rating` (`an_edit_naming_no_image_is_not_found`), plus one dedicated test
  beside `set_rating`'s covering both the success path and a refused address. Left the
  `the_tag_commands_need_a_library_before_they_answer` list untouched — it is scoped to
  `tags.rs`'s own commands by name, and `facts` is a separate module.
- Gate: `cargo test` 520 passed (up from 512, the 8 new `facts::tests` plus the 1 new
  `commands::tests` case), `cargo clippy --all-targets -- -D warnings` clean, `cargo fmt`
  applied, `pnpm -r typecheck` 0 errors, `pnpm lint` exit 0. Did not run `mise run check`
  (would race agent T's tree) or the app.
- Files touched, all inside this agent's ownership: `packages/app/src-tauri/src/facts.rs` (new),
  `lib.rs`, `model.rs`, `commands.rs`, `packages/shared/src/index.ts`,
  `packages/app/src/lib/api/commands.ts`. Not committed — group 1's tasks are ticked and this
  Handoff is appended per the run's boundaries; the owner or agent T commits.

## 2. The panel (agent T)

- [ ] 2.1 `api/search.svelte.ts`: `saveFacts(id, edit)` per design D2. `Inspector.svelte`: the
      Edit action, the form in the rows, Save/Cancel, Enter/Escape, the failure line, the draft
      discarded on an image change, `onrelease` after a successful save (design D3). Verify:
      typecheck, lint and tests pass.
      Hand check (both placements): on a file import, Edit → type a title and an X post address →
      Save — the rows update, the blue account entry appears, the viewer's title bar and the
      tile's title show the new title; Edit → type `nope` as the image address → Save — refused
      with a reason, the text stays; Cancel discards; Escape inside the viewer's form cancels the
      edit and does not close the viewer; Enter saves; after Save beside the grid the arrows move
      the grid's focus.

## Handoff (agent T, group 2)

- `search.svelte.ts`: `saveFacts(id, edit: FactsEdit): Promise<ImageRecord>` — same shape as
  `saveTags`/`saveRating`, calls `updateFacts` then `replace(record)`. Test added beside
  `saveTags`'/`saveRating`'s in `search.svelte.test.ts` (task 2.1 says "+ its test if you add
  `saveFacts` coverage" — added, since the other two writes have exactly this coverage and a
  third with none would be the odd one out).
- `Inspector.svelte`: state `editingFacts`, `draftTitle`, `draftPageUrl`, `draftImageUrl`,
  `factsSaving`, `factsError`; functions `startEditFacts`, `cancelEditFacts`, `saveFacts`,
  `onFactsKeydown`. A ghost pencil `Button` (`size="icon-xs"`) in the header opens the form;
  hidden while `editingFacts` so a second press cannot silently discard the draft (not in
  design D3's text — a judgment call, named here since D3 does not rule on it either way).
  Kept a separate `$effect` keyed on `image?.id` alone (not `image.id:image.updatedAt`, the
  tag editor's key) to discard the draft only when the *described image* changes, per D3 —
  a facts save's own `replace()` must not close the form it just succeeded in, and no other
  write touches these three fields.
  Enter/Escape are wired per-`Input` via `onFactsKeydown`, matching D3's `preventDefault` +
  `stopPropagation` on Escape.
- No design deviations beyond the hide-while-editing judgment call above.
- Gate: `pnpm -r typecheck` — `packages/shared` and `packages/extension` clean; `packages/app`
  fails with 4 errors, all in `src/lib/components/library/LibraryScreen.svelte` (duplicate
  `rate` identifier/function, lines 225/280/506) — a file owned by the concurrent bulk-confirm
  agent, not touched here; no error in `Inspector.svelte` or `search.svelte.ts`. `pnpm lint`
  exit 0. `pnpm --filter @boorubox/app test` — 43 files, 479 passed (up from 478 before the new
  `saveFacts` test).
- Files touched, all inside this agent's ownership: `packages/app/src/lib/components/library/
  Inspector.svelte`, `packages/app/src/lib/api/search.svelte.ts`,
  `packages/app/src/lib/api/search.svelte.test.ts`. Not committed. Task 2.1 left `[ ]` — it
  carries a `Hand check:` line.

## 3. Change-level verification (owner)

- [ ] 3.1 `mise run check` green; the hand check passes on Windows.
