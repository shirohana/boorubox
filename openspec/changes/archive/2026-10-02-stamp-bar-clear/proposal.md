## Why

The stamp bar's field is the one signal for what is active, and the only way to stop
stamping today is to select its text and delete it. The owner (2026-10-01): "Add a reset
button to clear the text. Make the button naturally inside the area. Maybe we can use clicking
the active stamp to clear." Requirements §6 (edit mode).

## What Changes

- **Clicking the active stamp's chip clears the field.** A chip whose text is the field's
  text is drawn pressed today; clicking it again empties the field, so the chip is a toggle.
  This reverses the owner's own 2026-09-23 review ("a chip only ever fills the field on click,
  never toggles itself off"); the design records why both readings were right in their time.
- **A clear control inside the field**, at its right end, shown only while the field has
  text; one click empties it. Typed one-offs have no chip to click, so the chip toggle alone would leave them with no way
  out but selecting the text.
- The bar's help line says both.

## Capabilities

### Modified Capabilities

- `stamps`: "Edit mode applies the active stamp by a click" says how the field is cleared.

## Non-goals

- A key to clear the field; Escape already blurs it, and a key that empties a field the user
  is typing in is a surprise.
- Any change to how a stamp is saved, edited or deleted from the bar.
