> One Sonnet agent for group 1 (Rust + shared type), the lead for group 2 (webview) and the
> artifacts, in parallel — disjoint files. Gate: `cargo fmt`, `cargo clippy --all-targets -- -D
> warnings`, `cargo test` in `packages/app/src-tauri`; `pnpm -r typecheck`, `pnpm lint`,
> `pnpm --filter @boorubox/app test`; the lead runs `mise run check` on the joined tree.

## 1. The count (agent — `query.rs`, `model.rs`, `packages/shared/src/index.ts`, test fixtures)

- [x] 1.1 `AccountClause`, `Plan::for_account_counts`, `account_counts`, `view_predicate`,
      `TagCounts.accounts` on both sides per design D1, with the four tests the brief names.
      Verify: the gate.

## 2. The rail (lead — `AccountRail.svelte`, `LibraryScreen.svelte`)

- [x] 2.1 `AccountRail` takes `accounts: GroupSlice[] | null`, orders the search's own first,
      keeps the last rows while `null`; `LibraryScreen` passes `results.counts?.accounts`.
      Verify: typecheck, lint, tests; `mise run check` on the joined tree.
      Hand check: group by X account, include one account — every other account stays
      listed with its own count, the included one first and marked; add a tag — the counts
      change and zero-count accounts stay; scroll the rail and include an account near the
      end — the rail does not jump to its top.
