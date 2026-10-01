## Context

`RulesSection.svelte` loads `entries: RuleListEntry[]` (ordered by name in Rust,
`auto-tag-rules` design D4) and hands them to `RuleList.svelte`, which draws one stacked entry
per rule. Settings → Artists already carries a filter box over its list, so the page family
has a precedent for a search that is a plain substring over what is on screen.

## Decisions

**D1. The filter is a pure function, `filterRules(entries, query)` in
`components/rules/filter.ts`.** It lowercases the query, trims it, and keeps an entry when its
`rule.name`, `rule.pattern` or any element of `rule.tags` contains the text; a blank query
answers the entries unchanged, same array identity, so an unfiltered list never re-keys. Pure
and tested on its own (`filter.test.ts`); `RulesSection` derives `shown` from it and passes
`shown` to `RuleList`, which stays ignorant of the search. Matching is in the webview, not
Rust: the list is already in memory in full, and a round trip per keystroke would buy nothing.

**D2. The field sits between the description paragraph and the New rule button, full width,
shown only while the library has at least one rule.** Placeholder "Search rules", an `Input`
with `aria-label`, cleared by Escape through `blurOnEscape`'s sibling behaviour (clear, do
not blur: the next search starts there). `app-frame`'s "no control appears before it does
something": with no rules there is nothing to search, and the empty-state paragraph stands
alone as today. The query is component state, dropped on leaving the page (non-goal).

**D3. No match is said, not shown as an empty box.** When `entries.length > 0` and
`shown.length === 0`, `RuleList`'s empty paragraph is not the right text (it says "No rules
yet"), so `RulesSection` draws its own line, `No rules match “<text>”`, and does not mount the
list. The import's `newIds` and the run report are untouched by the filter: a run report lists
by its own counts, and a New badge on a filtered-out rule simply waits for the filter to clear.

## Risks / Trade-offs

- [A long tag list per rule makes `some()` per keystroke O(rules × tags)] → thirty rules with
  ten tags each is three hundred string checks; nothing to optimise.
