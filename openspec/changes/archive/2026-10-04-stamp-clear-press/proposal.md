## Why

The stamp bar's clear control (the "x" inside the field) jumps down when pressed and the
press does not clear the field; the field loses its focus too (owner, 2026-10-04: "it moves
down and I lose the focus"). The kit's button nudges itself one pixel down while active
through the same CSS translate the control uses to centre itself vertically, so on the press
the centring is replaced by the nudge, the control drops half its height, and the release
lands outside it: no click, no clear, and the field the press blurred stays blurred.
Requirements §6.

## What Changes

- **The clear control stays put while pressed.** Its vertical centring moves off the button
  onto a wrapper, so the button's own press nudge is the only thing the press moves.

## Capabilities

### Modified Capabilities

- `stamps`: "Edit mode applies the active stamp by a click" says the clear control stays
  under the pointer while pressed, and that the press clears the field and leaves the caret
  in it.

## Non-goals

- A clear control on any other field.
- Changing the kit's button.
