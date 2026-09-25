# artist-entries Specification

## Purpose
Artist entries, as Danbooru keeps them: an artist tag that owns profile URLs, so a capture
from a known artist carries the tag the owner chose rather than the handle or display name
the site shows, and a wrong artist tag is corrected once, where it is seen, for every image
and every later capture.

## Requirements

### Requirement: An artist entry is a tag that owns profile URLs
The library SHALL keep artist entries: an artist tag and the list of URLs it owns, as Danbooru
keeps an artist's URLs. A URL SHALL have one owner; giving a URL another artist owns SHALL be
refused naming that artist. URLs SHALL be compared normalised — scheme ignored, host
lower-cased with a leading `www.`, `mobile.` or `m.` dropped and `twitter.com` read as `x.com`,
query and fragment dropped, trailing slashes dropped, path lower-cased — and an entry's URL
SHALL own a URL that equals it or continues it past a `/`. A URL with no host, a host without a dot, or no path SHALL be
refused, and an entry naming a tag that exists under a category other than artist SHALL be
refused. Entries SHALL be stored with the library, carried in its describing file, and
restored on rebuild; a describing file written before entries existed SHALL restore none.

#### Scenario: A prefix owns the post
- **WHEN** `metaljelly` owns `https://x.com/metaljelly0811`
- **THEN** it owns `https://twitter.com/MetalJelly0811/status/123?s=20` and `http://www.x.com/metaljelly0811/` and not `https://x.com/metaljelly08110`

#### Scenario: One owner per URL
- **WHEN** `alice` owns `https://x.com/alice_art` and the user gives that URL to `bob`
- **THEN** the write is refused naming `alice`, and both entries are unchanged

#### Scenario: Entries survive a rebuild
- **WHEN** `metaljelly` owns two URLs and the library is rebuilt from its folder
- **THEN** `metaljelly` owns the same two URLs afterwards

### Requirement: An artist is renamed from the image that shows the wrong name
The inspector SHALL offer "Rename artist…" on a tag of the artist category, and nowhere else
(the sidebar's row and the pinned chip have no image to read URLs from; a tag of another
category is renamed through bulk edit — owner, 2026-09-25). The dialog SHALL show the new
name, the author's profile URL read from the image — the one URL the derivation matches, or
nothing for a record that names no author — editable, and how many images carry the old
name, trash included, before anything is written. Confirming SHALL, in one transaction, record
the new name as the owner of those URLs, move any URLs the old name owned to it, and retag
every carrier of the old name to the new one: when the new name is not yet a tag the old tag
row is renamed keeping its pinned group; when it is an artist tag already, the carriers are
merged into it, a pinned group moves to it if it has none, and the old row is removed, so no
carrier-less row of the old name remains. Renaming to a name that exists under a category
other than artist SHALL be refused with the reason, and only an artist tag SHALL be offered
for renaming. Each retagged image SHALL record the change
as its last change. The panel and the grid SHALL show the new name once it lands.

#### Scenario: Rename an X artist
- **WHEN** 120 images carry the artist tag `metaljelly0811` from X posts, and the user renames it to `metaljelly` keeping the prefilled `https://x.com/metaljelly0811`
- **THEN** the dialog said 120 images would be retagged, all 120 carry `metaljelly` and none carries `metaljelly0811`, `metaljelly` owns `https://x.com/metaljelly0811`, and the next capture from that handle carries `metaljelly`

#### Scenario: Merge into an existing artist
- **WHEN** `kantoku` is an artist tag on 10 images and pinned in group 2, `kantoku_(pixiv)` is on 3 images, and the user renames `kantoku_(pixiv)` to `kantoku`
- **THEN** 13 images carry `kantoku`, still pinned in group 2, and `kantoku_(pixiv)` is gone

#### Scenario: The name is taken by a character
- **WHEN** `miku` is a character tag and the user renames the artist `miku_draws` to `miku`
- **THEN** the rename is refused with the reason and nothing changes

#### Scenario: A trashed carrier
- **WHEN** one carrier of `alice_art` is in the trash when `alice_art` is renamed to `alice`
- **THEN** restoring it later shows `alice`

### Requirement: Entries are listed and edited in settings
The app SHALL list the artist entries in its settings, one per artist with its URLs, and offer
adding an artist with its URLs, editing an artist's URLs, and deleting an artist. The section
SHALL say that URL changes apply to future captures only and that images already in the
library keep their tags, and SHALL point at the inspector's rename for retagging. Deleting an
entry SHALL confirm, saying that future captures fall back to the handle or display name and
that no image changes. The list SHALL fit the settings page's width without a sideways scroll.

#### Scenario: Edit URLs
- **WHEN** the user adds `https://www.pixiv.net/users/3439325` to `metaljelly`'s URLs
- **THEN** the next Pixiv capture from user 3439325 carries `metaljelly`, and no stored image changes

#### Scenario: Delete an entry
- **WHEN** the user deletes `metaljelly`'s entry and confirms
- **THEN** the next capture from `metaljelly0811` carries `metaljelly0811`, and every image carrying `metaljelly` still does
