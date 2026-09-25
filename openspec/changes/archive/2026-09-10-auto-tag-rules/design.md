## Context

`ingest::store_image` is the one door every image enters by (Phase 1 D4): decode, write through
`inbox/`, insert the row and its `image_tags` links in one transaction. It takes the tags its
caller hands it, and all three callers hand it something fixed — captures pass `&[]`, local
import passes `&[]`, the bundle importer passes what the bundle said.

Two changes land before this one and are what make rules possible. `bridge-extension` adds
schema v2 (`images.adapter_json`, its D11): the whole `{ site, fields }` record as the
extension sent it, with `handle` / `postUrl` / `postText` / `originalUrl` for X and `artist` /
`workId` / `title` / `originalUrl` for Pixiv (its D8). `tags-and-ratings` adds `tags.rs`, the
one Rust tag write path, which owns the rule that `rating:g|s|q|e` among an image's tags sets
the rating instead of becoming a tag (its D3) and collects orphan tags (its D5).

`app-shell` fixes placement: `/settings` is a route with sections owned by the capability that
owns each fact (its D16), and the slot map reserves `Settings · Rules` and a notes panel under
`Sidebar · filters`. Motivation: see proposal.md.

Constraints that shape this design:

- Rules and notes live in `library.sqlite`, not `settings.json` — the owner's decision of
  2026-09-06: library policy travels with the folder.
- Extraction lives in the extension, policy lives in the app (§4). A rule is policy, and this
  is the change that puts the extension's extracted record to work.
- `packages/shared/src/index.ts` and `model.rs` are hand-mirrored (Phase 1 D11): every new
  type is written twice, in one commit.
- Rollback journal, one writer (§7). A run over ten thousand images is one long transaction on
  the one connection; nothing else writes while it holds the mutex.

## Goals / Non-Goals

**Goals:**

- One definition of when a rule matches, in Rust, called by ingest and by the run. The legacy
  had one too; a second one in the webview would drift the first time a field is added.
- A rule can never cost a capture. Every failure mode of a pattern ends as "this rule did not
  match, and here is why", never as an error the delivering client sees.
- The tags a rule writes are indistinguishable from tags a user typed — same write path, same
  `rating:` rule, same orphan invariant.
- Nothing about rules is a second source of truth for anything the library already knows.

**Non-Goals:**

- Speed of the run. It is a deliberate, occasional action over the whole library; a plain loop
  in one transaction is the shape until someone measures a problem.
- Matching on anything but text the source supplied (proposal.md, Non-goals).
- A rules cache. Ingest reads the rules table per image (D7).

## Decisions

**D1. Schema v4 creates `rules` and `notes`. This change owns v4.**

Version ownership, stated because several Phase 2 changes are drafted in parallel: v1 is
`phase-1-app-mvp` (its D2), v2 is `bridge-extension` (`images.adapter_json`, its D11),
`tags-and-ratings` appends nothing (its D1). This design originally took v3, reasoning that
`tags-and-ratings` left it free — true when this was written, but `browse-polish`
(`images.file_modified_at`) landed first and took v3 before this change was implemented,
which is what the sequence in the change queue decides, not the order designs are drafted in.
`db.rs`'s `MIGRATIONS` is append-only, so the schema this change writes is simply the next
entry: v4. A later Phase 2 change needing one takes v5.

```sql
CREATE TABLE rules (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    pattern    TEXT NOT NULL,
    is_regex   INTEGER NOT NULL,
    tags_json  TEXT NOT NULL,
    enabled    INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE notes (
    id         INTEGER PRIMARY KEY CHECK (id = 1),
    content    TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);
```

`id TEXT` on `rules` because the ids come from `uuid::Uuid::new_v4()`, the same as image ids,
and because the JSON format carries string ids (D10). No index: the table holds tens of rows
and every read is "all of them" — ingest needs the whole enabled set, and the settings list
needs the whole set including the disabled ones. Adding one would be an index nothing uses.

`created_at` and `updated_at` are kept even though no requirement reads them: when a run's
report surprises someone, "which rule did I change last" is the first question, and two
integers on a table this size cost nothing. They are stamped in epoch milliseconds by the same
`db::now_ms()` as every other timestamp.

**D2. A rule's tags are one JSON document in `tags_json`, not rows in `tags` / `image_tags`.**

§7's "tags as rows, never a comma string" is a rule about the tags *of an image*: it exists so
tag search can be a join and a tag can be counted. A rule's tags are its payload — a list of
strings to write later — and they are never searched, never counted, and never joined against
anything. Putting them in `tags` would be worse than clumsy: it would create `tags` rows for
tags no image carries, which breaks the invariant `tags-and-ratings` D5 establishes ("a row in
`tags` has at least one use") and would put the rule's not-yet-used tags into the editor's
autocomplete vocabulary.

Alternative — a `rule_tags` child table — rejected: it would be read only to rebuild the list
it was flattened from, and it would need its own ordering column to round-trip an export.
Alternative — a space-separated string, which is how the legacy form's input is split —
rejected: the JSON array is already the export format's shape (D10), so storing it that way
means one representation from the file to the column to the wire.

The column is named `_json` for the reason `bridge-extension` D11 named `adapter_json` that
way: it is a document, not a value to compare with `=`.

**D3. The note is one row, and the table's shape says so.**

`CHECK (id = 1)` rather than a convention that every writer has to remember. `note_set` is an
`INSERT ... ON CONFLICT (id) DO UPDATE`, so the first write creates the row and no caller has
to know whether it exists; `note_get` returns an empty string when there is no row, which is
what the spec requires of a library that has never been written to. Alternative — a
general-purpose `library_settings (key, value)` table with a `note` key — rejected: the second
key would arrive without an argument for what belongs in the library rather than in
`settings.json`, and that boundary is exactly what the owner decided on 2026-09-06.

**D4. There is no rule order, and no reorder control. The list is ordered by name.**

Rule order cannot change the outcome: the tags of every matching rule are unioned into a set,
which is what `getAutoTags` does and what the spec requires ("a tag named by two rules SHALL be
carried once"). A `position` column would therefore be a stored value with no observable
effect, maintained on every insert, delete and import for nothing — and a drag handle in the
settings table would be a control that does nothing, which `app-frame`'s "No control appears
before it does something" forbids.

The legacy sorted its list by name, case-insensitively, and that is kept: a list a user scans
to find a rule wants a stable, predictable order, and creation order is neither.

This is a departure from the brief's schema sketch, which listed `position/order` among the
columns. Recorded rather than silently dropped: if a later rule kind ever makes order
observable — a rule that removes a tag another added, say, which proposal.md rules out — the
column comes back with that change and its argument.

**D5. A rule is matched against the title, the adapter record's site, and every string in its
fields. Not the page or image address.**

The legacy matched the page title alone, because that was all the extension had. This app has
the adapter record, and §4 says the app is what decides what to do with it — so the natural
first use is that a rule can say "anything from this account" or "anything whose tweet text
mentions this" without the user having to hope the title carries it.

Every string value is matched, including each entry of a field holding several values, and
including field names the app has no meaning for. Not a whitelist of `handle` / `artist` /
`postText`: `bridge-extension` stores the record verbatim precisely so the app is not a second
definition of what the adapter produces, and a whitelist here would mean a new adapter field
is invisible to rules until someone edits a match list in Rust. The site is matched too — it
is the one field every adapter record has, and "everything from Pixiv" is the most obvious
rule anyone writes.

The page address and the image address are **not** matched. They are long, punctuation-dense
strings in which a short substring pattern matches by accident (`art` matches
`https://…/artworks/…`, `/art/`, `smart`, half of every CDN hostname), and the legacy's
semantics — plain substring, case-insensitive, no anchoring — is what a year of rules was
written against. Every address that carries information a rule would want is already an
adapter field (`postUrl`, `originalUrl`) and is matched as one. Alternative — match the page
address too — rejected on that noise; changing this later changes a spec scenario ("Addresses
are not matched"), so it is a decision, not something to leave open.

For a local import the title is the filename, which `import.rs` already sets as `page_title`.
The legacy stripped the extension off first; this app does not, so `.png` is matchable — a
difference recorded here because a legacy rule anchored with `$` would behave differently.

**D6. Matching is ported verbatim, and validity is computed at read time, never stored.**

`rules::matches(rule, haystacks)` reproduces `matchesRule` line for line: disabled → false;
empty pattern → true; regex → case-insensitive test; otherwise case-insensitive substring.
`rules::auto_tags(rules, haystacks)` reproduces `getAutoTags`: a set, in rule order, so a tag
named twice lands once. Absent text is empty, not an error. The one extension is that
`haystacks` is a list of strings rather than one (D5) and a match against any is a match.

The regular expression engine is the `regex` crate (new dependency) with
`case_insensitive(true)`, not a pattern with `(?i)` prepended — prepending would corrupt a
pattern that legitimately starts with something else and would be a second place the
case rule is written.

A rule's validity is not a column. `rules_list` compiles each regex pattern as it reads the
row and returns `patternError: Option<String>`; the ingest path compiles as it matches. The
engine is the only authority on whether a pattern is usable, and a stored flag would be a
cached answer that goes stale the day the crate is upgraded — exactly the "compute, don't
cache" case. Compiling tens of short patterns per ingest is microseconds; if that ever shows
up in a profile, the fix is a per-call compile cache inside one function, not a column.

`rules_upsert` refuses an unusable pattern with the compiler's message, so a rule written in
this app is valid by construction, while `rules_import` accepts one and stores it: the file
came from another library — in the limit from the legacy extension, whose patterns were
written against JavaScript's engine — and dropping a rule on import would lose it silently.
The asymmetry is the point, and the spec states both halves.

**D7. Rules run inside `store_image`, before the row is inserted, and the source decides
whether they run at all.**

Inside the one door, because the alternative is each of the three callers remembering to call
a rules function first — and the moment a fourth source appears (§5 exists so a CLI or a script
can post captures), it silently arrives untagged. Before the insert rather than as an update
after it, so an image is never briefly in the library without the tags it was going to get and
the FTS triggers fire once.

`store_image` decides by `input.source`: `extension` and `local` get rules, `legacy-bundle`
does not. `legacy-bundle-import` records "whatever the bundle carries is what is stored" (its
Non-Goals), and those images already carry the tags the old library gave them, produced by the
same rules — re-deriving them would fight that decision and would double-apply a rule that has
since been edited. Alternative — an `apply_rules: bool` on `IngestInput` — rejected: it makes
the answer a property of the caller's mood rather than of what the source is, and a new caller
gets whatever bool its author typed. The `source` match is in one place with the reason beside
it.

The enabled rules are read from the table on every `store_image` call, not cached in
`AppState`. A cache would need invalidating from `rules_upsert`, `rules_delete` and
`rules_import`, and the failure mode of missing one is silent: images tagged by a rule the user
deleted. The read is one prepared statement over a table of tens of rows on the connection the
call already holds.

**D8. The tags go through `tags.rs`, and the `rating:` split is a function of its own.**

`ingest::link_tag` is deleted; `insert_rows` calls the same tag writer `update_tags` calls, so
there is one place in the app that turns a list of strings into `tags` and `image_tags` rows.
On a fresh row the "replace the whole set" semantics degenerate correctly: the existing set is
empty, nothing is unlinked, and orphan collection has nothing to collect.

The `rating:` rule needs to be applied at a different moment here than in the editor, so it is
extracted from `tags-and-ratings`' write path into `tags::split_rating(tags) -> (Vec<String>,
Option<String>)` — one definition of what `rating:g|s|q|e` means, two applications: the editor
applies it with an `UPDATE`, ingest applies it by choosing the value it inserts. If
`tags-and-ratings` shipped that rule inline, the first task of group 2 lifts it out; what must
not happen is a second regex for `rating:` in `rules.rs`.

Precedence: the rating the source supplied wins over a rule's. `input.rating` is what a bundle
row or (later) a smarter capture path stated about this image; a rule is a guess from a
pattern. `rating = input.rating.or(extracted)`, one line, stated in the spec ("A rule SHALL NOT
overwrite a rating the source supplied").

**D9. `rules_run()` covers every non-deleted image in the library, and reports per rule.**

Not the current search. The button lives in `Settings · Rules` (D11), and Settings has no
search box — a button whose blast radius depends on a query the user cannot see from where
they are standing is the worse of the two. Scoping to the search would also mean the same
action means different things in two places, and would put a rules control in `Toolbar ·
actions`, which the slot map assigns to import and to the selection toolbar. The bounded thing
here is not the image set but the effect: a run only adds tags, so an over-broad run is
recoverable with `selection-and-bulk`'s bulk remove, while a run that quietly skipped the
images the user meant is not detectable at all.

Trashed images are skipped: `trash` lands before this change in the sequence, and an image in
the trash is not part of the library being tagged. Restoring it and running again picks it up.

The report is `RulesRunReport { examined, changed, rules: Vec<RuleRunCount { id, name, matched
}>, invalid: Vec<RuleRunCount-with-reason> }` — bounded by the number of rules, which is tens.
Alternative — a per-image list of what each image gained — rejected: on a ten-thousand-image
library it is a payload the size of the library crossing IPC to be scrolled by nobody. The
question a run actually asks is "did the rule I just wrote do anything", and the per-rule count
answers it.

The run is one transaction per image, not one for the whole library: a ten-thousand-image
transaction holds the library mutex and the rollback journal for its whole duration, and a
failure part-way would roll back work the report had already counted. Per image, a failure
stops the run with what has been done already committed and reported — the same shape
`import.rs` uses, where a per-item failure is an outcome rather than the end of the run.

**D10. The JSON format is the legacy `TagRule[]`, and the dedupe fingerprint is a canonical
tuple rather than a serialised object.**

Export writes `[{ id, name, pattern, isRegex, tags, enabled }]` — the legacy's six fields,
camelCase, pretty-printed. Keeping the shape is what lets `legacy-bundle-import` hand the
bundle's `tagRules` array straight to `rules_import` instead of writing a second importer, and
what lets a user's old exported rules files still work. `created_at` / `updated_at` are not
exported: they are this library's bookkeeping, and an imported rule is new here.

Import parses that shape, ignores unknown keys, assigns a fresh id per rule (the file's ids
belong to another library and could collide), and skips a rule whose fingerprint matches one
already present. The fingerprint is the legacy's equivalence class — name, pattern, kind, and
the tags **sorted** — but computed as a tuple joined on a character that cannot occur in a tag
(`\u{1f}`), not as a serialised JSON object. The legacy's `JSON.stringify` fingerprint depends
on key order in a literal; reproducing that byte-for-byte in Rust would be a coupling to
`serde_json`'s field ordering for no gain, since the fingerprint is only ever compared against
fingerprints this same function computed.

One departure from the legacy, recorded: the legacy forced `enabled: true` on every imported
rule regardless of the file. This import honours `enabled` when the file carries it and
defaults to `true` when it does not. Exporting your rules, turning three off, and importing
the file into a second library should not quietly re-enable them; the legacy's behaviour was a
consequence of `enabled` being excluded from the fingerprint, and excluding it from the
fingerprint is still right — a rule you already have is a duplicate whether or not it is
switched on — while forcing the value is not.

A file that is not an array of rule objects is refused whole, with a reason, leaving the
library untouched. Alternative — import the entries that parse and report the rest — rejected:
a rules file is small and hand-managed, and "17 of 20 imported" leaves the user comparing two
lists by hand to find out which three.

**D11. The rules UI is a section on `/settings`, not a nav item and not a route.**

The slot map lists both a `Rules` item under `Sidebar · nav` and a `Rules` section under
`Settings · Rules`. Only the second is built, and this decision spends the first row. A
sidebar nav item is for a place you look at images — `Library`, `Trash`, `Import` all show
image records; rules are configuration of the library, which is what `/settings` is, and the
sidebar's nav is the app's short list of destinations. Filling both would put one screen behind
two doors.

`Settings · Booru` is a separate section (`booru-upload` owns it, per the brief's addendum), so
the two do not share a slot despite the map's row naming both.

The section is a table (name, pattern with an "(matches all)" marker when empty and a regex
marker when it is one, the tags as pills, an enable switch, edit and delete) over a form (name,
pattern, regex switch, tags), with `Import`, `Export` and `Run on existing images` above it —
the legacy's layout, which the owner knows, rebuilt in the frame's components. Newly imported
rules are marked in the list until the section is left, which is the legacy's `NEW` badge and
the only way to see what an import of thirty rules actually added.

2026-09-25 (`rules-panel-layout`): the legacy's table became a stacked list. The table was
right when this design chose it: it was the legacy's own layout, and the owner already knew it.
It stopped being right at 672px — a table with five columns (name, pattern, tags, on, actions)
scrolled sideways at that width, hiding the switch and the edit and delete buttons behind the
scroll — and fitting the settings column outranks the familiarity the table traded on. The
section now renders one bordered entry per rule — a header line with the name, its badges, the
switch and the buttons, then the pattern and tags as labelled rows under it.

**D12. The run's progress is reported the way import's is.**

A run over ten thousand images is the second long-running blocking job in the app, and Phase 1
D12 already decided the shape for the first: `spawn_blocking`, a progress callback turned into
a Tauri event, the report as the command's return value. `rules_run` emits `rules:progress`
with the same `{ done, total }` shape `import:progress` carries, and the settings section
shows it inline. Alternative — no progress, just a spinner — rejected for the same reason it
was rejected for import: a job whose end cannot be predicted reads as a hung app.

**D13. `notesCollapsed` goes in `settings.json`, reversing app-shell's "everything but the
theme and the tile size is session state".**

Why the old reading was right at the time: app-shell was drawing a line against persisting
view state that a user does not think of as a choice — which panel happens to be open, where a
list was scrolled — because storing those makes the app open in a state the user cannot explain
and did not ask for. The inspector's visibility is exactly that: it is toggled with `i` many
times a session, and it belongs to the moment.

Why it stops being right here: the notes panel is not a view of the library, it is a text area
big enough to write in, sitting in the sidebar above the tag list and the rating pills that
this app's search is driven from. Collapsing it is not a transient toggle, it is the user
saying "I do not use notes" or "I do" — a preference, held for months, whose cost if forgotten
is paid on every launch by the tag list it pushes down. The legacy stored the same boolean
under the same name for the same reason, and the owner has been living with it.

It goes in `settings.json` and not in the library (rules and notes themselves are in the
library, and this is the boundary): the note's *content* is library data that should travel
with a copied folder, while whether one machine's sidebar shows the panel is that machine's
display preference, which Phase 1 D6 keeps out of the library file. Read with `app_settings()`
and written with a command, per app-shell D5's one-command-per-field rule.

**D14. The note autosaves on a debounce, and again when the panel loses the text.**

The legacy's 500 ms debounce is kept — it is the interval a year of use has not complained
about, and it turns a paragraph into one write instead of two hundred. The addition the legacy
did not have is that closing a library, navigating away and closing the window flush a pending
write, because this app can switch libraries mid-sentence and the legacy could not: a
`beforeunload` and an explicit flush on unmount, both calling the same `note_set`. The spec
requires it ("Leaving immediately").

**D15. Slots filled, by the row names of app-shell's slot map.**

| Slot | What this change puts there |
| --- | --- |
| Settings · Rules | the rules table, the rule form, Import / Export, and Run on existing images with its progress and report |
| Sidebar · filters | the notes panel, below the tag list and the rating pills `tags-and-ratings` put there, collapsible |
| Sidebar · nav | *nothing* — the map's `Rules` item is deliberately not built (D11) |

## Risks / Trade-offs

- [A legacy rule's regular expression uses lookahead or a backreference, which the `regex`
  crate does not support, so a rule that worked in the browser is reported invalid] → it is
  reported, with the engine's message, in the list and in the run's report — the honest
  failure. Silently not matching would be the dangerous one. If real rules hit it, the fix is
  the `fancy-regex` crate behind the same `matches` function, which is one dependency and no
  spec change.
- [Widening the haystack from the title to the adapter fields makes an old rule match more
  than it used to] → it can only add tags, and the first run of an import is visible in the
  report; the addresses, which are where accidental matches actually live, are excluded (D5).
- [A rule with an empty pattern tags every image that ever enters the library, including ones
  the user did not mean] → it is the legacy semantics and it is specified; the list marks such
  a rule "(matches all)" instead of showing an empty field, which is where the user sees it.
- [`rules_run` over a large library holds the one connection for minutes] → one transaction per
  image, not one for the run (D9), and progress is emitted; a switch or a capture arriving
  mid-run waits for the image in flight, the same as during an import.
- [The run adds tags that are hard to take back] → it never removes and never re-rates, the
  report says exactly which rules matched how many, and `selection-and-bulk` has bulk tag
  removal.
- [Ingest reading the rules table per image slows a thousand-file import] → tens of rows on an
  open connection, inside a transaction the ingest already opens; measured against the decode
  and the fsync it is noise. The cache that would remove it is the one D7 refuses on
  correctness grounds.
- [Two libraries' rules files fingerprint the same rule differently because tag order differs]
  → the fingerprint sorts the tags, and a scenario pins it.

## Open Questions

- The debounce interval for the note (500 ms, ported). One constant, no spec or task depends on
  the number.
- Whether the rules table needs its own filter box once someone keeps more than a screenful of
  rules. It is one text input over a list already in memory; it changes no spec and no command.
