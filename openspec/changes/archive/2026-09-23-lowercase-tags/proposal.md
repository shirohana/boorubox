## Why

Tag names compare with SQLite's BINARY collation, so `Tagme` and `tagme` are two rows. The
owner hit it on 2026-09-23: with `meta:tagme` in the library, `artist:tagme` is refused as it
should be, but `artist:Tagme` is accepted and creates a second tag that reads as the first.
Danbooru normalises every tag to lowercase and the owner wants the same ("case insensitive and
ALL IN LOWERCASE"). Requirements §6 (Danbooru-style tagging). This closes the open decision
recorded in the 2026-09-23 backlog (case-sensitive tags: lowercase on write, or leave).

## What Changes

- **Every tag name is stored lowercase.** Whatever door a tag comes through — the editor, a
  stamp, the bulk dialog, a rule, a capture, a sidecar on rebuild, a bundle or booru import —
  the name is lowercased before it is looked up or created, so `Tagme`, `TAGME` and `tagme`
  name one tag, and a category conflict is found whatever the case typed.
- **Search is case-insensitive.** `Cat` in the search finds the images tagged `cat`; the
  panels mark `cat` as included.
- **Existing libraries are migrated.** One migration lowercases every tag row and merges the
  rows that then collide: images tagged with either spelling are tagged with the survivor, a
  non-general category wins over general, a pin survives.

## Capabilities

### Modified Capabilities

- `tag-vocabulary`: "Every tag has one category" gains the name rule — lowercase, unique
  without regard to case.
- `tag-editing`: a save lowercases what was typed.
- `library-browse`: the tag search matches without regard to case.

## Non-goals

- Unicode case folding beyond what `str::to_lowercase` and `String.prototype.toLowerCase`
  do; both are Unicode-aware and agree on every tag this library holds.
- Renaming or merging tags by hand (a management page).
- Rewriting every sidecar after the migration: the doors lowercase on read, so a sidecar that
  still says `Tagme` restores `tagme`.

## Impact

- Schema: one migration (the design names the number when applied).
- Rust: `tags.rs` (one `canonical` function at every door), `db.rs` (the migration list gains
  a Rust-driven step), `query.rs` (tag clauses compile lowercased), `recover.rs` (sidecar and
  vocabulary restore), `rules.rs`, `commands.rs` (`set_category`, `set_pinned`,
  `selection_tag_counts` by name).
- Webview: `domain/tag-utils.ts` (`parseTagSearch` lowercases tag terms so the marking and
  the query-rewrite helpers agree with what Rust stores), `domain/stamp.ts` (the parsed edit
  is lowercase, so the bar shows what will be written).
