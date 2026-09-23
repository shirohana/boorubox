> One unit, three commits, one Sonnet agent (retry on Opus if the gate fails). The files are
> not disjoint by package — the wire shape change breaks the webview typecheck until the parser
> follows it — so each commit crosses `packages/shared`, `packages/app/src-tauri` and
> `packages/app` together and is green on its own. Design D1–D6 decide every shape; do not
> re-decide them. Gate for every commit: `mise run check` green. No unit ticks a hand check.
> No migration in this change. Lands before `auto-artist-tag` and `tag-category-visibility`.

## 1. Unit Q — count terms and collection membership (`packages/shared`, `packages/app/src-tauri`, `packages/app`)

- [x] 1.1 Count terms on the wire and in the compiler, per D1–D3. `packages/shared`:
      `TagCountTerm`, `ParsedTagSearch.tagCount` → `tagCountTerms`. `src-tauri/src/model.rs`:
      `TagCountTerm`, `tag_count` → `tag_count_terms`, camel-case wire test
      `a_tag_count_term_crosses_the_wire_in_camel_case` (a `null` category and an `"artist"`
      one). `src-tauri/src/query.rs`: `CATEGORY_TAG_COUNT`, `push_tag_count` over the list with
      the leading category parameter, `push_tag_count_comparison` taking the expression; the
      existing `tag_count_*` tests re-pointed through `tag_count_ids` (now building a
      one-entry list), plus new tests over a fixture whose tags get categories through
      `tags::set_category`: `a_category_count_of_zero_finds_images_without_that_category`
      (`copytags:0`), `a_category_count_greater_than_zero` (`arttags:>0`),
      `a_category_count_range_binds_the_category_before_min_and_max` (`chartags:1..2`),
      `a_category_count_list` (`gentags:0,1`), `two_count_terms_both_apply`
      (`copytags:0` + `chartags:>0`), `a_category_count_without_its_operand_matches_nothing`.
      `packages/app/src/lib/domain/tag-utils.ts`: `COUNT_METATAGS` and step 1 per D2.
      `tag-parser.test.ts`: every `tagCount` assertion becomes `tagCountTerms`; the operator
      table runs under `it.each` for each of the six names with its category; new cases
      "combines different count metatags" (`copytags:0 chartags:>0` → two terms in table
      order), "keeps the first of a repeated count metatag" (`copytags:0 copytags:>0`, both
      stripped, no tag terms), "keeps the first across forms" (`tagcount:5 tagcount:1,3` →
      `=5`), "drops a leading minus" (`-copytags:0` → `copytags:0`, no exclusion),
      "`COPYTAGS:0` reads case-insensitively". `api/commands.test.ts`: the literal's
      `tagCountTerms: []`. Verify: `cargo test query:: model::` and
      `pnpm --filter @boorubox/app test tag-parser commands` pass; `mise run check` green.
- [x] 1.2 `collection:none` / `collection:any`, per D4–D5. `packages/shared` and `model.rs`:
      `anyCollection` / `noCollection` (`any_collection` / `no_collection`), wire test extended.
      `query.rs`: `IN_SOME_COLLECTION` in `push_collections`; tests
      `collection_none_finds_images_in_no_collection`,
      `collection_any_finds_images_in_some_collection`,
      `none_and_any_together_match_nothing`, `none_with_a_slug_matches_nothing`.
      `tag-utils.ts`: exported `COLLECTION_KEYWORD` run before the slug regex, `none`/`any`
      dropped inside a slug list, `rewriteMetatagList`'s `keep` predicate passed by the three
      collection rewriters. `tag-parser.test.ts`: `collection:none` → `noCollection`, no slug;
      `collection:any` → `anyCollection`; `-collection:none` → `anyCollection`;
      `-collection:any` → `noCollection`; `COLLECTION:None` case-insensitive;
      `collection:none_left` is a slug; `collection:cute,none` → slugs `['cute']`, no flag.
      `tag-query.test.ts`: `excludeCollectionFromQuery('collection:none', 'queue')` →
      `collection:none -collection:queue`; `toggleCollectionInQuery` and `addCollectionToQuery`
      keep `-collection:any` standing; `activeTerms('collection:none').collections` is empty.
      Verify: `cargo test query::` and `pnpm --filter @boorubox/app test tag-parser tag-query`
      pass; `mise run check` green.
      Hand check: search `collection:none` — only images in no collection; add one of them to
      Favorites from the tile menu and refresh — it leaves the result; exclude Queue from the
      Collections list — the field reads `collection:none -collection:queue`; search
      `collection:any` — the complement; `-collection:none` — the same as `collection:any`.
- [x] 1.3 Follow-throughs, per D6 (`packages/app`). `domain/stamp.ts`:
      `SEARCH_ONLY_METATAGS` gains the five names with a comment naming `COUNT_METATAGS`;
      `stamp.test.ts`: `touhou copytags:0` → error naming `copytags:0`, and each of the other
      four names refused. `domain/tag-input.ts`: `METATAG` gains the five names;
      `tag-input.test.ts`: `suggestionPrefix('copytags:', 9)` and `suggestionPrefix('arttags:>',
      9)` are `null`, `gentags` alone (no colon) still suggests. Verify:
      `pnpm --filter @boorubox/app test stamp tag-input` passes; `mise run check` green.
      Hand check: search `copytags:0` on a library with some copyright tags — only images
      without one; `copytags:0 chartags:>0` narrows further; typing `copytags:` in the search
      field opens no suggestion list; a stamp `touhou copytags:0` is refused under the field
      naming `copytags:0`.
