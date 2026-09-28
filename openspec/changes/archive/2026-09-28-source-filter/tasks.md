> One unit, Sonnet (parser and compile, each with a test gate). Runs after
> `text-search-substring` has landed: both edit `query.rs`, and D3 reuses its `like_literal`.
> Design D1–D5 decide every shape; do not re-decide them. Gate: `mise run check`. No unit
> commits or ticks a hand check.

## 1. Unit S — the `source:` metatag (`packages/shared`, `packages/app`, `packages/app/src-tauri`)

- [x] 1.1 `packages/shared/src/index.ts`: `sources`, `excludeSources`, `noSource`, `anySource`
      on `ParsedTagSearch` per D1, doc comments carrying the Danbooru rule; `sourceUrl` on
      `ImageRecord` per D2. `model.rs` mirrors both.
- [x] 1.2 `tag-utils.ts`: the `source:` step per D1, before the tag tokenizer and before
      `collection:`; the header comment's "Supports:" line names it. Tests in
      `tag-parser.test.ts`: `source: reads a URL prefix as typed` (`source:https://x.com/*` →
      `sources: ['https://x.com/*']`, no tag), `-source: excludes`, `two source terms both
      apply`, `source:none and -source:none set the flags` (case-insensitive `NONE`), `a bare
      source: is source:none`, `source: with a comma keeps the comma in the value`, `a source:
      term leaves the tags around it alone` (`cat source:https://a.b/x dog`).
- [x] 1.3 `stamp.ts` and `tag-input.ts` per D4. Tests: `stamp.test.ts` `source:x is not an
      edit`; `tag-input.test.ts` `suggestionPrefix('source:https://x', …)` is `null`.
- [x] 1.4 `query.rs`: `SOURCE_URL_SQL` per D2 with the "not the `source` column" doc, added to
      `MATCHED_COLUMNS` and read into the record; `push_sources` per D3. Tests (fixture rows
      already carry page URLs; add one with an image URL and no page URL, and one with neither):
      `source_matches_a_prefix` (`https://example.test` finds the `example.test/gallery` row),
      `source_star_is_a_wildcard` (`https://*.test/gall*` — the `fanbox` shape),
      `source_ignores_ascii_case` (`HTTPS://EXAMPLE.TEST/*`), `source_underscore_is_literal`
      (a `_` in the value does not match a row differing at that character),
      `source_falls_back_to_the_image_url`, `source_none_finds_images_with_no_source`,
      `excluding_a_source_keeps_images_with_none`, `none_and_any_source_together_match_nothing`,
      `search_returns_the_source_url` (page URL when present, else image URL, else `None`).
- [x] 1.5 `upload-form.ts` per D2, its test updated to the `sourceUrl` field.
- [x] 1.6 Gate green. Handoff below: final field names and anything the hand check should
      watch.
- [ ] 1.7 Hand check (owner): on the real library, `source:https://x.com/*`,
      `source:https://*fanbox.cc/*`, `-source:https://x.com/*`, `source:none`; open Upload on an
      X capture and a local import and read the Source field.

## Handoff

- **Wire shapes.** `ParsedTagSearch` gains `sources: string[]`, `excludeSources: string[]`,
  `noSource: boolean`, `anySource: boolean` (`Vec<String>`/`Vec<String>`/`bool`/`bool` in
  `model.rs`, same camelCase names). `ImageRecord` gains `sourceUrl: string | null`
  (`source_url: Option<String>` in Rust). Every existing literal `ParsedTagSearch` in the repo
  built the four new fields for free through `..Default::default()` except one:
  `packages/app/src/lib/api/commands.test.ts`'s hand-written `SearchRequest` fixture, which I
  extended with `sources: [], excludeSources: [], noSource: false, anySource: false` — a
  mechanical follow of D1's shape, not a design call.
- **`SOURCE_URL_SQL`** (`packages/app/src-tauri/src/query.rs`, `pub(crate) const`):
  `COALESCE(images.page_url, images.image_url)`. `push_sources` compiles against it directly
  (patterns via a new `source_pattern()` helper: split on `*`, `like_literal` each segment,
  join on `%`, append a trailing `%`). Compiled clauses:
  - include: `SOURCE_URL_SQL LIKE ? ESCAPE '\'`, one per term in `sources`, ANDed.
  - exclude: `(SOURCE_URL_SQL IS NULL OR SOURCE_URL_SQL NOT LIKE ? ESCAPE '\')`, one per term in
    `excludeSources`, ANDed — keeps sourceless images, the `-account:` rule.
  - `noSource`: `SOURCE_URL_SQL IS NULL`. `anySource`: `SOURCE_URL_SQL IS NOT NULL`. Both set:
    two contradictory clauses ANDed, matches nothing — no special case, same as
    `anyCollection`/`noCollection`.
- **Record wiring, deviation from the design's literal wording.** D2 says "`MATCHED_COLUMNS`
  selects it as `source_url`, which the page load puts on `ImageRecord.source_url`." That is
  not where the page load actually reads a row from: `MATCHED_COLUMNS` (query.rs) only feeds
  the `matched`/`grouped`/`slices` CTEs that back paging, sorting, counts and `x_account`
  grouping — it is never read into `ImageRecord` (confirmed by tracing `search()`: ids come
  from the `Plan`, then `ingest::load_records` re-reads full rows separately, through its own
  `IMAGE_COLUMNS`/`row_to_record`, the same place `account` is derived from `page_url` today).
  I did not add `SOURCE_URL_SQL` to `MATCHED_COLUMNS` — nothing reads it from there and it
  would be dead weight. Instead I put it where `account` already lives: `ingest.rs`'s column
  list, which I turned from `pub const IMAGE_COLUMNS: &str` into `pub fn image_columns() -> String`
  (it had exactly one call site, `load_records`) so it can interpolate
  `crate::query::SOURCE_URL_SQL` — the one expression, not a second copy hand-typed into
  `ingest.rs`. The column is appended last (design D11's own convention: appended, never
  slotted in), so `row_to_record` reads it at index 19 into `ImageRecord.source_url`. This
  keeps D2's actual guarantee (filter and display read the same SQL) while fixing what its
  prose named as the mechanism; nothing in D1–D5's decided *shape* changed.
- **Parser regex, deviation from the design's literal wording.** D1 specifies
  `/(-?)source:(\S+)/gi`. With `\S+` a bare `source:` (nothing before the next space or the
  end) cannot match at all — the regex requires at least one non-whitespace character after
  the colon — so it would fall through to the tag tokenizer as a literal tag `source:`, not
  read as `source:none` the way D1's own prose says it must ("a bare `source:` ... reads as
  `source:none`"). I implemented the value group as `\S*` instead, so an empty capture is
  itself a term the loop reads as the `none` keyword. Every other case (a real value, `none`,
  `NONE`, exclusions) behaves exactly as D1 describes; this only fixes the one input the stated
  `\S+` couldn't reach, matching D1's own stated invariant over its literal regex text.
- **Parse order**: `source:` is step 5 in `parseTagSearch` (`tag-utils.ts`), after `account:`
  and before `collection:none`/`any` (now 6a) and the collection slug list (now 6b) and the tag
  tokenizer (now 7) — before both, as D1's risk note requires. The `Supports:` header comment
  now lists `source:`.
- **`upload-form.ts`**: `prefillUploadForm` now reads `source: image.sourceUrl ?? ''` and no
  longer derives from `pageUrl`/`imageUrl` itself; the "never a local path" reasoning that used
  to sit on that line now lives on `SOURCE_URL_SQL`'s own doc comment in `query.rs`, per D2.
  `image-fixture.ts`'s `img()` fixture (used by many unrelated tests) gained a default
  `sourceUrl: 'https://example.com/page'` matching its existing default `pageUrl` — needed
  because `ImageRecord.sourceUrl` is non-optional; no other fixture or literal `ImageRecord`
  construction in the app needed a change (checked every `ImageRecord {` in Rust and every
  full-object `img()`-free construction in TS).
- **Gate**: `mise run check` exit 0. Rust: 853 passed, 0 failed, 1 ignored (pre-existing,
  unrelated), in `packages/app/src-tauri` (was 844 before this unit; +9 `source_*`/
  `*_source_*` tests in `query.rs`). JS: `packages/shared` 1 passed, `packages/extension` 96
  passed, `packages/app` 845 passed (was 837; +7 in `tag-parser.test.ts`'s new `source filters`
  describe block, +1 in `stamp.test.ts`). Typecheck 0 errors/warnings across 1240 files. Lint
  and `cargo fmt --check` clean. Clippy clean. All builds succeeded.
- **Hand check** (owner, task 1.7, not ticked here): on the real library, try
  `source:https://x.com/*`, `source:https://*fanbox.cc/*`, `-source:https://x.com/*`,
  `source:none`, and confirm each narrows the grid the way its name says. Open Upload on an X
  capture and confirm Source prefills the page URL; open Upload on a local import (no page, no
  captured address) and confirm Source is empty rather than a filesystem path. If any library
  image predates this schema in a way that leaves both `page_url` and `image_url` null, confirm
  it shows up under `source:none` and is not silently excluded from every search.
