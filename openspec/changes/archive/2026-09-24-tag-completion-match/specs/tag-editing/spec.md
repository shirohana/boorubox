## MODIFIED Requirements

### Requirement: The editor suggests tags the library already uses
While a tag is being typed the app SHALL offer tags already used in the library that contain
what has been typed, ranked: the tag equal to the typed text first, then tags that begin with
it, then the rest, each band most used first and then by name (owner, 2026-09-24: tags are
compounds like `cute_cat` whose remembered half is the tail). It SHALL exclude tags already
named elsewhere in the input, but never the word being typed itself, so a whole typed tag is
offered and highlighted and one confirmation finishes it. Each suggestion SHALL be drawn in
its category's colour. It SHALL NOT offer anything while the token being typed is a metatag
or the `or` operator of the query language, nor while it begins with a category prefix.
Accepting a suggestion SHALL replace the token being typed, preserving a leading `-` when
the token has one.

#### Scenario: Prefix first
- **WHEN** the library uses `cat`, `cathedral`, `cute_cat` and `dog`, and the user types `cat`
- **THEN** the list reads `cat`, `cathedral`, `cute_cat`, and `dog` is not offered

#### Scenario: A whole tag confirms
- **WHEN** the library uses `cat` and `cathedral`, the user types `cat` and presses Enter
- **THEN** the input reads `cat ` with the caret after the space, and `cathedral` was not inserted

#### Scenario: Substring
- **WHEN** the library uses `cute_cat` and `strong_cat`, and the user types `cat`
- **THEN** both are offered

#### Scenario: Already typed
- **WHEN** the input already holds `cat` and the user starts typing `cat` again
- **THEN** `cat` is not offered

#### Scenario: Inside a metatag
- **WHEN** the token being typed is `rating:` or `or`
- **THEN** no suggestions appear

#### Scenario: Inside a category count metatag
- **WHEN** the token being typed is `copytags:` or `arttags:>`
- **THEN** no suggestions appear

#### Scenario: Behind a category prefix
- **WHEN** the token being typed is `artist:kan`
- **THEN** no suggestions appear

#### Scenario: Accepting into an exclusion
- **WHEN** the token being typed is `-cathe` and the suggestion `cathedral` is accepted
- **THEN** the token becomes `-cathedral`
