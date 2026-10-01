## Why

The Rules page lists every rule stacked, in name order, and the owner's list has grown past
what a scroll finds (2026-10-01: "Settings > Rules: Support search"). Requirements §6 (the app
as the place rules are kept and edited).

## What Changes

- **A search field above the rules list** filters the list as the user types: a rule stays
  when its name, its pattern or any of its tags contains the text, case-insensitively. A blank
  field lists every rule. When nothing matches, the list says so, naming the text.
- Nothing else about a rule changes: the New badge an import marks, the switch, edit and
  delete stay on every listed entry, and the form above the list is unaffected.

## Capabilities

### Modified Capabilities

- `auto-tag-rules`: "Rules are created, changed and deleted from settings" gains the search.

## Non-goals

- Searching from the keyboard with a shortcut, or remembering the text across visits.
- Matching by regular expression or by whole word; the Artists page's filter box is the
  precedent and it is a substring too.
