> One unit, Sonnet, retried on Opus if the gate fails. Design D1–D3 decide every shape; do not
> re-decide them. Owns `components/tags/PinnedDot.svelte` (new), `FilterRow.svelte` and its
> test, `TagSidebar.svelte`, and in `Inspector.svelte` only the described image's tag list
> (~line 1099) — `pinned-group-management` edits the pinned snippets in the same file after
> this unit has landed. Gate: `pnpm lint && pnpm typecheck && pnpm --filter @boorubox/app
> test`. Does not commit; does not tick the hand check.

## 1. Unit M — the dot (`packages/app/src/lib/components`)

- [x] 1.1 `tags/PinnedDot.svelte` per D1, header comment saying why a dot and why muted.
      `PinnedDot.test.ts` (`// @vitest-environment jsdom`, mount): nothing for `false`, the
      disc and the hidden text for `true`.
- [x] 1.2 `tags/FilterRow.svelte`: `pinned = false` prop, `PinnedDot` between the name button
      and `TagNoteIndicator` per D2. `FilterRow.svelte.test.ts`: a pinned row draws the dot
      before the count, an unpinned row draws none. `TagSidebar.svelte` passes
      `pinned={vocabulary.isPinned(name)}`.
- [x] 1.3 `library/Inspector.svelte`: `PinnedDot` after the name span in the described
      image's tag buttons per D2. `Inspector.svelte.test.ts`: a pinned tag in the list
      carries the dot (seed the vocabulary store as its other tests seed stores).
- [x] 1.4 Gate green. Handoff: anything the design did not foresee.
- [ ] 1.5 Hand check (owner): pin a tag; its sidebar row and its entry under the inspector's
      Tags show a small dot after the name, the same in the viewer; include the tag in the
      search and the tint plus dot read as two different facts; the dot adds no visible
      width beyond one gap.
      Seen (lead's smoke 2026-10-02): the sidebar rows `blue_archive` and `hololive` carry a small dot before the count, `daran_9` reads dot then note glyph (boorubox-vault/smoke-2026-10-01/01-launch.png); the inspector's tag list shows the dot after every pinned tag and none after `kani_biimu` (boorubox-vault/smoke-2026-10-01/03-inspector.png). The viewer and an included-tag tint were not exercised.

## Handoff

Unit M landed: `tags/PinnedDot.svelte` (+ test), `FilterRow` `pinned = false` prop with the dot
between the name button and `TagNoteIndicator`, `TagSidebar` passes `vocabulary.isPinned(name)`,
Inspector's described-image tag buttons render the dot after the name span. Tests added in
`PinnedDot.test.ts`, `FilterRow.svelte.test.ts`, `Inspector.svelte.test.ts` (seeds
`vocabulary.entries`, resets it after). Gate green (lint 0 errors, typecheck 0, 902 tests).
Dot markup: `<span aria-hidden="true" class="size-1 shrink-0 rounded-full bg-muted-foreground/70"></span><span class="sr-only">pinned</span>`.
Unforeseen: the sr-only span is a second flex child, so it sits in the row's gap flow
(position absolute, so no width). The Inspector pinned chip also is a button containing the tag
name, so its test selects the list buttons by `button.max-w-full`. Hand check 1.5 left open.
