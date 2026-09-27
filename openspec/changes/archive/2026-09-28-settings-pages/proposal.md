## Why

Settings is one scroll column of nine sections (`routes/settings/+page.svelte`, 593 lines), and
the list sections grow with the library: the owner keeps many artist entries, and a long
Artists list pushes Rules, Stamps, Booru, Appearance, About and Keyboard down past it (owner,
2026-09-28). Settings becomes pages with a nav panel, so each list has a page of its own and
nothing is below it. Requirements §6 (the app's UI is a redesign, not a port of the legacy
viewer's options page).

## What Changes

- **Settings is a set of pages** with a nav list at the left of the settings screen: General,
  Library, Artists, Rules, Stamps, Booru, Keyboard, About, in that order. Each is its own
  address (`/settings/<page>`), reachable by a link from anywhere in the app.
- **General** holds the set-once settings: the theme, the default thumbnail size and the click
  zoom (today's Appearance), "Open the last library at launch" (moved from Library), and the
  capture listener's status and port under a "Capture" heading (today's Capture section).
  "Appearance" and "Capture" stop being section names at the top level: a port is set once,
  and a page holding one field is not worth a nav entry (owner, 2026-09-28).
- **Library** holds everything else of today's Library section: the path and the library
  menu, the per-source counts, the trash count, Regenerate thumbnails, Rebuild library index.
- **Artists, Rules, Stamps, Booru** are today's sections, one per page, unchanged — except
  that Artists gains a filter field above its list, matching the tag and the URLs, ignoring
  case.
- **Keyboard** and **About** are today's sections on pages of their own.
- **Opening Settings lands on the last settings page visited** on this machine, or General the
  first time.
- The sidebar's Settings entry stays lit on every settings page.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `app-frame`: the settings screen is pages with a nav (a new requirement); the keyboard map
  is listed on the Keyboard page.
- `app-update`: the running version and the explicit check are on the About page.
- `library-folder`: "Open the last library at launch" is on the General page.
- `stamps`: stamps are managed on the Stamps page, not "beside the rules".
- `artist-entries`: the list has a filter field.

## Non-goals

- A search across every setting, or keyboard navigation between pages beyond what links give.
- Any change to what a section does: every control keeps its store call, its wording and its
  refusals. Rules, Stamps and Booru are moved, not touched.
- Moving a per-machine preference into the library or back: the remembered page is view state
  of this machine, like the sidebar's collapsed state.
- Filters on Rules, Stamps or Booru: the owner asked for Artists only; the others are short.
- Hash or query deep links (`/settings#artists`): a page is a path, nothing else addresses one.
