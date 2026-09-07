## ADDED Requirements

### Requirement: The app is told a capture is coming before its bytes are fetched
When the user chooses the context-menu entry, the extension SHALL tell the app the capture's
id and page facts before it obtains the image bytes, SHALL wait for the app's answer only
briefly, and SHALL go on with the capture whether or not the app answered. When neither route
produces bytes it SHALL withdraw the announcement with the reason. A retry SHALL NOT announce:
its bytes are already in hand.

#### Scenario: App running
- **WHEN** the entry is chosen while the app is running
- **THEN** the app is told the capture is coming before any request goes to the image host or the page is asked for the bytes, under the same id the capture is later posted with

#### Scenario: App not running
- **WHEN** the entry is chosen while the app is not running
- **THEN** the capture proceeds, and ends `failed` with its bytes kept as it does today

#### Scenario: Both routes fail
- **WHEN** neither the page context nor the fetch produces bytes after the announcement
- **THEN** the app is told the id is withdrawn with the reason, and the user is notified as before

#### Scenario: Retry
- **WHEN** the user retries a failed entry
- **THEN** the kept bytes are posted and no announcement is made
