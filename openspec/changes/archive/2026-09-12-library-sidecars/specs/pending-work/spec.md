# pending-work Specification (delta)

## ADDED Requirements

### Requirement: The library catching up with its folder is shown like any other work in flight
When the app writes the describing files a library is missing (`library-recovery`), the
library screen SHALL show that as one tile among the tiles for work in flight, saying how many
images are done of the total. The library SHALL stay usable while it runs: searching,
browsing, editing, capturing and importing SHALL all be answered between two images rather
than after the last one, the same bound an import is held to.

The tile SHALL go when the pass finishes and SHALL leave no result to dismiss — nothing the
user asked for happened, and a library already in step shows no tile at all. Closing the
library or opening another SHALL stop the pass rather than let it write into the folder it no
longer belongs to.

#### Scenario: First open of a large library
- **WHEN** a library of 25,000 images is opened for the first time after the app learns to write describing files
- **THEN** a tile shows the count moving, the grid and the search work throughout, and the tile goes when it finishes

#### Scenario: A library already in step
- **WHEN** a library whose images all have describing files is opened
- **THEN** no tile appears

#### Scenario: Switching libraries mid-pass
- **WHEN** the user opens another library while the pass runs
- **THEN** the pass stops, nothing further is written into the folder that was left, and the tile is gone

#### Scenario: Capturing during the pass
- **WHEN** a capture is delivered while the pass runs
- **THEN** it is stored and appears without waiting for the pass to finish
