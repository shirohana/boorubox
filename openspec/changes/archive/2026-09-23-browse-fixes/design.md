## Context

See proposal.md — Why. Four independent defects, each in code whose design is recorded:

- `api/selection.svelte.ts` (`selection-and-bulk` D1–D4): a plain click focuses and clears
  (`click`, the fall-through branch); a multi-click resolves the live set and toggles one id
  (`#editIds`); `removeMany(ids)` is the one prune and its only caller is
  `LibraryScreen.afterTrashWrite`. Every other write ends with `results.refresh()` and leaves
  the selection alone: `BulkTagDialog.apply`, `LibraryScreen.writeRating`, the sidebar's
  collection `onchanged`. `results.refresh()` re-runs the query for real, so the rows leave the
  grid and the ids stay in the store. `SearchResults` is sparse (page 0 plus `total`), so the
  webview cannot answer "is this id still in the result" for a selection past the loaded pages.
  Rust already has `search_ids(req)` (a page of ids, one `Plan`) and `search_position(req, id)`
  (one id → row or null); `ID_CHUNK = 900` in `query.rs` is the chunk size for `IN (...)`.
- `ImageCard.svelte` focuses the tile on `mousedown` before the click that carries the modifier
  reaches the store (`tile-click.ts`'s note), and `LibraryGrid`'s `onfocusin` calls
  `selection.focusEntered(index)`, which moves `focus` and not `anchor`. So at the time a
  multi-click's `click()` runs, `focus` is already the clicked card and `anchor` is the card the
  user last clicked or arrowed to. The tile's checkbox sends `onselect({ multi: true })` through
  the same branch. `multi` is `metaKey || ctrlKey` on every platform.
- `Lightbox.svelte` opens with `showModal()`: the `<dialog>` is in the browser's top layer, its
  backdrop over everything else, and content outside it is inert. bits-ui portals menu, popover
  and dialog content to `<body>` by default. `TagInput.svelte` alone passes
  `portalProps={{ to: field.closest('dialog') ?? undefined }}`, with the reasoning in its
  comment; that is why the suggestion list works in the viewer and nothing else the inspector
  opens does. No `BitsConfig` is set anywhere.
- `LibraryGrid.svelte` handles its keys on the scroll container, by design (`app-shell` D14:
  keys fire where they act, no central dispatcher). `LibraryScreen`'s window handler binds only
  `/`, ⌘A and Escape. The inspector's root and the `<aside>` around it carry no `tabindex`, so a
  click on their text or empty space drops the focus on `<body>` (or on the panel's scroller in
  Chromium) and the grid's handler never sees the next key. Inside the viewer the handler is on
  the `<dialog>` itself, which a click on unfocusable content leaves as the active element, so
  the viewer is unaffected. `ImageCard.svelte`'s `onmenuclose` (commit `48b111d`) is the
  existing hand-back: when the focus is orphaned (`null`, `body`, or inside the closing menu) it
  focuses the card's `[data-card-focus]`; `LibraryGrid.refocus()` is the grid's own entry for
  "focus the current card without moving the anchor"; `Inspector`'s `onrelease` prop already
  routes to `grid.refocus()` beside the grid and `surface.focus()` inside the viewer.

The repo has no component-test harness; every rule lives in a pure module with a vitest file, and
component behaviour is a `Hand check:` line.

## Goals / Non-Goals

**Goals:** one after-write path that every search-re-reading write goes through, so the prune
cannot be forgotten by the next write; the anchor pickup inside the store, where the existing
tests are; one portal rule and one orphan-focus rule, each written once and called from every
place that needs it.

**Non-Goals:** pruning after the single-image paths (`tags-and-ratings` D10 keeps an edited image
on screen until the next search, and this design keeps its selection with it); a key dispatcher;
touching the viewer's own key handling or its Tab trap.

## Decisions

### D1. The selection is pruned by asking the search, after every refresh that a write caused

Rust gains `query::matching_ids(conn, req, ids) -> Vec<String>`: the same `Plan` as
`search_ids` and `search_position` (`Plan::for_request(req, RatingClause::Included)`), then
`SELECT id FROM {rows} WHERE id IN (...)`, ignoring `limit`/`offset`, chunked by `ID_CHUNK`,
answering the subset of `ids` the query matches in any order. Command `matching_ids(req, ids)` in
`commands.rs`, registered in `lib.rs`, wrapper `matchingIds` in `api/commands.ts` beside
`searchIds`.

`Selection` gains a second injected resolver, `MatchResolver = (ids: string[]) =>
Promise<string[]>`, and `keepMatching(): Promise<void>`: resolve the live set to ids (a range
first, as every action does), ask which still match, and write back the intersection through
`#editIds` so the existing "a gesture during the round trip owns the selection" guard applies.
An empty selection returns without a round trip.

`LibraryScreen` gains one `afterWrite()`: `await results.refresh()`, then
`await selection.keepMatching()`, then clamp the focus onto a row that exists
(`grid?.focusCard(Math.min(selection.focus, results.total - 1))` when the focus is past the end,
as `afterTrashWrite` does today). `BulkTagDialog` stops calling `results.refresh()` itself and
takes an `onapplied` callback the screen binds to `afterWrite`; `writeRating`'s bulk branch and
the sidebar's collection `onchanged` call `afterWrite` too. `afterTrashWrite` becomes
`afterWrite`: a trashed image no longer matches the library search and a restored one no longer
matches the trash search, so the prune by the search subsumes the prune by written ids, and
`removeMany` (with its test) is deleted rather than kept beside a second rule. `remove(id)`
stays: the inspector's strip drops one thumbnail by hand.

*Why ask Rust rather than intersect with `searchIds(0, total)` in the webview:* the whole
result's ids can be a hundred thousand UUIDs to prune a selection of fifty; `matching_ids`
transfers the selection's ids instead, bounded by the selection, and `ID_CHUNK`'s doc comment
already names this class of caller. *Why not prune by written ids everywhere, as trash did:* a
bulk tag edit's written ids are the whole selection, and which of them left the result depends
on the query, which only the search can read. *Why one `afterWrite` rather than a hook inside
`results.refresh()`:* `SearchResults` does not know the selection (it is injected the other way
round, D3 of `selection-and-bulk`), and a refresh caused by something other than a write (a
library switch, a rebuild) resets the selection through `reset()` already.

### D2. A multi-click over an empty selection picks up the anchored card; the checkbox does not

In `click()`'s `multi` branch, after the resolve and before the toggle: when the resolved set is
empty, `this.anchor >= 0` and `this.anchor !== index`, the anchored row's id is added first —
`await this.#resolve(this.anchor, 1)`, the same injected `IdResolver`, because the anchored row
may be on a page the grid has scrolled past (design D2 of `selection-and-bulk` assumed the
toggled tile is drawn; the picked-up one need not be). `focusAt(index)` at the end moves the
anchor to the clicked card, so the pickup happens once per selection, not on every later
multi-click. The read is `anchor`, never `focus`: by the time `click()` runs, `focusEntered` has
already moved `focus` to the clicked card (Context), and a rule on `focus` would select the same
card twice and pick up nothing.

The checkbox gets its own entry, `toggle(index, id)`: today's toggle body with no pickup. A
checkbox names the image it is drawn on and nothing else; ticking B's box after clicking A must
not select A. `ImageCard`'s `onCheckedChange` calls it through a new `ontoggle` prop threaded by
`LibraryGrid`.

*Compatibility with `selection-and-bulk` D1, recorded here because this change amends its
reading.* D1 rejected "a plain click selects one image" because the selection toolbar would
replace the action row the moment anyone clicked a thumbnail. That cost is paid at the plain
click, and the plain click is unchanged: browsing still selects nothing and shows no toolbar.
What D1's slogan "focus is not membership" did not anticipate is the gesture after it — the user
who clicks A, then reaches for the modifier to add B, has told the app twice which cards they
mean, and the shipped store kept only the second. Shift-click already includes the plain-clicked
end ("both ends included"); this makes the multi-click do the same. The reading was right when
written (nothing selected on a plain click is the whole point) and stops being complete once the
next deliberate gesture is considered.

*Alternative rejected:* selecting the anchored card on `Esc`-then-multi-click only when the
anchor is also the focus. After `Esc` the focus stays on the card the arrows move (design D4 of
`selection-and-bulk`), so the anchor is that card, and picking it up is the same rule; a second
condition would be a special case with no user-facing difference.

### D3. One portal rule: an overlay opens inside the nearest native dialog

A pure `portalTarget(el: Element | null | undefined): Element | undefined` in
`packages/app/src/lib/portal.ts`, returning `el?.closest('dialog') ?? undefined`, with
`TagInput.svelte`'s comment moved onto it (the top layer, not z-index, is what an overlay outside
a modal `<dialog>` cannot cross). `TagInput` calls it. `Inspector.svelte` binds its root element,
derives `portalTo = portalTarget(root)`, and passes `portalProps={{ to: portalTo }}` to the
add-menu's `DropdownMenu.Content` and both `ContextMenu.Content`s; `CollectionNameDialog` and
`UploadAction` (and through it `UploadDialog`) take a `portalTo` prop and pass it the same way,
so every overlay the inspector mounts lands in the viewer's dialog when the inspector is inside
it and in `<body>` beside the grid. jsdom test: inside a `<dialog>` the target is that dialog,
outside it is `undefined`.

*Why the dialog element and not the Lightbox's `surface`:* `closest('dialog')` is the rule that
already ships and is true wherever a native modal dialog appears later, with no prop threaded
from the viewer. The viewer's Tab trap walks `surface` and so does not see a portalled menu's
items, which is fine: bits-ui drives its own arrow and Tab handling inside an open menu and the
Lightbox bails on `defaultPrevented`. Escape inside an open menu closes the menu (bits-ui stops
the event); Escape on the dialog closes the viewer. Hand check.

### D4. One orphan-focus rule, applied at the screen, hands the keys back after any click

A pure `isOrphanedFocus(active: Element | null, within?: Element | null): boolean` in
`packages/app/src/lib/components/library/focus-handback.ts`: true when `active` is `null`, is
`document.body`, or is inside `within` (the element that is about to go away — the closing menu
today). `ImageCard`'s `onmenuclose` calls it instead of carrying the three-way test inline.

`LibraryScreen` binds a window `click` handler (the toolbar and the sidebar render through the
frame's snippets outside the screen's own subtree, so the root would miss them; bubbling, so it runs after every target's own handler
and after `mousedown` has moved the focus) that, when a card is current
(`selection.focus >= 0`) and `isOrphanedFocus(document.activeElement)`, calls `grid.refocus()`, and stands down for a click inside any dialog (`isInDialog`), where
the grid is not what the user is looking at.
`refocus()` focuses the current card with `preventScroll: true` so a click on the panel never
scrolls the grid — while the card is mounted. A wheel scroll never moves the focus, so the
current card can be outside the grid's window and unmounted (`grid-window`); there `refocus()`
falls back to `showCard`, which scrolls the row in and lets the grid's focus effect land on it.
(Amended at review: the first cut no-oped in exactly the state the rule exists for.) That covers the inspector's text and empty space, the account rail, the
toolbar band and the sidebar's gaps in one rule; the viewer needs nothing because a click inside
the modal dialog leaves the dialog itself active, which is not orphaned.

*Why at the screen and not on the inspector's root:* the inspector was where the owner met it,
but the rail, the toolbar and the sidebar have the identical defect for the identical reason,
and one handler on the region that owns the grid is smaller than a hand-back on each surface.
*Why this is not the dispatcher D14 rules out:* nothing here reads a key or decides which region
is live; the grid still handles its own keys on its own element. The rule only restores the
focus a click took to nowhere. `screenKeys` is unchanged.

*Amendment to `app-frame`'s hand-back paragraph, recorded in its delta:* "a completed action"
was the right scope when the panel's only focus-stealing clicks were its actions; the panel has
since grown plain text, addresses and empty space that take the focus off the grid just the
same, so the rule widens to any click that leaves no control focused.

## Risks / Trade-offs

- [A prune during a gesture] → `keepMatching` runs through `#editIds`, whose guard drops the
  write when the state changed during the round trip; the test for it copies the existing
  "resolves a live range before a multi-select-click" shape.
- [A large selection's ids travel to Rust on every bulk write] → bounded by the selection and
  chunked at 900; select-all over 25k images sends 25k ids once, which `selection-and-bulk` D3
  already accepted for resolving the range.
- [Ctrl-click on macOS is the context-menu gesture] → unchanged and out of scope; ⌘-click is the
  multi gesture there, Ctrl-click on Windows. Hand check on both.
- [An overlay portalled into the dialog may sit under the Lightbox's own controls] → the dialog's
  children stack in DOM order and bits-ui content is `position: fixed` with `z-50`; the viewer
  draws no chrome over the panel. Hand check inside the viewer.
- [A click on a control WebKit does not focus (a plain button)] → the active element is whatever
  it was before, usually the card, so nothing is orphaned and nothing moves; if it was already
  `body`, the grid gets its focus back, which is the wanted outcome.
