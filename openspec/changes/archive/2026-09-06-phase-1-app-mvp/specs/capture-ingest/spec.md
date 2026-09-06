## Purpose

Any local source (the bridge extension, a script, another browser) delivers images to the
running app over localhost HTTP with no registration step.

## ADDED Requirements

### Requirement: Listener binds loopback only
The app SHALL listen on `127.0.0.1` at a configurable port, default 47201, and SHALL NOT bind
any other interface. The port SHALL be shown in settings.

#### Scenario: Default start
- **WHEN** the app opens a library
- **THEN** `http://127.0.0.1:47201` accepts connections and no other interface does

#### Scenario: Port unavailable
- **WHEN** the configured port is already in use
- **THEN** the app still opens the library and settings show the listener as failed with the reason

### Requirement: Captures are accepted by multipart POST
`POST /captures` SHALL accept a multipart body with exactly one file part and one JSON part
carrying `id` (UUID), `imageUrl`, `pageUrl`, `pageTitle`, `capturedAt`, and an optional site
adapter record `{ site, fields }`. On success it SHALL respond 2xx with the stored record.
A response earlier than the stored record SHALL NOT be sent.

#### Scenario: New capture
- **WHEN** a request arrives with an unseen `id` and a decodable image
- **THEN** the file is written under `images/`, the row is stored with `source=extension`, and the response is 201 with the record

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
