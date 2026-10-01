> One unit, Sonnet, retried on Opus if the gate fails. Design D1–D2 decide every shape; do not
> re-decide them. Owns `packages/app/src/routes/settings/library/+page.svelte` and
> `packages/app/src/lib/components/tags/CollectionsSection.svelte` only. Gate: `pnpm lint &&
> pnpm typecheck && pnpm --filter @boorubox/app test`. Does not commit; does not tick the hand
> checks.

## 1. Unit P — two class changes (`packages/app/src`)

- [x] 1.1 `routes/settings/library/+page.svelte`: `ml-auto` on the Regenerate thumbnails,
      Clear playback samples and Rebuild library index buttons per D1; the comment over the
      first row says why in one line (a wrapped line holds one item, and `justify-between`
      starts it).
- [x] 1.2 `components/tags/CollectionsSection.svelte`: the `Collapsible.Trigger` gains
      `min-w-0 flex-1` per D2; the "+" stays its sibling. The header comment on the fold
      says the row is the target, as `NotesPanel`'s is.
- [x] 1.3 Gate green. Handoff: anything the design did not foresee.
- [ ] 1.4 Hand check (owner): narrow the window until a Library-page button wraps under its
      description; it sits at the right. In the sidebar, click the empty space right of
      "Collections"; the section folds; the "+" still creates without folding.
      Seen (lead's smoke 2026-10-02): at 860px the three Library buttons wrapped under their descriptions and sat flush right (boorubox-vault/smoke-2026-10-01/19-library-narrow.png). Clicking the empty space right of “Collections” unfolded the section (boorubox-vault/smoke-2026-10-01/02-collections-row-click.png); the “+” was not pressed in this run.

