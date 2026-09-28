> One unit, Sonnet (Rust with a test gate). Design D1–D5 decide every shape; do not re-decide
> them. The migration's number is whatever `MIGRATIONS.len()` is when the unit lands (planned
> v14; 13 today): amend design D2's sentence to the real number, never pin the planned one.
> Gate: `mise run check`. No unit commits or ticks a hand check.

## 1. Unit T — substring free text (`packages/app/src-tauri`)

- [x] 1.1 `db.rs`: `SCHEMA_V14` per D2 with a doc comment that says why the table is recreated
      (trigram) and why the triggers survive; appended to `MIGRATIONS`. Test, in the style of
      the migration tests already there: a database at the previous version holding an image
      titled `かにビーム的插畫` migrates to `MIGRATIONS.len()` and `images_fts MATCH '"ビーム"'`
      returns its row.
- [x] 1.2 `query.rs`: `like_literal` and the short-text branch of `push_text` per D3; the
      module doc's "FTS5 free-text match" sentence says substring. Tests beside the existing
      free-text ones (fixture gains one image titled `かにビーム的插畫`):
      `free_text_matches_inside_a_cjk_title` (`かにビーム` and `ビーム` both find it),
      `free_text_matches_inside_a_word` (`yoto` finds `cat-s`),
      `free_text_ignores_case` (`KYOTO`),
      `free_text_of_two_characters_matches_by_like` (`ky` finds `cat-s`; `かに` finds the CJK
      row; `k_` finds nothing — `_` is data),
      and the existing `free_text_*` tests, `text_with_no_searchable_word_is_not_an_error`
      (`--` is two characters: the `LIKE` branch, no rows) and `blank_text_is_no_clause_at_all`
      still pass.
- [x] 1.3 Gate green. Handoff below: the real migration number, `like_literal`'s signature for
      `source-filter`, anything the hand check should watch.
- [ ] 1.4 Hand check (owner): on the real library, `かにビーム`, `ビーム` and a two-character
      query in the Title or URL box; first open after the update takes a moment while the index
      rebuilds.

## Handoff

- **Migration number**: v14 (`MIGRATIONS.len()` was 13 before this unit, 14 after — matches
  the plan, no amendment needed to D2's sentence beyond what it already says). `SCHEMA_V14` in
  `packages/app/src-tauri/src/db.rs` drops and recreates `images_fts` with
  `tokenize = 'trigram'` and rebuilds it; the three v1 triggers are untouched and survive the
  drop (confirmed by the new migration test).
- **`like_literal`**: `pub(crate) fn like_literal(text: &str) -> String` in
  `packages/app/src-tauri/src/query.rs`. Escapes `\`, `%` and `_` with a `\` prefix (backslash
  first, so escaping the other two never double-escapes); does not add `%` wildcards — the
  caller wraps the result in whatever pattern it wants. `push_text`'s short-text branch wraps it
  as `%...%` for "contains"; `source-filter` is free to map its own `*` to `%` before wrapping.
  Binds the escaped pattern into a `page_title LIKE ? ESCAPE '\' OR page_url LIKE ? ESCAPE '\'
  OR image_url LIKE ? ESCAPE '\'` clause — three separate anonymous `?` placeholders, each bound
  the same value, rather than one `?1` reused three times: the rest of `Filter`'s clauses use
  plain anonymous `?` bound positionally (`params_from_iter`), and mixing in an explicit `?1`
  inside one clause would renumber every other clause's implicit placeholders sharing the same
  compiled statement. Same behaviour as D3's SQL, different binding mechanics.
- **Deviation from design**: D4 says `free_text_is_data_not_fts_syntax` "still holds"
  unmodified. It does not, for one line only. That test's last assertion searched
  `text_request("\"kyoto\"")` (the user typing a literal pair of `"` around the word) and
  expected the same match as unquoted `kyoto`, because `unicode61` treats `"` as punctuation and
  drops it during tokenization. `trigram` indexes every character, quotes included (confirmed
  against a scratch `sqlite3` database: `images_fts MATCH '"""kyoto"""'` — the phrase content
  `"kyoto"` with real quote characters — returns nothing over a row that only contains `kyoto`
  without quotes). The wrapping quoting (`fts_string`) still makes arbitrary text safe — the
  test's for-loop of operator-like inputs (`-kyoto`, `kyoto:`, `kyo*`, …) still passes, still no
  syntax error — but a *typed* quote pair is no longer swallowed as punctuation, it is now
  literal substring data that has to actually appear in the title or URL. I changed the final
  assertion to expect an empty result (with a comment explaining why) rather than `["cat-s"]`.
  This is a correction to the design's stated invariant, not a re-decision of D1/D2/D3: the
  substring-matching behaviour is exactly what D5 asks for, and this is the one place its
  consequence touches an existing test.
- **Everything else in D1–D5 held as written**: trigram tokenizer, char-count threshold at 3
  (`chars().count()`, not bytes), `fts_string` unchanged, `LIKE`'s ASCII-only case folding
  accepted as-is.
- **Fixture**: did not add the CJK-titled image to the shared `fixture()` — that function is
  called by every test in `query.rs`, many asserting an exact total, and a sixth row would
  silently shift every one of them. Added `fixture_with_cjk_title()` instead (calls `fixture()`
  then one extra `store()`, id `crab-beam`, title `かにビーム的插畫`), following the existing
  pattern of tests that build their own one-off `Fixture` on top of `store()` directly (e.g.
  `sortable()`, `paging_a_sort_whose_values_are_all_equal_shows_every_row_once`). Only the tests
  that need the CJK row call it.
- **Gate**: `mise run check` exit 0. Rust: 844 passed, 0 failed, 1 ignored (pre-existing,
  unrelated) in `packages/app/src-tauri`. JS: `packages/shared` 1 passed, `packages/extension`
  96 passed, `packages/app` 837 passed. Typecheck 0 errors/warnings across 1240 files. Clippy
  clean. All builds succeeded.
- **Hand check** (owner, 1.4, not ticked here): confirm on the real library that `かにビーム`,
  `ビーム`, and a two-character query (e.g. `AI` or `ky`) all find matches in the Title or URL
  box, and that first open after the update takes a visible moment while `images_fts` rebuilds
  from `images` — the library's existing title/URL text, not a small amount.
