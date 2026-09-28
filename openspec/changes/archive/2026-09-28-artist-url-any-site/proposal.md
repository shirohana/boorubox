## Why

Pasting `https://kanibiimu.fanbox.cc/` into Edit artist is refused with "does not look like a
URL" (owner, 2026-09-28). The refusal is `artists::normalized` (`artist-entries` design D3, the
spec's "no path SHALL be refused"), whose argument was that a bare `x.com` would own every X
capture. That argument holds for a host whose identity lives in the path and not for one whose
identity is the subdomain: on Fanbox the host *is* the profile, so the rule refuses a valid
profile and names a reason that is not the reason. The owner will bring Fanbox images in through
local file import (the page shows a reduced image; the original opens in a new page the
extension cannot capture), so the entry's job is the artist record, not a match today.
Requirements §6 (artist tags), `artist-entries` spec.

## What Changes

- **A host-only URL is refused only for a host the app reads a path identity from** — today
  `x.com` and `pixiv.net`, derived from the profile-URL table rather than listed twice — and
  the refusal says so: the host alone would own every account there; add the account's path.
- **Every other URL that parses is stored as pasted**, host-only or not, and matches nothing
  until the app reads that site. When an adapter for it lands, its candidate normalises to the
  same string and the existing ownership check already matches it.
- **The dialog says what happens to such a URL**: one hint line under Profile URLs, so an
  unread site is not mistaken for a mistake.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `artist-entries`: which URLs an entry may own, and what the dialog says about a URL from a
  site the app does not read.

## Non-goals

- A Fanbox site adapter, or any capture from Fanbox (owner: local file import instead).
- Folding `name.fanbox.cc` and `fanbox.cc/@name` into one stored form: the choice of which to
  keep belongs to the adapter that will produce a candidate, and nothing consumes it before then.
- Listing the read sites in the dialog: the list lives in Rust and a UI copy of it drifts.
- Reading the site's public-suffix list to tell a subdomain from a registrable domain.
