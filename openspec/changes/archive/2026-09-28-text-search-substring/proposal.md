## Why

Typing `かにビーム` into the Title or URL box finds nothing, while `かにビーム的插畫` finds the
image (owner, 2026-09-28). Free text goes to FTS5 with its default `unicode61` tokenizer, which
splits on whitespace and punctuation only: a CJK run is one token, and a phrase query matches
whole tokens. The same rule makes `yoto` miss `kyoto`. A box labelled "Title or URL" is a
find-in-text box, and there is no title or URL search where whole-word wins over contains. The
owner's first thought, a "no boundary" checkbox, is withdrawn: two search modes to explain, for
a case the substring rule covers outright. Requirements §6 (free text on title/URL) and §7
(SQLite + FTS5 locked, which this keeps).

## What Changes

- **The free-text index uses FTS5's `trigram` tokenizer**: a search matches any image whose
  title or URL contains the text, ignoring case, CJK included. One schema migration recreates
  `images_fts` with the tokenizer and rebuilds it from `images`.
- **Text shorter than three characters falls back to `LIKE`**: a trigram index cannot answer a
  one- or two-character query, and the box must not go dead for `AI`.
- **A `LIKE` pattern helper lands in `query.rs`**, escaping `%`, `_` and `\`, which the
  `source:` filter (`source-filter`, next in the queue) reuses.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `library-browse`: what free text matches.

## Non-goals

- A search-mode toggle (owner withdrew it, 2026-09-28).
- Matching tag names through the box: tags have the tag query.
- Diacritic folding (`remove_diacritics` on trigram): no case has asked for it.
- Relevance ranking: the result keeps the current sort.
