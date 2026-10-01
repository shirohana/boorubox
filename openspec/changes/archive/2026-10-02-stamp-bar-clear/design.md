## Context

`StampBar.svelte` binds `text` (owned by `LibraryScreen`, which derives the active stamp from
it and clears it on leaving the mode). Saved stamps are `Button`s whose `variant` and
`aria-pressed` follow `stamp.text === text`; `onclick` sets `text = stamp.text`. The field is
`TagInput`, which takes a `class` but has no trailing slot. No field in the app carries a
clear control today; the `input-group` kit component exists but is unused, and one absolutely
placed button is less than adopting it for one field.

## Decisions

**D1. The active chip toggles: `text = stamp.text === text ? '' : stamp.text`.** The
2026-09-23 reading — "a chip only ever fills the field on click, never toggles itself off, so
there is no boolean here for a real toggle to own" — was right when the bar was new: the one
signal was the field, and a chip that could clear it looked like a second owner of the active
state. It stopped being right once stamping became daily work (owner, 2026-10-01): the
pressed chip already reads as "on", and a pressed control that cannot be pressed off breaks
the expectation its own look sets. The field is still the one signal; the chip only writes
to it, now in both directions. `aria-pressed` was already truthful and stays.

**D2. A clear control inside the field, `StampBar`'s own markup, not a `TagInput` prop.**
The field is wrapped in a `relative flex-1` div; `TagInput` gets `pr-7` beside its existing
classes; a ghost `icon-xs` button with `XIcon` is positioned at the right edge inside the
wrapper, mounted only while `text !== ''`, `aria-label="Clear stamp"`, `onclick` sets `text =
''` and refocuses the field. In `StampBar` rather than inside `TagInput` because the bar is
the only field that wants it — the editor and the bulk dialog are multiline and submit on
Enter — and a prop on the shared input for one caller is the same "one caller's need in the
shared component" the inspector's menu rule already refuses.

**D3. The help line reads: "Type a stamp, or click a saved one to fill the field; click an
image to apply it. Click the active stamp again, or clear the field, to stop. There is no
undo — the inverse stamp is the way back."** One sentence changed, the rest as today.

## Risks / Trade-offs

- [A chip click that clears when the user meant to re-activate] → the chip is pressed, the
  field shows its text; nothing to re-activate. The miss is a second click that restores it.
