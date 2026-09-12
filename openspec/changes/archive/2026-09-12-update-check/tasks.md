> Lands **after** `import-pause-cancel` — group 3 reads the `Imports` queue that change is
> editing, and both touch `lib.rs`. Do not start until that change is committed.
>
> Two implementing agents once it does. **C** owns `packages/app/src-tauri/**` and
> `.github/workflows/release.yml`; **D** owns `packages/app/src/**`. Group 4 is the lead's.
>
> The contract they share:
> - No new Tauri commands. C registers `tauri_plugin_updater` and `tauri_plugin_process` and
>   grants their permissions; D calls the JS guest bindings (`check()`,
>   `downloadAndInstall()`, `relaunch()`) directly.
> - The running version reaches the webview through the existing status payload, which
>   already carries `version` — D does not add a command for it.

## 1. Rust: the plugins, the endpoint, the permissions

- [x] 1.1 `packages/app/src-tauri/Cargo.toml`: `tauri-plugin-updater` and
      `tauri-plugin-process`, both v2, behind the same
      `[target."cfg(not(any(target_os = \"android\", target_os = \"ios\")))".dependencies]`
      guard the plugin documents.
- [x] 1.2 `packages/app/src-tauri/src/lib.rs`: register both plugins beside the existing ones.
- [x] 1.3 `packages/app/src-tauri/tauri.conf.json`: `plugins.updater.endpoints` =
      `["https://github.com/shirohana/boorubox/releases/latest/download/latest.json"]`. The
      `pubkey` is already there; do not regenerate or replace it.
- [x] 1.4 `packages/app/src-tauri/capabilities/default.json`: the permissions the two plugins
      need, no wider than needed (`updater:default` and the restart permission, not a blanket
      `process:default` — check the plugins' own permission lists and take the narrow ones).
- [x] 1.5 `cargo test`, `cargo clippy --all-targets -- -D warnings` and
      `cargo fmt --check` pass. `pnpm --filter @boorubox/app tauri build --no-bundle` compiles
      (it needs no signing key; the full bundle build does, and you do not have it).

## 2. The release the updater points at

- [x] 2.1 `.github/workflows/release.yml`: publish a **regular** release, not a pre-release —
      both the `draft` job's `createRelease` and the `publish` job's `updateRelease`
      (design D2). The release name carries the readable date and `alpha`, e.g.
      `BooruBox 2026-09-12 alpha`; the body keeps the unsigned-build warning.
- [x] 2.2 `.github/workflows/release.yml`: `uploadPlainBinary: true` on the tauri-action
      step, and the release body says what the portable exe is and that taking an update from
      it installs the app properly (design D7).
- [x] 2.3 Do **not** bump the version as part of this change. The version names the day the
      release is cut, which is not the day the code lands, so it is set as part of tagging:
      bump `packages/app/src-tauri/Cargo.toml` to `year-2000`.`month`.`day*100 + sequence`
      computed for that day (design D3), commit, then tag `v<that version>`. Write that
      ritual into `CLAUDE.md` under Releases as part of 2.4; leave `Cargo.toml` at `0.1.0`.
- [x] 2.4 `CLAUDE.md` Releases section and `README.md`: the version scheme with its reasons
      (the sequence exists for same-day fixes; `year-2000` keeps MSI reachable; a published
      version is never reused), and that releases are regular with the stage in the name.

## 3. The webview: the version, the check, the confirmation

- [x] 3.1 Settings shows the running version, read from the status payload that already
      carries it — no new command, no second source (design D3's single-source rule).
- [x] 3.2 A launch check: fire-and-forget, silent on failure, shows something only when an
      update exists (design D4). It must not delay the window appearing.
- [x] 3.3 A Check control in Settings that answers in all three cases — newer, current,
      failed (design D4).
- [x] 3.4 The confirmation: what version is available, install or decline. Declining installs
      nothing and does not prompt again this session (design D5). Verify with tests that a
      decline leaves nothing downloaded and suppresses the rest of the session's prompts.
- [x] 3.5 Refuse to install while an import or a capture is in flight, naming what is running
      (design D6). Read the `Imports` queue — `import-pause-cancel` has just changed it, so
      read it as it is now, not as an older file showed it. Derive anything off a
      wholesale-reassigned store object with `$derived`, never an `$effect`.
- [x] 3.6 `pnpm -r test`, `pnpm typecheck`, `pnpm lint` pass.

## Handoff (agent D, group 3)

Landed as specced. One deviation from the first pass, made after a review found a real
mid-install restart hole — see "the in-flight recheck" below.

- Installed `@tauri-apps/plugin-updater` (`2.11.0`) and `@tauri-apps/plugin-process` (`2.3.1`)
  into `packages/app/package.json`, and read their real `dist-js/index.d.ts` before calling
  anything — `check()`, `Update#download()`, `Update#install()`, `relaunch()` — rather than
  working from memory. All invoke the same `plugin:updater|…`/`plugin:process|…`/
  `plugin:resources|close` commands the rest of the app's own commands go through, so tests use
  `mockIPC` exactly like a Rust command; no plugin module is mocked directly.
- New store `packages/app/src/lib/api/update.svelte.ts` (`AppUpdate`, singleton `appUpdate`):
  `checkOnLaunch()` (silent, deduped), `checkNow()` (returns `'available' | 'current' |
  'failed'`, and a second call while one is in flight shares the first's answer rather than
  racing it), `decline()`, `reopen()`, `install(busyWith)`. `available` is `$state.raw` — an
  `Update` is a Tauri `Resource` with its own `close()`, not a plain object to proxy.
  `install()` takes the in-flight description as a *function* (see below), not a value, so the
  one `$derived` read of `imports`/`pendingCaptures` lives in the dialog component (design D6's
  `$derived`-not-`$effect` rule binds a component, not a plain class method) while still being
  re-readable live.
- **The in-flight recheck (post-review fix).** The first pass checked `busyWith` once, at the
  click, and called the plugin's combined `downloadAndInstall()`. That leaves the whole download
  — which can run for minutes — with nothing checking whether an import or capture started
  after the click. Fixed by calling the plugin's `download()` and `install()` separately and
  reading `busyWith()` again immediately before `install()`, not just before `relaunch()`: on
  Windows (the only platform this app ships builds for today, design D3) `install()` itself is
  what exits the process, so a check placed after `downloadAndInstall()`'s combined call, or even
  right before a following `relaunch()`, can be too late — there is no "install but don't
  restart" on that platform, only "download but don't install yet". A refusal that lands after
  the download completes does not discard it (`downloaded` stays `true`, `available` stays set)
  — the next `install()` call, once free, installs from what is already on disk instead of
  fetching it twice.
- **Lifecycle fix.** `available`'s `Update` handle is now closed (`.close()`, fire-and-forget)
  and cleared on `decline()` and on any install failure, and superseded by `#discard()` inside
  `#found()` when a fresh check hands back a different one — no more a stale "Update to …" with
  no way back to Check, no leaked Rust-side resource id, and no offer that outlives a release
  that got pulled. `declined` itself is untouched by this — it is the session's own "already
  asked", not a property of the update — so `reopen()` still means something: a later explicit
  check can find the update again (surfacing "Update to …") without re-opening the dialog on its
  own, and `reopen()` is the deliberate way back in.
- **Re-entrancy.** `install()` now guards on `this.installing` at its top (synchronous, so two
  back-to-back calls before the first `await` cannot both proceed — no two installers over the
  same app from a double-click landing before the button's `disabled` re-renders). `checkNow()`
  got the same shape via an in-flight-promise cache rather than a boolean, since it has to answer
  with a real `CheckOutcome` rather than silently no-op.
- New pure function `packages/app/src/lib/components/update/update-work.ts` (`workInFlight`):
  turns `imports.runs.length` / `pendingCaptures.entries.length` into the refusal's wording —
  "an import is still running", "a capture is still running", or both joined. Tested directly.
- New `UpdateDialog.svelte`, mounted once from `+layout.svelte` (like `pendingCaptures`, an
  update can be found while the user is on any screen). Reads `appUpdate`, `imports`,
  `pendingCaptures` as singletons, no props — the same convention `BooruSection`/`RulesSection`
  use on the Settings screen. Dismissing by the overlay or Esc declines, matching
  `ConfirmDialog`'s "the safe direction" convention. The install button is not disabled while
  work is in flight — clicking it re-checks and reports the refusal (spec scenario "confirms
  ... while work is in flight" is a click that gets answered, not a button that never lets one
  through) — and the same reason is also shown as a standing note under the version text. Passes
  `install` a closure (`() => busyWith`) reading the live `$derived` value, not the value itself.
- Settings gained an "About" section: running version (guarded on `library.status` being
  answered yet — nothing renders a "Version" label over a blank frame) plus either "Check for
  updates" (idle) or "Update to <version>" (once one is on offer). `checkForUpdate()` shows "up
  to date"/the failure text locally; the `'available'` outcome hands off to the button and
  dialog, nothing more to say inline.
- Signature verification (spec "Tampered artifact") is entirely the plugin's per D1; `install()`
  just relays whatever `download()`/`install()` rejects with through `installError` rather than
  swallowing it, and drops the update rather than holding a handle that already failed once.
- `packages/app/package.json`'s `"version": "0.1.0"` field is removed — nothing read it (Tauri
  reads `src-tauri/Cargo.toml`/`tauri.conf.json`; `pnpm install`, `typecheck`, `lint` all stayed
  clean without it). `packages/shared` and `packages/extension` still carry their own `version`
  fields; left alone, out of this change's file ownership.
- Untestable until a later release exists (per the brief): the actual discovery of a real newer
  version, a real download, a real signature check, and a real restart — including which of
  `install()`'s two documented platform behaviors (Windows exits the process; macOS/Linux need
  our own `relaunch()`) actually plays out, which only a real Windows build can confirm. Tests
  cover the decision logic and refusals against `mockIPC` stand-ins: launch check
  silent-on-failure and deduped, explicit check answering all three cases and de-racing a
  concurrent call, decline dropping and closing the update, the in-flight recheck refusing
  immediately before `install()` while keeping the download, a later install finishing from the
  kept download without re-fetching, the re-entrancy guard preventing a second concurrent
  install, and a download/install failure surfacing (not swallowing) its rejection while
  dropping the update.
- Gate: `pnpm -r test` (520 passed across `shared`/`extension`/`app`), `pnpm typecheck`,
  `pnpm lint` — all clean.

## 4. Lead

- [x] 4.1 Review each unit against the spec before the next touches its files;
      `mise run check` green on the merged result.
- [ ] 4.2 Hand check: install the built app, confirm Settings shows the right version, that
      Check answers when current, and that an available update prompts and installs only on
      confirm. The install half cannot be proven until a *later* release exists to update to
      — the release after this one is the first real proof. Owner's to tick.
- [x] 4.3 Commit. Tag only on the owner's word.

## Handoff (agent C, groups 1-2)

Both groups landed as specced, no deviations.

- Plugin registration confirmed against the plugins' own source
  (`tauri-apps/plugins-workspace`, `v2` branch, `plugins/updater` and `plugins/process`), not
  memory: `tauri_plugin_updater::Builder::new().build()` and `tauri_plugin_process::init()`,
  both added to the same `.plugin()` chain as the existing plugins in `lib.rs` — no
  `#[cfg(desktop)]` guard needed since the app has no mobile scaffolding at all (no
  `gen/android` or `gen/apple`; the Cargo.toml target-guard alone is what the plugins' own
  install docs show).
- Permission identifiers, verified against each plugin's `permissions/` TOML in the same repo:
  `updater:default` (plugin `updater`'s only set: `allow-check`, `allow-download`,
  `allow-install`, `allow-download-and-install` — that's the whole conversation D1 describes,
  not overgranted) and `process:allow-restart` (plugin `process`; deliberately not
  `process:default`, which also grants `allow-exit` — nothing in this change quits the app).
- `tauri-action`'s `uploadPlainBinary` input name verified against `tauri-apps/tauri-action`'s
  own `action.yml` (`v1.0.0`), not memory.
- Release name step decodes `BooruBox <date> alpha` from the tag itself (`release.yml`,
  "The release name carries the tag's date and stage"). An earlier draft used the runner's
  own date; the review replaced it because the owner is UTC+8 and a runner's clock disagrees
  with the day the version names (D3).
- Gate run: `cargo fmt`, `cargo test` (439 passed), `cargo clippy --all-targets -D warnings`
  (clean), `pnpm --filter @boorubox/app tauri build --no-bundle` (compiles) — all green.
  `.github/workflows/release.yml` re-read against its own diff and validated as YAML
  (`python3 -c "import yaml; yaml.safe_load(...)"`); it cannot be run without a tag push, which
  is the owner's call.
- Left open: D7's FIXME (a portable build that knows it's portable) is unchanged — this change
  only makes the honest-labeling call the design already settled, not the build-time flag.
  Nothing else deferred from groups 1-2.
