## Context

- `packages/app/src/routes/settings/+page.svelte` (593 lines) is one scroll column: the
  drag-region scroller `min-h-0 flex-1 overflow-y-auto` (:225) around `mx-auto flex max-w-2xl
  flex-col gap-10 p-8` (:226), the `Settings` h1, then nine sections: Library (inline
  :229-402, with the rebuild `ConfirmDialog` at :584-593), Capture (inline :404-440),
  `ArtistsSection`, `RulesSection`, `BooruSection`, `StampsSection` (each its own `<section>`
  and h2, each loading itself on `library.status?.libraryPath`), Appearance (inline :472-522),
  About (inline :530-560), Keyboard (inline :563-576, `KEYBOARD_MAP`). One page-level `error`
  line at :578-580 serves every inline section. No nav, anchors, tabs or deep links.
- The app is a SvelteKit SPA (`+layout.ts` `ssr = false`, adapter-static fallback). Routes:
  `/`, `/import`, `/settings`, `/start`, `/trash`. The root layout's gate sends every route to
  `/start` while no library is open.
- `Sidebar.svelte` builds its nav from `resolve()` typed hrefs (:27-43, the comment at :28-29)
  and marks an entry current by `page.url.pathname === item.href` (:156), so a sub-route of
  `/settings` would light nothing.
- `AppSettings` (`settings.json`, Rust) is written through one Tauri command per field
  (`settings.svelte.ts`: every setter is a command, mirrored in `model.rs` and
  `packages/shared` with a wire test). Nothing in the webview uses `localStorage` today.
- `ui/` has `sidebar` (the app shell, bound to its one `Sidebar.Provider`), `scroll-area`,
  `separator`, `input`; no `tabs`, no `navigation-menu`. Tailwind is v4.3, so container
  queries (`@container`, `@3xl:`) are built in; the app does not use them yet.
- No Svelte component in `packages/app` has a render test: every test is a pure `.ts` module
  or a `.svelte.ts` store (verified 2026-09-28; the `artist-entries` handoff said the same).
- `app-shell` design D16: no requirement lists what the settings screen contains; each field
  is specified by the capability that owns the fact.

## Goals / Non-Goals

**Goals:** one route per settings page, a nav that never scrolls away, a remembered landing
page, the Artists filter; every existing control moved with its store calls unchanged.

**Non-Goals:** a component layer for the moved sections; any Rust or `packages/shared` edit;
a render-test harness for Svelte components (none exists; building one is its own change).

## Decisions

**D1. One SvelteKit route per page under `routes/settings/`, the nav in its `+layout.svelte`.**
`routes/settings/{general,library,artists,rules,stamps,booru,keyboard,about}/+page.svelte`;
`routes/settings/+layout.svelte` draws the scroller, the `Settings` h1 and the nav, and
`{@render children()}` the page. A path per page is what makes a deep link a plain
`goto(resolve('/settings/artists'))` for the later change that needs one, and it is how the
app already addresses a screen. Rejected: tabs (state or a query string, not an address; and
`ui/` has no `tabs` to copy in for it), anchors in the one column (the column stays as long as
it is, which is the problem), and `navigation-menu` (a dropdown menubar, not a list).

**D2. `/settings` redirects in a universal load, `routes/settings/+page.ts`.** `load` throws
`redirect(307, resolve(lastSettingsPage().path))` and the old `+page.svelte` is deleted. With
`ssr = false` the load runs in the webview before anything renders, so nothing flashes, and
SvelteKit follows a load redirect as the navigation's destination: history holds
`/settings/artists`, not `/settings`, so Back never lands on the redirecting address and
bounces forward again. Rejected: an `$effect` calling `goto` in `+page.svelte` — it paints an
empty panel first and pushes a history entry the Back button then loops on.

**D3. The last page is remembered in `localStorage`, not in `AppSettings`.** One key,
`boorubox.settings.lastPage`, holding the page's slug. `AppSettings` has no free slot: a new
field there is a Rust field, a command, a `packages/shared` mirror and a wire test, for a value
whose loss costs one click (the General page). It is view state of this machine, and the
webview's storage is per machine (the app's own data directory) and survives a relaunch — the
hand check proves it on both platforms. This is the app's first `localStorage` use, so the
module that reads it is the only one that touches it: both accessors are wrapped in
`try/catch`, a read that throws or finds a slug not in the page list answers General, a
write that throws is dropped. The settings layout writes on every page shown, from a
`$derived` slug (the project rule on `$effect` reading a field of a reassigned object:
`page.url` is replaced on every navigation, so the effect depends on the derived slug, not on
`page.url`).

**D4. One page list, `$lib/components/settings/settings-pages.ts`.** `SETTINGS_PAGES` is the
ordered array `{ slug, label, path }` with `path` a literal route id (`'/settings/general'`,
…, typed `as const`), so the nav, the redirect and the fallback read one list:
`settingsPageFor(stored: string | null)` answers the named page or General (pure);
`lastSettingsPage()` and `rememberSettingsPage(slug)` wrap the storage per D3. The module
imports nothing from `$app`: callers turn `path` into an href with `resolve(p.path)`, which
keeps the module testable under plain vitest. Order and labels: General, Library, Artists,
Rules, Stamps, Booru, Keyboard, About (owner, 2026-09-28). The module's doc comment carries
the order's reasons that used to live in the section comments (D6).

**D5. "Current" is one helper, used by the app sidebar and the settings nav.**
`isCurrentPath(pathname, href)` in `$lib/components/frame/current-path.ts`: `pathname ===
href || pathname.startsWith(`${href}/`)`. The sidebar's `isActive` calls it, so Settings is
lit on every `/settings/*`; the boundary slash keeps `/settings` from claiming a sibling
`/settingsx`, and `/` never claims another screen because nothing starts with `//`. The
settings nav marks its current link with the same helper and `aria-current="page"`.

**D6. The inline sections move into their route pages, markup and script verbatim.** No
`components/settings/` section components: each piece of inline state belongs to exactly one
page (the port to General, `rebuildResult` and the rebuild confirm to Library,
`checkMessage` to About), so no two pages would share wiring a component could hold, and the
route page is already a component — a wrapper per section would be a second file for
nothing. Split of today's script: General takes `themes`, `theme`, `tileSize`,
`clickZoomCeiling`, `openLastOnLaunch`, `port`, `applying`, `listener`, `applyPort`,
`chooseTheme`, `chooseOpenLastOnLaunch`, `commitTileSize`, `commitClickZoom`; Library takes
`libraryPath`, `canRebuild`, `countRows`, `rebuildConfirmOpen`, `rebuildResult`,
`runRegenerateThumbnails`, `confirmRebuild` and the `ConfirmDialog`; About takes
`checkMessage` and `checkForUpdate`; Keyboard reads `KEYBOARD_MAP`. Each page that sets
`error` keeps its own `error` state and error line (Library shows `error ??
libraryCounts.error`, as the page does today). Every existing comment moves with the code it
describes. The four list sections mount unchanged, one per page; their slot comments in
the old page and `ArtistsSection`'s header argue placement in a column ("above Rules",
"after Booru rather than beside Rules", "beside Capture"), which the nav's order now decides:
those clauses go, and D4's module comment says the order once.

Page headings: a page with one section component shows that component's own h2; General,
Library, Keyboard and About open with an h2 of the page's name in the same class
(`text-sm font-semibold`). On General the theme, the two sliders and the launch switch come
first, then an h3 "Capture" over the listener line and the port form; the launch switch keeps
its markup, losing only the top border it had as the last row of Library.

**D7. Geometry: a container query at 48rem of the content region, the nav sticky beside a
`max-w-2xl` panel.** The layout's scroller keeps today's classes plus `@container`; inside
it a row `mx-auto flex w-full max-w-4xl flex-col @3xl:flex-row`. The nav column: `shrink-0
px-8 pt-8 @3xl:sticky @3xl:top-0 @3xl:w-44 @3xl:self-start @3xl:pr-0 @3xl:pb-8`, the h1
`Settings` at its top, then a `<ul>` of plain `<a>` links (`rounded-md px-2 py-1.5 text-sm
text-muted-foreground hover:bg-accent hover:text-accent-foreground`; current: `bg-accent
text-accent-foreground font-medium`), stacked `flex flex-wrap gap-1` below the breakpoint and
`flex-col` at or above it. The panel: `flex min-w-0 max-w-2xl flex-1 flex-col gap-10 p-8`,
the width and padding of today's column. The breakpoint is a container query, not a viewport
one: the region's width depends on whether the app sidebar is expanded or on its icon rail,
and a viewport breakpoint would stack by the wrong measure. `@3xl` (48rem) is where an 11rem
nav leaves the panel about 32rem after padding — the narrowest a rule entry still reads on one
line of controls (`rules-panel-layout` D1's wrap covers anything tighter). Not `ui/sidebar`:
it binds to the app shell's one provider and is the thing the nav must not nest in; not a
new shadcn component either — a styled list of links is all this needs.

**D8. The Artists filter is a pure predicate and a local field.** `filterArtists(entries,
query)` in `$lib/components/artists/artist-filter.ts`: the query trimmed and lower-cased,
empty answers `entries` unchanged, otherwise the entries whose `tag` or any of whose `urls`
contains it, lower-cased. `ArtistsSection` holds `let query = $state('')`, derives the shown
list, and draws an `Input` (`type="search"`, `placeholder="Filter by tag or URL"`,
`aria-label="Filter artists"`) between the Add button/form and the list, only while there is
at least one entry. Zero matches with entries present shows `No artist matches “{query}”.` in
the empty-state line's classes. The field is not remembered and not cleared by a save: an
entry added that the filter hides is the user's own filter at work. The URLs matched are the
stored, normalised form the list shows (`x.com/metaljelly0811`, no scheme): what is matched is
what is on screen.

**D9. Spec placement keeps `app-shell` D16.** `app-frame` gains one requirement for the
structure — pages, order, the nav, the landing page, the address per page, the sidebar's
current entry — and names what General is for, but lists no fields. The owning capabilities
change only where a sentence or scenario said "opens Settings" and would now land on the
wrong page: `app-frame`'s keyboard map (Keyboard), `app-update` (About), `library-folder`'s
launch switch (General), `stamps` ("beside the rules" → the Stamps page). `capture-ingest`
("shown in settings"), `library-browse` ("on the settings screen"), `booru-sites`
("reachable from the settings screen"), `auto-tag-rules` and `library-folder`'s thumbnails
requirement stay true as written and are not touched.

**D10. Two archived placement decisions are amended, not silently overridden.** `auto-tag-rules`
D11 ("a section on `/settings`, not a nav item and not a route") keeps its argument against a
sidebar nav item; its "not a route" is reversed by this change — a settings page is a route
under `/settings`, not a door in the app's nav. `stamps` D5's "on `/settings` after the Booru
section" is reversed by the owner's order (Stamps before Booru, 2026-09-28). Each gets a dated
paragraph saying why the old reading held for a single column and what replaced it.

## Risks / Trade-offs

- [`localStorage` in a Tauri webview not persisting on one platform] → the fallback is General,
  never an error; the relaunch hand check is run on macOS, and on Windows when the owner next
  builds there.
- [The rebuild report is page-local and now lost by moving to another settings page, where
  before it was lost only by leaving Settings] → accepted: the report is shown while the user
  watches the rebuild on the Library page, and the kept-aside name is also in the report the
  start screen shows on the next damaged open.
- [Typed `resolve()` over a union of literal paths may not typecheck] → fallback for the unit:
  a `Record<slug, ReturnType<typeof resolve>>` built with one literal `resolve()` call per page
  in the layout, the list's order still read from `SETTINGS_PAGES`.
- [A filter typed as a full URL (`https://x.com/…`) matches nothing, since the list shows no
  scheme] → accepted per D8; the placeholder says "URL", and the listed form is what the user
  copies from.
- [`artist-workflow` (open, planned after this change: it links to `/settings/artists`) also
  MODIFIES `artist-entries` "Entries are listed and edited in settings"; the second to archive
  replaces the first's text] → this change archives first; `artist-workflow`'s delta is then
  re-copied from the main spec so it carries the filter paragraph and scenarios.
- [Clicking Settings in the app sidebar while already on a settings page pushes the same URL
  again, since the `/settings` redirect resolves to the current page, so one Back press stays
  put] → accepted: the link stays live.
