## Context

Every tag menu in the app is `TagVocabularyMenuItems.svelte` (Pin/Unpin, group moves,
Category), mounted by the sidebar row (`TagSidebar.svelte` → `FilterRow.svelte`'s `menu`
snippet), the inspector's tag list and the pinned chip (`Inspector.svelte`, snippets
`tagSearchItems`, `pinnedTagRows`, `pinnedChip`). The tile menu is in `ImageCard.svelte`
(Rating group, Collections submenu, trash actions). Collection rows (`CollectionsSection.svelte`),
the pinned collection chip (`CollectionPinMenuItem.svelte`) and stamps (`StampBar.svelte`) have
their own small menus. The menu primitives are shadcn copy-ins under `components/ui/context-menu`
and `components/ui/dropdown-menu`, excluded from lint and never edited; `app.css` already
overrides two sidebar slots by `data-slot` for the same reason. Rating colours are the two tables
in `components/tags/ratings.ts`; category icons are `CATEGORY_ICON` in `components/tags/categories.ts`.

## Decisions

**D1. The `#n` hint is drawn by the shared `pinnedTagRows` snippet, only for two or more
groups.** Each group's `<ul>` becomes `relative` with right padding for the label, and carries
one `<span aria-hidden="true">` reading `#{index + 1}` in `text-[10px] text-muted-foreground
tabular-nums`, absolutely positioned at the row's top right, aligned with the first line of
chips (the rows from the second on have a top border and top padding; the label sits inside
the padding, under the hairline). The `<ul>` gets `aria-label="Pinned group n"` so assistive
tech reads what the sighted reader sees. When `vocabulary.pinnedGroups.length === 1` nothing
is drawn and the padding is not reserved: one group has nothing to move to, so a number would
label nothing (owner, 2026-09-25). Both strips share the snippet, so both get it.

This amends `pinned-tag-groups` D5 ("no label, no number; the number exists only in the
menu"). D5 was right for three groups: the bands were self-evident and a number was noise. It
stopped being right at eight, when "Move to #6" meant counting rows from the top (owner,
2026-09-25). The hint is the number the menu already uses, not a name: the "bands, not names"
half of D5 stands.

**D2. One function chooses the Danbooru page, by category.** `packages/app/src/lib/domain/danbooru.ts`
exports `danbooruLookup(name, category): { label: string, url: string }`:
`artist` → label "Search artist on Danbooru", url
`https://danbooru.donmai.us/artists?commit=Search&search%5Bany_name_matches%5D=<encodeURIComponent(name)>&search%5Border%5D=created_at`
(the owner's URL, 2026-09-25: an artist's tag often is not the name the search knows);
every other category → label "Open Danbooru wiki", url
`https://danbooru.donmai.us/wiki_pages/<encodeURIComponent(name)>`. The host is a constant in
this file and nowhere else. Tests: both shapes, and a name with `/` and a non-ASCII name are
encoded. The item is the first in `TagVocabularyMenuItems` followed by a separator, so it is
the same item on the sidebar row, the inspector tag and the pinned chip, with `ExternalLinkIcon`.
It opens through `openExternal` from `$lib/api` (the `ExternalLink.svelte` path); its failure
string is not shown — `ExternalLink` shows one because it has a row to show it in, a menu that
has closed has none, and an `https:` URL built here is not one of the two failures
`ExternalLink` names (a `file:` address, a malformed one). A FIXME on the call names the right
shape: a shared transient-notice surface the frame does not have yet.

**D3. Menu density is one CSS block in `app.css`, for both menu kinds.** Selectors on
`[data-slot='context-menu-item']`, `-checkbox-item`, `-radio-item`, `-sub-trigger` and the
`dropdown-menu-` twins: `font-size: var(--text-xs)`, `line-height: calc(1 / 0.75)`, and the
padding written per side — `padding-block: 0.25rem` and `padding-left: 0.375rem` on all of
them, `padding-right: 0.375rem` on the item and sub-trigger slots, `padding-right: 2rem` on the
`-checkbox-item` and `-radio-item` slots. Not a `padding` shorthand: the block is unlayered and
so beats every Tailwind utility, and a shorthand's right value would strip the copy-ins'
own `pr-8` on the checkbox and radio slots — the space they reserve for the check mark drawn
`absolute right-2` — so a long checked label would run under it. An SVG child rule at `0.875rem`
(`size-3.5`) so every icon follows without a class on each.
`[data-slot='context-menu-group-heading']`, `[data-slot='context-menu-label']`,
`[data-slot='dropdown-menu-group-heading']` and `[data-slot='dropdown-menu-label']`: `text-xs`,
`font-medium`, `color: var(--muted-foreground)`, `padding: 0.25rem 0.375rem`. Named exactly,
not by suffix (`[data-slot$='-group-heading']`, `[data-slot$='-label']`): the suffix form would
also catch `select-group-heading` and `select-label`, slots this change does not own. Both
kinds, because the Library, Import, Upload and selection-toolbar dropdowns sit beside the
right-click menus on one screen and two densities would read as a mistake (owner did not
object, 2026-09-25). The copy-in files stay untouched: `pnpm dlx shadcn-svelte add` would
overwrite an edit, and the comment on the sidebar block already argues this placement.

**D4. Icons, the well-known set, and no others.** Category items → `CATEGORY_ICON[category]`
(the sidebar's five). Pin → lucide `pin`, Unpin → `pin-off`. "Move to trash" and "Delete
forever…" → `trash-2`; "Restore" → `undo-2`. "Edit…" (stamps) and "Rename…" (collections) →
`pencil`; "Delete…" (stamps, collections) → `trash-2`. The Danbooru item → `external-link`.
Every other item — Search for this tag, Exclude from the search, New group above/below, Move
to #n, Collections, Remove from this image, Pin (collections) — has no icon. Alignment: an item
without an icon is not indented to match one; the menu is a list, not a table (owner,
2026-09-25, on Apple's every-item icons).

**D5. The tile menu's rating is five menu items laid out as pills.** Inside the existing
`ContextMenu.Group` after its heading, a `<div class="flex gap-1 px-1.5 py-1">` holds one
`ContextMenu.Item` per rating plus `none`, each with `class` making it a pill: `justify-center
rounded-md border px-1.5 py-0.5 text-xs font-semibold uppercase` plus `RATING_COLOUR[r]` when
`image.rating === r` else `RATING_COLOUR_DIM[r]`; `none` uses the `RatingPills` neutral pair
(`bg-secondary text-secondary-foreground` when unrated, `border-border text-muted-foreground`
otherwise). Keyboard highlight is a ring, not the accent fill (`focus:ring-1 focus:ring-ring`,
and the accent background cancelled with `focus:bg-transparent!` where the fill is dim).
`aria-label` and `title` are `ratingLabel(r)`. They stay real menu items rather than a
`RatingControl` inside the menu: Enter picks and closes, `onSelect` fires the same `onrate`,
and bits-ui's arrow keys still step them in DOM order. Fallback if the row fights the menu's
key handling on screen: `RatingControl` in a plain row with `bind:open` closing the menu after
`onchoose` — decided by the hand check, not in advance.

**D6. "Move to trash" is `variant="destructive"` with no confirmation.** `trash` design D13
argued the reversible act plain and only the irreversible one marked. The owner overrides
(2026-09-25): the colour raises the action's identity in a list that is otherwise uniform, and
the distinction that matters — ellipsis plus confirmation on "Delete forever…" — is kept. The
trash spec's "one image moves without confirmation" is untouched.

**D7. "Remove from this image" is the last item of the inspector's tag menu**, after
`TagVocabularyMenuItems` and a separator: the destructive item last is the convention every
other menu here follows (stamps, collections, the tile).

## Risks / Trade-offs

- [The pill row's arrow keys step Up/Down through a horizontal row] → accepted; the mouse is
  the way a right-click menu is used, and D5 names the fallback.
- [A denser menu on Windows at 100% scale] → the sizes are the sidebar's own `text-xs`, already
  read there daily; hand check on Windows.
- [The `#n` label reserves right padding that a long row would otherwise use] → 1.5rem on a
  wrapping row of chips; accepted.
