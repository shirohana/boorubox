## Why

Two suggestion-list misses from the owner (2026-09-24). First: typing a whole tag, `cat`,
drops `cat` from the list because "an exact match never offers itself", so with `cathedral`
also in the library the first row is `cathedral` and Enter inserts it instead of the tag
just typed; with no other match the list closes and Tab does nothing. Second: the list
matches from the start of the name only, and the owner's tags are compounds like `cute_cat`
and `strong_cat` where the noun is what they remember — `cat` finds neither. Requirements §6
(Danbooru-style tagging: the editor is the daily door).

## What Changes

- **An exact match is offered, first.** The tag equal to the word being typed heads the list
  and is highlighted, so Enter or Tab confirms it and finishes the token with a space. A tag
  named elsewhere in the input is still not offered.
- **Matching is by substring, ranked.** A tag containing the typed text anywhere is offered;
  the exact match comes first, then tags that begin with the text, then the rest, each band
  most used first and then by name.

## Capabilities

### Modified Capabilities

- `tag-editing`: "The editor suggests tags the library already uses".

## Non-goals

- Fuzzy matching, or matching across `_` boundaries in any order.
- Changing what confirming does once the exact match is highlighted (the existing two-step
  rule stands).
