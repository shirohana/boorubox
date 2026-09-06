# BooruBox

Local image library and staging area in front of boorus. A Tauri desktop app owns the
files and metadata; a bridge Chrome extension is one of several sources that feed it.
Requirements and every closed decision: [docs/requirements.md](docs/requirements.md).

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
