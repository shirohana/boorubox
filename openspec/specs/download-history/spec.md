# download-history Specification

## Purpose

The record the extension keeps of recent captures, so the user can see what reached the app,
what did not, and act on the difference without opening the app first.

## Requirements

### Requirement: One history entry per capture
Every capture SHALL produce one entry holding a small thumbnail generated at capture time,
the image URL, the page title, the capture time, the byte size and the delivery status. The
entry SHALL survive a browser restart.

#### Scenario: Entry created at capture
- **WHEN** a capture is made
- **THEN** an entry with the thumbnail, image URL, page title, time, size and status appears in the history

#### Scenario: History after a restart
- **WHEN** the browser is restarted
- **THEN** the entries and the status of each are still there

### Requirement: Clear history removes delivered entries only
The Clear action SHALL remove entries the app has confirmed and SHALL leave every failed
entry in place with its bytes.

#### Scenario: Clear with failed entries present
- **WHEN** the user clears the history while delivered and failed entries exist
- **THEN** the delivered entries are gone and every failed entry remains with its bytes

### Requirement: A failed entry is discarded one at a time
Discarding a failed entry SHALL be an explicit per-entry action, and SHALL remove that
entry and its kept bytes and nothing else.

#### Scenario: Discarding one failure
- **WHEN** the user discards one failed entry
- **THEN** that entry and its bytes are removed and other failed entries are untouched

### Requirement: The badge counts failures
The extension's toolbar badge SHALL show the number of failed entries and SHALL show
nothing when there are none. It SHALL NOT show a total of saved images.

#### Scenario: Two captures fail
- **WHEN** two captures fail and none is retried
- **THEN** the badge reads 2

#### Scenario: Failures cleared by retrying
- **WHEN** the last failed entry is delivered by a retry
- **THEN** the badge shows nothing

### Requirement: History is bounded without losing failures
The number of retained delivered entries SHALL be capped, dropping the oldest first. Failed
entries SHALL never be dropped by that cap.

#### Scenario: Cap reached
- **WHEN** more deliveries succeed than the cap allows
- **THEN** the oldest delivered entries are dropped and every failed entry is still listed
