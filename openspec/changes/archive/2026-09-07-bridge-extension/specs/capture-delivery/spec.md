## Purpose

The bridge extension takes an image off a web page and hands it to the running app over
localhost, keeping the bytes until the app confirms it stored them, so a capture made while
the app is closed is never lost.

## ADDED Requirements

### Requirement: Capture from the image context menu
The extension SHALL add one context-menu entry on images whose label names this app and
differs from the legacy extension's entry. Choosing it SHALL capture the image bytes from
the page context, and SHALL fall back to fetching the image URL with the page URL sent as
the referrer when the page context cannot produce the bytes.

#### Scenario: Captured from the page
- **WHEN** the user chooses the entry on an image the page has already loaded
- **THEN** the bytes come from the page context and no second request is made to the image host

#### Scenario: Page context cannot produce the bytes
- **WHEN** the page context fails to produce bytes for the image
- **THEN** the image URL is fetched with the page URL as referrer and those bytes are used

#### Scenario: Both routes fail
- **WHEN** neither the page context nor the fetch produces bytes
- **THEN** the user is notified with the reason and no history entry is created

### Requirement: Delivery reports success only after the app stores the capture
A capture SHALL be posted to the app with a caller-generated UUID, the image bytes, the
image URL, page URL, page title, capture time and the site adapter record when one was
extracted. The capture SHALL be reported delivered only after the app answers 2xx. Until
then it SHALL be `pending`, and any other outcome SHALL make it `failed` with its bytes kept.

#### Scenario: App accepts the capture
- **WHEN** the app answers 2xx
- **THEN** the entry becomes `delivered` and the kept bytes are released

#### Scenario: App is not running
- **WHEN** the request cannot reach the app
- **THEN** the entry becomes `failed`, its bytes are kept, and a notification says to open the app to receive the image

#### Scenario: App refuses the capture
- **WHEN** the app answers a non-2xx status
- **THEN** the entry becomes `failed` with the app's reason shown on it and its bytes are kept

#### Scenario: Delivery interrupted
- **WHEN** the extension's background context is restarted while a capture is `pending`
- **THEN** that entry becomes `failed` and is retryable, never left `pending` forever

### Requirement: Retry does not re-fetch the image
Retrying a failed capture SHALL post the bytes kept from the original capture under the
same UUID, and SHALL NOT request the image from its host again. A retry of a capture the
app already holds SHALL end as delivered without creating a second image.

#### Scenario: Retry after the app opens
- **WHEN** the user retries a failed entry while the app is running
- **THEN** the kept bytes are posted, no request goes to the image host, and the entry becomes `delivered`

#### Scenario: Retry of a capture the app already stored
- **WHEN** the first delivery reached the app but its answer was lost, and the user retries
- **THEN** the app answers with the existing record, the entry becomes `delivered`, and the library holds one image for that UUID

#### Scenario: Kept bytes are gone
- **WHEN** a failed entry's bytes are no longer available in the browser
- **THEN** the entry says so, offers no Retry, and can only be discarded

### Requirement: The app's address is a setting
The extension SHALL post to `127.0.0.1` on a configurable port defaulting to the same
default the app uses, and SHALL NOT contact any other host. Changing the port SHALL take
effect on the next capture and retry without reinstalling the extension.

#### Scenario: Port changed on both sides
- **WHEN** the app is moved to another port and the extension's port field is set to match
- **THEN** the next capture is delivered to the new port

#### Scenario: Port mismatch
- **WHEN** the extension's port does not match the app's
- **THEN** captures fail and are retryable, and the connection indicator reports the app as not running

### Requirement: Connection indicator
While the extension's popup is open it SHALL report whether the app answers on the
configured port, and distinguish "running with a library open" (showing the library's image
count) from "running with no library open" and from "not reachable".

#### Scenario: App running with a library
- **WHEN** the popup is open and the app answers with its status
- **THEN** the popup shows the app as connected together with the library's image count

#### Scenario: App running without a library
- **WHEN** the app answers that no library is open
- **THEN** the popup says the app is running but has no library open

#### Scenario: App not reachable
- **WHEN** the app does not answer
- **THEN** the popup says the app is not running, and captures made now will be retryable
