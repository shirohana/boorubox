## Why

Phase 1 shipped the app and the listener it accepts captures on, but nothing sends
captures: the extension package is a scaffold whose two entry points are `export {}`
(docs/requirements.md §8, row "1b — Bridge extension"). Until it exists the app can only be
fed by local import, the legacy extension stays the only way to save from a page, and the
migration in §9 cannot start.

The app side has one hole of its own: the adapter record the transport already carries is
thrown away on ingest except for `site` (FIXME at `src-tauri/src/http/captures.rs:137`).
Every later change that reads source context — auto-tag rules, artist fields, booru upload —
reads it from the row, so the column has to exist before the extension starts filling it.

Depends on: nothing. Parallel-safe with `app-shell`; the extension is independent of the
app's frame, and the app-side work here is one column and one pass-through.

## What Changes

- **Capture and deliver** (§4.1, §4.3, §5): right-click an image → capture it in the page,
  falling back to a background fetch that sends the page as referrer → POST it to
  `127.0.0.1:47201/captures` with the caller's UUID. A capture is `failed` with its bytes
  kept until the app answers 2xx (§2 guarantee 3, §10 "App not running when saving").
- **Site adapters** (§4.2): X/Twitter and Pixiv adapters produce a plain `{ site, fields }`
  record. Extraction only — the app decides what the fields mean (CLAUDE.md layout rules).
  A rotted adapter never blocks a capture (§10 "Site adapters rot").
- **Download History** (§4.3): one entry per capture with thumbnail, source URL, page title,
  timestamp, size and status; Retry on a failed entry re-posts the kept bytes without
  re-fetching; "Clear history" removes delivered entries only; the toolbar badge counts
  failed entries, not total saves.
- **Connection indicator and port setting** (§5): the popup shows connected / no library /
  not running from `GET /status`, and holds the port field that matches the app's.
- **The app keeps the adapter record** (§4.2, §7): schema v2 adds one column holding the
  record verbatim, exposed on the image record so later changes read it from the row instead
  of re-deriving it. Spends the `captures.rs` FIXME.
- **Distinct context-menu label** "Save to BooruBox" (§3, §10 "Two 'Save image' menus").

## Capabilities

### New Capabilities

- `capture-delivery`: capturing an image from a page and handing it to the running app,
  including the failed/Retry contract and the app's address.
- `download-history`: what the extension keeps about recent captures, and the Clear / Retry
  / discard / badge rules over it.
- `site-adapters`: the per-site extraction of source context into `{ site, fields }`.

### Modified Capabilities

- `capture-ingest`: "Captures are accepted by multipart POST" gains the requirement that the
  adapter record is stored with the image and returned on the record. Today the spec accepts
  the record on the wire and says nothing about its fate, and the implementation drops it.

## Non-goals

- Any policy over the adapter fields. Auto-tag rules, artist extraction, rating and tag
  derivation stay in the app and land in `auto-tag-rules` and `tags-and-ratings` (§4 rule of
  thumb; CLAUDE.md).
- Searching or indexing the adapter record. The column is storage; no FTS change, no filter.
- Showing the record in the app UI. `app-shell`'s Inspector is read-only source context; a
  later change decides whether the adapter fields appear there.
- A library UI, an image store or a viewer in the extension (§4 "Single mode; no library UI;
  no IndexedDB image store" — the only bytes it keeps are undelivered captures).
- Auto-launching the app when a capture arrives while it is closed (§5, "later nicety").
- Browsers other than Chrome (§11).
- Legacy bundle export/import; that is `legacy-bundle-import`.

## Impact

- `packages/extension`: the whole package stops being a scaffold — background worker,
  content script, adapters with fixtures, delivery, history store, popup. New devDependency
  for a DOM test environment; `unlimitedStorage` added to the manifest.
- `packages/shared`: `ImageRecord` gains the adapter record.
- `packages/app/src-tauri`: schema v2 migration, `model.rs` mirror, `ingest.rs` write path,
  `http/captures.rs` pass-through.
- `packages/app/src`: none. The record rides on `ImageRecord` and no component reads it yet.
