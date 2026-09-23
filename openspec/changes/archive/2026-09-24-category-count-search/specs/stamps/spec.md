## MODIFIED Requirements

### Requirement: A stamp is an edit written in the tag language
A stamp SHALL be a text whose space-separated tokens each mean one of: `tag` add the tag,
`-tag` remove it, `collection:name` put the image in that collection, `-collection:name` take
it out, `rating:g|s|q|e` set the rating, and a category prefix (`artist:name` and the others
`tag-vocabulary` names) add the tag and create it under that category. A rating token SHALL
set the rating and never clear or toggle it; the last rating token wins. A token that is a
search-only metatag (`is:`, `tagcount:`, `gentags:`, `arttags:`, `chartags:`, `copytags:`,
`metatags:`, `account:`, `or`) or `-rating:` SHALL make the text invalid with a reason naming
the token; an empty text SHALL be invalid. A collection named that does not exist SHALL refuse
the apply with a reason naming it; nothing SHALL be created by a stamp except tags.

#### Scenario: Tags both ways
- **WHEN** the stamp `cat animal -dog` is applied to an image tagged `dog`, `cute`
- **THEN** the image is tagged `cat`, `animal`, `cute`

#### Scenario: Moving between collections
- **WHEN** the stamp `collection:cute -collection:uncategorized` is applied to an image in `Uncategorized`
- **THEN** the image is in `Cute` and not in `Uncategorized`

#### Scenario: A rating sets and stays
- **WHEN** the stamp `rating:g` is applied twice to an image rated `s`
- **THEN** the image is rated `g` after the first apply and still `g` after the second

#### Scenario: Not an edit
- **WHEN** the user saves a stamp reading `cat is:png`
- **THEN** the save is refused naming `is:png` as not an edit

#### Scenario: A category count is not an edit
- **WHEN** the user saves a stamp reading `touhou copytags:0`
- **THEN** the save is refused naming `copytags:0` as not an edit

#### Scenario: A collection that does not exist
- **WHEN** the stamp `collection:nope` is applied and no collection is named Nope
- **THEN** the apply is refused naming `nope`, and the image is unchanged

#### Scenario: A category conflict
- **WHEN** `cat` is a general tag and the stamp `artist:cat` is applied
- **THEN** the apply is refused with `tag-vocabulary`'s reason, and the image is unchanged
