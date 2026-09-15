## Why

The owner's first week on Windows (2026-09-15) turned up four things the inspector panel does
wrong while looking right: every write from it takes the keyboard away from the grid, the two
addresses it shows cannot be opened, a click on one of its tags throws away the image the user
was looking at and closes the viewer, and the one filter that finds an artist's other captures
(`account:`) has to be typed by hand. §6 makes the browsing screen the reference the legacy
viewer set; these are the gaps between the panel and that reference, and they are hit on every
tagging pass.

**Depends on:** `browse-polish` (the viewer's focus rules and the rating control's own keys),
`selection-and-bulk` (the selection store) and `tags-and-ratings` (the tag click and its menu),
all archived and implemented. It touches nothing `trash`, `booru-upload` or `library-sidecars`
owns.

## What Changes

- **A write from the inspector hands the keyboard back** (§6 keyboard nav): choosing a rating,
  saving or removing tags, or clicking a tag beside the grid returns the focus to the grid's
  current card, so the arrows, Space and `i` work again without a click. Inside the viewer the
  same actions already return the focus to the viewer; that placement is unchanged.
- **The page address and the image address can be opened** in the system browser from a small
  button after each, in both placements of the panel — the same way a posted label opens its
  post.
- **A tag click keeps what the user was looking at**: the image the panel describes stays the
  current image after the search re-runs, found again by identity in the new result, and the
  viewer stays open on it. A multi-image selection is dropped, because the result set changed
  (`selection` spec, amended). Only when the image is no longer in the result — an exclusion —
  does the screen fall back to today's reset.
- **Tags on screen show whether they are in the search**: a tag the query includes is drawn the
  way the sidebar draws it (green), an excluded one struck through, so the panel reads as a set
  of toggles.
- **An X capture's account is a filter on screen**: the first row under the tag editor shows the
  account handle its page address names, in blue, toggling `account:<handle>` in the search
  like a tag. The handle is the one the search itself matches on, derived once on the Rust side
  and carried on the image record, so the button can never name an account the search cannot
  find. Absent for any image whose page address names no X account. **BREAKING** for the IPC
  contract only in the additive sense: `ImageRecord` gains one nullable field.

## Non-goals

- Making the adapter record's `handle` searchable (backlog defect 3): the owner reports the
  extension captures the post's own address, so the page-address rule covers it in practice.
- Unifying the three X-handle rules that now exist (`query.rs`, `artist-from-url.ts` in the
  auto-tag rules, and this change's record field, which reuses the first). Noted in design
  Risks; the rules module answers a different question ("who is the artist") and is lifted
  verbatim by §6.
- Any change to what a *typed* query does to the selection: typing a new search still resets.
- Editing the facts, collections, the viewer's chrome — later changes in the same queue.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `app-frame`: "One inspector panel, two placements" gains the open-address action and the
  account row; "One keyboard map" gains the rule that a write from the panel hands the keys
  back to the region the panel sits beside.
- `tag-editing`: "Tags on screen are search terms" — a tag shows whether it is in the search,
  and acting on it keeps the inspected image current; a new requirement for the account row.
- `library-browse`: "Lightbox" — a search rewritten from inside the view keeps it open on the
  same image.
- `selection`: "A selection belongs to one result set" — a search rewritten by a click on a tag
  or account empties the selection but keeps the focus on the inspected image.

## Impact

- `packages/app/src-tauri`: `ImageRecord` gains `account` (derived from `page_url` by the
  `x_account` rule `query.rs` already owns, at load time, never stored); one new command,
  `search_position`, answering the row an id occupies in a search's order.
- `packages/shared`: the `ImageRecord` mirror.
- `packages/app/src/lib`: `tag-utils` gains the account rewriter and a shared "which terms are
  active" reader the sidebar and the inspector both use; `Inspector.svelte`, `LibraryScreen.svelte`,
  `LibraryGrid.svelte` and `Lightbox.svelte` for the focus hand-back and the keep-the-subject
  search; one new `ExternalLink` button component.
- Sidecar format and schema: untouched.
