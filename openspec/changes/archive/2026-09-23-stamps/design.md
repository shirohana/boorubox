## Context

See proposal.md — Why. Assumes `browse-fixes` and `tag-vocabulary` have landed: the screen has
one `afterWrite()` (refresh, prune the selection, clamp the focus); `pending-write.ts` has an
`edit` kind `{ ids, add, remove }` run through `bulkUpdateTags`; `tags::read_metatags` reads
rating and category prefixes and `tags::link_tags` applies the `Refuse`/`Keep` conflict policy;
`library.json` mirrors rules, sites, note, collections and the vocabulary and the rebuild
restores each in turn. What else the code holds:

- `tags::bulk_update_tags(ids, add, remove)` links and unlinks in one transaction and writes
  the sidecars after the commit; `tags::stamp(conn, id, rating)` is the helper that moves
  `updated_at` (and the rating when given), called by every tag writer and by
  `rules::apply_rules_to_image`. `collections::add(library, ids, collection_id)` and `remove`
  each open their own transaction, do not move `updated_at` ("favouriting is not an edit"), and
  answer the written `ImageRecord`s.
- `rules.rs` is the CRUD shape for a library-level list: `list`, `upsert(input)` refusing an
  empty name, `delete`, each rewriting `library.json`; `Rule`/`RuleInput` in `model.rs`;
  `RulesSection` + `RuleForm` + `RulesTable` on `/settings`, the section re-reading on a
  library path change through a `$derived`.
- The click on a tile: `ImageCard` reads the modifiers and calls `onselect(modifiers)`;
  `LibraryGrid` routes to `selection.click`; `tile-click.shouldActivate` decides the second
  click opens the viewer. `LibraryScreen` renders the route's toolbar through `frame.toolbar`
  (search, view controls, import, or the selection toolbar while a selection exists), owns
  `screenKeys` for the window-level bindings, and owns the one `ConfirmDialog`.
- `SearchResults.replaceMany(records)` shows written records in place without re-running the
  search (`tags-and-ratings` D10).
- `keyboard.ts` spells every binding once and lists the map for the settings screen;
  `isTypingTarget` and `isInDialog` are the guards.

## Goals / Non-Goals

**Goals:** one grammar, in the webview, tested as a pure module; one Rust write for every
multi-part edit, so a stamp, the bulk dialog and the pinned chip cannot disagree about what a
transaction contains; the mode changes one decision (what a plain click means) and nothing
else about the grid.

**Non-Goals:** a stamp reachable outside edit mode; reordering; undo; a Rust copy of the
grammar (Rust receives the parsed edit and validates what only it can: that the collections
exist, that a category does not conflict).

## Decisions

### D1. The grammar is one pure module in the webview

`domain/stamp.ts`: `parseStamp(text): { edit: TagEditSpec } | { error: string }` with
`TagEditSpec = { add: string[], remove: string[], addCollections: string[], removeCollections:
string[], rating?: Rating }`. Tokens split on whitespace; `-x` → remove; `collection:slug` /
`-collection:slug` (lower-cased, the slug rule `collections::slug` applies in Rust); `rating:v`
with `v` in the four letters → `rating` (last wins); `-rating:` → error; `is:`, `tagcount:`,
`account:`, `posted:`, `or` (case-insensitive) → error naming the token; everything else,
category prefixes included, → `add` verbatim (Rust's `read_metatags` reads the prefix and a
`rating:` that reaches `add` never does, because the grammar took it). Duplicates dropped; a
token in both `add` and `remove` → error. An empty text → error. The autocomplete's `METATAG`
list is not touched: the bar's field is a `TagInput` whose popover shuts on the same tokens.

*Why the webview and not Rust:* "extraction lives in the extension, policy lives in the app"
(CLAUDE.md) puts the language in the app, and the app's parsers live in `domain/` with vitest;
Rust receives a `TagEditSpec` and never a string, so there is no second reader to drift.
*Why not reuse `parseTagSearch`:* it is the search language, and its comments say why the two
readings must never be merged; a stamp's `-collection:x` means "take out", not "exclude".

### D2. One Rust door: `tags::apply_edit`

`tags::apply_edit(library, ids, edit: &TagEditSpec) -> Result<Vec<ImageRecord>>`: one
transaction over all ids; per id, `mark_updated` (today's `stamp`, renamed so the word is
free for the feature; the rating parameter stays) only when `add`, `remove` or `rating` is
non-empty, then `remove_tags`, then `link_tags(…, Conflict::Refuse)`, then the collection
memberships through new `collections::add_in(tx, ids, collection_id)` / `remove_in(tx, …)`
that `collections::add` / `remove` themselves call inside their own transaction (the existing
body split at the transaction boundary, not copied). Collection slugs are resolved to ids
first; an unknown slug is `BadRequest("no collection named `x`")` before anything is written.
`collect_orphans` over the unlinked ids, commit, sidecars for every id, `library.json` when a
categorised tag was born, then the records. `bulk_update_tags` is deleted: `apply_edit` with
only `add`/`remove` is that call, and the webview's `bulkUpdateTags(ids, add, remove)` becomes
`applyEdit(ids, spec)`; the bulk dialog and the pinned chip's `edit` pending write call it.
`update_tags` (whole-set replace) stays: its semantics are different and it is the editor's.

*Why the records come back:* a single-tile apply in edit mode must show the result without a
search re-run (spec), and `replaceMany` wants the written rows; the collection writes already
answer this way for the same reason.

### D3. Stamps are a table mirrored in `library.json`, in the rules' mould

Migration v8 (`SCHEMA_V8` in `db.rs` — the real `MIGRATIONS.len()` when unit R landed, one
after `tag-vocabulary`'s v7 as the design predicted by queue position, confirmed against the
checked-out tree rather than pinned ahead of it): `stamps (id TEXT PRIMARY KEY, name TEXT NOT
NULL, text TEXT NOT NULL, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL)`, listed by
`created_at, id`.
`Stamp { id, name, text, createdAt, updatedAt }`, `StampInput { id?, name, text }`.
`stamps.rs`: `list`, `upsert` (refuses an empty name or empty text; the text is stored as
typed — the grammar is the webview's, and storing the parsed lists would freeze a stamp the
user meant to keep editing), `delete`; each rewrites `library.json`. `LibraryFile.stamps:
Option<Vec<Stamp>>`, `#[serde(default)]`, format 1, restored verbatim by the rebuild after the
vocabulary. Commands `stamps_list`, `stamps_upsert`, `stamps_delete`; store
`api/stamps.svelte.ts` in `collections.svelte.ts`'s shape (`list`, `refresh()`, refreshed on a
library path change from `Sidebar.svelte` beside the other two).

### D4. Edit mode is one boolean on the screen, and the click is routed there

`LibraryScreen` holds `editMode = $state(false)` and `activeStamp`, cleared (per the amendment
below) when the mode is left. `KEY_EDIT_MODE = 'e'` in `keyboard.ts`, bound in `screenKeys`
(unmodified `e`, not typing, not in a dialog; the list gains `{ where: 'Library', keys: ['E'],
action: 'Enter or leave edit mode' }`), and a `Toggle` with a stamp icon in the toolbar
snippet, `aria-pressed`, present in both the plain and the selection toolbars so the mode can
be left while a selection stands.

*Amended (owner's review, 2026-09-23):* `activeStamp` is no longer its own `$state` — the
owner's review found the active chip hard to tell from the rest, and once clicked there was
no way to cancel. The field the bar shows now IS the active stamp: `stampText = $state('')`
(cleared on leaving the mode, same as before) and `activeStamp = $derived(...)` — `null` while
the text is blank or does not parse, else `{ id?, name, text, edit }` with `id`/`name` taken
from the first saved stamp whose `text` equals the field's, `undefined`/`null` for a one-off.
Activating a saved stamp now means filling the field with its text, not writing a separate
snapshot; cancelling is clearing the field, which `parseStamp`'s "an empty text → error" rule
already turns into `null` with no extra check. `onStamp` and `applyStampToSelection` below read
`activeStamp` exactly as they did before the amendment — only where it comes from changed.

`LibraryGrid` gets a prop `onstamp?: (index: number, id: string) => void`. `ImageCard`'s
click path stays exactly as it is; the routing is in `LibraryGrid`'s `onselect` handler: when
`onstamp` is set and the modifiers are neither `multi` nor `range`, call `onstamp` instead of
`selection.click`. `LibraryScreen` passes `onstamp` only while `editMode && activeStamp`, so
outside the mode (or with no active stamp) nothing about the click changes, and no code path
below the screen knows what a stamp is. `ImageCard`'s `shouldActivate` question is answered
by the same routing: `LibraryGrid` passes `onactivate` as `undefined` while `onstamp` is set,
so the second click applies again rather than opening; Enter and Space still open through the
grid's own keys (spec).

`onstamp(index, id)`: `applyEdit([id], activeStamp.edit)` → `results.replaceMany(records)` →
`selection.focusAt(index)` (the stamped card becomes current so the panel shows the result;
the selection is left as it was, per the selection delta) → `vocabulary.refresh()` when the
edit could have created a tag. A refusal shows in the bar's error line, once.

*Why a prop rather than a mode flag on the grid:* the grid and the tile keep one behaviour;
the screen, which owns the mode, owns the exception. *Why the plain click and not a button on
the tile:* Danbooru's edit mode is "click the post", the owner's model; a per-tile button
would put a target the size of the checkbox where the whole tile could be the target.

### D5. The stamp bar

`library/StampBar.svelte`, mounted by the screen between the toolbar and the grid while
`editMode`. The one-off field is a single-line `TagInput`. The `edit` pending write's fields
widen from `{ add, remove }` to `spec: TagEditSpec`, with a `label` (the stamp's name, or its
text while it is still a one-off with none) that `confirmPrompt` reads when the spec is not a
single tag add/remove: the prompt reads "Apply <name> to N images?" with "There is no undo."
and is not destructive; one image applies without asking, as the chip's write does.
`tag-vocabulary`'s chip builds a spec with only its tag parts.

`stamps/StampsSection.svelte`, `StampForm.svelte`, `StampsTable.svelte` on `/settings` after
the Booru section: the rules trio with a matcher-less form (name, text as a `TagInput` with
the parse error from `parseStamp` shown live, and Rust's refusal shown when it answers). The
form is the same component the bar's dialog mounts.

*Amended (owner's review, 2026-09-23):* the field is the one signal for what is active, so it
leads the bar rather than trailing the saved stamps the way a one-off field used to — the
owner's review found saved stamps pushing the field off the row. `StampBar` takes `bind:text`
and no `onactivate`: first row is the field (`TagInput`, `flex-1 min-w-0`, `h-7`), "Save as
stamp…" and, while `selectionCount > 0`, "Apply to N selected" (disabled while the field is
blank or does not parse); a live parse hint shows under the field, in `text-muted-foreground`,
only while the text is non-empty and does not parse — a hint while typing, not a refusal; the
second row, present only while `stamps.list` is non-empty, is the wrapping chip row, each chip
with its Edit…/Delete… context menu as before, now reading pressed (`variant="default"`,
`aria-pressed`) by `stamp.text === text` rather than by id, and filling the field with its
`text` on click rather than building an `ActiveStamp` itself; then the muted no-undo line, now
reading "Type a stamp, or click a saved one to fill the field; click an image to apply it.
Clear the field to stop. There is no undo — the inverse stamp is the way back."; then the
error line. `StampForm`'s `onsaved` (both the bar's dialog and a chip's Edit…) sets the field
to the saved text, so a save becomes the active stamp without a second click. A thumbnail
under the pointer now shows what a click there would do: an overlay, on hover, naming the
active stamp's text (D4's amendment and the hover overlay below).

`LibraryGrid` gains a prop `stampLabel?: string`, the screen's `activeStamp?.text`, passed
straight through to every `ImageCard` as `stampLabel` (widened from the earlier `stamping:
boolean`, D4's own history). `ImageCard` draws `cursor: cell` while it is set, as before, and
— *amended, owner's review, 2026-09-23, "What a click will do"* — on hover an overlay over
the whole image (`absolute inset-0 bg-orange-400/30 flex items-center justify-center p-2
pointer-events-none`, shown by the tile's own unnamed `group-hover`, the same one the caption
strip answers to) holding a label (`rounded-md bg-background/85 px-2 py-1 text-xs font-medium
text-foreground line-clamp-2 text-center`) reading `Apply {stampLabel}`. The overlay sits
above the image and below the checkbox — a cursor and a wash said a click does *something*,
never what, and the owner's review asked for the answer to be readable before the click, not
only after it.

## Risks / Trade-offs

- [A click meant to focus lands as a stamp] → the mode is loud: the toggle is pressed, the bar
  is on screen with the field holding the active stamp's text, the cursor over tiles is
  `cursor: cell`, and — *amended, owner's review, 2026-09-23* — hovering a tile names the edit
  a click would apply. Leaving the mode is one key.
- [`apply_edit` opens a transaction that `collections::add_in` also needs] → the split at the
  transaction boundary (D2) keeps one body; `add`/`remove` keep their public shape.
- [The `edit` pending write's shape changes after `tag-vocabulary` shipped it] → the two
  changes are in the same run; the widened shape is a superset and the prompt function reads
  `spec` to say "Add", "Remove" or "Apply".
- [A one-off stamp is lost when the mode is left] → intended; "Save as stamp…" is beside it.
- [`E` collides with nothing today] → checked against `keyboard.ts`; `i`, `/`, the arrows and
  the modifier bindings are the only unmodified letters.
