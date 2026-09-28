> One unit, Sonnet (Rust with a test gate; a two-line Svelte hint). Design D1–D5 decide every
> shape; do not re-decide them. Gate: `mise run check` (the unit owns Rust). No unit commits or
> ticks a hand check.

## 1. Unit A — host-only URLs (`packages/app/src-tauri`, `packages/app`)

- [x] 1.1 `artists.rs`: `canonical_host`, `path_identity_hosts`, `UrlRefusal` with its two
      messages, `normalized -> Result<String, UrlRefusal>` per D1–D3; `normalize_all` and
      `candidate` follow. Tests (replace `a_host_with_no_path_is_none`): `a_bare_path_identity_host_is_refused_naming_it`
      (`https://x.com/`, `x.com`, `https://www.pixiv.net` → `BareProfileHost`, message contains
      `x.com` / `pixiv.net` and "add the account's path"), `a_host_only_url_elsewhere_is_stored_as_its_host`
      (`https://kanibiimu.fanbox.cc/` → `kanibiimu.fanbox.cc`; `KANIBIIMU.fanbox.cc` the same),
      `a_host_only_entry_owns_its_own_pages_and_not_a_longer_host`
      (`owns("kanibiimu.fanbox.cc", "kanibiimu.fanbox.cc/posts/1")` true; `…fanbox.cc.evil` false),
      `path_identity_hosts_follow_the_profile_url_table` (exactly `x.com`, `pixiv.net`),
      `upsert_accepts_a_fanbox_profile` (both `https://kanibiimu.fanbox.cc/` and
      `https://www.fanbox.cc/@kanibiimu` save on one entry, listed back normalised), and
      `upsert_refuses_an_empty_tag_and_a_url_that_does_not_normalise` keeps a not-a-URL case.
- [x] 1.2 `ArtistDialog.svelte`: the hint line under Profile URLs per D4, `text-xs
      text-muted-foreground`, the same tone as the dialog's other helper text.
- [x] 1.3 `openspec/changes/archive/2026-09-25-artist-entries/design.md` D3: the dated
      paragraph per D5. `normalized`'s doc comment carries the D1 argument in its own words.
- [x] 1.4 Gate green. Handoff below: final signatures and anything the hand check should watch.
- [ ] 1.5 Hand check (owner): Edit artist on a real tag, paste `https://kanibiimu.fanbox.cc/`
      and `https://www.fanbox.cc/@kanibiimu` on two lines, Save; reopen and see both listed
      normalised; paste `x.com` alone and read the refusal.

## Handoff

**Signatures** (`packages/app/src-tauri/src/artists.rs`):

```rust
pub fn normalized(url: &str) -> std::result::Result<String, UrlRefusal>

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UrlRefusal {
    NotAUrl,
    BareProfileHost(String),
}
impl UrlRefusal {
    pub fn message(&self, url: &str) -> String
    // NotAUrl:          "{url:?} does not look like a URL"
    // BareProfileHost:  "{url:?} names {host} alone, which would own every {host} account;
    //                    add the account's path"
}

pub(crate) fn canonical_host(host: &str) -> String
pub(crate) fn path_identity_hosts() -> std::collections::HashSet<String>
```

`normalized`'s `Result` is written as `std::result::Result<String, UrlRefusal>`, not the bare
`Result<String, UrlRefusal>` the task text uses: this module's `use crate::error::{AppError,
Result}` shadows the name `Result` with `error::Result<T> = std::result::Result<T, AppError>`,
a one-parameter alias, so the two-parameter form fails to compile (`E0107`) under the shadowed
name. Everywhere else in the file — `normalize_all`, `candidate`, `apply_preview`, and every
test — reads the `Result`/`Option` as before; only `normalized`'s own signature needed the
`std::` prefix. No other deviation from D1–D5.

`normalize_all` turns a refusal into `AppError::BadRequest(refusal.message(url))` — the message
text lives on `UrlRefusal` alone, so `upsert`'s and `rename`'s refusals and any future caller
read the same wording. `candidate` and `apply_preview` read `normalized(...).ok()`.
`path_identity_hosts()` recomputes its `HashSet` from `PROFILE_URLS` on each call (two entries
today); nothing calls it often enough to want it cached.

**Hand check:** the owner's own task 1.5, untouched here. Two things worth watching there
beyond the script as written: the refusal text for a bare `x.com` reads "…would own every x.com
account; add the account's path" (D2's exact wording, not paraphrased); and a bare host for a
site the app doesn't read at all (something other than `kanibiimu.fanbox.cc` — e.g. pasting
just `example.com`) should also save silently, matching nothing, same as the Fanbox case.

**Gate:** `mise run lint`, `mise run typecheck`, `mise run clippy`, and `pnpm test` (837 app +
96 extension + 1 shared tests) are all green with this change alone. `cargo test` for this
module is green (`cargo test --manifest-path packages/app/src-tauri/Cargo.toml artists::` →
71 passed, 0 failed). The full `mise run check` / whole-crate `cargo test` does **not** pass
right now: `query::tests::free_text_is_data_not_fts_syntax` fails in `packages/app/src-tauri/
src/query.rs`, which a different in-flight change (`text-search-substring`, substring/trigram
free-text search) is mid-editing concurrently — that file and `db.rs` are explicitly not this
unit's to touch. Nothing in that diff or failure mentions `artists`, `UrlRefusal`, or any name
this unit added; re-run `mise run check` once the query.rs/db.rs unit lands to confirm the full
gate is green.
