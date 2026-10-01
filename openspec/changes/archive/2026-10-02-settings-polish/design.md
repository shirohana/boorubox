## Context

`routes/settings/library/+page.svelte` draws three action rows (Regenerate thumbnails, Clear
playback samples, Rebuild library index) as `flex flex-wrap items-center justify-between`,
description `min-w-0` first, `Button` second. `justify-between` puts the button right only
while both items share a line; once the description wraps the button onto a line of its own,
that line holds one item and `justify-between` places it at the start.

`CollectionsSection.svelte` draws its header as a `flex items-center justify-between` div
holding the `Collapsible.Trigger` (chevron + "Collections") and a `size-5` "+" button.
`NotesPanel.svelte`'s trigger is the whole row because nothing shares it.

## Decisions

**D1. The button gets `ml-auto`.** A flex item with an automatic left margin is pushed to the
end of whatever line it lands on, so the button is right-aligned on a shared line (where
`justify-between` already put it) and on a line of its own. Three rows, three class additions,
no restructuring; a shared snippet for the three rows was considered and rejected — each row's
button has its own disabled rule and label, and the three live in one file where the repeated
two-class wrapper is readable as is.

**D2. The Collections trigger grows to fill the row; the "+" stays outside it.** The trigger
gains `min-w-0 flex-1` (and keeps `justify-start`, which `flex` already gives it), so its hit
area runs from the chevron to the "+" button. The "+" stays a sibling, not a child: a button
inside a button is invalid markup, and creating a collection must not also fold the section.
The hover tint the trigger carries (`hover:bg-sidebar-accent`) now covers the whole row, which
is what the Notes header shows today, so the two headers read alike.

## Risks / Trade-offs

- [A right-aligned button under a wrapped description reads as detached] → it is how the
  General page's rows already fall, and the owner asked for exactly this ("more natural in my
  experience").
