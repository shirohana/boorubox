## Context

The query language is parsed once, in `$lib/domain/tag-utils.ts` (`parseTagSearch`), into
`ParsedTagSearch` (`packages/shared/src/index.ts`, mirrored by `model.rs` with
`rename_all = "camelCase"`); Rust never sees the query string and only compiles the parsed
shape (`query.rs`, one `push_*` per clause). `account:` is the template: a regex read off the
remaining text before the tag tokenizer, a comma list either side, `accounts` and
`excludeAccounts` on the shape, `push_accounts` compiling both. `stamp.ts` lists search-only
metatags so a stamp refuses them; `tag-input.ts`'s `METATAG` shuts the completion list behind
one. `ImageRecord.account` is the precedent for a field derived at load time from a column so
the filter and the display cannot disagree. `text-search-substring` lands first and leaves
`query::like_literal` (escapes `\`, `%`, `_`) behind.

## Decisions

**D1. Grammar: `/(-?)source:(\S*)/gi`, read before the tag tokenizer, every occurrence.**
(`\S*`, not `\S+`: a bare `source:` has to match so it can read as `source:none`, below;
with `+` it fell through to the tag tokenizer as a tag named `source:`.)
The value runs to whitespace, so a URL's `:`, `/`, `@`, `?` and `,` are all part of it; no
comma list. Each term is one entry in `sources` or `excludeSources` (both `string[]`, kept as
typed); several include terms all apply, as Danbooru's do. A bare `source:` (empty value) reads
as `source:none`, which is what Danbooru does with an empty value. `none`, compared ignoring
case, is a keyword only as the whole value: `source:none` sets `noSource`, `-source:none` sets
`anySource`, both `bool`, and both set compiles to a clause matching nothing (the
`collection:none`/`any` convention). The regex makes `\S+` cover `none` too, so the parser
decides after the match, not in it.

**D2. The value: `COALESCE(images.page_url, images.image_url)`, once.** `SOURCE_URL_SQL` in
`query.rs` is that expression; `push_sources` matches on it, and the page load selects it as `source_url` into
`ImageRecord.source_url` (`sourceUrl` in the mirror, `string | null`, doc comment as
`account`'s: derived at load, never stored). The page load reads rows through
`ingest::image_columns()`, not `MATCHED_COLUMNS` (which only feeds the paging and grouping
CTEs), so that is where the expression is interpolated — the column list became a function
for exactly that, so `ingest.rs` never spells the expression a second time. `upload-form.ts`
prefills `source` from `image.sourceUrl ?? ''`, dropping its own `pageUrl ?? imageUrl`; its
"never a local path" comment stays true, since a local import has neither URL, and moves to the
Rust expression's doc comment where the value is now defined. The `images.source` column is
the capture origin (`extension` / `local` / `legacy-bundle`) and is not what `source:` reads;
the design says so beside `SOURCE_URL_SQL` so nobody wires the column to the metatag.

**D3. Compile: `LIKE` with `ESCAPE '\'`.** The pattern is `like_literal` applied to the value
between its `*`s, each `*` becoming `%`, then `%` appended: Danbooru's `ILIKE value + '*'`.
A `_` in a URL (common) is therefore literal. SQLite's `LIKE` folds ASCII case only; URLs are
ASCII where it matters (scheme, host) and a Unicode path compared case-sensitively is
accepted. Include: `SOURCE_URL_SQL LIKE ?n ESCAPE '\'`, one per term, ANDed. Exclude:
`(SOURCE_URL_SQL IS NULL OR SOURCE_URL_SQL NOT LIKE ?n ESCAPE '\')`, keeping sourceless
images, the `-account:` rule (`excluding_an_account_keeps_pages_that_have_none`).
`source:none`: `SOURCE_URL_SQL IS NULL`; `-source:none`: `IS NOT NULL`. No index: a scan of
the matched set is what every other metatag does at this size.

**D4. The two metatag lists gain the word.** `stamp.ts`'s `SEARCH_ONLY_METATAGS` gets
`'source:'` (a stamp of `source:x` is refused as not an edit); `tag-input.ts`'s `METATAG`
alternation gets `source`. Both are hand-spelled lists by their own design (stamp D1), so this
change adds a row to each rather than importing one.

**D5. Spec.** `library-browse`'s query-language requirement names `source:` with the
Danbooru rule and `none`; `booru-upload`'s prefill sentence names `sourceUrl` as the value.
This delta is written against the text as `text-search-substring` leaves it, so archiving in
queue order keeps both changes' sentences.

## Risks

- A query like `source:https://x.com/*` typed into a box that also drives tag completion:
  D4's `METATAG` row keeps the list shut, the same as for `account:`.
- `parseTagSearch`'s order matters: the `source:` regex must run before the tag tokenizer and
  after nothing that would eat a URL; the count metatags read numbers, `account:` reads
  `[a-zA-Z0-9_]`, `collection:` reads `[^\s,]+` — a `collection:` regex would match inside
  `source:https://a.com/collection:x` only if such a URL existed; read `source:` first anyway.
