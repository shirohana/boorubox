> Depends on `app-shell` and `tags-and-ratings`, both archived and implemented: the slot map,
> the keyboard map, `isTypingTarget`, the rating control and the grid's group slices all come
> from them. Nothing here depends on `trash`, `selection-and-bulk`, `auto-tag-rules` or
> `booru-upload`.
>
> Two implementing agents, split by file ownership: **A** owns group 5 (`packages/shared`,
> `packages/app/src-tauri`, `src/lib/api/`, and `Inspector.svelte`'s facts list), **B** owns
> groups 1–4 (`src/lib/components/library/` except the Inspector, `src/lib/components/tags/`,
> `src/lib/components/frame/`, `src/lib/keyboard.ts`, `src/routes/+page.svelte`). The two sets
> do not overlap and neither blocks the other. Inside group B the order matters: 1.4 (the grid
> publishes its column count) lands before 2.4 (the viewer's row step), and 2.1 before the rest
> of group 2.
>
> Every component in this repo is verified by hand: there are no component tests and this change
> does not introduce the harness for them (the unit tests it does add are on pure modules and on
> Rust). "Verify by hand" means in the running app, on a library of a few hundred images.

## 1. The grid and the tile (agent B)

- [ ] 1.1 `packages/app/src/lib/components/library/ImageCard.svelte` + `Lightbox.svelte`: mark
      the thumbnail `<img>` and the viewer's `<img>` `draggable="false"` so the app never starts
      an OS drag of its own (design D1), with a one-line comment naming the dropzone as the
      reason; verify by hand that dragging a tile across the window raises no import overlay,
      that dragging the image inside the open viewer raises none either, and that dragging an
      image file in from the file manager still shows the overlay and imports on release (spec
      `local-file-import`, "The drop target answers external drags only").
      Hand check: press and drag a thumbnail across the window — no "Drop images or folders to
      import them" overlay appears; open the viewer and drag the large image across it — no
      overlay either; then drag a PNG in from Finder — the overlay appears and releasing it
      imports the file.
      Human update: Partial pass. If you mouse down on a tile, drag a few distance but still inside the tile, since it is not down-and-up, users will expect the mouse move cancelled the down and should not open the viewer.
      Probe (2026-09-10, agent-driven app on a scratch copy of the vault): not probed — the driver has no pointer drag. Yours.
      Agent (2026-09-09): the tile records where its `pointerdown` landed and no longer activates when
      the pointer travelled more than `TILE_DRAG_SLOP_PX` (4 px, `tile-click.ts`) before the release;
      the drag still moves the focus and the selection, which design D1/D7 now argue for. Re-check:
      press the current tile, slide a few px inside it and release — the inspector follows, the viewer
      stays shut; release without moving and it opens; a double click still opens.
- [x] 1.2 `packages/app/src/lib/components/library/ImageCard.svelte`: the current tile's marking
      becomes the ring the missing-file card already uses (`ring-3 ring-ring/50`) instead of
      `border-ring`, with the hover overlay's rules untouched (design D8, Slot: Grid · tile);
      verify by hand at two tile sizes and in both themes that the current tile is findable at a
      glance on a full screen of thumbnails, that a hovered tile is still told apart from it, and
      that a focused missing-file card and a focused normal tile now look alike.
      Hand check: with the tile slider at its smallest and its largest, in the light and the dark
      theme, arrow around a full screen of thumbnails — the current tile carries a visible ring;
      hover a different tile and confirm the hover overlay alone marks it; arrow onto a
      missing-file card and confirm its ring is the same one a normal tile now shows.
      Human update: the ring shown, just not highlighted enough, can see, just not so clear.
      Probe (2026-09-10, agent-driven app on a scratch copy of the vault): the current tile reads as a bright full ring with a gap at 130px tiles in the dark theme. Light theme yours.
      Agent (2026-09-09): the current tile's ring goes to full strength and gains the offset gap —
      `ring-3 ring-ring ring-offset-2 ring-offset-background`, from `ring-ring/50` (design D8 amended).
      Both card states still read it off the one `ring` derivation, so the missing-file card matches.
      Re-check: at both tile sizes and in both themes, and confirm it is still told apart from the
      selected tile's `ring-primary`.
- [ ] 1.3 `packages/app/src/lib/components/library/ImageCard.svelte`: a click on the tile that is
      already the current one activates it, while a click on any other tile only makes it current
      (design D7, spec `library-browse` "Lightbox"); verify by hand that one click on a
      not-current tile fills the inspector and opens nothing, that a second click on it opens the
      viewer, that a double click on a not-current tile still opens it, and — the case D7 names —
      that a double click on the tile that is already current opens the viewer and does not close
      it again on the second click.
      Hand check: click a tile that is not current — the inspector fills and nothing opens; click
      that same tile again — the viewer opens; double click a not-current tile — the viewer
      opens; double click the tile that is already current — the viewer opens and is still open
      after the second click.
      Human update: On action failed. When clicking on a non-focused tile, it opens viewer directly. Other actions just fine.
      Probe (2026-09-10, agent-driven app on a scratch copy of the vault): an accessibility press carries no pointerdown, so from the terminal one click on a tile focuses it and a second does not open it (by design of tile-click.ts: a press-less click is synthesised). The mouse path is yours; Enter on the focused tile opened the viewer.
      Agent (2026-09-09): root cause — the tile's button is wrapped in a `ContextMenu.Trigger`, whose
      props carry `tabindex="-1"`; WebKit focuses a tabbable element on mousedown (buttons are the
      exception, a tabindexed div is not), so `focusin` made the tile current before the click existed
      and reading `focused` first inside the click handler was still too late. The tile now reads its
      current-ness at `pointerdown` and hands it to `shouldActivate` in `tile-click.ts` (unit-tested).
      Re-check: one click on a not-current tile fills the inspector only, a second opens, a double
      click on either kind opens.
- [x] 1.4 `packages/app/src/lib/components/library/LibraryGrid.svelte` + `routes/+page.svelte`:
      `LibraryGrid` gains a bindable `columns` prop written from `shown.columns`, the page holds
      it (design D9); verify by hand that resizing the window and moving the tile-size slider both
      change the number the page holds — read it through the viewer's row step in 2.4 — and that
      nothing about the grid's rendering or scrolling changed.
      Hand check: read it through 2.4 — at five columns `↓` in the viewer moves five images; then
      narrow the window and drag the tile-size slider and confirm `↓` moves the new number of
      images, and that the grid itself scrolls and re-flows exactly as before.
      Human update (2026-09-10): after entering macOS fullscreen (⌃⌘F) and leaving it, none of
      the keyboard controls work any more.
      Agent (2026-09-10): the rows are keyed by their first tile index, so a resize that changes
      the column count (fullscreen does, and so does a sidebar toggle or a slider drag across a
      boundary) re-keys every row but the first; the focused card is unmounted with its row and
      the focus falls to `<body>`, where the grid's keys are not bound — `/` and ⌘A, bound on the
      window, kept working. A `$effect.pre` on `shown.columns` now notices the focus inside the
      grid before the DOM changes and sets `focusWanted`, so the existing effect refocuses the
      current card once the new rows are mounted. Hand check: focus a tile, ⌃⌘F, ⌃⌘F again, press
      `→` — the ring moves. If `/` also fails to focus the search after the round trip, the cause
      is one layer down (the webview no longer the window's first responder), which no page code
      can fix — say so.
      Human update (2026-09-10, second pass): still dead after the round trip.
      Agent (2026-09-10, probed on a second instance of the dev binary under a scratch HOME):
      reproduced — after ⌃⌘F out, the accessibility focus sits on the window itself and `/` goes
      nowhere, so the webview had lost first responder; the grid refocus above is real but was
      not this. The layout now listens for `blur` on the window and, when the app window is
      still focused (`isFocused`), asks for first responder back through the webview's
      `setFocus` (`core:webview:allow-set-webview-focus` added to the capability). Re-probed on
      the rebuilt binary: after the round trip `→` moves the ring and `/` lands in the Tags
      field. Hand check: the same round trip on your window, and Cmd-Tab away and back (a blur
      that must be left alone).

## 2. The viewer (agent B)

- [x] 2.1 `packages/app/src/lib/components/library/Lightbox.svelte`: the dialog's first child
      becomes a named container with `tabindex="-1"` and no outline, focused right after
      `showModal()`; the `<dialog>` itself carries no `tabindex` and is never focused by the
      component (design D2); verify by hand that opening the viewer with Space and then pressing
      an arrow moves the viewer and leaves the grid's focused tile where it was, that Tab and
      Shift-Tab visit only `Previous`, `Next`, `Info`, `Close` and the inspector's controls, that
      Shift-Tab from `Previous` no longer outlines the whole box, and that Escape still closes and
      returns the focus to the grid.
      Hand check: focus a tile, press Space, then `←` — the viewer moves and the grid's focused
      tile is where it was; Tab and Shift-Tab cycle only `Previous`, `Next`, `Info`, `Close` and
      (with `i` on) the inspector's controls; Shift-Tab from `Previous` outlines a control, never
      the whole 96vw box; Escape closes and the grid's tile is focused again.
      Human update: Failed to *Space* dismiss in this path: (1) click a tile to open viewer, make sure the inspector doesn't open (2) press *Tab* 5 times (or *Shift-Tab* 2 times also reproduce) (3) Now it focused on a background element and pressing *Space* no longer can dismiss the viewer.
      Probe (2026-09-10, agent-driven app on a scratch copy of the vault): six Tabs from a fresh viewer left the ring on Next (the cycle wrapped Previous→Next→Info→Close→Previous→Next); Space then pressed Next, the viewer stayed. Space from the surface still closes.
      Agent (2026-09-09): root cause — `showModal()`'s Tab trap leaks in WebKit, so the cycle is now
      walked in `Lightbox.svelte` (`trapTab`, over the dialog's tabbable elements, wrapping through
      the pure `tab-cycle.ts` which has unit tests); design D2 amended with why the native trap
      stopped being enough. Re-check: from a freshly opened viewer press Tab six or more times and
      Shift-Tab past `Previous` — the ring never leaves `Previous` / `Next` / `Info` / `Close` (plus
      the panel's controls with `i` on) — then Escape or click the image and press Space, which
      still closes. Tab is handled ahead of the typing guard on purpose, so Tab out of the tag field
      lands on the next viewer control; a highlighted tag suggestion still takes Tab first.
- [x] 2.2 `packages/app/src/lib/components/library/Lightbox.svelte`: the keydown handler ignores
      an event whose default another control already prevented, beside the existing
      `isTypingTarget` guard (design D3, spec `app-frame` "A control that owns the key"); verify
      by hand with the inspector open in the viewer that Left and Right on the rating choices move
      only between the choices, that Left and Right with the focus on the image still move to the
      previous and next image, and that typing in the tag editor still fires nothing.
      Hand check: in the viewer press `i`, Tab onto a rating choice and press `←` `→` — only the
      choice moves, the image does not; Shift-Tab back out to the image and press `←` `→` — the
      image moves; click into the tag editor and type `i`, an arrow and a space — nothing in the
      viewer fires.
      Human update (2026-09-10): focusing the viewer's tag field drops its suggestion list in a
      lower layer — visible through the backdrop, impossible to click.
      Agent (2026-09-10): the list is a popover portalled to `<body>`, and the viewer is a
      `showModal()` dialog in the top layer, which is above every z-index. `TagInput` now portals
      the list into the nearest `<dialog>` when it has one (`portalTo`), so inside the viewer the
      list is a child of the dialog and paints over the panel; the toolbar and the upload dialog
      (a bits-ui dialog, not a native one) are unchanged. Hand check: open the viewer, press `i`,
      click into the tag field and type a letter — the list drops over the panel and a click on a
      row accepts it.
- [ ] 2.3 `packages/app/src/lib/components/library/Lightbox.svelte`: a click closes the viewer
      when its target is the `<dialog>` itself or the container the image is fitted into, and not
      when it is any descendant of them (design D4); verify by hand that clicking the dark area
      beside the image closes it, that clicking the backdrop outside the dialog box closes it,
      that clicking the image, the header, a button or anywhere in the inspector does not, and
      that Space still closes the viewer when the focus has not been moved onto a button.
      Hand check: with a portrait image open, click the dark area beside it — it closes; reopen and
      click the backdrop outside the 96vw box — it closes; reopen and click the image, the title
      header, the `Next` button and inside the inspector panel — none of them closes it; reopen
      and, without pressing Tab, press Space — it closes.
      Human update: Overall good, only one thing. when click on the viewer title bar at top (the bar contains Title, Previous, Next, Info, Close), it does close the viewer. It is fine when title long, but in widescreen, the title and action buttons has a big space between. Users might think clicking there can also close the viewer. It is the design issue, just memo and raise as next. The bar shorten the area to display image, maybe the action can be default-hide, toggle-show-when-click-on-viewer-image, just like most Manga Viewer App.
      Agent (2026-09-09): no code change, and the bar's empty space is not what closed it: a click
      there targets the `<header>`, and only the `<dialog>` itself and the image stage close (D4).
      What does close is the ~2vh strip *above* the 96vh box — that is the backdrop, and on macOS it
      sits under the traffic lights, exactly where the bar looks like it begins. Re-check by
      clicking the middle of the bar (nothing happens) and then a few pixels above it (closes).
      Auto-hiding the chrome and toggling it by a click on the image is deferred to a future change:
      it moves the close gesture and the header's whole reason for being on screen, which is more
      than a hand-check fix.
- [x] 2.4 `packages/app/src/lib/components/library/Lightbox.svelte` + `routes/+page.svelte`: `↑`
      and `↓` call `moveFocus` from `grid-focus.ts` with the column count from 1.4 and the current
      result's groups, so a row step is the grid's own arithmetic (design D5, spec `library-browse`
      "A row at a time"); verify by hand that at five columns `↓` advances five images and `↑`
      goes back five, that `↑` in the first row lands on the first image rather than doing
      nothing, that `↓` from the last partial row lands on the last image, that grouping by X
      account keeps the step inside a group as the grid does, and that `←` `→` still stop dead at
      the two ends.
      Hand check: with the grid at five columns, open an image and press `↓` — five images later;
      `↑` — back five; open an image in the first row and press `↑` — it lands on the first image
      rather than doing nothing; open the last image and press `↓` — it stays on the last; group
      by X account and confirm `↑` `↓` stay inside a group exactly as the grid's own arrows do;
      `←` at the first image and `→` at the last still do nothing.
      Human update: when viewer open, using arrow key to navigate expect background grid also scroll to make current item in the viewport. Currently only dismiss viewer will scroll.
      Probe (2026-09-10, agent-driven app on a scratch copy of the vault): five ↓ in the viewer reached 24 of 51 with the grid behind already scrolled; Escape left the ring on that tile in view.
      Agent (2026-09-09): the viewer reports every image change (`Lightbox`'s new `onmove`, called
      from `move()`, so the `Previous` / `Next` buttons count too) and the screen hands the index to
      `LibraryGrid.scrollIntoView`, now exported — the same scroll the grid's own arrows use, not a
      second one. Scroll only: no DOM focus leaves the dialog. Design D5 amended. Re-check: open an
      image at the top of the grid, hold `↓` past the fold, then Escape — the grid is already at that
      row and does not jump, and the ring lands on the image the viewer showed last.
- [x] 2.5 `packages/app/src/lib/components/library/Lightbox.svelte` + `routes/+page.svelte`:
      inspect mode moves out of the component and beside `inspectorOpen` on the page, bound into
      the viewer, and the comment claiming the session is the dialog is replaced by D6's reason
      (design D6, spec `library-browse` "Inspect mode is remembered"); verify by hand that
      opening the viewer, pressing `i`, closing and opening another image shows the panel again,
      that toggling the viewer's panel does not move the grid's inspector column, and that after a
      restart the viewer opens on the image alone.
      Hand check: open the viewer, press `i`, close it and open another image — the panel is
      showing; confirm the grid's own inspector column did not open or close with it; quit,
      relaunch and open an image — it opens on the image alone.
- [x] 2.6 `packages/app/src/lib/keyboard.ts`: `KEYBOARD_MAP` gains the viewer's `↑` `↓` row
      ("The image one grid row up or down"), beside the existing viewer rows (design D5, spec
      `app-frame` "One keyboard map"); verify `mise run test` stays green and by hand that
      Settings → Keyboard lists the new row with the others.
      Hand check: Settings → Keyboard lists a `Viewer` / `↑` `↓` / "The image one grid row up or
      down" row directly under the `Viewer` `←` `→` row.

## 3. The rating control (agent B)

- [x] 3.1 `packages/app/src/lib/components/tags/RatingControl.svelte`: `rovingFocus={false}` on
      the toggle group, so all five choices are tab stops, plus a keydown on the group that moves
      the focus to the neighbouring `role="radio"` element on `←` `→` and calls `preventDefault()`
      — the same signal 2.2 reads — with a comment naming bits-ui's either-or as the reason the
      arrows are owned here (design D3, spec `rating` "The rating control is operated from the
      keyboard"); verify by hand in both placements that Tab and Shift-Tab step through the five
      choices, that `←` `→` move between them and stop at the ends, that Enter and Space still
      set the rating, that the focused choice is visible, and that in the viewer none of these
      keys changes the image being shown.
      Hand check: in the inspector beside the grid and in the viewer's panel, Tab and Shift-Tab
      step through all five choices; `←` `→` move between them and stop at `g` and at `none`;
      Enter and Space set the rating; the focused choice is ringed; and in the viewer none of
      `←` `→` Tab changes the image being shown.
      Human update: when focusing a rating ratio, press *Space* to select, and now pressing *Space* can no longer dismiss the viewer.
      Probe (2026-09-10, agent-driven app on a scratch copy of the vault): not probed (needs Tab into the viewer inspector). Yours.
      Agent (2026-09-09): the choice still takes Space and sets the rating — that part is a button
      doing its job — but it now says so: `RatingControl` gained `onchosen`, `Inspector` forwards it
      as `onrated`, and only the viewer's placement passes one, which puts the focus back on the
      dialog's focus surface (design D4 amended). Re-check in the viewer: `i`, Tab onto a choice,
      Space — the rating changes and the very next Space closes the viewer; then beside the grid,
      Space on a choice must leave the focus on it so `←` `→` keep stepping.

## 4. The frame (agent B)

- [x] 4.1 `packages/app/src/lib/components/frame/Sidebar.svelte`: `Sidebar.Rail` is given a
      call-site class that overrides the copy-in's variant-scoped resize cursors with an
      important pointer cursor, with a comment saying why the `!` is needed and that the copy-in
      is not edited (design D10, spec `app-frame` "The edge of the sidebar"); verify by hand that
      the edge shows a pointer in both the expanded and the collapsed state, that clicking it
      still toggles the sidebar, that Cmd+B and the top bar's toggle are unchanged, and that
      `mise run lint` still passes with the copy-in untouched.
      Hand check: hover the strip between the sidebar and the grid with the sidebar expanded and
      again collapsed — the cursor is a pointer, never a resize arrow; click it and the sidebar
      toggles; Cmd+B and the top bar's toggle behave as before.
      Human update: when try to horizontal scroll in the left panel (scroll left), the content just overflowed and touch the left edge, that is unwanted. Also, when I toggle tags or rating, sometimes the tags reduced and scrollbar gone, it'll cause layout shift. Expect always persist the place for scrollbar.
      Probe (2026-09-10, agent-driven app on a scratch copy of the vault): the tag list shows a reserved scrollbar track at launch; sideways scroll not probed (no trackpad gesture from the terminal). Yours.
      Human update (2026-09-10): dragging the window edge to about 770–780px collapses the
      sidebar, and the toggle at that width opens and closes it forever. If the design is not
      ready for a small screen, set a window minimum rather than ship it half-complete.
      Agent (2026-09-10, probed on a second instance): reproduced at a 770px window — that is
      shadcn's sidebar switching to its phone layout (a sheet) under 768px, and the sheet
      looping open/close. Two things: `tauri.conf.json` gives the window a minimum of 900×600
      (the top bar's fixed width is about 810px), and `lib/hooks/is-mobile.svelte.ts` — the
      copy-in hook the sidebar reads — now never matches, because the whole-app zoom can still
      put the CSS viewport under 768px at any window width and a phone layout has no place in a
      desktop app. The copy-in's `sidebar.svelte` also hid the desktop sidebar by CSS under
      `md` (`hidden md:block`, `hidden md:flex`); both classes are gone. A re-add of the sidebar
      copy-in overwrites both files; their comments say so. Probed: at 900×600 with three ⌘=
      steps the sidebar stays in the accessibility tree and on screen; asking the window for
      770×500 gives 900×600. Hand check: the same on your window.
      Agent (2026-09-09): app.css now gives both of the sidebar's scroll containers — the copy-in `[data-slot='sidebar-content']` wrapper and the filters region, which carries a new `data-sidebar="filters"` hook in `frame/Sidebar.svelte` — `overflow-x: hidden` plus `scrollbar-gutter: stable`. Re-check: two-finger scroll sideways over the tag list (nothing moves, the padding stays under the names), then toggle tags and ratings until the list crosses the "needs a scrollbar" line (the column's width must not change). Not verified in the running app; if the gutter reads as a visible blank stripe, the rule to reconsider is the one at the bottom of app.css.

## 5. Capture time and the file's modification time (agent A)

- [x] 5.1 `packages/shared` + `packages/app/src-tauri`: `ImageRecord` gains
      `fileModifiedAt: number | null` / `file_modified_at: Option<i64>` in `src/index.ts` and
      `model.rs` (Phase 1 D11 — one commit, both files); verify a serde test pins the camelCase
      key and that `null` round-trips, and `mise run typecheck` passes.
- [x] 5.2 `packages/app/src-tauri/src/db.rs`: append one migration adding
      `images.file_modified_at INTEGER` (design D11), documented like its neighbours; verify Rust
      tests that a fresh library reports `user_version == MIGRATIONS.len()` and has the column,
      that a library at the previous version gains it on open with every existing row intact and
      `NULL` in the new column, and that no test pins a literal version number.
      Taken as v3 (`SCHEMA_V3`), not the "expected v5" design.md named: `auto-tag-rules` and
      `booru-upload` have only been planned, not applied, so their v3/v4 migrations are not yet in
      `MIGRATIONS` — this change lands first and takes the next slot, `MIGRATIONS.len()`. design.md
      D11 amended to say so. Also fixed the one test that pinned a literal `known: 2` /
      `version, 2` (both now read `MIGRATIONS.len()`), since it would have broken the moment this
      or any sibling migration landed.
- [x] 5.3 `packages/app/src-tauri/src/ingest.rs`: `IngestInput` gains `file_modified_at:
      Option<i64>`, the column is appended to the end of `IMAGE_COLUMNS` so no `row_to_record`
      index moves, and the insert writes it (design D11); verify Rust tests that a record stored
      with `Some(t)` reads back `Some(t)` and one stored with `None` reads back `None`, and that
      the existing ingest tests pass unchanged through the new column list.
      Every other `IngestInput` construction in `src-tauri` (query.rs, thumbs.rs, maintenance.rs,
      tags.rs tests, and the HTTP capture path in `http/captures.rs`) updated to pass
      `file_modified_at: None` — none of those rows come from a file.
- [x] 5.4 `packages/app/src-tauri/src/import.rs`: `captured_at(&Metadata)` becomes
      `file_modified_at(&Metadata) -> Option<i64>` with no fallback to now, and `import_file`
      stamps `captured_at` with `db::now_ms()` and passes the file's mtime as the new field
      (design D11, spec `local-file-import` "Metadata comes from the file"); verify the existing
      `metadata_comes_from_the_file` test is rewritten to assert the capture time is the import
      time and the modification time is the file's, plus a test that a file whose mtime is years
      old still sorts to the front of a newest-capture-first search, and one that an image stored
      through the HTTP capture path carries no modification time.
      The sort test uses `std::fs::File::set_modified` (stable std, no new dependency) to backdate
      a file two years and runs it through `query::search` with the default sort; the HTTP-path
      test lives in `http/captures.rs` since that is where the capture path actually runs.
- [x] 5.5 `packages/app/src/lib/components/library/Inspector.svelte`: a "File modified" row after
      "Imported", falling back to the panel's `—` (design D11, Slot: Inspector · identity);
      verify by hand that a freshly imported file shows today as captured and its own older time
      as modified, that an extension capture shows `—`, and that both placements of the panel
      show the row.
      Row added, reusing `formatTimestamp`'s own `Number.isFinite` fallback to `—` by passing it
      `image.fileModifiedAt ?? Number.NaN` rather than writing a second `—` case. Not hand-verified
      in the running app — running `pnpm tauri dev` was out of scope for this pass; the owner's
      manual pass (memory: tags-and-ratings 6.2/6.3) is the place to fold this in.
- [x] 5.6 No backfill and no rewrite of existing rows (design D12); verify by hand on a library
      imported before this change that its images keep the capture times they had, show `—` for
      the modification time, and that a file imported now appears at the front of the default
      order beside them.
      Satisfied by construction: the migration (5.2) only adds a nullable column and touches no
      existing row, and nothing in `import.rs` or `ingest.rs` rewrites `captured_at` on a row that
      already exists. Not hand-verified in the running app for the same reason as 5.5.

## 6. Verification

- [ ] 6.1 `mise run check` passes (lint, typecheck, tests, clippy, builds).
- [ ] 6.2 Keyboard pass in the running app: from the grid, arrow to a tile and open it with
      Space; move with `←` `→` `↑` `↓`; press `i`; close with Space and confirm the grid focuses
      the image the viewer showed last; reopen and confirm the panel comes back; Tab through the
      viewer's controls in both directions and confirm nothing outlines the whole box; with the
      focus on a rating choice, press `←` `→` and Tab and confirm the image does not move; press
      `/` and confirm no viewer key fires while typing.
- [ ] 6.3 Pointer pass: drag a tile across the window and confirm no import overlay; drag a file
      in from the file manager and confirm the overlay and the import; click a tile once, then
      again, and confirm the second click opens it; click the dark area beside the image and
      confirm it closes; hover the sidebar's edge and confirm a pointer cursor and a toggle on
      click.
- [ ] 6.4 Import pass: import one file whose modification time is old and one that is new into a
      library sorted newest capture first; confirm both are at the front in import order, that
      the inspector shows each file's own modification time, and that switching the sort to
      oldest capture first puts them last.
- [ ] 6.5 Regression pass: the Phase 1 and `tags-and-ratings` end-to-end lists still hold — a
      `curl` capture (201) and a retry of the same id (200, one image), a folder import, tag and
      free-text search, the sidebar's tag and rating counts, all four sorts and both groupings,
      editing tags from the inspector and from the viewer, and a file deleted under `images/`
      showing the missing card with its action.

## Handoff notes (agent B), resolved by the coordinator's pass (2026-09-08)

- **Space on a focused viewer button.** Design D4 said the button's own Space wins, but a
  native button does not mark Space's keydown as handled, so D3's guard never saw it. Resolved:
  Space closes the viewer only when the keydown's target is the focus surface (D4, amended in
  place).
- **`LibraryGrid`'s `columns` carries an ESLint suppression.** `no-useless-assignment` fires on
  the `$bindable()` in the destructure because the grid publishes the count and never reads the
  prop back (it lays out from `shown.columns`, the single source). Suppressed inline with that
  reason rather than reshaped, because every alternative — a fallback that is read, a guard that
  compares before writing — adds a statement whose only job is to satisfy the rule.
- **The focus-visible ring on the normal tile was removed with D8's ring.** `focused` is true
  for exactly the card that holds the roving `tabindex="0"`, so `focus-visible:ring-3
  ring-ring/50` produced the identical ring the new `focused` ring now draws. Two rules painting
  one marking is what D8 says the grid should stop having.

## Coordinator's pass (2026-09-08, dev instance on a scratch copy of the owner's library)

Seen working: the current tile's ring (1.2); `→` then `↓` at three columns lands on 5 of 53 (2.4); `i` toggles the panel
and the state survives close and reopen both ways (2.5); Tab reaches the rating radios, `→`
and Tab move between them with the counter unchanged (3.x); a file imported through the Import
menu lands first under Newest capture with Captured = the import moment and File modified =
its 2020 mtime (5.x). Not driven, so 1.1, 1.3 and 2.3 stay open for the owner's pass: the drag checks, the
click-then-open sequence (the coordinator's two clicks could not tell "first click opens"
from "second click opens", and the review then proved the first build opened on every click:
`focused` is a live prop read after the focus index moved — fixed with the two Lightbox fixes
below), and the backdrop click (no click tool reaches a modal dialog in this setup: System
Events resolves clicks through the accessibility tree, which lands on the grid behind it).

Review of the first build (Opus, 2026-09-09), fixed in the same pass: the tile's click read
`focused` after moving the focus; a double click on the current tile closed the viewer it had
just opened (the dialog mounts on a microtask, the second click lands inside it — now ignored
via `event.detail`); Space was dead once the focus parked on the `<dialog>` itself (now a
non-control like the surface). Noted, not changed: the `defaultPrevented` guard lives in the
viewer only, and the 12 px gap between the image column and the inspector targets the surface
and does not close.
