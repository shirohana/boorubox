## ADDED Requirements

### Requirement: Imports started during a run are queued
An import started while another is running — by a drop or from the import menu — SHALL be
queued and run after it, in the order started, and SHALL NOT be refused or silently dropped.
The controls that start an import SHALL stay available while a run is going. Each run SHALL
produce its own result.

#### Scenario: Drop during a run
- **WHEN** files are dropped while an import is running
- **THEN** they are imported after the running import finishes, and both runs report their own result

#### Scenario: Menu during a run
- **WHEN** the user opens the import menu while an import is running
- **THEN** the menu's entries can be chosen and what they pick is queued

#### Scenario: Leaving the screen mid-run
- **WHEN** the user leaves the library screen while an import runs with another queued
- **THEN** both still run to completion and their results are shown on return
