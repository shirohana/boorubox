> One unit, Sonnet, retried on Opus if the gate fails; the lead checks it on screen. Design
> D1–D2 decide the shape. Owns `components/library/StampBar.svelte` and its test only.
> Gate: `pnpm lint && pnpm typecheck && pnpm --filter @boorubox/app test` from the repo root.
> Does not commit; does not tick the hand check.

## 1. Unit S — the clear control stays under the pointer (`packages/app/src/lib/components/library`)

- [x] 1.1 `StampBar.svelte`: the clear `Button` is placed by a wrapper per D1 (`absolute
      inset-y-0 right-0.5 flex items-center pointer-events-none`, the button
      `pointer-events-auto`), the button keeps `size="icon-xs" variant="ghost"
      aria-label="Clear stamp"` and `onclick={clear}` and loses `absolute top-1/2 right-0.5
      -translate-y-1/2`. The wrapper's comment says why the centring is not on the button.
- [x] 1.2 `StampBar.svelte.test.ts`: the existing clear test still passes; no class
      assertion is added (D2).
- [x] 1.3 Gate green. Handoff: anything the design did not foresee.
- [ ] 1.4 Hand check (owner, Windows): with text in the stamp field, press and release the
      "x" at its centre — the control does not move while pressed, the field empties and keeps
      the caret.
      Seen (lead, on screen 2026-10-04, macOS WebKit, scratch copy of the 10-01 smoke lib): with `cat -dog` typed, a plain press-and-release at the control's centre emptied the field and left the caret in it with the focus ring (boorubox-vault/smoke-2026-10-04/19-typed-s.png before, boorubox-vault/smoke-2026-10-04/21-clicked-s.png after). Windows not seen; the owner's press on WebView2 is the open half.

## Handoff

No deviations. Wrapper at StampBar.svelte (the `{#if text !== ''}` block); test unchanged and passing.
