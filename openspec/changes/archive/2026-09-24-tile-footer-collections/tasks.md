> One Sonnet unit, webview only, one commit, after `pinned-collections` unit W has landed (both
> edit `ImageCard.svelte`; W mounts the pin menu item, this edits the footer). No design: the
> shapes are the footer's existing tag run and the corner mark's existing `collectionNames`.
> Gate: `mise run check` green. No unit ticks a hand check.

## 1. The footer's collection run (`packages/app`)

- [x] 1.1 `library/ImageCard.svelte`: inside the footer's `{:else}` branch, before the tag
      groups, a run drawn only when `collectionNames.length > 0`: `BookmarkIcon` at
      `size-3 inline` (the corner mark's glyph, smaller for the `text-xs/4` line) followed by
      each name in `text-foreground`, each followed by the same `{' '}` expression the tag run
      uses so the clamp can wrap. The `No tags` branch becomes "no tags and no collections":
      an image in a collection but with no tags shows the collection run alone, so the
      condition for the placeholder is `tagGroups.length === 0 && collectionNames.length === 0`.
      `markOverflow`'s `deps` gains `collectionNames` so the hover expansion re-measures when
      membership changes. A doc comment on the run states the order and its reason in one
      line (collections first: few per image, and the answer to the pinned chip's click).
      Verify: `mise run check` green; commit.
      Hand check: with the footer on, an image in Cute and Queue reads bookmark, `Cute`,
      `Queue`, then its tags; adding it to a third collection from the inspector's pinned chip
      updates the footer at once; an image in no collection shows its tags alone; an image in
      a collection with no tags shows the collection run and no "No tags"; the hover
      expansion still opens when the three lines overflow.
