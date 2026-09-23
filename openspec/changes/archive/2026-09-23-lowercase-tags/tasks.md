> One unit, agent A (Sonnet): `packages/app/src-tauri` plus two pure webview modules.
> Design D1–D3 decide every shape; do not re-decide them. Gate: `mise run check` green.
> The migration's number is `MIGRATIONS.len()` when task 1.2 lands: amend design D2's
> sentence to it. Do not commit; the lead commits by file set. Do not tick a hand check.

## 1. Unit A — the rule, the doors, the migration (`packages/app/src-tauri`)

- [x] 1.1 `tags.rs`: `canonical` per D1 with its doc comment; called from `read_metatags`
      (prefix comparisons simplified to `==`), `link_tag`, `remove_tags`, `set_category`,
      `set_pinned`, `selection_tag_counts`'s names filter, `suggestions`' prefix;
      `rules.rs` where a rule's tags enter; `recover.rs`'s sidecar link and vocabulary upsert;
      `query.rs`'s tag clauses (`has_any_tag`'s comment rewritten to the rule). Tests: an
      editor save of `Cat Tagme` stores `cat` and `tagme`; `artist:Tagme` with `meta:tagme`
      present is refused with the conflict message naming `tagme`; `artist:Tagme` on a
      library without `tagme` creates `tagme` under artist; a rule text `Tagme` writes
      `tagme`; a sidecar naming `Tagme` restores `tagme` on rebuild; a `library.json` entry
      `Tagme` categorises `tagme`; a search `Cat` (include, exclude, and inside an or-group)
      matches the images tagged `cat`; `set_category("Tagme", …)` changes `tagme`.
      Verify: `cargo test tags:: query:: recover:: rules::` pass, `mise run clippy` clean.
- [x] 1.2 `db.rs`: `Migration` enum and the runner per D2; `merge_case_duplicates` with its
      doc comment stating the winner rule. Tests: a v8 fixture with `Tagme` (artist), `tagme`
      (meta, pinned) and `TAGME` (general), three images tagged one each and one tagged two,
      migrates to one row `tagme`, meta (the first non-general by id — build the fixture so
      the expected category is unambiguous), pinned, carried by all four images, with no
      dangling `image_tags` row; a library with no duplicates is unchanged but for the
      version; the existing "fresh database is at `MIGRATIONS.len()`" tests still pass.
      Verify: `cargo test db::` pass.
- [x] 1.3 `packages/app/src/lib/domain/tag-utils.ts` and `stamp.ts` per D3 with tests:
      `parseTagSearch('Cat -Dog {A ~ b}')` yields lowercase tag arrays while `rating:`,
      `is:` and `account:` terms are unchanged in behaviour; `activeTerms('Cat')` marks
      `cat`; `toggleTagInQuery('Cat dog', 'cat')` yields `dog`; `parseStamp('Cat -Dog')`
      yields `add: ['cat']`, `remove: ['dog']`. Verify: `pnpm --filter @boorubox/app test
      tag-utils stamp` pass; `mise run check` green.
      Hand check: with the owner's library open, the sidebar shows one `tagme` (meta) and no
      `Tagme`; typing `Cat` in the search finds the `cat` images and marks `cat` in both
      panels; saving `artist:Tagme` on an image is refused naming `tagme`.
      Seen by the lead on a scratch copy of test-1 (smoke run 2026-09-23): the copy opened at
      v8 and came up at v9 with no error in the dev log; `Bride`/`Cicero`/`Lorem` read
      lowercase in the sidebar; saving `Cat DOG` stored `cat` and `dog`; `artist:Kantoku`
      folded into the existing `kantoku` (one row); `meta:kantoku` was refused naming
      `kantoku`.

## Handoff

**Landed** (all three tasks, `packages/app/src-tauri/src` + the two owned webview modules):

- `tags.rs`: `canonical(name) -> String` (`trim().to_lowercase()`), called from
  `read_metatags` (category-prefix and rating-prefix comparisons simplified
  from `eq_ignore_ascii_case`/`to_ascii_lowercase` to plain `strip_prefix`,
  since the token is canonical before either check runs), `link_tag` (the one
  row birth — this alone covers `recover::insert_sidecar`'s sidecar link, no
  edit needed there), `remove_tags`, `set_category`, `set_pinned`,
  `selection_tag_counts`'s `names` filter, `suggestions`' `prefix`.
- `rules.rs` and `ingest.rs` needed **no direct edit**: both route a rule's or
  a capture's tags through `tags::read_metatags` already, so canonicalising it
  once covers both doors. Added a rules.rs test proving a rule text `Tagme`
  writes `tagme`.
- `recover.rs`: `insert_vocabulary` now canonicalises `entry.name` before the
  upsert (the sidecar link needed nothing, see above, since it goes through
  `link_tag`). Added two tests: a sidecar naming `Tagme` restores `tagme`; a
  `library.json` vocabulary entry naming `Tagme` categorises `tagme`.
- `query.rs`: `push_tags` canonicalises `include_tags`, `exclude_tags` and
  every `or_groups` member via a new `canonical_tag_values` helper;
  `has_any_tag`'s doc comment now states the lowercase-tags rule instead of
  the old BINARY-collation-as-case-sensitive-`Array.includes` one. Added a
  search test for a capitalised include/exclude/or-group term.
- `db.rs`: `MIGRATIONS` is now `&[Migration]` with `enum Migration { Sql(&str),
  Rust(fn(&Connection) -> rusqlite::Result<()>) }`; the runner matches on the
  variant. New step `merge_case_duplicates` (v9, the real number —
  `MIGRATIONS.len()` — design D2's sentence amended below) groups every tag
  row by `tags::canonical(name)`, picks a winner (the already-canonical row if
  one exists, else lowest id), folds category (first non-general in id order)
  and pin (any), remaps `image_tags` with `UPDATE OR IGNORE` then deletes the
  losers. Two new tests: the three-spelling/four-image worked example
  (including a `d` that already carries two spellings, to exercise the
  collision-tolerant remap), and a no-duplicates library left unchanged but
  for the version.
- `tag-utils.ts`: `parseTagSearch` lower-cases `includeTags`, `excludeTags`
  and every `orGroups` member at the point each token is classified;
  `removeTagFromQuery` and `removeExclusionFromQuery` (the latter not named in
  the design's D3 paragraph, but the same class of bug — kept it consistent
  since the query string itself is never rewritten) now compare tokens
  case-insensitively against the already-lowercase `tag`/`tagToRemove`
  argument. `rating:`/`is:`/`account:`/`collection:` extraction untouched.
- `stamp.ts`: `parseStamp` now pushes `lower` (not `token`) into `add`, and the
  existing `withoutSign` (already `lower` minus a leading `-`) into `remove`
  instead of recomputing from the raw token — no new logic, reused what was
  already there.
- Tests added per the task list's exact scenarios, in `tags.rs`, `db.rs`,
  `rules.rs`, `recover.rs`, `query.rs`, `tag-parser.test.ts`,
  `tag-query.test.ts`, `stamp.test.ts`. Two pre-existing tests were changed
  (not deleted) because the spec explicitly retires the old behaviour they
  pinned: `read_metatags_reads_categories_from_their_prefixes_alongside_the_rating`
  (expected `Cat` verbatim, now `cat`) and `a_prefix_matches_whatever_case_the_tag_was_stored_in`
  (renamed, now expects `cathedral` and also checks a capitalised prefix).
  `stamp.test.ts`'s "keeps a category prefix in add, verbatim" was renamed
  (dropped "verbatim", added a capitalised-prefix case) for the same reason.

**Real migration number:** v9 (`MIGRATIONS.len() == 9`). Design D2's sentence
"**v9** (planned by queue position)" is confirmed correct as landed — no
amendment needed, the plan and the real number agree.

**Deviations from the design:** none in shape. One clarification: D1 lists
`recover.rs`'s "sidecar link and vocabulary upsert" as both needing D1's rule
applied — only the vocabulary upsert needed an actual code change, since the
sidecar link already goes through `tags::link_tag`, which is itself one of
D1's listed call sites and canonicalises internally.

**Gate:** `mise run check` from the repo root — lint (cargo fmt clean after a
local `cargo fmt` run; the one pre-existing ESLint warning on
`CollectionsSection.svelte:190` is agent B's file, untouched here), typecheck
(0 errors), test (all packages, `packages/app` 54 files including the new
tag-utils/stamp/tags/db/query/recover/rules cases), clippy (clean), build
(app + extension) — all green. `cargo test --lib` alone: 648 passed, 1
ignored (pre-existing), 0 failed.

**Open items:** none for unit A. The Hand check in task 1.3 is left unticked
for the owner, as instructed:
> with the owner's library open, the sidebar shows one `tagme` (meta) and no
> `Tagme`; typing `Cat` in the search finds the `cat` images and marks `cat`
> in both panels; saving `artist:Tagme` on an image is refused naming
> `tagme`.

**Review fixes on `dbcb7e3`:** `merge_case_duplicates` (`db.rs`) now takes the
winner's own category when it is non-general, falling back to the first
non-general category among the *losers* in id order, general otherwise —
the previous code took the first non-general category across the whole
group (winner included) in id order, so a lower-id loser could outrank the
winner's own category. `recover.rs`'s `insert_vocabulary` follows the same
winner rule for a `library.json` listing two case-variant entries for one
tag: entries are stable-sorted so the one already spelled canonically (if
any) is upserted last, and its non-general category and its pin (`OR`,
never cleared) win regardless of the list's order — mirrored by two tests,
the pair in both orders. Also: a non-ASCII-capital Rust and TS test each
(`tags::read_metatags`, `parseTagSearch`) and a migration `Ä`/`ä` fixture;
`suggestions`' LIKE comment reworded; `query.rs`'s `canonical_tag_values`
now spells `tags::canonical` through the module's own `use`, matching
`push_tags`; `tag-utils.ts` folds its argument's case in `isIncluded`,
`addTagToQuery` and `excludeTagFromQuery` (a `fold` helper, reused by
`removeTagFromQuery`/`removeExclusionFromQuery` too).
