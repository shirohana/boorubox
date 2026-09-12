## Why

`v0.1.0` is installed by hand from a GitHub release page. Nothing in the app knows a newer
version exists, and nothing tells the user which version they are running — `VERSION` is
reported by `/status` and `library_status` but rendered nowhere. That makes a bug report
unanswerable ("which build?") and makes every fix a manual reinstall, which is the wrong
shape for a product about to collect feedback from people who are not the author.

A version check only works in a build that already contains it. `v0.1.0` can therefore never
discover its successor: this change plants the client so the release *after* it is the first
one that can arrive on its own.

## What Changes

- The app checks for a newer version — at launch, and on demand from Settings.
- A newer version is **never** installed silently. The user is shown what is available and
  installs only after confirming; declining leaves the app exactly as it was.
- Settings shows the running version, so a user can answer "which build are you on".
- **Releases become regular GitHub releases, not pre-releases.** GitHub's
  `/releases/latest/download/…` skips pre-releases, so the manifest the updater reads is
  unreachable while every release carries that flag. The "this is early software" signal
  moves to the release's name, which is where a human reads it anyway.
- **Versions become date-shaped**: `<year-2000>.<month>.<day*100 + sequence>`, e.g.
  `26.9.1201`, and `26.9.1202` for a second release the same day. Still valid semver, still
  ordered correctly by the comparison the updater actually performs.
- A **portable** Windows executable is published beside the installer, with the release page
  saying plainly that taking an update from it installs the app properly (it carries the same
  updater client, and the Windows update path is the installer).

## Capabilities

### New Capabilities

- `app-update`: the app learns that a newer version exists, tells the user, and installs it
  after they agree.

### Modified Capabilities

None. No existing capability's requirements change.

## Impact

- `packages/app/src-tauri/Cargo.toml` — `tauri-plugin-updater`, `tauri-plugin-process`.
- `packages/app/src-tauri/src/lib.rs` — plugin registration.
- `packages/app/src-tauri/tauri.conf.json` — the updater's `endpoints`; the pubkey is already
  committed.
- `packages/app/src-tauri/capabilities/default.json` — the permissions the two plugins need.
- `packages/app/src-tauri/Cargo.toml` version — the new date-shaped version.
- `packages/app/src/routes/settings` and the settings UI — the version row and Check control.
- `.github/workflows/release.yml` — regular release, readable name, portable binary.
- `README.md` / `CLAUDE.md` — the versioning rule and what a release now looks like.

The private signing key and the committed public key already exist; this change consumes
them rather than introducing them.
