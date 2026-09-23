## Context

- `components/tags/categories.ts` holds `CATEGORY_TEXT_CLASS` (general `''`) and re-exports
  `CATEGORY_ORDER` from `domain/tag-categories.ts` (`artist, copyright, character, meta,
  general`), which `editorText` (`domain/tag-input.ts`), the inspector's `groupedTags` and
  `TagVocabularyMenuItems` read.
- `TagSidebar.svelte` computes `listed`: the result's `TagCount[]` plus every included or
  excluded query term the result lacks, at zero; sorted active-first, then count, then name.
  A row is `+`, `−`, the name button (category colour, `bg-emerald-500/15 font-medium` when
  included, `bg-destructive/10 line-through` when excluded), the count, with
  `TagVocabularyMenuItems` on its context menu — which calls `set_category` / `set_pinned`
  and, for a name with no row, gets "tag not found" back.
- `Inspector.svelte`: the tag section is a heading row (count, pencil Edit), the pinned
  chips (`Badge secondary`, ring for all, dashed outline for some), the account button
  (`bg-sky-500/15` blue; emerald/destructive tints when in the search), then the badge list
  (`Badge secondary` per tag, the same tints, `CATEGORY_TEXT_CLASS` on the text). The facts
  `<dl>` above it has Title, Source, Page, Image rows; Page shows `ExternalLink`.
  `startEditTags` seeds `draft` and flips `editingTags`; the `TagInput` mounts on the flag.
- `TagInput.svelte` exports `blur()`; `write()` places the caret with `setSelectionRange`
  after a microtask; `field` is the `<textarea>` / `<input>`.
- `query.rs`: `Plan::for_request(req, …).tag_counts(conn)` is one SQL over the matched rows;
  the request's tag names are in `req.query` (`ParsedTagSearch`: `include_tags`,
  `exclude_tags`, `or_groups`).

## Goals / Non-Goals

**Goals:** one category order, one search-marking style and one colour table, each read by
both panels; the inspector's tag list reads as text; every zero row in the sidebar is a real
tag; the account entry is a fact of the page, shown with the other facts.

**Non-Goals:** changing what a click on a tag does; changing the pinned chips' tri-state
logic; group headings in the inspector (the colours and the order carry the grouping there).

## Decisions

### D1. One order: artist, copyright, character, general, meta

`CATEGORY_ORDER` in `domain/tag-categories.ts` becomes `['artist', 'copyright',
'character', 'general', 'meta']` — Danbooru's sidebar order, which the owner asked for in
the sidebar. It is the app's one order, so the inspector's groups and the editor's lines
follow (the `tag-editing` spec sentence naming the line order is amended). *Why one order
and not a sidebar-only one:* two orders for the same five words is a second source of truth
for a fact a reader compares across panels; the editor's line order was a lead's call in
`tag-vocabulary` with no argument recorded for meta before general. Lead's call, flagged
to the owner in the run report. The `tag-vocabulary` spec's "Every tag has one category"
requirement enumerates the five categories in prose; that enumeration follows this order too
(`artist, copyright, character, general, meta`), copied from `lowercase-tags`'s current text
with only the order changed, so the spec does not itself become a second order to compare
against the code.

### D2. Blue for general, recorded as a reversal of tag-vocabulary D6

`CATEGORY_TEXT_CLASS.general = 'text-blue-600 dark:text-blue-400'`. D6 kept general
uncoloured for two reasons: blue was the account chip's colour, and colouring the majority
hides the four minority colours. The first reason ends with D4 (the account entry leaves the
tag area and has no colour of its own); the second is a taste call the owner has made for
Danbooru's look. Both are written into the archived design's D6 as an amendment, so the
decision is not re-argued from the old text. The `tag-vocabulary` spec's "ordinary text
colour" for general becomes "the general colour (blue)".

### D3. One search-marking style, on plain text, read by both panels

`components/tags/categories.ts` gains `SEARCH_MARK_CLASS: Record<'included' | 'excluded' |
'none', string>`:

- `included`: `bg-emerald-500/15 underline decoration-emerald-500 decoration-2
  underline-offset-2 rounded-sm`
- `excluded`: `bg-destructive/10 line-through decoration-destructive decoration-2 rounded-sm`
- `none`: `''`

and `searchMark(name, terms): 'included' | 'excluded' | 'none'` over `activeTerms`' result.
The sidebar row's name button and the inspector's tag text both take
`{CATEGORY_TEXT_CLASS[…]} {SEARCH_MARK_CLASS[…]}`; the sidebar's row-level tint moves onto
the name (the `+`/`−` buttons and the count stay untinted). Font weight is the same in both
states and both panels (`text-xs`, normal): the tint and the underline are the marking, so
the weight never reflows the list.

The inspector's tag list becomes a `flex flex-wrap gap-x-2 gap-y-1` of `<button type="button"
class="text-xs …">` — no `Badge`. The context menu trigger and its items are unchanged.

**Amended (tag-panel-polish, 2026-09-23), from the owner's reading of the running app:**
`SEARCH_MARK_CLASS` drops the underline and the strike-through — `included:
'bg-emerald-500/15 font-medium'`, `excluded: 'bg-destructive/10'`, `none: ''`. An underline
makes English hard to read, and a tag already carrying its category's colour struck through
is one colour too many; background is the one marking left, so it has to sit on a real box —
`rounded-md px-1 py-0.5` plus `hover:bg-accent` on the inspector's tag buttons and the
account row's, `rounded-md px-1` plus `hover:bg-sidebar-accent` on the sidebar's row `div`
(unchanged from before this pass). The sidebar's tint moves back off the name and onto the
row `div` — the whole hoverable area, as it was before this design's first pass — because a
name-only background read as a smaller, harder target than the row it sits inside; the
category colour stays on the name alone. `included` keeps `font-medium`: without the
underline, weight is what tells the active row from its neighbours at a glance, so D3's
original "weight never reflows the list" no longer holds — one word is heavier, and the list
does not reflow because weight was never the thing rows are ordered by.

**Amended again (tag-panel-polish, 2026-09-23), from review:** a caller's own neutral hover
(`hover:bg-accent` on the inspector's tag buttons and its Account button) sat on the same
element as `SEARCH_MARK_CLASS`'s tint, at equal specificity — the neutral hover, later in the
class string, always won, so hovering an active or excluded tag lost its marking. `included`
and `excluded` now carry their own hover shade (`hover:bg-emerald-500/25`,
`hover:bg-destructive/20`), and `searchMarkClass(mark, neutralHover)` returns the mark's class
for `included`/`excluded` and `neutralHover` only for `none` — so a caller passes its idle-state
hover once, through the helper, rather than concatenating it beside the mark table. The account
rail and the collection rows read the same table through the same helper (`AccountRail.svelte`,
`CollectionsSection.svelte`), so an included account or collection takes the shared emerald
tint rather than a colour of its own, background only, no strike-through — the same rule D3
already states for a tag.

### D4. The account entry is a facts row above Page

In the facts `<dl>`, a row `<dt>Account</dt><dd>` before Page, present when `image.account`
is set, in both `editingFacts` states (it is derived from the page address, so the row reads
the same while the address is being edited). The `<dd>` holds one `<button>` with the
handle, `text-foreground` (no colour of its own now that blue is general's), the
`SEARCH_MARK_CLASS` for `terms.accounts` / `terms.excludedAccounts`, `hover:underline`, the
same `query(toggleAccountInQuery(...))` handler and `title="Search for this account"`. The
button under the tag heading is removed. The `tag-editing` requirement's "first entry under
the tag editor … blue" is amended to this row.

### D5. Edit puts the caret at the end

`TagInput` exports `focusEnd()`: `field?.focus()` then `setSelectionRange(value.length,
value.length)`. `startEditTags` sets `editingTags = true` and calls `tagInput?.focusEnd()`
inside `tick().then(…)`, since the field mounts on the flag. The seeded text ends with a
space (`editorText`), so the caret is on a fresh token.

### D6. Pinned chips carry a pin mark

Each chip's `Badge` gets a `PinIcon` (`@lucide/svelte/icons/pin`, `size-3`, `shrink-0`)
before the name. The chip stays a `Badge secondary` with the ring / dashed-outline fill
states; now that the image's own tags are plain text (D3), a chip is the only pill-shaped
thing in the section and carries a glyph as well, so it reads as a control.

### D7. The sidebar groups by category and labels the group

`TagSidebar.svelte`'s `listed` becomes `groups`: for each category of `CATEGORY_ORDER`, the
tags of that category (`vocabulary.categoryOf`) sorted by `localeCompare`, skipped when
empty. Active-first and count order are gone: the same rule holds with or without a search
(owner). Each non-empty group is preceded by a label row, `categoryLabel(category)` in
`text-[10px] font-medium uppercase tracking-wide text-muted-foreground px-1 pt-2 first:pt-0`,
so a reader knows where general ends and meta begins without relying on colour alone. The
count stays on each row.

**Amended (tag-panel-polish, 2026-09-23), from the owner's reading of the running app:**
the group labels are gone — the colour already carries the grouping, and a label row per
category was noise on top of it. The order they existed to explain stays, but changes shape:
the tags the current search includes or excludes come first, then the rest, each half
grouped by `CATEGORY_ORDER` and sorted alphabetically inside (`groupByCategory`, called
twice — once per half — rather than a second, hand-written comparator for "active first").
This is a reversal of this same design's "active-first and count order are gone" sentence
above: that sentence answered a *different* complaint (the list reshuffling on every search),
which colour-and-category order still solves without a labelled section header; it did not
anticipate the owner wanting the searched tags surfaced at the top once the labels that used
to mark them apart were gone. Both halves are still ordered by category and name, so within
each half the reader gets the same grouping the labelled version gave, just without the row
that spelled it out.

### D8. Zero rows come from Rust, and only for tags that exist

`query.rs`'s `tag_counts` (the `TagCounts.tags` half) appends, after the counted rows, every
name of `req.query.include_tags`, `exclude_tags` and the members of `or_groups` that is not
among the counted names *and* has a `tags` row, with `count: 0`. One extra `SELECT name FROM
tags WHERE name IN (…)` per request, over the request's own names — never over the result.
The sidebar's `missing` merge is deleted; `TagSidebar` groups what it is given. A name with
no row is not a tag: it is not listed, so no menu can ask Rust about it. The `tag-sidebar`
spec's first requirement gains the sentence; its "Empty result" scenario (`the tag list
holds only the searched tags at zero`) is read as "the searched tags that exist".

*Why Rust:* the webview cannot know whether a name has a row without asking, and the
counts request already crosses the wire with the parsed query in hand.

## Risks / Trade-offs

- [The editor's line order changes under the owner's hands] → flagged in the report; one
  edit of `CATEGORY_ORDER` reverts it if the sidebar and the editor are to differ.
- [Group labels cost a row each] → five at most; the tags section scrolls on its own.
- [A `tags` row that exists with no carrier (categorised or pinned, `tag-vocabulary` D3)
  shows at zero when searched] → correct: it is a tag, and its menu works.
- [The sidebar's `missing` computation had a test] → moved to `query.rs`'s tests.
