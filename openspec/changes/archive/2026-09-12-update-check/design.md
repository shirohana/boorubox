## Context

`tauri-plugin-updater` already does the part that is dangerous to write by hand: fetch a
manifest, compare versions, download, verify a minisign signature, install. The signing
keypair exists — the public half is in `tauri.conf.json`, the private half in the repo's
Actions secrets — and `bundle.createUpdaterArtifacts` is on, so every release since `v0.1.0`
has published a `latest.json` with real signatures. What is missing is a client that reads it
and a user who is asked first.

## Goals / Non-Goals

**Goals**
- The user learns a newer version exists without visiting GitHub.
- Nothing is replaced without a yes.
- The running version is answerable from inside the app.

**Non-Goals**
- Update channels (stable vs beta). One line of releases, one manifest.
- Background download before the user agrees.
- Delta updates.
- Rolling back to an earlier version.

## Decisions

### D1: The plugin does the mechanism; we own the conversation

`tauri-plugin-updater` for check/download/verify/install, `tauri-plugin-process` for the
restart. We add no Rust commands: the JS guest bindings do all of it from the webview, which
is where the confirmation lives anyway. The Rust side gains two `.plugin()` registrations and
the two permissions in `capabilities/default.json`.

Signature verification is the plugin's, against the embedded public key. Writing that
ourselves would be the single worst place in this app to be clever.

### D2: One endpoint, `/releases/latest/download/latest.json` — which forces regular releases

The manifest is read from
`https://github.com/shirohana/boorubox/releases/latest/download/latest.json`.

**This reverses a decision made the same week.** `v0.1.0` was published with GitHub's
pre-release flag, deliberately: the owner wanted users to see that the software is early. That
was the right call for a release nothing consumed. It stops being right the moment an updater
reads from GitHub, because `/releases/latest/` **skips pre-releases** — with the flag set, the
manifest URL resolves to nothing at all.

The signal does not disappear, it moves: the release's *name* carries "alpha" and the readable
date, which is where a person reads it. Nothing parses a release name. The alternative —
hosting `latest.json` somewhere else — buys a stable URL at the cost of a second place to
publish and keep in sync, for a signal a name already carries.

### D3: Date-shaped versions, computed rather than concatenated

`major = year - 2000`, `minor = month`, `patch = day * 100 + sequence`. Today's first release
is `26.9.1201`; a same-day fix is `26.9.1202`; October 1st is `26.10.101`.

- **Computed, never string-concatenated.** Semver forbids leading zeros in numeric
  identifiers, so a pasted `01` + `01` is invalid where `1 * 100 + 1 = 101` is fine.
- **The sequence exists because the date fills all three slots.** Without it there is no way
  to ship twice in one day, which is exactly what a bad release needs.
- **Ordering survives every rollover**: `1201 < 1202 < 1301` inside a month, `9 < 10` across
  months, `26 → 27` across years. The updater's comparison is plain semver `>`.
- **`major = year - 2000` is not cosmetic.** WiX caps an MSI's first version field at 255, so
  a literal `2026` would put MSI permanently out of reach. We bundle NSIS only today, by
  choice; `26` keeps that a choice.
- **A published version is never reused.** Not even seconds after publishing, and not by
  deleting and re-tagging: an install that has already seen `26.9.1201` will not take another
  one, so a re-tag reaches some users and not others.

The version lives in `packages/app/src-tauri/Cargo.toml` and nowhere else — `tauri.conf.json`
deliberately has no `version` key, so what `/status` reports and what the manifest advertises
cannot drift.

### D4: Two checks, different manners

The **launch** check is fire-and-forget and silent about everything except an available
update: a person opening an image library did not ask about the network, and an error toast on
every offline launch is noise that teaches people to ignore toasts.

The **Settings** check is explicit, so it answers in all three cases — newer, current, or the
check failed. Someone who pressed a button is owed a reply.

### D5: Declining is remembered for the session, not forever

A declined update does not prompt again until the app restarts. Forever would need stored
state and a way to undo it; once-per-launch is the smallest thing that is not nagging, and the
Settings check is always there for someone who changes their mind.

### D6: Work in flight outranks an update

Restarting mid-import would abandon a migration with no report. The webview already knows what
is running (the `Imports` queue, pending captures), so the confirmation refuses while work is
in flight and says what is running. This is a real refusal, not a delay-and-hope.

### D7: The portable executable is honest about what it is

The plain `boorubox.exe` is published beside the installer. It contains the same updater
client, so it *will* find updates, and taking one runs the NSIS installer — turning a portable
copy into an installed one. That is surprising enough to say on the release page rather than
engineer around; detecting "am I the portable build" is guesswork the binary has no reliable
basis for.

**FIXME**: the honest shape is a portable build that knows it is portable and offers a
download link instead of an in-place install. That needs a build-time flag the release
workflow sets, which is more machinery than this change should carry.

## Risks / Trade-offs

- **The first proof is one release away.** `v0.1.0` has no client, so it cannot discover
  anything; the build carrying this change must be installed by hand, and the release after
  *that* is the first update anyone takes. Nothing can shorten this.
- **`/releases/latest/` is a redirect we do not control.** If GitHub changed it, updates would
  stop rather than misfire; an update that cannot be found is the safe failure.
- **Unsigned for the OS.** The updater verifies our minisign signature, which is what protects
  the payload, but macOS and Windows still warn on install. Deferred deliberately; the
  README says how to get past it.
