## Why

The owner's tagging flow needs "which images lack a copyright tag": `tagcount:` counts every
tag, so an image with ten general tags and no copyright tag is indistinguishable from a
finished one. Danbooru answers the same question with per-category counts (`copytags:0`), and
with auto artist tags arriving (`auto-artist-tag`), `arttags:0` finds the images whose artist
tag was omitted or whose site has no adapter. The same flow needs "which images are in no
collection yet", which `collection:` cannot say today: it only names collections. Requirements
§6 (Danbooru-style tag search, the lifted `tag-utils` parser) and §7 (tag-count among the
queries the storage must serve).

## What Changes

- **Five category count metatags**: `gentags:`, `arttags:`, `chartags:`, `copytags:`,
  `metatags:` count an image's tags in one category (general, artist, character, copyright,
  meta). They take `tagcount:`'s syntax — exact, `>`, `<`, `>=`, `<=`, `a..b`, `1,3,5` — which
  is what Danbooru's cheatsheet promises ("uses same syntax as id search").
- **Count metatags combine**: `copytags:0 chartags:>0` asks for both. The same metatag given
  twice keeps the first, as `tagcount:` does today. A leading `-` on a count metatag is dropped
  exactly as it is on `tagcount:` (Danbooru does not negate count metatags either); this change
  notes that and leaves it.
- **`collection:none` and `collection:any`**: images in no collection, and images in at least
  one. `-collection:none` means `collection:any` and `-collection:any` means `collection:none`,
  mirroring Danbooru's `pool:none|any`. The sidebar's and the inspector's collection clicks
  leave these terms standing.
- **Wire shape**: `ParsedTagSearch.tagCount` (one optional filter) becomes a list of count
  terms, each with an optional category; `tagcount:` is the entry with none. Two booleans
  carry `none`/`any`. The webview and Rust change together (**BREAKING** for the IPC shape
  only; nothing outside the app reads it).
- **Follow-throughs**: the stamp grammar refuses the five names as search-only; the tag
  input's autocomplete stays shut behind them.

## Reversal: search by category was a non-goal

`tag-vocabulary`'s proposal listed "Search by category (`arttags:` and the like), or
`artist:` as a search prefix" as a non-goal, because "any later prefix must stay clear of the
category names". That reading was right then: the vocabulary had just made `artist:name` a
*creation* prefix in the editor, stamps and bulk add, and a search prefix spelled the same way
would have made one token mean "make this tag" in one field and "filter on this" in the next.
No flow asked for category search yet, so deferring it cost nothing. What changed is the
flow: tagging by category needs "which images lack a copyright tag", and only a count by
category answers it. The argument itself survives — `copytags:` and its siblings are
Danbooru's own names and none of them is a category name or a creation prefix (`copy:`,
`art:`, `char:`, `gen:`, `meta:` and the long forms stay creation-only) — so the non-goal
narrows rather than disappears: `artist:` as a *search* prefix stays out.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `library-browse`: "Tag search uses the legacy query language" gains the five category count
  metatags with `tagcount:`'s syntax, the combining and repeat rules, and `collection:none` /
  `collection:any` with their negations.
- `stamps`: the refusal list of search-only metatags gains the five names.
- `tag-editing`: "The editor suggests tags the library already uses" stays shut inside the
  five new metatags as it does inside `tagcount:`.

## Non-goals

- `artist:` (or any category name) as a search prefix — see the reversal above.
- Negating a count metatag (`-copytags:0`). The leading `-` is dropped as on `tagcount:`,
  `rating:` and `is:`; changing that is its own change for all of them.
- Searching a collection literally named `none` or `any`. Its slug is the keyword, so
  `collection:none` finds images in no collection, never that collection; the collection still
  exists, is listed and counted, and its members are reachable by browsing it. No escape
  syntax is built: the owner's call (2026-09-23) is that the trade-off is cheaper than a
  quoting rule nobody asked for.
- Counts by category in the sidebar or the inspector. The sidebar's category toggles are
  `tag-category-visibility`'s.
- `pool:`-style ordered membership or collection counts (`collections:>1`).

## Impact

- Shared: `ParsedTagSearch` (`tagCount` → `tagCounts: TagCountTerm[]`, new `anyCollection` /
  `noCollection`), new `TagCountTerm`.
- Rust: `model.rs` (the mirror), `query.rs` (`push_tag_count` over the list with a categorised
  count subquery, `push_collections` gains the membership clauses) and their tests. No
  migration: `tags.category` and `image_collections` already hold what is needed.
- Webview: `domain/tag-utils.ts` (`parseTagSearch`, the collection rewriters),
  `domain/stamp.ts` (`SEARCH_ONLY_METATAGS`), `domain/tag-input.ts` (`METATAG`), and their tests
  (`tag-parser.test.ts`, `tag-query.test.ts`, `stamp.test.ts`, `tag-input.test.ts`),
  `api/commands.test.ts`'s literal.
