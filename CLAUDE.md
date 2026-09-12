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
- Window drag regions are macOS-only: bind `data-tauri-drag-region={windowDragRegion}` from
  `$lib/platform`, never the bare attribute. Windows has its native title bar, and there a
  drag region only triggers the focus-toggle bug (tauri-apps/tauri#10767).
- Never re-take focus on a window `blur` outside macOS (`isMacos` from `$lib/platform`). On
  Windows a title-bar drag blurs the webview with the window still focused, so a handler that
  pulls focus back loops with the drag until the user switches windows. Same upstream bug.
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

Releases are **regular** GitHub releases, not pre-releases (`app-update` design D2):
`/releases/latest/download/…` is what the in-app updater reads, and GitHub's `/releases/latest/`
skips pre-releases — with that flag set the manifest URL resolves to nothing. The "this is
early software" signal that the flag used to carry moves to the release *name* instead, e.g.
`BooruBox 2026-09-12 alpha`: nothing parses a release name, a person reads it on the page.

The version is not part of this change's normal flow — it is set as part of tagging, by hand:
bump `packages/app/src-tauri/Cargo.toml`'s `version` to
`<year - 2000>.<month>.<day * 100 + sequence>` for the day the release is cut (e.g. `26.9.1201`
for the first release on 2026-09-12, `26.9.1202` for a same-day fix), commit, then tag
`v<that version>`.

- **Computed, never string-concatenated.** Semver forbids leading zeros in a numeric
  identifier, so pasting `01` next to `01` is invalid where `1 * 100 + 1 = 101` is fine.
- **The sequence exists because the date already fills all three slots.** Without it there is
  no way to ship a same-day fix, which is exactly what a bad release needs.
- **`year - 2000` keeps MSI reachable.** WiX caps an MSI's first version field at 255, so a
  literal `2026` would put MSI permanently out of reach. NSIS-only is a choice today, by
  design (below); `26` keeps it a choice rather than a wall.
- **A published version is never reused** — not by deleting and re-tagging either: an install
  that has already seen `26.9.1201` will not take another release under that number, so a
  re-tag reaches some installs and not others while looking, from GitHub, like one release.

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
