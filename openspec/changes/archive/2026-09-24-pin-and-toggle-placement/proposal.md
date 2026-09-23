## Why

The owner checked the run of 2026-09-24 in the app and moved two things (2026-09-24): the
pinned collections' chips belong in the inspector's Collections section, not in the pinned
strip beside the pinned tags, and the sidebar's five category toggles belong on the Tags
heading's row, right-aligned, not on a row of their own. Requirements §6 (sidebars, collections).

## What Changes

- **Pinned collection chips move to the Collections section**, in both inspector placements:
  a row of chips at the top of the section, above the membership badges (single image) — the
  selection panel, which had no Collections section, gains one holding the chips after its tag
  area. The pinned strip in the tag area holds tags again and its selection heading reads Tags.
  Chip look, fill states, writes and the confirm are unchanged.
- **The category toggles sit on the Tags heading's row**, at the right, the heading at the left.

### Two placements reversed

`pinned-collections` (archive `2026-09-24-pinned-collections`, design D7) put the collection
chips in the pinned strip after the tags because that strip was the one place a one-click
write already lived, and a second strip was a second surface. On screen a collection chip among
tag chips read as one more tag, and the Collections section — which lists the image's
collections and offers "Add to…" — is where a click that changes them is looked for.
`tag-category-visibility` (design D5) gave the toggles their own row under the heading so they
read as a legend; on the heading's row they read the same, cost no height, and sit where a
section's controls are.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `collections`: the pinned-collection requirement's placement and its scenarios; the rebuild
  scenario's wording.
- `tag-vocabulary`: the pinned strip holds tags only again.
- `tag-sidebar`: the toggles' placement and the legend scenario.

## Non-goals

- Any change to what a chip or a toggle does, to their storage, or to the menus that pin.
- Deduplicating a pinned collection's chip against its membership badge: the chip writes and
  the badge searches; both stay.

## Impact

- Webview only: `library/Inspector.svelte` (the chip rows, the selection panel's new section,
  the heading text), `tags/TagSidebar.svelte` (the heading row). No Rust, no shared type.
