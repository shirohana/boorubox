## Context

The app half of the transport is built and tested: `POST /captures` reads a `file` and a
`meta` part (Phase 1 D15), refuses anything else, is idempotent on the caller's UUID, and
answers only once the row exists. `GET /status` answers 200 with version, library path and
image count, or 503 when no library is open, and is exempt from the `Origin` check. What is
missing is the client, plus the one column that keeps the adapter record the client sends.

`packages/extension` is a scaffold: MV3 manifest with `contextMenus`, `storage`,
`activeTab`, `notifications`, `declarativeNetRequestWithHostAccess`, host permissions
`<all_urls>` and `http://127.0.0.1/*`, a `vite-plugin-web-extension` build, a node vitest
config, and two entry points that are `export {}`. No framework, no popup, no icons.

The capture technique is fixed by requirements §4: content-script canvas first, background
fetch with a `Referer` rule as fallback, "port this code, do not rewrite it". The source is
the legacy repo at `~/Repositories/@shirohana/chrome-image-storage`, `src/background/index.ts`
(the menu handler, lines 51–134) and `src/content/index.ts` (whole file).

Motivation: see proposal.md.

## Goals / Non-Goals

**Goals:**

- A capture that the app never acknowledged is still on disk in the browser, with the same
  UUID, until the user says otherwise (§2 guarantee 3). Every other decision below bends to
  this one.
- The extension holds no rule and no vocabulary. Everything it writes into a capture is
  something it read off the page.
- The adapter record lands in the library as one document, so the change that finally reads
  it reads what was extracted, not a normalisation of it.

**Non-Goals:**

- Sharing UI code with the app. The popup and the app window have no component in common.
- Making the extension work with the app closed beyond keeping bytes: no queue that drains
  by itself, no auto-launch (§5).

## Decisions

**D1. The popup is plain TypeScript over the DOM; no framework in the extension.**
It renders a status line, a list of at most a few hundred rows, and four controls. Adding
Svelte means a second component toolchain inside `vite-plugin-web-extension`, a second lint
and format surface, and a build whose failure mode is a popup that does not open. Rejected
alternative: Svelte 5, for idiom with `packages/app`. The idiom does not transfer — every
app component talks to Tauri commands, none of which exist here — so the shared part would be
the framework alone. This decision is spent the day the popup needs shared state across
views; it must not be spent to get nicer markup.

**D2. History entries live in `chrome.storage.local`, one key per capture
(`capture:<uuid>`); undelivered bytes live in IndexedDB; the manifest asks for
`unlimitedStorage`.**
Three separate constraints force this shape:
- `chrome.storage` serialises through JSON, so it cannot hold a `Blob` at all. The bytes
  need IndexedDB.
- One key per capture rather than one array under one key: the background worker is killed
  and restarted at will, and two captures started seconds apart would otherwise read the same
  array and write back over each other, losing an entry — exactly the guarantee this change
  exists to hold. Per-key writes cannot collide, the badge is a scan of a few hundred small
  values, and the popup subscribes to `storage.onChanged` for free.
- `unlimitedStorage`, because both `storage.local` and IndexedDB are otherwise capped at
  10 MB per extension and a handful of undelivered full-size images passes that. Over the cap
  the browser's answer is eviction, which is silent data loss. Rejected alternative: cap the
  kept bytes ourselves and drop the oldest — that is the same loss, chosen by us.
Delivered entries keep no bytes: the IndexedDB record is deleted the moment the app answers
2xx, so the store holds only what is still owed.

**D3. The thumbnail is 128 px on the longest edge, JPEG at quality 0.7, stored as a data URL
inside the entry.**
It is decoration for a 40-px popup row on a 2× display; 128 px covers that with nothing
spare. At roughly 4 KB per entry a full history costs about a megabyte, which is why it can
sit inside the entry and the popup can render from a single storage read. Rejected: 256 px
(four times the bytes for a row that never grows) and a separate IndexedDB thumbnail store
(a second async read per row, to save bytes we have). Unrelated to the app's own thumbnail
size — that is Phase 1's open question and a different display.

**D4. Delivery states are `pending` → `delivered` | `failed`, and the bytes are written
before the first POST is made.**
The order matters more than the states. The worker can be killed between the capture and the
response, and by then the tab may be gone; if the bytes were only in memory the capture is
lost. So: write the blob, write the entry as `pending`, then POST. On 2xx the entry becomes
`delivered` and the blob is deleted; on anything else `failed`, blob kept, one notification.
Any entry still `pending` when the worker starts is moved to `failed` — the worker that owned
it is gone, and there is no way to learn what happened to that request. Marking it failed
risks only a duplicate delivery, which the app's idempotency on the UUID absorbs (§10
"Duplicate delivery on retry"); leaving it pending risks a capture nobody ever retries.

**D5. Retry re-posts the kept bytes under the original UUID and never touches the image
host.** §5 requires it, and it is also the only correct behaviour: the page may be closed,
the URL may be single-use, and the bytes on disk are what the user saw. A retry of a
delivery that actually landed but whose answer was lost gets 200 and the existing record;
the entry becomes `delivered` and the library still holds one image. When the blob is
missing from IndexedDB — cleared site data — the entry offers no Retry and says why; a Retry
button that re-fetched instead would silently save different bytes than the ones captured.

**D6. The port is a `chrome.storage.local` key, defaulting to `DEFAULT_PORT` imported from
`@boorubox/shared`, and edited in the popup. No options page.**
Importing the constant is why `@boorubox/shared` is already a dependency of this package;
re-typing `47201` here would be the second copy of a number §5 fixes in one place. The
manifest's `http://127.0.0.1/*` host permission is port-agnostic, so changing the port needs
no permission prompt and no reinstall. An options page for one number is the same wrong order
Phase 1 D17 refused for the listener banner; the field sits in the popup footer next to the
indicator it explains. When the extension grows a second setting, an options page is the
right answer and this is spent.

**D7. The connected indicator polls `GET /status` only while the popup is open (once on
open, then every 3 s); the background never polls.**
An MV3 worker has no reliable timer — it is killed after about 30 s idle — so a background
poll means a `chrome.alarms` wake-up every minute, forever, to keep a dot green that nobody
is looking at. Rejected for that reason. The one background-visible signal is the badge, and
the badge is a count of stored entries, not a probe.

**D8. The adapter contract is `{ site, fields }` with `fields` a flat map of `string` or
`string[]`, and these field names:**

| site | fields |
| --- | --- |
| `x` | `handle` (no leading `@`), `postUrl` (canonical `…/status/<id>`), `postText`, `originalUrl` |
| `pixiv` | `artist`, `workId`, `title`, `originalUrl` |

`originalUrl` is one name across both adapters because it is one concept — the
highest-resolution URL of the image that was captured — even though the brief describes X's
as the "original media URL" and Pixiv's as the "original URL"; a rule in `auto-tag-rules`
that wants the full-size source should not have to know which site it is on. `handle` and
`artist` stay separate names because they are not the same thing: a handle is an account
identifier that appears in the URL, an artist is a display name that does not. Absent fields
are omitted rather than sent empty, so "the markup changed" and "the post had no text" are
the same shape and both are harmless.

*Kept, on the opposite argument (2026-09-06, while implementing).* That `handle` "appears in
the URL" is a reason **not** to extract it: the app already derives the X account from
`page_url` twice — `getXAccountFromUrl` in `src/lib/domain/grouping.ts` for the webview's
groups and `x_account()` in `query.rs` for the `account:` filter, which carries its own FIXME
about being one rule with two implementations. A stored `handle` would be a third, and the
first one that can drift, because it is written once and never recomputed. It is kept anyway
because it is the *only* source on a page whose URL has no account: `/home`, `/explore` and
`/i/…` are reserved paths where `x_account()` returns NULL, and on a quoted or retweeted post
the URL's account is not the image's author. On the permalink pages the owner actually
captures from, it is redundant; on the pages it is not redundant, nothing else can answer.

The consequence belongs to `tags-and-ratings`, which owns `account:`: that filter compiles to
`x_account(images.page_url)`, so an image captured from a timeline whose record reads
`handle: alice` does **not** match `account:alice`. Whether the filter should prefer the
stored handle over the URL is that change's decision, and this is the note that says it has
to be made rather than discovered.

**D9. Adapters run in the content script and are asked for separately from the bytes.**
The background asks the tab for the adapter record in both capture paths, so a page whose
image cannot be canvas-captured (tainted, cross-origin) still delivers its context. If the
tab answers neither — no content script on that page at all — the capture goes without a
record. Rejected: returning the record together with the captured bytes in one message,
which is fewer round trips but ties the context to the path that failed.

**D10. Fixtures are trimmed HTML snippets of one post / one artwork, committed under the
adapter's folder, each opening with a comment naming the page URL and the date it was
saved; adapter tests opt into a DOM per file rather than switching the package's test
environment.**
The package's other tests (delivery machine, history store, background orchestration) run in
node against fake `chrome` globals and are faster and clearer there, so the vitest
environment stays `node` and the adapter test files carry an environment docblock. The DOM
comes from `jsdom` rather than a lighter parser because the point of a fixture is that it is
the site's real markup — deeply nested, attribute-driven — and a parser that is merely close
would make a green test mean less. The saved date is what makes a rotted adapter diagnosable
later: it dates the last markup we know worked (§10 "Site adapters rot").

*What the 2026-09-06 saves turned out to say.* On X, a `/photo/N` page puts the full-size
image in the modal overlay, outside every `<article>`, while the page beneath holds the post
and twenty of its replies — so the adapter resolves the post from the captured image's media
id, not from "the first article". On Pixiv, `<meta id="meta-preload-data">` is gone from the
pixiv-web-next rewrite, and the `__NEXT_DATA__` that replaced it still describes whichever
page the tab loaded first, so it names the wrong artwork after any click-through; the adapter
reads the DOM, and every class on that page is a styled-components hash, which leaves the
`data-ga4-*` / `gtm-*` attributes and the `main` / `h1` / `h2` landmarks as the only hooks
worth selecting on.

**D11. App side: schema v2 adds `images.adapter_json TEXT NULL`, holding the whole
`{ site, fields }` record as received; `ImageRecord` gains `adapter`.**
One migration, `ALTER TABLE images ADD COLUMN adapter_json TEXT`, appended to the runner's
list; existing rows read NULL. The column is named `_json` because that is what the next
reader needs to know about it: it is a document, not a value to compare with `=`, and the
`auto-tag-rules` change will reach into it with SQLite's JSON functions. The TypeScript and
Rust field is `adapter`, matching `CaptureMeta.adapter`, because that is the same record.
Stored verbatim, not normalised: the moment ingest reshapes the fields, the stored record
becomes a second definition of what the adapter produced, and the extension's tests would no
longer describe what the app holds. `source_ref` keeps holding the site, unchanged, so
per-source counts and the existing column keep their meaning. No FTS change — the adapter
fields are not searchable in this change. This spends the FIXME at `http/captures.rs:137`.

**D12. The context-menu entry is `Save to BooruBox` with its own id, and every notification
the extension raises is titled `BooruBox`.**
The two extensions have different ids, so their menu entries are independent — the only
requirement is that the user can tell them apart while both are installed (§3, §10). The
legacy entry reads "Save to Image Storage"; the notification titles differ too, because a
failure notification is the other place the two are confusable.

**D13. The capture path is ported from the legacy files, with exactly two departures, both
recorded here.**
Ported as-is: `findImageElement` (exact match, then normalised, then query-stripped) and
`captureImageAsBlob` (wait for `complete`, reject on zero dimensions, draw to a canvas) from
`src/content/index.ts`; and the menu handler's control flow from `src/background/index.ts`
lines 51–134 — try the tab first, on any failure add a dynamic `modifyHeaders` rule setting
`Referer` to the page URL for the image host on `xmlhttprequest`, fetch, and remove the rule
in a `finally` that swallows its own errors.

Departure 1 — the captured bytes cross the messaging boundary as a data URL, not as a
`Blob`. Extension messages are JSON-serialised, so the legacy `sendResponse({ blob })`
delivers `{}`; the background then hands `{}` to the image decoder, that throws, and the
`catch` runs the fetch fallback. The consequence is that the legacy extension's canvas path
never actually ran — every capture it ever made came from the fetch fallback. Porting that
verbatim would ship a dead branch and the §4 decision it implements. The content script
therefore returns `canvas.toDataURL()` and the background rebuilds the blob.

Departure 2 — the dynamic rule id is drawn from a counter, not from
`Math.floor(Date.now()/1000)`. Two captures in the same second get the same id there: the
second `addRules` collides and the first `finally` removes the rule the second is still
using, so one of them fetches without a `Referer` and can 403. On a path whose failure loses
a capture, a one-second race is not a shortcut worth keeping.

**D14. The POST reuses Phase 1 D15's part names.** `file` for the bytes, `meta` for the
JSON; any other name is a 400 by design, so this is a contract, not a convention. The
request is a `FormData` body with no hand-set `Content-Type` (the boundary must come from the
form), which also makes the browser send `Origin: chrome-extension://<id>` — the header the
app's origin layer requires (§5). One 30 s timeout per attempt: the app is on loopback, so a
request still open after that is a wedged app, and the capture is better off `failed` and
retryable than pending forever.

**D15. A failed delivery raises exactly one notification, whose text names the app.**
Wording follows §5: "Open BooruBox to receive this image". Clicking it opens the popup where
the Retry is, falling back to doing nothing if the browser refuses the call outside a user
gesture — the badge and the toolbar icon are the reliable path, the click is a shortcut.

**D16. A stored capture is announced to the webview, which re-runs its search on it.**
The listener runs beside the window, not under it: a capture changes the library with nothing
on screen having asked for it, and the library screen only re-reads on its own actions — so
an image saved from the browser stayed invisible until the route was remounted (leaving for
Settings and coming back was the trick that showed it). `POST /captures` now emits
`capture:stored` with the record it created, the library route re-runs its current search on
it, and the layout re-reads `LibraryStatus` so the sidebar's image count follows a capture on
every screen, not only on the one that lists the images. Only a newly created row is announced: a retry the app has already accepted changed
nothing, and reloading the grid for it would move the list under the user for an image
already in it. Rejected: polling `image_count` from the webview, which spends work on the
common case where nothing arrives, and still shows an image late.

**D17. The page names itself; the tab's title is only a fallback.**
X does not rewrite `document.title` when a photo is opened straight off a timeline, so the
page is still called "Home" while a post is on screen — the owner confirms the legacy
extension hit this too, and that it is X's own behaviour, not a limit of the extension APIs.
Reading the title live is therefore not the fix: `chrome.tabs` reports the same string, and
there is nothing fresher to read. Only the post in the DOM says what is on screen. The
content script answers `{ record, pageTitle }` rather than a record alone, and `pageTitle` is
the live `document.title` — which buys only freshness against a lagging tab record — unless
the adapter can do better. An adapter's `title(fields)` builds that name from the fields `extract`
already read, so the page is walked once and the title can never name a different post than
the record does.

This is still extraction, not policy (CLAUDE.md): it answers "what is this page called",
which only the page can say, and nothing in it interprets a field into a library concept. The
X adapter's title is used for every X capture rather than only the stale ones — X's own
`<title>` is localised, and two captures of one post would otherwise be named differently
depending on how the user got there. It needed the author's display name, so `displayName`
joins the fields X extracts.

*What the title is not for.* The owner has seen the difference between X's own
`X 上的 イブ：「夜の教室 https://t.co/…」 / X` and this adapter's
`イブ (@IV70311741) on X: 夜の教室`, and accepts either: `pageTitle` is a display name for a
row, and nothing reads meaning out of it. The artist's commentary a Danbooru upload needs is
`adapter.fields.postText` (`夜の教室` alone), not the title — `booru-upload`'s call, and the
reason `postText` is stored whole while the title truncates.

## Risks / Trade-offs

- [Canvas-first now really runs, and re-encodes a JPEG as PNG — captures get bigger than the
  bytes the site served] → accepted: §4 fixes the order, and the canvas is what survives
  hotlink protection and referrer gating. Recorded here because the *observed* legacy
  behaviour was fetch-only (D13), so this is a change in what lands on disk, not a port of
  it. Preferring the fetched original when both routes work is a later decision, and needs
  §4 reopened.
- [`unlimitedStorage` is a broad-looking grant on an extension that claims to store nothing]
  → it holds undelivered captures only; every delivered capture's bytes are deleted on the
  2xx, and the popup shows exactly what is being kept.
- [Adapters rot when X or Pixiv change markup] → fields are individually optional, an
  adapter that raises is caught and produces no record, the capture is never blocked (§10),
  and the dated fixtures say when the markup last worked.
- [The worker is killed mid-POST and the app did store the capture] → the retry is
  idempotent on the UUID (§5); the cost of the wrong guess is one 200 answer, not a duplicate.
- [The user changes the port in the app and not in the extension] → every capture fails
  loudly with its bytes kept, the indicator says not running, and the field to fix it is on
  the same panel.
- [Two context-menu entries during migration] → distinct labels and notification titles
  (§3); the migration notice tells the user to disable the old extension (§9).
- [`adapter_json` is written by the extension and read by nobody yet] → it is written now
  because the alternative is a second migration later plus captures that arrived in between
  with nothing recorded; the FIXME it spends says as much.

## Open Questions

- Whether the delivered-entry cap should be by count (shipping: 200) or by age. Both are one
  predicate in the same trim step and neither changes a spec or a task.
- Whether the popup's connected line should also show the library path next to the image
  count. Cosmetic; `GET /status` already returns it.
