# BooruBox

Read `docs/requirements.md` before planning anything: every product decision is closed
there, with the argument. Do not re-open a §11 decision without recording why in the doc.

## Commands

All entry points are mise tasks (`mise tasks`). `mise run check` is the gate: lint,
typecheck, tests, clippy, builds. Tests run per package (`pnpm -r test`), never from a root
vitest config: the SvelteKit plugin pins vite `root` to `process.cwd()`, so a root runner
scans every package under the app project. Formatting is ESLint Stylistic (`mise run format`
runs `eslint --fix` and `cargo fmt`); there is no Prettier.

## Planning

Work is planned as openspec changes under `openspec/changes/`. Phase 1 is
`phase-1-app-mvp`; implement it with `/opsx:apply`. Task group 8 (legacy bundle import) is
gated on the legacy repo shipping Phase 0.

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
- An `$effect` that reads a field off a store object reassigned wholesale (the shape of
  `library.status?.libraryPath`) re-runs on every status refresh. Derive the field with
  `$derived` and depend on that. Three reviews in a row flagged the effect form.

## Storage rules (from §7)

- SQLite via rusqlite with bundled FTS5. Image blobs stay on disk under `<library>/images/`.
- Tags as rows (`images` / `tags` / `image_tags`), never a comma string.
- Rollback-journal mode, not WAL: cloud-sync clients must see one file. One machine writes
  at a time.
- Model `posts` (image, site, remote id, posted at) from day one; the `posted:` filter and
  the pull-back feature land on it.
- A plan claims its schema version by queue position; the real version is `MIGRATIONS.len()`
  in `db.rs` when the change is applied. Amend the design's sentence to the real number,
  never pin the planned one.

## Transport rules (from §5)

- Listener binds `127.0.0.1:47201` only. Accept requests only when `Origin` is
  `chrome-extension://…`. `POST /captures` is idempotent by the extension's UUID.
- A capture is `failed` (blob kept, Retry) until the app answers 2xx. Never report success
  earlier.

## Version pins

`mise.toml` is the only tool pin. Versions shared across packages live in the
`catalog:` of `pnpm-workspace.yaml`.

The app's own version lives in `packages/app/src-tauri/Cargo.toml` and nowhere else.
`tauri.conf.json` carries no `version` key on purpose: tauri falls back to Cargo.toml when
it is absent, and `VERSION` (`CARGO_PKG_VERSION`, what `/status` and `library_status`
report) has to be the same string the updater finds in `latest.json`. Two keys can drift,
and the drift shows up as an update that is offered forever or never.

## Releases

Tagging `v<the Cargo.toml version>` runs `.github/workflows/release.yml`: macOS and Windows
bundles, the bridge extension as a zip, `latest.json` and its signatures, drafted first and
published once every platform has uploaded. The release job fails if the tag and the
Cargo.toml version disagree.

Bundles are signed for the *updater* (minisign, `plugins.updater.pubkey` in
`tauri.conf.json`, private half in the repo's Actions secrets) but not for the OS: no Apple
Developer ID, no Authenticode, so first launch shows an unidentified-developer warning on
macOS and SmartScreen on Windows. Deferred until a production release, not forgotten.

Windows builds NSIS (`setup.exe`) only, not MSI: one file to tell a user to install, and
WiX rejects version strings the updater is otherwise happy with.

`mise run build:app` now needs the signing key, because a committed `pubkey` with no private
key is a hard error ("A public key has been found, but no private key"), not a warning — the
bundler refuses to emit an updater artifact it cannot sign. Export both before a local bundle
build:

```
export TAURI_SIGNING_PRIVATE_KEY=$(cat ~/.tauri/boorubox.key)
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=...
```

To check only that the app compiles, `pnpm --filter @boorubox/app tauri build --no-bundle`
skips bundling and needs no key.
