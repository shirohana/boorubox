## Context

`artists::normalized` (`artists.rs`) is the one definition of a storable URL; `normalize_all`
turns its `None` into the `BadRequest` the dialog shows. `PROFILE_URLS` is the table of sites the
app builds a candidate profile URL from (`x` → `https://x.com/{}`, `pixiv` →
`https://www.pixiv.net/users/{}`); `owns` is "equal, or continues past a `/`". The dialog shows
Rust's refusal verbatim and keeps no validation of its own (its `save` doc comment).

## Decisions

**D1. Refuse a host-only URL only for a path-identity host, and derive that set from
`PROFILE_URLS`.** `path_identity_hosts()` maps each template's host through the same
canonicalisation `normalized` applies to a pasted host (lower-case, `www.`/`mobile.`/`m.`
dropped, `twitter.com` → `x.com`), which moves out of `normalized` into `canonical_host(host)
-> String` so the two readers cannot disagree. The set is `x.com`, `pixiv.net` today; the next
adapter with a path identity adds itself by adding its `PROFILE_URLS` row. The old rule's
argument ("a bare `x.com` would own every X capture") stays exactly true for exactly the hosts
it was about; a candidate the app produces always carries a path, so no stored entry can own a
whole site.

**D2. Two refusals, two messages: `normalized` returns `Result<String, UrlRefusal>`.**
`UrlRefusal::NotAUrl` (no host, a host without a dot, whitespace in the host) keeps the message
`"{url}" does not look like a URL`; `UrlRefusal::BareProfileHost(host)` reads
`"{url}" names {host} alone, which would own every {host} account; add the account's path`.
`normalize_all` maps either to `BadRequest` with that text. `candidate` and the tests that read
`None` use `.ok()`. The owner's ask was that a refusal name its reason; with a bare Fanbox host
accepted there is nothing left to refuse there, and the one refusal that remains beyond "not a
URL" now says why.

**D3. A host-only URL is stored as its canonical host with no path**, `kanibiimu.fanbox.cc`,
and `owns` is unchanged: `kanibiimu.fanbox.cc` owns itself and `kanibiimu.fanbox.cc/posts/1`,
never `kanibiimu.fanbox.cc.evil`. The trailing-slash rule already makes `…fanbox.cc/` and
`…fanbox.cc` one URL.

**D4. The dialog hint is static text, not a per-URL verdict.** Under the Profile URLs textarea:
"One per line. A URL from a site the app does not read yet is kept, and matched once it does."
Rust does not answer "is this site read" per URL, and the sentence is true for every accepted
URL, so nothing is computed and nothing can drift.

**D5. The reversal keeps its argument.** The archived `artist-entries` design D3 gets a dated
paragraph (2026-09-28): the empty-path refusal was right while every profile URL the app knew
carried its identity in the path, and stopped being right at the first subdomain-identity
site; the argument now applies to `path_identity_hosts()` only. The main spec's sentence is
amended by this change's delta.

## Risks

- A registrable domain pasted alone (`fanbox.cc`) is accepted and owns nothing today; if a
  Fanbox adapter ever produced a host-only candidate for `fanbox.cc` itself it would own it,
  which is the same "bare host" trap. The adapter's row in `PROFILE_URLS` is where that gets
  decided, and D1 makes the refusal follow the row.
