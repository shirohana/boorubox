## Context

`PinnedDot.svelte` takes `pinned: boolean` and draws a 4px disc plus hidden text.
`FilterRow.svelte` draws name button, `PinnedDot`, `TagNoteIndicator`, count, in a
`flex items-center gap-1` row. `Inspector.svelte`'s tag list (~line 1099) draws each tag as
an `inline-flex items-center gap-1` button holding the name span, `PinnedDot`,
`TagNoteIndicator`, inside `<li>`s of a `flex flex-wrap gap-x-2` list. `TagNoteIndicator`
renders a bits-ui `Tooltip.Trigger` whose child is an `inline-flex shrink-0` span around a
`size-3` icon. `vocabulary.labelOf(position)` and `groupOf(name)` answer a group's label.

## Decisions

**D1. `PinnedDot` takes `group: string | null`, the label of the group, and draws nothing for
`null`.** Callers pass `vocabulary.pinnedLabelOf(name)`, a new store reader that answers
`labelOf(groupOf(name))` or `null`, so the three call sites cannot compose the label three
ways. The hidden text becomes "Pinned in <label>".

**D2. The hint is a tooltip, `TagNoteIndicator`'s own shape.** `Tooltip.Root` with
`delayDuration={150}`, trigger `tabindex={-1}`, a span child, content "Pinned in <label>";
`portalTo` threaded as the note indicator threads it, since the inspector inside the viewer
needs it. The dot's `aria-hidden` stays and the hidden text carries the same words.

**D3. Order.** `Inspector.svelte`'s tag button: name span, `TagNoteIndicator`, `PinnedDot`.
`FilterRow.svelte`: name button, `PinnedDot`, `TagNoteIndicator`, count — the gap between
dot and glyph is the row's own `gap-1` plus `ml-0.5` on the glyph, so the dot reads as part of
the name and the glyph as part of the trailing cluster (owner, 2026-10-02: "tag label,
marker, space, note, count").

**D4. The line.** The inspector's `<ul>` gains `items-center`, and the tooltip trigger's span
in `TagNoteIndicator` gains `leading-none`, so a trigger that is taller than the text no
longer stretches its `<li>` against the others. Unit A cannot see the app; the lead's smoke
compares a noted and an unnoted tag on one line, and the fallback is `align-middle` on the
span.

## Risks / Trade-offs

- [The tooltip on every pinned tag costs a bits-ui root per dot] → the note glyph already
  pays it per noted tag; a sidebar list of a few hundred rows draws a few dozen dots.
