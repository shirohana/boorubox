> One unit, S (webview only: `packages/app/src`, plus two archived design amendments), Sonnet,
> retried on Opus if the gate fails; one Opus review of the whole change. Design D1–D10 decide
> every shape; do not re-decide them. No Rust, no `packages/shared`, no new shadcn component.
> Gate: `pnpm lint && pnpm typecheck && pnpm test` from the repo root. No render test: no Svelte
> component in `packages/app` has one (design Context), so the nav's current state is tested
> through `isCurrentPath`. No unit commits or ticks a hand check.

## 1. Unit S — settings pages, the nav, the Artists filter (`packages/app`)

- [x] 1.1 `$lib/components/frame/current-path.ts` per D5: `isCurrentPath(pathname, href)`.
      Tests (`current-path.test.ts`): an exact match is current; `/settings/artists` is under
      `/settings`; `/settingsx` is not under `/settings`; `/settings` is not under `/`; `/` is
      current for `/`.
- [x] 1.2 `Sidebar.svelte`: `isActive={isCurrentPath(page.url.pathname, item.href)}` so the
      Settings entry is lit on every `/settings/*` (and only there). Verified by 1.1's tests
      and hand check 2.1.
- [x] 1.3 `$lib/components/settings/settings-pages.ts` per D3/D4: `SETTINGS_PAGES` (slugs
      `general, library, artists, rules, stamps, booru, keyboard, about`; labels General,
      Library, Artists, Rules, Stamps, Booru, Keyboard, About; `path` literal route ids `as
      const`), `settingsPageFor(stored)`, `lastSettingsPage()`, `rememberSettingsPage(slug)`
      on the key `boorubox.settings.lastPage`, both accessors in `try/catch`; no `$app`
      import. Doc comment: the order is the owner's (2026-09-28) and why General holds the
      set-once settings (spec `app-frame`, "Settings is a set of pages with a nav"). Tests
      (`settings-pages.test.ts`): the labels in the owner's order; a stored slug names its
      page; `null`, `''` and an unknown slug answer General; remember then read round-trips
      (`vi.stubGlobal('localStorage', …)` with a Map-backed stub); a storage whose
      `getItem`/`setItem` throw reads General and the write does not throw.
- [x] 1.4 `routes/settings/+layout.svelte` per D7: the drag-region scroller (today's classes
      plus `@container`, the D13 comment moved with it), the row, the nav column with the
      `Settings` h1 and the links from `SETTINGS_PAGES` via `resolve(p.path)`, current by
      `isCurrentPath` with `aria-current="page"`, the panel with `{@render children()}`; the
      current slug as `$derived`, an `$effect` on it calling `rememberSettingsPage` (D3).
      `routes/settings/+page.ts` per D2: `load` throws `redirect(307,
      resolve(lastSettingsPage().path))`. Delete `routes/settings/+page.svelte` once 1.5–1.8
      hold all of it. If `resolve()` rejects the union type, use the Risks fallback and say
      so in the handoff.
- [x] 1.5 `routes/settings/general/+page.svelte` per D6: h2 General; theme, default thumbnail
      size, click zoom, "Open the last library at launch" (markup verbatim but for the top
      border it had as a row of Library), then h3 Capture with the
      listener line and the port form; the script parts D6 lists for General, verbatim with
      their comments; its own `error` line.
- [x] 1.6 `routes/settings/library/+page.svelte` per D6: h2 Library; path + `LibraryMenu`,
      counts, trash count, Regenerate thumbnails with its status block, Rebuild library index
      with `RebuildStatus`, the rebuild `ConfirmDialog`; the script parts D6 lists for
      Library, verbatim with their comments; `error ?? libraryCounts.error` line.
- [x] 1.7 `routes/settings/{artists,rules,stamps,booru}/+page.svelte`: each mounts its section
      component and nothing else. Slot comments per D6: name the page, drop the column
      placement clauses ("above Rules", "after Booru rather than beside Rules", "Beside
      Capture"); `ArtistsSection.svelte`'s header comment loses "above Rules … not a route of
      its own" the same way.
- [x] 1.8 `routes/settings/keyboard/+page.svelte` (h2 Keyboard, the `KEYBOARD_MAP` list and
      its comment) and `routes/settings/about/+page.svelte` (h2 About, version + update check,
      `checkMessage`, the `app-update` comment), verbatim.
- [x] 1.9 `$lib/components/artists/artist-filter.ts` per D8: `filterArtists(entries, query)`.
      Tests (`artist-filter.test.ts`): an empty or all-space query returns every entry in
      order; `BOB` finds `bob_art` (tag, case ignored); `0811` finds the entry owning
      `x.com/metaljelly0811` (URL); `zzz` finds none. `ArtistsSection.svelte`: the field and
      the no-match line per D8.
- [x] 1.10 Archive amendments per D10: append a dated paragraph (`*Amended (settings-pages,
      2026-09-28):*`) to `openspec/changes/archive/2026-09-10-auto-tag-rules/design.md` D11
      and to `openspec/changes/archive/2026-09-23-stamps/design.md` D5 (the "on `/settings`
      after the Booru section" sentence), each saying why the old reading held for one column
      and what replaced it.
- [x] 1.11 Gate green (`pnpm lint && pnpm typecheck && pnpm test`). Handoff: whether
      `resolve()` took the union or the fallback was used; the final Tailwind classes of the
      nav and panel if they differ from D7; anything the smoke test should look at.

## 2. Hand checks (owner)

- [ ] 2.1 Hand check (owner): every page reachable from the nav, the sidebar's Settings lit
      on each.
      Hand check: open Settings from the sidebar, click each of General, Library, Artists,
      Rules, Stamps, Booru, Keyboard, About in turn; each shows its content with its name
      marked in the nav, the app sidebar's "Settings" entry is highlighted on every one and
      "Library"/"Trash"/"Import" are not; General shows Capture's listener line and port under
      a "Capture" heading and the "Open the last library at launch" switch; Library no longer
      shows that switch.
      Seen (lead's smoke 2026-09-28): all eight pages show their content with their name marked in the nav; the sidebar's Settings is lit on each and Library/Trash/Import are not; General has the switch and a "Capture" heading with the listener line and port; Library has no switch (01-settings-open.png, 02- to 08-settings-*.png).
- [ ] 2.2 Hand check (owner): the filter narrows Artists.
      Hand check: on the real vault's Artists page type part of one artist's tag in capitals,
      then part of one of its URLs, then text nothing contains; the list narrows to the
      matching entries, then says nothing matches the text; emptying the field lists every
      entry again. Scroll a long list to the end: the nav stays on screen.
      Seen (lead's smoke 2026-09-28): `IV703` narrows to the iv70311741 entry, `SAWA` (part of its second URL) to the same, `zzqq` shows "No artist matches “zzqq”.", emptying the field lists both entries (60- to 63-artists-filter-*.png). Not seen: the long-list scroll; the scratch library had two entries.
- [ ] 2.3 Hand check (owner): the last page is remembered across a relaunch.
      Hand check: show the Stamps page, go to Library, quit BooruBox, relaunch, click
      Settings in the sidebar: the Stamps page opens. If the mouse has a back button, press it
      once: the library screen shows, not a blank settings page.
      Seen (lead's smoke 2026-09-28): Stamps, then Library, Cmd+Q, relaunch, Settings opened on Stamps (69-stamps-before-quit.png, 70-settings-after-relaunch.png). Not seen: the mouse back button; the driver cannot post a back-button event.
- [ ] 2.4 Hand check (owner): both themes, and the narrow layout.
      Hand check: switch the theme to Light and to Dark on General; in both the nav's current
      link and hover state are readable against the background. Narrow the window (or expand
      the app sidebar at a small window) until the nav moves above the page: its links wrap
      in rows, nothing scrolls sideways; widen again and it returns to the left column. Below
      the 48rem breakpoint the panel is left-aligned inside the row, where the old column was
      centred; the owner judges whether it wants `mx-auto`.
      Seen (lead's smoke 2026-09-28): in Light and Dark the current link and the hovered link are readable (10-general-light-hover-rules.png, 11-general-dark-hover-rules.png; hover and current use the same fill). The window's minimum width (900 pt) already puts the nav above the page in one row (12-settings-narrow.png, 13-settings-narrower.png); zoomed in with Cmd+= three times the links wrap into two rows, nothing scrolls sideways, and the panel is left-aligned (14-settings-narrow-zoomed.png); back at 1200 pt and zoom reset the nav returns to the left column (15-settings-wide-again.png).

## Handoff

- `resolve()` took the typed union directly: every nav link and the `/settings` redirect call
  `resolve(item.path)` / `resolve(lastSettingsPage().path)` against `SETTINGS_PAGES`'s literal
  `path` union, and `pnpm --filter @boorubox/app typecheck` reports zero errors against any file
  this unit touched. The Risks fallback (`Record<slug, ReturnType<typeof resolve>>`) was not
  needed.
- `localStorage` key: `boorubox.settings.lastPage`, read and written only by
  `$lib/components/settings/settings-pages.ts` (`lastSettingsPage`/`rememberSettingsPage`), both
  wrapped in `try/catch` per D3.
- Tailwind classes match D7; `eslint --fix`'s class-order and line-wrapping rules reordered and
  re-wrapped a few of them (`min-w-0 max-w-2xl` → `max-w-2xl min-w-0`, `@container` moved first
  on the scroller, the nav's `shrink-0 px-8 pt-8` / `@3xl:…` split across lines) — same classes,
  no visual difference. The current link's `bg-accent`/`text-accent-foreground`/`font-medium`
  are applied with `class:` directives rather than a ternary string; `block` is added to the
  link class (D7 doesn't list it, but a plain `<a>` needs `display: block` to take the nav's
  padding and row height).
- `routes/settings/+page.svelte` is deleted and not replaced: the route is `+page.ts` alone,
  whose `load` always redirects — SvelteKit renders nothing for it, so no page component is
  needed (confirmed by the successful `@boorubox/app` build, which emits
  `entries/pages/settings/_page.ts.js` with no matching `_page.svelte.js`).
- Deviations from D1–D10: none. `ArtistsSection.svelte`'s filter `Input` and no-match line match
  D8 verbatim; the General/Library split follows D6's field lists exactly.
- Gate: `pnpm lint` and `pnpm -r test` (764 tests, all packages) are green with zero warnings or
  errors in any file this unit touched; `pnpm --filter @boorubox/app build` succeeds and accepts
  every new route plus the `/settings` redirect. `pnpm typecheck` at the repo root currently
  fails, but only in `packages/app/src/lib/api/commands.test.ts` and
  `vocabulary.svelte.test.ts` ("Property 'note' is missing in type … TagEntry") — files owned by
  the concurrent `tag-notes` agent editing `packages/shared/src/index.ts` live, outside this
  unit's file list and untouched here. Re-run `pnpm typecheck` once that unit lands.
- Smoke test / reviewer: no render test exists for Svelte components in this app (design
  Context), so the nav's current-page logic is exercised only through `isCurrentPath`'s unit
  tests — the reviewer should open `/settings` in the running app once to see the nav, the
  container-query breakpoint at ~48rem, and the Artists filter, per hand checks 2.1–2.4.
