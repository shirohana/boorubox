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

The metatags `gentags:`, `arttags:`, `chartags:`, `copytags:` and `metatags:` SHALL count an
image's tags in one category — general, artist, character, copyright and meta respectively —
where `tagcount:` counts all of them. Each SHALL take `tagcount:`'s syntax: a number for
exactly that many, `>n`, `<n`, `>=n`, `<=n`, `a..b` for a range inclusive at both ends in
either order, and `a,b,c` for any of the listed counts. Different count metatags in one query
SHALL all apply; the same count metatag given more than once SHALL keep its first occurrence
and ignore the rest. A leading `-` on a count metatag SHALL be ignored, as it is on
`tagcount:`: a count metatag is not negated.

`collection:none` SHALL match images in no collection and `collection:any` images in at least
one; `-collection:none` SHALL match what `collection:any` matches, and `-collection:any` what
`collection:none` matches. These two words SHALL be read as keywords only when they are the
whole value of the term: `none` or `any` inside a comma list SHALL name nothing, and a
collection whose name reduces to `none` or `any` SHALL NOT be searchable by name. Adding or
removing a collection from the sidebar or the inspector SHALL leave a `none`/`any` term in the
query standing.

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

#### Scenario: No copyright tag
- **WHEN** image A is tagged `cat` (general) and `touhou` (copyright), image B only `cat`, and the query is `copytags:0`
- **THEN** B is shown and A is not

#### Scenario: At least one artist tag
- **WHEN** image A has the artist tag `kantoku`, image B has no artist tag, and the query is `arttags:>0`
- **THEN** A is shown and B is not

#### Scenario: A range of character tags
- **WHEN** images carry zero, one, two and three character tags and the query is `chartags:1..2`
- **THEN** the images with one and two character tags are shown, and the others are not

#### Scenario: A list of general counts
- **WHEN** images carry zero, one and two general tags and the query is `gentags:0,1`
- **THEN** the images with zero and one general tag are shown, and the one with two is not

#### Scenario: Two count metatags combine
- **WHEN** the query is `copytags:0 chartags:>0`
- **THEN** only images with no copyright tag and at least one character tag are shown

#### Scenario: A repeated count metatag keeps the first
- **WHEN** the query is `copytags:0 copytags:>0`
- **THEN** the result is the same as for `copytags:0`, and neither term is read as a tag

#### Scenario: In no collection
- **WHEN** image A is in `Favorites`, image B is in no collection, and the query is `collection:none`
- **THEN** B is shown and A is not

#### Scenario: In some collection
- **WHEN** image A is in `Favorites`, image B is in no collection, and the query is `collection:any`
- **THEN** A is shown and B is not

#### Scenario: Negated none
- **WHEN** the query is `-collection:none`
- **THEN** the result is the same as for `collection:any`

#### Scenario: A sidebar click keeps the keyword
- **WHEN** the query is `collection:none` and the user excludes `Queue` from the collections list
- **THEN** the query reads `collection:none -collection:queue`
