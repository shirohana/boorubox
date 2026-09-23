## Context

- `tags.name` is `TEXT NOT NULL UNIQUE` with BINARY collation (`db.rs` `SCHEMA_V1`). Every
  row is born in `tags::link_tag` (plain) or `tags::link_one_tag` (with a category);
  `existing_tag_category` and `has_any_tag` (`query.rs`) look names up with `=` / `IN`.
- `tags::read_metatags` is the one reader for editor text: it takes `rating:` out and
  reads the category prefixes case-insensitively (`eq_ignore_ascii_case`), but leaves the
  name after the prefix as typed. `TagText { tags, rating, categories }` is what
  `update_tags`, `apply_edit`, `ingest::insert_rows` and `rules::apply_rules_to_image` hand
  to `link_tags`. `rules.rs` builds a `TagText` by hand (line ~463).
- `recover::insert_sidecar` calls `link_tag` per sidecar tag name; the vocabulary restore
  upserts `library.json`'s `tags` entries by name (`ON CONFLICT (name) DO UPDATE`).
- `MIGRATIONS: &[&str]` is SQL only; the runner loops it and the version is its length.
  SQLite's `lower()` is ASCII-only without ICU, so a SQL migration cannot do what the runtime
  does for a non-ASCII capital.
- Webview: `parseTagSearch` (`domain/tag-utils.ts`) splits the query into `includeTags`,
  `excludeTags`, `orGroups`, …; `activeTerms` reads the same for the panels' marking;
  `removeTagFromQuery` compares tokens as typed. `domain/stamp.ts` lowercases the collection
  slug but not the tag tokens. `TagInput` already filters suggestions case-insensitively.

## Goals / Non-Goals

**Goals:** one Rust function is the definition of a tag name's canonical form, and every
door calls it; a library that already holds both spellings comes out with one row per name;
the webview never shows a marking Rust disagrees with.

**Non-Goals:** a `COLLATE NOCASE` on the column (it hides the rule in the schema, is
ASCII-only, and would leave `Tagme` on screen); rewriting sidecars in the migration.

## Decisions

### D1. `tags::canonical(name) -> String` is the rule, called at every door

`pub fn canonical(name: &str) -> String { name.trim().to_lowercase() }` in `tags.rs`, with
its doc comment carrying the reason (Danbooru's rule, owner 2026-09-23). Callers:

- `read_metatags`: every token is canonicalised before the prefix is read, so `TagText.tags`
  and the names in `TagText.categories` are canonical; the prefix comparisons can then be
  plain `==` on the lowercased token (the `eq_ignore_ascii_case` calls become redundant —
  remove them rather than keep two ways of ignoring case).
- `link_tag` and `remove_tags`: canonicalise the name argument. `link_tag` is the only row
  birth, so a caller that bypasses `read_metatags` (`recover::insert_sidecar`) is covered
  here, and `unlink_tags_other_than` receives canonical names from `read_metatags` already.
- `rules.rs`'s hand-built `TagText`: `written_tags` come from `read_metatags` of the rule's
  text or from the extension's record — check which and canonicalise where the rule's tags
  enter (`apply_rules_to_image` / the capture path) so a rule text `Tagme` writes `tagme`.
- `set_category`, `set_pinned`, `selection_tag_counts`'s `names` filter, `suggestions`'
  prefix: canonicalise the incoming name.
- `recover`'s vocabulary restore: `canonical(entry.name)` in the upsert.
- `query.rs`: `include_tags`, `exclude_tags` and every `or_groups` member canonicalised where
  the tag clauses are built (`text_values` or the call sites); `has_any_tag`'s comment about
  BINARY collation is replaced by the rule (names are canonical on both sides, so BINARY is
  exact).

*Why Unicode lowercase and not ASCII:* the webview's `toLowerCase()` is Unicode-aware; an
ASCII rule in Rust would let the two disagree on the first non-ASCII capital. `to_lowercase`
is what `String.prototype.toLowerCase` does for every script this library tags in.

### D2. The migration is a Rust step, and the list gains a variant for it

`MIGRATIONS` becomes `&[Migration]` with `enum Migration { Sql(&'static str),
Rust(fn(&Connection) -> rusqlite::Result<()>) }`; the runner matches. `MIGRATIONS.len()`
stays the version; the existing eight entries wrap in `Sql`. The new step
`merge_case_duplicates` (in `db.rs`, calling `tags::canonical`):

1. Read every `(id, name, category, pinned)`; group by `canonical(name)`.
2. For a group with one row whose name is already canonical: nothing.
3. Otherwise the winner is the row whose `name == canonical` if one exists, else the lowest
   `id`. Its category becomes the first non-general category among the group in `id` order
   (the winner's own first), or general; `pinned` is true if any row is.
4. For each loser: `UPDATE OR IGNORE image_tags SET tag_id = winner WHERE tag_id = loser`
   (an image that carried both spellings hits the primary key and is ignored), then
   `DELETE FROM image_tags WHERE tag_id = loser`, then `DELETE FROM tags WHERE id = loser`.
5. `UPDATE tags SET name = canonical, category, pinned WHERE id = winner`.

The migration's real number, confirmed when task 1.2 landed, is `MIGRATIONS.len()` = **v9** —
the planned number held, since no other change reached `db.rs` first.

*Why not SQL:* `lower()` is ASCII in SQLite, and the merge (moving links, choosing a
category) is a loop over groups, which SQL expresses badly and Rust in twenty lines that
share the one `canonical` function.

*Sidecars and `library.json` are not rewritten by the migration.* A per-image sidecar may
still say `Tagme`; a rebuild reads it through `link_tag`, which canonicalises, so the file is
harmless and the next edit of that image rewrites it. `library.json`'s `tags` exceptions
are restored through the canonicalising upsert (D1) and rewritten by the next categorised
write.

### D3. The webview lowercases its own copy of the query, for its own comparisons

`parseTagSearch` lowercases `includeTags`, `excludeTags` and `orGroups` members. Everything
that compares a query term to a stored tag — `activeTerms`, `removeTagFromQuery`,
`toggleTagInQuery` — already reads through it or through the same token walk; the ones that
compare tokens as typed (`removeTagFromQuery`'s token match) compare lowercased. The query
*string* the user typed is not rewritten (`Cat` stays in the search field; the rewrite
helpers keep their tokens), only the parsed view is canonical. Rust canonicalises again on
its side (D1): two runtimes each apply the rule to their own copy, as they already do for
the metatag alphabet.

`parseStamp` lowercases the tag tokens of `add` and `remove` (it already lowercases the
collection slug), so a chip's title and the hover label show the text Rust will write.

## Risks / Trade-offs

- [A tag with a non-ASCII capital that only `to_lowercase` folds] → both runtimes fold it
  the same way; the migration folds it too since it runs in Rust.
- [The merge picks a category the owner did not mean] → the rule (non-general wins, first by
  id) is stated in the migration's doc comment; the owner's library today holds exactly the
  `Tagme`/`tagme` pair the proposal names, for which the rule keeps `meta`.
- [`has_any_tag` once relied on BINARY being "the case-sensitive `Array.includes`" of the
  legacy rule] → the legacy rule is superseded by the owner's 2026-09-23 decision; the
  comment names the new rule.
