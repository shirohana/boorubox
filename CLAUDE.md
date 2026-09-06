# BooruBox

Read `docs/requirements.md` before planning anything: every product decision is closed
there, with the argument. Do not re-open a §11 decision without recording why in the doc.

## Commands

All entry points are mise tasks (`mise tasks`). `mise run check` is the gate: lint,
typecheck, tests, clippy, builds. Tests run per package (`pnpm -r test`), never from a root
vitest config: the SvelteKit plugin pins vite `root` to `process.cwd()`, so a root runner
scans every package under the app project.

## Layout rules

- `packages/shared` holds only what both runtimes need: transport contract and site-adapter
  record types. It exports TypeScript source; Vite consumers compile it.
- **Extraction lives in the extension, policy lives in the app.** Site adapters produce a
  plain `{ site, fields }` record. Auto-tag rules, tag sorting, rating extraction, booru
  upload: app only. Two runtimes with the same rules drift.
- Lifted pure modules from the legacy repo (`tag-utils`, `filters`, `grouping`,
  `navigation-math`) go in the app, with their tests. Port the capture code
  (content-script canvas first, background fetch + Referer rule fallback) verbatim.
- Rust owns filesystem, storage, the HTTP listener and ingestion. The webview is UI only.
- Frontend: Svelte 5, no React. shadcn-svelte components under
  `src/lib/components/ui` are copy-in upstream code, managed by
  `pnpm dlx shadcn-svelte add <name>`; they are excluded from lint and format.

## Storage rules (from §7)

- SQLite via rusqlite with bundled FTS5. Image blobs stay on disk under `<library>/images/`.
- Tags as rows (`images` / `tags` / `image_tags`), never a comma string.
- Rollback-journal mode, not WAL: cloud-sync clients must see one file. One machine writes
  at a time.
- Model `posts` (image, site, remote id, posted at) from day one; the `posted:` filter and
  the pull-back feature land on it.

## Transport rules (from §5)

- Listener binds `127.0.0.1:47201` only. Accept requests only when `Origin` is
  `chrome-extension://…`. `POST /captures` is idempotent by the extension's UUID.
- A capture is `failed` (blob kept, Retry) until the app answers 2xx. Never report success
  earlier.

## Version pins

`mise.toml` is the only tool pin. Versions shared across packages live in the
`catalog:` of `pnpm-workspace.yaml`.
