## Context

`tags::suggestions(conn, prefix, limit)` answers `LIKE 'prefix%'` ordered by uses then name
(`tags-and-ratings` design D12). `domain/tag-input.ts`'s `filterSuggestions(value, candidates)`
drops every tag the parsed input names — the token being typed included, "so an exact match
never offers itself" — then caps the list; `initialHighlight` highlights row 0 whenever
something was typed, and `confirmAction` accepts the highlighted row on Enter, Tab accepts
only while a row is highlighted. `TagInput.svelte` wires them.

## Decisions

**D1. Substring match in SQL, ranked exact → prefix → contains.** `suggestions` matches
`LIKE '%' || prefix || '%'` and orders by `CASE WHEN name = ?p THEN 0 WHEN name LIKE ?p || '%'
THEN 1 ELSE 2 END, uses DESC, name`. The scan of `tags` is what it already was (D12's own
words: bounded by the vocabulary, not sped up by an index), so a leading `%` costs nothing
new. This amends `tags-and-ratings` D12's "prefix query": the argument there was the query's
cost and the list's usefulness, and neither changes; what changed is the owner's vocabulary,
compounds whose remembered half is the tail (2026-09-24). `like_prefix`'s escaping is kept
for the pattern's middle.

**D2. The word being typed is never "taken".** `filterSuggestions(value, caret, candidates)`
parses the input with the current token cut out (`currentToken`'s span replaced by a space),
so only tags named elsewhere are dropped; the exact match then arrives first from D1 and
`initialHighlight` highlights it, which makes Enter and Tab accept it and finish the token
with a space through the existing `applySuggestion` — no new confirm rule. The "Already
typed" scenario (`cat` already in the input, `cat` typed again) still drops it: it is named
elsewhere. The comment on `filterSuggestions` says why the token under the caret is exempt.

## Risks / Trade-offs

- [More rows match a short prefix] → the cap is unchanged (`SUGGESTION_LIMIT`), and the
  ranking keeps the tag someone is spelling out at the top.
- [An exact match highlighted means Enter accepts rather than completes] → both leave the
  same text (`cat `); the difference is only which rule fired.

## Migration

None.
