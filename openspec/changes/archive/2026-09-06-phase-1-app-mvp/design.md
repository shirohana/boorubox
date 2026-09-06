## Context

Greenfield: the repo holds a scaffolded Tauri v2 + SvelteKit app, an empty MV3 extension
and an empty shared package (see CLAUDE.md). Product decisions are closed in
docs/requirements.md; this design only settles how Phase 1 is built. Motivation: see
proposal.md.

Constraints that shape the approach: Rust owns filesystem, storage, listener and ingestion;
the webview is UI only (§6). SQLite + FTS5, tags as rows, rollback journal, one writer (§7).
Localhost HTTP with an `Origin` check, idempotent by id (§5). Extraction in the extension,
policy in the app (§4).

## Goals / Non-Goals

**Goals:**

- One process owns the library: every write goes through Rust, through one SQLite
  connection, so "one writer at a time" holds inside the app by construction.
- The search parser lifted from the legacy repo stays the single definition of the query
  language; Rust never re-parses query strings.
- The library folder is self-contained and copyable; nothing outside it is needed to open
  it except the remembered path.

**Non-Goals:**

- Multi-window or multi-library. Design for one open library per app instance.
- Rebuildable index from sidecars (§7 Phase 3 candidate).

## Decisions

**D1. Storage: rusqlite with `bundled` and `fts5` features, one connection behind a mutex
in Tauri managed state.** Alternative: tauri-plugin-sql. Rejected: it exposes SQL to the
webview, which would put storage logic in the UI layer and contradict §6.

**D2. Schema (v1), applied by a migration runner keyed on `user_version`.**

```
images     (id TEXT PK, ext TEXT, mime TEXT, size INTEGER, width INTEGER, height INTEGER,
            source TEXT, source_ref TEXT, image_url TEXT, page_url TEXT, page_title TEXT,
            rating TEXT, captured_at INTEGER, created_at INTEGER, updated_at INTEGER,
            deleted_at INTEGER NULL, missing INTEGER DEFAULT 0)
tags       (id INTEGER PK, name TEXT UNIQUE)
image_tags (image_id, tag_id, PK(image_id, tag_id))
posts      (image_id, site TEXT, remote_id TEXT, posted_at INTEGER, PK(image_id, site))
images_fts (FTS5 external-content over page_title, page_url, image_url)
```

`source` is `extension` | `local` | `legacy-bundle`, with `source_ref` carrying the
adapter site or the bundle id. Per-source counts are one `GROUP BY source`. `posts` is
created now so §6's `posted:` filter has a home; Phase 1 never writes it.
`PRAGMA journal_mode=DELETE` (the default) is set explicitly so a later edit cannot
silently switch to WAL.

**D3. Query language stays in TypeScript.** `parseTagSearch` is lifted verbatim with its
tests and runs in the webview. The webview sends the parsed structure (`ParsedTagSearch`:
AND tags, OR groups, exclusions, rating, is:, tagcount:, account:) to a Tauri command;
Rust compiles it to SQL. Alternative: port the parser to Rust. Rejected: two parsers drift,
and the TS one is the tested artifact §6 says to lift.

**D4. Ingest path: write to `inbox/<id>.part`, fsync, rename to `images/<id>.<ext>`, then
insert the row in one transaction.** A crash leaves at most a stray `.part` file, which is
swept at startup. Idempotency: `INSERT ... ON CONFLICT(id) DO NOTHING`; if the row existed,
the request still returns 200 with the existing record and the uploaded bytes are
discarded.

**D5. HTTP listener: axum on the tokio runtime Tauri already provides, started in
`setup()`, bound to `127.0.0.1` only, port from settings (default 47201).** A tower layer
rejects any request whose `Origin` header is absent or not `chrome-extension://…` with
403, except `GET /status`, which also answers same-origin-less requests from the
migration notice and CLI sources; the pairing-token slot from §5 is a second layer added
later. Alternative: tiny_http. Rejected: axum's multipart and layering are needed anyway.

**D6. Settings (library path, port) live in the app config dir as `settings.json`, via
`tauri-plugin-store`.** They must survive the library being moved and must not be inside
the library, or a copied library would carry another machine's path.

**D7. Images reach the webview through Tauri's asset protocol** (`convertFileSrc`), with
the asset scope granted for the library folder at runtime when it is chosen. Thumbnails
are generated on ingest with the `image` crate into `<library>/.thumbs/<id>.jpg` (derived
cache, safe to delete; regenerated on demand when missing). Alternative: base64 through
IPC. Rejected: too slow for a grid of thousands.

**D7a. The asset scope is granted per directory, not on the library root.** D7 said the
scope is granted "for the library folder", and that was wrong in a way only a running window
showed: Tauri's fs scope sets `require_literal_leading_dot` on unix, so a recursive pattern
over the root matches nothing inside the dotted `.thumbs/`, and every thumbnail in the grid
answered 403 while the Rust log stayed silent. `images/` and `.thumbs/` are therefore granted
by name. The narrower grant is also the better one on its own terms: `library.sqlite` and
`inbox/` are no longer reachable from the webview, which is half of the risk this design
already listed. Verified in the app on 2026-09-06, before and after.

**D8. Local import assigns fresh UUIDs, copies the file into `images/`, and records
`source=local`, `page_title=<filename>`, `captured_at=<mtime>`.** The original is never
moved or deleted.

**D9. Legacy bundle import — moved out of this change.** It stayed gated on the legacy
repo's Phase 0 long after the rest was done, so the capability and this decision left
together: the mapping is now D1 of `openspec/changes/legacy-bundle-import/design.md`. The
number stays retired rather than reused, because D10 and D12-D17 are cited by name in code
comments and commit messages and renumbering would quietly break every reference. D2 still
models `source=legacy-bundle`, which is what that change will write.

**D10. Frontend structure**: `src/lib/domain/` holds the lifted modules and the query
types shared with Rust via `@boorubox/shared`; `src/lib/api/` wraps every `invoke` so
components never call Tauri directly; routes are `/` (grid), `/setup` (folder pick),
`/import`.

**D11. Module map.** Each file below has one owner and one job, so the units of work
cannot collide. Rust (`packages/app/src-tauri/src/`):

| file | holds |
| --- | --- |
| `lib.rs` | Tauri builder, `AppState`, `setup()` (listener start, asset scope), command registration |
| `model.rs` | Rust mirror of `packages/shared/src/index.ts` (serde `camelCase`) |
| `error.rs` | `AppError` (thiserror) + `Result`, serialised for command failures |
| `settings.rs` | `Settings { library_path, port }` on tauri-plugin-store |
| `db.rs` | migration runner on `user_version`, schema v1, `journal_mode=DELETE` |
| `library.rs` | `Library` — folder layout, inbox sweep, owns the `Connection` |
| `ingest.rs` | the D4 write path: part file, fsync, rename, transactional row insert |
| `query.rs` | `ParsedTagSearch` → SQL, the `x_account()` scalar function |
| `maintenance.rs` | per-source counts, missing detection and clearing, record drop |
| `thumbs.rs` | thumbnail generation into `.thumbs/<id>.jpg` |
| `import.rs` | local file and folder import with progress events |
| `http/` | `mod.rs` router and start, `origin.rs` layer, `captures.rs`, `status.rs` |
| `commands.rs` | the `#[tauri::command]` surface, nothing but argument marshalling |

`AppState { library: Mutex<Option<Library>>, settings: Mutex<Settings>, listener:
Mutex<ListenerStatus> }` in Tauri managed state. The `Mutex` around `Library` is what makes
D1's "one connection" true: `rusqlite::Connection` is `Send` but not `Sync`.

Frontend (`packages/app/src/`): `lib/domain/` the lifted pure modules, `lib/api/` one
wrapper per command, `routes/` the pages.

**D12. Command surface** (the contract `lib/api/` wraps and `commands.rs` implements):

```
pick_library()                  -> LibraryStatus   // dialog; cancel returns status unchanged
open_library(path)              -> LibraryStatus
library_status()                -> LibraryStatus
search(req: SearchRequest)      -> SearchResult
image_counts()                  -> ImageCounts
drop_image_record(id)           -> LibraryStatus
thumbnail_path(id)              -> string          // absolute; generated on demand
import_paths(paths: string[])   -> ImportReport    // emits `import:progress`
```

**D13. `ParsedTagSearch` crosses IPC, so its sets become arrays.** D3 said the parser is
lifted verbatim; that held while the parser's output stayed inside one JavaScript process.
It stops holding here because the parsed structure is the wire format to Rust and `Set` has
no JSON form. The lifted parser therefore returns the `@boorubox/shared` shape with
`string[]` where the legacy module had `Set<string>`; its tests are lifted with the same
substitution. The query language itself is unchanged.

**D14. Free text is a second input, not bare words in the tag box.** The `library-browse`
requirement asks for both the tag syntax and free text over page title and URLs. Bare words
in the tag box are tags — that is the legacy syntax — so free text needs somewhere else to
live, exactly as the lifted `filters.ts` models it (`FilterInputs.tagSearch` and
`FilterInputs.urlSearch`). The search bar holds both, and `SearchRequest` carries them
separately: `query` compiles to tag clauses, `text` to an FTS5 match.
**D15. Multipart field names on `POST /captures`: `file` and `meta`.** The spec fixes the
parts ("exactly one file part and one JSON part") but not their names, and the bridge
extension in Phase 1b has to send the same two names this listener reads. `file` carries the
bytes, `meta` carries the `CaptureMeta` JSON. Any other field name is a 400: silently
ignoring an unknown part would turn a misspelled client into a capture that never arrives.

**D16. Dropping a record leaves `images/` alone.** The `library-folder` spec offers the drop
action on an image whose file is already gone, and Phase 1 has no trash UI to undo a
deletion. `drop_image_record` therefore deletes the row (with its tags and posts, by
cascade) and the derived thumbnail, and never unlinks a file under `images/`. A destructive
variant is a later decision, not a silent one.
**D17. A failed listener is a banner on the library, not a settings screen.** The
`capture-ingest` spec says settings show the listener as failed with the reason, and §5
calls the port a settings field — but Phase 1's route list (D10) is `/`, `/setup` and
`/import`, with no settings screen to put it on. Building one to hold a single line would
be the wrong order. The reason is shown as a persistent banner on the library view instead,
where a user who is waiting for captures will actually be. When a settings screen exists,
the banner moves and this decision is spent.
## Risks / Trade-offs

- [FTS5 external-content tables desync from `images` on partial writes] → every write is
  one transaction that updates both; a `rebuild` command exists from day one.
- [Large drop of folders blocks the UI] → import runs on a blocking thread pool with
  progress events; the UI shows a running count.
- [Asset protocol scope leaks the whole library to any webview page] → the webview is only
  our SPA; CSP stays default; no remote content is loaded.
- [Port 47201 taken] → listener start failure is surfaced in settings with the reason; app
  still opens.

## Open Questions

- Thumbnail edge size (256 vs 512 px). Default 384 until the grid exists to judge.
- Whether `GET /status` should include per-source counts or only the total. Total only
  until the migration notice is written.
