## Context

Phase 1 shipped: Rust owns the library (one `Connection` behind a `Mutex` in `AppState`), the
webview is a SvelteKit static SPA with two routes, and every command is wrapped once in
`src/lib/api/`. The UI is one screen — `ListenerBanner`, `CountsPanel`, `ImportPanel` and
`SearchBar` stacked in a header above `LibraryGrid`, with `Lightbox` as a modal — plus
`/setup`. Motivation: see proposal.md.

Constraints that shape this design:

- Rust owns filesystem, storage, listener and ingestion; the webview is UI only (§6). Nothing
  in this change moves logic across that line.
- Settings live in the app config dir, never in the library (Phase 1 D6): a copied library must
  not carry another machine's paths.
- `packages/shared/src/index.ts` and `src-tauri/src/model.rs` are hand-mirrored (Phase 1 D11).
  Every new type in this change is written twice, on purpose, in one commit.
- shadcn-svelte copy-ins under `src/lib/components/ui` are upstream code, excluded from lint
  and format (CLAUDE.md). Only `button` and `input` exist today.
- Phase 1 D7a: the asset scope is granted per directory when a library opens. Switching
  libraries grants a second pair of directories; the scope only ever grows within a session.

## Goals / Non-Goals

**Goals:**

- Every region of the app has a name, and every later change knows which one it lands in
  without re-deciding — the slot map below is that contract.
- What is on screen is what works. A read-only fact is honest; a disabled control that will
  mean something in two changes' time is not.
- The frame is one layout, not per-route markup: a new screen inherits the sidebar, the
  toolbar and the keyboard map by existing.
- Changing which library is open costs a click, and never leaves two open.

**Non-Goals:**

- A design system. Tokens are the ones `app.css` already defines from the shadcn "nova"
  preset; this change adds no palette of its own.
- Reworking the search, ingest or storage layers. The only Rust that changes is settings, the
  command surface, and the listener's ability to rebind.
- Persisting per-view state (which panel is open, scroll position) across launches. Only the
  theme and the tile size are settings; everything else is session state.

## Decisions

**D1. One library open at a time, but switchable — reversing Phase 1's "Multi-window or
multi-library. Design for one open library per app instance."**

Why the old reading was right at the time: Phase 1 had one folder, picked once on a screen
that then became unreachable. "One per app instance" bought the simplest thing that could
work — a single `Mutex<Option<Library>>`, a single listener, no library picker in the frame,
no question about which library a capture belongs to — and nothing in Phase 1 asked for a
second folder.

Why it stopped being right: the owner keeps more than one library and moves between them as a
matter of routine, not as an edge case. Today that costs a quit, a hand-edit of
`settings.json` and a relaunch, because `/setup` is the only screen with a folder picker and
the layout gate makes it unreachable while a library is open. A frame that cannot switch
libraries would have to be redesigned the first time it is used properly.

The reversal is narrow, and the half that was load-bearing survives: **one library is open at
any moment**. `AppState` keeps one `Mutex<Option<Library>>`, one connection, one listener, and
§7's "one machine writes at a time" is untouched. What changes is that the open one can be
closed and another opened without relaunching, and that the app remembers which folders it has
seen. Multi-window and two libraries open at once stay non-goals (proposal.md, Non-goals).

Switching opens the new library first and swaps it into the state on success, so a switch that
fails leaves the working library open. Alternative — clear the slot, then open — rejected: a
typo or an unmounted volume would close a library the user was working in.

**D2. Routes: `/`, `/start`, `/settings`. `/setup` is gone; `/import` stays reserved.**

| Route | Screen | Frame |
| --- | --- | --- |
| `/` | Library: toolbar, grid, inspector | yes |
| `/start` | No library open: recent list, choose a folder, missing-folder state, listener line | no — full window |
| `/settings` | Library section, Capture section, Appearance section | yes |
| `/import` | **not in this change** — `legacy-bundle-import` owns the bundle report and the §9 notice | yes, when it lands |

`/setup` is renamed rather than kept: the screen is no longer a one-time setup step. It is
reached on a first launch, after "Close library", and when a remembered folder is gone, and it
now lists recent libraries. A path named `setup` would describe one of its three entrances.
Alternative — keep the path and change only the screen — rejected: the route name is read by
the next person as what the screen is for.

`/settings` lives inside the frame and therefore needs an open library. Alternative — make
settings reachable from `/start` too — rejected: everything on it (library path, counts, the
listener that has nothing to deliver into) is about an open library. The start screen keeps the
one-line listener notice it already has for the case where the port is taken before any library
is open.

**D3. Closing a library clears `libraryPath`; the folder stays at the head of the recent list.**

`library_status` derives `missingPath` as "a remembered path with no library open" (Phase 1's
argument: a second stored field would one day not be cleared). A deliberate close would light
that up and tell the user their folder was missing. Clearing the remembered path on close keeps
the derivation true and makes "the remembered library opens on launch" mean what it says: after
a close there is no remembered library. The folder is not forgotten — it is the first entry of
`recentLibraries`, one click away. Alternative — a `closedDeliberately` flag beside the path —
rejected for exactly the reason Phase 1 refused a second field.

**D4. `recentLibraries: string[]` in `settings.json`: absolute paths, most recent first,
deduplicated by exact path, capped at 10.**

Strings, not objects: the display name is the folder's basename (derived, never cached) and an
image count would be stale the moment another instance of the app writes to that folder. The
`recent_libraries` command turns the strings into `RecentLibrary { path, name, available }`,
computing `available` — does `library.sqlite` exist and is it readable — **at call time**, when
the start screen is on screen. Alternative — check availability at startup so the status is
ready — rejected: ten `stat` calls on an unmounted network volume block the window from
appearing, and nothing needs the answer until the start screen is drawn.

An entry that fails to open stays in the list (the volume may come back) and is shown
unavailable; `forget_recent(path)` is the only thing that removes one, and it never touches the
folder.

**D5. The settings file grows `theme`, `gridTileSize` and `recentLibraries`; each field is read
and written by a command of its own.**

`settings.json` stays a flat object; `settings.rs` already falls back per field, so a
hand-edited or older file loses at most the field it broke. The webview reads the two
preferences with `app_settings()` and writes them with `set_theme` and `set_grid_tile_size`;
the port is written with `set_listener_port`. Alternative — one `update_settings(patch)` —
rejected: the port has a side effect (the listener rebinds, D6) and a generic patch command
hides that from every caller. The port is *read* from `LibraryStatus.listener.port` and lives
in no other type, so it has exactly one representation on the wire.

`gridTileSize` is stored rather than kept as session state, unlike every other view state in
this change: the alternative is a slider that resets on every launch. The size a user likes is
a preference, and it belongs in the file where preferences already live rather than in a second
store inside the webview.

**D6. Changing the port rebinds the listener in place, which spends Phase 1 D17.**

D17 parked the listener's failure state on a banner over the library because Phase 1 had no
settings screen and building one to hold a single line was the wrong order. The screen exists
now, so the banner goes and `ListenerBanner` is deleted: the `capture-ingest` spec's "settings
show the listener as failed with the reason" is satisfied literally.

The port becomes editable there, and editing it rebinds without a restart. That needs a
shutdown handle the listener does not have today: `http::start` returns the `ListenerStatus`
and spawns an accept loop that ends only with the process. It gains a `tokio::sync::oneshot`
sender kept in `AppState`, and `axum::serve(...).with_graceful_shutdown(...)` on the receiver;
`set_listener_port` signals the old loop, binds the new port, stores the port whether or not
the bind succeeded, and returns the new `ListenerStatus`. Alternative — "takes effect after
restart" — rejected: the one moment a user edits this field is when captures are not arriving,
and a fix they cannot see working is a fix they will retry three more times.

**D7. Per-source counts move to `/settings` → Library; the sidebar footer keeps the total.**

Their job is reconciling a migration against the browser (§2 guarantee 4, §9 step 4) — a
deliberate act performed once, not a number to glance at while browsing. On the library screen
they cost a row of the header that images should have. The running total stays in the sidebar
footer under the library name, where it answers "did that capture land" without a trip.
Alternative — keep the panel on the library screen — rejected as the header row that started
this change. `legacy-bundle-import` shows the total and the legacy-bundle count next to its own
report, which its spec requires and this decision does not touch.

**D8. Import controls become a toolbar menu; the drop listener moves out of the panel.**

"Import files…" and "Import folder…" become one `Import` menu in the toolbar's action slot;
progress shows inline in the toolbar and the per-item report opens as a dismissible card under
it. The window-wide drag-and-drop subscription moves from `ImportPanel` up to the library
route: it is registered inside the panel today, and a panel that lives in a dropdown is
unmounted most of the time, which would silently kill drop-to-import — the source the
`local-file-import` spec leads with.

**D9. The Inspector is one component in two placements, and the grid distinguishes focus from
activation.**

`Inspector.svelte` takes an `ImageRecord | null` and renders read-only facts. It is mounted in
the library route's right column and inside the lightbox's inspect mode; there is one editor to
build when `tags-and-ratings` lands, not two.

For it to show anything beside the grid, a card has to be *current* without being *open*. So a
single click focuses a card (and fills the inspector); double click, Enter or Space open the
lightbox. Alternative — click opens, and the inspector follows the hovered card — rejected: a
hover-driven panel flickers as the pointer crosses the grid, and there would be no way to keep
one image's facts on screen while reading them. This focus index is also what
`selection-and-bulk` extends into a selection model, so the interaction is built once.

The inspector is open by default and toggled with `i`; its visibility is session state, not a
setting (Non-Goals).

**D10. The lightbox keeps the native `<dialog>` and gains a mode flag.**

The modal dialog is load-bearing for focus (the browser traps Tab, closes on Escape, restores
focus to the opener); the comment in `Lightbox.svelte` says so and it stays true. `mode:
'gallery' | 'inspect'` decides whether the inspector column is rendered and whether the
backdrop and chrome are dark and minimal. The image is fitted to whatever space is left, so
switching modes refits rather than crops. Alternative — a second component for inspect mode —
rejected by the owner's rule: one editor, two placements.

Closing focuses the image the viewer showed last, not the card it opened from. As planned the
native dialog's own focus restoration was the whole answer, and it was right while the viewer
was a way to look at one card: opener and viewed image were the same. It stopped being right
once the arrows moved the viewer along a sequence — the owner pressed right ten times, closed,
and the grid was still on the first card, with the inspector describing an image other than the
one just looked at. The dialog still restores focus to the opener (that is what makes Tab and
Escape sound); the page then moves the grid focus to the viewer's last index, which also scrolls
it into view. Recorded here because it overrides the "returns focus to the thumbnail it was
opened from" scenario that shipped with this change.

**D11. The grid becomes square, aspect-fit tiles whose edge length is a parameter.**

Today `grid-window.ts` hard-codes `MIN_CARD_WIDTH = 168` and `CARD_HEIGHT = 200`, and each card
is an image box above a caption strip. The caption strip is what makes the grid look like a
file listing; it moves into the hover overlay and the inspector. `gridWindow` gains a `tile`
input (the target edge in px, clamped 120–360, default 180) and drops the two constants:
`columns = floor((contentWidth + GAP) / (tile + GAP))`, and the row height is the width a
`1fr` column actually gets plus `GAP`, returned as `rowHeight`. As planned this read
`ROW_HEIGHT = tile + GAP`, which was right while the card had a fixed height: the tile was
the row. It stopped being right once the tile became `aspect-square` inside
`repeat(columns, minmax(0, 1fr))`: `tile` is the width a column may not go below, the leftover
is shared out, and at one column a tile is nearly twice `tile` wide — rows positioned at
`tile + GAP` would overlap the ones the CSS grid lays out. The column count is unchanged. The
virtualisation claim — DOM nodes depend on the viewport, never on the library — is unchanged
and stays asserted in `grid-window.test.ts`, now across tile sizes. The toolbar slider writes
`gridTileSize` (D5) on release, not on every frame of the drag.

**D12. Theme: a `dark` class on `<html>`, resolved before the first paint.**

`app.css` already defines the whole dark palette under `.dark` and `@custom-variant dark`.
Nothing else is needed: a `theme.svelte.ts` module resolves `system | light | dark` against
`matchMedia('(prefers-color-scheme: dark)')`, toggles the class, and keeps listening while the
setting is `system`. The root layout awaits `app_settings()` alongside `library_status()` and
renders nothing until both answer — it already renders nothing until the status arrives, so
this costs no new flash and removes the light-palette flash a late toggle would cause.
Alternative — a CSS-only `prefers-color-scheme` media query — rejected: it cannot express the
explicit light-while-the-system-is-dark choice the spec requires.

**D13. macOS gets `titleBarStyle: "Overlay"`; Windows keeps its own title bar.**

Decided rather than deferred, because it changes the sidebar's top region and therefore a task.
With the default bar, the window stacks a system title bar, then a toolbar, then the grid —
three horizontal bands before an image, in an app whose content is images. Overlay removes one
and reads as a native modern media app. `titleBarStyle` is a macOS-only key, so Windows is
unaffected and no per-platform config is needed.

The cost is the traffic lights, which float over the window's top-left. A frame-owned top bar
spans the window above the sidebar and the content: it is the drag region
(`data-tauri-drag-region`), reserves the top-left 80×28 for the lights, and holds the sidebar
toggle followed by the route's toolbar controls. macOS is detected in the webview from the
user agent and stamped on `<html>` as `data-platform="macos"`, which the reserve keys off.

As first built the sidebar had its own header with the app name and the drag region, and the
toolbar band was rendered by each route. That was right while the sidebar could not collapse:
the header was always there to drag from. It stopped being right when the owner asked for a
collapsible sidebar (task 6.2): the toggle lived in that header, the icon rail is too narrow
to hold it under the lights, and a control that vanishes at the moment it is needed is not a
control — nobody knew the rail's edge strip re-expands it. Obsidian is the reference: the
toggle sits in the title bar at a fixed place, the icon ribbon stays, only the pane collapses.
The bar being the frame's also reverses the page-rendered band's argument (an empty bar on
`/settings`): with the toggle in it the bar is never empty. The app name went with the header;
it told the user nothing they did not know.
Alternative — `@tauri-apps/plugin-os` for the platform — rejected: a plugin, a permission and an
async call for one boolean the user agent already carries.

Drag regions need `core:window:allow-start-dragging` in the capability — `core:default` does
not include it, and without it `data-tauri-drag-region` is inert, which is how the first build
shipped with a window nobody could move. The regions are the sidebar header, the toolbar band,
and the background of the two screens with no band (`/start`, `/settings`): only the element
carrying the attribute drags, so controls inside them keep working. The sidebar collapses to
the shadcn icon rail (trigger in its header, edge strip and Cmd+B to expand): its later slots
(tag list, rating pills, Trash/Import/Rules) earn its width, but until they land the owner
wants it out of the way. Whole-app zoom is the webview's `setZoom` on Cmd/Ctrl `=` `-` `0`,
session-only, needing `core:webview:allow-set-webview-zoom`.

**D14. One keyboard map, one guard.**

The bindings live where they act (grid keys in the grid, viewer keys in the lightbox), because
a central dispatcher would need to know which region is live and would re-implement focus. The
one thing that must not be duplicated is "is the user typing" — `lib/keyboard.ts` holds
`isTypingTarget(event)` and the key names, and both regions call it. Grid movement uses the
lifted `offsetIndexClamped` (which has been in `navigation-math.ts` unused since Phase 1);
lightbox movement keeps `offsetIndexBounded`. The two edge behaviours differ on purpose and
that module is where the difference is written down.

**D15. The `tagcount:` parse reads one string.**

`tag-utils.ts:51` matches against `remainingQuery` and then runs `exec` against `query`. Today
the two are the same string at that point, so the defect is latent — which is why there is no
test that fails before the fix and why it must be closed by construction rather than by a
failing case. The fix removes the second read entirely: one `exec` on `remainingQuery`
produces both the "did it match" answer and the captures, and the redundant `.match()` goes.
The regression tests lock the behaviour the restructure must preserve — the full operator table
(`=`, `>`, `<`, `>=`, `<=`, range, reversed range, list, case-insensitive) and the fact that
the metatag is stripped from the tag terms — plus one that pins the coupling directly: a query
whose tagcount metatag survives an earlier stripping step must still parse. Marked as the fix
for the FIXME rather than left for a later reader to rediscover.

**D16. No requirement says "the settings screen contains X, Y, Z".**

Each field on `/settings` is specified by the capability that owns the fact: the listener port
and state by `capture-ingest`, the per-source counts by `library-browse`, the library path,
reveal and close by `library-switching`, the theme by `app-frame`. A single "settings screen"
requirement listing them all would be a second place every later change has to edit, and the
two lists would drift. The screen itself is a route in D2 and a set of slots in the slot map.

**D17. The renamed `library-folder` requirement is a REMOVED plus an ADDED, not a MODIFIED.**

"First launch asks for a library folder" changes both its name and its body: it now governs
three entrances, not one. openspec's RENAMED operation is name-only, and a MODIFIED keeping the
old header would leave a title that contradicts its own text. The REMOVED block carries the
reason and states that both original scenarios are carried over verbatim, so nothing is lost at
archive time.

## Slot map

The contract for every later change. A change fills its slots; it does not add regions.

| Slot | What lives there after this change | Filled later by |
| --- | --- | --- |
| Sidebar · nav | `Library` (`/`), `Settings` (`/settings`) | `Trash` item with a count (**trash**); `Import` item for the bundle report (**legacy-bundle-import**); `Rules` item (**auto-tag-rules**) |
| Sidebar · filters | *absent* — an empty panel is a control that does nothing | tag list of the current result set with counts and include/exclude (**tags-and-ratings** `tag-sidebar`); rating pills with counts (**tags-and-ratings** `rating`); notes panel (**auto-tag-rules** `notes`) |
| Sidebar · footer | library name, total image count, switch menu (recent libraries, Choose folder…, Reveal in Finder, Close library) | — |
| Toolbar · search | tag input and free-text input (Phase 1 D14) | tag autocomplete popover (**tags-and-ratings** `tag-editing`) |
| Toolbar · view | thumbnail size slider | sort select, group select (**tags-and-ratings** `sort-and-group`) |
| Toolbar · actions | `Import` menu (files, folder) | selection toolbar replacing this row while a selection exists: count, select all/none, bulk tags, bulk rating, export selected (**selection-and-bulk**); bulk move-to-trash and restore (**trash**) |
| Grid · tile | thumbnail, hover overlay (title, source, capture date), missing-file state | rating badge (**tags-and-ratings**); selection checkbox and selected ring (**selection-and-bulk**); "posted to `<site>` #id" badge (**booru-upload** `posted-label`); context menu (**tags-and-ratings** remove tag / set rating, **trash** move to trash) |
| Inspector · identity | title, source and source ref, page URL, image URL, dimensions, size, type, captured and imported times, id | — |
| Inspector · tags | read-only tag list | tag editor with autocomplete; tags clickable to add or exclude from the search (**tags-and-ratings** `tag-editing`) |
| Inspector · rating | read-only rating value | rating control (**tags-and-ratings** `rating`) |
| Inspector · actions | *absent* | "Upload to `<site>`" and the posted label (**booru-upload**); "Move to trash" / "Restore" (**trash**) |
| Inspector · header | the one image's title | thumbnails and count for a multi-selection (**selection-and-bulk** `selection`) |
| Lightbox · inspect panel | the Inspector, unchanged | inherits every Inspector slot above, by construction |
| Settings · Library | path, Reveal in Finder, Switch library, Close library, per-source counts | — |
| Settings · Capture | listener state and reason, port field | extension "connected" indicator help (**bridge-extension**) |
| Settings · Appearance | theme (System / Light / Dark), default thumbnail size | — |
| Settings · Keyboard | the keyboard map, read-only (`KEYBOARD_MAP` in `lib/keyboard.ts`) | new bindings from any change |
| Settings · Rules | *absent* | auto-tag rules table, JSON import/export, "Run rules on existing images" (**auto-tag-rules**); booru sites and credentials (**booru-upload** `booru-sites`) |

## Keyboard map

Nothing in this table fires while the focus is in a text field (D14).

| Where | Key | Action |
| --- | --- | --- |
| Anywhere in the frame | `/` | focus the tag search field |
| Anywhere | `Cmd/Ctrl B` | collapse or expand the sidebar |
| Anywhere | `Cmd/Ctrl =` `-` `0` | zoom the whole app in, out, back to 100% |
| Search field | `Esc` | leave the field; grid shortcuts are live again |
| Grid | `←` `→` | focus the previous / next card (clamped at the edges) |
| Grid | `↑` `↓` | focus one row up / down (clamped) |
| Grid | `Home` `End` | focus the first / last card |
| Grid | `Enter` `Space` | open the focused image in the lightbox |
| Grid | `i` | show or hide the inspector column |
| Lightbox | `←` `→` | previous / next image in the result order (bounded: nothing at the ends) |
| Lightbox | `i` | toggle inspect mode |
| Lightbox | `Esc` `Space` | close, focusing the card of the image it showed last (D10) |

Grid movement is clamped and lightbox movement is bounded on purpose; `navigation-math.ts`
holds both and says why.

## Risks / Trade-offs

- [The traffic lights land on a sidebar control and it cannot be clicked] → the sidebar header
  reserves the top-left 80×28 and holds only the app name; verified on macOS in the screenshot
  pass, both themes.
- [Rebinding the listener leaves the old port bound, or the new task racing the old] → the
  shutdown signal is awaited before the new bind, and the rebind test asserts the old port
  refuses connections after the change.
- [Switching libraries while an import or a capture is running] → both take the library mutex
  per file, so a switch waits for the file in flight; the run that was under way reports against
  the library it started in. Import is disabled while no library is open, which is the state a
  switch passes through only if it fails.
- [Making a single click focus rather than open surprises anyone used to the Phase 1 grid] →
  the hover overlay and the inspector make the focused card obvious, and double click still
  opens; recorded here because it is a deliberate behaviour change to a shipped screen.
- [The asset scope grows by two directories per library opened in a session] → scopes are
  additive and per-process; a long session with many libraries grants a handful of patterns,
  and none of them reach `library.sqlite` or `inbox/` (Phase 1 D7a).
- [New shadcn copy-ins pull `bits-ui` in and enlarge the bundle] → the components are the ones
  the frame actually uses; the app is a desktop bundle, not a page load.

## Open Questions

- Whether the recent-list cap of 10 is right. It changes one constant and no spec, task or
  screen; revisit when someone hits it.
- Whether the thumbnail size slider should offer discrete steps rather than a continuous range.
  Both write the same setting through the same command; decide against a running grid.
