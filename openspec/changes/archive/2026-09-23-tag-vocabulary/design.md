## Context

See proposal.md — Why. What the code holds today, as far as this change reaches:

- `tags` is `(id, name UNIQUE)` and nothing else (`db.rs`). `tags::link_tag` is the one place a
  row is born; `ingest::insert_rows` and `recover::insert_sidecars` go through it. Six
  migrations; `MIGRATIONS.len()` is the schema version.
- `tags::collect_orphans` deletes a `tags` row the moment its last `image_tags` link goes, so
  the vocabulary is exactly the set of tags in use; `tags::suggestions` inner-joins
  `image_tags` on the strength of that.
- `tags::split_rating` reads `rating:g|s|q|e` out of editor text; `TagEdit::read` calls it for
  `update_tags`, and `ingest::insert_rows` calls it when applying a rule's tags at capture
  time. `bulk_update_tags` does not read metatags at all: it links its add list plainly.
- Per-image sidecars carry tag names only; `library.json` (`LibraryFile`, format 1) carries
  rules, sites, note and `collections: Option<Vec<Collection>>`, whose `Option` tells "the user
  has none" from "written before the key existed". `recover::rebuild` re-creates every `tags`
  row from sidecar names, then restores the library file's rules, sites, note and collections.
  `sidecar::write_library` recomposes the file from the tables and is called after every rule,
  site, note and collection write.
- `ImageRecord.tags` is `string[]` everywhere: the sidecar, the wire, the webview, the tests.
  `TagCount` is `{ name, count }` for the sidebar list, the suggestion popover and the bulk
  dialog's pills.
- The webview keeps one store per library-level list (`collections.svelte.ts`: `list`,
  `refresh()`, refreshed from `Sidebar.svelte` when the library path changes and by every
  writer). The inspector's tag section shows the textarea and the badge list at once, with
  Save under the field while dirty; the facts form above it is the edit-mode pattern (an Edit
  button, a `$state` flag, Escape cancels). `TagInput`'s `multiline` makes Enter confirm, never
  a line break; `tagList` splits on any whitespace, so a newline in the editor is already a
  separator. `editorText(tags)` joins with spaces and a trailing space. `pending-write.ts`
  holds the one confirmation's kinds (`trash | delete | empty | rate`).
- The tag context menu exists in two places: the sidebar row has none (only include, exclude,
  toggle buttons); the inspector badge has search-for, exclude, remove. `ContextMenu.Trigger`
  with `{#snippet child({ props })}` is the pattern that keeps a row's inner buttons working.

## Goals / Non-Goals

**Goals:** the category and the pin live on the tag row and in `library.json`, and nowhere
else; one Rust reader for the editor prefixes, shared by every writer; one webview reader for
"what colour is this tag", so a new artist is coloured the same in the sidebar, the badges and
the popover the moment its write returns; the inspector's tag area becomes read-first without
changing what a save stores.

**Non-Goals:** a `category` on `ImageRecord.tags` (it would touch the sidecar format, the wire
and every fixture for a fact the vocabulary store answers in one lookup); search by category;
the booru upload (a later change reads the vocabulary); a management page.

## Decisions

### D1. Two columns on `tags`, one migration

`ALTER TABLE tags ADD COLUMN category TEXT NOT NULL DEFAULT 'general' CHECK (category IN
('artist','copyright','character','meta','general'))` and `ALTER TABLE tags ADD COLUMN pinned
INTEGER NOT NULL DEFAULT 0`, as one migration — schema v7 (`MIGRATIONS.len()` in `db.rs`; unit R
landed it on main at `18963d9`, matching the plan's queue-position guess). The category is a
property of the tag, so it is a column
names are the storage form and the wire form (`TagCategory` in `model.rs` with serde
lower-case, mirrored in `packages/shared`); the `CHECK` keeps a hand-edited file from putting a
sixth in. `pinned` is a flag on the same row for the same reason, and because the orphan rule
(D3) reads both together.

*Alternative rejected:* categories only in `library.json`, read into the webview and never in
SQL. Every Rust reader that has to refuse a conflicting prefix (D4) would then read a JSON file
per save.

### D2. `library.json` lists the vocabulary's exceptions; format stays 1

`LibraryFile.tags: Option<Vec<TagEntry>>` with `TagEntry { name, category, pinned }`, holding
every tag whose row is not `(general, unpinned)`, sorted by name; `#[serde(default)]`, written
by every build from here on (`Some(vec![])` when nothing is categorised or pinned). The
`Option` reproduces the collections precedent and the rebuild reads it the same way: `None` is
a file from before this build and every tag comes back general; `Some` is restored verbatim.
The format number does not move: an old reader ignores the key, and `check_version` refusing
every older library over an additive key is the outcome the collections change already argued
against.

`tags::vocabulary(conn) -> Vec<TagEntry>` is the one reader of the exceptions, used by
`write_library`, by the `tag_vocabulary` command and by the tests. `write_library` is called
after every write that can change the vocabulary: `set_tag_category`, `set_tag_pinned`, and any
tag write that created a tag under a category (D4 answers whether it did, so `update_tags`,
`bulk_update_tags` and the rule application rewrite the file only when a categorised row was
born). The rebuild restores the vocabulary after the sidecar pass: `INSERT INTO tags (name,
category, pinned) … ON CONFLICT (name) DO UPDATE SET category = excluded.category, pinned =
excluded.pinned`, so a tag the sidecars already linked takes its category and a tag no sidecar
names is created with none — the carrier-less artist of the spec.

*Why not the per-image sidecar:* a category is one fact about one tag; writing it into every
carrier's file repeats it once per image, makes a category change an N-file rewrite, and gives
a rebuild N possibly disagreeing answers.

### D3. Orphan collection spares a categorised or pinned tag; suggestions offer it

`collect_orphans` gains `AND category = 'general' AND pinned = 0`. A tag someone took the
trouble to file as an artist or to pin is vocabulary the user built, not a by-product of one
image; losing it with the image's last trash-and-delete would make the next capture by that
artist a general tag again. `tags::suggestions` moves from an inner join to a `LEFT JOIN` so
such a tag is offered at count 0 — the join's own comment said the inner join rested on
orphans being unreachable, and that premise ends here. The sidebar list and `tag_counts` are
unchanged: they describe a result, and a carrier-less tag is in no result. The bulk-operations
spec's "no tag left that no image carries" is narrowed to the general, unpinned case in its
delta.

### D4. One reader for the editor's metatags, with two conflict policies

`split_rating` becomes `read_metatags(tags) -> TagText { tags: Vec<String>, rating:
Option<String>, categories: Vec<(String, TagCategory)> }`: a token `artist:x`, `copyright:x`,
`character:x`, `meta:x` or `general:x` (prefix case-insensitive; Danbooru's short forms `art:`,
`copy:`, `char:`, `gen:` accepted too, since the owner types Danbooru) yields the plain tag `x`
in `tags` and the pair in `categories`. A token with an empty name after the prefix is dropped
like a blank. Everything else is as `split_rating` was.

`link_tags(conn, image_id, text: &TagText, policy: Conflict) -> Result<bool>` replaces the
per-writer loops over `link_tag`: for each pair it reads the existing row; none → create with
the category (the answer is `true`, "the vocabulary changed", so the caller rewrites
`library.json`); same category → link plainly; different category → `Conflict::Refuse` answers
`AppError::BadRequest` with the spec's message ("`cat` is a general tag and cannot become an
artist tag; use another name, e.g. `cat_(artist)`") and the transaction rolls back, while
`Conflict::Keep` links the existing tag as it is. `update_tags` and `bulk_update_tags` use
`Refuse` — the text came from the user's hands and the refusal is the answer they need to pick
another name; `ingest::insert_rows`'s rule application uses `Keep` — a capture's 2xx must not
depend on a rule's spelling (`capture-ingest`: adapters and rules never block the save). The
error text is composed in one function so the editor, the bulk dialog and the later stamp
apply say the same thing.

*Why the prefix never re-categorises:* the owner's rule (2026-09-23). A prefix is a creation
spelling, and a save that silently changed `cat` for every image in the library because one
editor read `artist:cat` is the kind of wide write an editor for one image must not make. The
context menu (D7) is the deliberate, visible act for that.

### D5. The webview reads the vocabulary from one store

`api/vocabulary.svelte.ts`: `Vocabulary { entries: TagEntry[]; categoryOf(name): TagCategory
(general when absent); isPinned(name); pinned: string[] (sorted by name); refresh();
setCategory(name, category); setPinned(name, pinned) }`, the shape of `collections.svelte.ts`.
The two setters call the commands and replace `entries` with the answer (each command answers
the whole vocabulary, the cheapest way to keep one list in step). `refresh()` runs where the
collections store's does (`Sidebar.svelte` on a library path change) and after every tag
write: the results store's `saveTags` path, `LibraryScreen.afterWrite`, the pinned chip's
write. The list is the exceptions only (D2), so it is hundreds of names in a library of
thousands of tags, and a lookup is a `Map`.

*Why not `category` on `TagCount` or `ImageRecord.tags`:* the badge list, the sidebar and the
popover would each carry a copy of the same fact and a freshly categorised tag would be right
in one place and stale in the others until three refreshes agreed. One store, one lookup.

### D6. Colours and order live in one module

`components/tags/categories.ts`: `CATEGORY_ORDER = ['artist', 'copyright', 'character', 'meta',
'general']` (the editor's line order and the badge grouping), `categoryLabel`, and
`CATEGORY_TEXT_CLASS`: artist `text-red-600 dark:text-red-400`, copyright `text-violet-600
dark:text-violet-400`, character `text-green-600 dark:text-green-400`, meta `text-amber-600
dark:text-amber-300`, general `''`. Danbooru's five hues, minus blue for general: general is
the majority of every panel, and colouring the majority makes the four that matter harder to
find, not easier; blue is also the account chip's and the link colour here. The search marking
stays on the background (`bg-emerald-500/15`, `bg-destructive/10 line-through`), so it is
visible on a coloured tag. The hex-precise Danbooru palette is not copied: Tailwind's steps
are what the rest of the app is drawn in, and the owner asked for a design for this app.

*Amended (tag-panel-polish, 2026-09-23):* general is now coloured too —
`text-blue-600 dark:text-blue-400`, Danbooru's own hue for it. Both reasons above have
ended: blue stopped being spoken for the moment the account entry left the tag area for a
facts row of its own (`tag-panel-polish` design D4), and the owner's taste call for that
pass is that colouring the majority is the point of a Danbooru-style panel, not a problem —
the four minority colours are still found by their hue, not by general's absence of one.
`CATEGORY_ORDER` is amended in the same change, to `artist, copyright, character, general,
meta` (`tag-panel-polish` design D1); this D6 keeps only the colour and label decision.

`domain/tag-input.ts`: `editorText(tags, categoryOf)` groups by `CATEGORY_ORDER`, sorts each
group with `sortTags`, joins groups with `\n` and tags with a space, and ends with a space
(or with nothing for no tags), so the trailing-space rule of the existing tests holds per file.
The `METATAG` list that shuts the suggestion popover gains the category prefixes and their
short forms — the second copy of the metatag alphabet, kept in step by the tests.

### D7. The inspector's tag area: read-first, Edit opens the field

The tag section follows the facts form: a heading row with the count and a pencil Edit button;
read mode shows the pinned chips (D8) and the badge list grouped by `CATEGORY_ORDER`; edit mode
swaps the badge list for the `TagInput` (still `multiline`: Enter saves, as `tag-editing`'s
"Confirming a tag takes two steps" requires; a line break is only ever written by
`editorText`) with Save and Cancel under it, Escape cancelling as the facts form does. A
successful save closes the editor and releases the focus (`onrelease`); a refused save keeps
it open with the text and the reason under it. The draft is seeded by `editorText` from the
vocabulary store's `categoryOf` when the editor opens, not on every status refresh: the
existing `$effect` keyed on `id:updatedAt` re-seeds only while the editor is closed, so a
capture arriving mid-edit does not overwrite the text.

The badge's context menu keeps its three items and gains the vocabulary group (D9). The
`TagInput` popover rows take `CATEGORY_TEXT_CLASS[vocabulary.categoryOf(name)]`.

### D8. Pinned chips in the tag section, tri-state over a selection

Above the badge list, a wrapping row of `Badge`-styled toggle buttons, one per
`vocabulary.pinned`, drawn as `secondary` when the described image lacks the tag and filled
when it carries it, with the category colour on the text either way. One image: the click
calls `results.saveTags(id, toggled set)` — the same whole-set write the editor makes, so the
badge list, the sidebar and `updatedAt` follow as they do for a save. A selection (the
"Several selected" branch of the panel): the chips read `selectionTagCounts(ids, …)` fetched
once per selection change (the same command the bulk dialog uses; the limit is raised to cover
every pinned tag — ask for the counts of the pinned names, which is a filter the command gains
as an optional `names` argument rather than a second command), and show all / some / none by
fill; a click builds `{ add: [tag] }` or `{ remove: [tag] }` and hands it to the screen as a
`PendingWrite` of a new kind `edit: { ids, add, remove }`, which the one `ConfirmDialog` asks
about when `needsConfirmation(count)` ("Add `tagme` to 12 images?" / "Remove `tagme` from 12
images?", not destructive), and which runs `bulkUpdateTags` then `afterWrite`. The `edit` kind
is the shape the later `stamps` change widens (collections, rating); it is introduced here with
the two fields this change needs.

*Why the chip does not ask for one image:* the write is the same one the editor makes and is
reversed by the same click.

*Amended (review pass, 2026-09-23):* "filled" read as `variant="default"` (`bg-primary`), and
against `CATEGORY_TEXT_CLASS`'s amber/violet/red/green text that is near-white in dark mode — the
text lost its contrast the moment a chip filled. Every chip now draws `secondary`; `all` adds
`ring-1 ring-foreground/40`, `some` a dashed `outline-1 outline-foreground/40` (Tailwind's `ring-*`
utilities have no dashed style, so `some` reaches for `outline-*` instead). The fill state is a
ring or outline, not a background, so it never competes with the category colour, which stays the
text's alone against the muted background either way.

*Amended (review pass, 2026-09-23):* the selection branch's read is not the id-free fetch this
decision implies. `selectionTagCounts` still needs ids, and reading them — even through the
inspector's own action-free `peekIds()` rather than `ids()` — resolves a range through
`search_ids` to draw a chip nobody has clicked, against D3's "only when an action needs ids". The
honest fix is a Rust `selection_tag_counts` that takes the search request plus a row range and
counts over the plan's rows directly, so no ids cross the wire for something that is only ever
drawn until it is clicked; not built in this pass, since a Rust unit was in flight on the same
files (`tags.rs`, `commands.rs`). Left as a `FIXME` on `Inspector.svelte`'s effect.

### D9. One menu fragment for pin and category

`components/tags/TagVocabularyMenuItems.svelte`: a snippet-free component taking `name` and
rendering `Pin` / `Unpin` (by `vocabulary.isPinned`), a separator, and a `ContextMenu.Group`
with heading "Category" and five `ContextMenu.CheckboxItem`s (checked = current), calling the
store's setters. Mounted inside the sidebar row's new `ContextMenu` (the `CollectionsSection`
row shape), inside the inspector badge's existing menu after its separator, and inside the
pinned chip's menu (where only `Unpin` and the category group make sense; `Pin` is never
offered on a chip). The sidebar row's include/exclude/name buttons stay inside the trigger's
child snippet so the row keeps working.

## Risks / Trade-offs

- [A hand-edited `library.json` with an unknown category] → `TagCategory::from_str` fails
  inside serde before the file is ever read row by row, so the whole file — rules, sites,
  note, collections and vocabulary alike — is refused as one unreadable document and named
  in the rebuild's report, the same failure an unreadable sidecar gets, repaired by fixing
  the file and rebuilding again.
- [The metatag alphabet now has three copies: `read_metatags`, `parseTagSearch`, `METATAG`]
  → the search parser does not learn the category prefixes (a non-goal); the editor's two
  readers are pinned by tests that list the same tokens.
- [`suggestions`' LEFT JOIN offers a carrier-less tag] → wanted (spec); it sorts last by count.
- [The tag section re-seeds the draft while editing] → the effect re-seeds only while the
  editor is closed (D7).
- [Every tag write now may rewrite `library.json`] → only when a categorised row was created
  (`link_tags` answers whether one was); a plain save costs nothing more.
- [`selectionTagCounts` grows an optional `names` filter] → additive; the bulk dialog's call
  is unchanged.
