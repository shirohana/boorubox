# app-update Specification

## Purpose
The app finds out that a newer version has been published, tells the user, and installs it
only after they agree — so a fix reaches people without a manual reinstall, and without ever
changing the software under someone mid-task.

## Requirements

### Requirement: The running version is visible
Settings SHALL show the version the app is running. The string shown SHALL be the same one
the app reports over its status endpoint, so a user reading it aloud and a log agree.

#### Scenario: Reading the version
- **WHEN** the user opens Settings
- **THEN** the running version is shown as text they can read and repeat

#### Scenario: One version, not two
- **WHEN** the version shown in Settings is compared with the one `GET /status` answers
- **THEN** they are the same string

### Requirement: The app checks for a newer version
The app SHALL check for a newer version at launch and SHALL offer an explicit check in
Settings. A check SHALL compare against the published version and SHALL treat only a strictly
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

### Requirement: An update installs only after the user agrees
The app SHALL NOT download-and-install an update without an explicit confirmation from the
user in that session. Declining SHALL leave the running version installed and SHALL NOT
repeat the prompt for the rest of the session.

#### Scenario: Confirming
- **WHEN** the user is told an update is available and agrees to install it
- **THEN** it is downloaded, installed, and the app restarts on the new version

#### Scenario: Declining
- **WHEN** the user is told an update is available and declines
- **THEN** nothing is downloaded or installed, the app carries on, and the prompt does not reappear until the app is started again

#### Scenario: Never silent
- **WHEN** an update is available and the user has not answered
- **THEN** no part of the installed application has been replaced

### Requirement: An update is refused unless it is signed by this project's key
The app SHALL verify the publisher's signature over an update before installing it and SHALL
refuse an artifact that fails verification, reporting the refusal rather than installing.

#### Scenario: Tampered artifact
- **WHEN** a downloaded update does not verify against the app's embedded public key
- **THEN** it is not installed and the user is told the update could not be verified

### Requirement: An update is not taken while work is in flight
The app SHALL NOT restart into a new version while an import or a capture is running. Where
the user confirms an update while work is in flight, the app SHALL say what is running and
SHALL leave the work alone.

#### Scenario: Update during a migration
- **WHEN** the user confirms an update while a 25,000-image import is running
- **THEN** the app does not restart, says the import is still running, and the import keeps going
