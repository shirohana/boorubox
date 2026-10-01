> One unit, Sonnet, retried on Opus if the gate fails. Design D1–D3 decide every shape; do not
> re-decide them. Owns `packages/app/src/lib/components/library/StampBar.svelte` only —
> `stamp-order` edits `StampsTable.svelte` and the stamps store beside it, never this file.
> Gate: `pnpm lint && pnpm typecheck && pnpm --filter @boorubox/app test`. Does not commit;
> does not tick the hand check.

## 1. Unit C — the toggle and the clear control (`packages/app/src/lib/components/library`)

- [x] 1.1 `StampBar.svelte`: the chip's `onclick` per D1, and the chip comment's "never toggles
      itself off" paragraph rewritten to say the chip writes the field in both directions and
      why (D1's argument in two lines, not its history).
- [x] 1.2 `StampBar.svelte`: the wrapper, `pr-7`, and the clear button per D2, with a comment
      saying why it is here and not in `TagInput`. The help line per D3.
- [x] 1.3 `StampBar.svelte.test.ts` (new, `// @vitest-environment jsdom`, the
      `FilterRow.svelte.test.ts` mount shape, `stamps` store seeded with one stamp): clicking
      a chip fills the field; clicking it again empties it; the clear control is absent
      with an empty field, present with text, and empties it.
- [x] 1.4 Gate green. Handoff: anything the design did not foresee.
- [ ] 1.5 Hand check (owner): in edit mode click a stamp, click it again — the field empties
      and a thumbnail click opens the viewer; type a one-off, the × appears inside the
      field's right end and clears it; the help line reads as D3.
      Seen (lead's smoke 2026-10-02): clicking Stamp1 filled `tagme`, the chip read pressed and the × showed at the field's end (boorubox-vault/smoke-2026-10-01/21-chip-on.png); a second click emptied the field (boorubox-vault/smoke-2026-10-01/22-chip-off.png); a typed `cat` showed the × (boorubox-vault/smoke-2026-10-01/23-typed.png); the × emptied the field and left the caret in it (boorubox-vault/smoke-2026-10-01/24-cleared.png). The help line reads as D3 (boorubox-vault/smoke-2026-10-01/20-edit-mode.png). A thumbnail click after clearing was not tried.

## Handoff

Unit C landed: `StampBar.svelte` chip toggles per D1 (comment rewritten), field wrapped in
`relative min-w-0 flex-1` with `pr-7`, `icon-xs` ghost X clear button (`aria-label="Clear stamp"`,
refocuses via `TagInput.focusEnd()`), help line per D3. New `StampBar.svelte.test.ts` (no harness
needed; mocks `$lib/api/commands` so the store imports cleanly) covers chip fill/empty and the
clear control. No deviations. Gate: lint, typecheck, app tests green. 1.5 hand check left open.
