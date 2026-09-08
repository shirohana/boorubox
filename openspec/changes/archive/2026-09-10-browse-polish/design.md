## Context

See proposal.md for motivation. What the source actually does today, read before deciding
anything below:

- **The dropzone.** `routes/+page.svelte` holds `hovering` and renders the overlay from it;
  `lib/api/drag-drop.ts` maps Tauri's webview drag-drop events onto three handlers — `drop`
  and `leave` by name, *everything else* (`enter`, `over`) onto `onhover`. Those are OS-level
  events: the file says so, and says why HTML5 drop cannot be used (no filesystem path). The
  tile's thumbnail is a plain `<img>` inside a `<button>`, neither marked undraggable, so the
  webview starts a drag of its own for it, which the window then reports as a drag over
  itself.
- **The viewer.** `Lightbox.svelte` is a native `<dialog>` opened with `showModal()` from an
  effect. Nothing sets the focus, and nothing marks a focus target. Its `onkeydown` sits on the
  `<dialog>` element, so it only ever sees keys whose target is inside the dialog: if the focus
  never entered it, the arrows are the grid's. `mode: 'gallery' | 'inspect'` is component
  state, and the component is unmounted by `{#if lightboxOpen}` on close. There is no click
  handler anywhere on the dialog.
- **The rating control.** `RatingControl.svelte` uses the `toggle-group` copy-in over bits-ui.
  With `rovingFocus` at its default the item's keydown hands arrows to `RovingFocusGroup`,
  which moves the focus and calls `preventDefault()` — but not `stopPropagation()`, so the
  event still reaches the dialog's handler, which moves the image as well. Only one item
  carries `tabindex="0"` (`getTabIndex`), so Tab leaves the group instead of stepping through
  it. bits-ui documents `rovingFocus` as the switch between the two: "when enabled, users
  navigate between the items using the arrow keys. When disabled, users navigate between the
  items using the tab key."
- **The grid.** `LibraryGrid.svelte` computes `shown.columns` from the viewport width and
  keeps it to itself. `grid-focus.ts`'s `moveFocus` is the pure row arithmetic, group slices
  included, and is clamped by `navigation-math.ts`'s `offsetIndexClamped`; the lightbox uses
  `offsetIndexBounded` from the same module. `ImageCard.svelte` marks the current tile with
  `border-ring` on the button and shows the hover overlay for it, and a single click only
  focuses.
- **The sidebar edge.** `Sidebar.Rail` is a shadcn copy-in: a `<button tabindex="-1">` that
  calls `sidebar.toggle` and carries `in-data-[side=left]:cursor-w-resize` and its mirror.
  Copy-ins are upstream code managed by `shadcn-svelte add` (CLAUDE.md).
- **Import metadata.** `import.rs`'s `captured_at(&Metadata)` returns the file's mtime and
  falls back to `db::now_ms()` when the platform cannot report one, because `images.captured_at`
  is `NOT NULL`. `ingest::IngestInput` carries `captured_at`; `IMAGE_COLUMNS` fixes the column
  order `row_to_record` reads by index. `db.rs`'s `MIGRATIONS` is a list whose length is the
  `user_version`; v1 is Phase 1, v2 is `bridge-extension`.
- `packages/shared/src/index.ts` and `src-tauri/src/model.rs` are hand-mirrored (Phase 1 D11):
  a type added here is written twice, in one commit.

## Goals / Non-Goals

**Goals:**

- Every key and click in the library screen has exactly one effect. Nothing fires twice, and
  nothing fires in a region the user is not in.
- The viewer is a region the user is *in*: it takes the focus, owns its keys, and gives them
  back to the grid on close.
- No affordance promises something that does not happen — not a resize cursor, not a drop
  overlay, not a focus outline around a box that is not a control.
- One place per behaviour. The row step is the grid's arithmetic wherever it is used; the
  guard that keeps a region's binding out of a control's way is one condition, not a list of
  components.

**Non-Goals:**

- Rewriting the viewer's markup or its chrome. Every decision here is additive to the dialog
  that ships today.
- Editing a shadcn copy-in. Where one behaves wrongly for this app it is configured from the
  call site (D3, D10), so `pnpm dlx shadcn-svelte add` stays safe to re-run.
- Making the focus ring a system. One tile marking and one viewer tab order; no pass over the
  rest of the app.
- A general "which region owns the keyboard" dispatcher. `app-shell` D14 refused one and its
  argument stands; D3 is a condition, not a dispatcher.

## Decisions

**D1. The tile stops starting an OS drag; `drag-drop.ts` is left alone.**

The overlay is drawn from OS-level drag events, and the only way one arrives while the pointer
never left the window is that the app started a drag itself: an `<img>` is draggable by
default and a webview promises the image to the OS. So the fix is at the source — the tile's
thumbnail (and the viewer's image, which is the same `<img>` case one layer up) is marked
`draggable="false"` — and not in the event mapping, where "was this drag ours?" is not a
question the payload answers: Tauri's `over` event carries a position and no paths at all, so
a paths test would have to run on `enter` alone and would be guessing about what a promised
drag puts on the pasteboard.

Nothing in the app wants to drag an image: in-app drag and drop is a non-goal (proposal), so
suppressing the drag costs no feature. If one is ever built it will need its own drag source
and its own decision about the overlay, and `draggable="false"` is exactly the line it will
delete.

Amended after the owner's pass: `draggable="false"` stopped the OS drag and with it the overlay,
but it left the gesture itself meaning nothing — a press dragged a few pixels across the tile and
released still arrived as a click, and so still opened the viewer. What a travelled press does is
a question about the tile's click, and it is answered in D7.

**D2. The viewer takes the focus on open, onto an element that is not a tab stop.**

`showModal()` is left to do what it is there for — trapping Tab, closing on Escape, restoring
focus to the opener (`app-shell` D10) — but where the focus lands inside the dialog stops being
left to the engine's dialog-focusing steps, which differ between WebKit and Chromium and which
nothing in this app pins. The dialog's first child becomes a named container with
`tabindex="-1"` and no outline, focused immediately after `showModal()`; the `<dialog>` itself
gets no `tabindex` and is never focused by us. From there Tab visits `Previous`, `Next`,
`Info`, `Close` and then the inspector's controls when it is showing, and Shift-Tab from
`Previous` reaches the last of them instead of outlining the whole box.

Alternative — `autofocus` on the `Previous` button — rejected: the viewer would open with a
button ringed and, at the first image, that button is disabled, so the focus would land
somewhere else again. The user opened a picture; the focus belongs on the thing showing it.

Amended after the owner's pass: `showModal()` is no longer left to trap Tab either. Leaving it
was right at the time and for the same reason the rest of the dialog is native — a hand-rolled
trap is how keyboard users get stranded, and the trap is the one part of `showModal()` no
engine was known to differ on. It stopped holding in the engine this app ships: in WebKit five
Tabs (or two Shift-Tabs) from the viewer's chrome park the focus on a control of the page
*behind* the modal, where Space is no longer the viewer's key and nothing reaches the dialog at
all. So the component walks the cycle itself, over the dialog's own tabbable elements read at
the keystroke — so the inspector's controls join and leave the cycle as the panel opens and
closes, and a disabled `Previous` is not a stop. `tab-cycle.ts` holds the wrap and is tested;
everything else `showModal()` gives is untouched. The Tab branch sits *before* the typing
guard, because the tag field is inside the dialog: Tab out of it is a move between the viewer's
controls, not a character. The suggestion list still takes Tab first — it prevents the default,
which D3's condition reads.

**D3. A binding does not fire for a key another control already handled.**

One condition, added to the viewer's `onkeydown` beside the existing `isTypingTarget` guard:
an event whose default is already prevented was handled by whatever it came from, and the
region does not act on it a second time. That is what makes Left/Right inside the rating radios
move only the radios, and it is written once rather than as a list of "controls the viewer must
ignore" that every later change would have to extend.

The other half — Tab and Shift-Tab stepping through the five choices — cannot come from the
same place, because bits-ui makes roving focus and tab-stepping exclusive (Context). So
`RatingControl.svelte` sets `rovingFocus={false}`, which gives every choice `tabindex="0"`, and
owns the arrow step itself: a keydown on the group element moves the focus to the neighbouring
`role="radio"` element and calls `preventDefault()`, which is also what D3's condition then
reads. Enter and Space still toggle through the copy-in, which handles them before the roving
branch it now skips.

Alternative — keep roving focus and tell the owner that a radio group is one tab stop by the
ARIA pattern — rejected by the owner, who asked for both; and the group is not a `radiogroup`
in the first place (bits gives the root `role="group"` and the items `role="radio"`), so the
pattern's own reason for a single tab stop is not being honoured today either. The trade-off is
that the control now costs six tab stops in the inspector instead of one; the viewer's tab
order is short enough that this is what the owner was reaching for.

The same control sits in the inspector beside the grid, where nothing leaks: the grid's keys are
bound on its scroll container and the inspector is outside it.

**D4. Space closes the viewer wherever the focus is inside it, and a click beside the image
closes it too.**

Space keeps its row in the keyboard map, and after D2 it is reached again because the focus is
inside the dialog. Where the focus is on one of the viewer's own buttons, the button's own Space
wins (it is a button, and stealing that would be worse than the binding); the container D2
focuses on open is the case the map is about.

For the click: the dark region is two things — the dialog's `::backdrop`, whose clicks target
the `<dialog>` element itself, and the empty space around the image *inside* the transparent
dialog box, which is the larger of the two at 96vw and reads as the same surface. Both close,
by the same rule: a click closes when its target is the element that was clicked on, not a
descendant of it — the dialog itself, or the container the image is fitted into. A click that
lands on the picture, the header, a button or the inspector is a click on that thing and does
nothing here.

Amended in place after the first build: "the button's own Space wins" needed its own
mechanism. A native button does not mark Space's keydown as handled (it acts on keyup), so
D3's `defaultPrevented` guard never sees it. Space closes only when the keydown's target is
one of the two non-controls — the focus surface from D2, or the `<dialog>` itself, which is
where browsers park the focus after a click on the image or on empty space — not a list of
controls. (Measured while checking this: WebKit sizes the `<img>` to the picture —
513×717 for a 1144×1600 image in a wider stage — so the strips beside it are the stage and
the target rule holds; no rectangle test is needed.)

Alternative — close on any click that is not inside the image — rejected: it would swallow a
click that ends a text selection in the inspector, and the "is my target this element" test is
the one the platform already gives for free.

Amended again after the owner's pass: "on a focused button Space is the button's press" is
still right — a rating choice takes Space and sets the rating, which is what the `rating` spec
asks of it — but it was written as if the press ended there. It does not: the focus stays on
the choice, so the *next* Space is another press and the viewer has quietly lost its close key,
with nothing on screen saying so. So a choice reports itself (`RatingControl`'s `onchosen`,
forwarded by the inspector as `onrated`) and the viewer's placement — only that one — puts the
focus back on the D2 surface, which is where Space closes. Beside the grid nothing is passed
and the choice keeps the focus, because there is no viewer key to hand back. Alternative — let
Space close when the target is a rating choice — rejected: that makes the close rule a list of
controls, which is the thing D3 and this decision exist to avoid.

**D5. Up and Down in the viewer are the grid's row step. This reverses `app-shell`'s "Up/down
in the viewer: decided against".**

Why the old reading was right at the time: `app-shell` was building the viewer as a way of
looking at *one* image, and its arrows as a walk along the result (`←` `→`, bounded — nothing at
the ends). In that reading a vertical key has nothing to move along: the viewer shows no rows,
so "one row down" is a number of images that depends on a layout the viewer cannot see, and
adding it would have meant teaching the viewer about the grid it deliberately knew nothing
about. Deferring it cost nothing while the result sets were small.

Why it stopped being right: the owner uses the viewer as a lightbox *over* the visible grid,
not as a separate screen — the grid is what is behind it, the tile it opened from is where it
returns, and moving one image at a time through a few hundred is heavy when the grid the user
came from moves five at a time. The layout the viewer supposedly cannot see is already on the
page: `LibraryGrid` computes the column count every frame, and the page that owns both of them
can hand it over (D9). The half of the old decision that survives is that the viewer does not
learn to lay out a grid: it asks the grid's own arithmetic for an index and shows what comes
back.

So the viewer calls `moveFocus` from `grid-focus.ts` for `↑` and `↓` — the same function the
grid's own keydown calls, group slices included, so a row step means the same thing in both
places and a change to it cannot land in only one. That brings the grid's *clamped* edge with
it, which differs from the viewer's bounded `←` `→`: at the top row, `↑` lands on the first
image rather than doing nothing. Kept deliberately — the vertical keys are the grid's gesture,
performed through the viewer, and `navigation-math.ts` is where the two edge rules are already
written down and argued.

Amended after the owner's pass: the grid behind the dialog follows the viewer. Leaving it
parked was right while the viewer's `←` `→` walked a few images and the grid was still roughly
where the user opened it; a row step at a time makes the two drift far apart, and closing then
scrolls the grid somewhere the user never watched it go — the jump the D5 argument above says
this viewer is *not* (it is a lightbox over the visible grid). So every image change is reported
to the page (`Lightbox`'s `onmove`), which hands the index to `LibraryGrid.scrollIntoView` — the
same scroll the grid's own arrow keys use, exported rather than written a second time. Scroll
only: the DOM focus stays in the dialog, which is what D2 pinned, and the grid's focused card
is still moved once, on close.

**D6. Inspect mode becomes the page's session state, not the dialog's.**

The flag moves out of `Lightbox.svelte` and next to `inspectorOpen` in `routes/+page.svelte`,
bound into the viewer. The comment in the viewer that says the session "is this dialog" is
replaced by the reason it is not: the user who opened the panel wants the panel, and closing a
picture is not a statement about it. This is still session state, not a setting — `app-shell`'s
non-goal (nothing but the theme and the tile size persists across launches) is untouched, and
that is what the spec scenario "after a restart it opens on the image alone" pins.

The grid's own inspector column keeps its own flag. The two panels are in different places and
the owner toggles them for different reasons; one flag for both would mean opening the viewer
could not help changing the grid behind it.

**D7. A click on the tile that is already current opens it. This reverses half of `app-shell`
D9.**

Why the old reading was right at the time: D9 needed a card to be *current* without being
*open*, so that the inspector had something to describe, and it made every single click
focus-only to get it. That rule is what the inspector is built on and it stays.

Why half of it stopped being right: the second click on the same tile carries no information
for the inspector — the panel is already showing that image — so refusing to act on it buys
nothing and costs the owner the most natural way to open a picture. Double click still opens
(unchanged), Enter and Space still open, and the first click on any other tile still only
focuses, so the rule D9 was protecting is intact: a click never opens an image the inspector was
not already describing.

The cost is that a double click on the tile that is already current opens on the first of the two
clicks and the second lands inside the dialog — which, after D4, is a click that closes it. The
dialog's close therefore ignores a click that belongs to a multi-click gesture (`event.detail`
above one). The first build assumed the DOM would sort this out because the dialog "is not yet
mounted" for the second click; the review showed otherwise: the dialog mounts on the effect
flush, a microtask after the first click, and a modal dialog makes the rest of the document
inert, so the second click of the pair can only land on the dialog. The tile's handler has a
trap of its own: `focused` is a live prop, so it must be read before the handler moves the
focus index, or every first click reads as a click on the current tile.

Amended after the owner's pass, which found every click on a not-current tile opening the viewer
even though the handler already read `focused` first. Reading it first is not early enough: the
tile's button is wrapped in a `ContextMenu.Trigger`, whose props carry `tabindex="-1"`, and WebKit
focuses a tabbable element on mousedown — buttons are the macOS exception, a `tabindex`ed div is
not. So `focusin` fires on the wrapper and `onfocus` makes the tile current *before* the click
event exists, and no code inside the click handler can see what the tile was. The current-ness is
therefore read at `pointerdown`, which is dispatched before mousedown's focus, and carried to the
click.

The same press answers the drag question D1 hands over: a press that travels more than
`TILE_DRAG_SLOP_PX` (4 px) before it is released does not activate. It still moves the focus and
the selection — the user pressed on that tile and the inspector should describe it, and cancelling
the focus as well would make a twitchy click do nothing at all — but opening the viewer is the
half of the gesture the user abandoned. The decision and the threshold live in `tile-click.ts`
with a unit test, because the component around them is verified only by hand.

**D8. The current tile is marked with a ring; the hover overlay is untouched.**

`border-ring` swaps a one-pixel border colour that reads as nothing at a glance across a screen
of thumbnails. It becomes the ring the missing-file card already uses for the same state
(`ring-3 ring-ring/50`), so the two card states mark "current" the same way and the grid has one
focus marking rather than two. The overlay keeps firing on hover *and* on current, exactly as
the `library-browse` requirement says: the ring says which tile the keyboard is on, the overlay
says what the tile is, and they answer different questions.

Alternative — dim or scale the tiles that are not current — rejected: it changes what the grid
looks like at rest, and the grid is the product.

Amended after the owner's pass: `ring-3 ring-ring/50` was findable but not at a glance. The
palette is fully neutral, so half-strength `--ring` — a mid grey — against a thumbnail reads much
like the tile's own border. The ring keeps its token and its width and drops the opacity, full
`ring-ring`, and gains `ring-offset-2 ring-offset-background`: the gap the selected-and-current
tile already draws, which separates the mark from the picture instead of laying it on top. It is
not given `--primary`, which is the selected mark and, in this palette, the same near-black as the
foreground. Both card states still take the class from the one `ring` derivation, so the
missing-file card matches without a second rule.

**D9. The grid publishes its column count; the page passes it to the viewer.**

`LibraryGrid` gains a bindable `columns` prop written from `shown.columns`, the page holds it,
and the viewer takes it as a prop. The grid stays the one place that computes it — a second
computation on the page would drift the day the gap or the padding changes — and the viewer
takes a number, not a reference to the grid.

Alternative — the viewer reads the grid through `bind:this` — rejected: it would couple the two
components that today only share a result set, and the viewer is also mounted while the grid is
scrolled away.

**D10. The sidebar's edge keeps its click and loses its resize cursor. It does not become a
resizer.**

The rail is a toggle wearing a resizer's cursor, and the two honest ways out are to make it a
resizer or to stop it looking like one. The second is chosen.

A real resizer would need a width to drag to, and a width the user set has to survive the next
launch or the drag is a joke — that is a third persisted setting in an app whose non-goals say
only the theme and the tile size persist (`app-shell`), plus a minimum, a maximum, and the
collapse threshold interacting with the icon rail. It would also mean hand-editing the copy-in
or replacing it, and `shadcn-svelte add sidebar` would overwrite the edit. Against that, the
thing being bought is a sidebar width, on a screen whose one width control (the tile size)
already shapes what the user is actually looking at. The other panel edge in the frame — the
inspector's — has no resize affordance and nobody has missed one; that is the state this makes
the sidebar match, and it is what the `app-frame` delta now requires of every region edge.

The cursor is overridden from the call site with an important utility rather than by editing the
copy-in, because the copy-in's `cursor-w-resize` is variant-scoped (`in-data-[side=left]:`) and a
plain utility does not outrank it — the same reason and the same shape as `RATING_COLOUR`'s `!`
(`tags-and-ratings` 7.10) and the sidebar's own `top-14!`.

If a resizable sidebar is ever wanted it is its own change: a persisted width, a drag handle, and
a decision about what the icon rail does under one.

**D11. Capture time is the import time; the file's modification time gets its own column, and
the schema version is read from the list rather than pinned here.**

`images.file_modified_at INTEGER NULL`, epoch milliseconds, appended to `MIGRATIONS` as one
entry. Version ownership across the parallel Phase 2 changes was planned as v1 `phase-1-app-mvp`,
v2 `bridge-extension`, v3 `auto-tag-rules` (its D1), v4 `booru-upload` (its D1), making this
change **v5** on paper — but `auto-tag-rules` and `booru-upload` were only planned, not applied,
when this change's group 5 was implemented, so `MIGRATIONS` held only `SCHEMA_V1` and
`SCHEMA_V2` at the time and this change took the next slot, **v3**. Whichever of the three lands
first takes the next number; none of them is pinned to one, and none of them needed to be
re-planned to land out of the order they were written in. Following `booru-upload` D1 rather than
re-arguing it, the number is not written into a test: the test asserts
`user_version == MIGRATIONS.len()` and the presence of the column, so a sibling landing first (or,
as it turned out, this change landing before its siblings) costs nothing. Nullable, because most
images never came from a file the user had; `NULL` is "there is no such fact", which is the
honest state for every extension capture.

The column is appended to the end of `IMAGE_COLUMNS`, after `missing`, so no index in
`row_to_record` moves. `IngestInput` gains `file_modified_at: Option<i64>`: the local import
passes the file's mtime, the HTTP capture path passes `None`, and the field is the only thing
that knows about files, so nothing else in ingest changes.

`import.rs`'s `captured_at(&Metadata)` becomes `file_modified_at(&Metadata) -> Option<i64>` and
loses its fallback to `now`: the fallback existed because `captured_at` is `NOT NULL` and a row
claiming 1970 was worse than a row claiming today, and the new column has no such problem — a
platform that cannot report a modification time gives `None`. `captured_at` becomes
`db::now_ms()`, so the two timestamps a fresh import carries (`captured_at`, `created_at`) are
the same moment, which is exactly what "this arrived now" means for a file the user handed over
today.

The Inspector shows it (Slot: Inspector · identity) as a "File modified" row after "Imported",
falling back to the `—` the panel already uses for facts an image does not have. Not a sort key:
`SortField` gains nothing here (proposal, Non-goals), and the day an ordering wants it the column
is already there.

**D12. Nothing is backfilled, and no existing row is rewritten.**

Rows imported before this change keep the capture time they were given and get `NULL` for the
file's modification time. Reading the mtime of the copy under `images/` would not recover the
fact: that file was written by ingest, so its mtime is when the import ran, and a backfill would
be inventing a value that looks like a measurement. Rewriting `captured_at` on old local rows is
worse still — it would reorder a library the owner already knows the shape of, in a change whose
entire point is that images stop moving unexpectedly.

The practical consequence is a seam: local images imported before this change sort by their old
files' mtimes, ones imported after by when they were imported. The seam is visible, explicable
and one-directional, which is better than a silent rewrite of history.

**D13. Where each change lands (`app-shell` slot map).**

| Slot | This change puts there |
| --- | --- |
| Grid · tile | the current-tile ring (D8), the click that opens the current tile (D7), and no OS drag (D1) |
| Inspector · identity | the "File modified" row (D11) |
| Inspector · rating | the rating control's own arrow step and its five tab stops (D3) |
| Lightbox · inspect panel | unchanged; the mode that decides whether it is showing moves to the page (D6) |
| Settings · Keyboard | the viewer's `↑` `↓` row, through `KEYBOARD_MAP` (D5) |

The sidebar rail is not a slot: it is the frame's own chrome from `app-shell` 6.2, and D10
changes how it looks, not what lives in it.

## Risks / Trade-offs

- [`draggable="false"` also kills dragging an image *out* of the app — into a browser, a chat
  window, a folder] → accepted, and it is the state the app has always been in for the tile as a
  whole; nothing in §6 or the legacy viewer offers drag-out, and the overlay firing on every
  in-grid drag is the concrete cost being paid for a capability nobody has asked for. The day
  drag-out is wanted, D1 names the line to delete and the overlay question comes back with it.
- [D3's `defaultPrevented` guard silences a viewer binding whenever some control inside it
  prevents that key for its own reasons] → that is the intent, and the failure mode is a key that
  does nothing rather than a key that does two things; a control that prevents a default it does
  not act on is a bug in that control.
- [`rovingFocus={false}` diverges from what every other toggle group in the app would do] →
  there is only one toggle group in the app (the rating control) and it is the one the owner
  asked this of; a second one that wants roving focus gets the default by writing nothing.
- [Up/Down clamp while Left/Right are bounded, inside the same viewer] → deliberate (D5), and the
  asymmetry is the one `navigation-math.ts` already documents between the grid and the viewer;
  the alternative — bounding the row step — makes `↓` dead on the last partial row, which is
  where the user is most likely to press it.
- [The viewer's `↑` `↓` are wrong the moment the grid re-flows behind it — a window resize, a
  tile-size change while the viewer is open] → the column count is a live binding, so it follows;
  the residual case is a re-flow whose result the user cannot see because the viewer covers it,
  which changes how far a row step jumps and nothing else.
- [A migration lands while three other Phase 2 changes each plan one] → `MIGRATIONS` is a list and
  the version is its length (D11); the entry adds a nullable column to `images` and collides with
  nothing a sibling adds. A library opened by an older build after the migration is refused with
  `SchemaTooNew`, which is the existing behaviour for every migration and the reason the runner
  reads the version rather than assuming it.
- [The seam D12 leaves in capture times] → visible in the inspector, where both times are shown
  side by side and named apart.

## Migration Plan

One schema migration, forward-only, applied on the next open of every library: it adds a nullable
column and rewrites no row, so opening is not slowed measurably and nothing has to be rebuilt (no
FTS change — the column is not searchable text). There is no down-migration; the runner's
`SchemaTooNew` refusal is what an older build does with a migrated library, unchanged from Phase
1. Rolling back means not shipping the change, which is why the migration is the last group of
tasks rather than the first.

The webview change that goes with it is additive: `ImageRecord.fileModifiedAt` is a new field on
a type only this repo consumes, and every reader that ignores it keeps working.

## Open Questions

- Whether `↑` `↓` should also work in the viewer while a group heading falls between the two rows
  — `moveFocus` already steps into the neighbouring group at the same column, so the behaviour is
  defined; what is unknown is whether it reads as right in a grouped view, which only the owner
  using it will say.
- Whether the "File modified" row should be hidden entirely for images that have none, rather than
  showing `—`. The panel's other absent facts show `—`, so this follows them until the row is on
  screen next to real data.
