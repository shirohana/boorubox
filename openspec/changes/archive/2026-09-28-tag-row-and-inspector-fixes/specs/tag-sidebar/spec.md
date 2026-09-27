## ADDED Requirements

### Requirement: A tag row looks the tag up on Danbooru
Each tag listed in the sidebar SHALL offer, on the row itself and before its include and
exclude controls, a `?` control that opens the tag on Danbooru in the system browser — the
same destination the tag's context menu offers (`tag-vocabulary`, "A category is changed where
the tag is shown"): the tag's wiki page for a tag of any category but artist, the artist search
by name for an artist tag. The control SHALL be named for what it opens, SHALL be reachable by
keyboard like the row's other controls, and SHALL NOT change the search. A collection row SHALL
NOT offer it: a collection has no Danbooru page. The `?`, include and exclude controls SHALL
read as one group, set apart from the tag's name. Danbooru's own tag list puts a `?` beside
every tag for the same reason: a tagger looks a tag up far more often than a right-click
invites (owner, 2026-09-28).

#### Scenario: A general tag
- **WHEN** the sidebar lists the general tag `solo` and the user presses its `?`
- **THEN** the system browser opens `https://danbooru.donmai.us/wiki_pages/solo` and the search is unchanged

#### Scenario: An artist tag
- **WHEN** the sidebar lists the artist tag `metaljelly` and the user presses its `?`
- **THEN** the system browser opens Danbooru's artist search with `metaljelly` as the name to match

#### Scenario: Named for what it opens
- **WHEN** the user hovers the `?` of the general tag `solo`
- **THEN** it reads "Open Danbooru wiki", the label the context menu uses for the same tag

#### Scenario: No look-up on a collection
- **WHEN** the sidebar lists the collection `favourites`
- **THEN** its row offers include and exclude and no `?`
