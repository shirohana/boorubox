## MODIFIED Requirements

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

## ADDED Requirements

### Requirement: A stored capture reaches the open window
A capture the listener stores while the app is open SHALL be announced to the webview, and a
screen showing the library SHALL read the library again on it. A request that stored nothing
SHALL NOT be announced.

#### Scenario: Capture arrives while the library is on screen
- **WHEN** a capture is stored and the library screen is open
- **THEN** the image appears in the grid without the user leaving the screen and coming back

#### Scenario: Capture arrives while another screen is on
- **WHEN** a capture is stored while the user is on a screen that does not list images
- **THEN** the frame's image count follows it

#### Scenario: Delivery the app has already accepted
- **WHEN** a capture is re-posted under an id already stored
- **THEN** nothing is announced and the grid is left as it is

#### Scenario: Capture refused
- **WHEN** a request is refused before anything is stored
- **THEN** nothing is announced
