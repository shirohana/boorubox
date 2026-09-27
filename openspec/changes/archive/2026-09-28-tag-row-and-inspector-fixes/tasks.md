> One unit, U (webview, `packages/app`), Sonnet; retried on Opus if the gate fails; one Opus
> review of the whole change. Design D1–D7 decide every shape; do not re-decide them. The
> wrong-image fix and its test (1.1–1.2) go first: they are the one bug here, and 1.2 builds
> the component-mount harness that 1.5's test reuses. Gate: `pnpm lint && pnpm typecheck &&
> pnpm test`, run from the repo root (tests run per package through `pnpm -r`). No unit commits
> or ticks a hand check.

## 1. Unit U — the row cluster, the `?` look-up, the facts pencil, the editor reset (`packages/app`)

- [x] 1.1 `components/library/Inspector.svelte` per `tag-row-and-inspector-fixes` D6:
      `const imageId = $derived(image?.id)` and one `$effect` that closes both editors
      (`editingTags`, `error`, `editingFacts`, `factsError`) when `imageId` differs from the id
      it last saw, recording without resetting on its first run; the facts form's own effect
      (~:316-326) is removed into it. The comment above the effect says why it is keyed on the
      id and not `updatedAt`, and why the first run only records. Verify: 1.2's tests.
- [x] 1.2 `components/library/Inspector.svelte.test.ts` (new, `// @vitest-environment jsdom`)
      per D7, `mockIPC`/`clearMocks` as `lib/fullscreen.svelte.test.ts` uses them; the
      `vite.config.ts` browser condition only if `mount` needs it. Tests:
      `closes the tag editor and writes nothing when the image changes` (open via
      `startEditTags`, type `dog`, assign another image, `flushSync` → no textarea on screen,
      `saveTags` never called); `keeps the tag editor open when the same image gets a new
      record` (same id, later `updatedAt` → textarea still there, still reading the typed text);
      `keeps an editor opened right after mount` (mount, `startEditTags` after `tick()` → open —
      the `e` path D6 protects); `closes the facts form when the image changes`.
- [x] 1.3 `components/library/Inspector.svelte` per D5: the facts `<dl>` inside a `section` with
      a "Details" heading row and the second pencil (same `aria-label`, `title` and
      `startEditFacts`, hidden while `editingFacts`); the title row's pencil unchanged. Test in
      1.2's file: `the facts heading's pencil opens the form` (two buttons named "Edit title and
      addresses" before, clicking the second → the Title input has focus and neither button is
      on screen).
- [x] 1.4 `components/tags/danbooru-open.ts` (new) per D4: `openDanbooruLookup(lookup)`, the
      FIXME moved onto it and reworded for both callers; `TagVocabularyMenuItems.svelte`'s item
      calls it and loses its own FIXME. Test `danbooru-open.test.ts`: with `mockIPC` answering
      the opener plugin's open command, `openDanbooruLookup` sends the lookup's URL; with the
      command throwing, the call neither throws nor leaves a rejected promise.
- [x] 1.5 `components/tags/FilterRow.svelte` per D1–D3: the button group with `gap-0`, the
      optional `lookup` prop and the `?` button first in the group; the header comment gains
      one sentence that the look-up is optional because collections have no Danbooru page.
      `components/tags/TagSidebar.svelte`: `tagLookup(name)` per D2, passed to every tag row.
      `CollectionsSection.svelte` untouched. Test `FilterRow.svelte.test.ts` (jsdom, 1.2's
      harness): no `lookup` → the row has exactly two buttons before the name and none reads
      `?`; with `lookup` → the first button reads `?`, is titled the label, is labelled
      `"{label}: {name}"`, and a click calls `open` once and none of `oninclude`, `onexclude`,
      `ontoggle`.
- [x] 1.6 Gate green. Handoff: whether `vite.config.ts` needed the browser condition; what the
      mount needed from `mockIPC` (the commands answered), so the next component test starts
      from it; anything that differs from D1–D7.
- [ ] 1.7 Hand check (owner): the `?` opens the right Danbooru page per category; the cluster
      reads as one group in both themes; the facts heading's pencil behaves like the title
      row's; switching images closes an open tag editor from the grid and from the viewer.
      Hand check: in the sidebar press `?` on a general tag (e.g. `solo`) and confirm the
      browser opens its Danbooru wiki page, then on an artist tag (red text) and confirm it
      opens Danbooru's artist search for that name; confirm collection rows have no `?`. Look
      at the rows in light and in dark theme: `? + −` sit as one tight group with the usual gap
      before the name, glyphs the same size, the row no taller than before; Tab walks `?`,
      `+`, `−`, name. In the inspector, scroll to the facts and confirm the "Details" heading
      (say if you want another word) with a pencil; pressing it turns the rows into the form
      with the Title field focused and in view, and both pencils disappear; the title row's
      pencil still does the same. Open a tag editor, type a tag, click another tile: the
      editor closes, the new image's tags show read-only, and neither image carries the typed
      tag. Repeat in the viewer's inspector with `→`/`←`. Choose a rating while editing tags:
      the editor stays open with the typed text.
      Seen (lead's smoke 2026-09-28): `?` tooltips read "Open Danbooru wiki" on general `blue_archive` and "Search artist on Danbooru" on artist `someone` (18-question-tooltip-general.png, 26-question-tooltip-artist.png); clicking `?` brought Chrome to the front, but the URL it opened was not read (the agent may not read browser tabs); collection rows have no `?` (27-collections-expanded.png); `? + −` sit as one tight group, same glyph size, in dark and light (17-library.png, 36-rows-light.png); Tab walks `?`, `+`, `−`, name (28- to 31-tab-walk-*.png). The "Details" pencil opens the form with Title focused and in view, both pencils gone; the title pencil does the same (34-details-pencil-form.png, 35-title-pencil-form.png). Grid: typed a tag, clicked another tile, the editor closed read-only and no image got the tag (32-grid-editor-typed.png, 33-grid-editor-after-switch.png); viewer: `→` closed the editor, neither image got the tag (20-viewer-after-arrow-with-typed-tag.png); choosing a rating kept the editor open with its text (24-viewer-rating-while-editing.png). Problem: in the viewer, once the editor closes on `→` (focus was on its Cancel button), `←`/`→` do nothing until a click inside the viewer; reproduced twice (21-viewer-back-no-typed-tag.png, 71-viewer-arrows-dead-after-editor-close.png; after a click they work, 23-viewer-arrow-after-click.png). No error in the dev log. Also: with a long file-name title the title-row pencil is clipped at the inspector's right edge (25-seeded-artist-tag.png).
      Seen (lead's re-smoke 2026-09-28, at deab612): in the viewer, `e` opened the editor, one Tab focused Cancel (re-03-viewer-focus-cancel-before-arrow.png), `→` changed the image and closed the editor (re-04-viewer-after-arrow-editor-closed.png); with no click, `→` then went to the next image (re-05-viewer-arrow-right-again.png) and `←` came back (re-06-viewer-arrow-left.png). The arrows-dead-after-close problem did not recur.

## Handoff

- **`vite.config.ts` did need the browser condition.** Without it `mount(...)` throws
  `lifecycle_function_unavailable` — vite-plugin-svelte resolves `svelte` to its server build
  under Vitest by default. Added exactly D7's recipe:
  `resolve: process.env.VITEST ? { conditions: ['browser'] } : undefined`. The rest of the
  suite (all pre-existing store tests) still passes unchanged under it.
- **What the mount needed from `mockIPC`.** With the fixture image carrying no tags and no
  collections (`domain/image-fixture.ts`'s `img()` defaults), `Inspector` never reaches
  `vocabulary`/`collections`/`booruSites` commands at mount — those stores start empty and
  nothing in the render path calls their `refresh()`. The one command a test can reach is
  `tag_suggestions`, fired by `TagInput`'s `oninput` the moment a keystroke lands in the tag
  editor. `setup()` in `Inspector.svelte.test.ts` answers it with `[]` and returns `null` for
  anything else; no test needed more than that. A future component test that gives the fixture
  image actual tags or collections will start pulling in `ContextMenu`/`DropdownMenu` bits-ui
  trees and should budget time for that, per the design's own risk note.
- **Deviations from D1–D7:** none in shape. One implementation detail D6 leaves open: "the
  first run only records" is a boolean flag (`sawFirstImageId`), not a sentinel compared
  against `imageId`, since `imageId` is itself legitimately `undefined` on a later run (no
  image selected) and a sentinel of `undefined` would misfire there.
- **Gate: lint green, `pnpm test` green (65 files / 750 tests in `packages/app`, all packages
  green from the root).** `pnpm typecheck` is currently red, but not from anything in this
  unit's files: `svelte-check` reports 10 errors, all in `packages/app/src/lib/api/commands.test.ts`
  and `packages/app/src/lib/api/vocabulary.svelte.test.ts` ("Property 'note' is missing in type
  ... but required in type 'TagEntry'"), from `packages/shared/src/index.ts` being edited live
  by another unit while this one ran (confirmed via `git status`: that file and those two tests
  are outside this unit's ownership and untouched by it). Re-ran typecheck a second time after
  a pause; the errors were unchanged, so whoever owns that edit still has it in flight — worth
  a final `pnpm typecheck` once every unit has landed.
- Reviewer note: `Inspector.svelte`'s facts section is now a `<section>` wrapping the `<dl>`
  (D5); the `<dl>`'s own border/padding moved to the `<section>` and its rows gained `mt-2` in
  place of the border. Everything inside the `<dl>` (title/page/image address rows, the facts
  Save/Cancel row) is unchanged content, just re-indented one level.

- Opus review of `db74818` (lead's note, 2026-09-28): no correctness findings. Test fixes to
  apply once `selection-by-id` has landed (it holds `Inspector.svelte.test.ts` until then):
  (1) "keeps an editor opened right after mount" cannot fail — `mount` runs no effects and
  `await tick()` flushes the first run before `startEditTags()`; call `startEditTags()`
  synchronously right after `mount`, then `flushSync()`, so the test fails without the
  `sawFirstImageId` guard. (2) the `saveTags not called` assertion is vacuous (nothing clicks
  Save) — assert no Save button exists after the swap instead. (3) optional: the refusal test
  in `danbooru-open.test.ts` should also record that `plugin:opener|open_url` was reached.
