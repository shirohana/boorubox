## Context

The library screen after `pin-and-toggle-placement` (2026-09-24, `5026bd0`): the inspector is
`Inspector.svelte` (title row, facts list, Rating, Tags, Collections, Posted, actions with
`mt-auto`); the sidebar frame is `frame/Sidebar.svelte` with `Sidebar.Rail`; the filter region
renders `ViewControls`, `TagSidebar`, `CollectionsSection` in a flex column with `TagSidebar`
the one `flex-1`; `NotesPanel` is mounted by the frame under the region. The keyboard map is
`lib/keyboard.ts` (`KEY_INSPECT = 'i'` bound in `LibraryGrid` and `Lightbox`). The grid tile
size is `GRID_TILE_MIN/MAX/DEFAULT` in `packages/shared/src/index.ts` and `model.rs`, clamped by
Rust on write and by `grid-window.ts` on read.

## Goals / Non-Goals

Goals: the nine asks in the proposal, each in the place it already lives. Non-goals: in the
proposal.

## Decisions

**D1. The cap is 640, in both constants, and nothing else about the tile changes.**
`GRID_TILE_MAX = 640` in `packages/shared/src/index.ts` and `model.rs`; `grid-window.ts`'s
clamp, the toolbar slider and the Settings slider read the constant. Existing thumbnails are
384px on the long edge and will look soft past 384 CSS px at DPR 1 until `one-level-buckets`
regenerates them at 768; that change owns the edge, this one only the cap. A test in
`grid-window.test.ts` pins the clamp at 640.

**D2. Inspector order follows use frequency; the title row stays as the anchor.** Owner's
order (2026-09-24): Rating, Tags, Collections, Upload, Info, Move to trash. The title row with
its pencil stays at the top (owner: "keeping the title row") because it answers "which image
is this" before anything else is read. Info is the facts `<dl>` and the Posted section, moved
as one block below Upload. The actions section keeps `mt-auto` so Move to trash (or Restore /
Delete forever in the trash) stays at the foot under a short panel. `UploadAction` splits by a
`part` prop, `'action' | 'hint'`: mounted once with `part="action"` after Collections, where it
renders the button(s) and nothing when no booru is configured; and once with `part="hint"`
after the actions row, where it renders the "No booru configured" paragraph (and the
could-not-read error) and nothing otherwise. One component keeps the one reading of
`booruSites` (the decision "is a booru configured") in one place; two mounts are just two
slots. The selection panel's order (thumbs, pinned tags, collections) is untouched.

**D3. The two search items are one snippet, shared by the tag list and the pinned chip.**
`Inspector.svelte` gains a snippet `tagSearchItems(tag)` rendering "Search for this tag"
(`query(toggleTagInQuery(tagQuery, tag))`) and "Exclude from the search"
(`query(excludeTagFromQuery(tagQuery, tag))`), rendered by the Tags list's menu (where the two
items are today) and by `pinnedChip`'s menu for `chip.kind === 'tag'`, above the vocabulary
items with a separator. Both placements of the panel and the selection panel get it, since
`tagQuery` and `query` are the same props everywhere the chip is drawn.

**D4. The rail goes; the toggle and ⌘B stay.** `Sidebar.Rail` and its cursor override leave
`frame/Sidebar.svelte`. This amends `browse-polish` design D10, which kept the rail's click and
only removed its resize cursor: D10's argument was against making the edge a *resizer*, not
for keeping the click, and the owner finds the click fires by accident (2026-09-24). What D10
required of the frame — no region edge shows a resize affordance — still holds, and the
`app-frame` delta keeps that sentence. `KEY_SIDEBAR` and the toolbar toggle are unchanged.

**D5. Notes and the navigation share the bottom.** In `frame/Sidebar.svelte` the `mt-auto`
moves from the nav menu to the Notes wrapper, so on a route with no filter region (Settings,
Trash, Import) the note and the nav sit together at the bottom above the footer. On the
library route `TagSidebar` is already `flex-1`, so nothing moves there. The spec's sidebar
order sentence (search, ratings, tags, collections, filter, note, nav, footer) is unchanged.

**D6. Filter selects go compact.** `ViewControls`' two `Select.Trigger`s take
`data-[size=sm]:h-6 text-xs` (the copy-in's `size="sm"` is already `h-7` via
`data-[size=sm]:h-7`; a plain `h-6` loses to that data-variant class, so the override has to
win the same way), the section's gap follows. Nothing else about the section changes. This is
a class edit, no test.

**D7. One row component for the tag list and the collection list.** `tags/FilterRow.svelte`:
props `name`, `count` (`number | string`, the collection list passes `''` while a search runs),
`mark` (the `searchMark` result), `nameClass` (the category colour for a tag, nothing for a
collection), `oninclude`, `onexclude`, `ontoggle`, and a `menu` snippet for the context menu's
content. Markup is the tag row's today (`px-1 text-xs`, `py-0.5` on the name, no list gap);
`CollectionsSection` and `TagSidebar` both render it, and the collection list's `gap-0.5` /
`p-0.5` go with the border. Reason: the two rows were the same eleven lines twice, and the ask
is exactly that they stop drifting. No behaviour changes: the include / exclude / toggle
handlers stay where they are, the row only draws them.

**D8. A section is resized by dragging its top edge.** New `common/SectionResizer.svelte`: a
full-width strip (`h-1.5 cursor-row-resize`, a hairline that shows on hover and while
dragging) rendered as the first child of the resizable section, with props `height`
(`number`, the section's current height in px), `min`, `max` and `onresize(next: number)`.
Pointer down captures the pointer (`setPointerCapture`), pointer move calls `onresize` with
`clampHeight(start - (clientY - startY), min, max)` — the handle is on the top edge, so
dragging *up* grows the section below it — pointer up releases. `clampHeight` lives in
`common/section-resizer.ts` with a vitest test; the DOM half is a hand check. The section
keeps its height as local `$state` (session-only, as the CSS handle's was — `browse-feedback`
design D4's session-only reasoning stands) and sets it as an inline `height` style on the
scrolling box (Collections: `min 40px`, `max 50vh` as today; Notes: the textarea, `min 96px`,
`max 50vh`, `resize-none`). The collection list loses `rounded-md border border-border`; the
handle's hairline is its top edge now. This amends `browse-feedback` design D4, which chose
the CSS corner handle: D4 called the handle "small and unlabelled" and accepted it because
it was the note's already; the owner finds it unusable on a section whose bottom is pinned
(2026-09-24, "a draggable handle should not pin itself in the same pos"). `max-h-[50vh]` on
the Collections box stays as the ceiling the clamp mirrors.

This amends itself further (smoke, 2026-09-24): `max={innerHeight / 2}` on Collections and
Notes are two independent ceilings, and each is legal on its own — dragging either section to
half the window is exactly what `max-h-[50vh]` promises. Dragged together, though, they can
claim the whole column between them, since neither knows what the other has already taken;
the tag list (`min-h-32`) absorbs the rest and the sidebar overflows below it. `max` on
`SectionResizer` takes a function as well as a number, called once at pointer-down (the room
is only known when a drag begins, and it depends on the other section's current height, which
`max` as a plain prop cannot read fresh per drag). Collections and Notes each pass
`max={() => Math.min(innerHeight / 2, height + roomAbove(tagList))}` —
`common/section-resizer.ts`'s `roomAbove`, `list.clientHeight` minus its computed
`min-height` — so a section may grow only into the room the tag list still has above its own
floor, on top of the height it already holds. The two sections' ceilings now share the tag
list's one budget instead of racing two independent halves.

**D9. `e` opens the tag editor from the grid and the viewer.** `KEY_EDIT_TAGS = 'e'` in
`keyboard.ts`, with two `KEYBOARD_MAP` rows ("Grid · E · Edit the focused image's tags",
"Viewer · E · Edit this image's tags"). `Inspector.svelte` exports `startEditTags` (Svelte 5
`export function`), which already guards `!image`. `LibraryScreen.screenKeys` — already
guarded by `lightboxOpen`, `isTypingTarget` and `isInDialog` — on `KEY_EDIT_TAGS` with a
focused image and `selection.count <= 1`: sets `browseSession.inspectorOpen = true`, awaits
`tick()`, calls `inspector?.startEditTags()` on a `bind:this`. Over a multi-selection the
panel shows the selection, so the key does nothing there. `Lightbox.onkeydown` on
`KEY_EDIT_TAGS` (after the typing guard, beside `KEY_INSPECT`): `mode = 'inspect'`, `tick()`,
`inspector?.startEditTags()`. The `i` key's guards are the shape to copy; `e` is a plain key
with no modifier, per the keyboard rule.

## Risks / Trade-offs

- [Two mounts of `UploadAction`] → both read one store; the `part` prop is the only branch.
- [`FilterRow` extraction touches both lists] → no handler moves; the diff is markup only,
  and both lists' existing hand checks cover it.
- [Pointer drag has no keyboard equivalent] → the CSS handle had none either; noted, not
  built.

## Migration

None. Session-only state only; no schema, no settings key.
