## Context

See proposal.md — Why. The frame owns the top bar and the sidebar; a route fills two slots
through `frame.svelte.ts`: `toolbar` (a snippet rendered after the sidebar trigger) and
`filters` (a snippet rendered at the top of the sidebar, inside a `min-h-0 flex-1
overflow-y-auto` wrapper keyed `data-sidebar="filters"` that `app.css` gives a stable scrollbar
gutter). `LibraryScreen.svelte`'s `toolbar` snippet holds `SearchBar`, `ViewControls`, the tile
`Slider`, the inspector toggle and the screen-dependent row; its `filters` snippet holds
`RatingPills` and `TagSidebar`. `NotesPanel` is mounted by the frame below the slot, then the
nav (`mt-auto`) and the footer. `SearchBar` is a `<form>` with two `h-8` fields side by side and
a Clear button; `/` (`screenKeys`) focuses `#tag-query`. `ViewControls` is two `Select`s.

## Goals / Non-Goals

**Goals:** one flexible section (the tag list) and everything else fixed; the search and the
order controls keep their components and behaviour; the top bar's remaining contents never
give way to the selection row.

**Non-Goals:** a second layout for narrow windows; moving the note (it is the frame's, `notes`
design D15, and stays where it is relative to the nav).

## Decisions

### D1. The slot is renamed `sidebar` and becomes a flex column; the tag list is its `flex-1`

`frame.filters` → `frame.sidebar` ("filters" no longer describes a slot that holds the search
and the order). The frame's wrapper becomes `flex min-h-0 flex-1 flex-col` with no scroll of its
own; `TagSidebar`'s `<section>` becomes `min-h-0 flex-1 overflow-y-auto` and takes the
`data-sidebar="filters"` hook (renamed `data-sidebar="tags"` in `app.css` with it), so the
stable gutter and the sideways clip apply to the one thing that scrolls. Rating, Search and
Filter are natural height above and below it.

*Alternative rejected:* keeping one scrolling wrapper for the whole slot. That is what exists,
and it is why the search fields would scroll away above a long tag list.

### D2. `SearchBar` stacks and shrinks; it stays one component

The same `SearchBar` renders in the sidebar as a column: the tag field, the text field, and
Clear as a small text button under them, right-aligned, present only when there is a query
(as now). Fields are `text-xs` — the text field `h-7`, the tag field `min-h-7 max-h-32` because a
`multiline` field sizes to its content and ignores a fixed height — with shorter placeholders (`Tags` / `Title or URL`); the
example syntax moves to the tag field's `title` so the hint is still there on hover. The
component's typing pause, Enter and Escape behaviour is unchanged. It keeps its floor rule but
the floor is now the sidebar's width.

### D3. `ViewControls` stacks under a `Filter` heading

Two full-width selects under a section heading `Filter`, the owner's name for the section (it
matches the Danbooru sidebar they are used to, and the panel's headings are the words they
gave: Search, Rating, Tags, Filter, Notes). The component is the same; only its wrapper changes.

### D4. `/` expands the sidebar before focusing

`screenKeys`' `/` branch calls `useSidebar().setOpen(true)` (the provider's context, available
because the screen renders inside `Sidebar.Provider`) and then focuses `#tag-query` after the
next frame, because the field is `display: none` on the rail until the sidebar has expanded.
The map's row text stays "move focus to the tag search field"; the spec's regions requirement
records the expansion.

### D5. What stays in the top bar, in order

Sidebar trigger · tile size · inspector toggle · the screen-dependent row · full-screen button
(from `windows-fullscreen`, `ms-auto`). The tile slider stays by the owner's call ("keep
thumbnail size there"): it acts on the grid beside it, not on the query.

## Risks / Trade-offs

- [The sidebar is narrower than the old band: long queries wrap in a small field] → the tag
  field is the growing `TagInput`; multiline is already how the inspector uses it, and the
  sidebar's field takes the same `multiline` prop so a long query is readable.
- [A short window leaves the tag list a few rows tall] → the note can be folded (its own
  spec); nothing else in the column is collapsible, and that is by design — the search and
  the pills are why the panel exists.
