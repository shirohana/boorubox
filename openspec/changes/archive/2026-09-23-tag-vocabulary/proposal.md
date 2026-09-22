## Why

Every tag in the library is the same kind of tag, so a panel of thirty tags reads as one grey
list and the artist, the series and the character are found by reading every entry. Danbooru
tells them apart by category and colour, and the owner tags for Danbooru (requirements §6:
booru integration is two-way; backlog, owner's story 2026-09-15: categories so galleries import
and upload cleanly). The same panel's most frequent edit — putting `tagme` on an image or
taking it off — costs a click into the editor, typing, and a save; a tag the user keeps
reaching for should be one click (owner, 2026-09-23). Requirements §6 (tag search, sidebars,
bulk ops).

## What Changes

- **Every tag has a category**: artist, copyright, character, meta or general, general by
  default. A category belongs to the tag, not to one image's use of it, and a tag name is
  unique across categories: `cat` cannot be both the animal and an artist, so the artist is
  `cat_(artist)`, as on Danbooru.
- **A new tag is created under a category by a prefix in any tag editor**: `artist:kantoku`
  tags the image `kantoku` and creates it under Artist. When `kantoku` already exists under
  another category the whole save is refused with a message naming the tag and the conflict;
  when it already exists under Artist the prefix is accepted as a plain tag. A prefix never
  re-categorises an existing tag; that is done from the tag's context menu.
- **Tags are coloured by category wherever they are read**: the inspector's badges, the
  sidebar's tag list, the editor's suggestion list. General tags keep the default text colour;
  the four rarer categories carry the colour. Text being typed stays plain.
- **The inspector's tag area is read-only until Edit is pressed.** The editor then opens with
  one line per category in the order artist, copyright, character, meta, general, tags
  alphabetical within a line; save and cancel close it. The lines are for reading: a tag's
  category comes from the vocabulary, never from the line it was typed on, and the stored set
  stays unordered.
- **A tag can be pinned.** Pinned tags are always shown at the top of the inspector's tag
  area as toggles: one click adds the tag to the described image or takes it off; over a
  selection the chip shows whether all, some or none of the selection carry it and one click
  adds it to all or removes it from all, asking first when more than one image is written.
  Pin and unpin from the tag's context menu in the sidebar and in the inspector, and unpin
  from the chip's own context menu.
- **A categorised or pinned tag survives having no image**: the vocabulary keeps it, so an
  artist whose last image was trashed is still an Artist when the next one arrives.
- **The vocabulary is described in the library's own file and comes back on a rebuild**, as
  the rules, sites, note and collections do.

## Capabilities

### New Capabilities

- `tag-vocabulary`: categories, the uniqueness of names, the creation prefix and its refusal,
  re-categorising from where a tag is shown, pinned tags and the pinned chips, and the survival
  of a categorised or pinned tag with no carriers.

### Modified Capabilities

- `tag-editing`: the editor is closed until opened and presents one line per category; the
  suggestion list stays shut behind a category prefix as behind a rating one; tags on screen
  carry their category colour beside the search marking.
- `tag-sidebar`: listed tags carry their category colour; each row's context menu offers pin
  and category.
- `bulk-operations`: the add list reads the creation prefix under the same rule; a bulk removal
  that empties a categorised or pinned tag keeps it in the vocabulary.
- `library-recovery`: the library's own file describes the vocabulary; a rebuild restores it.

## Non-goals

- A tag management page. The sidebar's rows and the inspector's badges, with a context menu,
  are where a category or a pin is set; the owner's call (2026-09-23) is "we do it simple".
- Search by category (`arttags:` and the like), or `artist:` as a search prefix. The search
  language is unchanged; any later prefix must stay clear of the category names.
- Pushing categories to a booru on upload (`artist:` for a new tag there). `booru-upload` can
  read the vocabulary in a later change.
- Aliases and implications.
- A separate pinned-tags section elsewhere than the inspector.

## Impact

- Schema: one migration (planned as v7 by queue position; the design's sentence is amended
  to the real `MIGRATIONS.len()` when applied) adding `category` and `pinned` to `tags`.
- `library.json` gains a `tags` key listing every tag that is not general-and-unpinned;
  format stays 1 (the collections precedent). Per-image sidecars are unchanged.
- Rust: `tags.rs` (vocabulary, category prefix parsing where `split_rating` is today, the
  conflict refusal, orphan exemption), `sidecar.rs`, `recover.rs`, `model.rs`, `commands.rs`,
  `lib.rs`, `ingest.rs` (rule application keeps an existing tag's category).
- Shared: `TagCategory`, `TagEntry`.
- Webview: a `vocabulary` store, `tags/categories.ts` (order, colours, labels),
  `domain/tag-input.ts` (grouped editor text, the prefix in the metatag list),
  `Inspector.svelte` (read-only default, Edit, pinned chips), `TagSidebar.svelte` (colours,
  context menu), `TagInput.svelte` (suggestion colours), `pending-write.ts` (an `edit` kind
  for the pinned write over a selection), `LibraryScreen.svelte` (wiring), one shared menu
  component for the tag context menu's vocabulary items.
