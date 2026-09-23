## Why

The owner's review of the 2026-09-23 run (tag-vocabulary, stamps), 2026-09-23 evening: the
inspector's tag pills make the "in the search" marking hard to see; the account chip above the
tags doubles the artist now that `artist:` tags exist; Edit should put the caret where a tag
is about to be typed; the sidebar's count order shuffles as the search changes, and the owner
wants Danbooru's grouping; general tags should be blue as on Danbooru; a searched name no tag
has appears as a zero row with a menu that only errors; pinned chips read as tags.
Requirements §6 (Danbooru-style tagging, browse).

## What Changes

- **Inspector tags are plain coloured text**, no pill, at the sidebar's weight, and both
  panels mark "included" and "excluded" with one shared style that is visible on plain text.
- **The account entry moves into the facts list**, a row labelled Account above Page, and
  keeps its search-term behaviour. Blue is then free for general tags.
- **Edit puts the caret at the end** of the seeded text, after the trailing space.
- **The sidebar groups tags by category** — artist, copyright, character, general, meta —
  alphabetical inside a group, search or no search; a group is labelled. The same order
  becomes the app's one category order (the inspector's groups, the editor's lines).
- **General tags are blue**, Danbooru's colour, reversing tag-vocabulary D6 with the
  argument recorded there.
- **A searched name that no tag has is not listed.** A tag the search names that the result
  lacks is still listed at zero (the undo affordance stays); a name with no tag row behind it
  is not a tag and gets no row.
- **Pinned chips carry a pin mark**, so a chip is read as a control and never as one of the
  image's tags.

## Capabilities

### Modified Capabilities

- `tag-editing`: tags shown as plain text with the shared marking; the account entry's
  place; the editor's caret; the category order.
- `tag-sidebar`: the list's order and grouping; which zero rows exist.
- `tag-vocabulary`: general's colour; the pinned chip's mark; the category order.

## Non-goals

- Group headings that collapse, or counts per group.
- A management page for tags.
- Changing the search marking's colours (emerald / destructive) — only where they are drawn.

## Impact

- Rust: `query.rs` (`tag_counts` appends the request's named tags that exist at zero;
  `Plan` learns the request's tag names), `commands.rs` unchanged in shape.
- Webview: `domain/tag-categories.ts` (`CATEGORY_ORDER`), `components/tags/categories.ts`
  (general's colour, the shared search-marking classes), `TagSidebar.svelte`,
  `Inspector.svelte`, `TagInput.svelte` (`focusEnd`), `domain/tag-input.ts` tests (order),
  the archived `2026-09-23-tag-vocabulary/design.md` D6 (the reversal recorded).
