## Why

Phase 1 proved the machinery and left the UI as one screen: a header of loose panels above a
grid, `/setup` as the only other route, and no place to put anything. Every Phase 2 change —
tag editing, ratings, selection, trash, rules, booru upload — needs somewhere to live, and if
each invents its own placement the app becomes a pile of screens instead of the redesign
§6 asks for ("UI: redesign, not port… use a component library to stop hand-rolling CSS").

So the frame comes first, before the features that fill it: regions, routes, tokens, a
keyboard map, the grid and the lightbox. Feature-level styling lands with each feature, into
slots this change names.

The second reason is the library folder itself. Phase 1 designed for "one open library per
app instance" and made switching impossible: the folder picker lives on a screen that is
unreachable once a library is open. Switching libraries turns out to be a routine act, not an
edge case, so this change reverses that non-goal — still one library open at a time, but
switchable, with the ones seen before offered on a start screen (design D1).

**Depends on:** nothing. Every later change cites this one's slot map instead of inventing
placement.

## What Changes

- **A frame with named regions** (§6): left sidebar (navigation, current library and its
  switch menu), top toolbar (the two search inputs, view controls, import), main content, and
  a right Inspector. The regions persist across routes; later features land in the slots
  design.md maps.
- **One Inspector, two placements**: the panel beside the grid and the panel the lightbox
  slides in are the same component. Read-only in this change (source, page, dimensions, size,
  type, times, tags, rating).
- **Lightbox gets two modes**: gallery (image fitted to the viewport on a dark backdrop) and
  inspect (`i`), plus a keyboard map shared with the grid.
- **The grid is denser**: square aspect-fit tiles with a hover overlay instead of a fixed card
  with a caption strip, and a thumbnail-size slider in the toolbar.
- **Routes**: `/` (library), `/start` (no library open: recent libraries, choose a folder,
  missing-folder state — replaces `/setup`), `/settings` (library, capture listener, theme).
- **Switching libraries** (reverses a Phase 1 non-goal): open a recent library, close the open
  one, reveal the folder in the file manager, forget a recent entry. At most one library is
  open at any moment, which is what §7's "one machine writes at a time" needs from the app.
- **The listener stops being a banner** (Phase 1 D17 is spent): its state, its reason and its
  port move to `/settings`, and the port becomes editable there, rebinding without a restart
  (§5: "configurable and shown in app settings").
- **Per-source counts move off the library header** onto `/settings`, where a migration is
  reconciled deliberately (§2 guarantee 4, §9 step 4); the sidebar keeps the running total.
- **Theme**: system / light / dark, chosen in settings and persisted, applied before the first
  paint. The dark palette already exists in `app.css` and nothing has ever toggled it.
- **Defect fix**: the `tagcount:` branch of the search parser matches one string and reads
  another (`tag-utils.ts:51`), which is latent today and a real bug the moment anything is
  stripped before it. Closed with the parse reading one string, locked by regression tests.

## Capabilities

### New Capabilities

- `app-frame`: the persistent frame — regions, the Inspector as one panel with two placements,
  the keyboard map, the theme, and the rule that no control appears before it does something.
- `library-switching`: one library open at a time, changed without relaunching; the start
  screen, the recent list, closing, and revealing the folder.

### Modified Capabilities

- `library-browse`: the grid becomes aspect-fit tiles with a hover overlay and an adjustable
  tile size; the lightbox fits the viewport, gains inspect mode, and distinguishes focusing a
  card from activating it; per-source counts move to a screen reached without a search.
- `library-folder`: the setup screen becomes the start screen (reached on first launch, after
  a deliberate close, and when the remembered folder is gone); a deliberate close is
  remembered as "no library", not as a missing one.
- `capture-ingest`: the listener port is not only shown in settings but changeable there, and
  a change rebinds the listener without restarting the app.

## Non-goals

- **No feature-level UI.** Tag editing, rating controls, selection, bulk actions, trash,
  rules, notes and upload are named in the slot map and built by their own changes. This
  change ships no control that does nothing.
- **Multi-window and two libraries open at once.** The reversal in design D1 is narrow: one
  open library, switchable. §7's one-writer rule is untouched.
- **Zoom and pan in the lightbox.** Fit-to-viewport only.
- **The `/import` route.** It belongs to `legacy-bundle-import`, which owns the bundle report
  and the §9 notice; the slot map reserves its sidebar item.
- **Sort and group controls.** They are `tags-and-ratings`' (`sort-and-group`); a placeholder
  select is exactly what this change forbids.

## Impact

- `packages/shared`: `Theme`, `AppSettings`, `RecentLibrary`; `model.rs` mirrors them by hand
  (Phase 1 D11), so both change together.
- `packages/app/src-tauri`: `settings.rs` gains `theme`, `gridTileSize` and `recentLibraries`;
  `commands.rs` gains `app_settings`, `set_theme`, `set_grid_tile_size`, `set_listener_port`,
  `close_library`, `recent_libraries`, `forget_recent`, `reveal_library`, and `open_library`
  starts pushing to the recent list; `http/mod.rs` gains a shutdown handle so the listener can
  be rebound. `tauri-plugin-opener` is already a dependency (`reveal_item_in_dir`).
- `packages/app/src`: new `routes/start`, `routes/settings`; `routes/+layout.svelte` becomes
  the frame; `lib/components/frame/` (sidebar, toolbar, library switch menu); `lib/components/
  library/` reshaped (grid, card, lightbox, new Inspector); `lib/keyboard.ts`; `lib/theme`;
  `lib/domain/tag-utils.ts` defect fix. `routes/setup` is removed.
- New shadcn-svelte copy-ins (`sidebar`, `dialog`, `dropdown-menu`, `tooltip`, `slider`,
  `badge`, `scroll-area`, `separator`, `select`, `kbd`), which pull `bits-ui` in as a runtime
  dependency of `packages/app`. They stay excluded from lint and format (CLAUDE.md).
- `tauri.conf.json`: macOS `titleBarStyle: "Overlay"` (design D11); `app.html` title fixed.
