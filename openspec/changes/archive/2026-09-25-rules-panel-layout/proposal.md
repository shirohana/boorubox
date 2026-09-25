## Why

The Rules section in Settings lists rules as a five-column table inside a horizontally
scrolling box. At the settings page's width (`max-w-2xl`, 672px) the table never fits, so the
switch and the edit and delete buttons sit off-screen and every read of a rule is a sideways
scroll (owner, 2026-09-25: "It's hard to use"). Requirements §11: the smallest thing that works,
and a screen the owner already uses daily.

## What Changes

- **One entry per rule, stacked**, replacing the table: a header line with the name, its badges,
  the enable switch and the edit and delete buttons; under it the pattern and the tags, wrapping
  as they need to. No horizontal scroll at any width the settings page can have.
- Everything the table showed is still shown: name, New badge, pattern or "(matches all)",
  regex badge, invalid reason, tags, enabled, edit, delete, the delete confirmation.

## Capabilities

### Modified Capabilities

- `auto-tag-rules`: the settings listing fits the page's width without a sideways scroll.

## Non-goals

- Any change to what a rule is or does; the form (`RuleForm.svelte`) is untouched.
- The Stamps and Booru tables, which fit their columns today.
- Removing the shadcn `table` copy-in: two other sections use it.
