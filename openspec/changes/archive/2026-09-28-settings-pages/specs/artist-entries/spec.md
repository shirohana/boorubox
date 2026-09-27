## MODIFIED Requirements

### Requirement: Entries are listed and edited in settings
The app SHALL list the artist entries on the Artists page of Settings, one per artist with its URLs, and offer
adding an artist with its URLs, editing an artist's URLs, and deleting an artist. The section
SHALL say that URL changes apply to future captures only and that images already in the
library keep their tags, and SHALL point at the inspector's rename for retagging. Deleting an
entry SHALL confirm, saying that future captures fall back to the handle or display name and
that no image changes. The list SHALL fit the settings page's width without a sideways scroll.

A filter field above the list SHALL narrow it, as the user types, to the entries whose tag or
one of whose URLs, as listed, contains the typed text, ignoring case; an empty field SHALL show
every entry. When no entry matches, the page SHALL say so, naming the text. The filter SHALL
change only what is listed: adding, editing and deleting SHALL work as they do unfiltered. It
is not remembered: the page shows every entry when it is opened again.

#### Scenario: Edit URLs
- **WHEN** the user adds `https://www.pixiv.net/users/3439325` to `metaljelly`'s URLs
- **THEN** the next Pixiv capture from user 3439325 carries `metaljelly`, and no stored image changes

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
