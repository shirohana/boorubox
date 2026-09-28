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

### Requirement: An artist is edited from any of its tags
Every context menu of a tag of the artist category — the sidebar's row, the pinned chip and
the inspector's badge — SHALL offer "Edit artist…", and no tag of another category SHALL
offer it (a tag of another category is renamed through bulk edit — owner, 2026-09-25). The
dialog SHALL show, before anything is written: the name, prefilled with the tag; the profile
URLs the tag's entry owns, editable, one per line, empty for a tag with no entry; when opened
from an image that yields a profile URL that no entry owns yet, that URL appended as
the last line; and how many images carry the tag, trash included. A profile URL an entry
already owns — this one or another — SHALL NOT be appended. The confirming button SHALL read
"Save" while the name is unchanged and "Rename N images" once it differs, and SHALL be
unavailable while neither the name nor the URLs differ from what the dialog opened with.

Saving under the unchanged name SHALL replace the entry's URLs with the dialog's, creating the
entry if the tag had none, and SHALL change no image. Saving under a new name SHALL, in one
transaction, record the new name as the owner of the dialog's URLs, move any URLs the old name
owned to it, and retag every carrier of the old name to the new one: when the new name is not
yet a tag the old tag row is renamed keeping its pinned group; when it is an artist tag
already, the carriers are merged into it, a pinned group moves to it if it has none, and the
old row is removed, so no carrier-less row of the old name remains. A URL removed from the
lines while renaming still moves with the name; removing it takes a second save. Renaming to a
name that exists under a category other than artist SHALL be refused with the reason, and a
URL another artist owns SHALL be refused naming that artist. Each retagged image SHALL record
the change as its last change. The panel, the sidebar and the grid SHALL show the result once
it lands.

#### Scenario: Rename an X artist
- **WHEN** 120 images carry the artist tag `metaljelly0811` from X posts, and the user opens Edit artist from one of them, renames it to `metaljelly` keeping the appended `https://x.com/metaljelly0811`
- **THEN** the dialog said 120 images and its button read "Rename 120 images", all 120 carry `metaljelly` and none carries `metaljelly0811`, `metaljelly` owns `https://x.com/metaljelly0811`, and the next capture from that handle carries `metaljelly`

#### Scenario: Add a second account from the sidebar
- **WHEN** `metaljelly` owns `https://x.com/metaljelly0811` and the user opens Edit artist on `metaljelly`'s sidebar row, adds `https://x.com/metaljelly_sub` as a second line and saves
- **THEN** the button read "Save", `metaljelly` owns both URLs, no image changed, and the next capture from `metaljelly_sub` carries `metaljelly`

#### Scenario: The image's own account is offered
- **WHEN** `metaljelly` owns `https://x.com/metaljelly0811` and the user opens Edit artist on the `metaljelly` badge of an image captured from `@metaljelly_sub`
- **THEN** the URLs read `https://x.com/metaljelly0811` then `https://x.com/metaljelly_sub`, and saving makes `metaljelly` own both

#### Scenario: Nothing to save
- **WHEN** the user opens Edit artist from a sidebar row and changes nothing
- **THEN** the confirming button reads "Save" and cannot be pressed

#### Scenario: Merge into an existing artist
- **WHEN** `kantoku` is an artist tag on 10 images and pinned in group 2, `kantoku_(pixiv)` is on 3 images, and the user renames `kantoku_(pixiv)` to `kantoku`
- **THEN** 13 images carry `kantoku`, still pinned in group 2, and `kantoku_(pixiv)` is gone

#### Scenario: The name is taken by a character
- **WHEN** `miku` is a character tag and the user renames the artist `miku_draws` to `miku`
- **THEN** the rename is refused with the reason and nothing changes

#### Scenario: A trashed carrier
- **WHEN** one carrier of `alice_art` is in the trash when `alice_art` is renamed to `alice`
- **THEN** restoring it later shows `alice`

#### Scenario: Not an artist
- **WHEN** the user opens the context menu of the general tag `solo` anywhere
- **THEN** no "Edit artist…" item is offered

### Requirement: Entries are listed and edited in settings
The app SHALL list the artist entries on the Artists page of Settings, one per artist with its URLs, and offer
adding an artist with its URLs, editing an artist's URLs, deleting an artist, and applying an
artist to the images already stored (the requirement "An entry is applied to stored images on
request"). The section SHALL say that URL changes apply to future captures only unless the
entry is applied, and that applying only adds the artist's tag. Applying from settings SHALL
confirm first, naming how many stored images come from the entry's URLs and how many of them
carry no artist tag, with a button reading "Apply to N images". Deleting an entry SHALL
confirm, saying that future captures fall back to the handle or display name and that no image
changes. The list SHALL fit the settings page's width without a sideways scroll.

A filter field above the list SHALL narrow it, as the user types, to the entries whose tag or
one of whose URLs, as listed, contains the typed text, ignoring case; an empty field SHALL show
every entry. When no entry matches, the page SHALL say so, naming the text. The filter SHALL
change only what is listed: adding, editing, deleting and applying SHALL work as they do
unfiltered. It is not remembered: the page shows every entry when it is opened again.

#### Scenario: Edit URLs
- **WHEN** the user adds `https://www.pixiv.net/users/3439325` to `metaljelly`'s URLs
- **THEN** the next Pixiv capture from user 3439325 carries `metaljelly`, and no stored image changes

#### Scenario: Apply from settings
- **WHEN** `metaljelly` owns `https://www.pixiv.net/users/3439325`, 8 stored images come from that Pixiv user, 5 carry the artist tag `name@お仕事募集中` and 3 carry no artist tag, and the user chooses Apply on `metaljelly`
- **THEN** the confirmation says 8 images, 3 of them with no artist tag, the button reads "Apply to 8 images", and confirming leaves all 8 carrying `metaljelly` and the 5 still carrying `name@お仕事募集中`

#### Scenario: Delete an entry
- **WHEN** the user deletes `metaljelly`'s entry and confirms
- **THEN** the next capture from `metaljelly0811` carries `metaljelly0811`, and every image carrying `metaljelly` still does

#### Scenario: Filter by tag
- **WHEN** the entries are `metaljelly`, `alice` and `bob_art`, and the user types `BOB` into the filter
- **THEN** only `bob_art` is listed

#### Scenario: Filter by URL
- **WHEN** `metaljelly` owns `x.com/metaljelly0811` and the user types `0811`
- **THEN** `metaljelly` is listed with its URLs

#### Scenario: Nothing matches
- **WHEN** the user types `zzz` and no entry's tag or URL contains it
- **THEN** no entry is listed and the page says that nothing matches `zzz`

#### Scenario: Cleared filter
- **WHEN** the user empties the filter field
- **THEN** every entry is listed again

### Requirement: An entry shows its tag's note
The Artists list in Settings SHALL show an artist tag's note, when it carries one, under the
tag and above its URLs, muted and wrapped as written. An entry whose tag carries no note SHALL
show nothing there. The note is edited from the tag's own context menu, not from this list
(owner, 2026-09-28: the note records which Danbooru artist tag a handle was confirmed as).

#### Scenario: A confirmed artist
- **WHEN** `metaljelly` owns `x.com/metaljelly0811` and carries the note `danbooru: metaljelly (confirmed)`
- **THEN** its entry reads `metaljelly`, then the note, then `x.com/metaljelly0811`

#### Scenario: No note
- **WHEN** `alice` owns a URL and carries no note
- **THEN** its entry reads `alice`, then its URLs, with nothing between

### Requirement: The inspector names the artist who owns the author
For an image that yields a profile URL — an X record's handle, a Pixiv record's user id, or
for an image stored with no record at all (a legacy-bundle import) the X account its page URL
names, read by the rule the Account row reads it by — the inspector SHALL show a row of the
image's facts labelled Artist, directly above the Account row's place. An image with a record
SHALL NOT be matched by its page URL: the page is the tab's address, not the author's. When an entry owns that URL, the row SHALL show the
entry's tag in the artist colour, and acting on it SHALL toggle that tag in the tag search
exactly as acting on a tag badge does, showing the same included or excluded marking. When
no entry owns it, the row SHALL offer "Create artist…". The row SHALL be absent for an image
that yields no profile URL: another site, a file import, a Pixiv capture stored before the
record carried the user id, or an image with no record whose page is not an X account's. The row SHALL be read for the image the panel shows
when it changes, and again after an artist is saved from the dialog, and an answer for an
image the panel has since left SHALL NOT be shown. The row SHALL NOT add or remove a tag
until the user confirms a dialog.

#### Scenario: An owned X account
- **WHEN** `metaljelly` owns `https://x.com/metaljelly0811` and the panel shows an image whose record names the handle `MetalJelly0811`
- **THEN** the Artist row reads `metaljelly` in the artist colour, and acting on it adds `metaljelly` to the search

#### Scenario: An owned Pixiv user
- **WHEN** `metaljelly` owns `https://www.pixiv.net/users/3439325` and the panel shows a Pixiv capture whose record carries `userId` `3439325`
- **THEN** the Artist row reads `metaljelly`

#### Scenario: Nobody owns it yet
- **WHEN** no entry owns `https://x.com/alice_art` and the panel shows an image whose record names the handle `alice_art`
- **THEN** the Artist row offers "Create artist…"

#### Scenario: An old Pixiv capture
- **WHEN** the panel shows a Pixiv capture whose record has no `userId`
- **THEN** no Artist row is shown

#### Scenario: A legacy import from X
- **WHEN** no entry owns `https://x.com/alice_art` and the panel shows an image imported from a legacy bundle, stored with no record, whose page URL is `https://x.com/Alice_Art/status/1/photo/1`
- **THEN** the Artist row offers "Create artist…", and the dialog it opens prefills `alice_art` and `https://x.com/alice_art`

#### Scenario: A legacy import off an account page
- **WHEN** the panel shows an image stored with no record whose page URL is `https://x.com/home`
- **THEN** no Artist row is shown

#### Scenario: Moving on before the answer
- **WHEN** the panel shows image A, then image B before A's row was read
- **THEN** the row shows B's answer and never A's

### Requirement: An artist is created from the image's author
"Create artist…" SHALL open the artist dialog with the name prefilled with the artist tag a
capture from this record derives (the handle or display name) and the URLs with the record's
profile URL, and SHALL say, before anything is written, how many stored images, trash
excluded, come from those URLs, how many of them carry no artist tag at all, and that saving
tags all of them with the name; the line SHALL follow the URLs as they are edited. When the
chosen name differs from the derived tag and the derived tag already exists as an artist tag,
the dialog SHALL also say, before anything is written, that the derived tag is renamed to the
chosen name and on how many images, and confirming SHALL first rename the derived tag onto the
chosen name (the rename of "Edit artist": its carriers, trash included, are retagged and its
URLs move) — otherwise the handle tag would stay beside the chosen name on every image a
capture already tagged, and apply only ever adds. Otherwise confirming SHALL record the entry —
adding the URLs to the entry the name already has, when it has one, rather than replacing them.
Either way it SHALL then apply the entry to the stored images (the requirement "An entry is
applied to stored images on request"). A refusal of the entry or the rename SHALL leave every
image unchanged.

#### Scenario: Create from an X image
- **WHEN** no entry owns `https://x.com/alice_art`, 14 stored images come from `@alice_art` of which 12 carry the derived `alice_art` and 2 carry no artist tag, and the user chooses Create artist… and confirms the prefilled name
- **THEN** the dialog said 14 images with 2 carrying no artist tag, `alice_art` owns `https://x.com/alice_art`, all 14 carry `alice_art`, and the Artist row now reads `alice_art`

#### Scenario: Create under another name
- **WHEN** the user creates `alice` from an image of `@alice_art`, no `alice_art` artist tag exists, and `alice` already owns `https://www.pixiv.net/users/42`
- **THEN** `alice` owns both URLs, and the images from `@alice_art` carry `alice` beside whatever artist tag they already carried

#### Scenario: Create under another name renames the derived tag
- **WHEN** 9 images, one of them in the trash, carry the artist tag `alice_art`, and the user creates `alice` from an image of `@alice_art`
- **THEN** before confirming the dialog said `alice_art` is renamed to `alice` on 9 images; after it, all 9 carry `alice`, no image carries `alice_art`, `alice` owns `https://x.com/alice_art`, and the stored images from `@alice_art` outside the trash that carried neither tag carry `alice` too

#### Scenario: A taken URL
- **WHEN** the user creates an artist whose URLs include one `bob` owns
- **THEN** the save is refused naming `bob`, and no image changes

### Requirement: An entry is applied to stored images on request
Applying an artist entry SHALL add its tag, as an artist tag, to every stored image outside
the trash whose profile URL the entry owns — its record's, or for an image stored with no
record the X account its page URL names — keeping every tag the image already has;
an image already carrying the tag SHALL be left unchanged, so applying twice changes nothing
the second time. Images that yield no profile URL SHALL NOT be touched. The
application SHALL be one transaction, SHALL record the change as each tagged image's last
change, and SHALL answer with how many images it tagged and how many it left because they
already carried the tag. Applying SHALL happen only when the user asks for it — from Create
artist, from the Edit artist dialog's "Apply to existing images" choice, which starts unchecked
and shows the same counts line as Create artist, or from settings; saving an entry otherwise
changes no image (owner, 2026-09-28: no migration by default).

#### Scenario: Only the owner's images
- **WHEN** `alice` owns `https://x.com/alice_art` and is applied, and the library holds images from `@alice_art`, from `@alice_art2`, and from a Danbooru page
- **THEN** only the images from `@alice_art` gain `alice`

#### Scenario: Additive and idempotent
- **WHEN** an image from `@alice_art` carries `alice_art` and `cat`, and `alice` is applied twice
- **THEN** after the first it carries `alice_art`, `cat` and `alice`, the answer counted it as tagged, and the second answer counted it as already carrying the tag

#### Scenario: The trash is left alone
- **WHEN** an image from `@alice_art` is in the trash when `alice` is applied
- **THEN** it does not gain `alice`, and restoring it shows its tags as they were

#### Scenario: Editing without applying
- **WHEN** the user adds a URL in Edit artist and saves with "Apply to existing images" unchecked
- **THEN** no stored image changes
