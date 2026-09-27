## MODIFIED Requirements

### Requirement: The running version is visible
The About page of Settings SHALL show the version the app is running. The string shown SHALL be the same one
the app reports over its status endpoint, so a user reading it aloud and a log agree.

#### Scenario: Reading the version
- **WHEN** the user opens the About page of Settings
- **THEN** the running version is shown as text they can read and repeat

#### Scenario: One version, not two
- **WHEN** the version shown in Settings is compared with the one `GET /status` answers
- **THEN** they are the same string

### Requirement: The app checks for a newer version
The app SHALL check for a newer version at launch and SHALL offer an explicit check on
the About page of Settings. A check SHALL compare against the published version and SHALL treat only a strictly
greater version as an update.

A check that fails — no network, an unreachable or malformed manifest — SHALL be reported as
a failed check where the user asked for it and SHALL NOT block, delay or interrupt anything
else in the app. A launch check that fails SHALL be silent.

#### Scenario: A newer version exists
- **WHEN** the published version is greater than the running one
- **THEN** the user is told a newer version is available, and which version it is

#### Scenario: Already current
- **WHEN** the user checks from Settings and the running version is the published one
- **THEN** they are told they are up to date

#### Scenario: Offline at launch
- **WHEN** the app launches with no network
- **THEN** it opens normally, nothing about updates is shown, and no error is raised

#### Scenario: Offline on demand
- **WHEN** the user presses Check in Settings with no network
- **THEN** they are told the check failed, and the app is otherwise unaffected

