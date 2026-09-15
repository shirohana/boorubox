> Depends on `app-shell`, `tags-and-ratings`, `notes`, archived; lands after `inspector-polish`
> and `windows-fullscreen`. One implementing agent, webview only. Gate: `pnpm -r typecheck`,
> `pnpm lint`, `pnpm --filter @boorubox/app test`, then `mise run check`.

## 1. The slot and the column (agent A)

- [x] 1.1 `frame.svelte.ts` + `frame/Sidebar.svelte` + `app.css`: rename the slot to `sidebar`,
      the wrapper becomes a flex column with no scroll, the scroll rules move to
      `data-sidebar="tags"` (design D1). `tags/TagSidebar.svelte`: its section is the flexible
      scrolling one. Verify: typecheck and lint pass.
- [x] 1.2 `library/SearchBar.svelte`: stacked, `h-7 text-xs`, short placeholders, syntax hint
      in `title`, `multiline` tag field, Clear under the fields (design D2). Verify: typecheck,
      lint and the app's tests pass.
- [x] 1.3 `library/ViewControls.svelte`: stacked full-width selects under a `Filter` heading
      (design D3). Verify: typecheck and lint pass.

## 2. The screen (agent A)

- [ ] 2.1 `library/LibraryScreen.svelte`: the `sidebar` snippet renders SearchBar, RatingPills,
      TagSidebar, ViewControls in that order; the `toolbar` snippet keeps the slider, the
      inspector toggle and the screen-dependent row (design D5); `/` expands the sidebar first
      (design D4). Verify: typecheck, lint and tests pass.
      Hand check: the sidebar reads Search, Rating, Tags, Filter, Notes, nav, library; on the 25k
      vault the tag list scrolls alone while everything else holds still; typing in the tag field
      still runs the search after a pause and on Enter, Escape leaves it; collapse the sidebar,
      press `/` — it expands and the tag field has the focus; select twenty images at a 1000px
      window — every selection action is visible; the thumbnail slider is still in the top bar.

## 3. Change-level verification (owner)

- [ ] 3.1 `mise run check` green; the hand check passes on Windows.

## Handoff

Groups 1 and 2 (agent A) are implemented and gated (see below); 2.1's hand check is left
unticked for the owner.

- **`SearchBar.svelte` gained a `Search` heading.** Task 1.2 didn't name it, but design D3 lists
  the sidebar's headings as "Search, Rating, Tags, Filter, Notes" and the hand check reads them
  off the panel, so `SearchBar` now wraps its form in `<section><h2>Search</h2>...` to match
  `RatingPills`, `TagSidebar` and the new `ViewControls` heading.
- **The syntax hint reaches `title` without editing `TagInput.svelte`.** `TagInput` is outside
  this agent's file ownership. Its Props interface has no `title` passthrough, so
  `SearchBar.svelte` wraps just the tag field in a plain `<div title="cat -dog · cat or dog ·
  rating:s,q">`: an element with no `title` of its own shows the nearest ancestor's on hover
  (HTML's own inheritance rule), so the tag field's tooltip works without a `TagInput` change.
  If a future change touches `TagInput.svelte`, a first-class `title` prop would let this drop
  the wrapper.
- **The tag field's height is `max-h-32 min-h-7`, not the literal `h-7` design D2 names.**
  `multiline` renders a `Textarea`, whose base class sets `field-sizing: content`; a fixed `h-7`
  has no effect on that element (browsers use it only as `min`/`max`, not a hard sizing constraint
  once content-sizing is on), so `min-h-7` gives it the same resting height as the `h-7` text
  field and `max-h-32` is a new cap (unspecified in design) so a very long query can't fill the
  whole sidebar.
- **`TopBar.svelte` needed no change.** The full-screen button's `ms-auto` already pins it last
  regardless of what `frame.toolbar` renders before it; moving `SearchBar`/`ViewControls` out of
  the toolbar snippet didn't require touching it.
- **`/` (design D4):** `useSidebar()` is read once at `LibraryScreen`'s top level (`sidebarState`)
  because `getContext` only works during component initialisation, not later inside the
  `screenKeys` handler. The handler calls `sidebarState.setOpen(true)`, then `await tick()` before
  focusing `#tag-query` — `tick()` is enough because the sidebar-gap/-container transition is
  already disabled in `app.css`, so there's no animation to wait out, only the DOM update the
  `hidden` class removal needs. The focus guard now accepts `HTMLTextAreaElement` as well as
  `HTMLInputElement`, since the sidebar's tag field is multiline.
