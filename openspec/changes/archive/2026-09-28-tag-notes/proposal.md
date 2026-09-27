## Why

A tag's name is often not enough to use it right: whether `sky` means the whole background or
a patch of it, which of two look-alike characters a tag names, what a Danbooru meta tag is for.
Danbooru answers with a wiki page per tag; the owner wants the useful part of that page kept
next to the tag in their own library, and on artist tags a record of which Danbooru artist tag
an auto-derived handle was confirmed as (owner, 2026-09-28). Requirements §6 (Danbooru-style
tagging), §7 (tags as rows; `library.json` carries what a rebuild would otherwise lose).

## What Changes

- **A tag can carry a note**: a short free text, stored on the tag's row, mirrored into
  `library.json` with the rest of the vocabulary and restored by a rebuild. An empty note is no
  note.
- **A noted tag outlives its carriers**, as a categorised or pinned one does: it stays in the
  vocabulary and keeps completing when its last image loses it.
- **Edit where the tag is shown**: the tag context menu (sidebar row, inspector badge, pinned
  chip) gains "Edit note…", opening a "Tag note" dialog with the text prefilled.
- **Shown where the tag is read**: a small muted glyph after the tag name on sidebar rows,
  inspector badges and pinned chips, whose hover shows the note; the editor's suggestion rows
  show the note after the name on one line; Settings → Artists shows an artist tag's note under
  the tag. Tile footers show nothing.
- No new keyboard binding.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `tag-vocabulary`: a tag carries a note, edited from the tag's menu and shown where the tag is
  read; the vocabulary outlives its carriers for a noted tag too.
- `tag-editing`: a suggestion shows its tag's note.
- `library-recovery`: the library-level file lists a noted tag with its note, and a rebuild
  restores it.
- `artist-entries`: the Artists list shows each artist tag's note.

## Non-goals

- Rich text, links or images in a note; it is plain text, shown as written.
- Fetching a note from Danbooru's wiki. The Danbooru look-up item already opens the page; the
  owner copies what is worth keeping.
- A note on a tile's tag footer, or a search over note text.
- Matching the note in the Artists page's filter.
- A keyboard shortcut to edit a note (owner's rule: no new bindings for this).

## Impact

- `packages/app/src-tauri`: schema migration adding `tags.note`; `tags.rs` (`set_note`, the
  orphan and vocabulary predicates), `artists.rs` (a merge carries the note), `recover.rs`,
  `model.rs` (`TagEntry.note`), `commands.rs` + `lib.rs` (`set_tag_note`).
- `packages/shared`: `TagEntry.note`.
- `packages/app/src`: `api/commands.ts`, `api/vocabulary.svelte.ts`; new
  `components/tags/TagNoteIndicator.svelte`, `TagNoteDialog.svelte`; `TagVocabularyMenuItems`,
  `FilterRow`, `TagSidebar`, `Inspector`, `TagInput`, `ArtistsSection`.
- `library.json` gains a `note` key per vocabulary entry; older builds ignore it.
