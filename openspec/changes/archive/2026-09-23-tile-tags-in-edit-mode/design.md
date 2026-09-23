## Context

- `ImageCard.svelte`: the `ContextMenu.Trigger` is `group/tile relative aspect-square`; the
  image button inside is `size-full`; the hover caption strip and the stamp overlay are
  absolute over the image. The card already holds `image.tags` (an `ImageRecord`).
- `grid-window.ts`: `GAP = 12`; `rowHeight` is "a tile is square, so this is the width a
  `1fr` column gets" plus the gap; `sectionsOf` and `imageTop` take `rowHeight`; the grid
  computes `shown` from `gridWindow({ …, tile })` and places rows by it.
- `LibraryGrid.svelte` receives `stampLabel` from the screen while `editMode && activeStamp`;
  the screen owns `editMode`.
- `groupByCategory` (`domain/tag-categories.ts`) and `CATEGORY_TEXT_CLASS` are the one
  order and colour.

## Goals / Non-Goals

**Goals:** the tile shows its tags whenever a click could change them; the row math stays
one computation; nothing is fetched that the grid does not already hold.

**Non-Goals:** a per-tile tag editor.

## Decisions

### D1. The footer is a fixed strip under the square, only while edit mode is on

`ImageCard` gets `showTags = false`. When true, the trigger becomes a column: the square
image block (unchanged, still `aspect-square` on its own wrapper) and under it a strip of
fixed height `TAG_FOOTER = 40` px (`grid-window.ts`, exported beside `GAP`: two lines of
`text-xs` at `leading-4` plus `py-1`), `overflow-hidden`, holding the tags as a
`line-clamp-2` run of inline spans separated by spaces, each in `CATEGORY_TEXT_CLASS`, in
`groupByCategory` order — artist first, so the most telling tags survive the clamp. No
tags: the strip reads "No tags" in `text-muted-foreground`. The strip is not part of the
image button (a click on it does nothing) and sits under the caption and the stamp
overlay, which stay over the image only.

*Why a fixed height:* the grid windows rows by one `rowHeight`; a footer that grows with
its text would need a per-row measurement the grid does not do.

*Amended (owner, 2026-09-24):* two lines cut a heavily tagged image short, and the owner
wants the whole list. A row height that follows its tallest footer would need per-row
measurement in the windowed grid, with the scrollbar shifting as rows measure, so the strip
stays fixed and grows in two cheaper ways: `TAG_FOOTER` is three lines (56 px), and while the
tile is hovered a strip whose clamped text overflows (`markOverflow`, `scrollHeight` against
`clientHeight`, measured only while the tile is not hovered so the expansion cannot feed its
own measurement) drops the clamp and grows over the row below on its own background, lifted
by `z-10` above the later rows, which are positioned but stack at auto. A short list gets no
expansion and no shadow. The row math never sees the growth.

### D2. The row height carries the footer

`gridWindow` takes `footer: number` (0 or `TAG_FOOTER`) and `rowHeight = tile + footer +
GAP`; `sectionsOf`, `imageTop` and the scroll-to-row math read the same `rowHeight`, so
nothing else changes. `LibraryGrid` gets `showTags?: boolean`, passes `footer: showTags ?
TAG_FOOTER : 0` and hands `showTags` to every `ImageCard`. `LibraryScreen` passes
`showTags={editMode}` — the mode, not the active stamp: the tags matter before a stamp is
chosen too.

*Why not `tile` grown by the footer:* `tile` is the column width the user set; the footer
adds height only.

### D3. The footer outside edit mode is a view setting, not a mode-only feature

Reversing this change's original non-goal ("a footer in browse mode; a view toggle for
one"): the owner ran the shipped edit-mode footer and found it useful for ordinary browsing
too (2026-09-24) — the non-goal was right while the footer only existed to say what a stamp
click would change, and stopped being right the moment it turned out to answer "what tags
does this have" on its own. `showTileTags: boolean` (default `false`) joins `Settings` and
`AppSettings` beside `grid_tile_size`/`gridTileSize`: it is a display preference of this
machine, the same kind as the tile size, not a fact about the library, so it belongs in
`settings.json` rather than the library's own file. `LibraryScreen` computes
`showTags = editMode || (settings.current?.showTileTags ?? false)` — a `$derived`, per the
rule against an `$effect` reading a field off a store object reassigned wholesale — and a
`Button size="icon-sm"` beside the thumbnail-size `Slider` toggles the setting. The button
sits with the grid's other display controls, not with the stamp bar's edit-mode controls,
because the setting itself is a display preference and outlives the mode. Edit mode disables
the button rather than hiding it: hiding it would make "on because of the mode" and "on
because of the setting" look the same, and disabling it while it still reads pressed keeps
that distinction visible.

## Risks / Trade-offs

- [Entering the mode reflows the grid] → the rows grow in place; the scroll position is
  kept as a plain refresh keeps it (`scrolledForQuery` untouched).
- [Dozens of tags per image] → three lines, and the whole list over the row below while the
  tile is hovered; the inspector is the full list otherwise.
- [Hover caption over the image already shows the title] → unchanged; the footer is under
  the image, not over it.
