## ADDED Requirements

### Requirement: An entry shows its tag's note
The Artists list in Settings SHALL show an artist tag's note, when it carries one, under the
tag and above its URLs, muted and wrapped as written. An entry whose tag carries no note SHALL
show nothing there. The note is edited from the tag's own context menu, not from this list
(owner, 2026-09-28: the note records which Danbooru artist tag a handle was confirmed as).

#### Scenario: A confirmed artist
- **WHEN** `metaljelly` owns `x.com/metaljelly0811` and carries the note `danbooru: metaljelly (confirmed)`
- **THEN** its entry reads `metaljelly`, then the note, then `x.com/metaljelly0811`

#### Scenario: No note
- **WHEN** `alice` owns a URL and carries no note
- **THEN** its entry reads `alice`, then its URLs, with nothing between
