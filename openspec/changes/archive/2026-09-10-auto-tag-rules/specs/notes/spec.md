## Purpose

One free-text scratchpad per library — the tags to remember to add, the account still to go
through, the thing the library is for — kept beside the results and saved without being asked.

## ADDED Requirements

### Requirement: A library has one note
The app SHALL keep one free-text note per library, stored with the library so that copying
the folder carries the note and opening a different library shows that library's note. A
library that has never been written to SHALL show an empty note rather than an error or a
placeholder that has to be cleared before typing.

#### Scenario: A note belongs to its library
- **WHEN** a note is written in one library and a second library is opened
- **THEN** the second library's note is empty, and reopening the first shows the note unchanged

#### Scenario: A library with no note yet
- **WHEN** a freshly created library is opened
- **THEN** the note is empty and can be typed into immediately

#### Scenario: Copying the folder carries the note
- **WHEN** a library folder holding a note is copied and the copy is opened
- **THEN** the copy holds the same note

### Requirement: The note saves itself
The note SHALL be saved as it is typed, without a save action, and SHALL survive closing and
reopening the app. Saving SHALL NOT happen on every keystroke, and a pause in typing SHALL be
enough to have it saved. Text typed and then not changed again SHALL NOT be lost by leaving
the panel, switching libraries or closing the window.

#### Scenario: Typing and relaunching
- **WHEN** the user types into the note, waits a moment, quits the app and reopens the library
- **THEN** the note holds what was typed

#### Scenario: Leaving immediately
- **WHEN** the user types into the note and switches library or closes the window without pausing
- **THEN** the note still holds what was typed when the library is reopened

#### Scenario: Emptying the note
- **WHEN** the user deletes everything in the note
- **THEN** the note is empty after a relaunch, rather than showing the previous text

### Requirement: The note lives in a panel that can be folded away
The note SHALL be shown beside the results, below the list of tags, in a panel the user can
collapse and expand. Whether it is collapsed SHALL be remembered across restarts, because a
panel large enough to write in is large enough to be in the way of the tag list. The panel
SHALL be absent while no library is open, since there is no note to show.

#### Scenario: Collapsing
- **WHEN** the user collapses the note panel and restarts the app
- **THEN** the panel is still collapsed, and expanding it shows the note unchanged

#### Scenario: No library open
- **WHEN** no library is open
- **THEN** no note panel is shown
