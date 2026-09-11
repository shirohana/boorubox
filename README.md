# BooruBox

Local image library and staging area in front of boorus. A Tauri desktop app owns the
files and metadata; a bridge Chrome extension is one of several sources that feed it.
Requirements and every closed decision: [docs/requirements.md](docs/requirements.md).

## Install

Releases live at <https://github.com/shirohana/boorubox/releases>: the `.dmg` for macOS, the
`-setup.exe` for Windows, and `boorubox-extension-<version>.zip` for the bridge extension
(unzip, then load it unpacked in `chrome://extensions`).

Builds are **not signed for the OS** -- no Apple Developer ID, no Authenticode -- so both
systems refuse them on first launch. Nothing is wrong with the download.

macOS says *"BooruBox is damaged and can't be opened"*. It is not damaged: Gatekeeper is
refusing the quarantine attribute that every downloaded file carries, and an unsigned app
has no signature to check it against. Clear the attribute:

```sh
xattr -rd com.apple.quarantine /Applications/BooruBox.app
```

Add `sudo` if that reports a permission error. Windows shows SmartScreen instead: *More
info* -> *Run anyway*.

## Layout

| Path                     | Package               | What                                                                |
| ------------------------ | --------------------- | ------------------------------------------------------------------- |
| `packages/app`           | `@boorubox/app`       | Tauri v2 app. SvelteKit (static, SPA) + Tailwind 4 + shadcn-svelte. |
| `packages/app/src-tauri` | crate `boorubox`      | Rust side: filesystem, SQLite, HTTP listener, ingestion.            |
| `packages/extension`     | `@boorubox/extension` | MV3 bridge extension: capture, site adapters, deliver, history.     |
| `packages/shared`        | `@boorubox/shared`    | Types both runtimes need (transport contract, adapter records).     |

## Setup

[mise](https://mise.jdx.dev) pins node, pnpm and rust; nothing else is installed globally.

```sh
mise install
pnpm install
mise run dev        # Tauri window with HMR
mise run check      # lint + typecheck + tests + clippy + builds
mise tasks          # everything else
```

The extension builds to `packages/extension/dist`; load it unpacked in `chrome://extensions`.
