## Context

`images_fts` is an FTS5 external-content table over `page_title`, `page_url`, `image_url`
(`SCHEMA_V1` in `db.rs`), kept in step by three triggers on `images` that name the table.
`query::push_text` wraps the typed text as one FTS5 string literal (`fts_string`) and adds
`images.rowid IN (SELECT rowid FROM images_fts WHERE images_fts MATCH ?)`; every search entry
point (`search`, `search_ids`, `search_position`, `matching_ids`, the counts) compiles through
the same `Filter`, so one clause covers them all. `MIGRATIONS` has 13 entries today.
`db::rebuild_fts` issues the `rebuild` command. The bundled SQLite (libsqlite3-sys 0.38) is far
past 3.34, where the trigram tokenizer arrived.

## Decisions

**D1. Trigram, not prefix and not plain `LIKE`.** `"text"*` (a prefix query) is a one-line
change with no migration, but `ビーム` inside `かにビーム` still misses. Plain `LIKE` over the
three columns would be fast enough at this library's size, but requirements §7 locks FTS5 for
title/URL text and the trigger machinery already exists; trigram keeps the lock and gives
substring matching everywhere, case-insensitive under FTS5's own Unicode folding. Index cost is
about three times the indexed text — titles and URLs of a 25,000-image library are a few
megabytes.

**D2. One SQL migration recreates the table.** Planned as v14 (`MIGRATIONS.len()` is 13
today; the real number is whatever it is when this lands — amend this sentence, never pin
it): `DROP TABLE images_fts; CREATE VIRTUAL TABLE images_fts USING fts5 (page_title,
page_url, image_url, content = 'images', content_rowid = 'rowid', tokenize = 'trigram');
INSERT INTO images_fts (images_fts) VALUES ('rebuild');`. The triggers are on `images` and
name `images_fts`; SQLite does not bind a trigger body to the table it names until it runs, so
they survive the drop and write to the new table. A library rebuilt from sidecars runs every
migration from v1, so it gets the tokenizer the same way. `SCHEMA_V1` is not edited: the rule
in `db.rs` is that a shipped schema is never rewritten.

**D3. Under three characters, `LIKE`.** FTS5's trigram tokenizer returns no rows for a MATCH
of fewer than three characters, by design. `push_text` counts characters (`chars().count()`,
not bytes — `かに` is six bytes) and for a count below three adds
`(page_title LIKE ?1 ESCAPE '\' OR page_url LIKE ?1 ESCAPE '\' OR image_url LIKE ?1 ESCAPE '\')`
with `%` + `like_literal(text)` + `%`. `like_literal(text) -> String` escapes `\`, `%` and
`_` with `\`, so a typed `_` or `%` is data — the same rule as `fts_string`'s for FTS syntax.
`LIKE`'s case folding is ASCII-only; for a two-character query that is accepted and said in the
doc comment. `source-filter` reuses `like_literal` with `*` mapped to `%`.

**D4. `fts_string` stays.** Trigram changes what a phrase matches, not what is syntax: the text
is still one quoted literal, and `free_text_is_data_not_fts_syntax` still holds for every
operator-shaped input. One consequence, found while implementing: `unicode61` dropped a typed
`"` as punctuation, so `"kyoto"` with quotes matched `kyoto`; trigram indexes every character,
so a typed quote is substring data and matches only a title or URL that contains a quote. That
is the "contains the text" rule applied to the quote itself, and the test's last assertion now
expects no match.

**D5. The spec says "contains".** `library-browse`'s "Free text in `page title` and URLs SHALL
be searchable" becomes: free text SHALL match an image whose title or URL contains the text,
ignoring case. The archived phase-1 design that chose FTS5 needs no amendment: it chose the
engine, not the tokenizer.

## Risks

- A short common substring (`the`) now matches most English titles. That is what a find box
  does; the tag query is the precise instrument.
- The migration rebuilds the index on first open after update; on a 25,000-image library that
  is a moment, not minutes (three short text columns).
