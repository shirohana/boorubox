> One unit, Sonnet (two pure formatters with tests; markup edits in one component). Design
> D1–D4 decide every shape; do not re-decide them. Gate: `pnpm lint && pnpm typecheck && pnpm
> test` from the repo root. No unit commits or ticks a hand check.

## 1. Unit I — the facts rows (`packages/app`)

- [x] 1.1 `src/lib/domain/format.ts`: `formatSizeLine` per D2 and `formatRelative` per D3, doc
      comments in the file's own voice. Tests in `format.test.ts`: `formatSizeLine` — a jpg
      (`1.3 MB .jpg (1200×2200)`), a file with no extension, an upper-case extension lower-cased;
      `formatRelative` with a fixed `now` — under a minute is `just now`, `yesterday`, `3 days
      ago`, `2 years ago`, a non-finite value is `—`.
- [x] 1.2 `src/lib/components/library/Inspector.svelte`: labels per D1 (the two inputs'
      `aria-label`s too); the Size row per D2 replacing Dimensions, Size, Type; the Date row per
      D3 replacing Captured, Imported, File modified, its tooltip mounted the way
      `TagNoteIndicator.svelte` mounts its own (same `delayDuration`, span trigger with
      `tabindex={-1}`, the same `portalProps` target the inspector already hands its tag notes);
      the comment above the old File modified row (`NaN` reuse) moves with the value into the
      tooltip. Nothing else in the panel moves.
- [x] 1.3 Gate green. Handoff below: the two signatures, the tooltip's exact mount, anything the
      hand check should watch.
- [ ] 1.4 Hand check (owner): the panel beside the grid and inside the viewer — the five new
      rows read as designed; hovering Date shows the three absolute times in both placements;
      the edit form still opens on Title, Source, Source file and saves.

## Handoff

**Signatures** (`src/lib/domain/format.ts`):
- `formatSizeLine({ size, file, width, height }: { size: number; file: string; width: number; height: number }): string`
  — `${formatBytes(size)} .${ext} (${width}×${height})`, `ext` from the text after the last `.`
  in `file`'s last `/`-segment, lower-cased; the `.${ext}` term (and its leading space) is
  omitted when the name has no `.`. Called as `formatSizeLine(image)` — `ImageRecord` already
  has all four fields, so the object literal type isn't reconstructed at the call site.
- `formatRelative(ms: number, now: number = Date.now()): string` — largest of
  year(365d)/month(30d)/day/hour/minute whose magnitude is ≥ 1, via
  `Intl.RelativeTimeFormat('en', { numeric: 'auto' })`; under a minute reads `just now`; a
  non-finite `ms` reads `—`. `now` is a parameter so tests can fix it; production calls
  (`formatRelative(image.capturedAt)`) rely on the default.

**Date row's tooltip mount** (`Inspector.svelte`, the `dl`'s Date row): `Tooltip.Root
delayDuration={150}` → `Tooltip.Trigger tabindex={-1}` with a `{#snippet child({ props })}`
`<span {...props}>{formatRelative(image.capturedAt)}</span>` trigger → `Tooltip.Content
portalProps={{ to: portalTo }} class="block text-left"` holding three `<p>` lines (`Captured
…`, `Imported …`, `File modified …`, each through `formatTimestamp`, the `File modified` one
still `?? Number.NaN`). `class="block text-left"` overrides the component's own
`inline-flex items-center` default (via `twMerge`) so the three `<p>`s stack instead of sitting
in a row — `TagNoteIndicator` didn't need this override since its content is one text node, not
three lines.

**Deviation from tasks.md, not from design.md**: `Inspector.svelte.test.ts` (existing component
test harness, not named in 1.1–1.2 but broken by 1.2's markup change) mounted `Inspector`
directly with no `Tooltip.Provider` ancestor; the Date row's now-unconditional `Tooltip.Root`
throws `Context "Tooltip.Provider" not found` without one, same reason
`TagNoteIndicator.test-harness.svelte` exists. Wrapping `Inspector` itself in a `Tooltip.Provider`
(the obvious fix) breaks a real invariant several tests rely on: `instance.startEditTags()` is
called synchronously, before any flush, in the same tick `mount()` returns (the "right after
mount" and focus-handback tests depend on this landing before the panel's own reset effect's
first pass) — a wrapper component's `bind:this` into `Inspector` is itself effect-based and
isn't populated until the next flush, so a forwarded call through it silently no-ops once and is
never retried. Fixed instead with two new test-only files: `CaptureTooltipContext.test-harness.svelte`
(a bare child of `Tooltip.Provider` that calls `getAllContexts()` — Svelte's own public,
documented API "for handing existing context to a programmatically-mounted component" — during
its own init) and `Inspector.test-harness.svelte`, now repurposed as the `Tooltip.Provider` +
capture wrapper (not an `Inspector` wrapper). `Inspector.svelte.test.ts`'s `setup()` captures
the context Map once (module-level `captureTooltipContext()`) and passes it as `mount(Inspector,
{ target, props, context })`'s `context` option, so `Inspector` stays the literal mounted root —
zero change to every other test's timing. Verified against the original (pre-change) suite:
same 13 tests, same assertions, all still pass.

**Hand check should watch**: the Date row's tooltip in both placements (beside the grid and
inside the viewer) — hovering shows all three absolute times, and the trigger is not in the Tab
order (`tabindex={-1}`, same as a tag note's glyph). The Size row's extension case for an image
whose stored file has no extension (rare — a legacy import) reads `1.3 MB (1200×2200)` with no
stray space before the parenthesis.
