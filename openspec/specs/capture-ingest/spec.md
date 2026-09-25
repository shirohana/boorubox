# capture-ingest Specification

## Purpose
Any local source (the bridge extension, a script, another browser) delivers images to the
running app over localhost HTTP with no registration step.

## Requirements

### Requirement: Listener binds loopback only
The app SHALL listen on `127.0.0.1` at a configurable port, default 47201, and SHALL NOT bind
any other interface. The port SHALL be shown in settings and SHALL be changeable there.
Changing it SHALL take effect without restarting the app: the app SHALL stop listening on the
old port, bind the new one, and report the outcome. A port that cannot be bound SHALL be
stored anyway, so the setting the user chose is what they see when they come back to it.

#### Scenario: Default start
- **WHEN** the app opens a library
- **THEN** `http://127.0.0.1:47201` accepts connections and no other interface does

#### Scenario: Port unavailable
- **WHEN** the configured port is already in use
- **THEN** the app still opens the library and settings show the listener as failed with the reason

#### Scenario: Port changed to a free one
- **WHEN** the user sets a different, free port in settings
- **THEN** captures posted to the new port succeed, the old port no longer accepts connections, and settings show the listener running on the new port — with no restart

#### Scenario: Port changed to one already in use
- **WHEN** the user sets a port another process holds
- **THEN** settings show the listener as failed with the reason, the chosen port is what the field shows, and the app keeps working

### Requirement: Captures are accepted by multipart POST
`POST /captures` SHALL accept a multipart body with exactly one file part and one JSON part
carrying `id` (UUID), `imageUrl`, `pageUrl`, `pageTitle`, `capturedAt`, and an optional site
adapter record `{ site, fields }`. On success it SHALL respond 2xx with the stored record.
A response earlier than the stored record SHALL NOT be sent.

When an adapter record is present it SHALL be stored with the image exactly as received and
SHALL be returned on the stored record, so that a later reader of the library sees what the
capturing client extracted without re-deriving it. The app SHALL NOT reject a capture over
the shape of the adapter record's fields.

#### Scenario: New capture
- **WHEN** a request arrives with an unseen `id` and a decodable image
- **THEN** the file is written under `images/`, the row is stored with `source=extension`, and the response is 201 with the record

#### Scenario: Capture with an adapter record
- **WHEN** a request arrives with an adapter record naming a site and carrying fields
- **THEN** the stored record carries that site and those fields unchanged, and reading the image back returns them

#### Scenario: Capture without an adapter record
- **WHEN** a request arrives with no adapter record
- **THEN** the image is stored and its record carries no adapter record

#### Scenario: Adapter fields the app does not know
- **WHEN** the adapter record carries field names the app has no meaning for
- **THEN** the capture is stored and those fields are kept as sent

#### Scenario: Missing part
- **WHEN** the file part or the JSON part is absent, or the JSON lacks `id`
- **THEN** the response is 400 with a reason and nothing is stored

#### Scenario: Undecodable image
- **WHEN** the file part is not an image the app can decode
- **THEN** the response is 422 with a reason and nothing is stored

### Requirement: Delivery is idempotent by id
A `POST /captures` whose `id` is already stored SHALL succeed without storing a second copy.

#### Scenario: Retry after success
- **WHEN** the same `id` is posted again, with the same or different bytes
- **THEN** the response is 200 with the existing record and the library contains one image for that id

### Requirement: Origin check rejects web pages
Requests other than `GET /status` SHALL be accepted only when the `Origin` header starts with
`chrome-extension://`. Others SHALL be rejected with 403 before the body is read.

#### Scenario: Request from a web page
- **WHEN** a request carries `Origin: https://example.com`
- **THEN** the response is 403 and nothing is stored

#### Scenario: Request without Origin
- **WHEN** a `POST /captures` carries no `Origin` header
- **THEN** the response is 403

### Requirement: Status endpoint
`GET /status` SHALL return app version, library path and image count, and SHALL be usable
by callers without an `Origin` header.

#### Scenario: Status while open
- **WHEN** `GET /status` is called while a library is open
- **THEN** the response is 200 JSON with `version`, `libraryPath`, `imageCount`

#### Scenario: Status before a library is chosen
- **WHEN** `GET /status` is called and no library is open
- **THEN** the response is 503 with a JSON body saying no library is open

### Requirement: A stored capture reaches the open window
A capture the listener stores while the app is open SHALL be announced to the webview as
stored, and a screen showing the library SHALL read the library again on it. A request that
stored nothing SHALL NOT be announced as stored; it settles any pending announcement as
withdrawn instead, and the library screen SHALL NOT re-read the library for it.

#### Scenario: Capture arrives while the library is on screen
- **WHEN** a capture is stored and the library screen is open
- **THEN** the image appears in the grid without the user leaving the screen and coming back

#### Scenario: Capture arrives while another screen is on
- **WHEN** a capture is stored while the user is on a screen that does not list images
- **THEN** the frame's image count follows it

#### Scenario: Delivery the app has already accepted
- **WHEN** a capture is re-posted under an id already stored
- **THEN** nothing is announced as stored and the grid is left as it is

#### Scenario: Capture refused
- **WHEN** a request is refused before anything is stored
- **THEN** nothing is announced as stored

### Requirement: A coming capture can be announced
`POST /captures/pending` SHALL accept a JSON body describing the capture the client will later
post — the same fields as the `meta` part: `id`, `imageUrl`, `pageUrl`, `pageTitle`,
`capturedAt`, and an optional adapter record — SHALL store nothing, and SHALL answer 202 after
telling the open window about it. The origin rule for `POST /captures` applies. With no library
open it SHALL answer 503 and tell the window nothing, since the capture it announces would be
refused.

#### Scenario: Announced while a library is open
- **WHEN** a request with an unseen `id` arrives while a library is open
- **THEN** the response is 202, the window is told the capture is pending with the fields sent, and the library's image count is unchanged

#### Scenario: Announced with no library open
- **WHEN** a request arrives and no library is open
- **THEN** the response is 503 and nothing is told to the window

#### Scenario: Announcement without an id
- **WHEN** the body lacks `id` or is not JSON
- **THEN** the response is 400 and nothing is told to the window

### Requirement: An announced capture can be withdrawn
`DELETE /captures/pending/{id}`, with an optional JSON body carrying a `reason`, SHALL answer
204 and tell the open window that `id` is withdrawn with that reason. An `id` the app was never
told about SHALL be answered the same way: the app holds nothing to check it against.

#### Scenario: Withdrawn with a reason
- **WHEN** a withdrawal arrives for an announced `id` with a reason
- **THEN** the response is 204 and the window is told the `id` is withdrawn with that reason

#### Scenario: Withdrawn for an unknown id
- **WHEN** a withdrawal arrives for an `id` never announced
- **THEN** the response is 204

### Requirement: Every answer to a capture settles its announcement
When `POST /captures` answers for a request whose `id` could be read, the open window SHALL
be told the outcome: a newly stored row as stored, and anything else — the `id` already stored,
or the capture refused — as withdrawn, a refusal carrying its reason. A request refused before
its `id` could be read SHALL settle nothing.

#### Scenario: New capture stored
- **WHEN** a capture is stored as a new row
- **THEN** the window is told it is stored and is not told it is withdrawn

#### Scenario: Capture already stored
- **WHEN** a capture is posted under an `id` already stored
- **THEN** the window is told the `id` is withdrawn, with no reason, and is not told anything is stored

#### Scenario: Capture refused
- **WHEN** a capture with a readable `id` is refused
- **THEN** the window is told the `id` is withdrawn with the refusal's reason

#### Scenario: Refused before the id is known
- **WHEN** a request is refused because its JSON part is missing or unreadable
- **THEN** nothing is told to the window

### Requirement: A capture is stored with its author as an artist tag
An image entering the library with an adapter record SHALL be stored carrying one artist tag.
When an artist entry (`artist-entries`) owns the author's profile URL the record names (for
`x`, `https://x.com/<handle>`; for `pixiv`, `https://www.pixiv.net/users/<userId>`), the tag
SHALL be that entry's; among several owning entries the one with the longest URL SHALL win. The
page URL and the post URL SHALL NOT be consulted: the page URL is the tab's address, which on X
is often the timeline or another user's page. An entry whose tag exists under a category other
than artist SHALL be ignored for this purpose. Otherwise the tag SHALL be named after the
author the record names: for a record from `x`, its `handle`; for a record from `pixiv`, its
`artist` (the display name). The name SHALL be spelled the way every tag is:
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

#### Scenario: An entry owns the handle's profile URL
- **WHEN** the artist `metaljelly` owns `https://x.com/metaljelly0811` and a capture arrives with the record `{ site: "x", fields: { handle: "metaljelly0811", postUrl: "https://x.com/metaljelly0811/status/1" } }`
- **THEN** the stored image carries `metaljelly`, an artist tag, and not `metaljelly0811`

#### Scenario: An entry owns the Pixiv user
- **WHEN** the artist `kani_beam` owns `https://www.pixiv.net/users/3439325` and a capture arrives with the record `{ site: "pixiv", fields: { artist: "かにビーム", userId: "3439325" } }`
- **THEN** the stored image carries `kani_beam` and not `かにビーム`

#### Scenario: The page URL names nobody
- **WHEN** the artist `bob` owns `https://x.com/bob` and a capture of alice's post arrives from bob's profile page with the record `{ site: "x", fields: { handle: "alice" } }` and page URL `https://x.com/bob`
- **THEN** the stored image carries `alice`

#### Scenario: A neighbouring handle is not owned
- **WHEN** the artist `metaljelly` owns `https://x.com/metaljelly0811` and a capture arrives from X with the handle `metaljelly08110`
- **THEN** the stored image carries `metaljelly08110`

#### Scenario: An old Pixiv record without the user id
- **WHEN** the artist `kani_beam` owns `https://www.pixiv.net/users/3439325` and a capture arrives with the record `{ site: "pixiv", fields: { artist: "かにビーム" } }`
- **THEN** the stored image carries `かにビーム`, since nothing in the record names the user
