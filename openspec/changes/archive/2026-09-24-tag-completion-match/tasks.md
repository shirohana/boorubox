> One unit, Sonnet (retried on Opus if the gate fails), after `pinned-tag-groups` unit R has
> landed (both edit `tags.rs`). Design D1–D2 decide the shapes. Gate: `mise run check` green.
> No unit ticks the hand check.

## 1. Unit S — the query and the filter (`packages/app/src-tauri`, `packages/app`)

- [x] 1.1 `tags.rs`: `suggestions` per D1, doc comment amended (substring, the ranking, why
      the leading `%` costs nothing new). Tests through the module's `suggested` helper:
      `suggestions_match_anywhere_in_the_name`, `an_exact_match_is_offered_first`,
      `a_prefix_match_outranks_a_contains_match_whatever_the_counts`, and the existing
      prefix test kept. Verify: `cargo test tags::` pass.
- [x] 1.2 `domain/tag-input.ts`: `filterSuggestions(value, caret, candidates)` per D2;
      `TagInput.svelte` passes `caretNow()`. Tests in `tag-input.test.ts`: the exact match
      survives (`filterSuggestions('cat', 3, ['cat', 'cathedral'])` keeps both), a tag named
      elsewhere is dropped (`'cat ca'` with caret at the end drops `cat`), the existing cases
      updated to the new signature. Verify: `mise run check` green.
- [ ] 1.3 Hand check: type `cat` in the inspector's editor with `cat` and `cathedral` in the
      library — the list shows `cat` first, highlighted; Enter leaves `cat ` and Tab does the
      same; type `cat` with `cute_cat` in the library — it is offered.

Hand check: run the app against a scratch library carrying `cat`, `cathedral` and `cute_cat`,
open an image's tag editor, type `cat` and confirm: the popover reads `cat` first and
highlighted, `cathedral` after it; Enter and, separately, Tab each leave `cat ` with the caret
after the space; clear and type `cat` again with only `cute_cat` in the library and confirm it
is offered. Left open per the brief — 1.3's box stays unticked.
Seen 2026-09-24 (smoke, scratch copy of test-1, which has no cat/cathedral/cute_cat, so other tags stood in): typing the whole tag `electronic` listed it, highlighted (/Users/shirohana/.claude/jobs/8dbef24a/tmp/32c.png). Enter accepted it with a trailing space (/Users/shirohana/.claude/jobs/8dbef24a/tmp/33c.png): typing `i` next gave `electronic i`. For `i`, the list showed `i` first and highlighted, then in, including, industry's… (/Users/shirohana/.claude/jobs/8dbef24a/tmp/34c.png). Tab accepted `i ` the same way. The mid-name fragment `rch` offered kei_(blue_archive) and plana_(blue_archive), and dropped blue_archive because it was already in the draft (/Users/shirohana/.claude/jobs/8dbef24a/tmp/35c.png). Escape cancelled with the tags unchanged (/Users/shirohana/.claude/jobs/8dbef24a/tmp/36c.png).

## Handoff

- **1.1** `tags.rs::suggestions` now matches `LIKE '%' || escaped_prefix || '%'` (was
  `escaped_prefix || '%'`), ranked `ORDER BY CASE WHEN name = ?prefix THEN 0 WHEN name LIKE
  ?prefix_pattern THEN 1 ELSE 2 END, uses DESC, name`. `like_prefix` was renamed
  `like_escape` (escaping only, no `%` appended) so `suggestions` builds both the contains and
  the prefix pattern from the one escape. New tests: `suggestions_match_anywhere_in_the_name`,
  `an_exact_match_is_offered_first`, `a_prefix_match_outranks_a_contains_match_whatever_the_counts`.
  One existing test's assertion changed on purpose:
  `a_prefix_that_looks_like_like_syntax_is_only_ever_a_value` — typing a bare `%` now matches
  `100%_wool` (it contains a literal `%`) rather than nothing, which is the correct substring
  reading; the test still pins that `%` is never SQL wildcard syntax; a reviewer should check
  that reasoning.
- **1.2** `filterSuggestions(value, caret, candidates): string[]` (was `(value, candidates)`)
  — cuts the current token's span out of `value` (replaced by a single space) before parsing,
  so only a tag named elsewhere in the input lands in the drop set; the token being typed is
  never in it. `TagInput.svelte`'s `refresh()` now captures `caretNow()` once into a local
  `caret` and reuses it for both `suggestionPrefix` and `filterSuggestions`, rather than
  calling `caretNow()` a second time after the `await` (the caret could have moved by then;
  same fix as the existing `request !== asked` staleness guard, just for the caret instead of
  the prefix). `initialHighlight`, `applySuggestion`, `confirmAction` untouched — D2 says the
  ranking and the highlight are what make Enter/Tab accept the exact match, no new confirm
  rule.
- **Gate**: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test tags::` (95
  passed) all green. `mise run check` green end-to-end on a clean rerun (745 Rust tests, 709
  app tests, lint, typecheck 0 errors, build). One rerun mid-session hit a single flaky
  failure, `recover::tests::a_rebuild_compacts_the_groups_a_merge_left_with_a_gap` (an `ENOENT`
  from a filesystem race under `cargo test`'s parallel runner) — passes alone and passes in
  the full suite on rerun; unrelated to this unit (`recover.rs` untouched) and not reproduced
  on a second full run. `npx eslint` clean on the touched files.
- **Reviewer**: start with `tags.rs`'s new `suggestions` (the two-pattern query) and the
  changed assertion in `a_prefix_that_looks_like_like_syntax_is_only_ever_a_value`; then
  `filterSuggestions`'s token-cut-out in `tag-input.ts` and the two required scenarios in
  `tag-input.test.ts`.
