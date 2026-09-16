## Context

See proposal.md — Why. The state of the code this change edits, as read on 2026-09-16 at
`575a940`:

- `LibraryScreen.svelte` owns the route's `toolbar` and `sidebar` snippets (rendered by the
  frame's `TopBar` and `Sidebar` through `frame.svelte.ts`), and holds `tagQuery`, `text`,
  `inspectorOpen`, `lightboxMode` and `new SearchResults(view)` as component state — every
  route visit is a fresh mount, so all of it starts over. `sort-and-group` already promises the
  sort and grouping survive a change of screen; the per-mount `SearchResults` breaks that
  promise today, which is the owner's "inputs and filters are gone".
- The toolbar snippet renders the slider, the inspector toggle, then the selection toolbar /
  Empty trash / Import, with no spacer; on Windows the frame's full-screen button carries the
  only `ms-auto`, on macOS nothing pushes right.
- `SearchBar.svelte` holds both fields, a 250 ms typing pause, Enter, Escape-blurs and a Clear
  button rendered only while there is a query. The tag field is `TagInput` with `multiline`.
- `CollectionsSection.svelte` sits between `RatingPills` and `TagSidebar`, sorts active rows
  first, and gives each row a `DropdownMenu` behind an ellipsis button. `NotesPanel.svelte` is
  the folding pattern: a `Collapsible` whose open state is `settings.current.notesCollapsed`
  written through `settings.setNotesCollapsed` → `set_notes_collapsed` in `commands.rs` →
  `settings.rs` (a `NOTES_COLLAPSED` store key, a field, a default, load and save, tests).
- `ImageCard.svelte` wraps the tile in `ContextMenu.Root`; its content opens with a bare
  `ContextMenu.GroupHeading`. bits-ui 2.19's group heading reads a `Menu.Group` context and
  throws without one — the dev log shows `Error: Context "Menu.Group | Menu.RadioGroup" not
  found … in context-menu-group-heading.svelte … in ImageCard.svelte` the moment the menu is
  asked to open. Nothing on that menu is reachable.
- Menu content copy-ins (`dropdown-menu-content`, `context-menu-sub-content`) set no max
  height. bits-ui exposes `--bits-floating-available-height` on floating content.
- `Lightbox.svelte`: a `header` bar over the stage, `hidden` unless `chrome` is true, toggled by
  a single click through `click-intent.ts` (a 250 ms timer distinguishing single from double);
  a double click calls `zoomTarget` (natural size, or 2× the fit); the wheel calls
  `zoomStep(scale, ±1, fit)`, one ×1.25 step per event regardless of `deltaY`; `panOffset`
  maps the pointer's fraction across the whole viewport to the overflow. `viewer-zoom.ts` is
  pure with tests.
- `SearchResult.groups` is `GroupSlice[]` (`{ key, count }`), the account handles largest
  first then by name (`ACCOUNTS_LARGEST_FIRST` in `query.rs`), complete for the whole result.
  `tag-utils` has `toggleAccountInQuery` and `activeTerms(...).accounts` /
  `.excludedAccounts`, but no add/exclude pair for accounts as `addTagToQuery` /
  `excludeTagFromQuery` are for tags.
- `ImageRecord.collections` is the image's collection ids; `collections.byId(id)` names one.

## Goals / Non-Goals

**Goals:** every one of the owner's fourteen items plus the two state fixes, with the pure
parts (zoom arithmetic, query helpers, the session store) tested without a DOM and the
components only wiring them; no second copy of the typing pause, the fold pattern or the
account-term logic.

**Non-Goals:** persisting the query, the sort or the grouping across launches; touch; a viewer
redesign; a count on the tile mark; any Rust beyond one settings field.

## Decisions

### D1. A session store outside the component holds what a route visit must not reset

`packages/app/src/lib/api/browse-session.svelte.ts` exports `browseSession`: one
`SearchResults` per view, created on first ask (`resultsFor(view)`), plus `inspectorOpen`
(`$state`, initially true) and `lightboxMode`. `LibraryScreen` reads all three from it instead
of owning them. On mount the screen initialises `tagQuery` / `text` from `results.inputs` and
calls `results.run(results.inputs)` — a load from scratch with the kept query, never
`refresh`: the other screen may have restored, imported or switched libraries, and a fresh run
is what the spec's "re-run against the library as it now is" means. `Selection` stays
per-mount and starts empty, as the spec says.

*Alternative rejected:* keeping the whole `LibraryScreen` mounted behind other routes
(`{#key}`-free layout). It would keep the scroll position too, but every effect in the screen
(capture events, drop zone, library switch) would then run on screens that must not react,
and the trash and the library would need two kept instances anyway.

The module-level instance reverses `trash` design D1's "one `SearchResults` per route mount";
its argument — the library and the trash never share one — still holds, they are two
instances; only their lifetime changes, and the comment on the class's `view` field is amended
to say so.

### D2. The toolbar is three groups with two spacers; the frame's full-screen button follows

The `toolbar` snippet renders, in order: the text field (`Input`, `w-56 shrink`, `h-7
text-xs`, the same `aria-label`, `placeholder="Title or URL"`); `<div class="flex-1">`; the
`SelectionToolbar` while `selection.count > 0`; `<div class="flex-1">`; then the screen's action
(`ImportMenu` on the library with no selection, Empty trash… on a non-empty trash with no
selection — the same `{#if}` chain as today, only moved), the `Slider` and the inspector toggle.
Two `flex-1` spacers centre the middle group and hold the end groups at the edges whether or not
the middle exists; at a narrow window the spacers give way first and the selection toolbar's
own `overflow-x-auto` still governs, so the 1000 px scenario holds. The Windows full-screen
button in `TopBar` keeps `ms-auto`, which with no free space left simply follows the toggle.

### D3. One typing pause, in the screen; the search bar becomes the tag field alone

The 250 ms pause moves from `SearchBar` to `LibraryScreen` as `searchAfterPause()` /
`searchNow()`, because two fields in two regions would otherwise each own a timer.
`SearchBar` keeps only the tag field — `TagInput` without `multiline`, `id="tag-query"`,
`class="h-7 text-xs"`, `placeholder="cat -dog · cat or dog · rating:s,q"` (the syntax examples
back in the field; the `title` that carried them goes) — and takes `oninput` / `onsubmit`
callbacks. The Escape-blurs-the-field handler moves to `$lib/keyboard.ts` as
`blurOnEscape(event)` so both fields share it. The Clear button is removed: the field is one
line, every pill and row toggles itself off, and select-all-and-delete clears it (the owner's
call, 2026-09-16: "let it goes"). This reverses `sidebar-layout` D2's two stacked fields and
Clear: that decision put the text field in the sidebar because the toolbar had lost its room to
the selection; D2 above gives the selection the middle of the bar instead, and the text field
fits at its left.

### D4. The collections section folds like the note and sizes like a resizable box

`CollectionsSection` becomes a `Collapsible` in the `NotesPanel` shape, open when
`!settings.current.collectionsCollapsed`, written through `settings.setCollectionsCollapsed`.
The list sits in a `div` with `resize-y overflow-y-auto` and a default height (`h-40`), a floor
(`min-h-10`) and a ceiling (`max-h-[50vh]`): CSS `resize` on an element whose overflow is not
`visible` draws the corner handle in both WebKit and WebView2, and the height it sets is the
element's own, session-only — exactly the note's textarea. The section is rendered after
`TagSidebar`, which stays the one `flex-1` region, so growing the box shrinks the tag list and
nothing else. Rows are drawn in the order Rust returns (name order); the active-first sort is
deleted with its comment. Each row is a `ContextMenu.Trigger` (the `li`), its content Rename…
and Delete…; the ellipsis button and its `DropdownMenu` go. The heading row keeps the "+"
button and gains the fold chevron.

The stored preference is the fourth field of its kind: `collectionsCollapsed` in
`AppSettings` (shared), `collections_collapsed` in `settings.rs` with a `COLLECTIONS_COLLAPSED`
key, default `false`, load and save, `set_collections_collapsed` in `commands.rs` registered in
`lib.rs`, `setCollectionsCollapsed` in `api/commands.ts` and on the `settings` store — each
copied from the notes field's line, with the same tests.

### D5. Every collection menu is capped at the floating layer's available height

The three places the shared `CollectionMenuItems` are mounted pass
`class="max-h-(--bits-floating-available-height) overflow-y-auto"` to their content:
`Inspector`'s `DropdownMenu.Content`, `SelectionToolbar`'s `DropdownMenu.Content`, and
`ImageCard`'s `ContextMenu.SubContent`. The variable is bits-ui's own, set on every floating
content from the collision boundary; the copy-ins are not edited (a `shadcn-svelte add`
would overwrite them). The dropdown copy-in already scrolls; the sub-content one does not,
hence the explicit overflow.

### D6. The tile menu: a group around the ratings; a mark for memberships

The rating heading and its six items are wrapped in `ContextMenu.Group`, which is what the
heading's context lookup needs; nothing else on the menu changes. The mark is a `BookmarkIcon`
badge in the top-left badge group beside the rating and posted marks, rendered only when
`image.collections.length > 0`, with `title` the joined names from `collections.byId`
(an id with no name — a placeholder collection of a rebuilt library — shows as the id).

### D7. The viewer: no chrome, a click toggles cover and fit, animated

`Lightbox.svelte` loses the `header`, the `chrome` state, `onimageclick`'s timer and the
`click-intent.ts` module (deleted with its test). The image's `onclick` is `toggleZoom`
directly. `viewer-zoom.ts` replaces `zoomTarget` with `coverScale(natural, viewport) =
max(viewport.width / natural.width, viewport.height / natural.height)` and
`clickTarget(natural, viewport) = cover > fit ? cover : 2 * fit`, so the click always
visibly zooms (the twice-the-fit fallback fires only when the aspect ratios match exactly).
`Tab` keeps `trapTab` over the inspector's controls; the branch that revealed the chrome first
goes. Everything else — keys, Space, the close targets, `move` resetting the zoom — stays.

The animation: a `zooming` flag set by `toggleZoom` puts `transition-[width,height,transform]
duration-200 ease-out` on the `<img>` and is cleared on `transitionend` (and by a 300 ms
timeout, for an image that did not change size). Only the click sets it: the wheel and the
pointer pan write every frame and must not be eased, or the pan lags the pointer.

**Amended 2026-09-16, after the owner's hand check.** The CSS transition above was right on
paper and wrong on screen twice over. First, WebKit runs a `transform` transition on the
compositor and a width/height transition on the main thread, a frame or two apart, so the top
edge of an image the maths keeps still visibly dipped. Moving the pan to `left`/`top` put the
four properties on one thread and cured that — but the easing it needed on the pan (so a pan
retargeted mid-zoom is not cut) made the pointer feel followed late, even at 80 ms. The
shipped shape is a `requestAnimationFrame` loop in the component writing `scale` each frame
through `zoomAt(from, to, elapsed)` (pure, `viewer-zoom.ts`, ease-out cubic over
`ZOOM_EASE_MS`); the pan is the `transform` it always was, written every frame from the
pointer's current position with no easing at all. The wheel, a pinch and a move cancel the
loop and take over from wherever it has got to.

### D8. The pan maps the middle third; the wheel and the pinch scale by their delta

`panOffset` takes a `MINIMAP_FRACTION = 1 / 3`: on each axis the pointer's position inside the
centred box `viewport × 1/3` is the fraction (clamped to `[0, 1]`); outside the box it rests at
the nearer edge. The rest of the mapping is unchanged.

`zoomStep(scale, direction, fit)` becomes `zoomBy(scale, factor, fit)`, the same clamp to
`[fit, ZOOM_MAX × fit]` applied to a multiplicative factor, and `wheelZoomFactor(event)`
turns a wheel event into one: `deltaY` normalised to pixels (`deltaMode` 1 lines × 16, 2
pages × 800), then `exp(-pixels × WHEEL_SENSITIVITY)` with `WHEEL_SENSITIVITY = 0.002`, so a
mouse notch (~100 px) is ×1.22 — the one visible step it is today — and a trackpad's few-pixel
events are a few percent each. A wheel event with `ctrlKey` is Chromium's pinch (WebView2 on
Windows): the same function with `PINCH_SENSITIVITY = 0.01`, since those deltas are small.
WKWebView (macOS) reports a pinch as `gesturestart` / `gesturechange` / `gestureend` with
`event.scale` relative to the gesture's start: the viewport records the scale at `gesturestart`,
applies `startScale × event.scale` clamped on each `gesturechange`, and prevents the default on
all three, or the webview zooms the page. Both paths anchor at the pointer through
`updatePointer`, as the wheel does now.

### D9. The account rail is a `TagSidebar`-shaped list fed by the result's groups

`packages/app/src/lib/components/library/AccountRail.svelte` takes `groups: GroupSlice[]`,
`tagQuery` and `onquery`; rows in the given order (largest first from Rust), each with `+`,
`−`, the handle and the count, the include/exclude marking from `activeTerms` (blue for the
account colour the inspector already uses, red struck-through for excluded). `tag-utils` gains
`addAccountToQuery` and `excludeAccountFromQuery`, written on `rewriteMetatagList` beside
`toggleAccountInQuery` with the same excluded-stops-being-excluded rule, and tests beside the
tag pair's. `LibraryScreen` renders the rail as an `aside` (`w-48 shrink-0 border-s
overflow-y-auto`) between the grid column and the inspector's `aside`, only while
`results.group === 'x-account'`; clicks go through `searchKeeping` like every other sidebar
click. The inspector's width is untouched, so the grid gives up the width.

## Risks / Trade-offs

- [The CSS resize handle is small and unlabelled] → it is the note's textarea's handle already,
  in the same panel; the fold covers the "I want it gone" case.
- [A pinch on WebView2 arrives as ctrl+wheel and so does a real ctrl+wheel from a mouse] → both
  zoom, which is what ctrl+wheel means in every image viewer; no harm.
- [`gesturechange` is WebKit-only and TypeScript's DOM lib does not declare it] → a local
  `interface GestureEvent extends UIEvent { scale: number }` and `on:gesturechange` through
  a typed `addEventListener` in an effect, not a Svelte attribute, so the checker sees it.
- [The session store keeps a result's rows in memory for a screen nobody is on] → a page of
  records per view, freed by the next `run`; nothing compared to the thumbnail cache.
- [Cover upscales a small image] → covering is the ask; the wheel down returns to the fit.
- [The fold setting adds a Rust command] → copied from the notes field line for line, tests
  included; the one place a field is easy to forget is `settings.rs`'s save.

## Migration Plan

None: no schema, no file format. The new settings key reads as `false` when absent.

## Open Questions

None.
