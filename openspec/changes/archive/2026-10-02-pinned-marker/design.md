## Context

`FilterRow.svelte` draws a sidebar tag row: controls cluster, name button, `TagNoteIndicator`,
count. `TagSidebar.svelte` mounts it with `note={vocabulary.noteOf(name)}`. `Inspector.svelte`
draws the described image's tags (~line 1099) as buttons: name span, `TagNoteIndicator`. The
viewer (`Lightbox.svelte`) mounts the same `Inspector`, so the viewer's panel comes with it.
`vocabulary.isPinned(name)` already answers from the store. The search marking is a background
tint on the row plus `font-medium` for an included tag (`SEARCH_MARK_CLASS`); the note glyph is
a `size-3` icon.

## Decisions

**D1. One component, `PinnedDot.svelte` in `components/tags`, drawn where `TagNoteIndicator`
is.** It takes `pinned: boolean` and renders nothing when false; when true, a
`<span aria-hidden="true" class="size-1 shrink-0 rounded-full bg-muted-foreground/70">`
and a visually hidden "pinned" text for assistive tech. 4px wide plus the row's `gap-1`: the
narrowest mark that still reads as deliberate (owner's call, 2026-10-01: a dot over the pin
glyph, which costs 12px plus the gap). Muted at 70% so it sits below the count's text in
contrast, and no tint on the row — the tint is the search's mark and the owner's one rule is
not to confuse the two. A component rather than a class on each site, so the two sites cannot
drift in size or colour.

**D2. `FilterRow` takes `pinned = false`; `TagSidebar` passes `vocabulary.isPinned(name)`;
the inspector's tag buttons render the dot after the name span.** The dot precedes the note
indicator in both places, so a pinned, noted tag reads name · dot · glyph. The collections
rows mount `FilterRow` without `pinned` and render nothing. `CollectionsSection`'s pinned
collections have their own chip and are out of scope.

**D3. Not drawn on the pinned chips themselves, the tile footers or `TagInput`'s suggestions.**
A chip's pin icon already says it; a footer is read for what the image carries, and the
suggestion row is read for a name.

## Risks / Trade-offs

- [Too anonymous on screen] → the fallback is the pin glyph at the note indicator's size,
  one class and one import in `PinnedDot`; the hand check decides.
