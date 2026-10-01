> One unit, Sonnet, retried on Opus if the gate fails. Design D1–D4 decide every shape; do not
> re-decide them. Owns `components/tags/PinnedDot.svelte` + its test, `FilterRow.svelte` + its
> test, `TagNoteIndicator.svelte`, `TagSidebar.svelte`, `api/vocabulary.svelte.ts` + its test,
> and in `Inspector.svelte` only the described image's tag list (~line 1099) + its test. The
> `pinned-tags-panel` change edits `Inspector.svelte`'s pinned snippets after this unit lands;
> stay out of them. Gate: `pnpm lint && pnpm typecheck && pnpm --filter @boorubox/app test`.
> Does not commit; does not tick the hand check.

## 1. Unit A — order, line, hint (`packages/app/src/lib`)

- [x] 1.1 `api/vocabulary.svelte.ts`: `pinnedLabelOf(name): string | null` per D1 with a doc
      line; `vocabulary.svelte.test.ts`: a pinned tag in a named group answers the name, in an
      unnamed group `#n`, an unpinned tag `null`.
- [x] 1.2 `tags/PinnedDot.svelte` per D1–D2 (`group` prop, tooltip, `portalTo?`); `PinnedDot.test.ts`
      (a Tooltip.Provider harness as `TagNoteIndicator.test-harness.svelte` has): nothing for
      `null`, the disc and the hidden "Pinned in Scene" for a label.
- [x] 1.3 `tags/FilterRow.svelte`: `pinnedLabel: string | null = null` replaces `pinned`,
      order per D3; `TagSidebar.svelte` passes `vocabulary.pinnedLabelOf(name)`;
      `FilterRow.svelte.test.ts` asserts the order dot, glyph, count.
- [x] 1.4 `library/Inspector.svelte` tag list: order per D3, `items-center` on the `<ul>` per
      D4, `group={vocabulary.pinnedLabelOf(tag)}` and `{portalTo}`;
      `tags/TagNoteIndicator.svelte`: `leading-none` on the trigger span per D4.
      `Inspector.svelte.test.ts`: the pinned-dot test asserts the glyph precedes the dot.
- [x] 1.5 Gate green. Handoff: anything the design did not foresee.
- [ ] 1.6 Hand check (owner): in the inspector a noted pinned tag reads name, glyph, dot on
      the same line as its neighbours; in the sidebar name, dot, gap, glyph, count; hovering a
      dot shows "Pinned in …" with the group's name or number, in the viewer too.
      Seen (lead's smoke 2026-10-02, scratch copy of test-3): sidebar row `daran_9` reads name, dot, gap, note glyph, count (boorubox-vault/smoke-2026-10-01/r2-01-inspector.png); the inspector lists `daran_9`, note glyph, dot on the same line as `blue_archive`, dot (boorubox-vault/smoke-2026-10-01/r2-02-inspector-noted.png); hovering `new_added`'s dot in the sidebar showed “Pinned in #2” (boorubox-vault/smoke-2026-10-01/r2-03-dot-hint.png). The viewer was not opened.

## Handoff

Unit A landed, 1.1–1.5 ticked, 1.6 hand check left open.
- `vocabulary.pinnedLabelOf` added with test; `PinnedDot` takes `group` + `portalTo?` and draws the tooltip (new `PinnedDot.test-harness.svelte`).
- `FilterRow`: `pinnedLabel` replaces `pinned`; order name, dot, glyph, count; new `FilterRow.test-harness.svelte` (tooltip provider), since the row now mounts tooltips.
- Inspector tag list: name, glyph, dot; `items-center` on the `<ul>`; `leading-none` on the note trigger span (and the dot's trigger).
- Deviation: the sidebar's `ml-0.5` sits on a wrapper `<span class="ml-0.5 inline-flex empty:hidden">` around `TagNoteIndicator`, which takes no class; `empty:hidden` keeps an unnoted row from gaining a gap.
- Gate: lint (warnings only), typecheck, app tests 952 passed.
- Hand check: compare a noted and an unnoted tag on one line in the inspector; fallback `align-middle` per D4.
- Review fixes applied: 9 — the dot and the note glyph share `tags/HoverHint.svelte` (the D2 tooltip shape, one place); `PinnedDot.test-harness.svelte`, `FilterRow.test-harness.svelte` and `TagNoteIndicator.test-harness.svelte` are replaced by `tags/TooltipHarness.svelte` (`component` + `props`).
