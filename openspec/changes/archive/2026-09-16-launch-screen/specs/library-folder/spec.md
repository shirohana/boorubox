## MODIFIED Requirements

### Requirement: Remembered library opens on launch
The app SHALL reopen the last library on launch without asking, unless the user closed it
deliberately, in which case there is no library to reopen, or unless the user has turned the
automatic open off in settings, in which case the start screen is shown with the recent
libraries and nothing is opened until one is chosen. The remembered path SHALL be kept in
both cases.

The launch open SHALL NOT keep the window from appearing: while it runs the app SHALL show a
screen naming the folder being opened, and SHALL show the library UI, or the start screen
with the reason, once it settles.

#### Scenario: Library present
- **WHEN** the app starts and the stored path exists and contains `library.sqlite`
- **THEN** the library UI opens on that folder

#### Scenario: Library missing
- **WHEN** the app starts and the stored path does not exist or is not readable
- **THEN** the start screen is shown with the missing path named, and the stored path is kept until the user picks another

#### Scenario: Closed before quitting
- **WHEN** the user closed the library and then quit
- **THEN** the next launch shows the start screen with that folder offered as the most recent, and nothing is reported as missing

#### Scenario: A large library at launch
- **WHEN** the app starts with a library of 25,000 images remembered
- **THEN** the window appears at once showing that folder's name as opening, and the library UI replaces it when the open finishes

#### Scenario: Automatic open turned off
- **WHEN** the user has turned "Open the last library at launch" off and starts the app
- **THEN** the start screen is shown with the recent libraries, nothing is opened, nothing is reported as missing, and choosing one opens it

#### Scenario: The setting is where the other settings are
- **WHEN** the user opens the settings screen
- **THEN** "Open the last library at launch" is offered as a switch, on unless turned off, and changing it takes effect at the next launch
