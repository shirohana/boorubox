## MODIFIED Requirements

### Requirement: Tag search uses the legacy query language
A search box SHALL accept the legacy extension's query syntax: space-separated tags are AND,
`a or b` is OR, `-tag` excludes, and the metatags `rating:`, `is:`, `tagcount:` and `account:`
filter as they do in the legacy viewer. A tag term SHALL match without regard to case: tag
names are stored lowercase (`tag-vocabulary`), and a term is lowercased before it is
matched, so `Cat` finds what `cat` finds. Free text in `page title` and URLs SHALL be
searchable. The metatag `collection:<name>` SHALL match images in the collection whose name,
lower-cased with spaces as underscores, is `<name>`; `-collection:<name>` SHALL exclude them;
a name no collection has SHALL match nothing.

#### Scenario: AND and NOT
- **WHEN** the query is `cat -dog`
- **THEN** only images tagged `cat` and not tagged `dog` are shown

#### Scenario: Case
- **WHEN** the query is `Cat -DOG`
- **THEN** the result is the same as for `cat -dog`, and the panels mark `cat` as included and `dog` as excluded

#### Scenario: OR group
- **WHEN** the query is `cat or dog`
- **THEN** images tagged either `cat` or `dog` are shown

#### Scenario: Rating metatag
- **WHEN** the query is `rating:s,q`
- **THEN** only images rated `s` or `q` are shown

#### Scenario: Empty result
- **WHEN** no image matches
- **THEN** the grid shows an empty state naming the query, not a blank page

#### Scenario: Collection metatag
- **WHEN** the query is `cat collection:my_favorites`
- **THEN** only images tagged `cat` that are in the collection named `My favorites` are shown

#### Scenario: Excluding a collection
- **WHEN** the query is `-collection:queue`
- **THEN** images in `Queue` are not shown and every other image is
