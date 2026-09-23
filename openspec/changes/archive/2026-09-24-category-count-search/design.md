## Context

See proposal.md — Why. The search language has one definition, `parseTagSearch` in
`packages/app/src/lib/domain/tag-utils.ts`; Rust never sees the query string, only the
`ParsedTagSearch` it produces (`library-browse` design D3), and compiles it in
`query.rs` `compile()`. What the code holds today:

- `parseTagSearch` strips each metatag off `remainingQuery` in a fixed order (design D15: every
  step reads and strips the same string): 1 `tagcount:`, 2 `rating:`, 3 `is:`, 4 `account:`,
  5 `collection:`, 6 the tag terms. `tagcount:` is two regexes: the comma-list form is tried
  first, then one `exec` of `tagcount:(>=|<=|>|<|)(\d+)(\.\.(\d+))?`; whichever matches, every
  occurrence is stripped. Neither regex anchors at a token start, so `-tagcount:5` leaves a
  lone `-` that step 6 drops as an empty exclusion — the "leading `-` is ignored" behaviour.
- `ParsedTagSearch.tagCount: TagCountFilter | null` (`packages/shared/src/index.ts`), mirrored
  as `tag_count: Option<TagCountFilter>` in `model.rs`; `TagCountFilter { operator, value?,
  values?, min?, max? }` with `TagCountOperator` serialised as `=` `>` `<` `>=` `<=` `range`
  `list`.
- `query.rs`: `TAG_COUNT` is a correlated `COUNT(*)` over `image_tags`; `push_tag_count`
  compiles one filter through `Filter::add_bound`, falling back to `MATCHES_NOTHING` when an
  operand is missing. `push_collections` compiles the slug lists through
  `has_any_collection(n)` (`EXISTS … JOIN collections … slug IN (?…)`), `NOT` for the exclude
  side.
- `tags.category TEXT NOT NULL DEFAULT 'general' CHECK (…five names…)` (schema v7);
  `TagCategory` in `model.rs` already implements `ToSql` with the storage string, and
  `TagCategory` exists in shared.
- The collection rewriters (`toggleCollectionInQuery`, `addCollectionToQuery`,
  `excludeCollectionFromQuery`) rewrite through `rewriteMetatagList(query, marker, kept)`,
  which drops **every** token starting with `collection:` (or `-collection:`) and appends one
  rebuilt list. The `collections` spec already requires that a sidebar click "SHALL NOT disturb
  the rest of the query".
- The stamp grammar's `SEARCH_ONLY_METATAGS` (`domain/stamp.ts`) and the tag input's `METATAG`
  regex (`domain/tag-input.ts`) each spell the metatag names by hand.

## Goals / Non-Goals

**Goals:** one table of count metatags in the parser, so adding a sixth is one row; one wire
list for every count term, so Rust has one loop and no per-category field; `none`/`any` as a
membership test that needs no slug and no join to `collections`; every value in the SQL a
bound parameter.

**Non-Goals:** a Rust copy of the language; changing how `-` reads on any count metatag;
touching the sidebar, the inspector or the tag counts `tag_counts` answers; a migration.

## Decisions

### D1. The wire shape: `tagCountTerms: TagCountTerm[]`

```ts
export interface TagCountTerm {
  /** `null` for `tagcount:`, which counts every tag. */
  category: TagCategory | null
  filter: TagCountFilter
}
// ParsedTagSearch: `tagCount: TagCountFilter | null` → `tagCountTerms: TagCountTerm[]`
```

Rust mirror in `model.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagCountTerm {
    pub category: Option<TagCategory>,
    pub filter: TagCountFilter,
}
// ParsedTagSearch: `tag_count: Option<TagCountFilter>` → `tag_count_terms: Vec<TagCountTerm>`
```

`TagCountFilter` and `TagCountOperator` are unchanged. An empty list is "no count filter".

*Why a list and not six optional fields:* the six would be the same shape spelled six times on
both sides of the wire, and `push_tag_count` would be six copies of one call; the list is one
loop, and a new count metatag is a parser row, not a wire change. *Why `category: … | null`
and not `category?:`:* the existing wire says "absent" with `null` (`tagCount: … | null`), the
parser always writes every field, and a `toEqual` in a test then compares one shape, not two.
*Why `tagCountTerms` and not `tagCounts`:* `tag_counts` is already the sidebar's command and
`TagCounts`/`TagCount` its answer types in `query.rs`; a query field of the same name would
make every grep for the sidebar counts land in the compiler too.

### D2. The parser reads count metatags from one table, first occurrence wins

Step 1 of `parseTagSearch` becomes a loop over one module-level table:

```ts
const COUNT_METATAGS: [name: string, category: TagCategory | null][] = [
  ['tagcount', null], ['gentags', 'general'], ['arttags', 'artist'],
  ['chartags', 'character'], ['copytags', 'copyright'], ['metatags', 'meta'],
]
```

For each row, one regex per name — `-?NAME:` followed by, in this order, `(>=|<=|>|<)(\d+)`,
`(\d+)\.\.(\d+)`, `(\d+(?:,\d+)+)`, `(\d+)` — case-insensitive and, like every metatag regex
in the file, not anchored. The **first match in the text** is read into a `TagCountFilter`
exactly as today (a range is normalised to `min ≤ max`; a list keeps its order), then
**every** match of that name is stripped from `remainingQuery` (the D15 rule: the read and the
strip are over the same string). The term is pushed as `{ category, filter }`; `tagCountTerms`
is therefore in the table's order, not the text's, which is what the tests assert. The `-?`
makes the strip take the sign with the term, so `-copytags:0` reads as `copytags:0` with nothing left behind — the same
observable result as today's lone `-` being dropped, stated rather than accidental.

*Why one alternation instead of today's list-first pair:* "keeps the first" is the rule the
spec now states, and the pair broke it — `tagcount:5 tagcount:1,3` read the list because the
list regex ran first. With one regex the first occurrence wins in every form. This is the one
behaviour change `tagcount:` gets; the existing test "takes the first tagcount and strips every
one of them" keeps passing, and a new one covers the mixed forms. *Why no token-start anchor:*
none of the six names is a suffix of another, so an unanchored `copytags:` cannot fire inside
`tagcount:` or the reverse, and an anchor would need a lookbehind no other metatag regex in
the file uses. *Why keep the operator form single-valued:* `tagcount:>1,2` reads `>1` today
and leaves `,2` as a tag term; the alternation keeps that, since changing it is not asked for.

### D3. The SQL for a categorised count: the same subquery with a bound category

```rust
const TAG_COUNT: &str = /* unchanged */;
/// The image's tag count in one category — `TAG_COUNT` narrowed by a bound `tags.category`.
const CATEGORY_TAG_COUNT: &str = "(SELECT COUNT(*) FROM image_tags
    JOIN tags ON tags.id = image_tags.tag_id
    WHERE image_tags.image_id = images.id AND tags.category = ?)";
```

`push_tag_count` iterates `query.tag_count_terms`; for each term it takes the count expression
and its leading parameters — `(TAG_COUNT, [])` for `None`, `(CATEGORY_TAG_COUNT,
[Value::Text(category.as_str())])` for `Some` — and compiles the filter exactly as today, with
the leading parameters chained **before** the comparison's values, because the category's `?`
comes first in the clause text (`{expr} BETWEEN ? AND ?` binds category, min, max). A missing
operand is still `MATCHES_NOTHING`. `push_tag_count_comparison` gains the expression and the
leading parameters as arguments rather than a second copy.

*Why a bound value and not the category string formatted in:* `query.rs`'s rule is that nothing
but a placeholder is formatted into SQL; `TagCategory` is a closed enum today, but the rule is
what keeps the file auditable by grep. *Why a join and not `tag_id IN (SELECT id FROM tags WHERE
category = ?)`:* both are correct; the join reads as "`TAG_COUNT` narrowed", which is the claim
the doc comment makes, and `image_tags`' primary key `(image_id, tag_id)` drives both.

### D4. `collection:none` / `collection:any` are two booleans, compiled as one membership test

Wire: `ParsedTagSearch` gains `anyCollection: boolean` and `noCollection: boolean` (Rust
`any_collection: bool`, `no_collection: bool`), the shape `includeUnrated` already has. The
parser maps `collection:any` and `-collection:none` to `anyCollection = true`, and
`collection:none` and `-collection:any` to `noCollection = true`.

`push_collections` adds, before the slug lists:

```rust
/// "this image is in at least one collection". No join to `collections`: a membership row
/// cannot outlive its collection (`ON DELETE CASCADE`).
const IN_SOME_COLLECTION: &str =
    "EXISTS (SELECT 1 FROM image_collections WHERE image_collections.image_id = images.id)";
if query.any_collection { filter.add(IN_SOME_COLLECTION) }
if query.no_collection { filter.add(format!("NOT {IN_SOME_COLLECTION}")) }
```

No parameters, so `filter.add`, not `add_bound`. Both set (`collection:none collection:any`)
compiles to a clause that is never true and matches nothing, which is what the query says; no
special case. `collection:none collection:cute` likewise matches nothing, and
`collection:any -collection:queue` matches images in some collection other than only `Queue` —
both fall out of AND-ing the clauses.

*Why two booleans and not a `'none' | 'any' | null` field:* the query can state both, and a
single field would have to pick one or invent a third value for the contradiction; two
independent clauses let SQL answer it. *Why not `-collection:` slug list with a sentinel:* the
slug lists compile through `collections.slug IN (…)`; a sentinel there would be a slug Rust has
to recognise, i.e. a second reader of the language.

### D5. How the parser tells a keyword from a slug, and the stripping order

Step 5 becomes two regexes, in this order:

1. `COLLECTION_KEYWORD = /(-?)collection:(none|any)(?=\s|$)/gi` — the keyword is the **whole
   value** of the term: the word is followed by whitespace or the end, never by a comma or more
   slug text. Unanchored at the front, like the slug regex beside it. Every match sets the D4
   boolean from its sign and word, then every match is stripped.
2. The existing slug regex, unchanged, over what is left. Inside its comma lists an element
   equal to `none` or `any` (after lower-casing) is dropped rather than added as a slug.

Because step 5.1 strips the keywords before 5.2 runs, `collection:none` never reaches the slug
regex; because 5.1 requires whitespace or the end after the word, `collection:none_left` and
`collection:none,cute` fall through to 5.2 as slug text. The drop in 5.2 is what makes the
proposal's trade-off true in every spelling: a collection slugged `none` is not searchable by
name, whether written alone or in a list. Without it `collection:none,none` would be an
accidental escape the owner declined to build.

`COLLECTION_KEYWORD` is exported to the rewriters (single source): `rewriteMetatagList` gains an
optional `keep?: (token: string) => boolean`, and the three collection rewriters pass one that
keeps tokens matching the keyword, so `toggleCollectionInQuery('collection:none', 'queue')`
reads `collection:none collection:queue`. The account rewriters pass nothing: `account:none` is
a possible X handle, not a keyword.

`activeTerms` is unchanged: the keywords are not slugs, so no collection row reads as active
under `collection:none`.

### D6. The follow-throughs name the table, not a second list

- `domain/stamp.ts`: `SEARCH_ONLY_METATAGS` gains `gentags:`, `arttags:`, `chartags:`,
  `copytags:`, `metatags:`. The stamp grammar may not import the parser's table (the two
  languages are kept apart on purpose, `stamps` design D1), so the five names are spelled here
  with a comment pointing at `COUNT_METATAGS`.
- `domain/tag-input.ts`: `METATAG`'s alternation gains the five names. `collection` is already
  in it, so `collection:none` already shuts the list.
- `tag-parser.test.ts`'s operator table is lifted into an `it.each` over every count metatag, so
  a sixth row in `COUNT_METATAGS` without the tests is visible.

## Risks / Trade-offs

- [A collection named `None` or `Any` cannot be searched by name] → Documented in the proposal's
  non-goals and the spec; the collection is still listed and counted. A click on its row writes
  `collection:none`, which reads as the keyword and the row does not show as active — the
  visible symptom if it ever happens. No create-time refusal: the owner chose the trade-off over
  more machinery.
- [A tag literally named `copytags:0` (or any `<name>:<digits>`) becomes unsearchable as a tag]
  → Same trade-off `tagcount:` already makes; such tags come only from hand typing.
- [The wire change breaks every `ParsedTagSearch` literal] → One webview literal
  (`api/commands.test.ts`) and Rust tests that build through `..Default::default()`; the
  typecheck and `cargo test` in the gate find any other.
- [`tagcount:5 tagcount:1,3` now reads `5`, not the list] → Intended (D2); covered by a test.

## Migration Plan

None: no schema change, no stored query. The wire change ships in one build of the app, webview
and Rust together.
