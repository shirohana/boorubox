> Three units in sequence: unit R (Rust + shared), then unit W1 (grammar, stores, the settings
> section, the widened edit kind), then unit W2 (edit mode and the bar). Assumes `browse-fixes`
> and `tag-vocabulary` have landed. Design D1–D5 decide every shape; do not re-decide them.
> Gate for every unit: `mise run check` green. No unit ticks a hand check. The migration's
> number is whatever `MIGRATIONS.len()` is when unit R lands: amend design D3's sentence to it.

## 1. Unit R — the door and the table (`packages/app/src-tauri`, `packages/shared`)

- [x] 1.1 `tags.rs`: rename `stamp` → `mark_updated` everywhere it is called (doc comment kept);
      `TagEditSpec` in `model.rs` + `packages/shared` (camel-case wire test); `apply_edit` per D2
      with `collections::add_in` / `remove_in` split out of `add` / `remove`; `bulk_update_tags`
      deleted and its command with it. Tests: every scenario of `stamps` "A stamp is an edit"
      that Rust can see (tags both ways, moving between collections, rating sets and stays,
      unknown collection refuses with nothing written over fifty ids, category conflict
      refuses), "An apply is one write" (collection-only leaves `updated_at`; a tag part moves
      it), and the former `bulk_update_tags` tests re-pointed at `apply_edit`. Verify:
      `cargo test tags:: collections::` pass, `mise run clippy` clean.
- [x] 1.2 `db.rs`: the stamps migration per D3 with its doc comment; `Stamp`/`StampInput` in
      `model.rs` + shared; `stamps.rs` (`list`, `upsert`, `delete`, tests copied from
      `rules.rs`'s: create, edit by id, empty name refused, empty text refused, delete, order by
      creation); `LibraryFile.stamps` written by `write_library` and restored by the rebuild
      after the vocabulary, with the round-trip, old-file and rebuild tests extended ("Stamps
      come back"). Verify: `cargo test db:: stamps:: sidecar:: recover::` pass.
- [x] 1.3 `commands.rs` + `lib.rs`: `apply_edit(ids, edit)`, `stamps_list`, `stamps_upsert`,
      `stamps_delete`, registered; command tests copied from the rules commands'. Verify:
      `mise run check` green.

## 2. Unit W1 — grammar, stores, settings (`packages/app`), after unit R

- [x] 2.1 `domain/stamp.ts` per D1 and `stamp.test.ts`: `cat animal -dog`; `collection:cute
      -collection:uncategorized`; `rating:g` and `rating:s rating:g` → `g`; `-rating:g` error;
      `cat is:png` error naming `is:png`; `cat or dog` error naming `or`; `artist:kantoku` stays
      in `add`; `cat -cat` error; empty error; `Collection:Cute` lower-cased. Verify:
      `pnpm --filter @boorubox/app test stamp` passes.
- [x] 2.2 `api/commands.ts`: `applyEdit(ids, spec)` replacing `bulkUpdateTags`, `stampsList`,
      `stampsUpsert`, `stampsDelete`, with invoke-shape tests; `api/stamps.svelte.ts` per D3 with
      its test; `Sidebar.svelte` refreshes it beside the other stores; `pending-write.ts`: the
      `edit` kind carries `spec: TagEditSpec` and a `label`, the prompt says Add / Remove / Apply
      per D5 (tests updated); `LibraryScreen` runs a confirmed `edit` through `applyEdit` then
      `afterWrite`; `BulkTagDialog.svelte` and the inspector's pinned chip build a spec. Verify:
      `mise run check` green.
- [x] 2.3 `components/stamps/StampsSection.svelte`, `StampForm.svelte`, `StampsTable.svelte` per
      D5, mounted in `routes/settings/+page.svelte` after the Booru section with a comment in
      the Rules section's voice. Verify: `mise run check` green.
      Hand check: create Cat = `cat animal` — listed; edit the text — the row follows; a text
      `cat or dog` is refused under the field before Rust is asked; delete asks and removes;
      `library.json` lists the stamps.

## 3. Unit W2 — edit mode and the bar (`packages/app`), after unit W1

- [x] 3.1 `keyboard.ts`: `KEY_EDIT_MODE` and the map row; `LibraryScreen.svelte`: `editMode`,
      `activeStamp`, the toolbar `Toggle` in both toolbars, the `e` binding in `screenKeys`,
      `onstamp` per D4 passed to the grid only while a stamp is active; `LibraryGrid.svelte`:
      the `onstamp` prop and the routing (plain click → `onstamp`; `onactivate` withheld while
      `onstamp` is set). Verify: `mise run check` green; `keyboard.test.ts` covers the new key
      as it covers `i`.
- [x] 3.2 `library/StampBar.svelte` per D5 (chips with Edit…/Delete… menus, the one-off field,
      Save as stamp…, Apply to N selected, the no-undo line, the error line), mounted by the
      screen while `editMode`; the tile cursor `cell` while a stamp is active. Verify:
      `mise run check` green.
      Hand check: press `E` — the bar appears with the toggle pressed; activate Cat; click three
      tiles — each shows `cat animal` in the panel at once, the viewer never opens; click the
      current tile again — no viewer; shift-click — a range selects, nothing stamped; select
      twelve, Apply to selection — the dialog names twelve and Cat; type `rating:g -tagme`,
      Enter — active, a click rates and untags; Save as stamp… → Reviewed appears in the bar and
      on /settings; `E` again — the bar is gone and a click focuses; `e` typed into the search
      field is a character.
      Seen by the lead on a scratch copy of test-1 (2026-09-24 early, driven through accessibility, not
      the owner's hands): E showed the bar with the toggle pressed; a one-off `tagme rating:s` needed
      Enter twice (the owner note below) and then a click on a tile rated it S and tagged it `tagme`
      at once, the sidebar counts following, the viewer not opening. Saved stamps, Apply to selection,
      shift-click in the mode and Save as stamp… were not exercised.

## 4. Unit S — the bar reworked after the owner's review (`packages/app`), folded into this change

> Owner's review, 2026-09-23 evening: the active chip is hard to tell from the rest; once a
> chip is clicked there is no way to cancel; saved stamps pushed the field off the row. The
> field becomes the one place the active stamp is read from. Agent B (Sonnet), running beside
> agents A (Rust + `domain/tag-utils.ts`, `domain/stamp.ts`) and C (`Inspector`, `TagSidebar`,
> `TagInput`, `categories.ts`) — do not touch their files. Gate: `mise run check` green. Do not
> commit; do not tick a hand check. This unit is folded into the `stamps` commit, so the
> archived design and spec are amended to the shipped shape, not appended to.

- [x] 4.1 `LibraryScreen.svelte` and `StampBar.svelte`: the active stamp is derived from the
      field's text — the screen holds `stampText = $state('')` (cleared when the mode is
      left) and `activeStamp = $derived(...)`: `null` while the text is blank or does not
      parse, else `{ id?, name, text, edit }` where `id`/`name` are the saved stamp whose
      `text` equals the field's (first by list order) or `undefined`/`null`. The bar takes
      `bind:text` and no `onactivate`; `confirmOneOff` and its Enter path go (Enter in the
      field is left to `TagInput`'s own suggestion confirm; this closes the "Enter twice"
      note under 3.2). A chip's click sets the text to the stamp's text; a chip reads pressed
      (`variant="default"`, `aria-pressed`) while `stamp.text === text`. The parse error shows
      under the field only while the text is non-empty and does not parse, in
      `text-muted-foreground` — it is a live hint while typing, not a refusal. `StampForm`'s
      `onsaved` sets the text to the saved text. The `ActiveStamp` type and its doc comment
      move with the derivation; `onStamp` and `applyToSelection` read `activeStamp` as before.
      Verify: `mise run check` green.
- [x] 4.2 `StampBar.svelte` layout: first row is the field (`flex-1 min-w-0`, `h-7`),
      "Save as stamp…" and, while `selectionCount > 0`, "Apply to N selected"; the second
      row, present only while `stamps.list` is non-empty, is the wrapping chip row, each chip
      with its Edit…/Delete… context menu as before; then the muted no-undo line, now
      reading "Type a stamp, or click a saved one to fill the field; click an image to apply
      it. Clear the field to stop. There is no undo — the inverse stamp is the way back."; then
      the error line. Verify: `mise run check` green.
- [x] 4.3 `LibraryGrid.svelte` and `ImageCard.svelte`: the tile's `stamping: boolean` becomes
      `stampLabel?: string` (the active stamp's `text`; `undefined` outside the mode or with no
      active stamp), passed by the grid from a new prop `stampLabel` the screen sets to
      `activeStamp?.text`. While set: `cursor: cell` as before, and on hover an overlay over
      the whole image — `absolute inset-0 bg-orange-400/30 flex items-center justify-center
      p-2 pointer-events-none` shown by `group-hover` — holding a label `rounded-md
      bg-background/85 px-2 py-1 text-xs font-medium text-foreground line-clamp-2
      text-center` reading `Apply {stampLabel}`. The overlay sits above the image and below
      the checkbox. Verify: `mise run check` green.
- [x] 4.4 Docs, in this archive and the main spec: `design.md` D4 rewritten so `activeStamp`
      is derived from the field's text (the state is the text; the derivation is the
      parse plus the saved-stamp lookup; leaving the mode clears the text) and D5 rewritten
      to the layout of 4.2, the chip behaviour of 4.1 and the hover overlay of 4.3, each with
      an *Amended (owner's review, 2026-09-23)* paragraph that says why the earlier shape
      was replaced (the field as the one signal: a non-empty field means a click applies;
      cancelling is clearing; saved stamps only fill the field). The risk line "the cursor
      over tiles is `cursor: cell`" gains the overlay. `specs/stamps/spec.md` here **and**
      `openspec/specs/stamps/spec.md`, requirement "Edit mode applies the active stamp by a
      click": the bar holds a field first and the saved stamps under it; the active stamp
      is the field's text; activating a saved stamp fills the field with its text; clearing
      the field leaves no stamp active; a thumbnail under the pointer shows what a click
      would apply while a stamp is active. Scenarios "A one-off stamp" (typed, active at
      once, no confirm), new "A saved stamp fills the field", "Cancelling" (clear the field
      → a click focuses as outside the mode), "What a click will do" (hover → the tile shows
      `Apply cat animal`). Verify: `openspec validate --specs` passes.
      Hand check: press `E`; the field is first and full width, chips under it; click Cat —
      the field reads `cat animal` and Cat is filled; hover a tile — orange wash with
      "Apply cat animal"; click — applied; clear the field — Cat is plain, hover shows
      nothing, a click focuses; type `rating:g -tagme` — active without Enter, hover says
      so; Save as stamp… seeds the dialog with the field's text and the new chip reads
      pressed after saving; a refused click's message disappears when the field is retyped;
      Edit… on a chip leaves the field alone; Escape in the field then E leaves the mode.
      Seen by the lead on a scratch copy of test-1 (smoke run 2026-09-24): the field first
      and full width with the Cat chip below; clicking Cat filled the field and the chip; a
      hovered tile showed the orange wash with "Apply cat animal" and a click applied both
      tags; clearing the field un-filled the chip, hover showed nothing and a click only
      focused; typing `tagme` armed it without Enter. Not exercised: Apply to N selected, Save
      as stamp… from the bar, the refusal line, Edit… on a chip, Escape then E.

## Handoff

### Unit R

Landed 1.1–1.3. The real migration is **v8** (`SCHEMA_V8` in `db.rs`, confirmed
`MIGRATIONS.len()` after the append — `tag-vocabulary`'s `SCHEMA_V7` landed in `22322d1`);
design D3's sentence is amended to name it directly rather than "amended at apply".

**1.1** — `tags::stamp` renamed to `tags::mark_updated` at every call site (`tags.rs`,
`rules.rs`'s `apply_rules_to_image`, `facts.rs`'s `update`; doc comments kept, only the
identifier changed). `TagEditSpec` added to `model.rs` (camel-case wire test
`a_tag_edit_spec_crosses_the_wire_in_camel_case`) and `packages/shared/src/index.ts`.
`collections.rs` gained `id_for_slug` (slug → id, `BadRequest("no collection named \`x\`")`
when there is none) and `add_in`/`remove_in`, the transaction bodies of `add`/`remove` split out
at the boundary so `apply_edit` can run them inside its own transaction; `add`/`remove`
themselves are unchanged in shape, just call the split-out bodies. `tags::apply_edit(library,
ids, &TagEditSpec) -> Result<Vec<ImageRecord>>` replaces `bulk_update_tags`: resolves every
collection slug to an id before opening the transaction (so an unknown one refuses with nothing
written), marks `updated_at` only when `add`/`remove`/`rating` is non-empty, then per id
`remove_tags` → `link_tags(Refuse)`, then the collection writes over all ids at once, then
`collect_orphans`, commit, sidecars, `library.json` on a newly categorised tag, and answers with
`ingest::load_records`. The `bulk_update_tags` command and its `lib.rs` registration are gone.
Amended after review — see "Review fixes" below: an out-of-alphabet `rating` now refuses before
the transaction opens; a spec with nothing set in any part answers early with no transaction; a
spec that only moves collections now requires every id exists first.

**1.2** — `SCHEMA_V8`: `stamps (id TEXT PRIMARY KEY, name TEXT NOT NULL, text TEXT NOT NULL,
created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL)`. `Stamp`/`StampInput` in `model.rs` +
shared, camel-case wire test `a_stamp_and_its_input_cross_the_wire_in_camel_case`. New
`stamps.rs`: `list` (by `created_at, rowid` — amended after review, see below), `upsert` (empty
name/text refused after trim, text stored exactly as typed, name trimmed), `delete`; each
rewrites `library.json`. `LibraryFile`
gained `stamps: Option<Vec<Stamp>>` (`#[serde(default)]`, always written `Some`); `write_library`
now calls `stamps::list`. `recover::rebuild` restores stamps verbatim (fresh ids never minted)
right after the vocabulary, inside the same transaction, through a new `insert_stamps`.

**1.3** — Commands `apply_edit(ids, edit)`, `stamps_list()`, `stamps_upsert(input)`,
`stamps_delete(id)` added to `commands.rs` and registered in `lib.rs`'s `generate_handler!`
(replacing `bulk_update_tags`'s registration).

**Deviation**: `RebuildReport` was **not** given a `stamps` count field (unlike `rules`/`sites`)
— nothing in the `stamps` spec or tasks.md asks the rebuild report to count stamps, and the
"Stamps come back" scenario only needs `stamps::list` to answer correctly after a rebuild, which
`rebuild_restores_the_stamps_in_creation_order` (`recover.rs`) checks directly.

**Gate**: `cargo test --manifest-path packages/app/src-tauri/Cargo.toml` — 631 passed, 0 failed,
1 ignored (the pre-existing manual-timing test). `mise run clippy` clean. `cargo fmt
--manifest-path packages/app/src-tauri/Cargo.toml --check` clean. `pnpm -r typecheck` — 0 errors
across `shared`, `app`, `extension`. `mise run check` fails, but only in `packages/app/src`
outside this unit's files: an ESLint max-len warning in `CollectionsSection.svelte` and one
failing vitest (`categories.test.ts`: `categoryLabel is not a function`) — both in the other
agents' in-flight `tag-vocabulary`/webview work, not touched by unit R.

**Handoff to unit W1** — what the webview calls:
- `applyEdit(ids: string[], edit: TagEditSpec): Promise<ImageRecord[]>` — replaces
  `bulkUpdateTags(ids, add, remove)`; now answers with the written rows.
- `TagEditSpec = { add: string[], remove: string[], addCollections: string[],
  removeCollections: string[], rating?: Rating }` — `addCollections`/`removeCollections` are
  collection **slugs**, not ids; an unknown one refuses the whole call.
- `stampsList(): Promise<Stamp[]>`, `stampsUpsert(input: StampInput): Promise<Stamp>`,
  `stampsDelete(id: string): Promise<void>`.
- `Stamp = { id, name, text, createdAt, updatedAt }`; `StampInput = { id?, name, text }` — `text`
  is stored exactly as the webview sent it; validate the grammar client-side before calling
  `stampsUpsert`, since Rust only refuses an empty name/text.

### Unit W1

Landed 2.1–2.3. `pnpm lint`, `pnpm -r typecheck`, `pnpm -r test` (shared 1, extension 94, app
617, all passed) and `mise run check` (cargo 638 passed/0 failed/1 ignored, clippy clean, the
webview build) all green — the other agents' `tag-vocabulary`/Rust review-fix work had already
landed clean by the time this unit ran, so nothing here was blocked on it.

**2.1** — `domain/stamp.ts`: `parseStamp(text): { edit: TagEditSpec } | { error: string }`.
Tokens split on `/\s+/`. Per token, lower-cased for matching only: `or` alone errors; a
search-only metatag (`is:`, `tagcount:`, `account:`, `posted:`, with or without a leading `-`)
errors, naming the token as typed; `-rating:*` errors; `rating:g|s|q|e` sets `rating` (last
token wins, an unrecognised letter errors); `-collection:slug`/`collection:slug` lower-case the
slug into `removeCollections`/`addCollections`; `-tag` (non-collection, non-rating) goes into
`remove`; everything else — a category prefix included — goes into `add`, exact case. Duplicates
within one list are dropped silently; a tag in both `add` and `remove` errors after the loop,
naming the tag; an empty text (or all-whitespace) errors before the loop. Every error reads
`“<token>” is not an edit.` except the empty text (`A stamp needs at least one token.`) and the
add/remove conflict (`“<tag>” is both added and removed.`). Not built on `tagList`/
`parseTagSearch`: `tag-utils.ts`'s own comment on why the two readings must stay apart applies
here too, and the split is one line, not worth importing for.

**2.2** — `commands.ts`: `bulkUpdateTags` deleted, replaced by `applyEdit(ids, edit):
Promise<ImageRecord[]>` (`invoke('apply_edit', { ids, edit })`); `stampsList()`,
`stampsUpsert(input)`, `stampsDelete(id)` added, invoke args `stamps_list`/`{}`,
`stamps_upsert`/`{ input }`, `stamps_delete`/`{ id }` — verified against the Rust command
signatures directly (`commands.rs`), not guessed. `api/stamps.svelte.ts`: `Stamps` class,
`collections.svelte.ts`'s shape (`list`, `error`, `refresh()`), exported as `stamps` from
`api/index.ts`. `Sidebar.svelte`'s library-path effect now also calls `stamps.refresh()`,
beside `collections.refresh()` and `vocabulary.refresh()`.

`pending-write.ts`: `PendingWrite`'s `edit` kind is now `{ kind: 'edit', ids: string[], spec:
TagEditSpec, label: string }`. `confirmPrompt` reads a new private `singleTagEdit(spec)`: when
the spec touches exactly one tag (`add.length === 1` xor `remove.length === 1`, every other
field empty/`undefined`) it prompts "Add/Remove "tag" to N images?" exactly as before; otherwise
it prompts "Apply `<label>` to N images? … There is no undo." (not destructive, `confirmLabel:
'Apply'`). `label` is read only in that second branch.

`LibraryScreen.svelte`: `editSelectionTags(ids, add, remove)` — **the inspector's `onedit` prop
keeps its `(ids, add, remove)` signature unchanged; `Inspector.svelte` itself needed no edit** —
builds `{ add, remove, addCollections: [], removeCollections: [] }` and a `label` of `add[0] ??
remove[0] ?? ''` (unread by `confirmPrompt` for a single-tag edit, which this always is), then
asks or writes exactly as before. `writeEdit` now calls `applyEdit(pending.ids, pending.spec)`
then `afterWrite()` (a full search re-run + selection prune), not `results.replaceMany` — a
selection-wide edit can move an image out of the current search (a tag removed, a collection
left), and only a re-run prunes the selection to what still matches; `applyEdit`'s written rows
are for a future single-tile apply (unit W2's `onstamp`), not this path. `BulkTagDialog.svelte`
builds `{ add: tagList(add), remove: tagList(remove), addCollections: [], removeCollections: []
}` and calls `applyEdit` in place of `bulkUpdateTags`.

**2.3** — `components/stamps/`: `StampForm.svelte` (props `stamp: Stamp | null`, `initialText?:
string`, `onsaved: (stamp: Stamp) => void`, `oncancel: () => void` — `RuleForm`'s shape; W2's bar
mounts this same component for "Save as stamp…", passing `initialText` for the one-off's typed
text) shows `parseStamp(text)`'s error live under the field, via a `$derived`, and disables/blocks
Save while one is present; Rust's own refusal (from `stampsUpsert`) is shown only when there is no
live error, since Rust is never asked while one stands. `StampsTable.svelte` (props `stamps:
Stamp[]`, `onedit`, `onchanged`, `onerror` — `RulesTable`'s shape, no pattern column, no enable
switch, `ConfirmDialog` on delete). `StampsSection.svelte` reads `stamps` (the `api/stamps.svelte`
store) directly rather than fetching its own list — unlike `RulesSection`, which has no such
store — since `Sidebar.svelte` already refreshes it on every library switch. Mounted in
`routes/settings/+page.svelte` right after `<BooruSection />`.

**Hand check left open (2.3's box is unticked, nothing added under it)**: create Cat = `cat
animal`; edit the text; a text `cat or dog` refused under the field before Rust is asked; delete
asks and removes; `library.json` lists the stamps.

**Handoff to unit W2** — what the bar and edit mode import:
- `parseStamp(text): { edit: TagEditSpec } | { error: string }` from `$lib/domain/stamp`. Error
  strings: `` `“<token>” is not an edit.` `` for every grammar refusal except `A stamp needs at
  least one token.` (empty text) and `` `“<tag>” is both added and removed.` `` (a token in both
  lists). Use `'error' in parsed ? parsed.error : null` to read it, same as `StampForm`.
- `stamps` from `$lib/api` (`api/stamps.svelte.ts`): `.list: Stamp[]`, `.error: string | null`,
  `.refresh(): Promise<void>` — already kept current by `Sidebar.svelte`, so the bar only needs
  to read `stamps.list`, never call `refresh()` itself.
- `StampForm` from `$lib/components/stamps/StampForm.svelte`: props `stamp: Stamp | null`,
  `initialText?: string`, `onsaved: (stamp: Stamp) => void`, `oncancel: () => void` — mount it in
  a dialog for "Save as stamp…", passing `initialText` as the bar's one-off text and `stamp={null}`.
- The `edit` pending kind: `{ kind: 'edit', ids: string[], spec: TagEditSpec, label: string }`,
  raised by `LibraryScreen.svelte`'s `editSelectionTags(ids, add, remove)` (the inspector's chip)
  or, for a stamp applied to a selection, build the `PendingWrite` directly with `kind: 'edit'`,
  the stamp's `spec`, and `label` set to the stamp's `name` (or its `text` for an unsaved
  one-off) — `confirmPrompt` reads `label` only when the spec is not a single tag add/remove, so
  a stamp with exactly one part reads as "Add/Remove" instead of "Apply", which is correct and
  needs no special-casing on the caller's side. `needsConfirmation`/`writeEdit` are
  `LibraryScreen`'s own, already wired to this shape — a caller past one image should route
  through the same `pendingWrite = {...}` / `void writeEdit({...})` branch `editSelectionTags`
  does, not duplicate the confirm/write split.
- For the per-tile apply in edit mode (design D2's `onstamp`), call `applyEdit([id],
  activeStamp.edit)` directly and use its returned `ImageRecord[]` with `results.replaceMany` —
  `writeEdit` above is the selection-wide path and intentionally re-runs the whole search, which
  is too slow for a single click.

**Review fixes (unit R)** — none of these change a wire shape; all are internal to
`packages/app/src-tauri`.
- `stamps::list` orders by `created_at, rowid`, not `created_at, id`: `id` is a random uuid, so
  two stamps created in the same millisecond came back in random order roughly a third of the
  time `cargo test stamps::` ran alone. `rowid` is monotonic on insert and `recover::insert_stamps`
  inserts a rebuild's stamps in `list`'s own last order, so the fix survives a rebuild too.
- `apply_edit` refuses a `rating` outside `RATINGS` before opening the transaction, the guard
  `set_rating`/`bulk_set_rating` already had and `apply_edit` was missing.
- `apply_edit` requires every id exists (`ingest::require_records_exist`, new) before the
  collection writes when the spec marks nothing — previously a collection-only spec (no
  `mark_updated` call to catch a missing id) silently no-opped over an unknown id instead of
  refusing.
- `apply_edit` returns the records straight back, before any transaction or sidecar write, when
  the whole spec is empty — previously a no-op spec still rewrote every sidecar and read every
  record twice.
- `ingest::load_records` chunks its four `IN (…)` queries to `query::ID_CHUNK`, the same limit
  `bulk_set_rating`/`selection_tag_counts` already chunk to; this is also what `sidecar::write_for`
  and `collections::add`/`remove` chunk on now, since they call through it. `apply_edit` reads
  `ids` once, through `load_records`, and writes the sidecars from that same read via the new
  `sidecar::write_for_records` rather than a second read through `write_for`.
- `collections::remove`'s doc now points at `add_in` (the rule moved there); the `bulk_update_tags`
  "used to be" sentences in `tags.rs`/`commands.rs` are gone (commit history has that story now);
  "stamp" meaning "mark updated" is renamed to "mark"/"now" everywhere in this unit's files —
  `recording_a_post_stamps_the_row` (`booru/posts.rs`) and a "stamp it at the moment" comment
  (`booru/upload.rs`) are the same smell but outside this unit's owned files, left for whichever
  unit owns `booru/`.
- `db.rs`'s v8 migration test now asserts `stamps`' column names, matching what the v7 test
  already asserts for `tags`' new columns.

### Unit W2

Landed 3.1–3.2. `mise run check` green: `pnpm lint` 0 errors (one pre-existing warning in
`CollectionsSection.svelte`, outside this unit's files), `pnpm -r typecheck` 0 errors, `pnpm
--filter @boorubox/app test` 619 passed (was 617 after unit W1; +2 from `keyboard.test.ts`),
`cargo test` 638 passed/0 failed/1 ignored, clippy clean, both `mise run build` and the Tauri
`--no-bundle` compile step untouched by this unit's files.

**A reviewer should look at, in order:**
1. **The click routing** — `LibraryGrid.svelte`'s `onselect` handler (around its `{#each}`
   over the row): `onstamp` set and the modifiers neither `multi` nor `range` routes to
   `onstamp(index, image.id)` instead of `selection.click`; every other modifier combination —
   including a plain click with no `onstamp` — falls through to `selection.click` unchanged.
   `ImageCard`'s `onactivate` prop is now optional (`onactivate?: () => void`), and the grid
   passes `undefined` for it exactly when `onstamp` is set — both its click and its dblclick
   handlers read `onactivate?.()`, so `ImageCard` itself needed no other change (design D4's
   "the click path here is unchanged").
2. **The toggle placement** — `LibraryScreen.svelte`'s `toolbar` snippet: one `Toggle`,
   `bind:pressed={editMode}`, sits between the two `flex-1` spacers, before the
   `{#if selection.count > 0}<SelectionToolbar>{/if}` block rather than inside it — one render
   site, unconditional on `selection.count`, is what makes it present in both the plain and the
   selection toolbar (design D4's "beside/before that component").
3. **`onStamp`/`applyStampToSelection`** in `LibraryScreen.svelte` (just above `commit`): the
   per-tile path calls `applyEdit([id], ...)` and `results.replaceMany` directly, never
   `afterWrite` — the selection-wide path goes through the existing `pendingWrite`/`writeEdit`
   machinery unchanged, per the unit R handoff's steer.
4. **`StampBar.svelte`** (new file): chips are `Button`s with a derived `variant`/`aria-pressed`
   (`active?.name === stamp.name`), not the `Toggle` primitive — a chip always *activates* its
   stamp on click rather than toggling itself off, so there is no boolean for a real toggle to
   own; the edit-mode button above is a genuine toggle and stays a `Toggle`. `activeStamp` is a
   snapshot (`{ name, text, edit }`) taken at the moment of activation, not a live reference to
   the stored row — editing or deleting a stamp via its chip's Edit…/Delete… never changes what
   is currently active underneath it (design D4's shape has no `id` to look one up by).

**Deviations:**
- **The bar's error line is not the screen's `actionError`.** A new `stampError` field on
  `LibraryScreen` (cleared on leaving edit mode, on activating a new stamp, and at the start of
  every `onStamp` attempt) is threaded to `StampBar` as `error`, per design D4 ("a refusal shows
  in the bar's error line, once") — the existing banner above the grid is what every other
  write failure already uses, and a stamp refusal is supposed to appear beside the chip the user
  is looking at, not scrolled away above a grid the click never left.
- **`StampBar` also carries a second, local `manageError`** for a chip's Edit…/Delete… failure
  (`StampsTable`'s own shape, copied since neither task 3.2 nor design D5 names a prop for it
  and it is not the D4 refusal `error` is for) — shown on the same line as `error` when present,
  `error` taking precedence, since the two cannot fire from the same click.
- **The one-off field's error is not the bar's error line.** `parseStamp`'s refusal on the
  `TagInput`'s confirm shows under the field (`oneOffError`), matching `StampForm`'s own
  placement and task 3.2's explicit split between "the one-off field" and "the error line" as
  separate bullet items.
- **A stale stored stamp is handled defensively, not left to throw.** `StampBar`'s `stampEdit`
  re-parses a saved stamp's `text` with `parseStamp` before building the chip's `ActiveStamp`;
  today every saved stamp already passed `parseStamp` in `StampForm` before it could be
  written, so this only guards a row a future grammar change left behind — such a chip disables
  itself and names the reason in its title rather than activating a spec that was never built.
- **`onStamp`/`applyStampToSelection` are not `unit W2's onstamp`** in `writeEdit`'s doc
  comment any more — the comment named the future unit; now that it has landed, it names the
  function (`applyStampToSelection`) instead, per CLAUDE.md's "a comment is for the next edit".

**Hand check left open (3.2's is unticked, nothing added under it)**: press `E` — the bar
appears with the toggle pressed; activate Cat; click three tiles — each shows `cat animal` in
the panel at once, the viewer never opens; click the current tile again — no viewer;
shift-click — a range selects, nothing stamped; select twelve, Apply to selection — the dialog
names twelve and Cat; type `rating:g -tagme`, Enter — active, a click rates and untags; Save as
stamp… → Reviewed appears in the bar and on /settings; `E` again — the bar is gone and a click
focuses; `e` typed into the search field is a character.

**Review fixes (unit W2)**
- The tile menu opening on an unselected tile (`ImageCard`'s `onOpenChange`, design D8) called
  `onselect({})`, which `LibraryGrid`'s router in edit mode read as a plain click and applied the
  active stamp to a right-click. `ClickModifiers` (`api/selection.svelte.ts`) gains an optional
  `menu?: boolean`; the menu's open sends `onselect({ menu: true })`; the router requires
  `!modifiers.menu` alongside `!multi`/`!range` before it routes to `onstamp`. `selection.click`
  itself reads only `multi`/`range`, so it already ignored the field.
- "Save as stamp…" prefilled `active?.text ?? oneOffText`, discarding text typed into the one-off
  field while a chip was active; now `oneOffText || (active?.text ?? '')` prefers what is
  actually in the field.
- Saving a one-off as a stamp left it inactive with the field still holding the old text;
  `onsaved` now clears `oneOffText` and calls `activateChip` on the saved row, so the save
  becomes the active stamp at once.
- A chip whose saved text no longer parses used `disabled`, which also closed off its context
  menu (the only Edit…/Delete… in the bar) and dropped its `title`. It is `aria-disabled` plus a
  muted class instead; `activateChip` already no-ops on the parse failure, and the reason stays
  in `title`.
- The toggle's `title` now names the key ("Edit mode (E): …"), and a chip's pressed state
  compares `active?.id === stamp.id` rather than by name — a stamp's name is not unique, so a
  rename could otherwise leave the wrong chip (or none) reading pressed. `ActiveStamp` gained an
  optional `id`.

**Owner note — the one-off field needs Enter twice.** `TagInput`'s confirm is a two-step
confirm (`tag-editing` design D13): the first Enter completes the token being typed, the second
submits it. The search field has the same two-step rule already, so this is not new to the bar
— not fixed here, since it would mean changing `TagInput` itself. If it keeps annoying, the
one-line follow-up is a `submitIncomplete` prop on `TagInput` that skips straight to submit.

### Unit S

Landed 4.1–4.4, folded into the shipped shape per the owner's 2026-09-23 evening review.

**4.1** — `LibraryScreen.svelte`: `activeStamp` is no longer its own `$state`. `editMode`
still gates a `$effect` that now clears a new `stampText = $state('')` (and `stampError`) on
leaving the mode; `activeStamp = $derived.by(...)` re-parses `stampText` with `parseStamp` on
every keystroke, returning `null` on a blank or unparseable field (`parseStamp`'s own "an
empty text → error" rule covers blank for free) and otherwise `{ id?, name, text, edit }`,
`id`/`name` taken from the first `stamps.list` row whose `text` matches. The `ActiveStamp` type
and its doc comment moved from `StampBar.svelte` into `LibraryScreen.svelte`, next to the
derivation. `onStamp`/`applyStampToSelection` are untouched — they still just read
`activeStamp`. `StampBar.svelte` takes `bind:text` in place of `active`/`onactivate`;
`oneOffText`/`oneOffError`/`confirmOneOff`/`activateChip` are gone. A chip's `onclick` is now
`() => (text = stamp.text)` unconditionally (even for a stale, unparseable stamp — the live
hint under the field then says why, which is more informative than the old silent no-op) and
its pressed state is `stamp.text === text`, not an id comparison.

**4.2** — `StampBar.svelte` layout, top to bottom: field row (`TagInput` `flex-1 min-w-0 h-7`,
"Save as stamp…", "Apply to N selected" while `selectionCount > 0`) → live parse hint (only
while `text` is non-empty and does not parse) → chip row (only while `stamps.list.length > 0`)
→ the muted no-undo line, reworded → the error line (`error ?? manageError`).

**4.3** — `ImageCard.svelte`'s `stamping?: boolean` prop is now `stampLabel?: string`;
`LibraryGrid.svelte` gained the same-named prop, threaded straight to every tile in place of
the old `stamping={onstamp !== undefined}` derivation. `LibraryScreen.svelte` passes
`stampLabel={activeStamp?.text}` to the grid. `cursor: cell` now keys off
`stampLabel !== undefined`. The hover overlay is the last child inside the tile's `<button>`
(after the caption strip), on the button's own unnamed `group`, so `group-hover` covers it and
it paints above the caption but — being inside the button, which closes before the checkbox's
sibling `<span>` — below the checkbox with no `z-index` needed.

**4.4** — `design.md` D4 gained an *Amended* paragraph replacing the `$state` description with
the derivation; D5 gained one covering the reworked layout, the chip's text-based pressed
state, and a new paragraph for the hover overlay (with its own amendment marker, D4's "What a
click will do"); the risk line now names the overlay. `specs/stamps/spec.md` (both copies,
kept byte-identical below their differing headers) had "Edit mode applies the active stamp by
a click" rewritten for the field-first bar and the field-is-the-signal rule, plus scenarios "A
one-off stamp" (updated: active at once, no confirm step), new "A saved stamp fills the
field", new "Cancelling", and new "What a click will do". `openspec validate --specs` passes
(`spec/stamps` ✓, only the pre-existing INFO-level "very long requirement" notices every spec
in this repo gets).

**Deviations**
- **`StampBar` drops the `active` prop entirely, not just `onactivate`.** Task 4.1 says the bar
  "takes `bind:text` and no `onactivate`," but nothing left in the bar needed a read-only
  `active` either: the chip's pressed state moved to `stamp.text === text` and "Apply to N
  selected"'s `disabled` only needs pass/fail, which the bar now derives itself
  (`canApply = text.trim() !== '' && !('error' in parsed)`) from its own `parseStamp(text)`
  call — the same pure function `LibraryScreen`'s derivation calls, not a second copy of the
  grammar. This also drops a prop `LibraryScreen` would otherwise have had to keep threading
  for no reader.
- **`StampForm`'s `onsaved` sets the field on the chip's Edit… dialog too, not only "Save as
  stamp…."** Task 4.1 names `StampForm`'s `onsaved` without scoping it to one of the two
  dialogs the bar mounts; editing a stamp's text and saving now also sets `text` to the saved
  text (`editing = null; text = saved.text; void stamps.refresh()`), so an edit becomes the
  active stamp the same way a create does — checkable by hovering a tile at once, consistent
  with "the field is the one place the active stamp is read from."
- **Closes Unit W2's "Owner note — the one-off field needs Enter twice."** `confirmOneOff` and
  its Enter-triggered `onsubmit` are gone; the field is bound live, so there is no confirm step
  left to need a second Enter. `TagInput`'s own two-step suggestion-confirm still applies to
  completing a token, unchanged, but nothing above it double-gates activation any more.

**Gate**: `pnpm --filter @boorubox/app typecheck` — 0 errors. `pnpm --filter @boorubox/app
test` — all passing (632 at last run; the count moves between runs as agents A/C land their
own units in the same tree). `pnpm eslint` on this unit's four files — clean. `openspec
validate --specs` — all 29 specs pass, `spec/stamps` included. `mise run check` itself is
**not** green: it fails at the `lint` step on `cargo fmt --check` diffs in
`src-tauri/src/{db,query,tags}.rs` (agent A's files, not touched here) and a pre-existing
`@stylistic/max-len` warning in `CollectionsSection.svelte` (not in this unit's ownership
either, and already noted as pre-existing in unit R's own handoff above) — neither is
reachable from this unit's four files or the docs it touched.

**Hand check (4.4's) is unticked**, as instructed — nothing added under it.
