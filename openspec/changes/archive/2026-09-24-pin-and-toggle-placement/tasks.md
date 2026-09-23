> One Opus unit, webview only, one commit, on main. No design: the shapes are the chip rows
> and the heading row that exist, moved. Gate: `mise run check` green. No unit ticks a hand
> check.

## 1. The two placements (`packages/app`)

- [x] 1.1 `library/Inspector.svelte`: the pinned strip in the tag area renders `vocabulary.pinned`
      only, drawn while any tag is pinned; the selection panel's strip heading reads Tags again.
      The single-image Collections section gains, under its heading and above the badge list, a
      `<ul>` of `pinnedChip` for `collections.pinned` (same fill state and activation as before);
      the selection panel gains a Collections section after its tag area, heading "Collections",
      holding the same chips with the tri-state fill and the confirm, drawn while any collection is
      pinned. The `pinnedChip` snippet, `collectionCountOf`, the fetch ticket and the writes are
      untouched. Comments that describe the strip as holding both kinds are corrected. Verify:
      `mise run check` green.
- [x] 1.2 `tags/TagSidebar.svelte`: the `<h2>Tags</h2>` and the toggles' `role="group"` share one
      `flex items-center justify-between` row, the group at the right; the group keeps its
      `aria-label`, buttons and classes. Verify: `mise run check` green; commit.
      Hand check: the toggles sit on the Tags heading's line, flush right; the single-image
      inspector shows a pinned collection's chip above its badges, and none in the tag strip;
      selecting several images shows a Collections section with the tri-state chips under the
      Tags section; a click on a chip still adds or removes membership.
