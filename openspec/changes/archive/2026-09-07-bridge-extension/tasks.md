## 1. Shared contract (`packages/shared`)

- [x] 1.1 Add `adapter: SiteAdapterRecord | null` to `ImageRecord` in `packages/shared/src/index.ts`, and document on `SiteAdapterRecord` that `fields` values are `string | string[]` and are stored verbatim (design D8, D11); verify `mise run typecheck` passes and `packages/app` still compiles against it.

## 2. The app keeps the adapter record (`packages/app/src-tauri`)

- [x] 2.1 Schema v2 in `db.rs`: append `ALTER TABLE images ADD COLUMN adapter_json TEXT` to `MIGRATIONS` (design D11); verify a unit test that a fresh database opens at `user_version = 2` with the column, and a second that a database left at v1 migrates to v2 keeping its rows.
- [x] 2.2 Mirror the column through the write path: `ImageRecord.adapter` in `model.rs`, `IngestInput.adapter`, `IMAGE_COLUMNS`, the INSERT and `row_to_record` in `ingest.rs` (serialise the record to JSON on write, parse on read; a row whose JSON is unparseable reads as `None` rather than failing the load); verify an ingest test that stores a record with fields and reads them back, and one that a row with NULL reads back as `None`.
- [x] 2.3 Pass the whole record through in `http/captures.rs`, deleting the FIXME at line 137 and keeping `source_ref` as the site; verify the existing capture tests still pass plus a new one asserting `body["adapter"]["fields"]["handle"]` survives a `POST /captures` round trip, and one asserting a capture with unknown field names is stored (spec `capture-ingest`, scenarios "Capture with an adapter record" and "Adapter fields the app does not know").

## 3. Extension shell and the ported capture path (`packages/extension`)

- [x] 3.1 Manifest and package plumbing: add `unlimitedStorage`, `icons` (16/48/128) and `action.default_popup`, add the icon assets, add `jsdom` and `fake-indexeddb` as devDependencies; verify `mise run build` emits `packages/extension/dist` and Chrome loads it unpacked with no manifest warnings.
- [x] 3.2 Port `findImageElement` and `captureImageAsBlob` from the legacy `src/content/index.ts` into `src/content/capture.ts`, returning a data URL instead of a `Blob` (design D13, departure 1); verify jsdom tests for the three matching rules (exact, normalised, query-stripped) and for "no matching image" returning an error rather than raising.
- [x] 3.3 Port the menu handler's control flow from the legacy `src/background/index.ts` lines 51–134 into `src/background/capture.ts`: tab first, then a dynamic `modifyHeaders` `Referer` rule for the image host on `xmlhttprequest`, fetch, rule removed in a `finally`; draw the rule id from a counter (design D13, departure 2); verify node tests with a fake `chrome` that the tab path is used when the tab answers, that the fetch path is used when it does not, that the rule is added before the fetch and removed after it including on failure, and that two captures started in the same second get different rule ids.
- [x] 3.4 Register the context menu as `Save to BooruBox` with its own id on `chrome.runtime.onInstalled`, and title every notification `BooruBox` (design D12); verify manually side by side with the legacy extension that the two entries read differently.

## 4. Site adapters with fixtures (`packages/extension`)

- [x] 4.1 Adapter registry in `src/adapters/index.ts`: pick an adapter by hostname, run it inside a catch that turns any raise into "no record", and drop empty fields (design D8, spec `site-adapters`); verify tests that an unknown host yields no record and that a raising adapter yields no record instead of propagating.
- [x] 4.2 X/Twitter adapter emitting `handle`, `postUrl`, `postText`, `originalUrl`, with dated fixture snippets under `src/adapters/__fixtures__/`; verify jsdom tests for a post with text, a post without text (field omitted, others present), and a fixture with the author element removed (no raise, remaining fields sent).
- [x] 4.3 Pixiv adapter emitting `artist`, `workId`, `title`, `originalUrl`, with dated fixture snippets; verify jsdom tests for a single-image work, a multi-image work where the captured image is the second (its own original URL, not the first's), and a fixture with the title element removed.
- [x] 4.4 Add an `EXTRACT_CONTEXT` message to the content script and ask for the record from both capture paths (design D9); verify a background test that the record reaches the `meta` part on the tab path and on the fetch path, and that a tab which answers nothing still delivers the capture without a record.

## 5. Delivery, history and popup (`packages/extension`)

- [x] 5.1 Port setting in `src/settings.ts` reading `chrome.storage.local` with `DEFAULT_PORT` from `@boorubox/shared` as the default, and the endpoint module that builds `http://127.0.0.1:<port>/captures` and `/status` (design D6); verify tests for the default when unset, a stored override, and a rejected out-of-range value.
- [x] 5.2 Delivery client posting `file` + `meta` as `FormData` with no hand-set `Content-Type` and a 30 s timeout, plus the `GET /status` probe (design D14); verify tests with a stubbed fetch mapping 201 and 200 to delivered, 4xx/5xx to failed with the app's reason, a network error to failed, an abort to failed, and `/status` 200 / 503 / unreachable to the three indicator states.
- [x] 5.3 Pure delivery state machine in `src/delivery/state.ts` (`pending` → `delivered` | `failed`, plus the startup sweep that fails every `pending` entry) with no `chrome` access (design D4); verify unit tests for every transition including the sweep and including that `delivered` is terminal.
- [x] 5.4 History store: one `capture:<uuid>` key per entry in `chrome.storage.local`, undelivered bytes in IndexedDB, the 128 px JPEG thumbnail generated at capture with `OffscreenCanvas`, the badge as the failed count, and the delivered-entry trim (design D2, D3, spec `download-history`); verify tests with a fake `chrome.storage` and `fake-indexeddb` for entry round trip, blob deleted on delivered, badge count, Clear removing delivered only, discard removing one entry with its blob, and the trim never dropping a failed entry.
- [x] 5.5 Wire the background: capture → blob written → entry `pending` → POST → status update, one notification on failure titled `BooruBox` with the §5 wording, the startup sweep, and the retry entry point that re-posts the kept bytes under the same id (design D4, D5, D15); verify a node test with fakes that a capture made while the app is unreachable ends `failed` with its blob kept, badge 1 and exactly one notification, and that retrying it against a stubbed 200 ends `delivered` with the blob deleted and no request to the image host.
- [x] 5.6 Popup shell in plain TypeScript and DOM (design D1): connection line polling `GET /status` on open and every 3 s while open, and the port field; verify a jsdom test rendering the three indicator states, and manually that the popup opens with no console error.
- [x] 5.7 Popup history list: rows with thumbnail, page title, image URL, time, size and status; Retry and Retry-all on failed rows, per-entry discard, and Clear history removing delivered only; verify a jsdom test on the render function given a fixed mixed entry list, and that the list re-renders from `storage.onChanged` without reopening the popup.

## 6. Verification

- [x] 6.1 `mise run check` green (lint, typecheck, `pnpm -r test` including the new extension vitest suites, clippy, builds).
- [x] 6.2 Manual end-to-end pass — run by the owner, who reports it passed (2026-09-07). Their first run found the two defects group 7 fixes; this is the re-run against those fixes. The steps:
  - load `packages/extension/dist` unpacked in Chrome; open a running app with a library
  - capture an image from an X post: popup entry `delivered`, image in the app grid, and the stored record's `adapter.fields.handle` present (check via `GET /status` count plus the app's inspector or a `sqlite3` read of `adapter_json`)
  - capture from a Pixiv artwork page: `artist`, `workId`, `title`, `originalUrl` present in `adapter_json`
  - quit the app, capture twice: both entries `failed`, bytes kept, badge reads 2, one notification per capture
  - reopen the app, Retry one entry: it becomes `delivered` and the worker's network log shows a request to `127.0.0.1` only, none to the image host. The app answers 201: the first attempt never reached it, so this delivery is the first one it sees
  - Retry the same entry once more from a hand-made request with the same UUID (`curl` the multipart body): the app answers 200 with the existing record and the library still holds one image for it
  - Clear history: delivered entries gone, the remaining failed entry still listed with its bytes; discard it explicitly
  - install the legacy extension alongside: both menu entries visible and differently labelled
  - change the app's port in settings and match it in the popup: the next capture is delivered

## 7. What the first manual pass found

- [x] 7.1 A capture stored while the library screen is open now reaches it (design D16, spec `capture-ingest`, "A stored capture reaches the open window"). `HttpState` carries an `on_stored` callback, `AppState::http_state` builds one that emits `capture:stored` with the record, `routes/+page.svelte` re-runs its search on it, and `routes/+layout.svelte` re-reads `LibraryStatus` so the sidebar count follows too (the app-shell follow-up "the sidebar footer total does not move when a capture arrives over HTTP"). Verified by rust tests that a created capture is announced once, that a re-post under a stored id is not and that a refused request is not, plus a webview test that the payload reaches the subscriber unwrapped.
- [x] 7.2 The page names itself instead of the tab naming it (design D17, spec `site-adapters`, "The page names itself"). The content script answers `{ record, pageTitle }`; `pageTitle` is the live `document.title` unless the adapter builds one from the fields it read. The X adapter reads `displayName` and names the page `Name (@handle) on X: <first line of the post>`. Verified by adapter tests against the saved fixtures, a registry test that a raising `title` costs the title and not the record, and background tests that the page's title beats the tab's and that a tab with no content script still uses the tab's.
- [x] 7.3 `popup.test.ts` waited a fixed number of turns for the mount's reads and failed under the whole workspace's parallel test run (on the tip commit too, not only with 7.1–7.2 applied); the waits now poll for the state each step is about. Verified by `mise run check` green from a clean tree.
