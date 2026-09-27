## Context

- `tags (id, name UNIQUE, category, pinned_group)` (`db.rs` v1, v7, v11); schema is v12 today
  (`MIGRATIONS.len()`, `SCHEMA_V12` is the last entry and the shape to copy).
- `tags::vocabulary()` lists the exceptions (`category != 'general' OR pinned_group > 0`);
  `tags::collect_orphans` deletes a carrier-less row only when `category = 'general' AND
  pinned_group = 0`; `tags::set_category(library, name, category)` updates the row, refuses a
  name with no row (`AppError::NotFound("tag …")`), writes `library.json`, answers
  `vocabulary()`. `set_tag_category` / `set_tag_pinned_group` are its commands.
- `TagEntry { name, category, pinned_group }` (`model.rs`) has hand-written `Serialize`
  (keeps the old `pinned` key for an older reader) and `Deserialize` (reads either key),
  `pinned-tag-groups` design D2. `sidecar::LibraryFile.tags: Option<Vec<TagEntry>>` mirrors the
  vocabulary; `recover::insert_vocabulary` upserts it after the sidecar pass, applying the
  canonically spelled entry last. `sidecar.rs`'s `every_tags_column_but_id_is_represented_on_tag_entry`
  fails the suite the moment `tags` gains a column `TagEntry` does not carry.
- `artists::rename` merges a renamed artist into an existing one by moving links, carrying
  `pinned_group` over when the target has none, then `DELETE FROM tags` on the old row.
- Webview: `vocabulary.svelte.ts` holds `entries` and the lookups (`categoryOf`, `groupOf`),
  arrow properties so they can be handed around detached; setters replace `entries` with the
  command's answer. `TagVocabularyMenuItems.svelte` (`{ name }`) is mounted from three menus:
  the sidebar row (`TagSidebar.svelte` via `FilterRow`'s `menu` snippet), the inspector's
  per-image badge and the pinned chip (`Inspector.svelte`, `pinnedChip` snippet, used by the
  single-image and the selection strip).
- `TagInput.svelte`'s `suggest` prop returns `string[]`: the default `libraryTags` maps
  `tag_suggestions`' `TagCount[]` to names; `BulkTagDialog` passes `selectionTags`. Rows are
  coloured by `vocabulary.categoryOf(tag)`.
- `ui/tooltip` is installed. `ui/sidebar/sidebar-provider.svelte` wraps its whole subtree in
  `Tooltip.Provider delayDuration={0}`, and `routes/+layout.svelte` mounts `Sidebar.Provider`
  around the top bar, the app sidebar and every route while a library is open — the viewer
  and Settings included. bits-ui 2.19's `Tooltip.Root` takes its own `delayDuration`.
- Dialogs opened from a menu are mounted by the host, outside the menu, unconditionally, and
  fed a snapshot (`Inspector.svelte`'s `renamingArtist` → `RenameArtistDialog`), with
  `portalTo = portalTarget(root)` so one opened in the viewer lands in its `<dialog>`.
- The library-wide free text is "Notes" / "Library note" (`components/notes/NotesPanel.svelte`,
  `notes.rs`); no lucide note glyph is used anywhere yet.

## Goals / Non-Goals

**Goals:** the proposal's list, with one note per tag row as the single source every reader
reads.

**Non-Goals:** keeping an older build's rebuild from dropping notes (it ignores the key; see
Risks); a length limit on a note.

## Decisions

**D1. `tags.note TEXT`, nullable; `NULL` is "no note", never an empty string.** Migration
`SCHEMA_V13` is `ALTER TABLE tags ADD COLUMN note TEXT;` — `MIGRATIONS.len()` was 12 before unit
R landed, so `SCHEMA_V13` is the real number, not a placeholder. A column on `tags`
rather than a `tag_notes` table: a note is one value per tag with the tag's own lifetime,
exactly like `category` and `pinned_group`, and every reader already reads that row. No
`CHECK`: the one normaliser (D2) is the only way text reaches the column, and a stored `''`
would be a second spelling of "no note" that every predicate below would have to know about.

**D2. `TagEntry.note: Option<String>` / `note: string | null`, normalised once.**
`model::normalized_note(text: Option<&str>) -> Option<String>` trims both ends and maps an
empty result to `None`; inner newlines stay. `set_note` (D4) and `TagEntry`'s `Deserialize`
both call it, so a hand-edited `"note": "  "` in `library.json` restores as no note. The
hand-written `Serialize` writes `note` always (`null` when none) — the struct grows to five
fields — and the `Raw` in `Deserialize` gains `#[serde(default)] note: Option<String>`: a
`library.json` written before this change has no key and reads `None`. An older build reading
a file this build wrote ignores the unknown key (no `deny_unknown_fields` anywhere on the
library file), so it parses; its own rebuild then drops the notes, the same as it drops any key
it predates. The sidecar drift test passes once `note` is on the wire.

**D3. A note is vocabulary: both predicates widen, the restore merges.** `vocabulary()` reads
`WHERE category != 'general' OR pinned_group > 0 OR note IS NOT NULL` and selects `note`;
`collect_orphans` adds `AND note IS NULL`. So a noted general tag stays in the store's
`entries`, in `library.json` and in completion after its last carrier, for the reason
`collect_orphans`' own doc gives for a categorised tag: it was kept deliberately.
`grep -n pinned_group src-tauri/src` is the checklist of exceptions reads, as it was for
`pinned-tag-groups` D1. `recover::insert_vocabulary` writes `note` and on conflict sets
`note = COALESCE(excluded.note, tags.note)`: with the canonical spelling applied last, its note
wins when both spellings carry one, and a spelling with none never erases the other's.

**D4. `tags::set_note(library, name, note: Option<&str>) -> Result<Vec<TagEntry>>`, the shape
of `set_category`.** Name canonicalised; note through `normalized_note`; one transaction:
`UPDATE tags SET note = ?1 WHERE name = ?2`, `NotFound("tag {name}")` when no row changed; when
the note became `NULL`, `collect_orphans(&[id])` in the same transaction — a general, unpinned
tag nothing carries has just lost the one thing keeping it, and leaving the row would have it
suggest itself at count 0 with nothing to show (the orphan rule's own argument). Then commit,
`sidecar::write_library`, answer `vocabulary()`. **A name with no row is refused, not
created**, as `set_category` refuses: every door to this command is a menu on a tag drawn on
screen, which has a row; creating one from a note would add a tag no image ever carried to
completion. Command `set_tag_note(name: String, note: Option<String>) -> Vec<TagEntry>`
beside `set_tag_category`; wrapper `setTagNote(name, note)` in `api/commands.ts`.

**D5. A merge carries the note like the pin.** `artists::rename`'s merge branch reads the old
row's note and sets it on the target `WHERE note IS NULL`, before deleting the old row — the
`pinned_group` rule beside it, for the same reason: the owner's note on the handle is what the
merge is confirming. A fresh rename (`UPDATE tags SET name …`) keeps it already.

**D6. Completion reads the note from the vocabulary; `TagCount` does not change.** The
owner's brief (2026-09-28) had `TagCount` gain `note` with `suggestions()` joining `tags`, and
`None` from `selection_tag_counts`. Overturned at planning, flagged for the owner's yes:
`TagInput` throws `TagCount` away in `libraryTags` and its `suggest` source is pluggable —
`BulkTagDialog` passes `selectionTags`, which is exactly where the brief's field would be
`None` — so a field on `TagCount` reaches neither source without widening `suggest` to objects
across every caller, and bulk-edit completion would show no notes. Every noted tag is in the
vocabulary (D3), and the row already reads `vocabulary.categoryOf(tag)` for its colour:
`vocabulary.noteOf(tag)` beside it covers both sources with one lookup and no second copy of
the note on the wire. The sidebar's `tag_counts` and `selection_tag_counts` answer as today.

**D7. The store: `noteOf` and `setNote`.** `noteOf = (name: string): string | null =>
this.#byName.get(name)?.note ?? null`, an arrow property like `categoryOf`.
`setNote(name, note: string | null): Promise<boolean>` replaces `entries` with the command's
answer and reports a refusal through `error` as `setCategory` does, and also answers whether
it landed: a menu's fire-and-forget setter has nowhere to show a failure, a dialog does and
must stay open on one (D10). The header comment's "a general, unpinned tag is never in it"
becomes "… with no note".

**D8. One indicator component, glyph-only hover.** `components/tags/TagNoteIndicator.svelte`,
props `{ note: string | null, portalTo?: Element }`, renders nothing for `null`. Otherwise a
`Tooltip.Root delayDuration={400}` (overriding the frame provider's `0`, which suits the
sidebar's icon buttons and not a text that pops over a dense list; no new provider — the
frame's covers every mount point) around `StickyNoteIcon` (`size-3 shrink-0
text-muted-foreground`, `aria-label="Tag note"`). `StickyNote` rather than `NotebookPen`:
the glyph says "there is a note", not "edit"; the pen reads as an action on a row full of
actions. The trigger renders through `child` as a `<span>`: the default `<button>` would nest
inside the badge's and the chip's own `<button>`. `Tooltip.Content` gets `portalProps={{ to:
portalTo }}` and `class="block text-left whitespace-pre-wrap wrap-break-word"` over the
upstream `inline-flex … max-w-xs` (20rem, kept), so a long note wraps as written. The trigger
is the glyph, not the whole name: a tooltip over every name the pointer crosses on its way
down the sidebar would pop constantly, and the glyph is the thing that says there is more.
Taking `note` rather than `name` keeps the component free of the store, which is what the
jsdom test mounts.

**D9. Where the glyph and the text go.** Sidebar rows: `FilterRow` gains `note?: string |
null` (default `null`) and renders `<TagNoteIndicator {note} />` between the name button and
the count; `TagSidebar` passes `note={vocabulary.noteOf(name)}`, the collection list passes
nothing; no `portalTo`, the sidebar is never inside the viewer. Inspector badges: inside the
badge `<button>` after `{tag}`; pinned chips: inside the `<Badge>` after `{label}`, tag chips
only; both pass `{portalTo}`. Completion rows (`TagInput`): the name in a `shrink-0` span, then
`vocabulary.noteOf(tag)` in a `ml-2 min-w-0 truncate text-xs text-muted-foreground` span (a
newline reads as a space in one truncated line). Settings → Artists (`ArtistsSection`): a
`mt-1 text-xs whitespace-pre-wrap wrap-break-word text-muted-foreground` paragraph between the
tag row and the URL list when `vocabulary.noteOf(entry.tag)` is non-null — the vocabulary
store is refreshed by the frame (`frame/Sidebar.svelte`) on library open, so it is live on
Settings too. Tile footers (`ImageCard`): nothing.

**D10. "Edit note…" is in the shared menu; each host owns one dialog.**
`TagVocabularyMenuItems` gains a required prop `oneditnote: (name: string) => void` and the
item "Edit note…" with `StickyNoteIcon`, right after the Danbooru look-up item and before the
separator above Pin — a note is a property of the tag, not of an image, so unlike "Rename
artist…" (`artist-entries` D6) it belongs in the component every tag menu shares. The dialog
cannot live inside the component (menu content unmounts on select), nor once in the frame (a
dialog there portals into `<body>`, under the viewer's top-layer `<dialog>`, and would need to
learn which dialog to portal into — the host already knows). So each host holds `editingNote =
$state<string | null>(null)` (the tag name, snapshotted when the item is chosen) and mounts one
`TagNoteDialog` unconditionally, the `RenameArtistDialog` shape (a bits-ui `Dialog` torn down
while open logs `derived_inert`): `TagSidebar` one, no `portalTo`; `Inspector` one for the
chips of both strips and the badges, with `{portalTo}`, whose `onclose` also calls
`onrelease?.()` as the rename dialog's does. The prop is required so a fourth mount that
forgets it fails typecheck rather than offering a dead item.

**D11. The dialog.** `components/tags/TagNoteDialog.svelte`, props `{ open, name, portalTo?,
onclose }`, the `CollectionNameDialog` shape. On an `open` transition: `text =
vocabulary.noteOf(name) ?? ''`, `error = null`, the textarea (`ui/textarea`, ~5 rows) focused
with the caret at the end. Title "Tag note" — "Note" is the library panel's name; under it the
tag name in `CATEGORY_TEXT_CLASS[vocabulary.categoryOf(name)]`. Cancel closes. Save: when
`text.trim() || null` equals `vocabulary.noteOf(name)`, close without a write; otherwise
`saving = true`, `await vocabulary.setNote(name, text)`, close on `true`, on `false` show
`vocabulary.error` in the dialog's refusal line and stay open with the text. Save is disabled
while saving. Enter in the textarea is a newline; no key binding is added (owner's rule).

## Risks / Trade-offs

- [`set_category` and `place_pinned` leave a carrier-less row stranded when they make it
  general and unpinned] → pre-existing, not this change's; `set_note` collects on clear (D4)
  so a note does not add a third door to the same gap.
- [The glyph is a `size-3` target] → the owner's hover hand check decides; widening the
  trigger to the name is one component edit.
- [An older build's rebuild drops notes] → the same as every key added to `library.json`;
  this build writes them back on its next vocabulary write.
- [Unbounded note length] → a very long note makes a tall tooltip; the owner writes short ones.

## Migration Plan

Schema v13 (D1, real number at apply), additive, automatic on open. `library.json` gains `note`
per vocabulary entry on its next write; files without it read as no notes.
