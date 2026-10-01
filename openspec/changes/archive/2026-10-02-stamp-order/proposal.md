## Why

Stamps are listed in creation order everywhere — the settings table and the bar — and the
`stamps` change (2026-09-23) made reordering a non-goal. The owner's bar has grown, the stamps
used daily sit wherever they were born, and on 2026-10-01 they asked: "Settings > Stamps:
Support rearrange stamps." Requirements §6 (edit mode as a day-to-day tool).

## What Changes

- **A stamp has a position**, kept with the library and restored by a rebuild. New stamps
  land last. The settings table and the bar both list by position.
- **The Stamps page reorders**: a drag handle on each row for the pointer, and "Move up" /
  "Move down" on the row for the keyboard. The bar follows at once.
- The one reorder primitive is shared: `pinned-group-management` reorders groups with the
  same helper and the same handle, so two lists cannot drag differently.

## Capabilities

### Modified Capabilities

- `stamps`: "Stamps are kept with the library and managed on the settings screen" — order is
  the user's, not creation's.
- `library-recovery`: the library's own file keeps the stamps in their order.

## Non-goals

- Reordering from the bar: the bar is for applying; management is on the settings page, and
  a drag among chips that fill a field on click is two gestures on one control.
- Sorting by name or by use.
