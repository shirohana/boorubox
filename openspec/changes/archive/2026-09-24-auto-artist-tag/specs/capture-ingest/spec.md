## ADDED Requirements

### Requirement: A capture is stored with its author as an artist tag
An image entering the library with an adapter record SHALL be stored carrying one artist tag
named after the author the record names: for a record from `x`, its `handle`; for a record
from `pixiv`, its `artist` (the display name). The name SHALL be spelled the way every tag is:
lower-cased, surrounding whitespace dropped, and every run of whitespace inside it read as one
`_`. A record from any other site, a record without that field, or a field that is not text or
is empty once spelled, SHALL add no artist tag.

When no tag of that name exists, it SHALL be created as an artist tag. When a tag of that name
exists as an artist tag, the image SHALL carry it. When a tag of that name exists under any
other category — general included — the image SHALL NOT carry it at all: the derived tag is
left off rather than linked under the existing category, and the existing tag is unchanged.
The tags the image's source supplied and the enabled rules' tags SHALL be settled before the
artist tag is, so a rule naming the same name in the same capture decides its category.

The artist tag SHALL NOT decide the answer to the capture: a capture that would be stored
without it SHALL be stored, and answered 2xx, whether the tag was created, carried or left off.
An image imported from the legacy extension's export bundle SHALL NOT gain an artist tag, and
a delivery whose `id` is already stored SHALL NOT gain one.

#### Scenario: From an X post
- **WHEN** a capture arrives with the record `{ site: "x", fields: { handle: "Alice_Art", displayName: "Alice ✿" } }` and no tag `alice_art` exists
- **THEN** the stored image carries `alice_art`, and `alice_art` is an artist tag

#### Scenario: From a Pixiv artwork, a name with spaces and capitals
- **WHEN** a capture arrives with the record `{ site: "pixiv", fields: { artist: "Some  Artist" } }` and no tag `some_artist` exists
- **THEN** the stored image carries `some_artist`, an artist tag

#### Scenario: A site with no author field the app reads
- **WHEN** a capture arrives with an adapter record naming a site other than `x` or `pixiv`
- **THEN** the image is stored with no artist tag

#### Scenario: The field is missing
- **WHEN** a capture arrives with the record `{ site: "pixiv", fields: { workId: "123" } }`
- **THEN** the image is stored with no artist tag

#### Scenario: No adapter record
- **WHEN** a capture or a local import arrives with no adapter record
- **THEN** the image is stored with no artist tag

#### Scenario: The name is already a general tag
- **WHEN** `alice` is a general tag and a capture arrives from X with the handle `alice`
- **THEN** the capture is stored, the image does not carry `alice`, and `alice` is still general

#### Scenario: The name is already a character tag
- **WHEN** `miku` is a character tag and a capture arrives from Pixiv with the artist `Miku`
- **THEN** the capture is stored, the image does not carry `miku`, and `miku` is still a character tag

#### Scenario: The name is already an artist tag
- **WHEN** `kantoku` is an artist tag and a capture arrives from Pixiv with the artist `Kantoku`
- **THEN** the image carries `kantoku`, still an artist tag, and no new tag is created

#### Scenario: A rule names the same name in the same capture
- **WHEN** no tag `alice` exists, an enabled rule matching `alice` adds the plain tag `alice`, and a capture arrives from X with the handle `alice`
- **THEN** the image carries `alice` once, and `alice` is general

#### Scenario: A legacy bundle import
- **WHEN** an image is imported from the legacy extension's export bundle
- **THEN** it gains no artist tag, whatever its record says

#### Scenario: Retrying a delivery
- **WHEN** a capture from X is delivered again with an id already stored that carries no artist tag
- **THEN** the stored image is unchanged and gains no artist tag

#### Scenario: The answer never waits on the tag
- **WHEN** a capture arrives from X whose handle names an existing general tag
- **THEN** the response is 201 with the stored record, exactly as it would be with no adapter record
