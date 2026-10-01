## Context

`Inspector.svelte`'s `pinnedTagRows` draws each group as `<div class="flex flex-wrap gap-1">`
holding a `<ul class="contents">` of `<li>` chips; a chip is a `button` around a `Badge`
(`flex items-center gap-1`) holding the pin icon, the name and `TagNoteIndicator`, which is
now `HoverHint` (an `inline-flex shrink-0 leading-none` span around a `size-3` icon).
`PinnedTagsDialog.svelte` is `Dialog.Content class="max-h-[85vh] overflow-y-auto"` holding
header, `PinnedTagsPanel`, and a footer with Done. `PinnedTagsPanel.svelte`'s root is a
`flex flex-col gap-3`: the groups (with `use:tagDrag`), then the naming form or the bar with
the move `Select` and "New group…". The Settings page mounts the panel inside its section.

## Decisions

**D1. Each chip's `li` is a flex box.** The chip is an inline `button` inside a block `li`,
so it sits on the line box's baseline, and a chip whose glyph wrapper (an `inline-flex`
span) moves that baseline is lifted off its row — centring the row's items did nothing,
since every `li` was already the same height (lead, on screen 2026-10-02). With `class="flex"`
on the `li` the button is a flex item and no baseline rule applies; a noted and an unnoted
chip then share the line. The glyph's `leading-none` stays.

**D2. The panel owns its scroll region and its bar; the host decides what else sits in the
bar.** The panel's root becomes `flex min-h-0 flex-col`: the groups region is `min-h-0
flex-1 overflow-y-auto pr-1` (the right padding keeps the scrollbar off the group boxes), and
the bar follows it, outside the scroll region; on the Settings page, which scrolls as a
whole, the host asks for `stickyBar` and the bar is `sticky bottom-0 bg-background`. Not
sticky in the dialog: the bar is outside the scroll region there already, and WebKit
misplaces a sticky box inside the dialog's centring transform (lead, on screen 2026-10-02). The panel takes an optional `barEnd: Snippet`, rendered at the
bar's end after "New group…"; the dialog passes Done through it and drops its own
`Dialog.Footer`. The naming form takes the bar's place while it is open, as today.

**D3. The dialog bounds the panel instead of scrolling itself.** `Dialog.Content` is the kit's grid, so it
keeps `grid` and gets `max-h-[85vh] grid-rows-[auto_minmax(0,1fr)]` with no
`overflow-y-auto` — a `flex flex-col` on it fights the kit's `grid` for `display` (lead, on
screen 2026-10-02); the header keeps its place; the panel's row is the `minmax(0,1fr)` one. The scrollbar is then the groups region's, inside
the content box, under the header and above the bar.

**D4. The sidebar dot follows the name's text, not the row's end.** `FilterRow`'s name
`button` is `flex-1`, so a dot placed after it lands at the row's right beside the glyph and
the count — the owner saw "name, gap, dot, note, count" (2026-10-02). The dot moves inside
the name button, right after a `truncate` span holding the text, the button itself `flex
items-center gap-1`; the glyph and the count stay at the row's end. Then the row reads name,
dot, the stretched gap, glyph, count, which is the order asked for.

## Risks / Trade-offs

- [A sticky bar over the page's last group on Settings] → the bar carries the page
  background and a top padding, so the last group scrolls under it and is reachable.
