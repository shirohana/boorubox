## MODIFIED Requirements

### Requirement: The editor suggests tags the library already uses
While a tag is being typed the app SHALL offer tags already used in the library that begin
with what has been typed, most used first, excluding tags already present in the input, each
drawn in its category's colour. It SHALL NOT offer anything while the token being typed is a
metatag or the `or` operator of the query language, nor while it begins with a category
prefix. Accepting a suggestion SHALL replace the token being typed, preserving a leading `-`
when the token has one.

#### Scenario: Prefix
- **WHEN** the library uses `cat`, `cathedral` and `dog`, and the user types `cat`
- **THEN** `cathedral` is offered and `dog` is not

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
