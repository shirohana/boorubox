## Context

`PinnedTagsPanel.svelte` sends every write through `write(change)`; its groups region is
`[data-groups]`, `overflow-y-auto`; `tagDrag` is `pointerDrag` on it with `onmove` resolving
the section under the pointer. `PinnedTagsDialog.svelte` mounts the panel in a bits-ui
`Dialog.Content`, whose focus scope autofocuses the first tabbable on open and, when the
focused element leaves the document or becomes disabled, moves focus to the first tabbable
again — the first group's name `Input`. `TagInput.svelte` and `TagNoteDialog.svelte` already
use `onOpenAutoFocus` to take that decision away from the kit.

## Decisions

**D1. The groups region is the panel's resting focus: `tabindex="-1"` on `[data-groups]`.**
`write()` reads `document.activeElement` before the change; after the change and a `tick()`,
if that element is still in the document and not disabled it is focused again, otherwise the
groups region is. The region, not the panel root, so Page Up/Down and the arrow keys scroll
the groups from there. On the Settings page the same rule holds and costs nothing.

**D2. The dialog opens on the groups region.** `Dialog.Content` gets `onOpenAutoFocus` that
prevents the default and focuses `[data-groups]` inside the content, the `TagNoteDialog`
shape; Escape still closes, since the kit listens on the document.

**D3. Edge auto-scroll lives in the panel's drag.** On every `onmove`, with the groups
region's rect: inside 40px of its top edge scroll up, inside 40px of its bottom edge scroll
down, by `2 + 14 * (1 - distance / 40)` px per animation frame, in a `requestAnimationFrame`
loop that runs while the last reported pointer is in an edge band and stops on `onend` or
when the pointer leaves the band. The section under the pointer is re-resolved after each
scroll step, so the ring follows what scrolls under the pointer. A frame loop, not a timer:
one step per paint is the smoothest scroll the engine can draw and it idles with the tab.

## Risks / Trade-offs

- [The pointer parked exactly on the edge scrolls forever] → it stops at the region's
  scroll limit; the drag can still end.
