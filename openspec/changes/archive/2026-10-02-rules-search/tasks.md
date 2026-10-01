> One unit, Sonnet, retried on Opus if the gate fails. Design D1–D3 decide every shape; do not
> re-decide them. Owns `packages/app/src/lib/components/rules/**` only. Gate: `pnpm lint &&
> pnpm typecheck && pnpm --filter @boorubox/app test`. Does not commit; does not tick the hand
> check.

## 1. Unit S — the filter and the field (`packages/app/src/lib/components/rules`)

- [x] 1.1 `filter.ts`: `filterRules(entries: RuleListEntry[], query: string):
      RuleListEntry[]` per D1, doc comment saying what it matches. `filter.test.ts`: blank and
      whitespace-only queries answer the same array identity; a match by name, by pattern and
      by a tag; case folded both sides; no match answers an empty array.
- [x] 1.2 `RulesSection.svelte`: `query` state, `shown = $derived(filterRules(entries,
      query))`, the `Input` per D2 between the description and the New rule button, mounted
      only while `entries.length > 0`; the no-match line per D3 in place of `RuleList` when
      `shown` is empty and `entries` is not. `RuleList` receives `shown`, unchanged itself.
- [x] 1.3 Gate green. Handoff: where the field landed and anything the design did not foresee.
- [ ] 1.4 Hand check (owner): type part of a tag on the Rules page; only the rules carrying it
      stay, the New badge of an imported rule is still there when its rule matches, and
      clearing the field brings every rule back.
      Seen (lead's smoke 2026-10-02 on a scratch copy of test-3, two seeded rules): typing `low` kept only Kantoku, matched through its `lowres` tag (boorubox-vault/smoke-2026-10-01/14-rules-low.png); `zzz` showed “No rules match “zzz”” (boorubox-vault/smoke-2026-10-01/15-rules-zzz.png); Escape cleared the field and listed both rules again (boorubox-vault/smoke-2026-10-01/15b-rules-esc.png). No imported rule was on hand for the New badge.

