> Four units. A (tile cap), B (inspector) and C (sidebar) run in parallel on disjoint files;
> D (`e` key) starts after B lands, since both touch `Inspector.svelte`. Sonnet each, retried
> on Opus if the gate fails; B and C get an Opus review before D starts. Design D1–D9 decide
> every shape; do not re-decide them. Gate for every unit: `mise run check` green. No unit
> ticks a hand check: an agent that cannot run the app writes what it saw under the
> `Hand check:` line and leaves the box open. Sibling changes in flight: `one-level-buckets`
> (Rust `library.rs`, `thumbs.rs`, `recover.rs`, `commands.rs`, `lib.rs`; Settings → Library;
> `thumbnail-cache.ts`, `assets.ts`, `commands.ts`) and `pinned-tag-groups` (after B and D:
> `db.rs`, `tags.rs`, `commands.rs`, shared `TagEntry`, `vocabulary.svelte.ts`,
> `TagVocabularyMenuItems.svelte`, `Inspector.svelte`). Unit A's `model.rs` edit and
> `one-level-buckets`' `commands.rs` edits are additive; rebase, never merge by hand.

## 1. Unit A — the tile cap (`packages/shared`, `packages/app`, `packages/app/src-tauri`)

- [x] 1.1 `GRID_TILE_MAX = 640` in `packages/shared/src/index.ts` and `model.rs` (D1); the
      doc comment on each names the other as its twin and the reason (2K monitors). Test:
      `grid-window.test.ts` clamps `1000` to `640` (extend the existing clamp test); `model.rs`
      settings clamp test, if one names 360, names 640. Verify: `mise run check` green.

## 2. Unit B — the inspector (`packages/app`: `Inspector.svelte`, `booru/UploadAction.svelte`)

- [x] 2.1 `UploadAction` gains `part: 'action' | 'hint'` (D2): `'action'` renders the button
      or dropdown and nothing when the list is empty or unreadable; `'hint'` renders the
      "No booru configured" paragraph and the could-not-read error and nothing otherwise. The
      component's head comment says why one component has two mounts.
- [x] 2.2 `Inspector.svelte` single-image branch reordered per D2: header (unchanged);
      Rating; Tags; Collections; `<UploadAction part="action">` (library view only, as
      today); facts `<dl>` and Posted (moved as one block, comments kept); the actions
      section with `mt-auto` (Move to trash / Restore + Delete forever);
      `<UploadAction part="hint">` last, library view only. The facts block's `border-t`
      follows it. Selection branch untouched.
- [x] 2.3 Snippet `tagSearchItems(tag)` (D3): the two `ContextMenu.Item`s lifted out of the
      Tags list's menu, rendered there and in `pinnedChip`'s menu for a tag chip above
      `TagVocabularyMenuItems` with a `ContextMenu.Separator` between.
- [x] 2.4 `export function startEditTags()` (D9's hook): the existing function, exported;
      no other change. Verify: `mise run check` green.
- [ ] 2.5 Hand check: the panel reads title, rating, tags, collections, upload, facts, posted,
      move to trash; with no booru the hint is last; right-click on a pinned chip offers
      Search / Exclude and they rewrite the query; the same in the viewer and over a selection.
      Hand check: run the app against a library with a booru configured and confirm the
      order beside the grid and inside the viewer reads title, Rating, Tags, Collections,
      Upload (button or dropdown), the facts `dl`, Posted (if any), Move to trash — then
      unconfigure every booru (or open a library with none) and confirm no bordered gap
      appears where Upload was and the "No booru configured" paragraph is the very last
      thing in the panel, below Move to trash. Right-click a pinned tag chip (in the
      single-image panel, the viewer, and over a multi-selection) and confirm "Search for
      this tag" / "Exclude from the search" appear above a separator and a
      Pin/Unpin-labelled vocabulary item, and that choosing them rewrites the query the same
      way clicking a plain tag does. Also worth a look: the Upload dropdown for 2+ unposted
      sites, and a booru list that fails to read (the destructive-red error line, not the
      muted hint, should show for `part="hint"` and nothing for `part="action"`).
      Seen 2026-09-24 (smoke, scratch copy of test-1, HanaBooru configured): beside the grid the panel reads title, Rating, Tags, Collections, Upload to HanaBooru, the facts list, Move to trash at the foot (/Users/shirohana/.claude/jobs/8dbef24a/tmp/07-inspector-panel.png); the viewer's inspect mode shows the same order (/Users/shirohana/.claude/jobs/8dbef24a/tmp/38-viewer-e.png). Right-click on a pinned chip offers Search for this tag / Exclude from the search above a separator and the Unpin/group items, beside the grid (/Users/shirohana/.claude/jobs/8dbef24a/tmp/14-chip-menu.png), in the viewer (/Users/shirohana/.claude/jobs/8dbef24a/tmp/40-viewer-chip-menu.png) and over a two-image selection (/Users/shirohana/.claude/jobs/8dbef24a/tmp/47c.png); Exclude on `has` set the query to `-has`, then Search on `blue_archive` made it `-has blue_archive` (/Users/shirohana/.claude/jobs/8dbef24a/tmp/26-exclude-has.png, /Users/shirohana/.claude/jobs/8dbef24a/tmp/28c.png). Not seen: the no-booru hint as the last line. The only way to remove the booru was Settings → Remove, and its dialog says the API key is deleted from this computer's credential store, which the owner's libraries share, so it was cancelled. Also not seen: the 2+ sites dropdown and the list-read error.

## 3. Unit C — the sidebar (`packages/app`: `frame/Sidebar.svelte`, `library/ViewControls.svelte`, `tags/TagSidebar.svelte`, `tags/CollectionsSection.svelte`, `notes/NotesPanel.svelte`, new `tags/FilterRow.svelte`, new `common/SectionResizer.svelte` + `common/section-resizer.ts`)

- [x] 3.1 `frame/Sidebar.svelte`: `Sidebar.Rail` and its comment removed (D4); the Rail's
      comment's point about copy-in cursors moves to nowhere — the `app-frame` delta records
      the decision. `mt-auto` moves from `Sidebar.Menu` to the Notes wrapper `div`, comment
      amended (D5).
- [x] 3.2 `ViewControls.svelte`: both triggers `class="h-7 w-full text-xs"` (D6).
- [x] 3.3 `tags/FilterRow.svelte` per D7, rendered by `TagSidebar` and `CollectionsSection`;
      the collection list's `gap-0.5`, `p-0.5` and `py-1` go. Existing tests still pass
      (`TagSidebar` has none of its own; `sidebarRows` tests unaffected).
- [x] 3.4 `common/section-resizer.ts`: `clampHeight(start: number, delta: number, min: number,
      max: number): number` (`delta` positive = pointer moved up = taller); test
      `section-resizer.test.ts` (grows on an upward drag, shrinks on a downward one, clamps at
      both ends, `NaN` start reads as `min`). `common/SectionResizer.svelte` per D8 with
      pointer capture; a11y: `role="separator"` `aria-orientation="horizontal"`
      `aria-label` from a prop.
- [x] 3.5 `CollectionsSection`: the list box loses `rounded-md border border-border resize-y
      h-32`, gains `style:height="{height}px"` from `let height = $state(128)`, and
      `<SectionResizer>` as the section's first child (above the heading row) with
      `min={40}` `max={window.innerHeight / 2}` computed at drag start (the `max-h-[50vh]`
      class stays as the CSS ceiling). Comment on the box names D8 and the session-only
      height. `NotesPanel`: textarea `resize-none`, `style:height`, `let height = $state(96)`,
      `<SectionResizer>` above the trigger row, `min={96}`. Verify: `mise run check` green.
- [ ] 3.6 Hand check: no edge click on the sidebar; Settings shows the note directly above
      the nav; the Filter selects are shorter; collection rows sit at the tag rows' height;
      dragging the Collections top edge up grows the list and shrinks the tags; dragging the
      Notes top edge up grows the textarea; neither shows a corner handle; light and dark.
      Hand check: run the app, open a library with more than a screenful of tags. Confirm
      clicking the strip of border between the sidebar and the grid does nothing (no
      collapse, plain arrow cursor). Open /settings and confirm the note sits directly above
      the nav with empty space above it. Confirm the two Filter selects are visibly shorter
      than before and the collection rows are the same height/text size as the tag rows,
      with no border around the collection list. Drag the hairline above "Collections"
      upward: the list should grow and the tag list above it should shrink by the same
      amount (and back down on a downward drag), clamped so it never exceeds half the
      window height nor drops below ~40px. Drag the hairline above "Notes" the same way,
      floor ~96px. Neither box should show its old bottom-right corner resize handle.
      Check both light and dark themes; the hairline should be invisible at rest and show
      only on hover/while dragging.
      Hand check (review-fix pass): confirm the Filter selects use `data-[size=sm]:h-6`
      (a plain `h-6` loses to the copy-in's `data-[size=sm]:h-7`) and are visibly shorter;
      confirm a Collections/Notes drag still ends cleanly when the pointer leaves the
      window or the drag is cancelled (`onlostpointercapture`, not `onpointerup`).
      Seen 2026-09-24 (smoke, scratch copy of test-1, dark theme only): a click on the sidebar/grid border does nothing (/Users/shirohana/.claude/jobs/8dbef24a/tmp/50-edge-click.png). /settings shows Notes directly above the nav with empty space above it (/Users/shirohana/.claude/jobs/8dbef24a/tmp/02-settings.png). The Filter selects are 24px high (AX size 223×24). Collection rows are on the same 19px pitch as tag rows, with no border (/Users/shirohana/.claude/jobs/8dbef24a/tmp/51-collections-notes-open.png). ⌘B icon rail: the nav icons sit at the bottom (/Users/shirohana/.claude/jobs/8dbef24a/tmp/56-icon-rail.png). A real drag (CGEvent, pointer moved 100px up) on the Collections hairline moved it up 80px: the list grew and the tag list shrank (/Users/shirohana/.claude/jobs/8dbef24a/tmp/53-after-collections-drag-up100.png). A 50px drag down moved it 30px. The Notes hairline dragged up 100px grew the textarea from 96 to 196px. No corner handle on either box (/Users/shirohana/.claude/jobs/8dbef24a/tmp/54b-sidebar-overflow-fullres.png). DEFECT? With Collections enlarged and then Notes enlarged, the upper sidebar overflows: the Filter heading is clipped, both Filter selects sit under the Notes block out of view, and the upper area gains a scrollbar that narrows Search and Rating from 223 to 206px (/Users/shirohana/.claude/jobs/8dbef24a/tmp/54-after-notes-drag-up100.png). Not checked: light theme, the pointer leaving the window mid-drag, and the hairline hover state at rest.
      Ceiling fix: growing Collections then Notes must stop when the tag list is at its 128px floor; the Filter selects stay visible.

## 4. Unit D — the `e` key (`packages/app`: `lib/keyboard.ts`, `library/LibraryScreen.svelte`, `library/Lightbox.svelte`), after B

- [x] 4.1 `keyboard.ts`: `KEY_EDIT_TAGS = 'e'` with its doc comment (the keyboard rule), the
      two `KEYBOARD_MAP` rows after each `I` row (D9). `keyboard.test.ts`: the map lists `E`
      for Grid and Viewer, if the file tests the map's rows.
- [x] 4.2 `LibraryScreen.svelte`: `let inspector = $state<Inspector | null>(null)`,
      `bind:this` on the grid-side `<Inspector>`; `screenKeys` handles `KEY_EDIT_TAGS` per D9
      (focused image, `selection.count <= 1`, open the inspector, `tick`, `startEditTags`,
      `preventDefault`). `Lightbox.svelte`: `bind:this` on its `<Inspector>`; `onkeydown`
      handles `KEY_EDIT_TAGS` per D9. Verify: `mise run check` green.
- [ ] 4.3 Hand check: `e` in the grid with the inspector hidden opens it with the editor
      focused at the end of the text; `e` in the viewer enters inspect mode with the editor
      open; `e` with three selected does nothing; `e` typed in the search field types.
      Hand check: run the app. With the inspector hidden, focus a card in the grid and press
      `e` — confirm the panel opens beside the grid, its tag editor is open, and the caret
      sits at the end of the draft text. Close it, open the viewer on an image, press `e` —
      confirm it enters inspect mode with the tag editor open the same way. Select three
      images (grid or viewer selection strip) and press `e` — confirm nothing happens (no
      panel state change, no editor). Click into the sidebar's tag search field and press
      `e` — confirm it types the letter rather than doing anything else.
      Seen 2026-09-24 (smoke, scratch copy of test-1): with the inspector hidden and a tile focused, `e` opened the panel with the tag editor focused and the caret after the trailing space (/Users/shirohana/.claude/jobs/8dbef24a/tmp/30-inspector-hidden.png → /Users/shirohana/.claude/jobs/8dbef24a/tmp/31-e-opens-editor.png). In the viewer (Enter on the tile), `e` entered inspect mode with the editor open the same way (/Users/shirohana/.claude/jobs/8dbef24a/tmp/38-viewer-e.png). With three selected, `e` changed nothing: the screenshots before and after are byte-identical (/Users/shirohana/.claude/jobs/8dbef24a/tmp/48-three-selected.png, /Users/shirohana/.claude/jobs/8dbef24a/tmp/49-three-selected-after-e.png). `e` in the sidebar search field typed `e` (/Users/shirohana/.claude/jobs/8dbef24a/tmp/29-e-in-search.png).

## Handoff (unit D)

- Landed: `KEY_EDIT_TAGS = 'e'` in `packages/app/src/lib/keyboard.ts` with its doc comment
  and two `KEYBOARD_MAP` rows (`{ where: 'Grid', keys: ['E'], action: "Edit the focused
  image's tags" }` right after the `Grid`/`I` row, `{ where: 'Viewer', keys: ['E'], action:
  "Edit this image's tags" }` right after the `Viewer`/`I` row). `keyboard.test.ts` does not
  assert on `KEYBOARD_MAP`'s contents today (no test imports it), so per 4.1's "if the file
  tests the map's rows" no test was added there.
- `LibraryScreen.svelte`: `let inspector = $state<Inspector | null>(null)` beside the
  existing `grid` state; `bind:this={inspector}` on the grid-side `<Inspector>`; `screenKeys`
  gained a branch after the `Escape`/clear-selection one — `if (event.key === KEY_EDIT_TAGS
  && focused && selection.count <= 1) { preventDefault(); browseSession.inspectorOpen =
  true; void tick().then(() => inspector?.startEditTags()) }`.
- `Lightbox.svelte`: `let inspector = $state<Inspector | null>(null)`; `bind:this={inspector}`
  on its `<Inspector>`; `onkeydown` gained an `else if (event.key === KEY_EDIT_TAGS)` branch
  beside the `KEY_INSPECT` one — `preventDefault(); mode = 'inspect'; void
  tick().then(() => inspector?.startEditTags())`. Imported `tick` from `'svelte'` (Lightbox
  had none before; `onDestroy` was the only import from there).
- No deviation from D9 or the task text.
- Gate: `pnpm --filter @boorubox/app test` — 695/695 green (unchanged count: no new test
  files, since neither `keyboard.test.ts` nor any file in D's scope owns a test that needed
  extending). `npx eslint` on the four owned files — clean. `pnpm --filter @boorubox/app
  typecheck` — 0 errors (unit C's `SectionResizer.svelte` import that 2.4/B's handoff
  flagged as broken has since landed; typecheck is clean repo-wide). Full `mise run check`
  fails only at `cargo fmt --check` inside `packages/app/src-tauri/src/commands.rs`,
  `library.rs`, `thumbs.rs` — the concurrent Rust unit's (`one-level-buckets`) in-flight
  formatting, not a file unit D touched; `pnpm lint`'s `eslint .` step (which runs before
  the `cargo fmt` step and covers the whole repo, not just D's files) passed with no output.
- Reviewer, look first at: the two `screenKeys`/`onkeydown` branches above (`keyboard.ts`
  lines defining `KEY_EDIT_TAGS`, `LibraryScreen.svelte`'s `screenKeys`, `Lightbox.svelte`'s
  `onkeydown`) against D9's plan; task 4.3's hand check is the only way to see the caret
  land at the end of the tag text and confirm the multi-selection no-op, since nothing here
  runs the app.

## Handoff (unit B → unit D)

- `Inspector.svelte` exports `startEditTags(): void` (Svelte 5 `export function`, no
  signature change from the private version). It already guards `!image` and returns
  early, so calling it with no image focused/selected is a no-op — D9's plan (call it after
  setting `inspectorOpen = true` and `await tick()`) works as written. It opens the tag
  editor and, after the field mounts, moves the caret to the end of the draft text.
- `bind:this` will work on both mounts (grid-side and the viewer's) since it's the same
  component; nothing about the reorder or the `part` split changes its public surface
  otherwise — `image`, `results`, `selection`, `actions`, `tagQuery`, `onquery`,
  `onrelease`, `onactivate`, `onedit` are all unchanged.
- Unrelated to D, but worth knowing: the single-image branch's section order changed
  (header, Rating, Tags, Collections, Upload, facts `dl` + Posted, trash actions, Upload
  hint) — if D's hand check screenshots or describes panel layout, this is the new order.

## Handoff (unit B → owner / next reviewer)

- `UploadAction` now takes a required `part: 'action' | 'hint'` prop; every call site must
  pass it. The only call sites are the two new mounts in `Inspector.svelte`.
- Deviation from a literal reading of D2: D2 doesn't say the two `UploadAction` mounts get
  their own `border-t border-border px-4 py-3` wrapper. I added one, owned by
  `UploadAction.svelte` itself (not `Inspector.svelte`), and made it conditional on there
  being content to show. Reason: `Inspector.svelte` cannot decide whether to draw a
  divider/padding wrapper around the mount without reading `booruSites` itself, which is
  exactly the duplicate read D2 says to avoid ("one reading of `booruSites`... in one
  place"); and an *unconditional* wrapper in `Inspector.svelte` would leave a visible empty
  bordered strip between Collections and the facts block whenever no booru is configured,
  which contradicts the "no upload action is drawn between the collections and the facts"
  scenario in `specs/app-frame/spec.md`. Giving `UploadAction` its own section wrapper,
  shown only when it has a button or a paragraph to draw, keeps that scenario true and
  matches every other section's `border-t border-border px-4 py-3` convention. Flagging
  this since it's a real (if small) design choice, not just a restatement of D2.
- The facts `<dl>` gained `border-t border-border` (it used to sit directly under the
  header's own `border-b` with no divider of its own; now that Rating/Tags/Collections/
  Upload sit between them, it needs one). Rating/Tags/Collections classes are untouched.
- `mise run check` was not run to completion: `pnpm --filter @boorubox/app typecheck` fails
  on `packages/app/src/lib/components/tags/CollectionsSection.svelte` (`Cannot find module
  '$lib/components/common/SectionResizer.svelte'`) — unit C's in-flight file (task 3.4/3.5),
  not mine. Verified my own scope instead: `npx eslint` on `Inspector.svelte` and
  `UploadAction.svelte` is clean (0 errors, 0 warnings after a couple of `--fix`s for line
  length and Tailwind class order); `pnpm --filter @boorubox/app test` is green, 695/695;
  a full-repo `npx eslint .` shows only pre-existing warnings in unit C's `ViewControls.svelte`.
  Owner should re-run `mise run check` once units C and the Rust unit land.

## Handoff (unit C → owner / next reviewer)

- `FilterRow.svelte` props: `name: string`, `count: number | string` (the collection list
  passes `''` while a search runs), `mark: SearchMark`, `nameClass?: string` (the tag list's
  category colour; the collection list passes nothing), `oninclude: () => void`,
  `onexclude: () => void`, `ontoggle: () => void`, `menu: Snippet` (the context menu's
  content). It owns the `<li>`, the `ContextMenu.Root`/`Trigger`/`Content`, the include /
  exclude / toggle buttons and the count — no handler logic moved, both callers still own
  their own `onquery` calls.
- `SectionResizer.svelte` props: `height: number`, `min: number`, `max: number`,
  `onresize: (next: number) => void`, `label: string` (the `aria-label`). It renders one
  `h-1.5` strip with `role="separator"` `aria-orientation="horizontal"`, captures the
  pointer on `pointerdown`, and calls `onresize(clampHeight(...))` on `pointermove` — `min`,
  `max` and the drag's start height are all captured at `pointerdown`, so a prop change
  mid-drag can't jump the section. `clampHeight(start, delta, min, max)` lives in
  `common/section-resizer.ts`, `delta` positive meaning the pointer moved *up*; a `NaN`
  `start` returns `min` outright, ignoring `delta`.
- Both `CollectionsSection` and `NotesPanel` only render their `SectionResizer` while
  `expanded` is true (collections/notes specs both scope the draggable height to "Unfolded" /
  "Expanded" — nothing to drag while the section is folded away).
- **Deviation from the literal task text**: 3.5 says `NotesPanel`'s textarea gets
  `style:height`. Svelte does not allow a `style:` *directive* on a component — `Textarea` is
  one, wrapping a native `<textarea>` — and `svelte-check` fails with
  `component_invalid_directive` if it's used there. Built the closest thing instead: a plain
  `style="height: {height}px"` prop, which `Textarea` forwards to the real element through
  its own `...restProps` spread the same way `placeholder`/`aria-label` already do. The
  `CollectionsSection` box (a plain `<div>`) does use the `style:height` directive as written,
  since a directive is valid there.
- `mise run check`: the lint step fails on `cargo fmt --check` inside `library.rs` /
  `thumbs.rs` — entirely the concurrent Rust unit's (`one-level-buckets`) in-flight
  formatting, not a file this unit touched. Ran the frontend gates directly instead, all
  green: `npx eslint packages/app/src/lib/components` (0 errors/warnings, includes every file
  this unit owns plus the new `FilterRow.svelte`/`SectionResizer.svelte`/
  `section-resizer.ts`); a full-repo `npx eslint .` also passed (so nothing here conflicts
  with unit B's or unit A's files either); `pnpm --filter @boorubox/app typecheck` — 0
  errors; `pnpm --filter @boorubox/app test` — 695/695 passed, including the new
  `section-resizer.test.ts`. Owner should re-run `mise run check` once the Rust unit lands
  and `cargo fmt` is clean.
- Reviewer, look first at: `common/section-resizer.ts` (the clamp math and its test),
  `common/SectionResizer.svelte` (the pointer-capture DOM half — no test, per D8, it's task
  3.6's hand check), and `tags/FilterRow.svelte` against the old markup in `TagSidebar.svelte`
  and `CollectionsSection.svelte`'s git history to confirm nothing besides the shared-row
  extraction changed (no handler moved, no behaviour changed).
