# tag-editing Specification

## Purpose
Giving one image its tags: an editor beside the image that suggests what the library already
uses, keeps the set clean and sorted, and turns any tag on screen into a search term.

## Requirements

### Requirement: The tags of one image can be edited
The app SHALL let the user change the tags of the image currently shown in the inspector,
from both of the inspector's placements, and SHALL store the result as the image's whole tag
set. Saving SHALL drop duplicates and blank entries, SHALL lowercase every name (so `Cat` and
`cat` in one editor are one tag), SHALL leave the set unordered but present it in one order
everywhere it is displayed, and SHALL record the image as changed at that moment. An empty
editor SHALL be a valid save that leaves the image with no tags. The new tags SHALL be visible
in the grid, the inspector and the tag list of the current results without reopening the
library.

The tag area SHALL be read-only until the user opens the editor with an edit action; saving
or cancelling SHALL close it again, and a refused save SHALL keep it open with the typed
text. The editor SHALL be a field that grows with its text, not a single line: an image
carries dozens of tags and a line that scrolls sideways shows a few of them. It SHALL open
with the image's tags on one line per category that has any, in the app's one category order
— artist, copyright, character, general, meta — alphabetical within a line, and with a space
after the last tag, and the caret SHALL be placed after that space when the editor opens, so
the editor opens ready to type a new token — a click into the editor means a tag is about to
be added, and the owner had been typing that space by hand on every edit (2026-09-12) and
clicking to the end after (2026-09-23). The lines are presentation: a tag's category comes
from the vocabulary and SHALL NOT be changed by the line it is typed or moved to, and a line
break SHALL count as a space. The space is not a change: a save trims.

#### Scenario: Opening the editor to add a tag
- **WHEN** the user opens the editor of an image tagged `cat` and `dog`
- **THEN** the field reads `cat dog ` with the caret at its end, ready to type a third tag

#### Scenario: Read-only until opened
- **WHEN** the panel describes an image and the user has not used the edit action
- **THEN** the tags are shown as text and no field is on screen

#### Scenario: One line per category
- **WHEN** the user opens the editor of an image tagged `kantoku` (artist), `1girl`, `highres` (meta) and `azur_lane` (copyright)
- **THEN** the field reads `kantoku`, `azur_lane`, `1girl`, `highres ` on four lines, in that order

#### Scenario: A line does not categorise
- **WHEN** the user moves `1girl` onto the artist line and saves
- **THEN** `1girl` is still a general tag

#### Scenario: Adding tags
- **WHEN** the user opens the editor of an image tagged `cat`, types `dog`, and saves
- **THEN** the image carries `cat` and `dog`, the grid tile and the inspector show both, and the tag list of the results counts `dog` one higher

#### Scenario: The same tag twice
- **WHEN** the user saves `cat cat dog`
- **THEN** the image carries `cat` and `dog`

#### Scenario: Typed in capitals
- **WHEN** the user saves `Cat DOG`
- **THEN** the image carries `cat` and `dog`, and the editor reopens reading `cat dog `

#### Scenario: Clearing every tag
- **WHEN** the user empties the editor and saves
- **THEN** the image carries no tags and is still in the library

#### Scenario: Editing from the full-size viewer
- **WHEN** the user opens the inspector inside the full-size view, edits the tags and saves
- **THEN** the same result as from the grid's inspector

#### Scenario: An edit that names no image
- **WHEN** the panel describes no image
- **THEN** no tag editor is on screen

### Requirement: A rating written as a tag sets the rating
A tag of the form `rating:g`, `rating:s`, `rating:q` or `rating:e` SHALL set the image's
rating rather than being stored as a tag, matching what the legacy library did with the same
text. Any other tag beginning with `rating:` SHALL be stored as an ordinary tag.

#### Scenario: Rating typed among the tags
- **WHEN** the user saves the tags `cat rating:s`
- **THEN** the image carries the tag `cat`, no tag named `rating:s`, and its rating is `s`

#### Scenario: Not a rating
- **WHEN** the user saves a tag `rating:unknown`
- **THEN** it is stored as a tag and the image's rating is unchanged

### Requirement: The editor suggests tags the library already uses
While a tag is being typed the app SHALL offer tags already used in the library that contain
what has been typed, ranked: the tag equal to the typed text first, then tags that begin with
it, then the rest, each band most used first and then by name (owner, 2026-09-24: tags are
compounds like `cute_cat` whose remembered half is the tail). It SHALL exclude tags already
named elsewhere in the input, but never the word being typed itself, so a whole typed tag is
offered and highlighted and one confirmation finishes it. Each suggestion SHALL be drawn in
its category's colour. It SHALL NOT offer anything while the token being typed is a metatag
or the `or` operator of the query language, nor while it begins with a category prefix.
Accepting a suggestion SHALL replace the token being typed, preserving a leading `-` when
the token has one.

#### Scenario: Prefix first
- **WHEN** the library uses `cat`, `cathedral`, `cute_cat` and `dog`, and the user types `cat`
- **THEN** the list reads `cat`, `cathedral`, `cute_cat`, and `dog` is not offered

#### Scenario: A whole tag confirms
- **WHEN** the library uses `cat` and `cathedral`, the user types `cat` and presses Enter
- **THEN** the input reads `cat ` with the caret after the space, and `cathedral` was not inserted

#### Scenario: Substring
- **WHEN** the library uses `cute_cat` and `strong_cat`, and the user types `cat`
- **THEN** both are offered

#### Scenario: Already typed
- **WHEN** the input already holds `cat` and the user starts typing `cat` again
- **THEN** `cat` is not offered

#### Scenario: Inside a metatag
- **WHEN** the token being typed is `rating:` or `or`
- **THEN** no suggestions appear

#### Scenario: Inside a category count metatag
- **WHEN** the token being typed is `copytags:` or `arttags:>`
- **THEN** no suggestions appear

#### Scenario: Behind a category prefix
- **WHEN** the token being typed is `artist:kan`
- **THEN** no suggestions appear

#### Scenario: Accepting into an exclusion
- **WHEN** the token being typed is `-cathe` and the suggestion `cathedral` is accepted
- **THEN** the token becomes `-cathedral`

### Requirement: Confirming a tag takes two steps
Confirming SHALL first act on what is pending, then submit: while a suggestion is highlighted
the first confirmation SHALL accept it and finish the token — a space after the tag, or none
if one is already there — leaving the input ready for the next tag; with an unfinished token
and no highlighted suggestion the first confirmation SHALL finish that token the same way;
with nothing pending it SHALL submit the input. Dismissing the suggestions SHALL leave the
typed text untouched.

#### Scenario: Accept, then submit
- **WHEN** the user types a prefix that highlights a suggestion and confirms twice without typing anything else
- **THEN** the first confirmation inserts the suggested tag followed by a space, and the second submits the input

#### Scenario: A tag the library does not have
- **WHEN** the user types a tag no image carries and confirms twice
- **THEN** the typed tag is kept as written and the input is submitted

#### Scenario: Dismissing
- **WHEN** suggestions are showing and the user dismisses them
- **THEN** the list closes, the typed text is unchanged, and the next confirmation submits

### Requirement: A tag can be removed without retyping the set
The app SHALL offer a remove action on each tag shown for an image, and SHALL apply it to
that image alone.

#### Scenario: Remove one tag
- **WHEN** the user removes one tag of an image carrying three
- **THEN** that image carries the other two, no other image changes, and the image's last-changed time is now

### Requirement: Tags on screen are search terms
Every tag shown for an image SHALL be usable as a search term: acting on it SHALL add it to
the tag search, and acting on a tag the search already includes SHALL take it out again. The
app SHALL also offer excluding it, which SHALL add it as an exclusion instead. A tag added or
removed this way SHALL leave the rest of the query intact. A pinned tag's chip SHALL offer
the same two actions from its context menu, in both placements of the panel and over a
selection, since the chip is a tag on screen like any other (owner, 2026-09-24).

Each tag shown for an image SHALL be drawn as plain text in its category's colour, not as a
pill: a pill's own fill competes with the marking that says the tag is in the search (owner,
2026-09-23). Each SHALL show whether the search includes it or excludes it, in the one
marking the tag sidebar uses, so the list reads as a set of toggles; that marking SHALL be a
background on the tag's whole box, with no underline and no strike-through — an underline
makes English hard to read, and a tag already carrying its category's colour struck through
is one colour too many (owner, 2026-09-23). The marking SHALL sit beside the tag's category
colour, not replace it; an included tag SHALL also carry extra weight, the one change of
weight the marking makes, since a background alone is easy to miss as the only cue once the
underline is gone. The tags SHALL be shown grouped by category in the app's one category
order, alphabetical within a group.

Acting on a tag from the inspector SHALL keep the image the panel describes as the current
image: after the search re-runs, that image SHALL be current at whatever row it now occupies,
and the panel SHALL still describe it. Only when the image is no longer in the result SHALL the
screen fall back to no current image.

#### Scenario: Click to narrow
- **WHEN** the search is empty and the user acts on the tag `cat`
- **THEN** the tag search reads `cat` and the results are the images tagged `cat`

#### Scenario: Click again to widen
- **WHEN** the search reads `cat dog` and the user acts on `cat`
- **THEN** the search reads `dog`

#### Scenario: Exclude
- **WHEN** the search reads `cat` and the user excludes `dog`
- **THEN** the search reads `cat -dog`

#### Scenario: Searching from a pinned chip
- **WHEN** `tagme` is pinned, the search is empty, and the user opens the chip's context menu and chooses "Search for this tag"
- **THEN** the search reads `tagme`, and no image's tags changed

#### Scenario: Excluding from a pinned chip
- **WHEN** `tagme` is pinned, the search reads `cat`, and the user chooses "Exclude from the search" on the chip
- **THEN** the search reads `cat -tagme`

#### Scenario: The rest of the query survives
- **WHEN** the search reads `cat rating:s is:png` and the user adds `dog`
- **THEN** the search still carries the rating and file-type terms and now also `dog`

#### Scenario: The tag shows it is active
- **WHEN** the search reads `cat -dog` and the panel shows an image tagged `cat`, `dog` and `bird`
- **THEN** `cat` is marked as included, `dog` as excluded, and `bird` as neither; `dog` and `bird` are the same weight and `cat` is heavier

#### Scenario: Active and coloured
- **WHEN** the search reads `kantoku` and the panel shows an image tagged `kantoku` (artist)
- **THEN** the text carries both the included marking and the artist colour

#### Scenario: The inspected image stays current
- **WHEN** the panel describes an image tagged `cat` at row 40 of an empty search, and the user acts on `cat`
- **THEN** the search reads `cat`, the same image is current at its row in the new result, the grid has scrolled to it, and the panel still describes it

#### Scenario: The inspected image leaves the result
- **WHEN** the panel describes an image tagged `cat` and the user excludes `cat`
- **THEN** the search reads `-cat` and no image is current

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
