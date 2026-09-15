> Depends on `app-shell`, archived. One implementing agent; the only Rust-side edit is the
> capability JSON. Gate: `pnpm -r typecheck`, `pnpm lint`, `pnpm --filter @boorubox/app test`,
> then `mise run check` (its build validates the permission identifiers).

## 1. The store, the key and the permissions (agent A)

- [x] 1.1 `packages/app/src-tauri/capabilities/default.json`: add
      `core:window:allow-set-fullscreen` and `core:window:allow-is-fullscreen` (design D4).
      Verify: `cargo build --manifest-path packages/app/src-tauri/Cargo.toml` succeeds.
- [x] 1.2 `packages/app/src/lib/fullscreen.svelte.ts`: `active`, `toggle()`, `refresh()` per
      design D1, with a header comment saying why the state is read back rather than assumed.
      Verify: typecheck and lint pass; a unit test with the window plugin mocked covers that
      `toggle()` asks for the opposite of `active` and stores what `isFullscreen()` answers.
- [x] 1.3 `packages/app/src/routes/+layout.svelte`: F11 in the window key handler →
      `fullscreen.toggle()` with `preventDefault`; `onresize` → `fullscreen.refresh()` (design
      D2). `packages/app/src/lib/keyboard.ts`: `KEY_FULLSCREEN = 'F11'` and the map row
      `Anywhere · F11 · Enter or leave full screen`. Verify: typecheck and lint pass; the row
      shows on /settings.

## 2. The button (agent A)

- [ ] 2.1 `packages/app/src/lib/components/frame/TopBar.svelte`: the full-screen button at the
      end of the bar, off macOS only, icon and `aria-pressed` from `fullscreen.active` (design
      D3). Verify: typecheck and lint pass.
      Hand check (Windows): the button sits at the right end of the top bar on the library and on
      /settings; pressing it fills the screen with no title bar and flips the icon; pressing
      again restores the window; F11 does the same; after leaving full screen the grid's arrows
      still work without a click. Hand check (macOS): no button; F11 either toggles full screen
      or is taken by the system, and ⌃⌘F still works.

## 3. Change-level verification (owner)

- [ ] 3.1 `mise run check` green; the hand checks above pass.

## Handoff

- Groups 1 and 2 landed. `fullscreen.svelte.ts` exports `fullscreen` (a singleton instance of
  a `Fullscreen` class, matching `settings.svelte.ts`'s shape): `fullscreen.active` (`$state`),
  `fullscreen.toggle()`, `fullscreen.refresh()`.
- Deviation from this task's own wording: the unit test mocks
  `@tauri-apps/api/window` with `mockIPC`/`mockWindows` from `@tauri-apps/api/mocks`
  (`plugin:window|set_fullscreen`, `plugin:window|is_fullscreen`), not `vi.mock`. Grepped the
  repo first (`grep -rl "vi.mock('@tauri-apps"`) and found no such pattern anywhere — the one
  actual precedent for testing tauri-backed code (`assets.test.ts`) uses `@tauri-apps/api/mocks`,
  which is also the library's own documented way to intercept IPC in Vitest. Followed the real
  precedent over the task text.
- `+layout.svelte`: kept `zoomKeys` as-is and added a sibling `fullscreenKey`, combined by a
  `windowKeys(event)` dispatcher bound to the single `<svelte:window onkeydown>` (a Svelte
  element can only take one `onkeydown`). `onresize` calls `fullscreen.refresh()` directly.
- `TopBar.svelte`: button uses `maximize-2` / `minimize-2` from `@lucide/svelte/icons`
  (confirmed present in the installed package), `Button` variant `ghost` size `icon-sm` (the
  sidebar-trigger's shape), `ms-auto shrink-0`, gated on `!isMacos`, `aria-pressed`, and
  `title="Full screen (F11)"`.
- Not ticked: 2.1 carries a `Hand check:` line (Windows/macOS) — left `[ ]` per instruction.
  3.1 is the owner's.
- Gate run from repo root: `cargo build --manifest-path packages/app/src-tauri/Cargo.toml`
  succeeded; `pnpm -r typecheck` 0 errors; `pnpm lint` clean; `pnpm --filter @boorubox/app test`
  43 files / 475 tests passed. `mise run check` was not run (would race agent T's tree).
- Did not touch `packages/app/src/lib/domain/`, `components/library/`, `components/tags/`, or
  `api/search.svelte.ts` — those are agent T's in-progress, uncommitted changes, left as found.
