## Why

After `pinned-marker` landed, the owner saw the two indicators side by side (2026-10-02): in
the inspector's tag list a tag with a note sits off the line of its neighbours, the dot and
the note glyph are in the wrong order for each panel, and the dot says nothing about which
group the tag is in. Requirements §6 (the inspector as the casual door).

## What Changes

- **Order of the indicators.** Inspector tag list: name, note glyph, dot. Sidebar row: name,
  dot, a gap, note glyph, count. The two panels differ on purpose: in the inspector the dot
  closes the tag, in the sidebar the dot stays next to the name and the glyph next to the
  count (owner, 2026-10-02).
- **A noted tag sits on the line.** The note glyph's wrapper no longer lifts its tag above
  its neighbours in the inspector's list.
- **The dot has a hint.** Hovering it shows "Pinned in <label>" — the group's name or `#n` —
  after the same short delay the note glyph uses.

## Capabilities

### Modified Capabilities

- `tag-vocabulary`: "A pinned tag is marked where it is read" gains the order and the hint.

## Non-goals

- Any change to the pinned chips, tile footers or completion rows.
