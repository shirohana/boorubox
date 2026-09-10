## Why

Every image that enters the library arrives untagged. A capture from an X post carries the
handle, the tweet text and the post URL in its adapter record (`bridge-extension` D11 stores
it verbatim); a local import carries its filename. Nothing reads any of that, so the tag
work the app exists for starts from zero on every image, one at a time in the Inspector.

The legacy extension solved this with auto-tag rules — a name, a pattern, a set of tags —
applied to the page title at save time, and the owner has been running them for a year. §4
puts them in the app on purpose: *extraction lives in the extension, policy lives in the
app*, and a rule is policy. §8 Phase 2 lists "auto-tag rules, notes" as parity work, and §10
records that rules and notes live outside IndexedDB in `chrome.storage.local`, which is why
the Phase 0 bundle carries `tagRules` and `notesContent` and why the app needs somewhere to
put them when that bundle lands.

Notes ride along because they are the other half of what §10 says is left behind: one
free-text scratchpad per library, autosaved, which the legacy viewer kept in the sidebar.

**Depends on:** `tags-and-ratings` (its `tags.rs` is the one Rust tag write path, and a rule
that names `rating:e` has to behave like a user typing it — its D3), `bridge-extension` (the
adapter record a rule matches against, and schema v2). Cites `app-shell`'s slot map for
placement.

## What Changes

- **Rules live in the library** (owner decision, 2026-09-06: library policy travels with the
  folder): a `rules` table in `library.sqlite` — name, pattern, substring or regex, the tags
  to add, enabled — behind commands the webview calls. Schema v3.
- **Rules run at ingest, in the one Rust door** (§4): `ingest::store_image` matches every
  enabled rule against the image's title and its adapter fields and adds the matching rules'
  tags before the row is written, so a capture and a local import land already tagged. The
  tags go through `tags-and-ratings`' write path, so `rating:s` in a rule's tags sets the
  rating instead of becoming a tag.
- **Matching semantics are the legacy's, unchanged**: a disabled rule never matches, an empty
  pattern matches everything, a substring match is case-insensitive, a regex match is
  case-insensitive, and a pattern that is not a valid regex makes the rule invalid — reported
  as such and skipped, never an error that reaches the image.
- **What a rule matches against grows** from the legacy's page title alone to the title plus
  the site and every string the adapter record's fields carry (§4: the app decides what to do
  with the extracted record). A local import's title is its filename, which is what the legacy
  matched there too.
- **"Run rules on existing images"** (Settings → Rules): apply the current rules to every
  image already in the library, with a report of what changed. Rules only ever add tags.
- **Rules import and export as JSON**, in the shape the legacy extension wrote, with its
  fingerprint dedupe — importing the same file twice adds nothing the second time. This is
  the door `legacy-bundle-import` reuses for the bundle's `tagRules`.
- **Notes** (§8 Phase 2, §10): one free-text note per library, autosaved as it is typed, in a
  collapsible sidebar panel below the tag list.

## Capabilities

### New Capabilities

- `auto-tag-rules`: rules that add tags to an image from what its source said about it —
  their storage, their matching, when they run, running them over images already stored, and
  moving them between libraries as JSON.
- `notes`: one free-text note per library, kept with the library and saved as it is typed.

### Modified Capabilities

None. Ingest gaining rules does not change what `capture-ingest` or `local-file-import`
require: neither says a stored image carries only the tags its source sent, and both keep
every scenario they have. The new behaviour is a requirement of the new capability
("Rules apply as an image enters the library"), which is also where a reader looking for
rules will go. A `MODIFIED` block would additionally collide with `bridge-extension`'s, which
is already rewriting the same `capture-ingest` requirement (the hazard `tags-and-ratings`
recorded as its D16).

## Non-goals

- **Rules that remove tags, set a source, or rewrite a title.** A rule adds tags, and
  `rating:` among those tags sets a rating. Anything else is a second rule language.
- **Rule ordering.** The tags of every matching rule are unioned, so order cannot change the
  outcome; a reorder control would be a control that does nothing (`app-frame`'s requirement).
  design D4.
- **Rules on legacy-bundle import.** `legacy-bundle-import` records "whatever the bundle
  carries is what is stored" (its Non-Goals); re-deriving tags for images that already have
  the old library's would fight it. design D7.
- **Rules matching image content, dimensions, size or existing tags.** The pattern is matched
  against text the source supplied, as in the legacy.
- **Bulk undo of a run.** A run's report says what changed; removing those tags again is
  `selection-and-bulk`'s bulk remove.
- **Notes per image.** One note per library. Per-image commentary is `booru-upload`'s form
  field, and a per-image free-text field is a schema decision nobody has asked for.
- **Rich text, or notes in search.** The note is plain text and is not indexed by FTS.
- **A rules nav item.** design D11 puts the rules UI in Settings, not on a route of its own.

## Impact

- `packages/shared` + `packages/app/src-tauri/src/model.rs` (hand-mirrored, Phase 1 D11, one
  commit for both): new `Rule`, `RuleInput`, `RuleListEntry`, `RulesImportReport`,
  `RulesRunReport`, `RuleRunCount`, `Note`.
- `packages/app/src-tauri`: schema v3 in `db.rs` (`rules`, `notes`); new `rules.rs` (the
  matching module, the JSON format and its fingerprint, the run) and `notes.rs`;
  `ingest.rs` gains the rule application step inside `store_image`'s transaction and routes
  its tags through `tags.rs`; `commands.rs` gains `rules_list`, `rules_upsert`,
  `rules_delete`, `rules_import`, `rules_export`, `rules_run`, `note_get`, `note_set`.
  New dependency: the `regex` crate.
- `packages/app/src`: `lib/components/rules/` (the rules table and its form) mounted in
  Settings (Slot: Settings · Rules); `lib/components/notes/NotesPanel.svelte` in the sidebar
  (Slot: Sidebar · filters, below the tag list); `lib/api/commands.ts` gains one wrapper per
  command.
- `settings.json` gains `notesCollapsed` (design D13, which reverses a written app-shell
  non-goal and says why).
- `legacy-bundle-import` gains a door it does not have to build: the bundle's `tagRules`
  array is exactly what `rules_import` parses, and `notesContent` is what `note_set` takes.
  Stated here so that change can cite it rather than write a second importer.
