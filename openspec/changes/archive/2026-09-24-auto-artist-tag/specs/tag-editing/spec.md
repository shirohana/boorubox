## MODIFIED Requirements

### Requirement: An X account on screen is a search term
For an image whose page address names an X account, the inspector SHALL show that account's
handle as a row of the image's facts, labelled Account, above the Page row — a fact of the
page, beside the page's other facts. The row SHALL show the handle as a button reading
`@handle`, padded and hover-highlighted so it looks clickable like the account bar it is, not
a plain fact to merely read (owner, 2026-09-23: the row did not look clickable). Acting on it
SHALL add `account:<handle>` to the tag search, and acting on it while the search already
names that account SHALL take the term out again; it SHALL show whether the search includes
or excludes it in the marking a tag uses. The handle shown SHALL be the one the search
matches: derived by the same rule from the same page address, so the entry never names an
account the search cannot find. The row SHALL be absent for an image whose page address names
no X account. The row SHALL NOT add or remove a tag: the handle reaches the image's tags only
as the artist tag a capture derives from its adapter record (`capture-ingest`, "A capture is
stored with its author as an artist tag").

Until 2026-09-23 this requirement kept the handle out of the tags altogether: with artist tags
in the vocabulary, a handle among the tags read as a second artist beside the one the owner
typed (owner, 2026-09-23). That was right while every artist tag was typed by hand and a
handle tag was a second name for the same person. It stopped being right when capture began
deriving the artist tag from the handle itself (`auto-artist-tag`): the handle among the tags
is then the one artist, not a second. The Account row keeps its place for its own reasons —
it is a fact of the page address, `account:` searches the address and not the tags, and it
is there for images captured before the derivation or whose artist tag was left off.

#### Scenario: Finding the same artist
- **WHEN** the panel shows an image captured from `https://x.com/alice/status/1` and the user acts on the Account row
- **THEN** the tag search reads `account:alice`, the result is every image whose page address names `alice`, and this image is still current

#### Scenario: Toggling off
- **WHEN** the search reads `cat account:alice` and the user acts on `alice`'s row
- **THEN** the search reads `cat`

#### Scenario: Where it sits
- **WHEN** the panel shows an image captured from `https://x.com/alice/status/1`
- **THEN** the facts list reads Title, Source, Account, Page, Image in that order

#### Scenario: The account and the artist tag side by side
- **WHEN** the panel shows an image captured from `https://x.com/Alice/status/1` whose adapter record names the handle `Alice`, and whose artist tag `alice` was created at capture
- **THEN** the Account row reads `@Alice`, the tags list `alice` once, as an artist tag, and acting on the Account row changes no tag

#### Scenario: Not an X page
- **WHEN** the panel shows an image captured from a Pixiv page or imported from a file
- **THEN** no Account row is shown

#### Scenario: X's own pages
- **WHEN** the panel shows an image whose page address is `https://x.com/home`
- **THEN** no Account row is shown
