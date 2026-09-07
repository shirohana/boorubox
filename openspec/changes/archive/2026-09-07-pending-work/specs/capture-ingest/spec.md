## ADDED Requirements

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

## MODIFIED Requirements

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
